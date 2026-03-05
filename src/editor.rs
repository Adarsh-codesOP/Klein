use ropey::Rope;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
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
    pub search_query: String,
    pub is_searching: bool,
    pub clipboard: Option<arboard::Clipboard>,
    pub selection_start: Option<(usize, usize)>,
    pub is_dirty: bool,
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
            search_query: String::new(),
            is_searching: false,
            clipboard: arboard::Clipboard::new().ok(),
            selection_start: None,
            is_dirty: false,
        }
    }

    pub fn open(&mut self, path: PathBuf) -> Result<()> {
        let content = fs::read_to_string(&path)?;
        self.buffer = Rope::from_str(&content);
        self.path = Some(path);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.scroll_y = 0;
        self.is_dirty = false;
        self.selection_start = None;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = &self.path {
            fs::write(path, self.buffer.to_string())?;
            self.is_dirty = false;
        }
        Ok(())
    }

    pub fn insert_char(&mut self, c: char) {
        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        self.buffer.insert_char(char_idx, c);
        self.cursor_x += 1;
        self.is_dirty = true;
        self.selection_start = None;
    }

    pub fn delete_char(&mut self) {
        if self.selection_start.is_some() {
            self.delete_selection();
            return;
        }

        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;

        if char_idx > 0 {
            if self.cursor_x > 0 {
                self.buffer.remove(char_idx - 1..char_idx);
                self.cursor_x -= 1;
            } else if self.cursor_y > 0 {
                // Join lines
                let prev_line_len = self.buffer.line(self.cursor_y - 1).len_chars();
                self.buffer.remove(char_idx - 1..char_idx); // Remove newline
                self.cursor_y -= 1;
                self.cursor_x = prev_line_len.saturating_sub(1);
            }
            self.is_dirty = true;
        }
    }

    pub fn delete_selection(&mut self) {
        if let Some((start_y, start_x)) = self.selection_start {
            let (sy, sx, ey, ex) = if (start_y, start_x) < (self.cursor_y, self.cursor_x) {
                (start_y, start_x, self.cursor_y, self.cursor_x)
            } else {
                (self.cursor_y, self.cursor_x, start_y, start_x)
            };

            let start_char = self.buffer.line_to_char(sy) + sx;
            let end_char = self.buffer.line_to_char(ey) + ex;

            if start_char < end_char {
                self.buffer.remove(start_char..end_char);
                self.cursor_y = sy;
                self.cursor_x = sx;
                self.is_dirty = true;
            }
            self.selection_start = None;
        }
    }

    pub fn get_gutter_width(&self) -> usize {
        let lines = self.buffer.len_lines();
        lines.to_string().len() + 2
    }

    pub fn get_highlighted_lines(&self, _width: usize, height: usize) -> Vec<ratatui::text::Line<'_>> {
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
            
            let mut current_char_in_line = 0;
            let spans: Vec<ratatui::text::Span> = highlights
                .into_iter()
                .map(|(style, text)| {
                    let mut span_style = ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    ));

                    // Check for selection
                    if let Some((start_y, start_x)) = self.selection_start {
                        let (sy, sx, ey, ex) = if (start_y, start_x) < (self.cursor_y, self.cursor_x) {
                            (start_y, start_x, self.cursor_y, self.cursor_x)
                        } else {
                            (self.cursor_y, self.cursor_x, start_y, start_x)
                        };

                        let text_len = text.chars().count();
                        let span_range_start = current_char_in_line;
                        let span_range_end = current_char_in_line + text_len;

                        let line_idx = i;
                        
                        let is_selected = if line_idx > sy && line_idx < ey {
                            true
                        } else if line_idx == sy && line_idx == ey {
                            span_range_start < ex && span_range_end > sx
                        } else if line_idx == sy {
                            span_range_end > sx
                        } else if line_idx == ey {
                            span_range_start < ex
                        } else {
                            false
                        };

                        if is_selected {
                            span_style = span_style.bg(ratatui::style::Color::Yellow).fg(ratatui::style::Color::Black);
                        }
                        current_char_in_line += text_len;
                    }

                    ratatui::text::Span::styled(text.to_string(), span_style)
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
            if self.cursor_y >= self.scroll_y + height && height > 0 {
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
            self.cursor_x = self.get_max_cursor_x(self.cursor_y);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_x < self.get_max_cursor_x(self.cursor_y) {
            self.cursor_x += 1;
        } else if self.cursor_y + 1 < self.buffer.len_lines() {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    pub fn toggle_selection(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some((self.cursor_y, self.cursor_x));
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
    }

    pub fn insert_tab(&mut self) {
        self.insert_char(' ');
        self.insert_char(' ');
        self.insert_char(' ');
        self.insert_char(' ');
    }

    pub fn copy(&mut self) {
        if let Some(clipboard) = &mut self.clipboard {
            let text = if let Some((start_y, start_x)) = self.selection_start {
                let (sy, sx, ey, ex) = if (start_y, start_x) < (self.cursor_y, self.cursor_x) {
                    (start_y, start_x, self.cursor_y, self.cursor_x)
                } else {
                    (self.cursor_y, self.cursor_x, start_y, start_x)
                };
                let start_char = self.buffer.line_to_char(sy) + sx;
                let end_char = self.buffer.line_to_char(ey) + ex;
                self.buffer.slice(start_char..end_char).to_string()
            } else {
                self.buffer.line(self.cursor_y).to_string()
            };
            let _ = clipboard.set_text(text);
        }
    }

    pub fn paste(&mut self, height: usize) {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                if self.selection_start.is_some() {
                    self.delete_selection();
                }
                let line_idx = self.buffer.line_to_char(self.cursor_y);
                let char_idx = line_idx + self.cursor_x;
                self.buffer.insert(char_idx, &text);
                
                // Update cursor after paste
                let text_rope = Rope::from_str(&text);
                if text_rope.len_lines() > 1 {
                    self.cursor_y += text_rope.len_lines() - 1;
                    self.cursor_x = text_rope.line(text_rope.len_lines() - 1).len_chars();
                } else {
                    self.cursor_x += text.len();
                }
                self.is_dirty = true;
                self.clamp_cursor_x();
                self.ensure_cursor_visible(height);
            }
        }
    }

    pub fn search(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }
        let content = self.buffer.to_string();
        let current_pos = self.buffer.line_to_char(self.cursor_y) + self.cursor_x;
        
        // Find next occurrence
        if let Some(found_pos) = content[current_pos + 1..].find(query) {
            let absolute_pos = current_pos + 1 + found_pos;
            self.move_to_pos(absolute_pos);
        } else if let Some(found_pos) = content[..current_pos].find(query) {
            // Wrap around
            self.move_to_pos(found_pos);
        }
    }

    fn move_to_pos(&mut self, pos: usize) {
        self.cursor_y = self.buffer.char_to_line(pos);
        self.cursor_x = pos - self.buffer.line_to_char(self.cursor_y);
        // Update scroll if needed
        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        }
    }

    pub fn ensure_cursor_visible(&mut self, height: usize) {
        if height == 0 { return; }
        
        if self.cursor_y < self.scroll_y || self.cursor_y >= self.scroll_y + height {
            // Center the cursor if it's out of view
            self.scroll_y = self.cursor_y.saturating_sub(height / 2);
            
            // Limit scroll to buffer length
            let max_scroll = self.buffer.len_lines().saturating_sub(height);
            if self.scroll_y > max_scroll {
                self.scroll_y = max_scroll;
            }
        }
    }

    fn get_max_cursor_x(&self, line_y: usize) -> usize {
        if self.buffer.len_lines() == 0 {
            return 0;
        }
        let line = self.buffer.line(line_y);
        let line_len = line.len_chars();
        
        let line_str = line.to_string();
        if line_str.ends_with('\n') || line_str.ends_with('\r') {
            line_len.saturating_sub(1)
        } else {
            line_len
        }
    }

    pub fn clamp_cursor_x(&mut self) {
        let max_x = self.get_max_cursor_x(self.cursor_y);
        if self.cursor_x > max_x {
            self.cursor_x = max_x;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_cursor_x() {
        let mut editor = Editor::new();
        
        // Empty buffer
        assert_eq!(editor.get_max_cursor_x(0), 0);
        
        // Line with newline
        editor.buffer = Rope::from_str("abc\n");
        assert_eq!(editor.get_max_cursor_x(0), 3); // Position after 'c', but before '\n'
        
        // Line without newline (last line)
        editor.buffer = Rope::from_str("abc");
        assert_eq!(editor.get_max_cursor_x(0), 3); // Position after 'c'
        
        // Multiple lines
        editor.buffer = Rope::from_str("abc\ndef");
        assert_eq!(editor.get_max_cursor_x(0), 3); // After 'c'
        assert_eq!(editor.get_max_cursor_x(1), 3); // After 'f'
    }

    #[test]
    fn test_move_cursor_right() {
        let mut editor = Editor::new();
        editor.buffer = Rope::from_str("abc");
        
        editor.move_cursor_right(); // -> 'a'
        assert_eq!(editor.cursor_x, 1);
        editor.move_cursor_right(); // -> 'b'
        assert_eq!(editor.cursor_x, 2);
        editor.move_cursor_right(); // -> 'c'
        assert_eq!(editor.cursor_x, 3);
        editor.move_cursor_right(); // stays at 3
        assert_eq!(editor.cursor_x, 3);
    }

    #[test]
    fn test_move_cursor_left_wrap() {
        let mut editor = Editor::new();
        editor.buffer = Rope::from_str("abc\ndef");
        
        editor.cursor_y = 1;
        editor.cursor_x = 0;
        editor.move_cursor_left();
        
        assert_eq!(editor.cursor_y, 0);
        assert_eq!(editor.cursor_x, 3); // Should wrap to position after 'c'
    }
}
