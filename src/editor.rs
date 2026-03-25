use ropey::Rope;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use std::path::PathBuf;
use std::fs;
use anyhow::Result;

struct UndoState {
    buffer: String,
    cursor_y: usize,
    cursor_x: usize,
}

pub struct Editor {
    pub buffer: Rope,
    pub path: Option<PathBuf>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
    pub clipboard: Option<arboard::Clipboard>,
    pub selection_start: Option<(usize, usize)>,
    pub is_dirty: bool,
    undo_stack: Vec<UndoState>,
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
            clipboard: arboard::Clipboard::new().ok(),
            selection_start: None,
            is_dirty: false,
            undo_stack: Vec::new(),
        }
    }

    /// Push the current buffer + cursor onto the undo stack (capped at 200 entries).
    fn save_undo_state(&mut self) {
        if self.undo_stack.len() >= 200 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoState {
            buffer: self.buffer.to_string(),
            cursor_y: self.cursor_y,
            cursor_x: self.cursor_x,
        });
    }

    /// Restore the most recent undo snapshot.
    pub fn undo(&mut self, height: usize) {
        if let Some(state) = self.undo_stack.pop() {
            self.buffer = Rope::from_str(&state.buffer);
            self.cursor_y = state.cursor_y;
            self.cursor_x = state.cursor_x;
            self.selection_start = None;
            self.is_dirty = true;
            self.ensure_cursor_visible(height);
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
        self.undo_stack.clear();
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
        self.save_undo_state();
        self.insert_char_raw(c);
    }

    fn insert_char_raw(&mut self, c: char) {
        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        self.buffer.insert_char(char_idx, c);
        self.cursor_x += 1;
        self.is_dirty = true;
        self.selection_start = None;
    }

    /// Delete the character to the right of the cursor (Delete key behaviour).
    pub fn delete_char_forward(&mut self) {
        self.save_undo_state();
        if self.selection_start.is_some() {
            self.delete_selection();
            return;
        }
        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        let total_chars = self.buffer.len_chars();
        if char_idx < total_chars {
            self.buffer.remove(char_idx..char_idx + 1);
            self.is_dirty = true;
        }
    }

    /// Copy selection (or current line) to clipboard, then delete it.
    pub fn cut(&mut self) {
        self.save_undo_state();
        self.copy();
        if self.selection_start.is_some() {
            self.delete_selection();
        } else {
            // Cut the entire current line including its newline
            let line_start = self.buffer.line_to_char(self.cursor_y);
            let line_end = if self.cursor_y + 1 < self.buffer.len_lines() {
                self.buffer.line_to_char(self.cursor_y + 1)
            } else {
                self.buffer.len_chars()
            };
            if line_start < line_end {
                self.buffer.remove(line_start..line_end);
                self.cursor_x = 0;
                let max_y = self.buffer.len_lines().saturating_sub(1);
                if self.cursor_y > max_y { self.cursor_y = max_y; }
                self.is_dirty = true;
            }
        }
    }

    pub fn delete_char(&mut self) {
        self.save_undo_state();
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

            // Resolve selection to a char-offset range for this line, or None if
            // this line has no selected characters.
            // sel_end == usize::MAX means "to end of line" (interior of multi-line selection).
            let line_sel: Option<(usize, usize)> = if let Some((start_y, start_x)) = self.selection_start {
                let (sy, sx, ey, ex) = if (start_y, start_x) < (self.cursor_y, self.cursor_x) {
                    (start_y, start_x, self.cursor_y, self.cursor_x)
                } else {
                    (self.cursor_y, self.cursor_x, start_y, start_x)
                };
                if i < sy || i > ey {
                    None
                } else {
                    let sel_start = if i == sy { sx } else { 0 };
                    let sel_end   = if i == ey { ex } else { usize::MAX };
                    Some((sel_start, sel_end))
                }
            } else {
                None
            };

            let mut spans: Vec<ratatui::text::Span> = Vec::new();
            let mut char_pos = 0usize;

            for (style, text) in highlights {
                let base_style = ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                ));
                let sel_style = base_style
                    .bg(ratatui::style::Color::Yellow)
                    .fg(ratatui::style::Color::Black);

                let chars: Vec<char> = text.chars().collect();
                let text_len = chars.len();
                let span_start = char_pos;
                let span_end   = char_pos + text_len;

                match line_sel {
                    None => {
                        spans.push(ratatui::text::Span::styled(text.to_string(), base_style));
                    }
                    Some((sel_start, sel_end)) => {
                        // Before selection
                        if span_start < sel_start {
                            let end = sel_start.min(span_end);
                            let s: String = chars[..end - span_start].iter().collect();
                            if !s.is_empty() {
                                spans.push(ratatui::text::Span::styled(s, base_style));
                            }
                        }
                        // Within selection
                        let ov_start = sel_start.max(span_start);
                        let ov_end   = sel_end.min(span_end);
                        if ov_start < ov_end {
                            let s: String = chars[ov_start - span_start..ov_end - span_start].iter().collect();
                            if !s.is_empty() {
                                spans.push(ratatui::text::Span::styled(s, sel_style));
                            }
                        }
                        // After selection
                        if span_end > sel_end && sel_end != usize::MAX {
                            let start = sel_end.max(span_start);
                            let s: String = chars[start - span_start..].iter().collect();
                            if !s.is_empty() {
                                spans.push(ratatui::text::Span::styled(s, base_style));
                            }
                        }
                    }
                }

                char_pos += text_len;
            }

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
        self.save_undo_state();
        self.insert_char_raw(' ');
        self.insert_char_raw(' ');
        self.insert_char_raw(' ');
        self.insert_char_raw(' ');
    }

    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_line = self.buffer.len_lines().saturating_sub(1);
        self.cursor_y = last_line;
        self.cursor_x = self.get_max_cursor_x(last_line);
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
                self.insert_paste(&text, height);
            }
        }
    }

    /// Insert arbitrary text at the cursor (used for bracketed paste events).
    pub fn insert_paste(&mut self, text: &str, height: usize) {
        self.save_undo_state();
        if self.selection_start.is_some() {
            self.delete_selection();
        }

        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        self.buffer.insert(char_idx, text);

        let text_rope = Rope::from_str(text);
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

    pub fn ensure_cursor_visible(&mut self, height: usize) {
        if height == 0 { return; }
        
        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        } else if self.cursor_y >= self.scroll_y + height {
            self.scroll_y = self.cursor_y - height + 1;
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

    pub fn move_to_line_start(&mut self) {
        self.cursor_x = 0;
    }

    pub fn move_to_line_end(&mut self) {
        self.cursor_x = self.get_max_cursor_x(self.cursor_y);
    }

    pub fn move_to_file_start(&mut self) {
        self.cursor_y = 0;
        self.cursor_x = 0;
        self.scroll_y = 0;
    }

    pub fn move_to_file_end(&mut self, height: usize) {
        let last_line = self.buffer.len_lines().saturating_sub(1);
        self.cursor_y = last_line;
        self.cursor_x = self.get_max_cursor_x(last_line);
        self.ensure_cursor_visible(height);
    }

    pub fn page_up(&mut self, height: usize) {
        if height == 0 { return; }
        self.cursor_y = self.cursor_y.saturating_sub(height);
        self.clamp_cursor_x();
        self.ensure_cursor_visible(height);
    }

    pub fn page_down(&mut self, height: usize) {
        if height == 0 { return; }
        let last_line = self.buffer.len_lines().saturating_sub(1);
        self.cursor_y = (self.cursor_y + height).min(last_line);
        self.clamp_cursor_x();
        self.ensure_cursor_visible(height);
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
