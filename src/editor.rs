use anyhow::Result;
use ropey::Rope;
use std::fs;
use std::path::PathBuf;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

#[derive(Clone)]
pub struct UndoState {
    pub buffer: Rope,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub selection_start: Option<(usize, usize)>,
}

pub struct Editor {
    pub buffer: Rope,
    pub path: Option<PathBuf>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
    pub selection_start: Option<(usize, usize)>,
    pub is_dirty: bool,
    pub undo_stack: Vec<UndoState>,
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
            selection_start: None,
            is_dirty: false,
            undo_stack: Vec::new(),
        }
    }

    pub fn save_undo_state(&mut self) {
        self.undo_stack.push(UndoState {
            buffer: self.buffer.clone(),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            selection_start: self.selection_start,
        });
        if self.undo_stack.len() > 200 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(state) = self.undo_stack.pop() {
            self.buffer = state.buffer;
            self.cursor_x = state.cursor_x;
            self.cursor_y = state.cursor_y;
            self.selection_start = state.selection_start;
            self.is_dirty = true;
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
        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        self.buffer.insert_char(char_idx, c);
        self.cursor_x += 1;
        self.is_dirty = true;
        self.selection_start = None;
    }

    pub fn delete_char(&mut self) {
        self.save_undo_state();
        if self.selection_start.is_some() {
            self.delete_selection_internal();
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

    pub fn delete_forward_char(&mut self) {
        self.save_undo_state();
        if self.selection_start.is_some() {
            self.delete_selection_internal();
            return;
        }

        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;

        if char_idx < self.buffer.len_chars() {
            let mut end_idx = char_idx + 1;
            // Handle CRLF windows newlines gracefully by deleting both parts
            if self.buffer.char(char_idx) == '\r'
                && end_idx < self.buffer.len_chars()
                && self.buffer.char(end_idx) == '\n'
            {
                end_idx += 1;
            }
            self.buffer.remove(char_idx..end_idx);
            self.is_dirty = true;
        }
    }

    #[allow(dead_code)]
    pub fn delete_selection(&mut self) {
        if self.selection_start.is_some() {
            self.save_undo_state();
            self.delete_selection_internal();
        }
    }

    fn delete_selection_internal(&mut self) {
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

    pub fn get_highlighted_lines(
        &self,
        _width: usize,
        height: usize,
    ) -> Vec<ratatui::text::Line<'_>> {
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
            let highlights = h
                .highlight_line(&line, &self.syntax_set)
                .unwrap_or_default();

            let mut current_char_in_line = 0;
            let mut spans: Vec<ratatui::text::Span> = Vec::new();

            for (style, text) in highlights {
                // Strip line terminators to prevent Ratatui from wrapping/double-spacing
                let text = text.trim_end_matches(['\n', '\r']);
                if text.is_empty() {
                    continue;
                }

                let span_style = ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                ));

                if let Some((start_y, start_x)) = self.selection_start {
                    let (sy, sx, ey, ex) = if (start_y, start_x) < (self.cursor_y, self.cursor_x) {
                        (start_y, start_x, self.cursor_y, self.cursor_x)
                    } else {
                        (self.cursor_y, self.cursor_x, start_y, start_x)
                    };

                    let line_idx = i;
                    let mut current_segment = String::new();
                    let mut current_is_selected = false;

                    for (idx, c) in text.chars().enumerate() {
                        let char_pos = current_char_in_line + idx;
                        let is_char_selected = if line_idx > sy && line_idx < ey {
                            true
                        } else if line_idx == sy && line_idx == ey {
                            char_pos >= sx && char_pos < ex
                        } else if line_idx == sy {
                            char_pos >= sx
                        } else if line_idx == ey {
                            char_pos < ex
                        } else {
                            false
                        };

                        if idx == 0 {
                            current_is_selected = is_char_selected;
                        } else if is_char_selected != current_is_selected {
                            let mut s_style = span_style;
                            if current_is_selected {
                                s_style = s_style
                                    .bg(ratatui::style::Color::Yellow)
                                    .fg(ratatui::style::Color::Black);
                            }
                            spans.push(ratatui::text::Span::styled(
                                current_segment.clone(),
                                s_style,
                            ));
                            current_segment.clear();
                            current_is_selected = is_char_selected;
                        }
                        current_segment.push(c);
                    }

                    if !current_segment.is_empty() {
                        let mut s_style = span_style;
                        if current_is_selected {
                            s_style = s_style
                                .bg(ratatui::style::Color::Yellow)
                                .fg(ratatui::style::Color::Black);
                        }
                        spans.push(ratatui::text::Span::styled(current_segment, s_style));
                    }
                    current_char_in_line += text.chars().count();
                } else {
                    current_char_in_line += text.chars().count();
                    spans.push(ratatui::text::Span::styled(text.to_string(), span_style));
                }
            }

            lines.push(ratatui::text::Line::from(spans));
        }

        // Fill remaining height with empty lines to ensure the background is fully drawn
        // and any debris from previous frames is overwritten.
        while lines.len() < height {
            lines.push(ratatui::text::Line::from(" "));
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
        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        self.buffer.insert(char_idx, "    ");
        self.cursor_x += 4;
        self.is_dirty = true;
        self.selection_start = None;
    }

    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_line = self.buffer.len_lines().saturating_sub(1);
        self.cursor_y = last_line;
        self.cursor_x = self.get_max_cursor_x(last_line);
    }

    pub fn copy(&mut self, clipboard: &mut Option<arboard::Clipboard>) {
        if let Some(clipboard) = clipboard {
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

    pub fn cut(&mut self, clipboard: &mut Option<arboard::Clipboard>) {
        self.save_undo_state();
        self.copy(clipboard);
        if self.selection_start.is_some() {
            self.delete_selection_internal();
        } else {
            // Nothing selected: cut whole line
            let start_char = self.buffer.line_to_char(self.cursor_y);
            let next_line_idx = (self.cursor_y + 1).min(self.buffer.len_lines());
            let end_char = if next_line_idx < self.buffer.len_lines() {
                self.buffer.line_to_char(next_line_idx)
            } else {
                self.buffer.len_chars()
            };
            if start_char < end_char {
                self.buffer.remove(start_char..end_char);
                self.cursor_x = 0;
                self.is_dirty = true;
            }
        }
    }

    pub fn paste(&mut self, clipboard: &mut Option<arboard::Clipboard>, height: usize) {
        if let Some(clipboard) = clipboard {
            if let Ok(text) = clipboard.get_text() {
                self.insert_paste(&text, height);
            }
        }
    }

    pub fn insert_paste(&mut self, text: &str, height: usize) {
        self.save_undo_state();
        if self.selection_start.is_some() {
            self.delete_selection_internal();
        }

        let line_idx = self.buffer.line_to_char(self.cursor_y);
        let char_idx = line_idx + self.cursor_x;
        self.buffer.insert(char_idx, text);

        // Update cursor after paste
        let text_rope = Rope::from_str(text);
        if text_rope.len_lines() > 1 {
            self.cursor_y += text_rope.len_lines() - 1;
            self.cursor_x = text_rope.line(text_rope.len_lines() - 1).len_chars();
        } else {
            self.cursor_x += text.len();
        }
        self.is_dirty = true;
        self.clamp_cursor_x();

        // Ensure the cursor (at the end of the paste) is visible.
        // This naturally pushes the view down for large pastes, showing the text.
        self.ensure_cursor_visible(height);
    }

    pub fn ensure_cursor_visible(&mut self, height: usize) {
        if height == 0 {
            return;
        }

        if self.cursor_y < self.scroll_y {
            self.scroll_y = self.cursor_y;
        } else if self.cursor_y >= self.scroll_y + height {
            self.scroll_y = self.cursor_y - height + 1;
        }
    }

    pub fn get_max_cursor_x(&self, line_y: usize) -> usize {
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
