pub const APP_TITLE: &str = " Klein IDE ";
pub const HELP_TITLE: &str = " HELP COMMANDS ";
pub const HELP_TEXT: &str = r#"
--- GENERAL NAVIGATION ---
[Ctrl + Arrows] Switch Panels
[Up/Down/Left/Right] Move Cursor (Editor)
[Up/Down] Scroll Viewport (Terminal)
[Enter] Expand Folder (Sidebar) / New Line (Editor)

--- FOCUS CONTROL ---
[Ctrl + R] Focus Explorer (Sidebar)
[Ctrl + E] Focus Editor
[Ctrl + T] Focus Terminal
[Ctrl + B] Toggle Sidebar Visibility
[Ctrl + `] Toggle Terminal Visibility

--- TAB MANAGEMENT ---
[Ctrl + Shift + Z] Next Tab
[Ctrl + Shift + X] Close Current Tab

--- FILE OPERATIONS ---
[Ctrl + S] Save Current File
[Ctrl + Q] Quit Application

--- EDITOR FEATURES ---
[Ctrl + C] Copy Selection
[Ctrl + V] Paste
[Ctrl + A] Select All
[Ctrl + Shift + Up/Down] Multiline Selection
[Backspace] Delete Character

--- HELP ---
[Ctrl + H] Toggle this help overlay
"#;

pub mod colors {
    use ratatui::style::Color;
    pub const EXPLORER_FOCUS: Color = Color::Green;
    pub const EDITOR_FOCUS: Color = Color::Yellow;
    pub const TERMINAL_FOCUS: Color = Color::Cyan;
    pub const HELP_BORDER: Color = Color::Cyan;
    pub const STATUS_BG: Color = Color::DarkGray;
    pub const STATUS_FG: Color = Color::Gray;
    #[allow(dead_code)]
    pub const SEARCH_BORDER: Color = Color::Cyan;
}

use serde::Deserialize;

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
