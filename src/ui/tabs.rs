use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::app::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let tabs: Vec<Span> = app.tabs.iter().enumerate().map(|(i, tab)| {
        let name = tab.editor.path.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        let label = if tab.editor.is_dirty {
            format!(" ● {} ", name)
        } else {
            format!(" {} ", name)
        };

        if i == app.active_tab {
            Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                label,
                Style::default().fg(Color::DarkGray),
            )
        }
    }).collect();

    let line = Line::from(tabs);
    let widget = Paragraph::new(line)
        .style(Style::default().bg(Color::Reset));
    f.render_widget(widget, area);
}
