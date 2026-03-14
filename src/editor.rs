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
    pub tree: Option<tree_sitter::Tree>,
    pub expansion_stack: Vec<((usize, usize), (usize, usize))>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
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
            tree: None,
            expansion_stack: Vec::new(),
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

    pub fn reparse(&mut self, ts_manager: &crate::treesitter::TSManager) {
        if let Some(path) = &self.path {
            if let Some(mut parser) = ts_manager.create_parser_for_file(path) {
                let content = self.buffer.to_string();
                // If we already have a tree that was edited, parse will be incremental.
                self.tree = parser.parse(content, self.tree.as_ref());
            }
        }
    }

    pub fn handle_edit(&mut self, edit: tree_sitter::InputEdit) {
        if let Some(tree) = &mut self.tree {
            tree.edit(&edit);
        }
    }

    fn get_ts_point(&self, char_idx: usize) -> tree_sitter::Point {
        let line = self.buffer.char_to_line(char_idx);
        let line_start = self.buffer.line_to_char(line);
        let col = char_idx - line_start;
        let line_slice = self.buffer.line(line);
        let col_bytes = line_slice.slice(0..col).len_bytes();
        tree_sitter::Point::new(line, col_bytes)
    }

    fn buffer_insert(&mut self, char_idx: usize, text: &str) {
        let start_byte = self.buffer.char_to_byte(char_idx);
        let start_point = self.get_ts_point(char_idx);

        self.buffer.insert(char_idx, text);

        let new_end_byte = start_byte + text.len();
        let new_end_point = self.get_ts_point(char_idx + text.chars().count());

        self.handle_edit(tree_sitter::InputEdit {
            start_byte,
            old_end_byte: start_byte,
            new_end_byte,
            start_position: start_point,
            old_end_position: start_point,
            new_end_position: new_end_point,
        });
    }

    fn buffer_remove(&mut self, range: std::ops::Range<usize>) {
        let start_byte = self.buffer.char_to_byte(range.start);
        let end_byte = self.buffer.char_to_byte(range.end);
        let start_point = self.get_ts_point(range.start);
        let end_point = self.get_ts_point(range.end);

        self.buffer.remove(range);

        self.handle_edit(tree_sitter::InputEdit {
            start_byte,
            old_end_byte: end_byte,
            new_end_byte: start_byte,
            start_position: start_point,
            old_end_position: end_point,
            new_end_position: start_point,
        });
    }

    fn ts_point_to_char_col(&self, point: tree_sitter::Point) -> (usize, usize) {
        let line = point.row;
        if line >= self.buffer.len_lines() {
            return (self.buffer.len_lines().saturating_sub(1), 0);
        }
        let line_slice = self.buffer.line(line);
        let char_col = line_slice.byte_to_char(point.column);
        (line, char_col)
    }

    pub fn expand_selection(&mut self) {
        if let Some(tree) = &self.tree {
            let char_idx = self.buffer.line_to_char(self.cursor_y) + self.cursor_x;

            // Current selection range in bytes
            let (start_byte, end_byte) = if let Some((sy, sx)) = self.selection_start {
                let (sy, sx, ey, ex) = if (sy, sx) < (self.cursor_y, self.cursor_x) {
                    (sy, sx, self.cursor_y, self.cursor_x)
                } else {
                    (self.cursor_y, self.cursor_x, sy, sx)
                };
                let sb =
                    self.buffer.line_to_byte(sy) + self.buffer.line(sy).slice(0..sx).len_bytes();
                let eb =
                    self.buffer.line_to_byte(ey) + self.buffer.line(ey).slice(0..ex).len_bytes();
                (sb, eb)
            } else {
                let sb = self.buffer.char_to_byte(char_idx);
                (sb, sb)
            };

            if let Some(node) = tree
                .root_node()
                .descendant_for_byte_range(start_byte, end_byte)
            {
                let mut target_node = node;

                // If the node exactly matches current selection, pick its parent
                if target_node.start_byte() == start_byte && target_node.end_byte() == end_byte {
                    if let Some(parent) = target_node.parent() {
                        target_node = parent;
                    }
                }

                // Save current state
                self.expansion_stack.push((
                    self.selection_start
                        .unwrap_or((self.cursor_y, self.cursor_x)),
                    (self.cursor_y, self.cursor_x),
                ));

                // Apply new selection
                let (sy, sx) = self.ts_point_to_char_col(target_node.start_position());
                let (ey, ex) = self.ts_point_to_char_col(target_node.end_position());

                self.selection_start = Some((sy, sx));
                self.cursor_y = ey;
                self.cursor_x = ex;
            }
        }
    }

    pub fn shrink_selection(&mut self) {
        if let Some((start, cursor)) = self.expansion_stack.pop() {
            self.selection_start = if start == cursor { None } else { Some(start) };
            self.cursor_y = cursor.0;
            self.cursor_x = cursor.1;
        }
    }

    pub fn swap_nodes(&mut self, right: bool) {
        let ranges = if let Some(tree) = &self.tree {
            let char_idx = self.buffer.line_to_char(self.cursor_y) + self.cursor_x;
            let byte_idx = self.buffer.char_to_byte(char_idx);

            if let Some(node) = tree
                .root_node()
                .descendant_for_byte_range(byte_idx, byte_idx)
            {
                let mut target_node = node;
                let mut result = None;
                while let Some(parent) = target_node.parent() {
                    let mut cursor = parent.walk();
                    let mut siblings = Vec::new();
                    if cursor.goto_first_child() {
                        loop {
                            siblings.push(cursor.node());
                            if !cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }

                    if let Some(pos) = siblings.iter().position(|n| n.id() == target_node.id()) {
                        let sibling_idx = if right {
                            pos + 1
                        } else {
                            pos.saturating_sub(1)
                        };

                        if sibling_idx < siblings.len() && sibling_idx != pos {
                            result = Some((
                                target_node.byte_range(),
                                siblings[sibling_idx].byte_range(),
                            ));
                            break;
                        }
                    }
                    target_node = parent;
                }
                result
            } else {
                None
            }
        } else {
            None
        };

        if let Some((r1, r2)) = ranges {
            self.swap_byte_ranges(r1, r2);
        }
    }

    fn swap_byte_ranges(&mut self, r1: std::ops::Range<usize>, r2: std::ops::Range<usize>) {
        let (first_r, second_r) = if r1.start < r2.start {
            (r1, r2)
        } else {
            (r2, r1)
        };

        let first_text = self
            .buffer
            .slice(self.buffer.byte_to_char(first_r.start)..self.buffer.byte_to_char(first_r.end))
            .to_string();
        let second_text = self
            .buffer
            .slice(self.buffer.byte_to_char(second_r.start)..self.buffer.byte_to_char(second_r.end))
            .to_string();

        let first_char_range =
            self.buffer.byte_to_char(first_r.start)..self.buffer.byte_to_char(first_r.end);
        let second_char_range =
            self.buffer.byte_to_char(second_r.start)..self.buffer.byte_to_char(second_r.end);

        // Replace second first to keep first's indices valid
        self.buffer_remove(second_char_range.clone());
        self.buffer_insert(second_char_range.start, &first_text);

        self.buffer_remove(first_char_range.clone());
        self.buffer_insert(first_char_range.start, &second_text);
    }

    pub fn move_block(&mut self, down: bool) {
        let ranges = if let Some(tree) = &self.tree {
            let char_idx = self.buffer.line_to_char(self.cursor_y) + self.cursor_x;
            let byte_idx = self.buffer.char_to_byte(char_idx);

            if let Some(node) = tree
                .root_node()
                .descendant_for_byte_range(byte_idx, byte_idx)
            {
                let mut target_node = node;
                let mut result = None;
                while let Some(parent) = target_node.parent() {
                    let mut cursor = parent.walk();
                    let mut siblings = Vec::new();
                    if cursor.goto_first_child() {
                        loop {
                            siblings.push(cursor.node());
                            if !cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }

                    if let Some(pos) = siblings.iter().position(|n| n.id() == target_node.id()) {
                        let sibling_idx = if down { pos + 1 } else { pos.saturating_sub(1) };

                        if sibling_idx < siblings.len() && sibling_idx != pos {
                            result = Some((
                                target_node.byte_range(),
                                siblings[sibling_idx].byte_range(),
                            ));
                            break;
                        }
                    }
                    target_node = parent;
                }
                result
            } else {
                None
            }
        } else {
            None
        };

        if let Some((r1, r2)) = ranges {
            self.swap_byte_ranges(r1.clone(), r2.clone());
            // Move cursor to stay with the block (simplified)
            let new_char_idx = self.buffer.byte_to_char(if r1.start < r2.start {
                r2.start
            } else {
                r1.start
            });
            self.cursor_y = self.buffer.char_to_line(new_char_idx);
            self.cursor_x = new_char_idx - self.buffer.line_to_char(self.cursor_y);
        }
    }

    pub fn open(&mut self, path: PathBuf, ts_manager: &crate::treesitter::TSManager) -> Result<()> {
        let content = fs::read_to_string(&path)?;
        self.buffer = Rope::from_str(&content);
        self.path = Some(path);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.scroll_y = 0;
        self.is_dirty = false;
        self.selection_start = None;
        self.undo_stack.clear();

        // Initial parse
        self.reparse(ts_manager);

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
        let mut s = String::new();
        s.push(c);
        self.buffer_insert(char_idx, &s);
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
                self.buffer_remove(char_idx - 1..char_idx);
                self.cursor_x -= 1;
            } else if self.cursor_y > 0 {
                // Join lines
                let prev_line_len = self.buffer.line(self.cursor_y - 1).len_chars();
                self.buffer_remove(char_idx - 1..char_idx); // Remove newline
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
            self.buffer_remove(char_idx..end_idx);
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
                self.buffer_remove(start_char..end_char);
                self.cursor_x = 0;
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
        if let Some(tree) = &self.tree {
            return self.get_ts_highlighted_lines(tree, height);
        }

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
        self.buffer_insert(char_idx, "    ");
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
        self.buffer_insert(char_idx, text);

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

    fn get_ts_style(&self, kind: &str) -> ratatui::style::Style {
        use ratatui::style::{Color, Modifier, Style};
        let style = Style::default();
        match kind {
            "keyword" | "storage_class" | "type_qualifier" | "repeat" | "conditional"
            | "exception" | "include" | "statement" => {
                style.fg(Color::Magenta).add_modifier(Modifier::BOLD)
            }
            "type" | "primitive_type" | "type_identifier" => style.fg(Color::Blue),
            "function" | "method" | "function_item" | "call_expression" | "function_declarator" => {
                style.fg(Color::Cyan)
            }
            "string" | "string_literal" | "char_literal" | "escape_sequence" => {
                style.fg(Color::Yellow)
            }
            "comment" | "line_comment" | "block_comment" => {
                style.fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            }
            "number" | "integer_literal" | "float_literal" => style.fg(Color::LightRed),
            "operator" | "binary_expression" | "unary_expression" => style.fg(Color::White),
            "punctuation" | "delimiter" | "bracket" => style.fg(Color::White),
            "identifier" | "field_identifier" => style.fg(Color::White),
            _ => style.fg(Color::White),
        }
    }

    fn get_ts_highlighted_lines(
        &self,
        tree: &tree_sitter::Tree,
        height: usize,
    ) -> Vec<ratatui::text::Line<'_>> {
        let start_line = self.scroll_y;
        let end_line = (start_line + height).min(self.buffer.len_lines());
        let mut lines = Vec::new();

        for i in start_line..end_line {
            let line_obj = self.buffer.line(i);
            let line_start_char = self.buffer.line_to_char(i);
            let start_byte = self.buffer.char_to_byte(line_start_char);
            let end_byte = start_byte + line_obj.len_bytes();

            let mut spans = Vec::new();
            let mut current_byte = start_byte;

            let node = tree
                .root_node()
                .descendant_for_byte_range(start_byte, end_byte)
                .unwrap_or(tree.root_node());
            self.walk_line_highlights(node, start_byte, end_byte, &mut current_byte, &mut spans);

            if current_byte < end_byte {
                let remaining_text = self.get_byte_range_text(current_byte..end_byte);
                if !remaining_text.is_empty() {
                    spans.push(ratatui::text::Span::styled(
                        remaining_text,
                        ratatui::style::Style::default().fg(ratatui::style::Color::White),
                    ));
                }
            }

            let mut cleaned_spans = Vec::new();
            for span in spans {
                let text = span.content.trim_end_matches(['\n', '\r']);
                if !text.is_empty() {
                    cleaned_spans.push(ratatui::text::Span::styled(text.to_string(), span.style));
                }
            }
            lines.push(ratatui::text::Line::from(cleaned_spans));
        }

        while lines.len() < height {
            lines.push(ratatui::text::Line::from(" "));
        }
        lines
    }

    fn walk_line_highlights(
        &self,
        node: tree_sitter::Node,
        line_start: usize,
        line_end: usize,
        current_byte: &mut usize,
        spans: &mut Vec<ratatui::text::Span>,
    ) {
        if node.start_byte() >= line_end || node.end_byte() <= line_start {
            return;
        }

        if node.child_count() == 0 {
            let node_start = node.start_byte();
            let node_end = node.end_byte();

            if node_start > *current_byte {
                let gap_text = self.get_byte_range_text(*current_byte..node_start);
                if !gap_text.is_empty() {
                    spans.push(ratatui::text::Span::styled(
                        gap_text,
                        ratatui::style::Style::default().fg(ratatui::style::Color::White),
                    ));
                }
            }

            let start = node_start.max(*current_byte);
            let end = node_end.min(line_end);
            if start < end {
                let text = self.get_byte_range_text(start..end);
                spans.push(ratatui::text::Span::styled(
                    text,
                    self.get_ts_style(node.kind()),
                ));
                *current_byte = end;
            }
        } else {
            for i in 0..node.child_count() {
                self.walk_line_highlights(
                    node.child(i).unwrap(),
                    line_start,
                    line_end,
                    current_byte,
                    spans,
                );
            }
        }
    }

    fn get_byte_range_text(&self, range: std::ops::Range<usize>) -> String {
        let start = self.buffer.byte_to_char(range.start);
        let end = self.buffer.byte_to_char(range.end);
        self.buffer.slice(start..end).to_string()
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
