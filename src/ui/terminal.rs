use crate::app::{App, Panel};
use crate::config;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    app.terminal_area.set(area);
    let parser_lock = app.terminal.parser.lock().unwrap();
    let mut screen = parser_lock.screen().clone();
    
    // We update the terminal size to the ratatui area if needed
    let term_width = area.width.saturating_sub(2);
    let term_height = area.height.saturating_sub(2);
    if term_width > 0 && term_height > 0 {
        if screen.size() != (term_height, term_width) {
            // Note: In a real app we would resize the PTY here using master_pty.resize().
            // For now we just resize the parser screen to match the layout.
            screen.set_size(term_height, term_width);
        }
    }

    screen.set_scrollback(app.terminal_scroll);
    let actual_scroll = screen.scrollback();

    let (rows, cols) = screen.size();
    
    let terminal_lines: Vec<ratatui::text::Line<'_>> = (0..rows)
        .map(|r| {
            let mut spans = Vec::new();
            let mut current_text = String::new();
            let mut current_attrs: Option<(vt100::Cell, bool)> = None;
            let abs_y = r; // Absolute Y for selection is tricky with scrollback, but let's approximate
            
            // To properly do selection check, we would need absolute line number.
            // Since VT100 doesn't expose absolute line index easily, we will skip selection highlights
            // inside the vt100 grid for this quick fix, or just do a simple highlight.
            // We can check if (abs_y, c) is within app.terminal_sel.

            for c in 0..cols {
                if let Some(cell) = screen.cell(r, c) {
                    let new_attrs = cell.clone();
                    let char_str = if cell.has_contents() { cell.contents() } else { " " };
                    
                    // For selection:
                    let is_selected = if let Some((sel_start, sel_end)) = app.terminal_sel {
                        let (sy, sx) = if sel_start < sel_end { sel_start } else { sel_end };
                        let (ey, ex) = if sel_start < sel_end { sel_end } else { sel_start };
                        
                        let cur_y = abs_y as usize;
                        let cur_x = c as usize;
                        if cur_y > sy && cur_y < ey {
                            true
                        } else if cur_y == sy && cur_y == ey {
                            cur_x >= sx && cur_x <= ex
                        } else if cur_y == sy {
                            cur_x >= sx
                        } else if cur_y == ey {
                            cur_x <= ex
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    
                    if let Some((ref attrs, was_selected)) = current_attrs {
                        if attrs.fgcolor() != cell.fgcolor() || attrs.bgcolor() != cell.bgcolor() || attrs.bold() != cell.bold() || was_selected != is_selected {
                            // Flush current_text
                            let mut style = ratatui::style::Style::default();
                            match attrs.fgcolor() {
                                vt100::Color::Idx(idx) => style = style.fg(ratatui::style::Color::Indexed(idx)),
                                vt100::Color::Rgb(r, g, b) => style = style.fg(ratatui::style::Color::Rgb(r, g, b)),
                                _ => {}
                            }
                            match attrs.bgcolor() {
                                vt100::Color::Idx(idx) => style = style.bg(ratatui::style::Color::Indexed(idx)),
                                vt100::Color::Rgb(r, g, b) => style = style.bg(ratatui::style::Color::Rgb(r, g, b)),
                                _ => {}
                            }
                            if attrs.bold() {
                                style = style.add_modifier(ratatui::style::Modifier::BOLD);
                            }
                            if was_selected {
                                style = style.bg(ratatui::style::Color::White).fg(ratatui::style::Color::Black);
                            }
                            spans.push(ratatui::text::Span::styled(current_text.clone(), style));
                            current_text.clear();
                        }
                    }
                    
                    current_text.push_str(char_str);
                    current_attrs = Some((new_attrs, is_selected));
                }
            }
            if !current_text.is_empty() {
                let mut style = ratatui::style::Style::default();
                if let Some((ref attrs, was_selected)) = current_attrs {
                    match attrs.fgcolor() {
                        vt100::Color::Idx(idx) => style = style.fg(ratatui::style::Color::Indexed(idx)),
                        vt100::Color::Rgb(r, g, b) => style = style.fg(ratatui::style::Color::Rgb(r, g, b)),
                        _ => {}
                    }
                    match attrs.bgcolor() {
                        vt100::Color::Idx(idx) => style = style.bg(ratatui::style::Color::Indexed(idx)),
                        vt100::Color::Rgb(r, g, b) => style = style.bg(ratatui::style::Color::Rgb(r, g, b)),
                        _ => {}
                    }
                    if attrs.bold() {
                        style = style.add_modifier(ratatui::style::Modifier::BOLD);
                    }
                    if was_selected {
                        style = style.bg(ratatui::style::Color::White).fg(ratatui::style::Color::Black);
                    }
                }
                spans.push(ratatui::text::Span::styled(current_text, style));
            }
            ratatui::text::Line::from(spans)
        })
        .collect();

    let terminal_block = Block::default()
        .title(" Terminal ")
        .borders(Borders::ALL)
        .border_style(if matches!(app.active_panel, Panel::Terminal) {
            ratatui::style::Style::default().fg(config::colors::TERMINAL_FOCUS)
        } else {
            ratatui::style::Style::default()
        });

    let terminal_widget = Paragraph::new(terminal_lines).block(terminal_block);
    f.render_widget(terminal_widget, area);

    // Show cursor in terminal if active and not scrolled back
    if matches!(app.active_panel, Panel::Terminal) && actual_scroll == 0 {
        let (r, c) = screen.cursor_position();
        let inner = Block::default().borders(Borders::ALL).inner(area);
        f.set_cursor(
            inner.x + c,
            inner.y + r,
        );
    }
}

pub fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' {
            let _start = i;
            i += 1;
            if i >= chars.len() {
                break;
            }
            match chars[i] {
                '[' => {
                    // CSI
                    i += 1;
                    let mut found = false;
                    while i < chars.len() {
                        let c = chars[i];
                        i += 1;
                        if (c as u32) >= 0x40 && (c as u32) <= 0x7E {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        break;
                    } // Truncate partial
                }
                ']' => {
                    // OSC (Window title etc)
                    i += 1;
                    let mut found = false;
                    while i < chars.len() {
                        if chars[i] == '\x07' {
                            i += 1;
                            found = true;
                            break;
                        }
                        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '\\' {
                            found = true;
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    if !found {
                        break;
                    }
                }
                '(' | ')' | '*' | '+' | '-' | '.' | '/' => {
                    // Charset
                    if i + 1 >= chars.len() {
                        break;
                    }
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        } else if chars[i] == '\x08' {
            result.pop();
            i += 1;
        } else {
            let c = chars[i];
            if c == '\r' {
                i += 1;
                continue;
            } // Skip \r for clean display in TUI
            if (c as u32) >= 32 || c == '\n' || c == '\t' {
                result.push(c);
            }
            i += 1;
        }
    }
    result
}
