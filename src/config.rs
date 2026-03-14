pub const APP_TITLE: &str = " Klein IDE ";
pub const HELP_TITLE: &str = " HELP COMMANDS ";
pub const HELP_TEXT: &str = r#"
--- SIDEBAR / FILE TREE ---
[.] Toggle Hidden Files
[Ctrl+D / PgDn] Page Down
[Ctrl+U / PgUp] Page Up
[Home / End] Jump to Top/Bottom
[Enter] Open File / Toggle Folder

--- EDITOR ---
[Home / End] Start/End of Line
[Ctrl+Home / Ctrl+End] Top/Bottom of File
[PgUp / PgDn] Scroll Page
[Delete] Forward Delete / Delete Selection
[Shift+Arrows] Extend Selection
[Ctrl+X] Cut Line / Selection
[Ctrl+C / Ctrl+V] Copy / Paste
[Ctrl+A] Select All
[Ctrl+Z] Undo

--- FILE MANAGEMENT ---
[Ctrl+P] Find File (fzf)
[Ctrl+G] Project Search (rg)
[Ctrl+W] Close Current File
[Ctrl+S] Save Current File
[Ctrl+Shift+Z] Next Tab
[Ctrl+Shift+X] Close Current Tab

--- FOCUS CONTROL ---
[Ctrl+F] Focus Sidebar
[Ctrl+E] Focus Editor
[Ctrl+T] Focus Terminal
[Ctrl+B] Toggle Sidebar Visibility
[Ctrl+J] Toggle Terminal Visibility
[Esc] Restore Standard Layout / Close Overlays

--- HELP ---
[Ctrl+H / Esc] Toggle Help Overlay
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
    #[allow(dead_code)]
    pub default_workspace: Option<String>,
    pub shell: Option<String>,
    pub enabled_lsps: Option<Vec<String>>,
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
