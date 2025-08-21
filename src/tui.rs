use chrono::{DateTime, Local};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{
        palette::tailwind::{BLUE, NEUTRAL, TEAL},
        Modifier, Style, Stylize,
    },
    text::Line,
    widgets::{
        Block, Borders, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame, TerminalOptions, Viewport,
};
use std::{env::current_dir, time::Duration};

use crate::{
    errors::{AppError, AppInfo, AppWarning, FileError, TuiError},
    files::{get_metadata, handle_paste},
    models::{PasteContent, RecordEntry, RecordType},
    records::{handle_remove, read_entries},
};

const HEIGHT: u16 = 20;
const OPERATION_WIDTH: u16 = 10;
const SELECTED_WIDTH: u16 = 8;
const TIMESTAMP_WIDTH: u16 = 30;
const POLL_INTERVAL: u64 = 100;
const CLIPBOARD_HELPER_TEXT: &str = "Navigation: j/k; Select: space; Paste: p; Remove: x; Quit: q";
const HISTORY_HELPER_TEXT: &str = "Navigation: j/k; Select: space; Paste: p; Quit: q";

pub struct Tui {
    pub entries: Vec<RecordEntry>,
    pub mode: RecordType,
    pub table_state: TableState,
    pub scroll_state: ScrollbarState,
    pub invalid: Vec<bool>,
    pub marked: Vec<bool>,
    pub should_exit: bool,
    pub warnings: Vec<AppWarning>,
    pub infos: Vec<AppInfo>,
    pub paste_content: Option<PasteContent>,
}

type ColumnDef<'a> = (
    &'static str,
    Constraint,
    Box<dyn Fn(usize, &RecordEntry) -> String + 'a>,
);

impl Tui {
    pub fn new(mode: RecordType) -> Result<Self, AppError> {
        let entries = read_entries(&mode)?;
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
            warnings: Vec::new(),
            infos: Vec::new(),
            paste_content: None,
        })
    }

    pub fn run(mut self) -> Result<(Vec<AppInfo>, Vec<AppWarning>), AppError> {
        let mut terminal = ratatui::init_with_options(TerminalOptions {
            viewport: Viewport::Inline(HEIGHT),
        });

        let loop_result = (|| {
            loop {
                if self.should_exit {
                    break;
                }

                terminal
                    .draw(|frame| {
                        self.render_ui(frame, frame.area());
                    })
                    .map_err(|source| TuiError::TerminalDraw { source })?;

                if event::poll(Duration::from_millis(POLL_INTERVAL))
                    .map_err(|source| TuiError::EventPolling { source })?
                {
                    match event::read().map_err(|source| TuiError::EventRead { source })? {
                        Event::Key(key) => self.handle_keypress(key)?,
                        Event::Resize(_, _) => terminal
                            .autoresize()
                            .map_err(|source| TuiError::TerminalAutoresize { source })?,
                        _ => {}
                    };
                }
            }
            Ok(())
        })();

        let _ = terminal.clear();
        ratatui::restore();

        if let Some(paste_content) = self.paste_content {
            let destination_path = current_dir().map_err(|source| FileError::Cwd { source })?;
            match handle_paste(destination_path, Some(paste_content)) {
                Err(error) => return Err(error),
                Ok((infos, warnings)) => {
                    self.infos.extend(infos);
                    self.warnings.extend(warnings);
                }
            }
        }

        match loop_result {
            Ok(_) => Ok((self.infos, self.warnings)),
            Err(error) => Err(error),
        }
    }

    fn render_ui(&mut self, frame: &mut Frame, area: Rect) {
        self.render_table(frame, area);
        self.render_scrollbar(frame, area);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let column_definitions: [ColumnDef; 4] = [
            (
                "Selected",
                Constraint::Length(SELECTED_WIDTH),
                Box::new(|index, _| {
                    if self.marked[index] {
                        "[X]".to_string()
                    } else {
                        "[ ]".to_string()
                    }
                }),
            ),
            (
                "Operation",
                Constraint::Length(OPERATION_WIDTH),
                Box::new(|_, entry| entry.operation.to_string()),
            ),
            (
                "Accessed",
                Constraint::Length(TIMESTAMP_WIDTH),
                Box::new(|_, entry| {
                    let local_datetime: DateTime<Local> = entry.timestamp.into();
                    local_datetime.format("%a, %d %b %Y %H:%M:%S").to_string()
                }),
            ),
            (
                "Path",
                Constraint::Min(0),
                Box::new(|_, entry| entry.path.to_string_lossy().into_owned()),
            ),
        ];

        let header = column_definitions
            .iter()
            .map(|(header, _, _)| Cell::from(*header))
            .collect::<Row>()
            .style(
                Style::default()
                    .bg(NEUTRAL.c700)
                    .fg(NEUTRAL.c300)
                    .add_modifier(Modifier::BOLD),
            )
            .height(1);

        let constraints: Vec<Constraint> = column_definitions
            .iter()
            .map(|(_, constraint, _)| *constraint)
            .collect();

        let rows = self.entries.iter().enumerate().map(|(index, entry)| {
            let valid = get_metadata(&entry.path).is_ok();
            self.invalid[index] = !valid;

            let style = if !valid {
                Style::default().fg(NEUTRAL.c500).crossed_out()
            } else if self.marked[index] {
                Style::default().fg(TEAL.c300)
            } else {
                Style::default()
            };

            let cells = column_definitions
                .iter()
                .map(|(_, _, render_entry)| Cell::from(render_entry(index, entry)));
            Row::new(cells).style(style)
        });

        let table = Table::new(rows, constraints)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title_top(format!("File Clipper - {}", self.mode))
                    .title_bottom(
                        Line::from(if self.mode == RecordType::Clipboard {
                            CLIPBOARD_HELPER_TEXT
                        } else {
                            HISTORY_HELPER_TEXT
                        })
                        .centered(),
                    ),
            )
            .header(header)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(Style::default().bg(BLUE.c800));

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
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
                code: KeyCode::Char('a'),
                ..
            } => {
                self.mark_all();
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
                code: KeyCode::Char('x') | KeyCode::Char('d'),
                ..
            } => self.remove(),
            KeyEvent {
                code: KeyCode::Char('p') | KeyCode::Enter,
                ..
            } => self.paste(),
            KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: event::KeyModifiers::CONTROL,
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
                if i < self.entries.len().saturating_sub(num_lines) {
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

    fn mark_all(&mut self) {
        if self.marked.iter().any(|marked| !marked) {
            self.marked = vec![true; self.entries.len()];
        } else {
            self.marked = vec![false; self.entries.len()];
        }
    }

    fn remove(&mut self) -> Result<(), AppError> {
        if self.mode == RecordType::Clipboard {
            if let Some(selected) = self.table_state.selected() {
                match handle_remove(self.entries[selected].id) {
                    Err(error) => return Err(error),
                    Ok(warnings) => {
                        self.warnings.extend(warnings);
                    }
                }
            }
            self.entries = read_entries(&self.mode)?;
        }
        Ok(())
    }

    fn paste(&mut self) -> Result<(), AppError> {
        let mut marked_entries: Vec<RecordEntry> = self
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
        if marked_entries.is_empty() {
            if let Some(selected) = self.table_state.selected() {
                marked_entries.push(self.entries[selected].clone());
            }
        }
        let paste_content = PasteContent {
            entries: marked_entries,
            source: self.mode.clone(),
        };
        self.paste_content = Some(paste_content);
        self.exit();
        Ok(())
    }

    fn exit(&mut self) {
        self.should_exit = true;
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::create_test_tui;

    #[test]
    fn test_tui_navigation_next() {
        let mut tui = create_test_tui(10);
        assert_eq!(tui.table_state.selected(), Some(0));

        tui.next(1);
        assert_eq!(tui.table_state.selected(), Some(1));

        tui.next(5);
        assert_eq!(tui.table_state.selected(), Some(6));

        tui.next(10);
        assert_eq!(tui.table_state.selected(), Some(9));
    }

    #[test]
    fn test_tui_navigation_previous() {
        let mut tui = create_test_tui(10);
        tui.table_state.select(Some(9)); // Start at the bottom

        tui.previous(1);
        assert_eq!(tui.table_state.selected(), Some(8));

        tui.previous(5);
        assert_eq!(tui.table_state.selected(), Some(3));

        tui.previous(10);
        assert_eq!(tui.table_state.selected(), Some(0));
    }

    #[test]
    fn test_tui_navigation_top_and_bottom() {
        let mut tui = create_test_tui(20);
        assert_eq!(tui.table_state.selected(), Some(0));

        tui.bottom();
        assert_eq!(tui.table_state.selected(), Some(19));

        tui.top();
        assert_eq!(tui.table_state.selected(), Some(0));
    }

    #[test]
    fn test_tui_mark() {
        let mut tui = create_test_tui(5);
        tui.table_state.select(Some(2));

        assert!(!tui.marked[2]);
        tui.mark();
        assert!(tui.marked[2]);
        tui.mark();
        assert!(!tui.marked[2]);
    }

    #[test]
    fn test_tui_mark_invalid_entry() {
        let mut tui = create_test_tui(5);
        tui.table_state.select(Some(2));
        tui.invalid[2] = true;

        assert!(!tui.marked[2]);
        tui.mark();
        assert!(!tui.marked[2]);
    }

    #[test]
    fn test_tui_mark_all() {
        let mut tui = create_test_tui(5);

        tui.mark_all();
        assert!(tui.marked.iter().all(|&m| m));

        tui.mark_all();
        assert!(tui.marked.iter().all(|&m| !m));
    }

    #[test]
    fn test_tui_mark_all_some_marked() {
        let mut tui = create_test_tui(5);
        tui.marked[0] = true;
        tui.marked[2] = true;

        tui.mark_all();
        assert!(tui.marked.iter().all(|&m| m));
    }

    #[test]
    fn test_tui_exit() {
        let mut tui = create_test_tui(5);
        assert!(!tui.should_exit);
        tui.exit();
        assert!(tui.should_exit);
    }
}
