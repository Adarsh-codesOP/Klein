use crate::app::{App, TopBarMenu};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let menus = vec![
        " Navigation ",
        " Edit ",
        " Files ",
        " Panels ",
        " Sidebar ",
        " Code ",
        " Help ",
    ];

    let selected_tab = match app.top_bar.active_menu {
        Some(TopBarMenu::Navigation) => 0,
        Some(TopBarMenu::Edit) => 1,
        Some(TopBarMenu::Files) => 2,
        Some(TopBarMenu::Panels) => 3,
        Some(TopBarMenu::Sidebar) => 4,
        Some(TopBarMenu::Code) => 5,
        Some(TopBarMenu::Help) => 6,
        None => 999, // Nothing selected
    };

    let tabs = Tabs::new(menus.clone())
        .select(if selected_tab == 999 { 0 } else { selected_tab })
        .style(Style::default().fg(Color::Gray))
        .highlight_style(if selected_tab == 999 {
            Style::default().fg(Color::Gray)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED)
        })
        .divider("│");

    f.render_widget(tabs, area);

    // If a menu is active, render the dropdown
    if let Some(active_menu) = app.top_bar.active_menu {
        // Calculate the starting x position of the selected tab approximately
        // Each tab name length + divider. Let's do a simple calculation.
        let mut x_offset = area.x;
        for i in 0..selected_tab {
            x_offset += menus[i].chars().count() as u16 + 1; // +1 for divider
        }

        render_dropdown(
            f,
            area.y + 1,
            x_offset,
            active_menu,
            app.top_bar.selected_index,
        );
    }
}

pub fn get_menu_items(menu: TopBarMenu) -> Vec<(&'static str, &'static str)> {
    match menu {
        TopBarMenu::Navigation => vec![
            ("Home / End", "Start / End of line"),
            ("Ctrl+Home / Ctrl+End", "Top / Bottom of file"),
            ("PgUp / PgDn", "Scroll page"),
            ("Ctrl+D / Ctrl+U", "Page down / up"),
            ("Shift+Arrows", "Extend selection"),
        ],
        TopBarMenu::Edit => vec![
            ("Delete", "Forward delete"),
            ("Ctrl+X", "Cut"),
            ("Ctrl+C", "Copy"),
            ("Ctrl+V", "Paste"),
            ("Ctrl+A", "Select all"),
            ("Ctrl+Z", "Undo"),
        ],
        TopBarMenu::Files => vec![
            ("Ctrl+P", "Find file (fzf)"),
            ("Ctrl+G", "Project search (rg)"),
            ("Ctrl+S", "Save file"),
            ("Ctrl+W", "Close file"),
            ("Ctrl+Shift+Z", "Next tab"),
            ("Ctrl+Shift+X", "Close tab"),
        ],
        TopBarMenu::Panels => vec![
            ("Ctrl+F", "Focus sidebar"),
            ("Ctrl+E", "Focus editor"),
            ("Ctrl+T", "Focus terminal"),
            ("Ctrl+B", "Toggle sidebar"),
            ("Ctrl+J", "Toggle terminal"),
            ("Esc", "Restore layout"),
        ],
        TopBarMenu::Sidebar => vec![
            (".", "Toggle hidden files"),
            ("Enter", "Open file / toggle folder"),
            ("Home", "Jump to top"),
            ("End", "Jump to bottom"),
            ("Ctrl+D", "Page down"),
            ("Ctrl+U", "Page up"),
        ],
        TopBarMenu::Code => vec![
            ("Ctrl+Space", "Autocomplete"),
            ("Alt+G d", "Go to definition"),
            ("Alt+G r", "References"),
            ("Alt+G n", "Rename symbol"),
            ("Alt+F", "Format document"),
            ("Alt+Enter", "Code actions"),
        ],
        TopBarMenu::Help => vec![("Ctrl+H", "Toggle help overlay"), ("Esc", "Close help")],
    }
}

fn render_dropdown(f: &mut Frame, y: u16, x: u16, menu: TopBarMenu, selected_index: usize) {
    let items = get_menu_items(menu);

    let max_shortcut_len = items
        .iter()
        .map(|(s, _)| s.chars().count())
        .max()
        .unwrap_or(0);
    let max_desc_len = items
        .iter()
        .map(|(_, d)| d.chars().count())
        .max()
        .unwrap_or(0);
    let width = (max_shortcut_len + max_desc_len + 6) as u16;
    let height = (items.len() + 2) as u16; // +2 for borders

    // Ensure it doesn't go off-screen
    let max_x = f.size().width.saturating_sub(width);
    let render_x = x.min(max_x);

    let dropdown_area = Rect {
        x: render_x,
        y,
        width,
        height,
    };

    f.render_widget(Clear, dropdown_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Black));

    let mut lines = Vec::new();
    for (i, (shortcut, desc)) in items.iter().enumerate() {
        let style = if i == selected_index {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };

        // Pad shortcut
        let padded_shortcut = format!("{:<1$}", shortcut, max_shortcut_len);

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}  ", padded_shortcut),
                style.add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{}  ", desc), style),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, dropdown_area);
}
