use crate::app::{App, Panel};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    f.render_widget(ratatui::widgets::Clear, area);
    let mut list_items = Vec::new();
    let height = area.height.saturating_sub(2) as usize;
    let offset = app.sidebar.offset;
    let visible_slice = app
        .sidebar
        .flat_list
        .iter()
        .enumerate()
        .skip(offset)
        .take(height);

    for (i, (path, depth, is_dir)) in visible_slice {
        let prefix = "  ".repeat(*depth);
        let icon = if *is_dir { "📁 " } else { "📄 " };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        let mut style = ratatui::style::Style::default();
        if i == app.sidebar.selected_index {
            if matches!(app.active_panel, Panel::Sidebar) {
                style = style
                    .bg(app.theme.sidebar.selected)
                    .fg(app.theme.sidebar.text);
            } else {
                style = style
                    .bg(app.theme.sidebar.background)
                    .fg(app.theme.sidebar.text)
                    .add_modifier(ratatui::style::Modifier::DIM);
            }
        } else {
            style = style
                .bg(app.theme.sidebar.background)
                .fg(app.theme.sidebar.text);
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
            ratatui::style::Style::default().fg(app.theme.sidebar.selected)
        } else {
            ratatui::style::Style::default().fg(app.theme.sidebar.text)
        })
        .style(ratatui::style::Style::default().bg(app.theme.sidebar.background));

    app.sidebar.last_height.set(area.height as usize);
    let sidebar_widget = Paragraph::new(list_items).block(sidebar_block);
    f.render_widget(sidebar_widget, area);
}
