use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let state = match &app.lsp_state.rename {
        Some(s) if s.active => s,
        _ => return,
    };

    let size = f.size();
    let width = 40.min(size.width);
    let height = 3;
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;

    let area = Rect::new(x, y, width, height);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Rename Symbol ");

    let para = Paragraph::new(format!(" New name: {}", state.new_name)).block(block);

    f.render_widget(para, area);

    // Set cursor for the rename input
    f.set_cursor(x + 11 + state.new_name.len() as u16, y + 1);
}
