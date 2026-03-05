use std::cell::Cell;
use crate::sidebar::Sidebar;
use crate::editor::Editor;
use crate::micro_editor::MicroEditor;
use crate::terminal::Terminal;

pub enum Panel {
    Sidebar,
    Editor,
    Terminal,
}

pub enum EditorMode {
    Internal,
    Micro,
}

pub struct App {
    pub active_panel: Panel,
    pub editor_mode: EditorMode,
    pub show_sidebar: bool,
    pub show_terminal: bool,
    pub should_quit: bool,
    pub sidebar: Sidebar,
    pub editor: Editor,
    pub micro_editor: Option<MicroEditor>,
    pub terminal: Terminal,
    pub last_editor_height: Cell<usize>,
    pub last_micro_width: Cell<u16>,
    pub last_micro_height: Cell<u16>,
    pub show_help: bool,
    pub terminal_scroll: usize,
    pub show_quit_confirm: bool,
}

impl App {
    pub fn new() -> App {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        App {
            active_panel: Panel::Editor,
            editor_mode: EditorMode::Micro,
            show_sidebar: true,
            show_terminal: true,
            should_quit: false,
            sidebar: Sidebar::new(&current_dir),
            editor: Editor::new(),
            micro_editor: Some(MicroEditor::new(80, 24)),
            terminal: Terminal::new(current_dir),
            last_editor_height: Cell::new(20),
            last_micro_width: Cell::new(80),
            last_micro_height: Cell::new(24),
            show_help: false,
            terminal_scroll: 0,
            show_quit_confirm: false,
        }
    }

    pub fn resize_micro(&mut self, width: u16, height: u16) {
        if let Some(micro) = &mut self.micro_editor {
            micro.resize(width, height);
        }
    }
}
