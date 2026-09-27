# Fuzzing

Fuzz targets for `ratatui-widgettable`, using [cargo-fuzz] (requires a nightly toolchain).

```shell
cargo install cargo-fuzz
cd fuzz
cargo +nightly fuzz run -a differential
cargo +nightly fuzz run -a widgets
```

`-a` enables debug assertions, so arithmetic overflow is caught. Add `-- -fork=8` to use more
cores.

## Targets

- **`differential`** renders random text-only tables (rows, header, footer, margins, column spans,
  constraints, flex, blocks, styles, selection, offset and scroll padding) with both this crate and
  the `Table` on ratatui's main branch that it was ported from, and checks that the rendered output
  and resulting `TableState` are identical, on the first render and when rendering again with that
  state.
- **`widgets`** renders random tables whose cells contain paragraphs, gauges, sparklines, blocks,
  nested tables and closures, and checks that rendering doesn't panic, nothing is drawn outside the
  table's area, closures only receive areas inside the table, the state is valid, and rendering
  again doesn't change the state.

## Known limitations

These are described in detail, with reproductions, in
[docs/fuzzing-findings.md](../docs/fuzzing-findings.md).

- When the header and footer don't fit in the table, ratatui's layout solver resolves the
  conflicting constraints nondeterministically (the same input can give the space to the header in
  one run and the footer in the next). The `differential` target skips these inputs, and the
  `widgets` target doesn't require repeated renders to produce the same output.
- Column constraints tens of thousands of cells wide can make ratatui's layout solver hang or panic
  with `InternalSolverError(ObjectiveUnbounded)` on some runs, so column constraints and spacing are
  limited to realistic values.
- ratatui's table panics with arithmetic overflow for some extreme sizes (e.g. rows `u16::MAX`
  lines tall), which this crate fixes. The `differential` target accepts this crate not panicking
  where ratatui does.
