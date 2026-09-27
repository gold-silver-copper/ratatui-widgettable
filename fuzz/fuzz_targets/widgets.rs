//! Renders random tables whose cells contain widgets, closures and nested tables, with a random
//! scroll padding, and checks that:
//!
//! - rendering doesn't panic (including arithmetic overflow, when run with `-a`)
//! - nothing is drawn outside of the table's area
//! - closure cells are only given areas inside the table's area
//! - the state is valid after rendering
//! - rendering again with the resulting state doesn't change the state
//!
//! Rendering again isn't required to produce the same buffer: when the header, footer and rows
//! can't possibly fit (e.g. margins of thousands of lines), ratatui's layout solver can resolve the
//! conflicting constraints differently between calls. That is a property of the layout engine
//! rather than the table, and doesn't happen with realistic sizes.
#![no_main]

use std::cell::RefCell;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Style};
use ratatui_core::text::{Line, Text};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::gauge::Gauge;
use ratatui_widgets::paragraph::{Paragraph, Wrap};
use ratatui_widgets::sparkline::Sparkline;
use ratatui_widgettable::{Cell, Row, Table, TableState};
use ratatui_widgettable_fuzz::{MAX_ROWS, RowSpec, TableSpec, configure_table, row, style};

#[derive(Debug, Arbitrary)]
struct Input {
    table: TableSpec,
    small_numbers: bool,
    /// The content of each cell, assigned to the cells in order (repeating if needed)
    contents: Vec<Content>,
    /// The spec used for cells containing a nested table
    nested: TableSpec,
    scroll_padding: usize,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum Content {
    Text,
    Paragraph {
        wrap: bool,
        bordered: bool,
        scroll: (u16, u16),
    },
    Gauge(u8),
    Sparkline,
    Block,
    Nested,
    Probe,
}

/// The areas given to closure cells, for checking that they are inside the table's area.
type Probes = RefCell<Vec<Rect>>;

const SENTINEL: &str = "¤";
const SENTINEL_STYLE: Style = Style::new().fg(Color::Indexed(201)).bg(Color::Indexed(202));
const MARGIN: u16 = 3;

fuzz_target!(|input: Input| {
    let Input {
        table: spec,
        small_numbers,
        contents,
        nested,
        scroll_padding,
    } = input;
    // percentages over 100 are documented to panic when creating the table
    // percentages over 100 are documented to panic when creating the table, and extreme column
    // constraints can make ratatui's layout solver hang or panic
    let mut spec = spec.bounded().valid_percentages().stable_layout();
    let mut nested = nested.bounded().valid_percentages().stable_layout();
    let mut scroll_padding = scroll_padding;
    if small_numbers {
        spec = spec.small_numbers();
        nested = nested.small_numbers();
        scroll_padding %= 8;
    }
    let probes = Probes::default();

    // A small area inside a larger buffer, so that drawing outside the area can be detected
    let (x, y, width, height) = spec.area.bounded();
    let area = Rect::new(x % 8 + MARGIN, y % 8 + MARGIN, width, height);
    let buffer_area = Rect::new(0, 0, area.right() + MARGIN, area.bottom() + MARGIN);

    let mut content = contents.iter().copied().cycle();
    let mut next_row = |row_spec| build_row(row_spec, &mut content, &nested, &probes);

    let mut table = Table::default().rows(spec.rows.iter().map(&mut next_row).collect::<Vec<_>>());
    if let Some(header) = &spec.header {
        table = table.header(next_row(header));
    }
    if let Some(footer) = &spec.footer {
        table = table.footer(next_row(footer));
    }
    let table = configure_table!(ours, table, &spec).scroll_padding(scroll_padding);

    let mut state = TableState::new()
        .with_offset(spec.state.offset)
        .with_selected(spec.state.selected)
        .with_selected_column(spec.state.selected_column);

    let render = |state: &mut TableState| {
        let mut buf = Buffer::filled(
            buffer_area,
            ratatui_core::buffer::Cell::new(SENTINEL)
                .set_style(SENTINEL_STYLE)
                .clone(),
        );
        StatefulWidget::render(&table, area, &mut buf, state);
        buf
    };

    let first = render(&mut state);
    let first_state = state;
    check_outside_area_untouched(&first, area);
    check_probes(&probes, area);
    if !inner_area(&spec, area).is_empty() {
        check_state(&spec, first_state);
    }

    render(&mut state);
    assert_eq!(first_state, state, "state changed when rendering again");
});

/// A row with the heights, margins and styles from `row_spec`, whose cells contain the next
/// widgets from `content`.
fn build_row<'a>(
    row_spec: &'a RowSpec,
    content: &mut impl Iterator<Item = Content>,
    nested: &'a TableSpec,
    probes: &'a Probes,
) -> Row<'a> {
    let text_row = row!(ours, row_spec);
    let cells = row_spec.cells.iter().map(|cell_spec| {
        let text = cell_spec.text.as_str();
        let cell = match content.next().unwrap_or(Content::Text) {
            Content::Text => Cell::from(text),
            Content::Paragraph {
                wrap,
                bordered,
                scroll,
            } => {
                let mut paragraph = Paragraph::new(text).scroll(scroll);
                if wrap {
                    paragraph = paragraph.wrap(Wrap { trim: true });
                }
                if bordered {
                    paragraph = paragraph.block(Block::bordered().title(text));
                }
                Cell::new(paragraph)
            }
            Content::Gauge(percent) => Cell::new(
                Gauge::default()
                    .percent(u16::from(percent.min(100)))
                    .label(text),
            ),
            Content::Sparkline => Cell::new(Sparkline::default().data(text.bytes().map(u64::from))),
            Content::Block => Cell::new(Block::bordered().title(text)),
            Content::Nested => Cell::new(nested_table(nested, probes)),
            Content::Probe => probe(probes),
        };
        cell.column_span(cell_spec.column_span)
            .style(style!(ours, cell_spec.style))
    });
    // keep the heights, margins and style of the text row
    text_row.cells(cells.collect::<Vec<_>>())
}

/// A text-only table from `spec`, with a probe in the first cell of each row.
fn nested_table<'a>(spec: &'a TableSpec, probes: &'a Probes) -> Table<'a> {
    let rows = spec.rows.iter().take(MAX_ROWS / 4).map(|row_spec| {
        let row = row!(ours, row_spec);
        let mut cells = vec![probe(probes)];
        cells.extend(
            row_spec
                .cells
                .iter()
                .map(|c| Cell::from(Text::from(c.text.as_str()))),
        );
        row.cells(cells)
    });
    configure_table!(ours, Table::default().rows(rows), spec)
}

/// A cell which records the area it is rendered into, and fills it.
fn probe(probes: &Probes) -> Cell<'_> {
    Cell::from_fn(move |area, buf| {
        probes.borrow_mut().push(area);
        // this panics if the area is outside the buffer
        for position in area.positions() {
            buf[position].set_symbol("p");
        }
        // `Line` clips to the area, unlike the `&str` widget which ignores the size of the area
        Line::from("probe").render(area, buf);
    })
}

/// The area inside the table's block, computed the same way as the table does. When this is empty,
/// the table returns early without updating the state.
fn inner_area(spec: &TableSpec, area: Rect) -> Rect {
    use ratatui_widgets::block::Padding;
    use ratatui_widgets::borders::Borders;
    spec.block.as_ref().map_or(area, |block| {
        let [left, right, top, bottom] = block.padding.map(u16::from);
        Block::new()
            .borders(Borders::from_bits_truncate(block.borders))
            .title(block.title.as_str())
            .padding(Padding::new(left, right, top, bottom))
            .inner(area)
    })
}

fn check_outside_area_untouched(buf: &Buffer, area: Rect) {
    for position in buf.area.positions() {
        if !area.contains(position) {
            let cell = &buf[position];
            assert!(
                cell.symbol() == SENTINEL && cell.style() == SENTINEL_STYLE,
                "cell at {position} outside of the table area {area} was changed: {cell:?}"
            );
        }
    }
}

fn check_probes(probes: &Probes, area: Rect) {
    for probe in probes.borrow_mut().drain(..) {
        assert!(
            probe.is_empty() || area.union(probe) == area,
            "closure cell was given {probe} which is outside of the table area {area}"
        );
    }
}

fn check_state(spec: &TableSpec, state: TableState) {
    let rows = spec.rows.len();
    if rows == 0 {
        assert_eq!(state.selected(), None, "selection with no rows");
        return;
    }
    if let Some(selected) = state.selected() {
        assert!(selected < rows, "selected row {selected} of {rows} rows");
        assert!(
            state.offset() <= selected,
            "offset {} is after the selected row {selected}",
            state.offset()
        );
    }
    assert!(
        state.offset() < rows,
        "offset {} of {rows} rows",
        state.offset()
    );
    let columns = spec
        .rows
        .iter()
        .chain(&spec.header)
        .chain(&spec.footer)
        .map(|r| r.cells.len())
        .max()
        .unwrap_or_default();
    if let Some(column) = state.selected_column() {
        assert!(column < columns, "selected column {column} of {columns}");
    }
}
