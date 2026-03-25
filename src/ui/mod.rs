use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use crate::app::{App, Maximized};

pub mod sidebar;
pub mod editor;
pub mod terminal;
pub mod status_bar;
pub mod help;
pub mod layout;
pub mod tabs;

pub fn render(f: &mut Frame, app: &App) {
    match app.maximized {
        Maximized::Editor => {
            let chunks = layout::get_main_layout(f.size(), false);
            help::render_hint(f, chunks[0]);
            tabs::render(f, chunks[1], app);
            let editor_area = layout::get_editor_layout(chunks[2], false);
            editor::render(f, editor_area[1], app);
            status_bar::render(f, chunks[4], app);
        }
        Maximized::Terminal => {
            let chunks = layout::get_maximized_terminal_layout(f.size());
            help::render_hint(f, chunks[0]);
            tabs::render(f, chunks[1], app);
            terminal::render(f, chunks[3], app);
            status_bar::render(f, chunks[4], app);
        }
        Maximized::None => {
            render_normal(f, app);
        }
    }

    // Overlays always draw on top regardless of maximized state
    render_overlays(f, app);
}

fn render_normal(f: &mut Frame, app: &App) {
    let chunks = layout::get_main_layout(f.size(), app.show_terminal);
    // chunks[0] = help hint
    // chunks[1] = tab bar
    // chunks[2] = main workspace
    // chunks[3] = terminal
    // chunks[4] = status bar

    // Render the subtle help hint tab at the very top
    help::render_hint(f, chunks[0]);

    // Render tab bar
    tabs::render(f, chunks[1], app);

    let main_chunks = layout::get_editor_layout(chunks[2], app.show_sidebar);

    if app.show_sidebar {
        sidebar::render(f, main_chunks[0], app);
    }

    editor::render(f, main_chunks[1], app);

    if app.show_terminal {
        terminal::render(f, chunks[3], app);
    }

    status_bar::render(f, chunks[4], app);
}

fn render_overlays(f: &mut Frame, app: &App) {

    // "File doesn't exist — create it?" prompt
    if let Some(path) = &app.create_file_prompt {
        let filename = path.display().to_string();
        let msg = format!("\"{}\" does not exist.\nCreate it? (y/n)", filename);
        let area = layout::centered_rect(70, 25, f.size());
        f.render_widget(ratatui::widgets::Clear, area);
        let block = ratatui::widgets::Block::default()
            .title(" File Not Found ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow));
        let paragraph = ratatui::widgets::Paragraph::new(msg)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, area);
    }

    if app.show_help {
        help::render(f, f.size(), app.help_scroll);
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

    // Close tab with unsaved changes dialog
    if app.show_close_confirm {
        let area = layout::centered_rect(44, 20, f.size());
        f.render_widget(ratatui::widgets::Clear, area);
        let block = ratatui::widgets::Block::default()
            .title(" Close File ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Reset));
        let paragraph = ratatui::widgets::Paragraph::new("File has unsaved changes!\nSave (y), Discard (n), Cancel (c)")
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
        let paragraph = ratatui::widgets::Paragraph::new("File has unsaved changes!\nSave (y), Discard (n), Cancel (c)")
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, area);
    }

    // Save As dialog (triggered for untitled files)
    if let Some(sa) = &app.save_as {
        let area = layout::centered_rect(72, 40, f.size());
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" Save File As ")
            .title_alignment(ratatui::layout::Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // padding
                Constraint::Length(1), // folder row
                Constraint::Length(1), // padding
                Constraint::Length(1), // filename row
                Constraint::Length(1), // padding
                Constraint::Length(1), // hint
            ])
            .split(inner);

        let active_style   = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let inactive_style = Style::default().fg(Color::White);
        let label_style    = Style::default().fg(Color::Gray);

        let folder_display = if sa.active_field == 0 {
            format!("{}█", sa.folder)
        } else {
            sa.folder.clone()
        };
        let filename_display = if sa.active_field == 1 {
            format!("{}█", sa.filename)
        } else {
            sa.filename.clone()
        };

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" Folder   : ", label_style),
                Span::styled(folder_display, if sa.active_field == 0 { active_style } else { inactive_style }),
            ])),
            rows[1],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" Filename : ", label_style),
                Span::styled(filename_display, if sa.active_field == 1 { active_style } else { inactive_style }),
            ])),
            rows[3],
        );

        f.render_widget(
            Paragraph::new(Span::styled(
                " [Tab] Switch Field   [Enter] Save   [Esc] Cancel",
                Style::default().fg(Color::DarkGray),
            )),
            rows[5],
        );
    }
}
