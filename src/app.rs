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

#[derive(Debug, PartialEq, Clone)]
pub enum SaveAsContext {
    SaveOnly,
    QuitAfter,
    CloseTabAfter,
    SwitchFileAfter(PathBuf),
}

pub struct SaveAsState {
    pub active: bool,
    pub filename: String,
    pub cur_dir: PathBuf,
    pub focus_filename: bool,
    pub context: SaveAsContext,
}

impl Default for SaveAsState {
    fn default() -> Self {
        Self {
            active: false,
            filename: String::new(),
            cur_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            focus_filename: true,
            context: SaveAsContext::SaveOnly,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Maximized {
    None,
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
    pub help_scroll: usize,
    pub terminal_scroll: usize,
    pub terminal_restarting: bool,
    pub terminal_area: Cell<ratatui::layout::Rect>,
    pub terminal_sel: Option<((usize, usize), (usize, usize))>,
    pub show_quit_confirm: bool,
    pub show_unsaved_confirm: bool,
    pub show_create_file_prompt: bool,
    pub pending_open_path: Option<PathBuf>,
    pub maximized: Maximized,
    pub save_as_state: SaveAsState,
    pub clipboard: Option<arboard::Clipboard>,
}

impl App {
    pub fn new(cli_file: Option<PathBuf>, clipboard: Option<arboard::Clipboard>) -> App {
        let config = crate::config::AppConfig::load();

        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        let mut app = App {
            active_panel: Panel::Sidebar,
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
            help_scroll: 0,
            terminal_scroll: 0,
            terminal_restarting: false,
            terminal_area: Cell::new(ratatui::layout::Rect::default()),
            terminal_sel: None,
            show_quit_confirm: false,
            show_unsaved_confirm: false,
            show_create_file_prompt: false,
            pending_open_path: None,
            maximized: Maximized::None,
            save_as_state: SaveAsState {
                cur_dir: current_dir.clone(),
                ..Default::default()
            },
            clipboard,
        };

        if let Some(file) = cli_file {
            let path = current_dir.join(&file);
            if path.exists() {
                app.open_in_current_tab(path);
                app.active_panel = Panel::Editor;
            } else {
                app.pending_open_path = Some(path);
                app.show_create_file_prompt = true;
            }
        }

        app
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

    pub fn copy_selection(&mut self) {
        let mut cb = self.clipboard.take();
        self.editor_mut().copy(&mut cb);
        self.clipboard = cb;
    }

    pub fn cut_selection(&mut self) {
        let mut cb = self.clipboard.take();
        self.editor_mut().cut(&mut cb);
        self.clipboard = cb;
    }

    pub fn paste_clipboard(&mut self, height: usize) {
        let mut cb = self.clipboard.take();
        self.editor_mut().paste(&mut cb, height);
        self.clipboard = cb;
    }

    pub fn insert_paste(&mut self, text: &str, height: usize) {
        self.editor_mut().insert_paste(text, height);
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

    pub fn save_current_file(&mut self) {
        let active_tab = self.active_tab;
        let tab = &mut self.tabs[active_tab];
        if tab.editor.path.is_some() {
            let _ = tab.editor.save();
        }
    }

    pub fn try_save_or_show_save_as(&mut self, context: SaveAsContext) -> bool {
        let tab = &mut self.tabs[self.active_tab];
        if tab.editor.path.is_some() {
            self.save_current_file();
            true
        } else {
            self.save_as_state.active = true;
            self.save_as_state.context = context;

            let now = chrono::Local::now();
            let proposed_name = now.format("untitled-%d-%m-%y-%H%M%S.txt").to_string();

            self.save_as_state.filename = proposed_name;
            self.save_as_state.focus_filename = true;
            false
        }
    }

    pub fn execute_save_as(&mut self) {
        let path = self
            .save_as_state
            .cur_dir
            .join(&self.save_as_state.filename);
        let tab = &mut self.tabs[self.active_tab];
        tab.editor.path = Some(path);
        self.save_current_file();
        self.save_as_state.active = false;

        self.sidebar.refresh();

        match self.save_as_state.context.clone() {
            SaveAsContext::QuitAfter => {
                self.should_quit = true;
            }
            SaveAsContext::CloseTabAfter => {
                self.close_tab();
            }
            SaveAsContext::SwitchFileAfter(path) => {
                self.open_in_new_tab(path);
                self.active_panel = Panel::Editor;
            }
            SaveAsContext::SaveOnly => {}
        }
    }
}
