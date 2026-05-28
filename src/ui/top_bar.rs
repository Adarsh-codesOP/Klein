use crate::app::{App, TopBarMenu};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let menu_labels = vec![
        " Navigation ",
        " Edit ",
        " Files ",
        " Panels ",
        " Sidebar ",
        " Code ",
        " Help ",
        " Theme ",
    ];

    let selected_tab = match app.top_bar.active_menu {
        Some(TopBarMenu::Navigation) => 0,
        Some(TopBarMenu::Edit) => 1,
        Some(TopBarMenu::Files) => 2,
        Some(TopBarMenu::Panels) => 3,
        Some(TopBarMenu::Sidebar) => 4,
        Some(TopBarMenu::Code) => 5,
        Some(TopBarMenu::Help) => 6,
        Some(TopBarMenu::Theme) => 7,
        None => 999, // Nothing selected
    };

    // Build styled menu labels with underlined first letter (the Alt shortcut key)
    let menus: Vec<Line> = menu_labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let trimmed = label.trim_start();
            let leading_spaces = label.len() - trimmed.len();
            let prefix = &label[..leading_spaces];
            let first_char = &trimmed[..1];
            let rest = &trimmed[1..];

            let is_selected = i == selected_tab;
            let base_style = if is_selected {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(app.theme.top_bar.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.top_bar.text)
            };

            Line::from(vec![
                Span::styled(prefix, base_style),
                Span::styled(first_char, base_style.add_modifier(Modifier::UNDERLINED)),
                Span::styled(rest, base_style),
            ])
        })
        .collect();

    // We style each tab manually in the Line construction above,
    // so set highlight_style same as base to avoid Tabs widget double-highlighting.
    let tabs = Tabs::new(menus)
        .select(selected_tab)
        .style(Style::default().fg(app.theme.top_bar.text))
        .highlight_style(
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(app.theme.top_bar.text)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│");

    f.render_widget(tabs, area);

    // Store menu positions for mouse click detection
    // Tabs widget layout: padding_left(1) + tab + padding_right(1) + divider(1) + padding_left(1) + tab + ...
    {
        let mut positions = Vec::new();
        let mut x = area.x;
        for (i, label) in menu_labels.iter().enumerate() {
            if i == 0 {
                x += 1; // initial left padding
            }
            let w = label.chars().count() as u16;
            positions.push((x, x + w));
            x += w + 3; // right_padding(1) + divider(1) + left_padding(1)
        }
        app.top_bar_positions.set(positions);
        app.top_bar_area.set(area);
    }

    // If a menu is active, render the dropdown; otherwise clear dropdown area
    if app.top_bar.active_menu.is_none() {
        app.dropdown_area.set(None);
    }
    if let Some(active_menu) = app.top_bar.active_menu {
        // Calculate the starting x position of the selected tab approximately
        let mut x_offset = area.x;
        for label in menu_labels.iter().take(selected_tab) {
            x_offset += label.chars().count() as u16 + 1; // +1 for divider
        }

        render_dropdown(
            f,
            area.y + 1,
            x_offset,
            active_menu,
            app.top_bar.selected_index,
            app,
        );
    }
}

pub fn get_menu_items(menu: TopBarMenu, app: &App) -> Vec<(String, String)> {
    match menu {
        TopBarMenu::Navigation => vec![
            ("Home / End".to_string(), "Start / End of line".to_string()),
            (
                "Ctrl+Home / Ctrl+End".to_string(),
                "Top / Bottom of file".to_string(),
            ),
            ("PgUp / PgDn".to_string(), "Scroll page".to_string()),
            ("Ctrl+D / Ctrl+U".to_string(), "Page down / up".to_string()),
            ("Shift+Arrows".to_string(), "Extend selection".to_string()),
        ],
        TopBarMenu::Edit => vec![
            ("Delete".to_string(), "Forward delete".to_string()),
            ("Ctrl+X".to_string(), "Cut".to_string()),
            ("Ctrl+C".to_string(), "Copy".to_string()),
            ("Ctrl+V".to_string(), "Paste".to_string()),
            ("Ctrl+A".to_string(), "Select all".to_string()),
            ("Ctrl+Z".to_string(), "Undo".to_string()),
        ],
        TopBarMenu::Files => vec![
            ("Ctrl+N".to_string(), "New file".to_string()),
            ("Ctrl+P".to_string(), "Find file (fzf)".to_string()),
            ("Ctrl+G".to_string(), "Project search (rg)".to_string()),
            ("Ctrl+S".to_string(), "Save file".to_string()),
            ("Ctrl+Shift+X".to_string(), "Close tab".to_string()),
            ("Ctrl+Shift+Z".to_string(), "Next tab".to_string()),
        ],
        TopBarMenu::Panels => vec![
            ("Ctrl+F".to_string(), "Focus sidebar".to_string()),
            ("Ctrl+E".to_string(), "Focus editor".to_string()),
            ("Ctrl+T".to_string(), "Focus terminal".to_string()),
            ("Ctrl+B".to_string(), "Toggle sidebar".to_string()),
            ("Ctrl+J".to_string(), "Toggle terminal".to_string()),
            ("Esc".to_string(), "Restore layout".to_string()),
        ],
        TopBarMenu::Sidebar => vec![
            (".".to_string(), "Toggle hidden files".to_string()),
            ("Enter".to_string(), "Open file / toggle folder".to_string()),
            ("Home".to_string(), "Jump to top".to_string()),
            ("End".to_string(), "Jump to bottom".to_string()),
            ("Ctrl+D".to_string(), "Page down".to_string()),
            ("Ctrl+U".to_string(), "Page up".to_string()),
        ],
        TopBarMenu::Code => vec![
            ("Ctrl+Space".to_string(), "Autocomplete".to_string()),
            ("Alt+G d".to_string(), "Go to definition".to_string()),
            ("Alt+G r".to_string(), "References".to_string()),
            ("Alt+G n".to_string(), "Rename symbol".to_string()),
            ("Alt+F".to_string(), "Format document".to_string()),
            ("Alt+Enter".to_string(), "Code actions".to_string()),
        ],
        TopBarMenu::Help => vec![
            ("Ctrl+H".to_string(), "Toggle help overlay".to_string()),
            ("Esc".to_string(), "Close help".to_string()),
        ],
        TopBarMenu::Theme => app
            .available_themes
            .iter()
            .map(|t| (t.clone(), "Theme".to_string()))
            .collect(),
    }
}

fn render_dropdown(
    f: &mut Frame,
    y: u16,
    x: u16,
    menu: TopBarMenu,
    selected_index: usize,
    app: &App,
)
 {
    let items = get_menu_items(menu, app);

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

    app.dropdown_area.set(Some(dropdown_area));

    f.render_widget(Clear, dropdown_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.top_bar.text))
        .style(Style::default().bg(app.theme.top_bar.background));

    let mut lines = Vec::new();
    for (i, (shortcut, desc)) in items.iter().enumerate() {
        let style = if i == selected_index {
            Style::default()
                .fg(app.theme.top_bar.background)
                .bg(app.theme.top_bar.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.top_bar.text)
        };

        // Pad shortcut
        let padded_shortcut = format!("{:<1$}", shortcut, max_shortcut_len);

        lines.push(Line::from(vec![
            Span::styled(format!("  {}  ", padded_shortcut), style),
            Span::styled(format!("{}  ", desc), style),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, dropdown_area);
}
