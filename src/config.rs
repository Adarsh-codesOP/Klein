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

--- LSP COMMANDS ---
[Ctrl+Space] Trigger Autocompletion
[Alt+G, then d] Go to Definition
[Alt+G, then r] Find References
[Alt+G, then n] Rename Symbol under Cursor
[Alt+F] Format Document
[Alt+Enter] Code Actions / Quick Fix
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
        let mut search_paths = Vec::new();

        // 1. Current Working Directory (highest priority for development)
        if let Ok(cwd) = std::env::current_dir() {
            search_paths.push(cwd.join("config.toml"));
        }

        // 2. Standard Roaming Config Dir
        if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "Klein") {
            search_paths.push(proj_dirs.config_dir().join("config.toml"));
        }

        // 3. KleinLocal (Used by local installer)
        if let Some(user_dirs) = directories::UserDirs::new() {
            let app_data_local = user_dirs
                .home_dir()
                .join("AppData")
                .join("Local")
                .join("KleinLocal");
            search_paths.push(app_data_local.join("config.toml"));
        }

        // 4. Standard Home Dir fallback
        if let Some(home) = directories::UserDirs::new() {
            search_paths.push(home.home_dir().join(".klein").join("config.toml"));
        }

        for config_path in search_paths {
            if config_path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&config_path) {
                    log::warn!("LSP: loading config from {}", config_path.display());
                    if let Ok(config) = toml::from_str::<AppConfig>(&contents) {
                        log::warn!("LSP: loaded config: {:?}", config);
                        return config;
                    }
                } else {
                    log::error!(
                        "LSP: found config but could not read file at {}",
                        config_path.display()
                    );
                }
            }
        }

        log::warn!("LSP: No config.toml found in search paths. LSP features will be DISABLED.");
        log::warn!("LSP: Please create a config.toml in the current directory or in %AppData%\\Klein\\config\\config.toml");
        AppConfig::default()
    }
}
