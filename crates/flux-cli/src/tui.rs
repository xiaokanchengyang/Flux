//! Terminal User Interface for archive browsing

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use flux_core::archive::ArchiveEntry;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;

/// TUI application state
pub struct App {
    /// Archive entries to display
    entries: Vec<ArchiveEntry>,
    /// Currently selected index
    selected: usize,
    /// List state for scrolling
    list_state: ListState,
    /// Search query
    search_query: String,
    /// Filtered entries based on search
    filtered_entries: Vec<usize>,
    /// Show help
    show_help: bool,
}

impl App {
    /// Create a new TUI app
    pub fn new(entries: Vec<ArchiveEntry>) -> Self {
        let filtered_entries: Vec<usize> = (0..entries.len()).collect();
        let mut list_state = ListState::default();
        if !entries.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            entries,
            selected: 0,
            list_state,
            search_query: String::new(),
            filtered_entries,
            show_help: false,
        }
    }

    /// Move selection up
    fn move_up(&mut self) {
        if self.filtered_entries.is_empty() {
            return;
        }

        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.filtered_entries.len() - 1;
        }
        self.list_state.select(Some(self.selected));
    }

    /// Move selection down
    fn move_down(&mut self) {
        if self.filtered_entries.is_empty() {
            return;
        }

        if self.selected < self.filtered_entries.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
        self.list_state.select(Some(self.selected));
    }

    /// Update search and filter entries
    fn update_search(&mut self, ch: char) {
        self.search_query.push(ch);
        self.filter_entries();
    }

    /// Remove last character from search
    fn backspace_search(&mut self) {
        self.search_query.pop();
        self.filter_entries();
    }

    /// Clear search
    fn clear_search(&mut self) {
        self.search_query.clear();
        self.filter_entries();
    }

    /// Filter entries based on search query
    fn filter_entries(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_entries = (0..self.entries.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_entries = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.path.to_string_lossy().to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        }

        // Reset selection
        self.selected = 0;
        if !self.filtered_entries.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    /// Get currently selected entry
    fn selected_entry(&self) -> Option<&ArchiveEntry> {
        if self.filtered_entries.is_empty() {
            return None;
        }

        self.filtered_entries
            .get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }
}

/// Run the TUI application
pub fn run_tui(entries: Vec<ArchiveEntry>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(entries);

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

/// Main application loop
fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') | KeyCode::F(1) => app.show_help = !app.show_help,
                    KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                    KeyCode::PageUp => {
                        for _ in 0..10 {
                            app.move_up();
                        }
                    }
                    KeyCode::PageDown => {
                        for _ in 0..10 {
                            app.move_down();
                        }
                    }
                    KeyCode::Home => {
                        app.selected = 0;
                        if !app.filtered_entries.is_empty() {
                            app.list_state.select(Some(0));
                        }
                    }
                    KeyCode::End => {
                        if !app.filtered_entries.is_empty() {
                            app.selected = app.filtered_entries.len() - 1;
                            app.list_state.select(Some(app.selected));
                        }
                    }
                    KeyCode::Char('/') => {
                        app.clear_search();
                    }
                    KeyCode::Esc => {
                        if !app.search_query.is_empty() {
                            app.clear_search();
                        } else {
                            app.show_help = false;
                        }
                    }
                    KeyCode::Backspace => {
                        if !app.search_query.is_empty() {
                            app.backspace_search();
                        }
                    }
                    KeyCode::Char(c) => {
                        if (!app.search_query.is_empty() || key.code == KeyCode::Char('/'))
                            && c != '/'
                        {
                            app.update_search(c);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Render the UI
fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ]
            .as_ref(),
        )
        .split(f.area());

    // Header
    let header = if app.search_query.is_empty() {
        Paragraph::new(format!(
            "Flux Archive Browser - {} entries",
            app.entries.len()
        ))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL))
    } else {
        Paragraph::new(format!(
            "Search: {} - {} matches",
            app.search_query,
            app.filtered_entries.len()
        ))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL))
    };
    f.render_widget(header, chunks[0]);

    // Main content area
    if app.show_help {
        render_help(f, chunks[1]);
    } else {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)].as_ref())
            .split(chunks[1]);

        render_file_list(f, app, content_chunks[0]);
        render_file_details(f, app, content_chunks[1]);
    }

    // Footer
    let footer = Paragraph::new("q: Quit | /: Search | ↑↓: Navigate | ?: Help")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

/// Render the file list
fn render_file_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_entries
        .iter()
        .filter_map(|&idx| app.entries.get(idx))
        .map(|entry| {
            let style = if entry.is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_symlink {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let text = if entry.is_dir {
                format!("📁 {}/", entry.path.display())
            } else if entry.is_symlink {
                format!("🔗 {}", entry.path.display())
            } else {
                format!("📄 {}", entry.path.display())
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Files")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.list_state.clone());
}

/// Render file details panel
fn render_file_details(f: &mut Frame, app: &App, area: Rect) {
    let selected = match app.selected_entry() {
        Some(entry) => entry,
        None => {
            let empty = Paragraph::new("No file selected")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(Block::default().title("Details").borders(Borders::ALL));
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines = vec![];

    // File type
    let file_type = if selected.is_dir {
        "Directory"
    } else if selected.is_symlink {
        "Symbolic Link"
    } else {
        "File"
    };
    lines.push(Line::from(vec![
        Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(file_type),
    ]));

    // Size
    let size_str = format_size(selected.size);
    lines.push(Line::from(vec![
        Span::styled("Size: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(size_str),
    ]));

    // Compressed size
    if let Some(compressed) = selected.compressed_size {
        let compressed_str = format_size(compressed);
        let ratio = if selected.size > 0 {
            100.0 - (compressed as f64 / selected.size as f64 * 100.0)
        } else {
            0.0
        };
        lines.push(Line::from(vec![
            Span::styled(
                "Compressed: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{} ({:.1}% saved)", compressed_str, ratio)),
        ]));
    }

    // Permissions
    if let Some(mode) = selected.mode {
        lines.push(Line::from(vec![
            Span::styled(
                "Permissions: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:o}", mode)),
        ]));
    }

    // Modified time
    if let Some(mtime) = selected.mtime {
        let datetime =
            chrono::DateTime::<chrono::Utc>::from_timestamp(mtime, 0).unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled("Modified: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(datetime.format("%Y-%m-%d %H:%M:%S").to_string()),
        ]));
    }

    // Link target
    if let Some(target) = &selected.link_target {
        lines.push(Line::from(vec![
            Span::styled(
                "Link Target: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(target.display().to_string()),
        ]));
    }

    let details = Paragraph::new(lines)
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(details, area);
}

/// Render help screen
fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Flux Archive Browser - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ↑/k         - Move up"),
        Line::from("  ↓/j         - Move down"),
        Line::from("  PageUp      - Move up 10 items"),
        Line::from("  PageDown    - Move down 10 items"),
        Line::from("  Home        - Go to first item"),
        Line::from("  End         - Go to last item"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  /           - Start search"),
        Line::from("  <text>      - Type to search"),
        Line::from("  Backspace   - Remove last character"),
        Line::from("  Esc         - Clear search"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Other:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ?/F1        - Toggle this help"),
        Line::from("  q           - Quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "File Icons:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  📁          - Directory"),
        Line::from("  📄          - Regular file"),
        Line::from("  🔗          - Symbolic link"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .alignment(Alignment::Left);

    f.render_widget(help, area);
}

/// Format file size in human-readable form
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
