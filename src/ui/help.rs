use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};
use crate::config;

pub fn render_hint(f: &mut Frame, area: Rect) {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    let key = |s: &'static str| Span::styled(s, Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    let sep = || Span::styled("  ", Style::default().fg(Color::DarkGray));
    let lbl = |s: &'static str| Span::styled(s, Style::default().fg(Color::DarkGray));
    let div = || Span::styled(" │ ", Style::default().fg(Color::DarkGray));

    let line = Line::from(vec![
        key("^S"), lbl(" Save"),       div(),
        key("^Q"), lbl(" Quit"),       div(),
        key("^E"), lbl(" Editor"),     div(),
        key("^R"), lbl(" Sidebar"),    div(),
        key("^T"), lbl(" Terminal"),   div(),
        key("^B"), lbl(" Toggle Sidebar"), div(),
        key("^J"), lbl(" Toggle Terminal"), div(),
        key("^H"), lbl(" Help"),
        sep(),
    ]);

    let hint = ratatui::widgets::Paragraph::new(line)
        .style(Style::default().bg(Color::Reset))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(hint, area);
}

pub fn render(f: &mut Frame, area: Rect, scroll: usize) {
    let block = Block::default()
        .title(config::HELP_TITLE)
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Double)
        .border_style(ratatui::style::Style::default().fg(config::colors::HELP_BORDER))
        .style(ratatui::style::Style::default().bg(ratatui::style::Color::Reset));

    let area = crate::ui::layout::centered_rect(65, 75, area);
    f.render_widget(Clear, area);

    let help_paragraph = Paragraph::new(config::HELP_TEXT)
        .block(block)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::White))
        .scroll((scroll as u16, 0));

    f.render_widget(help_paragraph, area);
}
