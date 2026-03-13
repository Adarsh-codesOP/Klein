use crate::app::App;
use crate::search::SearchMode;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    if !app.picker.active {
        return;
    }

    let area = crate::ui::layout::centered_rect(80, 80, f.size());
    f.render_widget(Clear, area);

    let title_color = match app.picker.mode {
        SearchMode::File => Color::Cyan,
        SearchMode::Grep => Color::Magenta,
    };

    let block = Block::default()
        .title(match app.picker.mode {
            SearchMode::File => " 📂 Find File ",
            SearchMode::Grep => " 🔍 Project Search ",
        })
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(title_color))
        .style(Style::default().bg(Color::Rgb(20, 20, 25)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Input prompt
            Constraint::Length(1), // Info line
            Constraint::Fill(1),   // List
        ])
        .split(inner);

    // Prompt with query
    let prompt_spans = vec![
        Span::styled("  ❯ ", Style::default().fg(title_color).add_modifier(Modifier::BOLD)),
        Span::styled(&app.picker.query, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(title_color).add_modifier(Modifier::SLOW_BLINK)),
    ];
    f.render_widget(Paragraph::new(Line::from(prompt_spans)), chunks[0]);

    // Info line
    let info = format!("  {} results   ↑/↓ navigate   Enter select   Esc cancel", app.picker.results.len());
    let info_para = Paragraph::new(info).style(Style::default().fg(Color::DarkGray));
    f.render_widget(info_para, chunks[1]);

    // List results and Preview
    let visible_height = chunks[2].height as usize;
    
    let main_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // List
            Constraint::Percentage(60), // Preview
        ])
        .split(chunks[2]);

    // Draw a vertical separator
    let separator_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::Rgb(40, 40, 50)));
    f.render_widget(separator_block, main_split[1]);

    let items: Vec<ListItem> = app
        .picker
        .results
        .iter()
        .enumerate()
        .skip(app.picker.scroll)
        .take(visible_height)
        .map(|(i, res)| {
            let is_selected = i == app.picker.selected_index;
            let mut line_spans = Vec::new();

            // Prefix space
            line_spans.push(Span::raw("  "));

            // Icon
            let icon = if app.picker.mode == SearchMode::File { " " } else { " " };
            line_spans.push(Span::styled(icon, Style::default().fg(title_color)));

            // Filename
            let file_name = res.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            line_spans.push(Span::styled(
                file_name.to_string(),
                Style::default().fg(if is_selected { Color::White } else { Color::Gray }).add_modifier(Modifier::BOLD)
            ));

            // Grep location (small)
            if let Some(l) = res.line {
                line_spans.push(Span::styled(
                    format!(" :{}", l + 1),
                    Style::default().fg(Color::Yellow)
                ));
            }

            let style = if is_selected {
                Style::default().bg(Color::Rgb(50, 50, 70))
            } else {
                Style::default()
            };

            ListItem::new(Line::from(line_spans)).style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, main_split[0]);

    // Render Preview Panel
    if let Some(preview_lines) = &app.picker.preview {
        let preview_inner = Rect {
            x: main_split[1].x + 2,
            y: main_split[1].y,
            width: main_split[1].width.saturating_sub(2),
            height: main_split[1].height,
        };

        let mut spans = Vec::new();
        for (i, line) in preview_lines.iter().enumerate() {
            let style = if line.starts_with('>') {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            spans.push(Line::from(Span::styled(line.clone(), style)));
        }
        
        let preview_para = Paragraph::new(spans)
            .block(Block::default()
                .title(" Preview ")
                .title_style(Style::default().fg(Color::DarkGray)));
        f.render_widget(preview_para, preview_inner);
    } else {
        let no_preview = Paragraph::new("\n\n   No preview available")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(no_preview, main_split[1]);
    }
}

use ratatui::layout::Rect;
