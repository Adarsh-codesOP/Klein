pub const APP_TITLE: &str = " Klein IDE ";
pub const HELP_TITLE: &str = " HELP COMMANDS ";
pub const HELP_TEXT: &str = r#"
--- GENERAL NAVIGATION ---
[Ctrl + Arrows] Switch Panels
[Up/Down/Left/Right] Move Cursor (Editor)
[Up/Down] Scroll Viewport (Terminal)
[Enter] Expand Folder (Sidebar) / New Line (Editor)

--- FOCUS CONTROL ---
[Ctrl + R]  Focus Explorer (Sidebar)
[Ctrl + E]  Focus Editor / Maximize Editor (if already focused)
[Ctrl + T]  Focus Terminal / Maximize Terminal (if already focused)
[Ctrl + B]  Toggle Sidebar Visibility
[Ctrl + J]  Toggle Terminal Visibility
[Esc]       Restore 3-pane view (when maximized)

--- TAB MANAGEMENT ---
[Ctrl + Shift + Z] Next Tab
[Ctrl + Shift + X] Close Current Tab

--- FILE OPERATIONS ---
[Ctrl + S] Save Current File
[Ctrl + W] Close Current File
[Ctrl + Q] Quit Application

--- SIDEBAR ---
[Up/Down]   Navigate File Tree
[Ctrl+D/U]  Page Down/Up in File Tree
[Home/End]  Jump to First/Last Entry
[PageUp/Dn] Page Up/Down in File Tree
[.]         Toggle Hidden Files/Folders

--- EDITOR FEATURES ---
[Ctrl + Z] Undo
[Ctrl + X] Cut Selection (or current line)
[Ctrl + C] Copy Selection (or current line)
[Ctrl + V] Paste
[Ctrl + A] Select All
[Shift + Arrows] Extend Selection
[Backspace] Delete Character (left)
[Delete]    Delete Character (right)
[Home]      Go to Start of Line
[End]       Go to End of Line
[Ctrl+Home] Go to Top of File
[Ctrl+End]  Go to Bottom of File
[PageUp/Dn] Page Up/Down in Editor

--- HELP ---
[Ctrl + H] Toggle Help
"#;


pub mod colors {
    use ratatui::style::Color;
    pub const EXPLORER_FOCUS: Color = Color::Green;
    pub const EDITOR_FOCUS: Color = Color::Yellow;
    pub const TERMINAL_FOCUS: Color = Color::Cyan;
    pub const HELP_BORDER: Color = Color::Cyan;
    pub const STATUS_BG: Color = Color::DarkGray;
    pub const STATUS_FG: Color = Color::Gray;
    pub const SEARCH_BORDER: Color = Color::Cyan;
}

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct AppConfig {
    pub default_workspace: Option<String>,
    pub shell: Option<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "Klein") {
            let config_dir = proj_dirs.config_dir();
            let config_path = config_dir.join("config.toml");
            
            if config_path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = toml::from_str::<AppConfig>(&contents) {
                        return config;
                    }
                }
            }
        }
        AppConfig::default()
    }
}
