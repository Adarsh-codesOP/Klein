use std::cell::Cell;
use std::path::PathBuf;
use crate::sidebar::Sidebar;
use crate::editor::Editor;
use crate::terminal::Terminal;
use crate::tabs::TabState;

#[derive(PartialEq)]
pub enum Maximized {
    None,
    Editor,
    Terminal,
}

/// What to do once a save-as completes (or a normal save for named files).
pub enum SaveAsContext {
    JustSave,
    SaveAndQuit,
    SaveAndClose,
    SaveAndSwitch,
}

pub struct SaveAsState {
    pub folder: String,
    pub filename: String,
    pub active_field: usize, // 0 = folder, 1 = filename
    pub context: SaveAsContext,
}

/// Generate a default filename like "untitled-DD-MM-YY-HHMMSS.txt" from UTC time.
pub fn default_save_filename() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = secs_to_utc(secs);
    format!("untitled-{:02}-{:02}-{:02}-{:02}{:02}{:02}.txt", d, mo, y % 100, h, mi, s)
}

fn secs_to_utc(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let sec  = secs % 60;
    let min  = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;
    let mut days = secs / 86400;
    let mut year = 1970u64;
    loop {
        let dy = if leap(year) { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        year += 1;
    }
    let month_lengths: [u64; 12] = [31, if leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &ml in &month_lengths {
        if days < ml { break; }
        days -= ml;
        month += 1;
    }
    (year, month, days + 1, hour, min, sec)
}

fn leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

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
    pub terminal_area: Cell<ratatui::layout::Rect>,
    /// Active mouse selection in the terminal: ((col,row),(col,row)) in inner-area coords.
    pub terminal_sel: Option<((u16, u16), (u16, u16))>,
    pub show_help: bool,
    pub help_scroll: usize,
    pub terminal_scroll: usize,
    pub show_quit_confirm: bool,
    pub show_unsaved_confirm: bool,
    pub show_close_confirm: bool,
    pub terminal_triggered_quit: bool,
    pub pending_open_path: Option<PathBuf>,
    pub save_as: Option<SaveAsState>,
    pub cwd: PathBuf,
    pub maximized: Maximized,
    /// Non-existent file passed via CLI argument. Triggers a "create it?" prompt on startup.
    pub create_file_prompt: Option<PathBuf>,
}

impl App {
    pub fn new(initial_file: Option<PathBuf>, clipboard: Option<arboard::Clipboard>) -> App {
        let config = crate::config::AppConfig::load();

        // Always open in the directory where Klein was launched.
        // Only fall back to the configured default_workspace if current_dir() fails.
        let launch_dir = std::env::current_dir().unwrap_or_else(|_| {
            config.default_workspace.as_deref()
                .map(std::path::PathBuf::from)
                .filter(|p| p.exists())
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        });

        // If a file was provided, root the sidebar/terminal in its parent directory.
        let current_dir = if let Some(ref file) = initial_file {
            file.parent()
                .map(|p| p.to_path_buf())
                .filter(|p| p != &PathBuf::from(""))
                .unwrap_or_else(|| launch_dir.clone())
        } else {
            launch_dir
        };

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
            terminal_area: Cell::new(ratatui::layout::Rect::default()),
            terminal_sel: None,
            show_help: false,
            help_scroll: 0,
            terminal_scroll: 0,
            show_quit_confirm: false,
            show_unsaved_confirm: false,
            show_close_confirm: false,
            terminal_triggered_quit: false,
            pending_open_path: None,
            save_as: None,
            cwd: current_dir.clone(),
            maximized: Maximized::None,
            create_file_prompt: None,
        };

        // Give the first editor the pre-initialized clipboard (created before raw mode).
        if let Some(cb) = clipboard {
            app.tabs[0].editor.clipboard = Some(cb);
        }

        // Send a clear command so the terminal panel opens with a clean slate
        // instead of showing shell startup noise or DA-query responses.
        app.terminal.send_clear();

        if let Some(file) = initial_file {
            if file.exists() {
                // Existing file — load its contents and focus the editor.
                app.open_in_current_tab(file);
                app.active_panel = Panel::Editor;
            } else {
                // Non-existent file: prompt the user before creating anything.
                // Sidebar keeps focus until they confirm.
                app.create_file_prompt = Some(file);
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

    /// Open a file in a new tab (always creates a new tab)
    pub fn open_in_new_tab(&mut self, path: PathBuf) {
        let mut tab = TabState::new();
        let _ = tab.editor.open(path);
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    /// Open a file in the current tab (replaces current editor state)
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
