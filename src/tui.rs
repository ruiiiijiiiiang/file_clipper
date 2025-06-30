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
use std::{env, error::Error, io::stdout, time::Duration};

use crate::file_handler::{handle_paste, handle_remove};
use crate::models::{PasteContent, RecordEntry, RecordType};
use crate::record_handler::{read_clipboard, read_history};
use crate::utils::get_metadata;

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
    pub fn new(mode: RecordType) -> Result<Self, Box<dyn Error>> {
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

    pub fn run(mut self) -> Result<(), Box<dyn Error>> {
        let mut terminal = ratatui::init_with_options(TerminalOptions {
            viewport: Viewport::Inline(HEIGHT),
        });
        loop {
            if self.should_exit {
                break;
            }
            terminal.draw(|frame| {
                let area = frame.area();
                self.render_table(frame, area);
                self.render_scrollbar(frame, area);
            })?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    event::Event::Key(key) => self.handle_keypress(key)?,
                    event::Event::Resize(_, _) => terminal.autoresize()?,
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
        )?;
        ratatui::restore();
        Ok(())
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["Path", "Timestamp", "Operation"]
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

    fn handle_keypress(&mut self, key: KeyEvent) -> Result<(), Box<dyn Error>> {
        match key {
            KeyEvent {
                code:
                    KeyCode::Char('h')
                    | KeyCode::Char('l')
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Char(' '),
                ..
            } => self.mark(),
            KeyEvent {
                code: KeyCode::Char('j') | KeyCode::Down,
                ..
            } => self.next(1),
            KeyEvent {
                code: KeyCode::Char('k') | KeyCode::Up,
                ..
            } => self.previous(1),
            KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => self.next(HEIGHT / 2),
            KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => self.previous(HEIGHT / 2),
            KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => self.next(HEIGHT),
            KeyEvent {
                code: KeyCode::Char('b'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => self.previous(HEIGHT),
            KeyEvent {
                code: KeyCode::Char('g'),
                ..
            } => self.top(),
            KeyEvent {
                code: KeyCode::Char('G'),
                ..
            } => self.bottom(),
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
            } => self.exit(),
            _ => Ok(()),
        }
    }

    fn next(&mut self, num_lines: u16) -> Result<(), Box<dyn Error>> {
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
        Ok(())
    }

    fn previous(&mut self, num_lines: u16) -> Result<(), Box<dyn Error>> {
        let num_lines = num_lines as usize;
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(num_lines),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
        Ok(())
    }

    fn top(&mut self) -> Result<(), Box<dyn Error>> {
        self.table_state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
        Ok(())
    }

    fn bottom(&mut self) -> Result<(), Box<dyn Error>> {
        self.table_state.select(Some(self.entries.len() - 1));
        self.scroll_state = self.scroll_state.position(self.entries.len() - 1);
        Ok(())
    }

    fn mark(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(selected) = self.table_state.selected() {
            if !self.invalid[selected] {
                self.marked[selected] = !self.marked[selected];
            }
        }
        Ok(())
    }

    fn remove(&mut self) -> Result<(), Box<dyn Error>> {
        if self.mode == RecordType::Clipboard {
            if let Some(selected) = self.table_state.selected() {
                handle_remove(self.entries[selected].id)?;
            }
        }
        Ok(())
    }

    fn paste(&mut self) -> Result<(), Box<dyn Error>> {
        let destination_path = env::current_dir()?;
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
        handle_paste(destination_path, Some(paste_content))?;
        self.exit()?;
        Ok(())
    }

    fn exit(&mut self) -> Result<(), Box<dyn Error>> {
        self.should_exit = true;
        Ok(())
    }
}
