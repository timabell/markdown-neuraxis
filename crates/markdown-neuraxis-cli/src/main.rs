use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use markdown_neuraxis_config::Config;
use markdown_neuraxis_engine::{Document, FileTree, FileTreeItem, io};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use relative_path::RelativePathBuf;
use std::{env, io::stdout, path::PathBuf, process};

struct App {
    notes_path: PathBuf,
    file_tree: FileTree,
    tree_items: Vec<FileTreeItem>,
    file_list_state: ListState,
    selected_document: Option<Document>,
    current_content: Vec<String>,
}

impl App {
    fn new(notes_path: PathBuf) -> Result<Self> {
        let file_tree = io::build_file_tree(&notes_path)?;
        let tree_items = file_tree.get_items();

        let mut app = Self {
            notes_path,
            file_tree,
            tree_items,
            file_list_state: ListState::default(),
            selected_document: None,
            current_content: Vec::new(),
        };

        // Select first item if available
        if !app.tree_items.is_empty() {
            app.file_list_state.select(Some(0));
            app.update_content_for_selection();
        }

        Ok(app)
    }

    fn next_file(&mut self) {
        let i = match self.file_list_state.selected() {
            Some(i) => (i + 1) % self.tree_items.len(),
            None => 0,
        };
        self.file_list_state.select(Some(i));
        self.update_content_for_selection();
    }

    fn previous_file(&mut self) {
        let i = match self.file_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tree_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.file_list_state.select(Some(i));
        self.update_content_for_selection();
    }

    fn update_content_for_selection(&mut self) {
        if let Some(index) = self.file_list_state.selected()
            && let Some(item) = self.tree_items.get(index)
        {
            if item.node.is_folder {
                // For folders, show folder info
                self.current_content = vec![
                    format!("📁 {}", item.node.name),
                    String::new(),
                    "Press Enter/Space to toggle, → to expand, ← to collapse".to_string(),
                ];
                self.selected_document = None;
            } else if let Some(ref file) = item.node.markdown_file {
                // Load and display file content
                match io::read_file(file.relative_path(), &self.notes_path) {
                    Ok(content) => match Document::from_bytes(content.as_bytes()) {
                        Ok(mut document) => {
                            document.create_anchors_from_tree();
                            self.selected_document = Some(document.clone());
                            self.current_content = self.render_document_content(&document);
                        }
                        Err(e) => {
                            self.current_content = vec![format!("Error parsing document: {}", e)];
                            self.selected_document = None;
                        }
                    },
                    Err(e) => {
                        self.current_content = vec![format!("Error reading file: {}", e)];
                        self.selected_document = None;
                    }
                }
            }
        }
    }

    fn activate_selected_item(&mut self) -> Result<()> {
        if let Some(index) = self.file_list_state.selected()
            && let Some(item) = self.tree_items.get(index)
            && item.node.is_folder
        {
            // Handle folder toggle
            self.toggle_folder(item.node.relative_path.clone());
            // Update content after toggle
            self.update_content_for_selection();
        }
        // Files don't need activation - they're already loaded by update_content_for_selection
        Ok(())
    }

    fn toggle_folder(&mut self, relative_path: RelativePathBuf) {
        self.file_tree.toggle_folder(&relative_path);
        self.tree_items = self.file_tree.get_items();
    }

    fn expand_selected_folder(&mut self) -> Result<()> {
        if let Some(index) = self.file_list_state.selected()
            && let Some(item) = self.tree_items.get(index)
            && item.node.is_folder
            && !item.node.is_expanded
        {
            self.file_tree.expand_folder(&item.node.relative_path);
            self.tree_items = self.file_tree.get_items();
            self.update_content_for_selection();
        }
        Ok(())
    }

    fn collapse_selected_folder(&mut self) -> Result<()> {
        if let Some(index) = self.file_list_state.selected()
            && let Some(item) = self.tree_items.get(index)
            && item.node.is_folder
            && item.node.is_expanded
        {
            self.file_tree.collapse_folder(&item.node.relative_path);
            self.tree_items = self.file_tree.get_items();
            self.update_content_for_selection();
        }
        Ok(())
    }

    fn render_document_content(&self, document: &Document) -> Vec<String> {
        let snapshot = document.snapshot();
        let mut lines = Vec::new();

        for block in &snapshot.blocks {
            match &block.kind {
                markdown_neuraxis_engine::editing::snapshot::BlockKind::Heading { level } => {
                    let prefix = "#".repeat(*level as usize);
                    lines.push(format!("{} {}", prefix, block.content));
                    lines.push(String::new()); // Empty line after heading
                }
                markdown_neuraxis_engine::editing::snapshot::BlockKind::Paragraph => {
                    lines.push(block.content.clone());
                    lines.push(String::new()); // Empty line after paragraph
                }
                markdown_neuraxis_engine::editing::snapshot::BlockKind::ListItem {
                    marker, ..
                } => {
                    let marker_str = match marker {
                        markdown_neuraxis_engine::editing::document::Marker::Dash => "•",
                        markdown_neuraxis_engine::editing::document::Marker::Asterisk => "*",
                        markdown_neuraxis_engine::editing::document::Marker::Plus => "+",
                        markdown_neuraxis_engine::editing::document::Marker::Numbered(_) => "1.",
                    };
                    lines.push(format!("{} {}", marker_str, block.content));
                }
                markdown_neuraxis_engine::editing::snapshot::BlockKind::CodeFence { lang } => {
                    lines.push(format!("```{}", lang.as_deref().unwrap_or("")));
                    lines.extend(block.content.lines().map(|s| s.to_string()));
                    lines.push("```".to_string());
                    lines.push(String::new());
                }
                markdown_neuraxis_engine::editing::snapshot::BlockKind::BlockQuote => {
                    for line in block.content.lines() {
                        lines.push(format!("> {}", line));
                    }
                    lines.push(String::new());
                }
                markdown_neuraxis_engine::editing::snapshot::BlockKind::ThematicBreak => {
                    lines.push("---".to_string());
                    lines.push(String::new());
                }
                markdown_neuraxis_engine::editing::snapshot::BlockKind::UnhandledMarkdown => {
                    lines.push(format!("[Unhandled] {}", block.content));
                    lines.push(String::new());
                }
            }
        }

        lines
    }
}

fn main() -> Result<()> {
    // Determine notes path from CLI args or config file
    let args: Vec<String> = env::args().collect();
    let config_path = Config::config_path();

    let notes_path;
    let from_config;

    if args.len() == 2 {
        // CLI argument provided - use it
        notes_path = PathBuf::from(&args[1]);
        from_config = false;
    } else if args.len() == 1 {
        // No CLI argument - try config file
        match Config::load() {
            Ok(Some(config)) => {
                notes_path = config.notes_path;
                from_config = true;
            }
            Ok(None) => {
                eprintln!("Error: No notes path provided and no config file found");
                eprintln!("Usage: {} <notes-folder-path>", args[0]);
                eprintln!("Or create a config file at {}", config_path.display());
                process::exit(1);
            }
            Err(e) => {
                eprintln!("Error: Failed to load config file: {e}");
                eprintln!("Usage: {} <notes-folder-path>", args[0]);
                process::exit(1);
            }
        }
    } else {
        eprintln!("Usage: {} [notes-folder-path]", args[0]);
        process::exit(1);
    };

    // Validate notes directory using engine
    if let Err(e) = io::validate_notes_dir(&notes_path) {
        let source = if from_config {
            format!(" from config file '{}'", config_path.display())
        } else {
            String::new()
        };
        eprintln!(
            "Error: Notes path '{}'{} is invalid: {e}",
            notes_path.display(),
            source
        );
        process::exit(1);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(notes_path)?;

    // Main loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => app.next_file(),
                KeyCode::Up | KeyCode::Char('k') => app.previous_file(),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let _ = app.activate_selected_item();
                }
                KeyCode::Right => {
                    let _ = app.expand_selected_folder();
                }
                KeyCode::Left => {
                    let _ = app.collapse_selected_folder();
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.area());

    // File list panel
    let file_items: Vec<ListItem> = app
        .tree_items
        .iter()
        .map(|item| {
            let indent = "  ".repeat(item.depth);
            let icon = if item.node.is_folder {
                if item.node.is_expanded {
                    "📂 "
                } else {
                    "📁 "
                }
            } else {
                "📄 "
            };
            let name = &item.node.name;
            let display_text = format!("{}{}{}", indent, icon, name);
            ListItem::new(vec![Line::from(vec![Span::raw(display_text)])])
        })
        .collect();

    let files_list = List::new(file_items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

    f.render_stateful_widget(files_list, chunks[0], &mut app.file_list_state);

    // Content panel
    let content_text = if app.current_content.is_empty() {
        vec![Line::from("Select a file to view its content")]
    } else {
        app.current_content
            .iter()
            .map(|line| Line::from(vec![Span::raw(line.clone())]))
            .collect()
    };

    let content = Paragraph::new(content_text)
        .block(Block::default().borders(Borders::ALL).title("Content"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(content, chunks[1]);

    // Instructions
    let help_text = Line::from(vec![
        Span::raw("q: Quit | "),
        Span::raw("↑/k: Previous | "),
        Span::raw("↓/j: Next | "),
        Span::raw("Enter/Space: Toggle | →: Expand | ←: Collapse"),
    ]);

    let help = Paragraph::new(vec![help_text]).block(Block::default());

    // Place help at bottom
    let bottom_chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    f.render_widget(help, bottom_chunk[1]);
}
