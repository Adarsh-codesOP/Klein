use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::{App, Panel};
use crate::config;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    match app.editor_mode {
        crate::app::EditorMode::Internal => render_internal(f, area, app),
        crate::app::EditorMode::Micro => render_micro(f, area, app),
    }
}

pub fn render_micro(f: &mut Frame, area: Rect, app: &App) {
    let editor_block = Block::default()
        .title(" Micro Editor ")
        .borders(Borders::ALL)
        .border_style(if matches!(app.active_panel, Panel::Editor) {
            ratatui::style::Style::default().fg(config::colors::EDITOR_FOCUS)
        } else {
            ratatui::style::Style::default()
        });
    
    let inner_area = editor_block.inner(area);
    app.last_micro_width.set(inner_area.width);
    app.last_micro_height.set(inner_area.height);
    f.render_widget(editor_block, area);

    if let Some(micro) = &app.micro_editor {
        let parser = micro.parser.lock().unwrap();
        let screen = parser.screen();
        
        for y in 0..inner_area.height {
            for x in 0..inner_area.width {
                if let Some(cell) = screen.cell(y, x) {
                    let fg = match cell.fgcolor() {
                        vt100::Color::Default => ratatui::style::Color::Reset,
                        vt100::Color::Idx(i) => ratatui::style::Color::Indexed(i),
                        vt100::Color::Rgb(r, g, b) => ratatui::style::Color::Rgb(r, g, b),
                    };
                    let bg = match cell.bgcolor() {
                        vt100::Color::Default => ratatui::style::Color::Reset,
                        vt100::Color::Idx(i) => ratatui::style::Color::Indexed(i),
                        vt100::Color::Rgb(r, g, b) => ratatui::style::Color::Rgb(r, g, b),
                    };
                    
                    let style = ratatui::style::Style::default().fg(fg).bg(bg);
                    let symbol = cell.contents();
                    if inner_area.x + (x as u16) < f.size().width && inner_area.y + (y as u16) < f.size().height {
                        f.buffer_mut().get_mut(inner_area.x + (x as u16), inner_area.y + (y as u16)).set_symbol(&symbol).set_style(style);
                    }
                }
            }
        }
        
        if matches!(app.active_panel, Panel::Editor) {
            let cursor = screen.cursor_position();
            if cursor.1 < inner_area.width && cursor.0 < inner_area.height {
                f.set_cursor(inner_area.x + cursor.1, inner_area.y + cursor.0);
            }
        }
    }
}

pub fn render_internal(f: &mut Frame, area: Rect, app: &App) {
    let editor_block = Block::default()
        .title(format!(
            " {} - {} ",
            config::APP_TITLE,
            app.editor
                .path
                .as_ref()
                .and_then(|p: &std::path::PathBuf| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "No file".to_string())
        ))
        .borders(Borders::ALL)
        .border_style(if matches!(app.active_panel, Panel::Editor) {
            ratatui::style::Style::default().fg(config::colors::EDITOR_FOCUS)
        } else {
            ratatui::style::Style::default()
        });

    let inner_rect = editor_block.inner(area);
    f.render_widget(editor_block, area);

    let gutter_width = app.editor.get_gutter_width();
    
    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Length(gutter_width as u16),
            ratatui::layout::Constraint::Min(0),
        ])
        .split(inner_rect);
    
    let gutter_area = layout[0];
    let content_area = layout[1];
    
    app.last_editor_height.set(content_area.height as usize);
    
    // Render Gutter (Line Numbers)
    let start_line = app.editor.scroll_y;
    let end_line = (start_line + content_area.height as usize).min(app.editor.buffer.len_lines());
    let mut line_numbers = String::new();
    for i in start_line..end_line {
        line_numbers.push_str(&format!("{:>width$} \n", i + 1, width = gutter_width - 1));
    }
    
    let gutter_widget = Paragraph::new(line_numbers)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
    f.render_widget(gutter_widget, gutter_area);

    // Render Editor Content
    let highlighted_lines = app.editor.get_highlighted_lines(
        content_area.width as usize,
        content_area.height as usize,
    );

    let editor_widget = Paragraph::new(highlighted_lines);
    f.render_widget(editor_widget, content_area);

    // Render search box
    if app.editor.is_searching {
        let search_area = Rect::new(
            content_area.x + 2,
            content_area.y + content_area.height.saturating_sub(2),
            content_area.width.saturating_sub(4),
            1,
        );
        let search_block = Block::default()
            .title(" Search ")
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(config::colors::SEARCH_BORDER));
        let search_text = format!("Find: {}", app.editor.search_query);
        f.render_widget(Paragraph::new(search_text).block(search_block), search_area);
    }

    // Show cursor in editor
    if matches!(app.active_panel, Panel::Editor) && !app.show_quit_confirm {
        f.set_cursor(
            content_area.x + app.editor.cursor_x as u16,
            content_area.y + (app.editor.cursor_y - app.editor.scroll_y) as u16,
        );
    }
}
