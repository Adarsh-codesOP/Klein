use crate::app::App;
use ratatui::Frame;

pub mod editor;
pub mod help;
pub mod layout;
pub mod sidebar;
pub mod status_bar;
pub mod tabs;
pub mod terminal;

pub fn render(f: &mut Frame, app: &App) {
    let show_terminal_layout = if app.maximized == crate::app::Maximized::Editor { false } else { app.show_terminal };
    let chunks = if app.maximized == crate::app::Maximized::Terminal {
        layout::get_maximized_terminal_layout(f.size())
    } else {
        layout::get_main_layout(f.size(), show_terminal_layout)
    };

    // Render the subtle help hint tab at the very top
    help::render_hint(f, chunks[0]);

    if app.maximized != crate::app::Maximized::Terminal {
        // Render tab bar
        tabs::render(f, chunks[1], app);

        let show_sidebar = if app.maximized == crate::app::Maximized::Editor { false } else { app.show_sidebar };
        let main_chunks = layout::get_editor_layout(chunks[2], show_sidebar);

        if show_sidebar {
            sidebar::render(f, main_chunks[0], app);
        }

        editor::render(f, main_chunks[1], app);
    }

    if app.maximized == crate::app::Maximized::Terminal || show_terminal_layout {
        terminal::render(f, chunks[3], app);
    }

    status_bar::render(f, chunks[4], app);

    if app.show_help {
        help::render(f, f.size(), app);
    }

    // Quit confirm dialog
    if app.show_quit_confirm {
        let area = layout::centered_rect(40, 20, f.size());
        f.render_widget(ratatui::widgets::Clear, area);
        let block = ratatui::widgets::Block::default()
            .title(" Quit ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Red))
            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Reset));
        let paragraph = ratatui::widgets::Paragraph::new("Unsaved changes! Save? (y/n/c)")
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, area);
    }

    // Unsaved changes on file switch dialog
    if app.show_unsaved_confirm {
        let area = layout::centered_rect(44, 20, f.size());
        f.render_widget(ratatui::widgets::Clear, area);
        let block = ratatui::widgets::Block::default()
            .title(" Unsaved Changes ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Reset));
        let paragraph = ratatui::widgets::Paragraph::new(
            "File has unsaved changes!\nSave (y), Discard (n), Cancel (c)",
        )
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, area);
    }
}
