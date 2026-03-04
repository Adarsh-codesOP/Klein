use ratatui::Frame;
use crate::app::App;

pub mod sidebar;
pub mod editor;
pub mod terminal;
pub mod status_bar;
pub mod help;
pub mod layout;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = layout::get_main_layout(f.size(), app.show_help, app.show_terminal);
    
    if app.show_help {
        help::render(f, chunks[0]);
    }

    let main_chunks = layout::get_editor_layout(chunks[1], app.show_sidebar);

    if app.show_sidebar {
        sidebar::render(f, main_chunks[0], app);
    }

    editor::render(f, main_chunks[1], app);

    if app.show_terminal {
        terminal::render(f, chunks[2], app);
    }

    status_bar::render(f, chunks[3], app);
}
