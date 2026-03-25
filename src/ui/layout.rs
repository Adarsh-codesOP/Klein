use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn get_main_layout(area: Rect, show_terminal: bool) -> Vec<Rect> {
    let constraints = vec![
        Constraint::Length(1), // Help Hint Tab
        Constraint::Length(1), // Tab Bar
        Constraint::Fill(1),   // Main workspace
        if show_terminal { Constraint::Length(10) } else { Constraint::Length(0) }, // Terminal
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
        .constraints(
            if show_sidebar {
                [Constraint::Percentage(20), Constraint::Percentage(80)]
            } else {
                [Constraint::Percentage(0), Constraint::Percentage(100)]
            }
        )
        .split(area)
        .to_vec()
}

/// Layout used when the terminal panel is maximized (fills all available height).
/// Slot indices match get_main_layout: [0]=hint [1]=tabs [2]=workspace(0) [3]=terminal [4]=status.
pub fn get_maximized_terminal_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // help hint
            Constraint::Length(1), // tab bar
            Constraint::Length(0), // workspace (hidden)
            Constraint::Fill(1),   // terminal fills remaining space
            Constraint::Length(1), // status bar
        ])
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
