pub const APP_TITLE: &str = " Klein IDE ";
pub const HELP_TITLE: &str = " Klein IDE Help ";
pub const HELP_TEXT: &str = "Navigation: [Ctrl+Arrows] Panels | [Arrows/hjkl] Move | [Enter] Select/Open | [Ctrl+H] Toggle Help\nShortcuts: [Ctrl+E/R/T] Editor/Explorer/Terminal | [Ctrl+B] Sidebar | [Ctrl+`] Term | [Ctrl+S] Save | [Ctrl+Q] Quit";

pub const TERMINAL_BASH_PATH: &str = "C:\\Program Files\\Git\\bin\\bash.exe";

pub mod colors {
    use ratatui::style::Color;
    pub const FOCUS_BORDER: Color = Color::Yellow;
    pub const HELP_BORDER: Color = Color::Cyan;
    pub const STATUS_BG: Color = Color::DarkGray;
    pub const STATUS_FG: Color = Color::Gray;
    pub const SEARCH_BORDER: Color = Color::Cyan;
}
