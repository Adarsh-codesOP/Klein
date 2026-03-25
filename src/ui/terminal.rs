use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::{App, Panel};
use crate::config;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let output_raw = app.terminal.output.lock().unwrap();
    let lines = process_output(&output_raw);

    let height = area.height.saturating_sub(2) as usize;

    let max_scroll = lines.len().saturating_sub(height);
    let scroll = app.terminal_scroll.min(max_scroll);

    let start = lines.len().saturating_sub(height).saturating_sub(scroll);
    let end = lines.len().saturating_sub(scroll);

    // Compute inner rect and store it so mouse events can map screen → line coords.
    let inner = Block::default().borders(Borders::ALL).inner(area);
    app.terminal_area.set(inner);

    // Normalise selection into (top-left, bottom-right) order.
    let norm_sel = app.terminal_sel.map(|((c1, r1), (c2, r2))| {
        if (r1, c1) <= (r2, c2) { ((c1, r1), (c2, r2)) } else { ((c2, r2), (c1, r1)) }
    });

    let sel_style = Style::default().bg(Color::White).fg(Color::Black);

    let terminal_lines: Vec<Line<'_>> = lines[start..end]
        .iter()
        .enumerate()
        .map(|(row_idx, line_text)| {
            let row = row_idx as u16;

            if let Some(((sc, sr), (ec, er))) = norm_sel {
                if row >= sr && row <= er {
                    let chars: Vec<char> = line_text.chars().collect();
                    let line_len = chars.len() as u16;

                    let col_start = (if row == sr { sc } else { 0 }).min(line_len);
                    let col_end   = (if row == er { ec } else { line_len }).min(line_len);

                    if col_start < col_end {
                        let before:   String = chars[..col_start as usize].iter().collect();
                        let selected: String = chars[col_start as usize..col_end as usize].iter().collect();
                        let after:    String = chars[col_end as usize..].iter().collect();

                        let mut spans = Vec::new();
                        if !before.is_empty()   { spans.push(Span::raw(before)); }
                        if !selected.is_empty() { spans.push(Span::styled(selected, sel_style)); }
                        if !after.is_empty()    { spans.push(Span::raw(after)); }
                        return Line::from(spans);
                    }
                }
            }

            Line::from(line_text.clone())
        })
        .collect();

    let terminal_block = Block::default()
        .title(" Terminal ")
        .borders(Borders::ALL)
        .border_style(if matches!(app.active_panel, Panel::Terminal) {
            Style::default().fg(config::colors::TERMINAL_FOCUS)
        } else {
            Style::default()
        });

    let terminal_widget = Paragraph::new(terminal_lines).block(terminal_block);
    f.render_widget(terminal_widget, area);

    // Show cursor in terminal if active and not scrolled back
    if matches!(app.active_panel, Panel::Terminal) && scroll == 0 {
        let last_line = lines.last().map(|s| s.as_str()).unwrap_or("");
        f.set_cursor(
            inner.x + last_line.len() as u16,
            inner.y + lines.len().min(height).saturating_sub(1) as u16,
        );
    }
}

/// Process raw terminal bytes into a list of display lines using a 1-D cursor model.
///
/// Rather than treating the current line as a simple append buffer, we track a
/// cursor column and let writes *overwrite* characters at that position — exactly
/// as a real VT100 terminal does.  This is required for fish, which:
///
///   - uses `\x1b[G]` or `\x1b[H]` (not `\r`) to return to column 0
///   - uses `\x1b[K]` to erase from the cursor to end-of-line
///   - uses `\x1b7`/`\x1b8` (DEC) or `\x1b[s`/`\x1b[u` (CSI) to bracket
///     autosuggestion text so it can be hidden by restoring the saved cursor
///
/// The display value of the in-progress line is always `buf[..cursor]`, so
/// any autosuggestion text written beyond the saved-and-restored cursor
/// position is automatically hidden.
///
/// PTY output line endings are `\r\n` (ONLCR).  A bare `\n` also commits the
/// line.  A bare `\r` (no following `\n`) moves the cursor to column 0.
pub fn process_output(raw: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // 1-D line buffer: characters may be overwritten at any column.
    let mut buf: Vec<char> = Vec::new();
    // Current cursor column (0-indexed write position within buf).
    let mut col: usize = 0;
    // Saved cursor column for \x1b7 / \x1b8 / \x1b[s / \x1b[u.
    let mut saved: Option<usize> = None;

    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;

    // Write one character at the current cursor column, then advance.
    macro_rules! put {
        ($c:expr) => {{
            if col < buf.len() {
                buf[col] = $c;
            } else {
                // Pad with spaces if the cursor jumped past the current end.
                while buf.len() < col { buf.push(' '); }
                buf.push($c);
            }
            col += 1;
        }};
    }

    // Commit the visible portion of the current line and reset.
    //
    // Normally we show buf[..col] so autosuggestion bytes (written beyond the
    // cursor by fish and then hidden via cursor-restore) are excluded.
    //
    // Special case: if col==0 but buf is non-empty, a preceding standalone \r
    // already reset the cursor to column 0 (e.g. fish positioning before the
    // prompt) while content from the previous command still sits in the buffer
    // (happens when cat outputs a file without a trailing newline).  In that
    // case commit the full buffer so the last output line is not lost.
    macro_rules! commit {
        () => {{
            let end = if col == 0 && !buf.is_empty() { buf.len() } else { col.min(buf.len()) };
            lines.push(buf[..end].iter().collect());
            buf.clear();
            col = 0;
            saved = None;
        }};
    }

    while i < chars.len() {
        match chars[i] {
            '\x1b' => {
                i += 1;
                if i >= chars.len() { break; }
                match chars[i] {
                    '[' => {
                        // CSI sequence: collect params, then dispatch on final byte.
                        i += 1;
                        let param_start = i;
                        while i < chars.len() {
                            let c = chars[i];
                            i += 1;
                            if (c as u32) >= 0x40 && (c as u32) <= 0x7E {
                                let params: String = chars[param_start..i - 1].iter().collect();
                                match c {
                                    // Cursor horizontal absolute (1-indexed → 0-indexed).
                                    'G' => {
                                        col = params.parse::<usize>()
                                            .unwrap_or(1)
                                            .saturating_sub(1);
                                    }
                                    // Cursor position / cursor home.
                                    'H' => {
                                        let col_param = params.split(';').nth(1)
                                            .and_then(|s| s.parse().ok())
                                            .unwrap_or(1usize);
                                        col = col_param.saturating_sub(1);
                                        // Note: row is ignored in our 1-D model.
                                    }
                                    // Erase in display.
                                    'J' => {
                                        if params == "2" {
                                            // ESC[2J — erase entire display.
                                            // Clear all committed lines and the current
                                            // line buffer so the terminal starts fresh.
                                            lines.clear();
                                            buf.clear();
                                            col = 0;
                                            saved = None;
                                        }
                                        // ESC[J / ESC[0J (erase to end of screen) — no-op
                                        // in our 1-D model; ESC[K handles the line level.
                                    }
                                    // Erase in line.
                                    'K' => {
                                        match params.as_str() {
                                            "" | "0" => buf.truncate(col), // to end
                                            "1" => {
                                                // to start: blank out [0..col]
                                                for b in buf.iter_mut().take(col) { *b = ' '; }
                                            }
                                            "2" => { buf.clear(); col = 0; } // whole line
                                            _ => {}
                                        }
                                    }
                                    // Cursor right / left — used by fish for sub-line positioning.
                                    'C' => {
                                        col += params.parse::<usize>().unwrap_or(1);
                                    }
                                    'D' => {
                                        col = col.saturating_sub(
                                            params.parse::<usize>().unwrap_or(1));
                                    }
                                    // CSI save / restore cursor.
                                    's' if params.is_empty() => { saved = Some(col); }
                                    'u' if params.is_empty() => {
                                        if let Some(p) = saved { col = p; }
                                    }
                                    _ => {} // all other CSI sequences: stripped
                                }
                                break;
                            }
                        }
                    }
                    ']' => {
                        // OSC sequence — strip until BEL or ST.
                        i += 1;
                        while i < chars.len() {
                            if chars[i] == '\x07' { i += 1; break; }
                            if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i+1] == '\\' {
                                i += 2; break;
                            }
                            i += 1;
                        }
                    }
                    // DEC save / restore cursor.
                    '7' => { saved = Some(col); i += 1; }
                    '8' => { if let Some(p) = saved { col = p; } i += 1; }
                    '(' | ')' | '*' | '+' | '-' | '.' | '/' => {
                        if i + 1 < chars.len() { i += 2; } else { break; }
                    }
                    _ => { i += 1; }
                }
            }
            '\x08' => {
                // Backspace: move cursor left without erasing.
                if col > 0 { col -= 1; }
                i += 1;
            }
            '\r' => {
                if i + 1 < chars.len() && chars[i + 1] == '\n' {
                    // CRLF — PTY ONLCR line ending for subprocess output.
                    // Commit the visible content (up to current col) as a complete line.
                    commit!();
                    i += 2;
                } else {
                    // Bare CR — go to column 0 (fish prompt redraw, bash readline).
                    col = 0;
                    i += 1;
                }
            }
            '\n' => {
                commit!();
                i += 1;
            }
            c => {
                if (c as u32) >= 32 || c == '\t' {
                    put!(c);
                }
                i += 1;
            }
        }
    }

    // The in-progress line (live prompt / partial input) is always the last element.
    let end = col.min(buf.len());
    lines.push(buf[..end].iter().collect());
    lines
}

/// Strip ANSI codes from a string, returning plain text.
/// Used for clipboard copy operations where \r semantics are not needed.
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' {
            i += 1;
            if i >= chars.len() { break; }
            match chars[i] {
                '[' => {
                    i += 1;
                    let mut found = false;
                    while i < chars.len() {
                        let c = chars[i];
                        i += 1;
                        if (c as u32) >= 0x40 && (c as u32) <= 0x7E { found = true; break; }
                    }
                    if !found { break; }
                }
                ']' => {
                    i += 1;
                    let mut found = false;
                    while i < chars.len() {
                        if chars[i] == '\x07' { i += 1; found = true; break; }
                        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i+1] == '\\' {
                            found = true; i += 2; break;
                        }
                        i += 1;
                    }
                    if !found { break; }
                }
                '(' | ')' | '*' | '+' | '-' | '.' | '/' => {
                    if i + 1 >= chars.len() { break; }
                    i += 2;
                }
                _ => { i += 1; }
            }
        } else if chars[i] == '\x08' {
            result.pop();
            i += 1;
        } else {
            let c = chars[i];
            if c == '\r' { i += 1; continue; }
            if (c as u32) >= 32 || c == '\n' || c == '\t' { result.push(c); }
            i += 1;
        }
    }
    result
}
