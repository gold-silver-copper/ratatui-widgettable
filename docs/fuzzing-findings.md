# Fuzzing findings

This document describes what fuzzing `ratatui-widgettable` found: four bugs in the table that were
fixed, and two problems in ratatui's layout engine that affect this crate (and ratatui's own
`Table`) but can't be fixed here.

- **Crate version:** 0.1.0. The fixes are in commit `47b0fbb` and listed under "Unreleased" in
  `CHANGELOG.md`.
- **Compared against:** ratatui's `Table` on the main branch at commit `54b6874` (the code this crate
  was ported from), and the released `ratatui-widgets` 0.3.2.
- **Layout engine versions:** `ratatui-core` 0.1.2, `kasuari` 0.4.12, `hashbrown` 0.16.1, `foldhash`
  0.2.0.

## Summary

| Finding | Where | Effect | Status |
| --- | --- | --- | --- |
| [Arithmetic overflow with huge sizes](#1-arithmetic-overflow-with-huge-sizes) | Table, also in ratatui | Panic in debug builds, wrong layout in release builds | Fixed |
| [Spanning cells drawn past the right edge](#2-spanning-cells-drawn-past-the-right-edge) | Table, also in ratatui | Drawing outside the table | Fixed |
| [Highlight symbol drawn past the right edge](#3-highlight-symbol-drawn-past-the-right-edge) | Table, also in ratatui | Drawing outside the table | Fixed |
| [Rows pushed out by a margin drawn outside the table](#4-rows-pushed-out-by-a-margin-drawn-outside-the-table) | Table, latent in ratatui | Drawing outside the table, or a panic | Fixed |
| [Header and footer sizes chosen at random](#a-header-and-footer-sizes-chosen-at-random) | ratatui layout engine | Output varies between calls or runs | Worked around in the fuzz targets |
| [Column layout hangs or panics](#b-column-layout-hangs-or-panics) | ratatui layout engine | Hang or `InternalSolverError(ObjectiveUnbounded)` panic | Worked around in the fuzz targets |

Each fixed bug has a regression test in `tests/table.rs`. For tables that fit in their area, the
output is unchanged, which is what the differential fuzzer checks (see [below](#why-normal-tables-are-unaffected)).

## How the fuzzing worked

The fuzz targets are in `fuzz/` and use [cargo-fuzz] (libFuzzer). See `fuzz/README.md` for how to
run them. Both are built with debug assertions (`-a`), so arithmetic overflow panics instead of
silently wrapping.

Each input is a random table: rows, header, footer, cell text, column spans, alignment, row heights
and margins, column constraints, column spacing, flex, highlight symbol and spacing, an optional
block (borders, title, padding), every style, the render area (including positions near the edge of
the `u16` coordinate space), and the `TableState` (offset, selected row and column).

- **`differential`** builds the same text-only table with this crate and with ratatui's `Table`,
  renders both, then renders both again with the state that the first render produced. Every cell
  of both buffers (symbol, foreground, background, modifiers and `skip`) and the resulting state
  must be identical. It compares against ratatui's main branch rather than the 0.3.2 release,
  because the crate was ported from main, which has `scroll_padding` and a scrolling fix that 0.3.2
  doesn't (see [Other observations](#other-observations)).
- **`widgets`** fills the cells with paragraphs (wrapped, bordered and scrolled), gauges, sparklines,
  blocks, nested tables, and closures created with `Cell::from_fn`. It renders into a small area in
  the middle of a larger buffer filled with a sentinel symbol and style, and checks that:
  - rendering doesn't panic;
  - no cell outside the table's area changes;
  - closures only receive areas inside the table's area;
  - the state is valid afterwards (selection within the rows and columns, offset not past the
    selection);
  - rendering again with the resulting state doesn't change the state.

Across all runs, the `differential` target executed over 400,000 inputs and the `widgets` target
over 240,000. The final runs, after the fixes, were clean: 93,708 inputs for `differential` (4
minutes) and 140,604 for `widgets` (2.5 minutes), with no failures or hangs.

Inputs that panic in ratatui's table but not in this crate are accepted by the differential target,
because this crate fixes some of those panics. The reverse, a panic only in this crate, is reported.

## Bugs fixed in the table

All four bugs are in ratatui's `Table` too: the code was ported unchanged, so this crate inherited
them. Line numbers are given for `ratatui-widgets` 0.3.2 (`src/table.rs`) and for ratatui's main
branch (`ratatui-widgets/src/table.rs`).

### 1. Arithmetic overflow with huge sizes

**Symptom.** Rendering a table panics with `attempt to add with overflow` or `attempt to multiply
with overflow` in debug builds. In release builds, overflow checks are off by default, so the
arithmetic wraps around and the table computes wrong positions instead of crashing.

**Triggers.** All of these panic in ratatui's `Table` 0.3.2 (checked while writing this document):

```rust
// A row taller than the space left in the coordinate space, in an area that doesn't start at y = 0
Table::new([Row::new(["a"]).height(u16::MAX)], [Constraint::Length(1)])
    .render(Rect::new(0, 1, 5, 5), &mut buf);

// Two rows whose heights add up to more than u16::MAX
Table::new(
    [Row::new(["a"]).height(3), Row::new(["b"]).height(u16::MAX)],
    [Constraint::Length(1)],
)
.render(Rect::new(0, 0, 5, 5), &mut buf);

// A cell spanning 3 columns, with a column spacing so large that 2 gaps overflow a u16
Table::new([Row::new([Cell::from("a").column_span(3)])], [Constraint::Length(1); 3])
    .column_spacing(u16::MAX)
    .render(Rect::new(0, 0, 5, 1), &mut buf);
```

**Cause.** Row positions, the total height of the visible rows, and the width of spanning cells are
all `u16` values computed with plain `+` and `*`:

| Code | 0.3.2 | main |
| --- | --- | --- |
| `render_rows`: `let y = area.y + y_offset + row.top_margin;` and `(y + row.height)` | 868 | 895 |
| `render_rows`: `y_offset += row.height_with_margin();` | 881 | 908 |
| `visible_rows`: `if height + item.height > area.height` and `height += item.height_with_margin();` | 1008, 1011 | 1038, 1041 |
| `get_cell_area`: `all_columns_width + (n_columns_taken - 1) * column_spacing` | 984 | 1011 |

`Row::height_with_margin` already uses saturating arithmetic, but the sums across rows don't.

**Fix.**

- Row positions and heights use saturating arithmetic, so a row that extends past the end of the
  coordinate space is clipped instead of wrapping around to the top:

  ```rust
  let y = area.y.saturating_add(y_offset).saturating_add(row.top_margin);
  let height = y.saturating_add(row.height).min(area.bottom()).saturating_sub(y);
  // ...
  y_offset = y_offset.saturating_add(row.height_with_margin());
  ```

- `visible_rows` accumulates the height with `saturating_add` (the rest of the function already
  did).
- `get_cell_area` sums the widths and spacing as `u32`, then clamps the result to `u16::MAX`:

  ```rust
  let width = all_columns_width + (n_columns_taken - 1) * u32::from(column_spacing);
  let width = u16::try_from(width).unwrap_or(u16::MAX);
  ```

  The clamped width is then clipped to the row by fix 2.

**Regression tests.** `tall_rows_do_not_overflow` and
`column_span_with_large_spacing_does_not_overflow`.

### 2. Spanning cells drawn past the right edge

**Symptom.** A cell spanning several columns can be drawn past the right edge of the table, over
whatever is next to it.

**Trigger.** Three columns of `Length(3)` with a spacing of 3, and a cell spanning all three, in a
table only 4 columns wide. The table is rendered into the left 4 columns of a 10-column buffer
filled with `.`:

```rust
let rows = [Row::new([Cell::from("abcdefghij").column_span(3)])];
let table = Table::new(rows, [Constraint::Length(3); 3]).column_spacing(3);
```

```text
ratatui 0.3.2:        abcdef....   (6 cells drawn by a 4-cell-wide table)
ratatui-widgettable:  abcd......
```

**Cause.** `get_cell_area` computes a spanning cell's width by adding the widths of the columns it
covers plus the full column spacing for every gap between them. When the columns don't fit, the
layout squeezes the gaps as well, but the spacing is still counted in full. In the example, the
layout gives all three columns a width of 0, at x = 0, 3 and 4 (the second gap is squeezed to 1),
but the cell's width is computed as 0 + 0 + 0 + 2 × 3 = 6, in a table 4 wide. `render_row_cells`
renders the cell into that area without clipping it (0.3.2 line 931, main line 958).

**Fix.** The cell's area is intersected with the row's area before rendering:

```rust
let area_to_render = Rect::new(new_x, row_area.y, cell_area.width, row_area.height)
    .intersection(row_area);
```

**Regression test.** `spanning_cell_is_clipped_to_table`.

### 3. Highlight symbol drawn past the right edge

**Symptom.** A highlight symbol wider than the table is drawn past the table's right edge.

**Trigger.** A 4-column-wide table with a selected row and a 6-character highlight symbol, rendered
into the left 4 columns of a 10-column buffer filled with `.`:

```rust
let table = Table::new([Row::new(["a"])], [Constraint::Length(1)]).highlight_symbol(">>>>>>");
// rendered with TableState::new().with_selected(Some(0))
```

```text
ratatui 0.3.2:        >>>>>>....
ratatui-widgettable:  >>>>......
```

**Cause.** The selection column's width is the width of the highlight symbol. `set_selection_style`
builds the symbol's area as `Rect { width: selection_width, ..row_area }` (0.3.2 line 945, main line
972) without limiting it to the row, and renders the symbol into it.

**Fix.** The selection area is intersected with the row's area.

**Regression test.** `wide_highlight_symbol_is_clipped_to_table`.

### 4. Rows pushed out by a margin drawn outside the table

**Symptom.** A cell in a row that is pushed below the table by its top margin can draw a line
outside of the table. If that position is outside the buffer too, rendering panics with `index
outside of buffer`.

**Trigger.** A table 1 line tall, whose only row has a top margin of 2, and a cell that renders with
`&str` (or any widget that ignores the height of its area):

```rust
let rows = [Row::new([Cell::from_fn(|area, buf| {
    buf.set_string(area.x, area.y, "x", Style::new());
})])
.top_margin(2)];
let table = Table::new(rows, [Constraint::Length(4)]);
```

Before the fix, the closure received an area 4 cells wide and 0 lines tall at `y = 2`, below the
1-line table, and drew `x` there.

**Cause.** When there's space left after the rows that fit, `visible_rows` includes one more partial
row. `render_rows` then places that row at `y = area.y + y_offset + row.top_margin`, which the top
margin can push past the bottom of the table. The height is clipped to 0, but the `x`, `width` and
`y` are kept, so each cell gets a zero-height area outside the table.

This is harmless in ratatui, because its cells always contain `Text`, and `Text` only draws as many
lines as its area is tall. A version of the trigger above using a text cell draws nothing in ratatui
0.3.2 (checked while writing this document). It matters here, because cells can contain any widget,
and some widgets only look at the area's width. For example, the `&str` widget calls
`buf.set_stringn(area.x, area.y, self, area.width, ...)`, which writes a line at `area.y` even when
the area is 0 lines tall. With a zero *width* it writes nothing, so only zero-height areas are a
problem.

After fix 1, the `y` of a row whose position would overflow saturates at `u16::MAX`, which is
outside any buffer, so the fuzzer found this as both stray drawing and `index outside of buffer`
panics.

**Fix.** `Cell::render` doesn't render cells whose area is empty:

```rust
if area.is_empty() {
    return;
}
```

A consequence worth knowing: closures created with `Cell::from_fn` are never called with an empty
area.

**Regression test.** `row_pushed_out_by_margin_is_not_rendered`.

### Why normal tables are unaffected

Each fix only changes the result when something doesn't fit:

- Saturating arithmetic gives the same result as `+` whenever `+` doesn't overflow.
- Clipping to the row (fixes 2 and 3) does nothing when the cell or highlight symbol is already
  inside the row.
- Empty areas (fix 4) have nothing visible to draw for text cells, which is all ratatui's table
  supports.

The `differential` fuzz target checks this directly: for text-only tables, the output and state are
identical to ratatui's `Table`. The remaining inputs where the two differ are the ones where ratatui
panics and this crate doesn't.

## Problems in ratatui's layout engine

The table uses `Layout` from `ratatui-core` twice per render: once to split its area vertically into
the header, rows and footer, and once to compute the column widths. `Layout` is built on
[kasuari], an implementation of the Cassowary constraint solving algorithm. Both problems below are
in that engine. They affect ratatui's `Table` in exactly the same way, and can't be fixed in this
crate without replacing the layout code.

### Background: why the results vary

kasuari stores the solver's state (its rows, variables and constraints) in `hashbrown` hash maps.
`hashbrown`'s default hasher is `foldhash`, whose `RandomState` gives every hash map a different
seed, derived from a stack address and a per-thread counter. So the iteration order of kasuari's
maps can differ between any two `Layout::split` calls, even with the same input in the same process.

When the constraints can all be satisfied, the solution is unique and the order doesn't matter. The
problems below only appear when constraints conflict or the values are extreme, and there the result
depends on the order in which the solver visits its rows. That is consistent with everything
observed, but I haven't traced kasuari's pivoting code in detail, so treat it as the likely cause
rather than a proven one.

The `layout-cache` feature of `ratatui-core` changes how this shows up. The feature caches
`Layout::split` results, per thread with the `std` feature and in one global cache without it. It's enabled by default by the `ratatui` crate, so most
applications have it. With the cache, a layout is solved once, and later calls return the cached
result, so the output is stable within a process (unless the entry is evicted). It can still differ
between processes.

### A. Header and footer sizes chosen at random

**Symptom.** When a table's header and footer (including their margins) can't both fit in the table,
the space goes to the header in some calls and to the footer in others, for the same input.

**When it happens.** The table splits its area vertically with these constraints:

```rust
[
    Length(header_top_margin), Length(header_height), Length(header_bottom_margin),
    Min(0), // the rows
    Length(footer_top_margin), Length(footer_height), Length(footer_bottom_margin),
]
```

These conflict when the header and footer need more lines than the table has. The results below
come from 1,000 calls to `Layout::split` per case, without the layout cache, repeated in 3 separate
processes. They are heights in the order of the constraints above.

| Header (top margin, height, bottom margin) | Footer (top margin, height, bottom margin) | Table height | Result |
| --- | --- | --- | --- |
| 0, 3, 0 | 0, 2, 0 | 4 | Always `[0, 2, 0, 0, 0, 2, 0]` |
| 0, 3, 0 | 0, 3, 0 | 5 | Always `[0, 2, 0, 0, 0, 3, 0]` |
| 0, 4, 0 | 0, 4, 0 | 3 | Always `[0, 0, 0, 0, 0, 3, 0]` |
| 0, 6, 0 | 0, 4, 0 | 5 | Always `[0, 1, 0, 0, 0, 4, 0]` |
| 1, 3, 1 | 0, 2, 0 | 5 | Always `[1, 1, 1, 0, 0, 2, 0]` |
| **2, 3, 2** | **1, 2, 1** | **5** | About half `[2, 1, 2, 0, 0, 0, 0]` (header gets the space), half `[1, 0, 1, 0, 1, 1, 1]` (footer gets it) |
| **5, 5, 5** | **3, 3, 3** | **10** | About half `[3, 4, 3, 0, 0, 0, 0]`, half `[0, 1, 0, 0, 3, 3, 3]` |

In these tests it only happened when both the header and the footer had margins. The sizes involved
are small: a 3-line header with 2-line margins and a 2-line footer with 1-line margins, in a table 5
lines tall. This can happen in a real application when the terminal is resized to be very small.

With the layout cache enabled, each case gave the same result for all 1,000 calls in a process, but
the 2, 3, 2 / 1, 2, 1 case gave the header the space in some processes and the footer in others.

**Reproduction** (`ratatui-core = "0.1.2"` with default features, so without the layout cache):

```rust
use std::collections::BTreeMap;

use ratatui_core::layout::Constraint::{Length, Min};
use ratatui_core::layout::{Layout, Rect};

fn main() {
    // The constraints Table uses for a header (margins 2, height 3) and footer (margins 1, height
    // 2) around the rows, in a table 5 lines tall
    let constraints = [Length(2), Length(3), Length(2), Min(0), Length(1), Length(2), Length(1)];
    let mut results = BTreeMap::<Vec<u16>, usize>::new();
    for _ in 0..1000 {
        let areas = Layout::vertical(constraints).split(Rect::new(0, 0, 10, 5));
        *results.entry(areas.iter().map(|a| a.height).collect()).or_default() += 1;
    }
    println!("{results:?}");
}
```

Output from three runs:

```text
{[1, 0, 1, 0, 1, 1, 1]: 518, [2, 1, 2, 0, 0, 0, 0]: 482}
{[1, 0, 1, 0, 1, 1, 1]: 506, [2, 1, 2, 0, 0, 0, 0]: 494}
{[1, 0, 1, 0, 1, 1, 1]: 517, [2, 1, 2, 0, 0, 0, 0]: 483}
```

**How the fuzz targets deal with it.** The `differential` target skips inputs where the header and
footer don't fit (`TableSpec::header_and_footer_fit` in `fuzz/src/lib.rs`), because the two
implementations can legitimately give different answers. The `widgets` target doesn't require two
renders to produce the same buffer, only the same state.

### B. Column layout hangs or panics

**Symptom.** With extreme column constraints, computing the column widths can either never finish
or panic with:

```text
failed to split: InternalSolverError(ObjectiveUnbounded)
```

from `Layout::split` (`ratatui-core` 0.1.2, `src/layout/layout.rs:794`). Like problem A, it only
happens on some calls for the same input.

**How it was found.** The `widgets` fuzz target stopped making progress. Sampling a stuck process
showed it had been inside kasuari's `Solver::add_constraint` and `Solver::optimize`, called from
`Table::get_column_widths`, for over 4 minutes. That input wasn't saved, so the two layouts the
table uses for its columns were then called directly with random constraints, spacing, flex, width
and selection width, logging each case before running it. Within 2 minutes, 6 of the 8 search
processes were stuck on a single case, and 2 had panicked.

**How often.** Each case found by the search, repeated in separate processes:

| Case | Runs | Hung (killed after 5 seconds) | Panicked | Finished |
| --- | --- | --- | --- | --- |
| Reproduction below | 60 | 10 | 3 | 47 (in under 3 ms) |
| Same case, via the table's two layouts | 40 | 3 | 2 | 35 |
| Another case with spacing 65535 and huge ratios | 40 | 0 | 4 | 36 |
| Three other cases | 40 each | 0 | 0 | 40 each |

The three cases that always finished in these runs had each hung or panicked once during the search,
so they fail too, just less often.

**What triggers it.** Every failing case had several constraint values far larger than the width
available (tens of thousands of cells, in a table less than 80 wide). It isn't specific to one kind
of constraint: the reproduction below uses only `Fill`, `Min`, `Max` and `Length`. Capping every
value in that case at 10,000, 1,000, 200 or 100 made it succeed in all 100 runs at each cap. Plain
extreme values on their own weren't enough either: 10 columns that are all `Length(u16::MAX)`, all
`Fill(u16::MAX)`, alternating `Min(u16::MAX)` and `Max(u16::MAX)`, all `Ratio(u32::MAX, 3)`, or
spaced by `u16::MAX`, each finished in a few milliseconds with every flex mode. So realistic column
widths shouldn't trigger it, but widths computed from data (e.g. the length of the longest value in
a column) could reach these sizes.

**Reproduction** (`ratatui-core = "0.1.2"`; run it several times, as most runs succeed):

```rust
use ratatui_core::layout::Constraint::{Fill, Length, Max, Min};
use ratatui_core::layout::{Flex, Layout, Rect};

fn main() {
    let widths = [
        Max(36), Fill(17949), Fill(65535), Fill(65535), Fill(28),
        Min(43850), Fill(38208), Fill(43), Length(0), Length(0),
    ];
    let areas = Layout::horizontal(widths).flex(Flex::Start).split(Rect::new(29, 0, 31, 1));
    println!("{:?}", areas.iter().map(|a| a.width).collect::<Vec<_>>());
}
```

**How the fuzz targets deal with it.** Both targets limit column constraint values and the column
spacing to at most 255 (`TableSpec::stable_layout` in `fuzz/src/lib.rs`), which keeps the column layout
realistic while still covering columns that don't fit.

### Reporting upstream

Both problems are in ratatui's layout engine and could be reported to [ratatui] (and possibly
[kasuari], which may be where they need to be fixed):

1. **`Layout::split` is nondeterministic when constraints conflict.** Include the header/footer
   reproduction and its output, and the likely cause (per-map random hash seeds changing the
   solver's pivot order). A possible fix is a fixed hasher for kasuari's maps, which would at least
   make the result deterministic.
2. **`Layout::split` can hang or panic with `InternalSolverError(ObjectiveUnbounded)` with extreme
   constraint values.** Include the column reproduction, and note that it only fails on some runs.

The four table bugs could also be fixed in ratatui's `Table` with the same changes.

## Other observations

The released `ratatui-widgets` 0.3.2 differs from ratatui's main branch when the selected row is
taller than the table. In 0.3.2, `visible_rows` only scrolls down until the selected row fits, which
can move the offset past the selection, for example `offset = 1` with row 0 selected, so the
selected row isn't shown. Main adds a step that scrolls back up until the selected row is visible.
This crate was ported from main, so it has the fix. This is why the differential target compares
against main rather than 0.3.2.

## Reproducing the findings

- **Table bugs:** `cargo test --test table`, which includes the five regression tests.
- **Fuzzing:** see `fuzz/README.md`, e.g. `cd fuzz && cargo +nightly fuzz run -a differential`.
- **Layout problems:** create a crate depending on `ratatui-core = "0.1.2"` and run the reproductions
  above in release mode. For the column reproduction, run it in a loop with a timeout, as most runs
  succeed.

[cargo-fuzz]: https://github.com/rust-fuzz/cargo-fuzz
[kasuari]: https://github.com/ratatui/kasuari
[ratatui]: https://github.com/ratatui/ratatui
