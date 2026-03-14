use crate::app::{App, Panel};
use crate::config;
use crate::lsp::types::DiagnosticSeverity;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    f.render_widget(ratatui::widgets::Clear, area);
    let is_preview = matches!(app.active_panel, Panel::Sidebar) && app.preview.is_some();
    let editor = app.active_editor();

    // Use a consistent background to prevent ghosting
    let bg_color = if is_preview {
        ratatui::style::Color::Rgb(15, 15, 25) // Very dark blue for preview depth
    } else {
        ratatui::style::Color::Black
    };

    let title = if is_preview {
        format!(
            " [PREVIEW] {} ",
            editor
                .path
                .as_ref()
                .and_then(|p: &std::path::PathBuf| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "No file".to_string())
        )
    } else {
        format!(
            " {} - {} ",
            config::APP_TITLE,
            editor
                .path
                .as_ref()
                .and_then(|p: &std::path::PathBuf| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "No file".to_string())
        )
    };

    let border_color = if is_preview {
        ratatui::style::Color::DarkGray
    } else if matches!(app.active_panel, Panel::Editor) {
        config::colors::EDITOR_FOCUS
    } else {
        ratatui::style::Color::Indexed(240) // Subdued gray
    };

    let editor_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(border_color))
        .style(ratatui::style::Style::default().bg(bg_color));

    let inner_rect = editor_block.inner(area);
    f.render_widget(editor_block, area);

    // Diagnostics for this file
    let empty_vec = Vec::new();
    let file_diagnostics = editor.path.as_ref()
        .and_then(|p| app.lsp_state.diagnostics.get(p))
        .unwrap_or(&empty_vec);

    let gutter_width = editor.get_gutter_width() + 2; // icon + space + padding

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Length(gutter_width as u16),
            ratatui::layout::Constraint::Min(0),
        ])
        .split(inner_rect);

    let gutter_area = layout[0];
    let content_area = layout[1];

    // Only update editor_area for real (non-preview) editor interaction
    if !is_preview {
        app.editor_area.set(content_area);
        app.last_editor_height.set(content_area.height as usize);
    }

    // Render Gutter (Line Numbers)
    let start_line = editor.scroll_y;
    let num_lines = editor.buffer.len_lines();
    let mut gutter_lines = Vec::new();

    let end_line = (start_line + content_area.height as usize).min(num_lines);
    for i in start_line..end_line {
        // Find highest severity diagnostic for this line
        let line_diag = file_diagnostics.iter()
            .filter(|d| d.line == i)
            .max_by_key(|d| d.severity.clone());

        let icon_span = if let Some(d) = line_diag {
            let color = match d.severity {
                DiagnosticSeverity::Error => Color::Red,
                DiagnosticSeverity::Warning => Color::Yellow,
                DiagnosticSeverity::Info => Color::Blue,
                DiagnosticSeverity::Hint => Color::DarkGray,
            };
            Span::styled(format!("{} ", d.severity.icon()), Style::default().fg(color))
        } else {
            Span::from("  ")
        };

        let line_num_span = Span::styled(
            format!("{:>width$} ", i + 1, width = gutter_width - 3),
            Style::default().fg(Color::DarkGray)
        );

        gutter_lines.push(Line::from(vec![icon_span, line_num_span]));
    }

    // Pad gutter to full height
    while gutter_lines.len() < gutter_area.height as usize {
        gutter_lines.push(ratatui::text::Line::from(" ".repeat(gutter_width)));
    }

    let gutter_widget = Paragraph::new(gutter_lines).style(
        ratatui::style::Style::default()
            .fg(ratatui::style::Color::DarkGray)
            .bg(bg_color),
    );
    f.render_widget(gutter_widget, gutter_area);

    // Render Editor Content
    let highlighted_lines =
        editor.get_highlighted_lines(content_area.width as usize, content_area.height as usize);

    let editor_widget =
        Paragraph::new(highlighted_lines).style(ratatui::style::Style::default().bg(bg_color));
    f.render_widget(editor_widget, content_area);

    // Show cursor — only for the real editor, not preview
    if !is_preview && matches!(app.active_panel, Panel::Editor) && !app.show_quit_confirm {
        let real_editor = app.editor();
        if real_editor.cursor_y >= real_editor.scroll_y {
            let cursor_screen_y = real_editor.cursor_y - real_editor.scroll_y;
            if cursor_screen_y < content_area.height as usize {
                f.set_cursor(
                    content_area.x + real_editor.cursor_x as u16,
                    content_area.y + cursor_screen_y as u16,
                );
            }
        }
    }
}
