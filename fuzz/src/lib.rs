//! Shared code for the fuzz targets.
//!
//! The same [`TableSpec`] is turned into a table using several implementations of the table widget
//! (ratatui-widgettable and ratatui's main branch). Each
//! implementation uses its own copy of the ratatui types, so the tables are built by the
//! [`render_table!`] macro, which is expanded once per implementation.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

use arbitrary::Arbitrary;

/// ratatui-widgettable
pub mod ours {
    pub use ratatui_core as core;
    pub use ratatui_widgets::block::{Block, Padding};
    pub use ratatui_widgets::borders::Borders;
    pub use ratatui_widgettable::*;
}

/// The commit on ratatui's main branch that ratatui-widgettable was ported from
pub mod main {
    pub use main_core as core;
    pub use main_widgets::block::{Block, Padding};
    pub use main_widgets::borders::Borders;
    pub use main_widgets::table::*;
}

// Limits which keep each fuzz iteration fast. They are well above the sizes needed to exercise the
// interesting cases (scrolling, clipping, spans, etc.).
pub const MAX_ROWS: usize = 48;
pub const MAX_CELLS: usize = 10;
pub const MAX_COLUMNS: usize = 10;
pub const MAX_TEXT: usize = 24;
pub const MAX_WIDTH: u16 = 80;
pub const MAX_HEIGHT: u16 = 48;

#[derive(Debug, Clone, Arbitrary)]
pub struct TableSpec {
    pub rows: Vec<RowSpec>,
    pub header: Option<RowSpec>,
    pub footer: Option<RowSpec>,
    pub widths: Vec<ConstraintSpec>,
    pub column_spacing: u16,
    pub flex: FlexSpec,
    pub highlight_spacing: HighlightSpacingSpec,
    pub highlight_symbol: String,
    pub block: Option<BlockSpec>,
    pub style: StyleSpec,
    pub row_highlight_style: StyleSpec,
    pub column_highlight_style: StyleSpec,
    pub cell_highlight_style: StyleSpec,
    pub area: AreaSpec,
    pub state: StateSpec,
}

#[derive(Debug, Clone, Arbitrary)]
pub struct RowSpec {
    pub cells: Vec<CellSpec>,
    pub height: u16,
    pub top_margin: u16,
    pub bottom_margin: u16,
    pub style: StyleSpec,
}

#[derive(Debug, Clone, Arbitrary)]
pub struct CellSpec {
    pub text: String,
    pub column_span: u16,
    pub alignment: Option<AlignmentSpec>,
    pub style: StyleSpec,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub enum ConstraintSpec {
    Min(u16),
    Max(u16),
    Length(u16),
    Percentage(u16),
    Ratio(u32, u32),
    Fill(u16),
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub enum FlexSpec {
    Legacy,
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub enum HighlightSpacingSpec {
    Always,
    WhenSelected,
    Never,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub enum AlignmentSpec {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Arbitrary)]
pub struct BlockSpec {
    pub borders: u8,
    pub title: String,
    pub padding: [u8; 4],
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub struct StyleSpec {
    pub fg: Option<u8>,
    pub bg: Option<u8>,
    pub add_modifier: u16,
    pub sub_modifier: u16,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub struct AreaSpec {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl AreaSpec {
    /// Returns (x, y, width, height), limiting the size so that buffers stay small. The position
    /// is not limited, so that areas near the edge of the `u16` coordinate space are tested.
    pub fn bounded(self) -> (u16, u16, u16, u16) {
        (
            self.x,
            self.y,
            self.width % (MAX_WIDTH + 1),
            self.height % (MAX_HEIGHT + 1),
        )
    }
}

#[derive(Debug, Clone, Copy, Arbitrary)]
pub struct StateSpec {
    pub offset: usize,
    pub selected: Option<usize>,
    pub selected_column: Option<usize>,
}

impl TableSpec {
    /// Truncates the collections in the spec so that each iteration stays fast.
    pub fn bounded(mut self) -> Self {
        self.rows.truncate(MAX_ROWS);
        self.widths.truncate(MAX_COLUMNS);
        for row in self
            .rows
            .iter_mut()
            .chain(self.header.iter_mut())
            .chain(self.footer.iter_mut())
        {
            row.cells.truncate(MAX_CELLS);
            for cell in &mut row.cells {
                cell.text = truncate(&cell.text);
            }
        }
        self.highlight_symbol = truncate(&self.highlight_symbol);
        if let Some(block) = &mut self.block {
            block.title = truncate(&block.title);
        }
        self
    }
}

impl TableSpec {
    /// Limits the numbers in the spec to small values, which are more realistic than random `u16`
    /// values and exercise the scrolling logic without hitting the overflow panics in ratatui's
    /// table (which make the differential comparison skip the input).
    pub fn small_numbers(mut self) -> Self {
        for row in self
            .rows
            .iter_mut()
            .chain(self.header.iter_mut())
            .chain(self.footer.iter_mut())
        {
            row.height %= 6;
            row.top_margin %= 3;
            row.bottom_margin %= 3;
            for cell in &mut row.cells {
                cell.column_span %= 5;
            }
        }
        for width in &mut self.widths {
            *width = match *width {
                ConstraintSpec::Min(v) => ConstraintSpec::Min(v % 40),
                ConstraintSpec::Max(v) => ConstraintSpec::Max(v % 40),
                ConstraintSpec::Length(v) => ConstraintSpec::Length(v % 40),
                ConstraintSpec::Percentage(v) => ConstraintSpec::Percentage(v % 101),
                ConstraintSpec::Ratio(a, b) => ConstraintSpec::Ratio(a % 10, b % 10),
                ConstraintSpec::Fill(v) => ConstraintSpec::Fill(v % 5),
            };
        }
        self.column_spacing %= 4;
        if let Some(block) = &mut self.block {
            block.padding = block.padding.map(|p| p % 3);
        }
        let rows = self.rows.len() + 2;
        self.state.offset %= rows;
        self.state.selected = self.state.selected.map(|s| s % rows);
        self.state.selected_column = self.state.selected_column.map(|s| s % (MAX_CELLS + 2));
        self
    }
}

impl TableSpec {
    /// Limits the column constraints and spacing, as values tens of thousands of cells wide can
    /// make ratatui's layout solver hang or panic on some runs. See `docs/fuzzing-findings.md`.
    pub fn stable_layout(mut self) -> Self {
        for width in &mut self.widths {
            *width = match *width {
                ConstraintSpec::Min(v) => ConstraintSpec::Min(v % 256),
                ConstraintSpec::Max(v) => ConstraintSpec::Max(v % 256),
                ConstraintSpec::Length(v) => ConstraintSpec::Length(v % 256),
                ConstraintSpec::Percentage(v) => ConstraintSpec::Percentage(v),
                ConstraintSpec::Ratio(a, b) => ConstraintSpec::Ratio(a % 256, b % 256),
                ConstraintSpec::Fill(v) => ConstraintSpec::Fill(v % 256),
            };
        }
        self.column_spacing %= 256;
        self
    }

    /// Whether the header and footer (including their margins) fit in the table.
    ///
    /// When they don't, the constraints passed to ratatui's layout solver conflict, and it resolves
    /// them nondeterministically (e.g. giving the space to the header in one run and the footer in
    /// another, even within one process). This is a property of ratatui's layout engine rather than
    /// the table, so such inputs can't be compared.
    pub fn header_and_footer_fit(&self) -> bool {
        let size = |row: &Option<RowSpec>| {
            row.as_ref().map_or(0, |r| {
                u32::from(r.height) + u32::from(r.top_margin) + u32::from(r.bottom_margin)
            })
        };
        // at most 2 lines for the borders and title, plus the padding
        let block = self
            .block
            .as_ref()
            .map_or(0, |b| 2 + u32::from(b.padding[2]) + u32::from(b.padding[3]));
        let (_, _, _, height) = self.area.bounded();
        size(&self.header) + size(&self.footer) + block <= u32::from(height)
    }

    /// Limits percentages to 100, as larger percentages are documented to panic.
    pub fn valid_percentages(mut self) -> Self {
        for width in &mut self.widths {
            if let ConstraintSpec::Percentage(p) = width {
                *p %= 101;
            }
        }
        self
    }
}

pub fn truncate(s: &str) -> String {
    s.chars().take(MAX_TEXT).collect()
}

/// A snapshot of every cell in a buffer, as strings so that buffers from different versions of
/// `ratatui-core` can be compared.
pub type BufferSnapshot = Vec<String>;

/// The state after rendering: (offset, selected, selected column).
pub type StateSnapshot = (usize, Option<usize>, Option<usize>);

/// The result of rendering a table twice, the second time with the state from the first render.
pub type RenderResult = [(BufferSnapshot, StateSnapshot); 2];

/// Runs `f`, returning `Err` with the panic message if it panics.
///
/// The panic hook installed by libfuzzer aborts the process before unwinding, so it is replaced
/// with a silent hook while `f` runs. Panics are then compared between the implementations.
pub fn catch_panic<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    panic::set_hook(hook);
    result.map_err(|payload| panic_message(&*payload))
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// Builds the style for `$spec` using the types of the implementation `$m`.
#[macro_export]
macro_rules! style {
    ($m:ident, $spec:expr) => {{
        use $crate::$m::core::style::{Color, Modifier, Style};
        let spec: $crate::StyleSpec = $spec;
        let mut style = Style::new()
            .add_modifier(Modifier::from_bits_truncate(spec.add_modifier))
            .remove_modifier(Modifier::from_bits_truncate(spec.sub_modifier));
        if let Some(fg) = spec.fg {
            style = style.fg(Color::Indexed(fg));
        }
        if let Some(bg) = spec.bg {
            style = style.bg(Color::Indexed(bg));
        }
        style
    }};
}

/// Builds a row for `$spec` using the implementation `$m`.
#[macro_export]
macro_rules! row {
    ($m:ident, $spec:expr) => {{
        use $crate::$m::core::layout::Alignment;
        use $crate::$m::core::text::Text;
        use $crate::$m::{Cell, Row};
        let spec: &$crate::RowSpec = $spec;
        let cells = spec.cells.iter().map(|cell| {
            let mut text = Text::from(cell.text.clone());
            if let Some(alignment) = cell.alignment {
                text = text.alignment(match alignment {
                    $crate::AlignmentSpec::Left => Alignment::Left,
                    $crate::AlignmentSpec::Center => Alignment::Center,
                    $crate::AlignmentSpec::Right => Alignment::Right,
                });
            }
            Cell::from(text)
                .column_span(cell.column_span)
                .style($crate::style!($m, cell.style))
        });
        Row::new(cells)
            .height(spec.height)
            .top_margin(spec.top_margin)
            .bottom_margin(spec.bottom_margin)
            .style($crate::style!($m, spec.style))
    }};
}

/// Builds a table for `$spec` using the implementation `$m`, with all the settings except the
/// rows, header and footer. This is shared between the fuzz targets.
#[macro_export]
macro_rules! configure_table {
    ($m:ident, $table:expr, $spec:expr) => {{
        use $crate::$m::core::layout::{Constraint, Flex};
        use $crate::$m::{Block, Borders, HighlightSpacing, Padding};
        let spec: &$crate::TableSpec = $spec;
        let widths = spec.widths.iter().map(|w| match *w {
            $crate::ConstraintSpec::Min(v) => Constraint::Min(v),
            $crate::ConstraintSpec::Max(v) => Constraint::Max(v),
            $crate::ConstraintSpec::Length(v) => Constraint::Length(v),
            $crate::ConstraintSpec::Percentage(v) => Constraint::Percentage(v),
            $crate::ConstraintSpec::Ratio(a, b) => Constraint::Ratio(a, b),
            $crate::ConstraintSpec::Fill(v) => Constraint::Fill(v),
        });
        let mut table = $table
            .widths(widths)
            .column_spacing(spec.column_spacing)
            .flex(match spec.flex {
                $crate::FlexSpec::Legacy => Flex::Legacy,
                $crate::FlexSpec::Start => Flex::Start,
                $crate::FlexSpec::End => Flex::End,
                $crate::FlexSpec::Center => Flex::Center,
                $crate::FlexSpec::SpaceBetween => Flex::SpaceBetween,
                $crate::FlexSpec::SpaceEvenly => Flex::SpaceEvenly,
                $crate::FlexSpec::SpaceAround => Flex::SpaceAround,
            })
            .highlight_spacing(match spec.highlight_spacing {
                $crate::HighlightSpacingSpec::Always => HighlightSpacing::Always,
                $crate::HighlightSpacingSpec::WhenSelected => HighlightSpacing::WhenSelected,
                $crate::HighlightSpacingSpec::Never => HighlightSpacing::Never,
            })
            .highlight_symbol(spec.highlight_symbol.clone())
            .style($crate::style!($m, spec.style))
            .row_highlight_style($crate::style!($m, spec.row_highlight_style))
            .column_highlight_style($crate::style!($m, spec.column_highlight_style))
            .cell_highlight_style($crate::style!($m, spec.cell_highlight_style));
        if let Some(block) = &spec.block {
            let [left, right, top, bottom] = block.padding.map(u16::from);
            table = table.block(
                Block::new()
                    .borders(Borders::from_bits_truncate(block.borders))
                    .title(block.title.clone())
                    .padding(Padding::new(left, right, top, bottom)),
            );
        }
        table
    }};
}

/// Builds a text-only table for `$spec` using the implementation `$m`, renders it twice (the
/// second time with the resulting state of the first render), and returns snapshots of the buffer
/// and state after each render.
///
/// `|table| expr` applies extra configuration, for settings that only some implementations
/// support (e.g. `|t| t.scroll_padding(1)`).
#[macro_export]
macro_rules! render_table {
    ($m:ident, $spec:expr, | $t:ident | $extra:expr) => {{
        use $crate::$m::core::buffer::Buffer;
        use $crate::$m::core::layout::Rect;
        use $crate::$m::core::widgets::StatefulWidget;
        use $crate::$m::{Table, TableState};
        let spec: &$crate::TableSpec = $spec;

        let mut table = Table::default().rows(spec.rows.iter().map(|row| $crate::row!($m, row)));
        if let Some(header) = &spec.header {
            table = table.header($crate::row!($m, header));
        }
        if let Some(footer) = &spec.footer {
            table = table.footer($crate::row!($m, footer));
        }
        let table = $crate::configure_table!($m, table, spec);
        let table = {
            let $t = table;
            $extra
        };

        let (x, y, width, height) = spec.area.bounded();
        let area = Rect::new(x, y, width, height);
        let mut state = TableState::new()
            .with_offset(spec.state.offset)
            .with_selected(spec.state.selected)
            .with_selected_column(spec.state.selected_column);
        let mut render = |state: &mut TableState| {
            let mut buf = Buffer::empty(area);
            StatefulWidget::render(&table, area, &mut buf, state);
            // `skip` is deprecated on main, but is still part of the output
            #[allow(deprecated)]
            let cells = buf
                .content()
                .iter()
                .map(|c| {
                    format!(
                        "{:?} {:?} {:?} {:?} {}",
                        c.symbol(),
                        c.fg,
                        c.bg,
                        c.modifier,
                        c.skip
                    )
                })
                .collect::<Vec<_>>();
            let snapshot = (state.offset(), state.selected(), state.selected_column());
            (cells, snapshot)
        };
        let first = render(&mut state);
        let second = render(&mut state);
        [first, second]
    }};
}

/// Panics with a readable report if the results of two implementations differ.
///
/// If both implementations panic, they are considered to behave the same. ratatui-widgettable fixes
/// some overflow panics which exist in ratatui's table, so it is allowed to not panic when the
/// other implementation does (the `widgets` fuzz target checks that the output is sensible).
/// Panics which only happen in ratatui-widgettable are reported.
pub fn assert_same(
    name: &str,
    spec: &TableSpec,
    expected: &Result<RenderResult, String>,
    actual: &Result<RenderResult, String>,
) {
    let (expected, actual) = match (expected, actual) {
        (Ok(expected), Ok(actual)) => (expected, actual),
        (Err(_), _) => return,
        (Ok(_), Err(e)) => panic!("ratatui-widgettable panicked but {name} did not: {e}"),
    };
    let (_, _, width, _) = spec.area.bounded();
    for (render, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        let (expected_buf, expected_state) = expected;
        let (actual_buf, actual_state) = actual;
        assert_eq!(
            expected_state, actual_state,
            "render {render}: state differs from {name} (offset, selected, selected column)"
        );
        if expected_buf != actual_buf {
            let index = expected_buf
                .iter()
                .zip(actual_buf)
                .position(|(e, a)| e != a)
                .unwrap();
            let width = usize::from(width.max(1));
            panic!(
                "render {render}: buffer differs from {name} at ({}, {}) (relative to the area)\n\
                 {name}:               {}\n\
                 ratatui-widgettable: {}\n\
                 {name} screen:\n{}\n\
                 ratatui-widgettable screen:\n{}",
                index % width,
                index / width,
                expected_buf[index],
                actual_buf[index],
                screen(expected_buf, width),
                screen(actual_buf, width),
            );
        }
    }
}

/// Renders the symbols of a buffer snapshot as lines of text.
fn screen(snapshot: &BufferSnapshot, width: usize) -> String {
    snapshot
        .chunks(width)
        .map(|row| {
            let line: String = row
                .iter()
                .map(|cell| {
                    // the snapshot starts with the Debug formatted symbol, e.g. "\"a\""
                    let symbol = cell.split(' ').next().unwrap_or_default();
                    symbol.trim_matches('"').to_string()
                })
                .collect();
            format!("|{line}|")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
