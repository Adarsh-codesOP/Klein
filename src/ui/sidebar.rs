use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::{App, Panel};
use crate::config;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let mut list_items = Vec::new();
    for (i, (path, depth, is_dir)) in app.sidebar.flat_list.iter().enumerate() {
        let prefix = "  ".repeat(*depth);
        let icon = if *is_dir { "📁 " } else { "📄 " };
        let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
        let mut style = ratatui::style::Style::default();
        if i == app.sidebar.selected_index {
            style = style.bg(ratatui::style::Color::DarkGray).fg(ratatui::style::Color::White);
        }
        list_items.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(prefix, style),
            ratatui::text::Span::styled(icon, style),
            ratatui::text::Span::styled(name, style),
        ]));
    }

    let sidebar_block = Block::default()
        .title(" File Explorer ")
        .borders(Borders::ALL)
        .border_style(if matches!(app.active_panel, Panel::Sidebar) {
            ratatui::style::Style::default().fg(config::colors::FOCUS_BORDER)
        } else {
            ratatui::style::Style::default()
        });
    
    let sidebar_widget = Paragraph::new(list_items).block(sidebar_block);
    f.render_widget(sidebar_widget, area);
}
