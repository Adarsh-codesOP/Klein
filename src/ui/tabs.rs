use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let tabs: Vec<Span> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let name = tab
                .editor
                .path
                .as_ref()
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
                        .fg(app.theme.tabs.text)
                        .bg(app.theme.tabs.active_bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(label, Style::default().fg(app.theme.tabs.text).bg(app.theme.tabs.inactive_bg))
            }
        })
        .collect();

    let line = Line::from(tabs);
    let widget = Paragraph::new(line).style(Style::default().bg(app.theme.tabs.inactive_bg));
    f.render_widget(widget, area);
}
