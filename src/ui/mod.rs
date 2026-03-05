use ratatui::Frame;
use crate::app::App;

pub mod sidebar;
pub mod editor;
pub mod terminal;
pub mod status_bar;
pub mod help;
pub mod layout;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = layout::get_main_layout(f.size(), app.show_terminal);
    
    // Render the subtle help hint tab at the very top
    help::render_hint(f, chunks[0]);

    let main_chunks = layout::get_editor_layout(chunks[1], app.show_sidebar);

    if app.show_sidebar {
        sidebar::render(f, main_chunks[0], app);
    }

    editor::render(f, main_chunks[1], app);

    if app.show_terminal {
        terminal::render(f, chunks[2], app);
    }

    status_bar::render(f, chunks[3], app);

    if app.show_help {
        help::render(f, f.size());
    }

    if app.show_quit_confirm {
        let area = layout::centered_rect(40, 20, f.size());
        f.render_widget(ratatui::widgets::Clear, area);
        let block = ratatui::widgets::Block::default()
            .title(" Quit ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Red))
            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Reset));
        let text = "Unsaved changes! Save? (y/n/c)";
        let paragraph = ratatui::widgets::Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, area);
    }
}
