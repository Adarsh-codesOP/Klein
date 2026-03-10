use crate::config;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_hint(f: &mut Frame, area: Rect) {
    let hint_text = " ^S Save │ ^Q Quit │ ^E Editor │ ^F Sidebar │ ^T Terminal │ ^B Toggle Sidebar │ ^J Toggle Terminal │ ^H Help ";
    let hint = Paragraph::new(hint_text)
        .style(
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD)
                .bg(ratatui::style::Color::Black),
        ) // Black background for top hint bar
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(hint, area);
}

pub fn render(f: &mut Frame, area: Rect, app: &crate::app::App) {
    let block = Block::default()
        .title(config::HELP_TITLE)
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Double) // Distinct double border
        .border_style(ratatui::style::Style::default().fg(config::colors::HELP_BORDER))
        .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));

    let area = crate::ui::layout::centered_rect(65, 75, area);
    f.render_widget(Clear, area); // Clear the area before rendering the popup

    let help_text = config::HELP_TEXT;
    let help_paragraph = Paragraph::new(help_text)
        .block(block)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::White))
        .scroll((app.help_scroll as u16, 0));

    f.render_widget(help_paragraph, area);
}
