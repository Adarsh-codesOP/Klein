use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::{App, Panel};
use crate::config;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let status_bar = Block::default()
        .borders(Borders::TOP)
        .border_style(ratatui::style::Style::default().fg(config::colors::STATUS_BG));
    
    let status_text = format!(
        " {} | {} | Ln {}, Col {} | Ctrl+H: Help | Ctrl+Arrows: Panel ",
        if let Some(path) = &app.editor.path {
            path.file_name().unwrap_or_default().to_string_lossy().into_owned()
        } else {
            "No file".to_string()
        },
        if matches!(app.active_panel, Panel::Editor) { "Mode: EDIT" } 
        else if matches!(app.active_panel, Panel::Sidebar) { "Mode: EXPLORE" }
        else { "Mode: TERM" },
        app.editor.cursor_y + 1,
        app.editor.cursor_x + 1,
    );
    
    let status_paragraph = Paragraph::new(status_text)
        .block(status_bar)
        .style(ratatui::style::Style::default().fg(config::colors::STATUS_FG));
    
    f.render_widget(status_paragraph, area);
}
