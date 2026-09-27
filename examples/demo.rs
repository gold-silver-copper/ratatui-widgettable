//! An interactive task list where the table cells contain paragraphs, gauges, and sparklines.
//!
//! Run with `cargo run --example demo`.
//!
//! Keys:
//!
//! - `j`/`k` or `↓`/`↑`: select row, `g`/`G`: first/last row
//! - `h`/`l` or `←`/`→`: select column
//! - `space`: start/pause the selected task
//! - `+`/`-`: adjust the progress of the selected task
//! - `enter`: expand/collapse the selected row
//! - `s`: sort by the selected column (press again to reverse)
//! - `a`: add a task, `d`: delete the selected task, `r`: reset the selected task
//! - `p`: cycle the scroll padding
//! - `q`/`esc`: quit
//!
//! Mouse: scroll to move the selection, click a cell to select it, click a header to sort.
//!
//! The mouse support uses [`Cell::from_fn`] to record the area that each cell was rendered into,
//! which is then used to work out which cell was clicked.

use std::cell::RefCell;
use std::io::stdout;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseButton,
    MouseEvent, MouseEventKind,
};
use crossterm::execute;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Gauge, HighlightSpacing, Paragraph, Sparkline, TableState, Widget, Wrap,
};
use ratatui::{DefaultTerminal, Frame};
use ratatui_widgettable::{Cell, Row, Table};

const TICK_RATE: Duration = Duration::from_millis(100);
const HISTORY_LEN: usize = 200;
const COLUMNS: [&str; 5] = ["Status", "Task", "Description", "Progress", "Activity"];

const TEMPLATES: &[(&str, &str)] = &[
    (
        "Update ratatui",
        "Upgrade from 0.25 to 0.30. Check BREAKING-CHANGES.md for all of the changes that need to \
         be made along the way, such as the new crate layout and the renamed highlight styles.",
    ),
    (
        "Switch to tracing",
        "Replace the log crate with tracing and render the logs in the app using tui-tracing so \
         that they don't mess up the terminal.",
    ),
    (
        "Nest widgets",
        "Render a Paragraph inside of a table cell. The paragraph wraps to fit the column width, \
         and the row height decides how many lines are visible.",
    ),
    (
        "Write docs",
        "Document how to use widgets as table cells, including closures created with \
         Cell::from_fn for custom rendering.",
    ),
    (
        "Add mouse support",
        "Record the area each cell is rendered into so that clicks can be mapped back to a row \
         and column.",
    ),
    (
        "Release",
        "Tag the release, publish the crate, and tell everyone on Discord about it.",
    ),
];

fn main() -> Result<()> {
    color_eyre::install()?;
    execute!(stdout(), EnableMouseCapture)?;
    let result = ratatui::run(|terminal| App::new().run(terminal));
    execute!(stdout(), DisableMouseCapture)?;
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Status {
    Running,
    Paused,
    Done,
}

impl Status {
    fn line(self) -> Line<'static> {
        match self {
            Self::Running => Line::from("▶ running").yellow(),
            Self::Paused => Line::from("⏸ paused").dark_gray(),
            Self::Done => Line::from("✔ done").green(),
        }
    }
}

struct Task {
    id: usize,
    name: String,
    description: &'static str,
    progress: f64,
    speed: f64,
    status: Status,
    expanded: bool,
    history: Vec<u64>,
}

impl Task {
    fn new(id: usize) -> Self {
        let (name, description) = TEMPLATES[id % TEMPLATES.len()];
        let name = if id < TEMPLATES.len() {
            name.to_string()
        } else {
            format!("{name} #{}", id / TEMPLATES.len() + 1)
        };
        Self {
            id,
            name,
            description,
            progress: 0.0,
            speed: 0.2 + (id * 37 % 10) as f64 / 10.0,
            status: if id.is_multiple_of(2) {
                Status::Running
            } else {
                Status::Paused
            },
            expanded: false,
            history: Vec::new(),
        }
    }

    fn color(&self) -> Color {
        match self.status {
            Status::Done => Color::Green,
            _ if self.progress >= 50.0 => Color::Yellow,
            _ => Color::Red,
        }
    }

    fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 100.0);
        if self.progress >= 100.0 {
            self.status = Status::Done;
        } else if self.status == Status::Done {
            self.status = Status::Paused;
        }
    }

    fn toggle(&mut self) {
        self.status = match self.status {
            Status::Running => Status::Paused,
            Status::Paused => Status::Running,
            Status::Done => Status::Done,
        };
    }

    fn tick(&mut self, tick: usize) {
        let activity = if self.status == Status::Running {
            // a cheap pseudo random number so that the sparklines look interesting
            let noise = (tick * 7 + self.id * 13) % 10;
            self.set_progress(self.progress + self.speed * (noise as f64 + 1.0) / 15.0);
            noise as u64 + 1
        } else {
            0
        };
        self.history.push(activity);
        if self.history.len() > HISTORY_LEN {
            self.history.remove(0);
        }
    }

    fn recent_activity(&self) -> u64 {
        self.history.iter().rev().take(20).sum()
    }
}

/// Where a cell was rendered on screen, used to handle mouse clicks.
#[derive(Debug, Clone, Copy)]
enum Hit {
    Header { column: usize },
    Cell { row: usize, column: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sort {
    column: usize,
    reversed: bool,
}

struct App {
    tasks: Vec<Task>,
    next_id: usize,
    state: TableState,
    sort: Option<Sort>,
    scroll_padding: usize,
    tick: usize,
    /// The areas that cells were rendered into during the last frame.
    hits: RefCell<Vec<(Hit, Rect)>>,
}

impl App {
    fn new() -> Self {
        let tasks: Vec<Task> = (0..TEMPLATES.len()).map(Task::new).collect();
        Self {
            next_id: tasks.len(),
            tasks,
            state: TableState::new().with_selected_cell((0, 1)),
            sort: None,
            scroll_padding: 1,
            tick: 0,
            hits: RefCell::default(),
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|frame| self.render(frame))?;
            let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) if key.is_press() => {
                        if !self.handle_key(key) {
                            return Ok(());
                        }
                    }
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    _ => {}
                }
            }
            if last_tick.elapsed() >= TICK_RATE {
                self.on_tick();
                last_tick = Instant::now();
            }
        }
    }

    fn on_tick(&mut self) {
        self.tick += 1;
        for task in &mut self.tasks {
            task.tick(self.tick);
        }
    }

    /// Handles a key press, returning `false` if the app should quit.
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Char('j') | KeyCode::Down => self.state.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.state.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.state.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.state.select_last(),
            KeyCode::Char('l') | KeyCode::Right => self.state.select_next_column(),
            KeyCode::Char('h') | KeyCode::Left => self.state.select_previous_column(),
            KeyCode::Char(' ') => self.with_selected(Task::toggle),
            KeyCode::Char('+' | '=') => self.with_selected(|t| t.set_progress(t.progress + 10.0)),
            KeyCode::Char('-') => self.with_selected(|t| t.set_progress(t.progress - 10.0)),
            KeyCode::Char('r') => self.with_selected(|t| {
                t.set_progress(0.0);
                t.status = Status::Paused;
                t.history.clear();
            }),
            KeyCode::Enter => self.with_selected(|t| t.expanded = !t.expanded),
            KeyCode::Char('s') => {
                if let Some(column) = self.state.selected_column() {
                    self.sort_by(column);
                }
            }
            KeyCode::Char('a') => self.add_task(),
            KeyCode::Char('d') => self.delete_task(),
            KeyCode::Char('p') => self.scroll_padding = (self.scroll_padding + 1) % 4,
            _ => {}
        }
        true
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.state.select_next(),
            MouseEventKind::ScrollUp => self.state.select_previous(),
            MouseEventKind::Down(MouseButton::Left) => {
                let position = Position::new(mouse.column, mouse.row);
                let hit = self
                    .hits
                    .borrow()
                    .iter()
                    .find(|(_, area)| area.contains(position))
                    .map(|(hit, _)| *hit);
                match hit {
                    Some(Hit::Header { column }) => self.sort_by(column),
                    Some(Hit::Cell { row, column }) => {
                        if self.state.selected_cell() == Some((row, column)) {
                            // clicking the selected cell again toggles the task
                            self.with_selected(Task::toggle);
                        }
                        self.state.select_cell(Some((row, column)));
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }

    fn selected_index(&self) -> Option<usize> {
        self.state
            .selected()
            .map(|i| i.min(self.tasks.len().saturating_sub(1)))
            .filter(|_| !self.tasks.is_empty())
    }

    fn with_selected(&mut self, f: impl FnOnce(&mut Task)) {
        if let Some(i) = self.selected_index() {
            f(&mut self.tasks[i]);
        }
    }

    fn add_task(&mut self) {
        let index = self.selected_index().map_or(0, |i| i + 1);
        self.tasks.insert(index, Task::new(self.next_id));
        self.next_id += 1;
        self.state.select(Some(index));
    }

    fn delete_task(&mut self) {
        if let Some(i) = self.selected_index() {
            self.tasks.remove(i);
        }
    }

    /// Sorts the tasks by the given column, reversing the order if it is already sorted by it.
    fn sort_by(&mut self, column: usize) {
        let reversed = self.sort
            == Some(Sort {
                column,
                reversed: false,
            });
        self.sort = Some(Sort { column, reversed });
        let selected_id = self.selected_index().map(|i| self.tasks[i].id);
        match column {
            0 => self.tasks.sort_by_key(|t| t.status),
            1 => self.tasks.sort_by(|a, b| a.name.cmp(&b.name)),
            2 => self.tasks.sort_by_key(|t| t.description.len()),
            3 => self.tasks.sort_by(|a, b| a.progress.total_cmp(&b.progress)),
            _ => self.tasks.sort_by_key(Task::recent_activity),
        }
        if reversed {
            self.tasks.reverse();
        }
        // keep the same task selected after sorting
        if let Some(id) = selected_id {
            self.state
                .select(self.tasks.iter().position(|t| t.id == id));
        }
        self.state.select_column(Some(column));
    }

    fn render(&mut self, frame: &mut Frame) {
        let [table_area, status_area, help_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .areas(frame.area());

        self.hits.borrow_mut().clear();
        let hits = &self.hits;
        let rows = self
            .tasks
            .iter()
            .enumerate()
            .map(|(row, task)| task_row(task, row, hits));
        let header = Row::new(COLUMNS.iter().enumerate().map(|(column, title)| {
            let arrow = match self.sort {
                Some(Sort {
                    column: c,
                    reversed,
                }) if c == column => {
                    if reversed {
                        " ▼"
                    } else {
                        " ▲"
                    }
                }
                _ => "",
            };
            let title = Line::from(format!("{title}{arrow}")).bold();
            tracked(hits, Hit::Header { column }, move |area, buf| {
                (&title).render(area, buf);
            })
        }))
        .bottom_margin(1);

        let widths = [
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Fill(2),
            Constraint::Length(16),
            Constraint::Fill(1),
        ];
        let done = self
            .tasks
            .iter()
            .filter(|t| t.status == Status::Done)
            .count();
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" ratatui-widgettable demo ".bold())
            .title(Line::from(format!(" {done}/{} done ", self.tasks.len())).right_aligned());
        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .row_highlight_style(Style::new().bg(Color::Indexed(236)))
            .column_highlight_style(Style::new().bg(Color::Indexed(234)))
            .cell_highlight_style(Style::new().bg(Color::Indexed(239)).bold())
            .highlight_symbol(Line::from("▌ ").cyan())
            .highlight_spacing(HighlightSpacing::Always)
            .scroll_padding(self.scroll_padding);
        frame.render_stateful_widget(table, table_area, &mut self.state);

        frame.render_widget(self.status_line(), status_area);
        let help = Paragraph::new(help_line()).wrap(Wrap { trim: true });
        frame.render_widget(help, help_area);
    }

    fn status_line(&self) -> Line<'static> {
        let task = self
            .selected_index()
            .map_or("none".to_string(), |i| self.tasks[i].name.clone());
        let column = self
            .state
            .selected_column()
            .and_then(|c| COLUMNS.get(c))
            .unwrap_or(&"none");
        let sort = self.sort.map_or("none".to_string(), |s| {
            let order = if s.reversed { "desc" } else { "asc" };
            format!("{} {order}", COLUMNS[s.column])
        });
        Line::from(vec![
            " task: ".dark_gray(),
            task.cyan(),
            "  column: ".dark_gray(),
            column.to_string().cyan(),
            "  sort: ".dark_gray(),
            sort.cyan(),
            "  scroll padding: ".dark_gray(),
            self.scroll_padding.to_string().cyan(),
        ])
    }
}

/// Builds the row for a task. Every cell is wrapped with [`tracked`] so that it can be clicked.
fn task_row<'a>(task: &'a Task, row: usize, hits: &'a RefCell<Vec<(Hit, Rect)>>) -> Row<'a> {
    let cell = |column, render: RenderFn<'a>| tracked(hits, Hit::Cell { row, column }, render);

    let status = task.status.line();
    let name = Paragraph::new(vec![
        Line::from(task.name.as_str()).bold(),
        Line::from(format!("#{}", task.id)).dark_gray(),
    ]);
    let description = Paragraph::new(task.description)
        .wrap(Wrap { trim: true })
        .dark_gray();
    let gauge = Gauge::default()
        .ratio(task.progress / 100.0)
        .label(format!("{:.0}%", task.progress))
        .gauge_style(task.color())
        .use_unicode(true)
        .block(Block::bordered().border_type(BorderType::Rounded));
    let color = task.color();

    Row::new(vec![
        cell(0, Box::new(move |area, buf| (&status).render(area, buf))),
        cell(1, Box::new(move |area, buf| (&name).render(area, buf))),
        cell(
            2,
            Box::new(move |area, buf| (&description).render(area, buf)),
        ),
        cell(3, Box::new(move |area, buf| (&gauge).render(area, buf))),
        // Only show as much history as fits in the cell
        cell(
            4,
            Box::new(move |area, buf| {
                let start = task.history.len().saturating_sub(area.width as usize);
                Sparkline::default()
                    .data(&task.history[start..])
                    .max(10)
                    .style(color)
                    .render(area, buf);
            }),
        ),
    ])
    .height(if task.expanded { 6 } else { 3 })
    .bottom_margin(1)
}

type RenderFn<'a> = Box<dyn Fn(Rect, &mut Buffer) + 'a>;

/// Creates a cell which records the area it was rendered into, so that clicks can be handled.
fn tracked<'a>(
    hits: &'a RefCell<Vec<(Hit, Rect)>>,
    hit: Hit,
    render: impl Fn(Rect, &mut Buffer) + 'a,
) -> Cell<'a> {
    Cell::from_fn(move |area, buf| {
        hits.borrow_mut().push((hit, area));
        render(area, buf);
    })
}

fn help_line() -> Line<'static> {
    let keys = [
        ("↑↓/jk", "row"),
        ("←→/hl", "column"),
        ("space", "start/pause"),
        ("+/-", "progress"),
        ("enter", "expand"),
        ("s", "sort"),
        ("a/d", "add/delete"),
        ("r", "reset"),
        ("p", "padding"),
        ("q", "quit"),
    ];
    let spans = keys.iter().flat_map(|(key, action)| {
        [
            Span::from(format!(" {key} ")).black().on_cyan(),
            Span::from(format!(" {action}  ")).dark_gray(),
        ]
    });
    Line::from_iter(spans)
}
