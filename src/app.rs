use std::cell::Cell;
use crate::sidebar::Sidebar;
use crate::editor::Editor;
use crate::terminal::Terminal;

pub enum Panel {
    Sidebar,
    Editor,
    Terminal,
}

pub struct App {
    pub active_panel: Panel,
    pub show_sidebar: bool,
    pub show_terminal: bool,
    pub should_quit: bool,
    pub sidebar: Sidebar,
    pub editor: Editor,
    pub terminal: Terminal,
    pub last_editor_height: Cell<usize>,
    pub editor_area: Cell<ratatui::layout::Rect>,
    pub show_help: bool,
    pub terminal_scroll: usize,
    pub show_quit_confirm: bool,
}

impl App {
    pub fn new() -> App {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        App {
            active_panel: Panel::Editor,
            show_sidebar: true,
            show_terminal: true,
            should_quit: false,
            sidebar: Sidebar::new(&current_dir),
            editor: Editor::new(),
            terminal: Terminal::new(current_dir),
            last_editor_height: Cell::new(20),
            editor_area: Cell::new(ratatui::layout::Rect::default()),
            show_help: false,
            terminal_scroll: 0,
            show_quit_confirm: false,
        }
    }
}
