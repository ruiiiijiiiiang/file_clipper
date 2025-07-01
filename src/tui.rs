use chrono::{DateTime, Local};
use crossterm::{
    cursor::{MoveToColumn, MoveUp, Show},
    event::{self, KeyCode, KeyEvent},
    execute,
    terminal::{Clear, ClearType},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{
        palette::tailwind::{BLUE, EMERALD, GRAY},
        Modifier, Style,
    },
    widgets::{
        Block, Borders, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame, TerminalOptions, Viewport,
};
use std::{env, io::stdout, time::Duration};

use crate::{
    exceptions::{AppError, FileError, TuiError},
    file_handler::{handle_paste, handle_remove},
    models::{PasteContent, RecordEntry, RecordType},
    record_handler::{read_clipboard, read_history},
    utils::get_metadata,
};

const HEIGHT: u16 = 20;
const TIMESTAMP_WIDTH: u16 = 30;
const OPERATION_WIDTH: u16 = 10;

pub struct App {
    pub entries: Vec<RecordEntry>,
    pub mode: RecordType,
    pub table_state: TableState,
    pub scroll_state: ScrollbarState,
    pub invalid: Vec<bool>,
    pub marked: Vec<bool>,
    pub should_exit: bool,
}

impl App {
    pub fn new(mode: RecordType) -> Result<Self, AppError> {
        let entries = match mode {
            RecordType::Clipboard => read_clipboard()?.unwrap_or(vec![]),
            RecordType::History => read_history()?.unwrap_or(vec![]),
        };
        if entries.is_empty() {
            println!("[Info]: {} is empty", mode);
        }
        Ok(Self {
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(entries.len().saturating_sub(1)),
            invalid: vec![false; entries.len()],
            marked: vec![false; entries.len()],
            should_exit: entries.is_empty(),
            entries,
            mode,
        })
    }

    pub fn run(mut self) -> Result<(), AppError> {
        let mut terminal = ratatui::init_with_options(TerminalOptions {
            viewport: Viewport::Inline(HEIGHT),
        });

        loop {
            if self.should_exit {
                break;
            }
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    self.render_table(frame, area);
                    self.render_scrollbar(frame, area);
                })
                .map_err(|error| TuiError::TerminalDraw { source: error })?;

            if event::poll(Duration::from_millis(100))
                .map_err(|error| TuiError::EventPolling { source: error })?
            {
                match event::read().map_err(|error| TuiError::EventRead { source: error })? {
                    event::Event::Key(key) => self.handle_keypress(key)?,
                    event::Event::Resize(_, _) => terminal
                        .autoresize()
                        .map_err(|error| TuiError::TerminalAutoresize { source: error })?,
                    _ => {}
                };
            }
        }

        execute!(
            stdout(),
            MoveToColumn(0),
            MoveUp(HEIGHT - 1),
            Clear(ClearType::FromCursorDown),
            Show
        )
        .map_err(|error| TuiError::TerminalCommand { source: error })?;
        ratatui::restore();
        Ok(())
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["Path", "Accessed", "Operation"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(Style::default().add_modifier(Modifier::BOLD))
            .height(1);

        let rows = self.entries.iter().enumerate().map(|(index, entry)| {
            let valid = get_metadata(&entry.path).is_ok();
            self.invalid[index] = !valid;

            let style = if !valid {
                Style::default().fg(GRAY.c500)
            } else if self.marked[index] {
                Style::default().fg(EMERALD.c300)
            } else {
                Style::default()
            };

            let path_display = entry.path.to_string_lossy().into_owned();

            let local_datetime: DateTime<Local> = entry.timestamp.into();
            let datetime_display = local_datetime.format("%a, %d %b %Y %H:%M:%S").to_string();

            let operation_display = entry.operation.to_string();

            let cells = [path_display, datetime_display, operation_display]
                .into_iter()
                .map(Cell::from);
            Row::new(cells).style(style)
        });

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let table = Table::new(
            rows,
            [
                Constraint::Min(0),
                Constraint::Length(TIMESTAMP_WIDTH),
                Constraint::Length(OPERATION_WIDTH),
            ],
        )
        .block(
            Block::default()
                .title(format!("File Clipper - {}", self.mode))
                .borders(Borders::ALL),
        )
        .header(header)
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(Style::default().bg(BLUE.c800));
        frame.render_stateful_widget(table, chunks[0], &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 0,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn handle_keypress(&mut self, key: KeyEvent) -> Result<(), AppError> {
        match key {
            KeyEvent {
                code:
                    KeyCode::Char('h')
                    | KeyCode::Char('l')
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Char(' '),
                ..
            } => {
                self.mark();
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('j') | KeyCode::Down,
                ..
            } => {
                self.next(1);
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('k') | KeyCode::Up,
                ..
            } => {
                self.previous(1);
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => {
                self.next(HEIGHT / 2);
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => {
                self.previous(HEIGHT / 2);
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => {
                self.next(HEIGHT);
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('b'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => {
                self.previous(HEIGHT);
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('g'),
                ..
            } => {
                self.top();
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('G'),
                ..
            } => {
                self.bottom();
                Ok(())
            }
            KeyEvent {
                code: KeyCode::Char('x'),
                ..
            } => self.remove(),
            KeyEvent {
                code: KeyCode::Char('d'),
                ..
            } => self.remove(),
            KeyEvent {
                code: KeyCode::Char('p') | KeyCode::Enter,
                ..
            } => self.paste(),
            KeyEvent {
                code: KeyCode::Char('q'),
                ..
            } => {
                self.exit();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn next(&mut self, num_lines: u16) {
        let num_lines = num_lines as usize;
        let i = match self.table_state.selected() {
            Some(i) => {
                if i < self.entries.len() - num_lines {
                    i + num_lines
                } else {
                    self.entries.len() - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    fn previous(&mut self, num_lines: u16) {
        let num_lines = num_lines as usize;
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(num_lines),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    fn top(&mut self) {
        self.table_state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    fn bottom(&mut self) {
        self.table_state.select(Some(self.entries.len() - 1));
        self.scroll_state = self.scroll_state.position(self.entries.len() - 1);
    }

    fn mark(&mut self) {
        if let Some(selected) = self.table_state.selected() {
            if !self.invalid[selected] {
                self.marked[selected] = !self.marked[selected];
            }
        }
    }

    fn remove(&mut self) -> Result<(), AppError> {
        if self.mode == RecordType::Clipboard {
            if let Some(selected) = self.table_state.selected() {
                match handle_remove(self.entries[selected].id) {
                    Err(error) => Err(error),
                    Ok(Some(warnings)) => todo!("show warnings"),
                    _ => Ok(()),
                };
            }
        }
        Ok(())
    }

    fn paste(&mut self) -> Result<(), AppError> {
        let destination_path =
            env::current_dir().map_err(|error| FileError::Cwd { source: error })?;
        let marked_entries = self
            .entries
            .clone()
            .into_iter()
            .zip(self.marked.clone())
            .filter_map(
                |(entry, selected)| {
                    if selected {
                        Some(entry)
                    } else {
                        None
                    }
                },
            )
            .collect();
        let paste_content = PasteContent {
            entries: marked_entries,
            source: self.mode.clone(),
        };
        match handle_paste(destination_path, Some(paste_content)) {
            Err(error) => Err(error),
            Ok(Some(warnings)) => todo!("show warnings"),
            _ => Ok(()),
        };
        self.exit();
        Ok(())
    }

    fn exit(&mut self) {
        self.should_exit = true;
    }
}
