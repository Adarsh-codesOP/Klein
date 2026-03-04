use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::config;

pub fn render(f: &mut Frame, area: Rect) {
    let help_block = Block::default()
        .title(config::HELP_TITLE)
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(config::colors::HELP_BORDER));
    
    let help_widget = Paragraph::new(config::HELP_TEXT)
        .block(help_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(help_widget, area);
}
