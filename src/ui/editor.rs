use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::{App, Panel};
use crate::config;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let editor_block = Block::default()
        .title(format!(
            " {} - {} ",
            config::APP_TITLE,
            app.editor
                .path
                .as_ref()
                .and_then(|p: &std::path::PathBuf| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "No file".to_string())
        ))
        .borders(Borders::ALL)
        .border_style(if matches!(app.active_panel, Panel::Editor) {
            ratatui::style::Style::default().fg(config::colors::EDITOR_FOCUS)
        } else {
            ratatui::style::Style::default()
        });

    let inner_rect = editor_block.inner(area);
    app.last_editor_height.set(inner_rect.height as usize);
    
    let highlighted_lines = app.editor.get_highlighted_lines(
        inner_rect.width as usize,
        inner_rect.height as usize,
    );

    let editor_widget = Paragraph::new(highlighted_lines).block(editor_block);
    f.render_widget(editor_widget, area);

    // Render search box
    if app.editor.is_searching {
        let search_area = Rect::new(
            inner_rect.x + 2,
            inner_rect.y + inner_rect.height.saturating_sub(2),
            inner_rect.width.saturating_sub(4),
            1,
        );
        let search_block = Block::default()
            .title(" Search ")
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(config::colors::SEARCH_BORDER));
        let search_text = format!("Find: {}", app.editor.search_query);
        f.render_widget(Paragraph::new(search_text).block(search_block), search_area);
    }

    // Show cursor in editor
    if matches!(app.active_panel, Panel::Editor) {
        f.set_cursor(
            inner_rect.x + app.editor.cursor_x as u16,
            inner_rect.y + (app.editor.cursor_y - app.editor.scroll_y) as u16,
        );
    }
}
