use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let state = match &app.lsp_state.completion {
        Some(s) => s,
        None => return,
    };

    if state.items.is_empty() {
        return;
    }

    let editor_area = app.editor_area.get();
    let editor = app.editor();

    // Calculate cursor screen position
    let cursor_screen_y = editor.cursor_y.saturating_sub(editor.scroll_y);
    let cursor_screen_x = editor.cursor_x;

    // The popup should appear below the cursor
    let mut x = editor_area.x + cursor_screen_x as u16;
    let mut y = editor_area.y + cursor_screen_y as u16 + 1;

    let width = 40.min(f.size().width.saturating_sub(x));
    let height = 10.min(f.size().height.saturating_sub(y));

    // If too close to bottom, show above cursor
    if y + height > f.size().height {
        y = (editor_area.y + cursor_screen_y as u16).saturating_sub(height);
    }

    // If too close to right, shift left
    if x + width > f.size().width {
        x = f.size().width.saturating_sub(width);
    }

    let area = Rect::new(x, y, width, height);
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Suggestions ");

    let items: Vec<ListItem> = state.items.iter().enumerate().map(|(i, item)| {
        let style = if i == state.selected_index {
            Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let content = Line::from(vec![
            Span::styled(format!(" {} ", item.kind.icon()), Style::default().fg(Color::Yellow)),
            Span::styled(item.label.clone(), style),
        ]);
        ListItem::new(content)
    }).collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
        .highlight_symbol(">");

    // Manual scroll management because ListState is tricky with manual render calls
    f.render_stateful_widget(list, area, &mut list_state);

    // Optionally render documentation next to it if we have space
    if let Some(item) = state.items.get(state.selected_index) {
        if let Some(doc) = &item.documentation {
            let doc_width = 40.min(f.size().width.saturating_sub(x + width));
            if doc_width > 10 {
                let doc_area = Rect::new(x + width, y, doc_width, height);
                f.render_widget(ratatui::widgets::Clear, doc_area);
                let doc_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Info ");
                let doc_para = Paragraph::new(doc.as_str())
                    .block(doc_block)
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(doc_para, doc_area);
            }
        }
    }
}
