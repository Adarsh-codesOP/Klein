use crate::app::{App, Panel};
use crate::config;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    app.terminal_area.set(area);
    let output_raw = app.terminal.output.lock().unwrap();
    let output = strip_ansi(&output_raw);

    let lines: Vec<&str> = output.lines().collect();
    let height = area.height.saturating_sub(2) as usize;

    let max_scroll = lines.len().saturating_sub(height);
    let scroll = app.terminal_scroll.min(max_scroll);

    let start = lines.len().saturating_sub(height).saturating_sub(scroll);
    let end = lines.len().saturating_sub(scroll);

    let terminal_lines: Vec<ratatui::text::Line<'_>> = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let abs_y = start + i;
            let mut spans = Vec::new();

            if let Some((sel_start, sel_end)) = app.terminal_sel {
                let (sy, sx) = if sel_start < sel_end { sel_start } else { sel_end };
                let (ey, ex) = if sel_start < sel_end { sel_end } else { sel_start };

                if abs_y < sy || abs_y > ey {
                    spans.push(ratatui::text::Span::raw(l.to_string()));
                } else {
                    let start_col = if abs_y == sy { sx } else { 0 };
                    let end_col = if abs_y == ey { ex } else { l.chars().count() };

                    let chars: Vec<char> = l.chars().collect();
                    let before: String = chars.iter().take(start_col.min(chars.len())).collect();
                    let selected: String = chars.iter().skip(start_col.min(chars.len())).take(end_col.saturating_sub(start_col)).collect();
                    let after: String = chars.iter().skip(end_col.min(chars.len())).collect();

                    if !before.is_empty() {
                        spans.push(ratatui::text::Span::raw(before));
                    }
                    if !selected.is_empty() {
                        let style = ratatui::style::Style::default().bg(ratatui::style::Color::White).fg(ratatui::style::Color::Black);
                        spans.push(ratatui::text::Span::styled(selected, style));
                    }
                    if !after.is_empty() {
                        spans.push(ratatui::text::Span::raw(after));
                    }
                }
            } else {
                spans.push(ratatui::text::Span::raw(l.to_string()));
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
    if matches!(app.active_panel, Panel::Terminal) && scroll == 0 {
        let last_line = lines.last().copied().unwrap_or("");
        let inner = Block::default().borders(Borders::ALL).inner(area);
        f.set_cursor(
            inner.x + last_line.len() as u16,
            inner.y + lines.len().min(height).saturating_sub(1) as u16,
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
