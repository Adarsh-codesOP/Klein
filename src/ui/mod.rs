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
    f.render_widget(ratatui::widgets::Clear, f.size());

    let show_terminal_layout = if app.maximized == crate::app::Maximized::Editor {
        false
    } else {
        app.show_terminal
    };
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

        let show_sidebar = if app.maximized == crate::app::Maximized::Editor {
            false
        } else {
            app.show_sidebar
        };
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

    // Save As Dialog
    if app.save_as_state.active {
        let area = layout::centered_rect(60, 25, f.size());
        f.render_widget(ratatui::widgets::Clear, area);

        let block = ratatui::widgets::Block::default()
            .title(" Save As ")
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
            .style(ratatui::style::Style::default().bg(ratatui::style::Color::Black));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Min(0), // Spacer
            ])
            .split(inner_area);

        let dir_str = format!("Dir:  {}", app.save_as_state.cur_dir.display());
        let dir_style = if !app.save_as_state.focus_filename {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::White)
        } else {
            ratatui::style::Style::default()
        };
        f.render_widget(
            ratatui::widgets::Paragraph::new(dir_str).style(dir_style),
            chunks[0],
        );

        let file_str_base = if app.save_as_state.filename.is_empty() {
            "".to_string()
        } else {
            app.save_as_state.filename.clone()
        };

        let file_display = if app.save_as_state.focus_filename {
            format!("File: {}_", file_str_base) // Visual cursor
        } else {
            format!("File: {}", file_str_base)
        };

        let file_style = if app.save_as_state.focus_filename {
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::White)
        } else {
            ratatui::style::Style::default()
        };

        // For long filenames, we want to show the end of the string (where typing happens)
        let inner_width = chunks[1].width as usize;
        let file_para_content = if file_display.chars().count() > inner_width && inner_width > 1 {
            let skip_count = file_display.chars().count() - (inner_width - 1);
            format!(
                "…{}",
                file_display.chars().skip(skip_count).collect::<String>()
            )
        } else {
            file_display
        };

        f.render_widget(
            ratatui::widgets::Paragraph::new(file_para_content).style(file_style),
            chunks[1],
        );

        f.render_widget(
            ratatui::widgets::Paragraph::new(
                "Tab/Up/Down switches field | Enter saves | Esc cancels",
            )
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center),
            chunks[2],
        );
    }

    // Create File Prompt Dialog
    if app.show_create_file_prompt {
        if let Some(path) = &app.pending_open_path {
            let area = layout::centered_rect(50, 10, f.size());
            f.render_widget(ratatui::widgets::Clear, area);
            let block = ratatui::widgets::Block::default()
                .title(" File Not Found ")
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
                .style(ratatui::style::Style::default().bg(ratatui::style::Color::Reset));
            let text = format!(
                "File does not exist:\n{}\n\nCreate it? (y/n)",
                path.display()
            );
            let paragraph = ratatui::widgets::Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(paragraph, area);
        }
    }
}
