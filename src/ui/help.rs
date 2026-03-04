use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};
use crate::config;

pub fn render_hint(f: &mut Frame, area: Rect) {
    let hint_text = " [Ctrl+H] Help Overlay ";
    let hint = Paragraph::new(hint_text)
        .style(ratatui::style::Style::default()
            .fg(ratatui::style::Color::DarkGray)
            .bg(ratatui::style::Color::Reset)) // Very subtle gray on black
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(hint, area);
}

pub fn render(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(config::HELP_TITLE)
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Double) // Distinct double border
        .border_style(ratatui::style::Style::default().fg(config::colors::HELP_BORDER))
        .style(ratatui::style::Style::default().bg(ratatui::style::Color::Reset));

    let area = centered_rect(65, 75, area);
    f.render_widget(Clear, area); // Clear the area before rendering the popup
    
    let help_text = config::HELP_TEXT;
    let help_paragraph = Paragraph::new(help_text)
        .block(block)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::White));
        
    f.render_widget(help_paragraph, area);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
