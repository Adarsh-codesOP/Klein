use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn get_main_layout(area: Rect, show_terminal: bool) -> Vec<Rect> {
    let constraints = vec![
        Constraint::Length(1), // Help Hint Tab
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
