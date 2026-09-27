use alloc::boxed::Box;
use core::fmt;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Style, Styled};
use ratatui_core::text::Text;
use ratatui_core::widgets::Widget;

/// A [`Cell`] contains the widget to be displayed in a [`Row`] of a [`Table`].
///
/// Unlike the `Cell` in ratatui's built-in table, which can only hold [`Text`], this `Cell` can
/// hold any widget that can be rendered by reference (i.e. `&W` implements [`Widget`]). This
/// includes all of the built-in ratatui widgets such as `Paragraph`, `Block`, `Gauge`, `List`,
/// `Sparkline`, `Chart`, and even another [`Table`].
///
/// For widgets that can only be rendered by value, or for custom drawing logic, use
/// [`Cell::from_fn`].
///
/// Note: unlike the built-in table, [`Cell::new`] takes a widget, so to create a cell from a string
/// use [`Cell::from`] (e.g. `Cell::from("text")`) or wrap it in [`Text`] (`Cell::new(Text::from(
/// "text"))`). Anything convertible into [`Text`] can also be passed directly to [`Row::new`].
///
/// You can apply a [`Style`] to the [`Cell`] using [`Cell::style`]. This will set the style for the
/// entire area of the cell before the widget is rendered on top of it.
///
/// # Examples
///
/// ```rust
/// use ratatui::style::Style;
/// use ratatui::widgets::{Block, Gauge, Paragraph, Wrap};
/// use ratatui_widgettable::Cell;
///
/// // Anything convertible into `Text` still works, just like the built-in table
/// Cell::from("simple string");
///
/// // Any widget can be used as the content of a cell
/// Cell::new(Paragraph::new("some long text that wraps").wrap(Wrap { trim: true }));
/// Cell::new(Gauge::default().percent(42));
/// Cell::new(Block::bordered().title("boxed"));
///
/// // Or use a closure for custom rendering
/// Cell::from_fn(|area, buf| {
///     buf.set_string(area.x, area.y, "custom", Style::new());
/// });
/// ```
///
/// `Cell` implements [`Styled`] which means you can use style shorthands from the [`Stylize`] trait
/// to set the style of the cell concisely.
///
/// ```rust
/// use ratatui::style::Stylize;
/// use ratatui_widgettable::Cell;
///
/// Cell::from("Cell 1").red().italic();
/// ```
///
/// [`Row`]: crate::Row
/// [`Row::new`]: crate::Row::new
/// [`Table`]: crate::Table
/// [`Stylize`]: ratatui_core::style::Stylize
pub struct Cell<'a> {
    content: Box<dyn CellContent + 'a>,
    style: Style,
    /// The number of columns this cell will extend over
    pub(crate) column_span: u16,
}

/// Object safe trait for anything that can render itself by reference into a cell.
trait CellContent {
    fn render_ref(&self, area: Rect, buf: &mut Buffer);
}

impl<W> CellContent for W
where
    for<'b> &'b W: Widget,
{
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Widget::render(self, area, buf);
    }
}

/// Wraps a closure so that it can be used as the content of a cell.
struct FnContent<F>(F);

impl<F> CellContent for FnContent<F>
where
    F: Fn(Rect, &mut Buffer),
{
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        (self.0)(area, buf);
    }
}

impl<'a> Cell<'a> {
    /// Creates a new [`Cell`] containing the given widget.
    ///
    /// The `widget` parameter accepts any widget which implements [`Widget`] for a reference to
    /// itself (`&W: Widget`), which all of the built-in ratatui widgets do.
    ///
    /// To create a cell from a string or other [`Text`], use [`Cell::from`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::text::Text;
    /// use ratatui::widgets::{Block, Paragraph};
    /// use ratatui_widgettable::Cell;
    ///
    /// Cell::new(Paragraph::new("hello").block(Block::bordered()));
    /// Cell::new(Text::from("a text"));
    /// ```
    pub fn new<W>(widget: W) -> Self
    where
        W: 'a,
        for<'b> &'b W: Widget,
    {
        Self {
            content: Box::new(widget),
            style: Style::default(),
            column_span: 1,
        }
    }

    /// Creates a new [`Cell`] which renders its content using the given closure.
    ///
    /// The closure is called with the area of the cell and the buffer to render into. This is
    /// useful for widgets that only implement [`Widget`] by value, stateful widgets, or custom
    /// drawing logic.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::widgets::{Paragraph, Widget};
    /// use ratatui_widgettable::Cell;
    ///
    /// let cell = Cell::from_fn(|area, buf| {
    ///     Paragraph::new("rendered from a closure").render(area, buf);
    /// });
    /// ```
    pub fn from_fn<F>(render: F) -> Self
    where
        F: Fn(Rect, &mut Buffer) + 'a,
    {
        Self {
            content: Box::new(FnContent(render)),
            style: Style::default(),
            column_span: 1,
        }
    }

    /// Set the content of the [`Cell`] to the given widget.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::widgets::Paragraph;
    /// use ratatui_widgettable::Cell;
    ///
    /// Cell::default().content(Paragraph::new("hello"));
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn content<W>(mut self, widget: W) -> Self
    where
        W: 'a,
        for<'b> &'b W: Widget,
    {
        self.content = Box::new(widget);
        self
    }

    /// Set the `column_span` of this cell
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Example
    /// ```rust
    /// use ratatui_widgettable::{Cell, Row};
    /// let rows = vec![
    ///     Row::new(vec![Cell::from("12345").column_span(2)]),
    ///     Row::new(vec![Cell::from("xx"), Cell::from("yy")]),
    /// ];
    /// // "12345",
    /// // "xx yy",
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn column_span(mut self, column_span: u16) -> Self {
        self.column_span = column_span;
        self
    }

    /// Set the `Style` of this cell
    ///
    /// `style` accepts any type that is convertible to [`Style`] (e.g. [`Style`], [`Color`], or
    /// your own type that implements [`Into<Style>`]).
    ///
    /// This `Style` will override the `Style` of the [`Row`] and can be overridden by any styles
    /// applied by the widget content.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::style::{Style, Stylize};
    /// use ratatui_widgettable::Cell;
    ///
    /// Cell::from("Cell 1").style(Style::new().red().italic());
    /// ```
    ///
    /// [`Row`]: crate::Row
    /// [`Color`]: ratatui_core::style::Color
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }
}

impl Cell<'_> {
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        // Cells of rows that don't fit in the table (e.g. because of a top margin) get an empty
        // area which may be outside of the table. Some widgets (e.g. `&str`) write to the buffer
        // at the position of the area even if it is empty, so these aren't rendered at all.
        if area.is_empty() {
            return;
        }
        buf.set_style(area, self.style);
        self.content.render_ref(area, buf);
    }
}

impl Default for Cell<'_> {
    fn default() -> Self {
        Self::new(Text::default())
    }
}

impl fmt::Debug for Cell<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cell")
            .field("content", &"<widget>")
            .field("style", &self.style)
            .field("column_span", &self.column_span)
            .finish()
    }
}

impl<'a, T> From<T> for Cell<'a>
where
    T: Into<Text<'a>>,
{
    fn from(content: T) -> Self {
        Self::new(content.into())
    }
}

impl Styled for Cell<'_> {
    type Item = Self;

    fn style(&self) -> Style {
        self.style
    }

    fn set_style<S: Into<Style>>(self, style: S) -> Self::Item {
        self.style(style)
    }
}

#[cfg(test)]
mod tests {
    use ratatui_core::style::{Color, Modifier, Stylize};
    use ratatui_widgets::paragraph::Paragraph;

    use super::*;

    #[test]
    fn style() {
        let style = Style::default().red().italic();
        let cell = Cell::default().style(style);
        assert_eq!(cell.style, style);
    }

    #[test]
    fn stylize() {
        assert_eq!(
            Cell::from("").black().on_white().bold().not_dim().style,
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
                .remove_modifier(Modifier::DIM)
        );
    }

    #[test]
    fn render_text() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        Cell::from("abc").render(buf.area, &mut buf);
        assert_eq!(buf, Buffer::with_lines(["abc  "]));
    }

    #[test]
    fn render_widget() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        Cell::new(Paragraph::new("abc").right_aligned()).render(buf.area, &mut buf);
        assert_eq!(buf, Buffer::with_lines(["  abc"]));
    }

    #[test]
    fn render_fn() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        Cell::from_fn(|area, buf| {
            buf.set_string(area.x + 1, area.y, "x", Style::new());
        })
        .render(buf.area, &mut buf);
        assert_eq!(buf, Buffer::with_lines([" x   "]));
    }
}
