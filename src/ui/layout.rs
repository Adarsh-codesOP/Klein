use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn get_main_layout(area: Rect, show_terminal: bool) -> Vec<Rect> {
    let constraints = vec![
        Constraint::Length(1), // Help Hint Tab
        Constraint::Length(1), // Tab Bar
        Constraint::Fill(1),   // Main workspace
        if show_terminal {
            Constraint::Length(10)
        } else {
            Constraint::Length(0)
        }, // Terminal
        Constraint::Length(1), // Status Bar
    ];

    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

pub fn get_maximized_terminal_layout(area: Rect) -> Vec<Rect> {
    let constraints = vec![
        Constraint::Length(1), // Help Hint Tab
        Constraint::Length(0), // Tab Bar hidden
        Constraint::Length(0), // Main workspace hidden
        Constraint::Fill(1),   // Terminal fills
        Constraint::Length(1), // Status Bar
    ];

    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

pub fn get_editor_layout(area: Rect, show_sidebar: bool) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if show_sidebar {
            [Constraint::Length(30), Constraint::Min(0)]
        } else {
            [Constraint::Length(0), Constraint::Min(0)]
        })
        .split(area)
        .to_vec()
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
