use crate::editor::Editor;
use crate::sidebar::Sidebar;
use crate::tabs::TabState;
use crate::terminal::Terminal;
use std::cell::Cell;
use std::path::PathBuf;

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
    pub tabs: Vec<TabState>,
    pub active_tab: usize,
    pub preview: Option<Editor>,
    pub terminal: Terminal,
    pub last_editor_height: Cell<usize>,
    pub editor_area: Cell<ratatui::layout::Rect>,
    pub show_help: bool,
    pub terminal_scroll: usize,
    pub show_quit_confirm: bool,
    pub show_unsaved_confirm: bool,
    pub pending_open_path: Option<PathBuf>,
}

impl App {
    pub fn new() -> App {
        let config = crate::config::AppConfig::load();

        // Try to respect workspace from config first, fallback to current_dir
        let current_dir = if let Some(ws) = config.default_workspace {
            let path = std::path::PathBuf::from(ws);
            if path.exists() {
                path
            } else {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            }
        } else {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        };

        App {
            active_panel: Panel::Editor,
            show_sidebar: true,
            show_terminal: true,
            should_quit: false,
            sidebar: Sidebar::new(&current_dir),
            tabs: vec![TabState::new()],
            active_tab: 0,
            preview: None,
            terminal: Terminal::new(current_dir.clone(), config.shell.clone()),
            last_editor_height: Cell::new(20),
            editor_area: Cell::new(ratatui::layout::Rect::default()),
            show_help: false,
            terminal_scroll: 0,
            show_quit_confirm: false,
            show_unsaved_confirm: false,
            pending_open_path: None,
        }
    }

    /// Get a reference to the editor that should be displayed.
    /// Returns preview editor when sidebar is focused and preview exists,
    /// otherwise returns the active tab's editor.
    pub fn active_editor(&self) -> &Editor {
        if matches!(self.active_panel, Panel::Sidebar) {
            if let Some(preview) = &self.preview {
                return preview;
            }
        }
        self.editor()
    }

    /// Get a reference to the current tab's editor
    pub fn editor(&self) -> &Editor {
        &self.tabs[self.active_tab].editor
    }

    /// Get a mutable reference to the current tab's editor
    pub fn editor_mut(&mut self) -> &mut Editor {
        &mut self.tabs[self.active_tab].editor
    }

    /// Open a file in a new tab (always creates a new tab)
    pub fn open_in_new_tab(&mut self, path: PathBuf) {
        let mut tab = TabState::new();
        let _ = tab.editor.open(path);
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    /// Open a file in the current tab (replaces current editor state)
    #[allow(dead_code)]
    pub fn open_in_current_tab(&mut self, path: PathBuf) {
        let _ = self.tabs[self.active_tab].editor.open(path);
    }

    /// Switch to the next tab (wraps around)
    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    /// Close the active tab. Switches to adjacent tab.
    pub fn close_tab(&mut self) {
        if self.tabs.len() == 1 {
            // Don't close the last tab; just clear it
            self.tabs[0] = TabState::new();
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }
}
