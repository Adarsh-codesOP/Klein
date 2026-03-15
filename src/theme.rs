use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub extends: Option<String>,
    pub editor: Option<EditorThemeConfig>,
    pub sidebar: Option<SidebarThemeConfig>,
    pub status_bar: Option<StatusBarThemeConfig>,
    pub tabs: Option<TabsThemeConfig>,
    pub top_bar: Option<TopBarThemeConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditorThemeConfig {
    pub background: Option<String>,
    pub text: Option<String>,
    pub cursor: Option<String>,
    pub selection: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SidebarThemeConfig {
    pub background: Option<String>,
    pub text: Option<String>,
    pub selected: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusBarThemeConfig {
    pub background: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TabsThemeConfig {
    pub active_bg: Option<String>,
    pub inactive_bg: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TopBarThemeConfig {
    pub background: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub extends: Option<String>,
    pub editor: EditorTheme,
    pub sidebar: SidebarTheme,
    pub status_bar: StatusBarTheme,
    pub tabs: TabsTheme,
    pub top_bar: TopBarTheme,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorTheme {
    pub background: Color,
    pub text: Color,
    pub cursor: Color,
    pub selection: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarTheme {
    pub background: Color,
    pub text: Color,
    pub selected: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusBarTheme {
    pub background: Color,
    pub text: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TabsTheme {
    pub active_bg: Color,
    pub inactive_bg: Color,
    pub text: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopBarTheme {
    pub background: Color,
    pub text: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "dark".to_string(),
            extends: None,
            editor: EditorTheme {
                background: Color::Rgb(30, 30, 30),
                text: Color::Rgb(212, 212, 212),
                cursor: Color::Rgb(255, 255, 255),
                selection: Color::Rgb(38, 79, 120),
            },
            sidebar: SidebarTheme {
                background: Color::Rgb(37, 37, 38),
                text: Color::Rgb(204, 204, 204),
                selected: Color::Rgb(55, 55, 61),
            },
            status_bar: StatusBarTheme {
                background: Color::Rgb(0, 122, 204),
                text: Color::Rgb(255, 255, 255),
            },
            tabs: TabsTheme {
                active_bg: Color::Rgb(30, 30, 30),
                inactive_bg: Color::Rgb(45, 45, 45),
                text: Color::Rgb(255, 255, 255),
            },
            top_bar: TopBarTheme {
                background: Color::Rgb(60, 60, 60),
                text: Color::Rgb(204, 204, 204),
            },
        }
    }
}

pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

impl Theme {
    pub fn merge_config(&mut self, config: &ThemeConfig) {
        self.name = config.name.clone();
        if let Some(ext) = &config.extends {
            self.extends = Some(ext.clone());
        }

        if let Some(c) = &config.editor {
            if let Some(bg) = &c.background {
                if let Some(color) = parse_hex_color(bg) { self.editor.background = color; }
            }
            if let Some(fg) = &c.text {
                if let Some(color) = parse_hex_color(fg) { self.editor.text = color; }
            }
            if let Some(cur) = &c.cursor {
                if let Some(color) = parse_hex_color(cur) { self.editor.cursor = color; }
            }
            if let Some(sel) = &c.selection {
                if let Some(color) = parse_hex_color(sel) { self.editor.selection = color; }
            }
        }

        if let Some(c) = &config.sidebar {
            if let Some(bg) = &c.background {
                if let Some(color) = parse_hex_color(bg) { self.sidebar.background = color; }
            }
            if let Some(fg) = &c.text {
                if let Some(color) = parse_hex_color(fg) { self.sidebar.text = color; }
            }
            if let Some(sel) = &c.selected {
                if let Some(color) = parse_hex_color(sel) { self.sidebar.selected = color; }
            }
        }

        if let Some(c) = &config.status_bar {
            if let Some(bg) = &c.background {
                if let Some(color) = parse_hex_color(bg) { self.status_bar.background = color; }
            }
            if let Some(fg) = &c.text {
                if let Some(color) = parse_hex_color(fg) { self.status_bar.text = color; }
            }
        }

        if let Some(c) = &config.tabs {
            if let Some(bg) = &c.active_bg {
                if let Some(color) = parse_hex_color(bg) { self.tabs.active_bg = color; }
            }
            if let Some(bg) = &c.inactive_bg {
                if let Some(color) = parse_hex_color(bg) { self.tabs.inactive_bg = color; }
            }
            if let Some(fg) = &c.text {
                if let Some(color) = parse_hex_color(fg) { self.tabs.text = color; }
            }
        }

        if let Some(c) = &config.top_bar {
            if let Some(bg) = &c.background {
                if let Some(color) = parse_hex_color(bg) { self.top_bar.background = color; }
            }
            if let Some(fg) = &c.text {
                if let Some(color) = parse_hex_color(fg) { self.top_bar.text = color; }
            }
        }
    }
}
