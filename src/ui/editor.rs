use crate::app::{App, Panel};
use crate::config;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let is_preview = matches!(app.active_panel, Panel::Sidebar) && app.preview.is_some();
    let editor = app.active_editor();

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
        ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)
    } else if matches!(app.active_panel, Panel::Editor) {
        ratatui::style::Style::default().fg(config::colors::EDITOR_FOCUS)
    } else {
        ratatui::style::Style::default()
    };

    let editor_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_color);

    let inner_rect = editor_block.inner(area);
    f.render_widget(editor_block, area);

    let gutter_width = editor.get_gutter_width();

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
    let end_line = (start_line + content_area.height as usize).min(editor.buffer.len_lines());
    let mut line_numbers = String::new();
    for i in start_line..end_line {
        line_numbers.push_str(&format!("{:>width$} \n", i + 1, width = gutter_width - 1));
    }

    let gutter_widget = Paragraph::new(line_numbers)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
    f.render_widget(gutter_widget, gutter_area);

    // Render Editor Content
    let highlighted_lines =
        editor.get_highlighted_lines(content_area.width as usize, content_area.height as usize);

    let editor_widget = Paragraph::new(highlighted_lines);
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
