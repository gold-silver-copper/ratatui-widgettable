//! A [Ratatui] table widget whose cells can contain any widget.
//!
//! This crate provides a [`Table`] that works exactly like the `Table` widget built into Ratatui,
//! except that each [`Cell`] can hold any widget (e.g. `Paragraph`, `Block`, `Gauge`, `List`, or
//! even another [`Table`]) rather than only [`Text`].
//!
//! The API mirrors the built-in table, and it uses the same [`TableState`] and
//! [`HighlightSpacing`] types, so switching over is usually just a matter of changing imports:
//!
//! ```diff
//! -use ratatui::widgets::{Cell, Row, Table, TableState};
//! +use ratatui::widgets::TableState;
//! +use ratatui_widgettable::{Cell, Row, Table};
//! ```
//!
//! # Example
//!
//! ```rust
//! use ratatui::layout::Constraint;
//! use ratatui::widgets::{Block, Gauge, Paragraph, Wrap};
//! use ratatui_widgettable::{Cell, Row, Table};
//!
//! let rows = [Row::new(vec![
//!     Cell::from("Build"),
//!     Cell::new(
//!         Paragraph::new("Compiling a very long list of crates, this might take a while")
//!             .wrap(Wrap { trim: true }),
//!     ),
//!     Cell::new(Gauge::default().percent(42)),
//! ])
//! .height(3)];
//! let widths = [
//!     Constraint::Length(8),
//!     Constraint::Fill(1),
//!     Constraint::Length(20),
//! ];
//! let table = Table::new(rows, widths)
//!     .header(Row::new(vec!["Step", "Details", "Progress"]))
//!     .block(Block::bordered().title("Jobs"));
//! ```
//!
//! [Ratatui]: https://ratatui.rs
#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Flex, Layout, Rect};
use ratatui_core::style::{Style, Styled};
use ratatui_core::text::Text;
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::{Block, BlockExt};
pub use ratatui_widgets::table::{HighlightSpacing, TableState};

pub use self::cell::Cell;
pub use self::row::Row;

mod cell;
mod row;

/// Ensures that the examples in the README compile.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// A widget to display data in formatted columns, where each cell can contain any widget.
///
/// A `Table` is a collection of [`Row`]s, each composed of [`Cell`]s. Each [`Cell`] can hold any
/// widget which implements [`Widget`] for a reference to itself, or a closure (see
/// [`Cell::from_fn`]). Widgets are rendered into the area of the cell, which is the width of the
/// column and the [`height`](Row::height) of the row.
///
/// You can construct a [`Table`] using either [`Table::new`] or [`Table::default`] and then chain
/// builder style methods to set the desired properties.
///
/// Table cells can be aligned, for more details see [`Cell`].
///
/// Make sure to call the [`Table::widths`] method, otherwise the columns will all have a width of 0
/// and thus not be visible.
///
/// [`Table`] implements [`Widget`] and so it can be drawn using `Frame::render_widget`.
///
/// [`Table`] is also a [`StatefulWidget`], which means you can use it with [`TableState`] to allow
/// the user to scroll through the rows and select one of them. When rendering a [`Table`] with a
/// [`TableState`], the selected row, column and cell will be highlighted. If the selected row is
/// not visible (based on the offset), the table will be scrolled to make the selected row visible.
///
/// Note: if the `widths` field is empty, the table will be rendered with equal widths.
/// Note: Highlight styles are applied in the following order: Row, Column, Cell.
///
///
/// # Constructor methods
///
/// - [`Table::new`] creates a new [`Table`] with the given rows.
/// - [`Table::default`] creates an empty [`Table`]. You can then add rows using [`Table::rows`].
///
/// # Setter methods
///
/// These methods are fluent setters. They return a new `Table` with the specified property set.
///
/// - [`Table::rows`] sets the rows of the [`Table`].
/// - [`Table::header`] sets the header row of the [`Table`].
/// - [`Table::footer`] sets the footer row of the [`Table`].
/// - [`Table::widths`] sets the width constraints of each column.
/// - [`Table::column_spacing`] sets the spacing between each column.
/// - [`Table::block`] wraps the table in a [`Block`] widget.
/// - [`Table::style`] sets the base style of the widget.
/// - [`Table::row_highlight_style`] sets the style of the selected row.
/// - [`Table::column_highlight_style`] sets the style of the selected column.
/// - [`Table::cell_highlight_style`] sets the style of the selected cell.
/// - [`Table::highlight_symbol`] sets the symbol to be displayed in front of the selected row.
/// - [`Table::highlight_spacing`] sets when to show the highlight spacing.
/// - [`Table::scroll_padding`] sets the number of rows to keep visible before and after the
///   selected row.
///
/// # Example
///
/// ```rust
/// use ratatui::layout::Constraint;
/// use ratatui::style::{Style, Stylize};
/// use ratatui::widgets::Block;
/// use ratatui_widgettable::{Row, Table};
///
/// let rows = [Row::new(vec!["Cell1", "Cell2", "Cell3"])];
/// // Columns widths are constrained in the same way as Layout...
/// let widths = [
///     Constraint::Length(5),
///     Constraint::Length(5),
///     Constraint::Length(10),
/// ];
/// let table = Table::new(rows, widths)
///     // ...and they can be separated by a fixed spacing.
///     .column_spacing(1)
///     // You can set the style of the entire Table.
///     .style(Style::new().blue())
///     // It has an optional header, which is simply a Row always visible at the top.
///     .header(
///         Row::new(vec!["Col1", "Col2", "Col3"])
///             .style(Style::new().bold())
///             // To add space between the header and the rest of the rows, specify the margin
///             .bottom_margin(1),
///     )
///     // It has an optional footer, which is simply a Row always visible at the bottom.
///     .footer(Row::new(vec!["Updated on Dec 28"]))
///     // As any other widget, a Table can be wrapped in a Block.
///     .block(Block::new().title("Table"))
///     // The selected row, column, cell and its content can also be styled.
///     .row_highlight_style(Style::new().reversed())
///     .column_highlight_style(Style::new().red())
///     .cell_highlight_style(Style::new().blue())
///     // ...and potentially show a symbol in front of the selection.
///     .highlight_symbol(">>");
/// ```
///
/// Rows can be created from an iterator of [`Cell`]s. Each row can have an associated height,
/// bottom margin, and style. See [`Row`] for more details.
///
/// ```rust
/// use ratatui::style::{Style, Stylize};
/// use ratatui::text::{Line, Span};
/// use ratatui_widgettable::{Cell, Row, Table};
///
/// // a Row can be created from simple strings.
/// let row = Row::new(vec!["Row11", "Row12", "Row13"]);
///
/// // You can style the entire row.
/// let row = Row::new(vec!["Row21", "Row22", "Row23"]).style(Style::new().red());
///
/// // If you need more control over the styling, create Cells directly
/// let row = Row::new(vec![
///     Cell::from("Row31"),
///     Cell::from("Row32").style(Style::new().yellow()),
///     Cell::from(Line::from(vec![Span::raw("Row"), Span::from("33").green()])),
/// ]);
///
/// // If a Row need to display some content over multiple lines, specify the height.
/// let row = Row::new(vec![
///     Cell::from("Row\n41"),
///     Cell::from("Row\n42"),
///     Cell::from("Row\n43"),
/// ])
/// .height(2);
/// ```
///
/// Cells can be created from anything that can be converted to [`Text`], or from any widget. See
/// [`Cell`] for more details.
///
/// ```rust
/// use ratatui::style::{Style, Stylize};
/// use ratatui::text::{Line, Span, Text};
/// use ratatui::widgets::Paragraph;
/// use ratatui_widgettable::Cell;
///
/// Cell::from("simple string");
/// Cell::from("simple styled span".red());
/// Cell::from(Span::raw("raw span"));
/// Cell::from(Span::styled("styled span", Style::new().red()));
/// Cell::from(Line::from(vec![
///     Span::raw("a vec of "),
///     Span::from("spans").bold(),
/// ]));
/// Cell::from(Text::from("text"));
/// Cell::new(Paragraph::new("a paragraph").centered());
/// ```
///
/// Just as rows can be collected from iterators of `Cell`s, tables can be collected from iterators
/// of `Row`s.  This will create a table with column widths evenly dividing the space available.
/// These default columns widths can be overridden using the `Table::widths` method.
///
/// ```rust
/// use ratatui::layout::Constraint;
/// use ratatui_widgettable::{Row, Table};
///
/// let text = "Mary had a\nlittle lamb.";
///
/// let table = text
///     .split("\n")
///     .map(|line: &str| -> Row { line.split_ascii_whitespace().collect() })
///     .collect::<Table>()
///     .widths([Constraint::Length(10); 3]);
/// ```
///
/// `Table` also implements the [`Styled`] trait, which means you can use style shorthands from
/// the [`Stylize`] trait to set the style of the widget more concisely.
///
/// ```rust
/// use ratatui::layout::Constraint;
/// use ratatui::style::Stylize;
/// use ratatui_widgettable::{Row, Table};
///
/// let rows = [Row::new(vec!["Cell1", "Cell2", "Cell3"])];
/// let widths = [
///     Constraint::Length(5),
///     Constraint::Length(5),
///     Constraint::Length(10),
/// ];
/// let table = Table::new(rows, widths).red().italic();
/// ```
///
/// # Stateful example
///
/// `Table` is a [`StatefulWidget`], which means you can use it with [`TableState`] to allow the
/// user to scroll through the rows and select one of them.
///
/// ```rust
/// use ratatui::Frame;
/// use ratatui::layout::{Constraint, Rect};
/// use ratatui::style::{Style, Stylize};
/// use ratatui::widgets::Block;
/// use ratatui_widgettable::{Row, Table, TableState};
///
/// # fn ui(frame: &mut Frame) {
/// # let area = Rect::default();
/// // Note: TableState should be stored in your application state (not constructed in your render
/// // method) so that the selected row is preserved across renders
/// let mut table_state = TableState::default();
/// let rows = [
///     Row::new(vec!["Row11", "Row12", "Row13"]),
///     Row::new(vec!["Row21", "Row22", "Row23"]),
///     Row::new(vec!["Row31", "Row32", "Row33"]),
/// ];
/// let widths = [
///     Constraint::Length(5),
///     Constraint::Length(5),
///     Constraint::Length(10),
/// ];
/// let table = Table::new(rows, widths)
///     .block(Block::new().title("Table"))
///     .row_highlight_style(Style::new().reversed())
///     .highlight_symbol(">>");
///
/// frame.render_stateful_widget(table, area, &mut table_state);
/// # }
/// ```
///
/// [`Stylize`]: ratatui_core::style::Stylize
#[derive(Debug)]
pub struct Table<'a> {
    /// Data to display in each row
    rows: Vec<Row<'a>>,

    /// Optional header
    header: Option<Row<'a>>,

    /// Optional footer
    footer: Option<Row<'a>>,

    /// Width constraints for each column
    widths: Vec<Constraint>,

    /// Space between each column
    column_spacing: u16,

    /// A block to wrap the widget in
    block: Option<Block<'a>>,

    /// Base style for the widget
    style: Style,

    /// Style used to render the selected row
    row_highlight_style: Style,

    /// Style used to render the selected column
    column_highlight_style: Style,

    /// Style used to render the selected cell
    cell_highlight_style: Style,

    /// Symbol in front of the selected row
    highlight_symbol: Text<'a>,

    /// Decides when to allocate spacing for the row selection
    highlight_spacing: HighlightSpacing,

    /// Controls how to distribute extra space among the columns
    flex: Flex,

    /// How many rows to try to keep visible before and after the selected row
    scroll_padding: usize,
}

impl Default for Table<'_> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            header: None,
            footer: None,
            widths: Vec::new(),
            column_spacing: 1,
            block: None,
            style: Style::new(),
            row_highlight_style: Style::new(),
            column_highlight_style: Style::new(),
            cell_highlight_style: Style::new(),
            highlight_symbol: Text::default(),
            highlight_spacing: HighlightSpacing::default(),
            flex: Flex::Start,
            scroll_padding: 0,
        }
    }
}

impl<'a> Table<'a> {
    /// Creates a new [`Table`] widget with the given rows.
    ///
    /// The `rows` parameter accepts any value that can be converted into an iterator of [`Row`]s.
    /// This includes arrays, slices, and [`Vec`]s.
    ///
    /// The `widths` parameter accepts any type that implements `IntoIterator<Item =
    /// Into<Constraint>>`. This includes arrays, slices, vectors, iterators. `Into<Constraint>` is
    /// implemented on u16, so you can pass an array, vec, etc. of u16 to this function to create a
    /// table with fixed width columns.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui_widgettable::{Row, Table};
    ///
    /// let rows = [
    ///     Row::new(vec!["Cell1", "Cell2"]),
    ///     Row::new(vec!["Cell3", "Cell4"]),
    /// ];
    /// let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths);
    /// ```
    pub fn new<R, C>(rows: R, widths: C) -> Self
    where
        R: IntoIterator,
        R::Item: Into<Row<'a>>,
        C: IntoIterator,
        C::Item: Into<Constraint>,
    {
        let widths = widths.into_iter().map(Into::into).collect::<Vec<_>>();
        ensure_percentages_less_than_100(&widths);

        let rows = rows.into_iter().map(Into::into).collect();
        Self {
            rows,
            widths,
            ..Default::default()
        }
    }

    /// Set the rows
    ///
    /// The `rows` parameter accepts any value that can be converted into an iterator of [`Row`]s.
    /// This includes arrays, slices, and [`Vec`]s.
    ///
    /// # Warning
    ///
    /// This method does not currently set the column widths. You will need to set them manually by
    /// calling [`Table::widths`].
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui_widgettable::{Row, Table};
    ///
    /// let rows = [
    ///     Row::new(vec!["Cell1", "Cell2"]),
    ///     Row::new(vec!["Cell3", "Cell4"]),
    /// ];
    /// let table = Table::default().rows(rows);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn rows<T>(mut self, rows: T) -> Self
    where
        T: IntoIterator<Item = Row<'a>>,
    {
        self.rows = rows.into_iter().collect();
        self
    }

    /// Sets the header row
    ///
    /// The `header` parameter is a [`Row`] which will be displayed at the top of the [`Table`]
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui_widgettable::{Cell, Row, Table};
    ///
    /// let header = Row::new(vec![
    ///     Cell::from("Header Cell 1"),
    ///     Cell::from("Header Cell 2"),
    /// ]);
    /// let table = Table::default().header(header);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn header(mut self, header: Row<'a>) -> Self {
        self.header = Some(header);
        self
    }

    /// Sets the footer row
    ///
    /// The `footer` parameter is a [`Row`] which will be displayed at the bottom of the [`Table`]
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui_widgettable::{Cell, Row, Table};
    ///
    /// let footer = Row::new(vec![
    ///     Cell::from("Footer Cell 1"),
    ///     Cell::from("Footer Cell 2"),
    /// ]);
    /// let table = Table::default().footer(footer);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn footer(mut self, footer: Row<'a>) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Set the widths of the columns.
    ///
    /// The `widths` parameter accepts any type that implements `IntoIterator<Item =
    /// Into<Constraint>>`. This includes arrays, slices, vectors, iterators. `Into<Constraint>` is
    /// implemented on u16, so you can pass an array, vec, etc. of u16 to this function to create a
    /// table with fixed width columns.
    ///
    /// If the widths are empty, the table will be rendered with equal widths.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui_widgettable::{Cell, Row, Table};
    ///
    /// let table = Table::default().widths([Constraint::Length(5), Constraint::Length(5)]);
    /// let table = Table::default().widths(vec![Constraint::Length(5); 2]);
    ///
    /// // widths could also be computed at runtime
    /// let widths = [10, 10, 20].into_iter().map(|c| Constraint::Length(c));
    /// let table = Table::default().widths(widths);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn widths<I>(mut self, widths: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Constraint>,
    {
        let widths = widths.into_iter().map(Into::into).collect::<Vec<_>>();
        ensure_percentages_less_than_100(&widths);
        self.widths = widths;
        self
    }

    /// Set the spacing between columns
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui_widgettable::{Row, Table};
    ///
    /// let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).column_spacing(1);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn column_spacing(mut self, spacing: u16) -> Self {
        self.column_spacing = spacing;
        self
    }

    /// Wraps the table with a custom [`Block`] widget.
    ///
    /// The `block` parameter is of type [`Block`]. This holds the specified block to be
    /// created around the [`Table`]
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui::widgets::Block;
    /// use ratatui_widgettable::{Cell, Row, Table};
    ///
    /// let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let block = Block::bordered().title("Table");
    /// let table = Table::new(rows, widths).block(block);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the base style of the widget
    ///
    /// `style` accepts any type that is convertible to [`Style`] (e.g. [`Style`], [`Color`], or
    /// your own type that implements [`Into<Style>`]).
    ///
    /// All text rendered by the widget will use this style, unless overridden by [`Block::style`],
    /// [`Row::style`], [`Cell::style`], or the styles of cell's content.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui::style::{Style, Stylize};
    /// use ratatui_widgettable::{Row, Table};
    ///
    /// # let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// # let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).style(Style::new().red().italic());
    /// ```
    ///
    /// `Table` also implements the [`Styled`] trait, which means you can use style shorthands from
    /// the [`Stylize`] trait to set the style of the widget more concisely.
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui::style::Stylize;
    /// use ratatui_widgettable::{Cell, Row, Table};
    ///
    /// # let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// # let widths = vec![Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).red().italic();
    /// ```
    ///
    /// [`Color`]: ratatui_core::style::Color
    /// [`Stylize`]: ratatui_core::style::Stylize
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }

    /// Set the style of the selected row
    ///
    /// `style` accepts any type that is convertible to [`Style`] (e.g. [`Style`], [`Color`], or
    /// your own type that implements [`Into<Style>`]).
    ///
    /// This style will be applied to the entire row, including the selection symbol if it is
    /// displayed, and will override any style set on the row or on the individual cells.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ratatui::{layout::Constraint, style::{Style, Stylize}};
    /// # use ratatui_widgettable::{Row, Table};
    /// # let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// # let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).row_highlight_style(Style::new().red().italic());
    /// ```
    /// [`Color`]: ratatui_core::style::Color
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn row_highlight_style<S: Into<Style>>(mut self, highlight_style: S) -> Self {
        self.row_highlight_style = highlight_style.into();
        self
    }

    /// Set the style of the selected column
    ///
    /// `style` accepts any type that is convertible to [`Style`] (e.g. [`Style`], [`Color`], or
    /// your own type that implements [`Into<Style>`]).
    ///
    /// This style will be applied to the entire column, and will override any style set on the
    /// row or on the individual cells.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ratatui::{layout::Constraint, style::{Style, Stylize}};
    /// # use ratatui_widgettable::{Row, Table};
    /// # let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// # let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).column_highlight_style(Style::new().red().italic());
    /// ```
    /// [`Color`]: ratatui_core::style::Color
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn column_highlight_style<S: Into<Style>>(mut self, highlight_style: S) -> Self {
        self.column_highlight_style = highlight_style.into();
        self
    }

    /// Set the style of the selected cell
    ///
    /// `style` accepts any type that is convertible to [`Style`] (e.g. [`Style`], [`Color`], or
    /// your own type that implements [`Into<Style>`]).
    ///
    /// This style will be applied to the selected cell, and will override any style set on the
    /// row or on the individual cells.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ratatui::{layout::Constraint, style::{Style, Stylize}};
    /// # use ratatui_widgettable::{Row, Table};
    /// # let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// # let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).cell_highlight_style(Style::new().red().italic());
    /// ```
    /// [`Color`]: ratatui_core::style::Color
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn cell_highlight_style<S: Into<Style>>(mut self, highlight_style: S) -> Self {
        self.cell_highlight_style = highlight_style.into();
        self
    }

    /// Set the symbol to be displayed in front of the selected row
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui_widgettable::{Cell, Row, Table};
    ///
    /// # let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// # let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).highlight_symbol(">>");
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn highlight_symbol<T: Into<Text<'a>>>(mut self, highlight_symbol: T) -> Self {
        self.highlight_symbol = highlight_symbol.into();
        self
    }

    /// Set when to show the highlight spacing
    ///
    /// The highlight spacing is the spacing that is allocated for the selection symbol column (if
    /// enabled) and is used to shift the table when a row is selected. This method allows you to
    /// configure when this spacing is allocated.
    ///
    /// - [`HighlightSpacing::Always`] will always allocate the spacing, regardless of whether a row
    ///   is selected or not. This means that the table will never change size, regardless of if a
    ///   row is selected or not.
    /// - [`HighlightSpacing::WhenSelected`] will only allocate the spacing if a row is selected.
    ///   This means that the table will shift when a row is selected. This is the default setting
    ///   for backwards compatibility, but it is recommended to use `HighlightSpacing::Always` for a
    ///   better user experience.
    /// - [`HighlightSpacing::Never`] will never allocate the spacing, regardless of whether a row
    ///   is selected or not. This means that the highlight symbol will never be drawn.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::layout::Constraint;
    /// use ratatui_widgettable::{HighlightSpacing, Row, Table};
    ///
    /// let rows = [Row::new(vec!["Cell1", "Cell2"])];
    /// let widths = [Constraint::Length(5), Constraint::Length(5)];
    /// let table = Table::new(rows, widths).highlight_spacing(HighlightSpacing::Always);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn highlight_spacing(mut self, value: HighlightSpacing) -> Self {
        self.highlight_spacing = value;
        self
    }

    /// Set how extra space is distributed amongst columns.
    ///
    /// This determines how the space is distributed when the constraints are satisfied. By default,
    /// the extra space is not distributed at all.  But this can be changed to distribute all extra
    /// space to the last column or to distribute it equally.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// Create a table that needs at least 30 columns to display.  Any extra space will be assigned
    /// to the last column.
    /// ```
    /// use ratatui::layout::{Constraint, Flex};
    /// use ratatui_widgettable::{Row, Table};
    ///
    /// let widths = [
    ///     Constraint::Min(10),
    ///     Constraint::Min(10),
    ///     Constraint::Min(10),
    /// ];
    /// let table = Table::new(Vec::<Row>::new(), widths).flex(Flex::Legacy);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn flex(mut self, flex: Flex) -> Self {
        self.flex = flex;
        self
    }

    /// Set the number of rows to keep visible before and after the selected row when scrolling.
    ///
    /// This is similar to the `scrolloff` option in Vim, and ensures context around the selected
    /// row is visible. If the padding value is too large for the visible area, it will be
    /// automatically reduced to keep the selected row visible.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui_widgettable::Table;
    ///
    /// let table = Table::default().scroll_padding(1);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn scroll_padding(mut self, padding: usize) -> Self {
        self.scroll_padding = padding;
        self
    }
}

impl Widget for Table<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(&self, area, buf);
    }
}

impl Widget for &Table<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = TableState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

impl StatefulWidget for Table<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        StatefulWidget::render(&self, area, buf, state);
    }
}

impl StatefulWidget for &Table<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        self.block.as_ref().render(area, buf);
        let table_area = self.block.inner_if_some(area);
        if table_area.is_empty() {
            return;
        }

        if state.selected().is_some_and(|s| s >= self.rows.len()) {
            state.select(Some(self.rows.len().saturating_sub(1)));
        }

        if self.rows.is_empty() {
            state.select(None);
        }

        let column_count = self.column_count();
        if state.selected_column().is_some_and(|s| s >= column_count) {
            state.select_column(Some(column_count.saturating_sub(1)));
        }
        if column_count == 0 {
            state.select_column(None);
        }

        let selection_width = self.selection_width(state);
        let column_widths = self.get_column_widths(table_area.width, selection_width, column_count);
        let (header_area, rows_area, footer_area) = self.layout(table_area);

        self.render_header(header_area, buf, &column_widths);

        self.render_rows(rows_area, buf, selection_width, state, &column_widths);

        self.render_footer(footer_area, buf, &column_widths);
    }
}

// private methods for rendering
impl Table<'_> {
    /// Splits the table area into a header, rows area and a footer
    fn layout(&self, area: Rect) -> (Rect, Rect, Rect) {
        let header_top_margin = self.header.as_ref().map_or(0, |h| h.top_margin);
        let header_height = self.header.as_ref().map_or(0, |h| h.height);
        let header_bottom_margin = self.header.as_ref().map_or(0, |h| h.bottom_margin);
        let footer_top_margin = self.footer.as_ref().map_or(0, |h| h.top_margin);
        let footer_height = self.footer.as_ref().map_or(0, |f| f.height);
        let footer_bottom_margin = self.footer.as_ref().map_or(0, |h| h.bottom_margin);
        let layout = Layout::vertical([
            Constraint::Length(header_top_margin),
            Constraint::Length(header_height),
            Constraint::Length(header_bottom_margin),
            Constraint::Min(0),
            Constraint::Length(footer_top_margin),
            Constraint::Length(footer_height),
            Constraint::Length(footer_bottom_margin),
        ])
        .split(area);
        let (header_area, rows_area, footer_area) = (layout[1], layout[3], layout[5]);
        (header_area, rows_area, footer_area)
    }

    /// Render the header cells, if they are not `None`
    ///
    /// The `x` and `width` fields of each `Rect` in `column_widths` denote the starting
    /// x-coordinate and width of each column in the table.
    fn render_header(&self, area: Rect, buf: &mut Buffer, column_widths: &[Rect]) {
        if let Some(ref header) = self.header {
            buf.set_style(area, header.style);
            for (cell_area, cell) in column_widths.iter().zip(header.cells.iter()) {
                let new_x = area.x + cell_area.x;
                let area_to_render = Rect::new(new_x, area.y, cell_area.width, area.height);
                cell.render(area_to_render, buf);
            }
        }
    }

    /// Render the footer cells, if they are not `None`
    ///
    /// The `x` and `width` fields of each `Rect` in `column_widths` denote the starting
    /// x-coordinate and width of each column in the table.
    fn render_footer(&self, area: Rect, buf: &mut Buffer, column_widths: &[Rect]) {
        if let Some(ref footer) = self.footer {
            buf.set_style(area, footer.style);
            for (cell_area, cell) in column_widths.iter().zip(footer.cells.iter()) {
                let new_x = area.x + cell_area.x;
                let area_to_render = Rect::new(new_x, area.y, cell_area.width, area.height);
                cell.render(area_to_render, buf);
            }
        }
    }

    /// Render the table rows
    ///
    /// The `x` and `width` fields of each `Rect` in `column_widths` denote the starting
    /// x-coordinate and width of each column in the table.
    fn render_rows(
        &self,
        area: Rect,
        buf: &mut Buffer,
        selection_width: u16,
        state: &mut TableState,
        columns_widths: &[Rect],
    ) {
        if self.rows.is_empty() {
            return;
        }

        let (start_index, end_index) = self.visible_rows(state, area);
        *state.offset_mut() = start_index;

        let mut y_offset = 0;

        let mut selected_row_area = None;
        for (i, row) in self
            .rows
            .iter()
            .enumerate()
            .skip(start_index)
            .take(end_index - start_index)
        {
            let y = area.y + y_offset + row.top_margin;
            let height = (y + row.height).min(area.bottom()).saturating_sub(y);
            let row_area = Rect { y, height, ..area };
            buf.set_style(row_area, row.style);

            let is_selected = state.selected().is_some_and(|index| index == i);
            if selection_width > 0 && is_selected {
                self.set_selection_style(buf, selection_width, row_area, row);
            }
            self.render_row_cells(buf, columns_widths.iter().collect(), &row.cells, row_area);
            if is_selected {
                selected_row_area = Some(row_area);
            }
            y_offset += row.height_with_margin();
        }

        let selected_column_area = state.selected_column().and_then(|s| {
            // The selection is clamped by the column count. Since a user can manually specify an
            // incorrect number of widths, we should use panic free methods.
            columns_widths.get(s).map(|cell_area| Rect {
                x: cell_area.x + area.x,
                width: cell_area.width,
                ..area
            })
        });

        match (selected_row_area, selected_column_area) {
            (Some(row_area), Some(col_area)) => {
                buf.set_style(row_area, self.row_highlight_style);
                buf.set_style(col_area, self.column_highlight_style);
                let cell_area = row_area.intersection(col_area);
                buf.set_style(cell_area, self.cell_highlight_style);
            }
            (Some(row_area), None) => {
                buf.set_style(row_area, self.row_highlight_style);
            }
            (None, Some(col_area)) => {
                buf.set_style(col_area, self.column_highlight_style);
            }
            (None, None) => (),
        }
    }

    /// Render cells into the columns of a row
    ///
    /// Render `Cell`s from `cells` into columns specified by `column_widths`, stopping
    /// if either of these iterators are finished.  Each `Cell` gets rendered across
    /// [`Cell::get_column_span`] columns plus the gaps between them, if this value is > 1.
    fn render_row_cells(
        &self,
        buf: &mut Buffer,
        column_widths: Vec<&Rect>,
        cells: &Vec<Cell>,
        row_area: Rect,
    ) {
        let mut column_widths_iterator = column_widths.into_iter();
        for current_cell in cells {
            if let Some(cell_area) = Self::get_cell_area(
                &mut column_widths_iterator,
                current_cell.column_span,
                self.column_spacing,
            ) {
                let new_x = row_area.x + cell_area.x;
                let area_to_render = Rect::new(new_x, row_area.y, cell_area.width, row_area.height);
                current_cell.render(area_to_render, buf);
            }
        }
    }

    /// Set the row style and render the highlight symbol
    fn set_selection_style(
        &self,
        buf: &mut Buffer,
        selection_width: u16,
        row_area: Rect,
        row: &Row,
    ) {
        let selection_area = Rect {
            width: selection_width,
            ..row_area
        };
        buf.set_style(selection_area, row.style);
        (&self.highlight_symbol).render(selection_area, buf);
    }

    /// Return the area that a [`Cell`] should occupy, taking into account its
    /// [`Cell::column_span`].
    ///
    /// Returns `None` when there are no more columns for the [`Cell`] to occupy.
    ///
    /// Otherwise, returns `Some(Rect{x, y = 0, width, height = 0})`, representing the start
    /// x-coordinate and width of the [`Cell`].
    ///
    /// This function consumes `cell_column_span` `Rect`s from `column_widths_iterator` (or all the
    /// `Rects` if the iterator is less than `cell_column_span` `Rect`s long). This function adds
    /// the width of each `Rect` plus `column_spacing` to a running total of the final width.  The
    /// return value is the original x coordinate and the final width, or `None` if
    /// `column_widths_iterator` is empty or `cell_column_span` is `0`.
    fn get_cell_area<'a, T>(
        column_widths_iterator: &mut T,
        cell_column_span: u16,
        column_spacing: u16,
    ) -> Option<Rect>
    where
        T: Iterator<Item = &'a Rect>,
    {
        if cell_column_span == 0 {
            return None;
        }
        let first = column_widths_iterator.next()?;
        let (n_columns_taken, all_columns_width) = column_widths_iterator
            .take((cell_column_span - 1).into())
            .map(|rect| (1, rect.width))
            .fold((1, first.width), |so_far, next_column| {
                (next_column.0 + so_far.0, next_column.1 + so_far.1)
            });
        let width = all_columns_width + (n_columns_taken - 1) * column_spacing;
        Some(Rect::new(first.x, first.y, width, 1))
    }

    /// Return the indexes of the visible rows.
    ///
    /// The algorithm works as follows:
    /// - start at the offset and calculate the height of the rows that can be displayed within the
    ///   area.
    /// - if the selected row is not visible, scroll the table to ensure it is visible.
    /// - if scroll padding is set, ensure the padding number of rows are visible before and after
    ///   the selected row, adjusting the padding down when items of inconsistent sizes make it
    ///   impossible.
    /// - if there is still space to fill then there's a partial row at the end which should be
    ///   included in the view.
    fn visible_rows(&self, state: &TableState, area: Rect) -> (usize, usize) {
        let last_row = self.rows.len().saturating_sub(1);
        let mut start = state.offset().min(last_row);

        if let Some(selected) = state.selected() {
            start = start.min(selected);
        }

        let mut end = start;
        let mut height = 0;

        for item in self.rows.iter().skip(start) {
            if height + item.height > area.height {
                break;
            }
            height += item.height_with_margin();
            end += 1;
        }

        if let Some(selected) = state.selected() {
            let selected = selected.min(last_row);

            let index_to_display = self.apply_scroll_padding_to_selected_index(
                selected,
                area.height as usize,
                start,
                end,
            );

            // scroll down until the target index is visible
            while index_to_display >= end {
                height = height.saturating_add(self.rows[end].height_with_margin());
                end += 1;
                while height > area.height {
                    height = height.saturating_sub(self.rows[start].height_with_margin());
                    start += 1;
                }
            }

            // scroll up until the target index is visible
            while index_to_display < start {
                start -= 1;
                height = height.saturating_add(self.rows[start].height_with_margin());
                while height > area.height {
                    end -= 1;
                    height = height.saturating_sub(self.rows[end].height_with_margin());
                }
            }
        }

        // Include a partial row if there is space
        if height < area.height && end < self.rows.len() {
            end += 1;
        }

        (start, end)
    }

    /// Applies scroll padding to the selected index, reducing the padding value to keep the
    /// selected row on screen even with rows of inconsistent sizes
    fn apply_scroll_padding_to_selected_index(
        &self,
        selected: usize,
        max_height: usize,
        first_visible_index: usize,
        last_visible_index: usize,
    ) -> usize {
        let last_valid_index = self.rows.len().saturating_sub(1);
        if self.scroll_padding == 0 {
            return selected;
        }

        let mut scroll_padding = self.scroll_padding.min(last_valid_index);
        let pad_start = selected.saturating_sub(scroll_padding);
        let pad_end = selected
            .saturating_add(scroll_padding)
            .min(last_valid_index);
        let mut height_around_selected = self.rows[pad_start..=pad_end]
            .iter()
            .map(|row| row.height_with_margin() as usize)
            .sum::<usize>();

        while scroll_padding > 0 && height_around_selected > max_height {
            if let Some(index) = selected.checked_sub(scroll_padding) {
                height_around_selected = height_around_selected
                    .saturating_sub(self.rows[index].height_with_margin() as usize);
            }
            if let Some(index) = selected
                .checked_add(scroll_padding)
                .filter(|&index| index <= last_valid_index)
            {
                height_around_selected = height_around_selected
                    .saturating_sub(self.rows[index].height_with_margin() as usize);
            }
            scroll_padding -= 1;
        }

        let selected_after_padding = selected
            .saturating_add(scroll_padding)
            .min(last_valid_index);
        if selected_after_padding >= last_visible_index {
            selected_after_padding
        } else if selected.saturating_sub(scroll_padding) < first_visible_index {
            selected.saturating_sub(scroll_padding)
        } else {
            selected
        }
    }

    /// Get all offsets and widths of all user specified columns.
    ///
    /// Returns (x, width). When self.widths is empty, it is assumed `.widths()` has not been called
    /// and a default of equal widths is returned.
    fn get_column_widths(
        &self,
        max_width: u16,
        selection_width: u16,
        col_count: usize,
    ) -> Vec<Rect> {
        let widths = if self.widths.is_empty() {
            // Divide the space between each column equally
            vec![Constraint::Length(max_width / col_count.max(1) as u16); col_count]
        } else {
            self.widths.clone()
        };
        // this will always allocate a selection area
        let [_selection_area, columns_area] =
            Layout::horizontal([Constraint::Length(selection_width), Constraint::Fill(0)])
                .areas(Rect::new(0, 0, max_width, 1));
        let rects = Layout::horizontal(widths)
            .flex(self.flex)
            .spacing(self.column_spacing)
            .split(columns_area);
        rects
            .iter()
            .map(|c| Rect::new(c.x, 0, c.width, 1))
            .collect()
    }

    fn column_count(&self) -> usize {
        self.rows
            .iter()
            .chain(self.footer.iter())
            .chain(self.header.iter())
            .map(|r| r.cells.len())
            .max()
            .unwrap_or_default()
    }

    /// Returns the width of the selection column if a row is selected, or the `highlight_spacing`
    /// is set to show the column always, otherwise 0.
    fn selection_width(&self, state: &TableState) -> u16 {
        let should_add = match self.highlight_spacing {
            HighlightSpacing::Always => true,
            HighlightSpacing::WhenSelected => state.selected().is_some(),
            HighlightSpacing::Never => false,
        };
        if should_add {
            self.highlight_symbol.width() as u16
        } else {
            0
        }
    }
}

fn ensure_percentages_less_than_100(widths: &[Constraint]) {
    for w in widths {
        if let Constraint::Percentage(p) = w {
            assert!(
                *p <= 100,
                "Percentages should be between 0 and 100 inclusively."
            );
        }
    }
}

impl Styled for Table<'_> {
    type Item = Self;

    fn style(&self) -> Style {
        self.style
    }

    fn set_style<S: Into<Style>>(self, style: S) -> Self::Item {
        self.style(style)
    }
}

impl<'a, Item> FromIterator<Item> for Table<'a>
where
    Item: Into<Row<'a>>,
{
    /// Collects an iterator of rows into a table.
    ///
    /// When collecting from an iterator into a table, the user must provide the widths using
    /// `Table::widths` after construction.
    fn from_iter<Iter: IntoIterator<Item = Item>>(rows: Iter) -> Self {
        let widths: [Constraint; 0] = [];
        Self::new(rows, widths)
    }
}
