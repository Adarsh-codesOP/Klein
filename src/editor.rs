use ropey::Rope;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use std::path::PathBuf;
use std::fs;
use anyhow::Result;

pub struct Editor {
    pub buffer: Rope,
    pub path: Option<PathBuf>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            buffer: Rope::from_str(""),
            path: None,
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn open(&mut self, path: PathBuf) -> Result<()> {
        let content = fs::read_to_string(&path)?;
        self.buffer = Rope::from_str(&content);
        self.path = Some(path);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.scroll_y = 0;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = &self.path {
            fs::write(path, self.buffer.to_string())?;
        }
        Ok(())
    }

    pub fn insert_char(&mut self, c: char) {
        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        self.buffer.insert_char(char_idx, c);
        self.cursor_x += 1;
    }

    pub fn delete_char(&mut self) {
        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;

        if char_idx > 0 {
            self.buffer.remove(char_idx - 1..char_idx);
            if self.cursor_x > 0 {
                self.cursor_x -= 1;
            } else if self.cursor_y > 0 {
                self.cursor_y -= 1;
                // Move to end of previous line
                self.cursor_x = self.buffer.line(self.cursor_y).len_chars().saturating_sub(1);
            }
        }
    }

    pub fn get_highlighted_lines(&self, width: usize, height: usize) -> Vec<ratatui::text::Line> {
        let syntax = if let Some(path) = &self.path {
            self.syntax_set
                .find_syntax_for_file(path)
                .unwrap_or(None)
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        let mut h = HighlightLines::new(syntax, &self.theme_set.themes["base16-ocean.dark"]);
        let mut lines = Vec::new();

        let start_line = self.scroll_y;
        let end_line = (start_line + height).min(self.buffer.len_lines());

        for i in start_line..end_line {
            let line = self.buffer.line(i).to_string();
            let highlights = h.highlight_line(&line, &self.syntax_set).unwrap_or_default();
            
            let spans: Vec<ratatui::text::Span> = highlights
                .into_iter()
                .map(|(style, text)| {
                    let color = ratatui::style::Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    ratatui::text::Span::styled(text.to_string(), ratatui::style::Style::default().fg(color))
                })
                .collect();
            
            lines.push(ratatui::text::Line::from(spans));
        }

        lines
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            if self.cursor_y < self.scroll_y {
                self.scroll_y = self.cursor_y;
            }
            self.clamp_cursor_x();
        }
    }

    pub fn move_cursor_down(&mut self, height: usize) {
        if self.cursor_y + 1 < self.buffer.len_lines() {
            self.cursor_y += 1;
            if self.cursor_y >= self.scroll_y + height {
                self.scroll_y = self.cursor_y - height + 1;
            }
            self.clamp_cursor_x();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.buffer.line(self.cursor_y).len_chars().saturating_sub(1);
        }
    }

    pub fn move_cursor_right(&mut self) {
        let line_len = self.buffer.line(self.cursor_y).len_chars().saturating_sub(1);
        if self.cursor_x < line_len {
            self.cursor_x += 1;
        } else if self.cursor_y + 1 < self.buffer.len_lines() {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    fn clamp_cursor_x(&mut self) {
        let line_len = self.buffer.line(self.cursor_y).len_chars().saturating_sub(1);
        if self.cursor_x > line_len {
            self.cursor_x = line_len;
        }
    }
}
