use std::io;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use crate::app::{App, Panel};

pub fn handle_event(app: &mut App, event: Event) -> io::Result<()> {
    match event {
        Event::Key(key) => {
            if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                handle_key_event(app, key)?;
            }
        }
        Event::Mouse(mouse) => {
            handle_mouse_event(app, mouse)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent) -> io::Result<()> {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if matches!(app.active_panel, Panel::Terminal) {
                app.terminal_scroll = app.terminal_scroll.saturating_add(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if matches!(app.active_panel, Panel::Terminal) {
                app.terminal_scroll = app.terminal_scroll.saturating_sub(3);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> io::Result<()> {
    // Global shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('b') => app.show_sidebar = !app.show_sidebar,
            KeyCode::Char('`') => app.show_terminal = !app.show_terminal,
            KeyCode::Char('s') => {
                let _ = app.editor.save();
            }
            KeyCode::Char('e') => {
                app.active_panel = Panel::Editor;
            }
            KeyCode::Char('r') => {
                app.active_panel = Panel::Sidebar;
                app.show_sidebar = true;
            }
            KeyCode::Char('t') => {
                app.active_panel = Panel::Terminal;
                app.show_terminal = true;
            }
            KeyCode::Char('f') => {
                app.editor.is_searching = true;
                app.editor.search_query.clear();
            }
            KeyCode::Char('c') => {
                app.editor.copy();
            }
            KeyCode::Char('h') => app.show_help = !app.show_help,
            KeyCode::Right | KeyCode::Left => {
                app.active_panel = match app.active_panel {
                    Panel::Sidebar => Panel::Editor,
                    Panel::Editor => Panel::Sidebar,
                    Panel::Terminal => Panel::Editor,
                };
            }
            KeyCode::Down => {
                app.active_panel = Panel::Terminal;
                app.show_terminal = true;
            }
            KeyCode::Up => {
                app.active_panel = Panel::Editor;
            }
            _ => {}
        }
        return Ok(());
    }

    if app.editor.is_searching {
        match key.code {
            KeyCode::Char(c) => app.editor.search_query.push(c),
            KeyCode::Backspace => {
                app.editor.search_query.pop();
            }
            KeyCode::Enter => {
                app.editor.search(&app.editor.search_query.clone());
                app.editor.is_searching = false;
            }
            KeyCode::Esc => {
                app.editor.is_searching = false;
            }
            _ => {}
        }
        return Ok(());
    }

    if matches!(app.active_panel, Panel::Terminal) {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.terminal.write("\x03"); // Ctrl+C
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.terminal_scroll = 0;
                app.terminal.write("\x08"); // Ctrl+H is often \x08
            }
            KeyCode::Char(c) => {
                app.terminal_scroll = 0;
                app.terminal.write(&c.to_string());
            }
            KeyCode::Enter => {
                app.terminal_scroll = 0;
                app.terminal.write("\r");
            }
            KeyCode::Backspace => {
                app.terminal_scroll = 0;
                app.terminal.write("\x7f"); // Git Bash usually expects \x7f
            }
            KeyCode::Tab => {
                app.terminal.write("\t");
            }
            KeyCode::Delete => app.terminal.write("\x1b[3~"),
            KeyCode::Up => {
                app.terminal_scroll = app.terminal_scroll.saturating_add(1);
            }
            KeyCode::Down => {
                app.terminal_scroll = app.terminal_scroll.saturating_sub(1);
            }
            KeyCode::Right => app.terminal.write("\x1b[C"),
            KeyCode::Left => app.terminal.write("\x1b[D"),
            KeyCode::PageUp => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.terminal_scroll = app.terminal_scroll.saturating_add(5);
                } else {
                    app.terminal_scroll = 0;
                    app.terminal.write("\x1b[5~");
                }
            }
            KeyCode::PageDown => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.terminal_scroll = app.terminal_scroll.saturating_sub(5);
                } else {
                    app.terminal_scroll = 0;
                    app.terminal.write("\x1b[6~");
                }
            }
            _ => {}
        }
        if key.code == KeyCode::Esc {
            app.active_panel = Panel::Editor;
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Down => {
            match app.active_panel {
                Panel::Sidebar => {
                    if let Some(path) = app.sidebar.next() {
                        let _ = app.editor.open(path);
                    }
                }
                Panel::Editor => app.editor.move_cursor_down(app.last_editor_height.get()),
                _ => {}
            }
        }
        KeyCode::Up => {
            match app.active_panel {
                Panel::Sidebar => {
                    if let Some(path) = app.sidebar.previous() {
                        let _ = app.editor.open(path);
                    }
                }
                Panel::Editor => app.editor.move_cursor_up(),
                _ => {}
            }
        }
        KeyCode::Left => {
            if matches!(app.active_panel, Panel::Editor) {
                app.editor.move_cursor_left();
            }
        }
        KeyCode::Right => {
            if matches!(app.active_panel, Panel::Editor) {
                app.editor.move_cursor_right();
            }
        }
        KeyCode::Enter => {
            match app.active_panel {
                Panel::Sidebar => {
                    if let Ok(Some(path)) = app.sidebar.toggle_selected() {
                        let _ = app.editor.open(path);
                        app.active_panel = Panel::Editor;
                    }
                }
                Panel::Editor => {
                    app.editor.insert_char('\n');
                    app.editor.cursor_y += 1;
                    app.editor.cursor_x = 0;
                }
                _ => {}
            }
        }
        KeyCode::Backspace => {
            if matches!(app.active_panel, Panel::Editor) {
                app.editor.delete_char();
            }
        }
        KeyCode::Char(c) => {
            if matches!(app.active_panel, Panel::Editor) {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.editor.insert_char(c);
                }
            }
        }
        _ => {}
    }

    Ok(())
}
