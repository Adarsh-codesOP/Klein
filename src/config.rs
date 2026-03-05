pub const APP_TITLE: &str = " Klein IDE ";
pub const HELP_TITLE: &str = " HELP COMMANDS ";
pub const HELP_TEXT: &str = r#"
--- GENERAL NAVIGATION ---
[Ctrl + Arrows] Switch Panels
[Up/Down/Left/Right] Move Cursor (Editor)
[Up/Down] Scroll Viewport (Terminal)
[Enter] Selection (Sidebar) / New Line (Editor)

--- FOCUS CONTROL ---
[Ctrl + R] Focus Explorer (Sidebar)
[Ctrl + E] Focus Editor
[Ctrl + T] Focus Terminal
[Ctrl + B] Toggle Sidebar Visibility
[Ctrl + `] Toggle Terminal Visibility

--- FILE OPERATIONS ---
[Ctrl + S] Save Current File
[Ctrl + Q] Quit Application

--- EDITOR FEATURES ---
[Ctrl + F] Start Search
[Ctrl + C] Copy Selection
[Backspace] Delete Character

--- HELP ---
[Ctrl + H] Toggle this help overlay
"#;

pub const TERMINAL_BASH_PATH: &str = "C:\\Program Files\\Git\\bin\\bash.exe";
pub const MICRO_PATH: &str = "micro";

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
