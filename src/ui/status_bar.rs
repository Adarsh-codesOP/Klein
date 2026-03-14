use crate::app::{App, Panel};
use crate::config;
use crate::lsp::types::DiagnosticSeverity;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let status_bar = Block::default()
        .borders(Borders::TOP)
        .border_style(ratatui::style::Style::default().fg(config::colors::STATUS_BG));

    let empty_vec = Vec::new();
    let file_diagnostics = app.editor().path.as_ref()
        .and_then(|p| app.lsp_state.diagnostics.get(p))
        .unwrap_or(&empty_vec);

    let errors = file_diagnostics.iter().filter(|d| matches!(d.severity, DiagnosticSeverity::Error)).count();
    let warnings = file_diagnostics.iter().filter(|d| matches!(d.severity, DiagnosticSeverity::Warning)).count();

    let mut spans = vec![
        Span::from(format!(" {} ", if let Some(path) = &app.editor().path {
            path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "".to_string())
        } else {
            "No file".to_string()
        })),
        Span::from(" | "),
        Span::from(format!(" {} ", if matches!(app.active_panel, Panel::Editor) {
            "EDIT"
        } else if matches!(app.active_panel, Panel::Sidebar) {
            "EXPLORE"
        } else {
            "TERM"
        })),
        Span::from(" | "),
        Span::from(format!("Ln {}, Col {} ", app.editor().cursor_y + 1, app.editor().cursor_x + 1)),
    ];

    if errors > 0 {
        spans.push(Span::from(" | "));
        spans.push(Span::styled(format!("● {} ", errors), Style::default().fg(Color::Red)));
    }
    if warnings > 0 {
        spans.push(Span::from(" | "));
        spans.push(Span::styled(format!("▲ {} ", warnings), Style::default().fg(Color::Yellow)));
    }

    let status_paragraph = Paragraph::new(Line::from(spans))
        .block(status_bar)
        .style(ratatui::style::Style::default().fg(config::colors::STATUS_FG).bg(config::colors::STATUS_BG));

    f.render_widget(status_paragraph, area);
}
