use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{
    self as upstream, Block, Gauge, HighlightSpacing, Paragraph, StatefulWidget, TableState,
    Widget, Wrap,
};
use ratatui_widgettable::{Cell, Row, Table};

// Note: `scroll_padding` is not compared here as it is not in the published `ratatui-widgets`
// 0.3.2 release yet (it is on ratatui's main branch). It is tested separately below using the
// test cases from ratatui's main branch.

/// (cells as (text, column span), height, top margin, bottom margin)
type RowSpec = (Vec<(&'static str, u16)>, u16, u16, u16);

/// Rows of text described in a way that can be turned into both an upstream table and ours.
struct Spec {
    rows: Vec<RowSpec>,
    header: Option<Vec<&'static str>>,
    footer: Option<Vec<&'static str>>,
    widths: Vec<Constraint>,
    column_spacing: u16,
    flex: Flex,
    highlight_spacing: HighlightSpacing,
    block: bool,
}

impl Spec {
    fn simple(rows: usize, height: u16) -> Self {
        let names: &'static [&'static str] = &[
            "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India",
            "Juliet", "Kilo", "Lima",
        ];
        Self {
            rows: (0..rows)
                .map(|i| {
                    let a = names[i % names.len()];
                    let b = names[(i + 3) % names.len()];
                    let c = names[(i + 7) % names.len()];
                    (vec![(a, 1), (b, 1), (c, 1)], height, 0, 0)
                })
                .collect(),
            header: Some(vec!["Col1", "Col2", "Col3"]),
            footer: None,
            widths: vec![
                Constraint::Length(8),
                Constraint::Fill(1),
                Constraint::Length(8),
            ],
            column_spacing: 1,
            flex: Flex::Start,
            highlight_spacing: HighlightSpacing::WhenSelected,
            block: false,
        }
    }

    fn upstream(&self) -> upstream::Table<'static> {
        let rows = self.rows.iter().map(|(cells, height, top, bottom)| {
            upstream::Row::new(
                cells
                    .iter()
                    .map(|(text, span)| upstream::Cell::new(*text).column_span(*span)),
            )
            .height(*height)
            .top_margin(*top)
            .bottom_margin(*bottom)
            .style(Style::new().italic())
        });
        let mut table = upstream::Table::new(rows, self.widths.clone())
            .column_spacing(self.column_spacing)
            .flex(self.flex)
            .highlight_spacing(self.highlight_spacing.clone())
            .highlight_symbol(">>")
            .row_highlight_style(Style::new().reversed())
            .column_highlight_style(Style::new().red())
            .cell_highlight_style(Style::new().on_blue())
            .style(Style::new().green());
        if let Some(header) = &self.header {
            table = table.header(upstream::Row::new(header.clone()).bold().bottom_margin(1));
        }
        if let Some(footer) = &self.footer {
            table = table.footer(upstream::Row::new(footer.clone()).top_margin(1));
        }
        if self.block {
            table = table.block(Block::bordered().title("T"));
        }
        table
    }

    fn ours(&self) -> Table<'static> {
        let rows = self.rows.iter().map(|(cells, height, top, bottom)| {
            Row::new(
                cells
                    .iter()
                    .map(|(text, span)| Cell::from(*text).column_span(*span)),
            )
            .height(*height)
            .top_margin(*top)
            .bottom_margin(*bottom)
            .style(Style::new().italic())
        });
        let mut table = Table::new(rows, self.widths.clone())
            .column_spacing(self.column_spacing)
            .flex(self.flex)
            .highlight_spacing(self.highlight_spacing.clone())
            .highlight_symbol(">>")
            .row_highlight_style(Style::new().reversed())
            .column_highlight_style(Style::new().red())
            .cell_highlight_style(Style::new().on_blue())
            .style(Style::new().green());
        if let Some(header) = &self.header {
            table = table.header(Row::new(header.clone()).bold().bottom_margin(1));
        }
        if let Some(footer) = &self.footer {
            table = table.footer(Row::new(footer.clone()).top_margin(1));
        }
        if self.block {
            table = table.block(Block::bordered().title("T"));
        }
        table
    }

    /// Renders both tables with a copy of `state` and asserts that the buffers and resulting
    /// states are identical.
    fn assert_parity(&self, area: Rect, state: TableState) {
        let mut expected_state = state;
        let mut expected = Buffer::empty(area);
        StatefulWidget::render(self.upstream(), area, &mut expected, &mut expected_state);

        let mut actual_state = state;
        let mut actual = Buffer::empty(area);
        StatefulWidget::render(self.ours(), area, &mut actual, &mut actual_state);

        assert_eq!(actual, expected, "buffer mismatch for state {state:?}");
        assert_eq!(actual_state, expected_state, "state mismatch for {state:?}");
    }
}

fn states(rows: usize, columns: usize) -> Vec<TableState> {
    let mut states = vec![TableState::default()];
    for offset in [0, 3, rows + 5] {
        for selected in [
            None,
            Some(0),
            Some(rows / 2),
            Some(rows.saturating_sub(1)),
            Some(rows + 2),
        ] {
            for column in [None, Some(0), Some(columns + 1)] {
                states.push(
                    TableState::new()
                        .with_offset(offset)
                        .with_selected(selected)
                        .with_selected_column(column),
                );
            }
        }
    }
    states
}

#[test]
fn parity_basic() {
    let spec = Spec::simple(20, 1);
    for state in states(20, 3) {
        spec.assert_parity(Rect::new(0, 0, 30, 8), state);
    }
}

#[test]
fn parity_tall_rows_block_footer() {
    let mut spec = Spec::simple(15, 2);
    spec.block = true;
    spec.footer = Some(vec!["F1", "F2", "F3"]);
    spec.highlight_spacing = HighlightSpacing::Always;
    for state in states(15, 3) {
        spec.assert_parity(Rect::new(2, 1, 32, 14), state);
    }
}

#[test]
fn parity_margins_spans_flex() {
    let mut spec = Spec::simple(6, 1);
    spec.rows[1].0 = vec![("spans two columns", 2), ("x", 1)];
    spec.rows[2].0 = vec![("all three columns wide", 3)];
    spec.rows[3].2 = 1;
    spec.rows[4].3 = 2;
    spec.column_spacing = 2;
    spec.flex = Flex::SpaceBetween;
    spec.widths = vec![Constraint::Length(5); 3];
    spec.highlight_spacing = HighlightSpacing::Never;
    for state in states(6, 3) {
        spec.assert_parity(Rect::new(0, 0, 40, 9), state);
    }
}

#[test]
fn parity_empty_and_default_widths() {
    let mut spec = Spec::simple(0, 1);
    for state in states(0, 3) {
        spec.assert_parity(Rect::new(0, 0, 20, 4), state);
    }
    spec = Spec::simple(4, 1);
    spec.widths.clear();
    for state in states(4, 3) {
        spec.assert_parity(Rect::new(0, 0, 21, 4), state);
    }
}

#[test]
fn render_wrapped_paragraph_in_cell() {
    let rows = [
        Row::new(vec![
            Cell::from("id-1"),
            Cell::new(
                Paragraph::new("the quick brown fox jumps over the lazy dog")
                    .wrap(Wrap { trim: true }),
            ),
        ])
        .height(3),
        Row::new(vec!["id-2", "short"]),
    ];
    let table = Table::new(rows, [Constraint::Length(4), Constraint::Length(15)]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
    Widget::render(table, buf.area, &mut buf);
    assert_eq!(
        buf,
        Buffer::with_lines([
            "id-1 the quick brown",
            "     fox jumps over ",
            "     the lazy dog   ",
            "id-2 short          ",
        ])
    );
}

#[test]
fn render_block_and_gauge_in_cells() {
    let rows = [Row::new(vec![
        Cell::new(Paragraph::new("hi").block(Block::bordered())),
        Cell::new(Gauge::default().percent(50).label("50%")),
    ])
    .height(3)];
    let table = Table::new(rows, [Constraint::Length(6), Constraint::Length(6)]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 13, 3));
    Widget::render(table, buf.area, &mut buf);
    // only compare the symbols, the gauge applies styles to its filled half
    let symbols: Vec<String> = (0..3)
        .map(|y| (0..13).map(|x| buf[(x, y)].symbol()).collect())
        .collect();
    assert_eq!(
        symbols,
        ["┌────┐ ███   ", "│hi  │ █50%  ", "└────┘ ███   ",]
    );
}

#[test]
fn render_nested_table() {
    let inner = Table::new(
        [Row::new(vec!["a", "b"]), Row::new(vec!["c", "d"])],
        [Constraint::Length(1), Constraint::Length(1)],
    );
    let rows = [Row::new(vec![Cell::from("outer"), Cell::new(inner)]).height(2)];
    let table = Table::new(rows, [Constraint::Length(5), Constraint::Length(3)]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 9, 2));
    Widget::render(table, buf.area, &mut buf);
    assert_eq!(buf, Buffer::with_lines(["outer a b", "      c d"]));
}

#[test]
fn render_from_fn_cell_gets_cell_area() {
    let rows = [
        Row::new(vec!["x"]),
        Row::new(vec![
            Cell::from("y"),
            Cell::from_fn(|area, buf| {
                let text = format!("{},{} {}x{}", area.x, area.y, area.width, area.height);
                buf.set_string(area.x, area.y, text, Style::new());
            }),
        ])
        .height(2),
    ];
    let table =
        Table::new(rows, [Constraint::Length(1), Constraint::Length(10)]).block(Block::bordered());
    let mut buf = Buffer::empty(Rect::new(0, 0, 14, 5));
    Widget::render(table, buf.area, &mut buf);
    assert_eq!(
        buf,
        Buffer::with_lines([
            "┌────────────┐",
            "│x           │",
            "│y 3,2 10x2  │",
            "│            │",
            "└────────────┘",
        ])
    );
}

#[test]
fn widget_cells_are_clipped_to_visible_area() {
    // A row taller than the remaining space is rendered as a partial row, and the widget inside
    // is given the clipped area.
    let rows = [
        Row::new(vec!["1"]),
        Row::new(vec![Cell::new(Paragraph::new("a\nb\nc"))]).height(3),
    ];
    let table = Table::new(rows, [Constraint::Length(3)]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 3));
    Widget::render(table, buf.area, &mut buf);
    assert_eq!(buf, Buffer::with_lines(["1  ", "a  ", "b  "]));
}

#[test]
fn highlight_applies_over_widget_cells() {
    let rows = [
        Row::new(vec![Cell::new(Paragraph::new("one"))]),
        Row::new(vec![Cell::new(Paragraph::new("two"))]),
    ];
    let table = Table::new(rows, [Constraint::Length(3)])
        .highlight_symbol(">")
        .row_highlight_style(Style::new().reversed());
    let mut state = TableState::new().with_selected(Some(1));
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 2));
    StatefulWidget::render(table, buf.area, &mut buf, &mut state);
    let mut expected = Buffer::with_lines([" one", ">two"]);
    expected.set_style(Rect::new(0, 1, 4, 1), Style::new().reversed());
    assert_eq!(buf, expected);
}

/// Test cases ported from ratatui's main branch (`ratatui-widgets/src/table.rs`).
#[test]
fn scroll_padding() {
    // (name, render height, offset, padding, selected, expected offset, expected selected)
    let cases = [
        ("padding_scroll_down", 4, 0, 1, Some(3), 1, Some(3)),
        ("padding_scroll_up", 4, 4, 1, Some(2), 1, Some(2)),
        ("no_padding_offset_behavior", 5, 2, 0, Some(3), 2, Some(3)),
        ("padding_two_before", 5, 2, 2, Some(3), 1, Some(3)),
        (
            "padding_keep_selected_visible",
            4,
            0,
            4,
            Some(1),
            0,
            Some(1),
        ),
        ("no_selection_no_padding", 4, 2, 1, None, 2, None),
        (
            "maximum_padding_when_all_rows_fit",
            6,
            0,
            usize::MAX,
            Some(3),
            0,
            Some(3),
        ),
        (
            "maximum_padding_when_rows_do_not_fit",
            4,
            0,
            usize::MAX,
            Some(3),
            1,
            Some(3),
        ),
    ];
    for (name, height, offset, padding, selected, expected_offset, expected_selected) in cases {
        let rows = (0..6).map(|i| Row::new(vec![format!("Row {i}")]));
        let table = Table::new(rows, [Constraint::Length(10)]).scroll_padding(padding);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, height));
        let mut state = TableState::new()
            .with_offset(offset)
            .with_selected(selected);
        StatefulWidget::render(table, buf.area, &mut buf, &mut state);
        assert_eq!(state.offset(), expected_offset, "{name}: offset mismatch");
        assert_eq!(
            state.selected(),
            expected_selected,
            "{name}: selected mismatch"
        );
    }
}

#[test]
fn scroll_padding_is_stable_across_renders() {
    let rows = (0..8).map(|i| Row::new(vec![format!("Row {i}")]));
    let table = Table::new(rows, [Constraint::Length(10)]).scroll_padding(3);
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
    let mut state = TableState::new().with_offset(2).with_selected(Some(4));
    StatefulWidget::render(&table, buf.area, &mut buf, &mut state);
    let offset_after_render = state.offset();
    StatefulWidget::render(&table, buf.area, &mut buf, &mut state);
    assert_eq!(offset_after_render, state.offset());
}
