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
    let area = app.editor_area.get();
    let is_in_editor = mouse.column >= area.x 
        && mouse.column < area.x + area.width 
        && mouse.row >= area.y 
        && mouse.row < area.y + area.height;

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if matches!(app.active_panel, Panel::Terminal) {
                app.terminal_scroll = app.terminal_scroll.saturating_add(3);
            } else if is_in_editor {
                app.editor.scroll_y = app.editor.scroll_y.saturating_sub(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if matches!(app.active_panel, Panel::Terminal) {
                app.terminal_scroll = app.terminal_scroll.saturating_sub(3);
            } else if is_in_editor {
                app.editor.scroll_y = app.editor.scroll_y.saturating_add(3);
                let max_scroll = app.editor.buffer.len_lines().saturating_sub(1);
                if app.editor.scroll_y > max_scroll {
                    app.editor.scroll_y = max_scroll;
                }
            }
        }
        MouseEventKind::Down(crossterm::event::MouseButton::Left) if is_in_editor => {
            app.active_panel = Panel::Editor;
            let new_y = (mouse.row - area.y) as usize + app.editor.scroll_y;
            let new_x = (mouse.column - area.x) as usize;
            
            if new_y < app.editor.buffer.len_lines() {
                if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                    if app.editor.selection_start.is_none() {
                        app.editor.toggle_selection();
                    }
                } else {
                    app.editor.clear_selection();
                }
                
                app.editor.cursor_y = new_y;
                app.editor.cursor_x = new_x;
                app.editor.clamp_cursor_x();
            }
        }
        MouseEventKind::Drag(crossterm::event::MouseButton::Left) if is_in_editor => {
            let new_y = (mouse.row - area.y) as usize + app.editor.scroll_y;
            let new_x = (mouse.column - area.x) as usize;
            
            if new_y < app.editor.buffer.len_lines() {
                if app.editor.selection_start.is_none() {
                    app.editor.toggle_selection();
                }
                app.editor.cursor_y = new_y;
                app.editor.cursor_x = new_x;
                app.editor.clamp_cursor_x();
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> io::Result<()> {
    // Handle Quit Confirmation
    if app.show_quit_confirm {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let _ = app.editor.save();
                app.should_quit = true;
                return Ok(());
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.should_quit = true;
                return Ok(());
            }
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') => {
                app.show_quit_confirm = false;
                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    // Global shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') => {
                if app.editor.is_dirty {
                    app.show_quit_confirm = true;
                } else {
                    app.should_quit = true;
                }
            }
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

    if matches!(app.active_panel, Panel::Editor) {
        let is_selecting = key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT);
        
        match key.code {
            KeyCode::Down => {
                if is_selecting { app.editor.toggle_selection(); }
                else { app.editor.clear_selection(); }
                app.editor.move_cursor_down(app.last_editor_height.get());
                return Ok(());
            }
            KeyCode::Up => {
                if is_selecting { app.editor.toggle_selection(); }
                else { app.editor.clear_selection(); }
                app.editor.move_cursor_up();
                return Ok(());
            }
            KeyCode::Left => {
                app.editor.clear_selection();
                app.editor.move_cursor_left();
                return Ok(());
            }
            KeyCode::Right => {
                app.editor.clear_selection();
                app.editor.move_cursor_right();
                return Ok(());
            }
            KeyCode::Tab => {
                app.editor.insert_tab();
                return Ok(());
            }
            KeyCode::Char('c') if app.editor.selection_start.is_some() => {
                app.editor.copy();
                app.editor.clear_selection();
                return Ok(());
            }
            KeyCode::Char('v') if app.editor.selection_start.is_some() => {
                app.editor.paste();
                return Ok(());
            }
            KeyCode::Backspace => {
                app.editor.delete_char();
                return Ok(());
            }
            KeyCode::Enter => {
                app.editor.insert_char('\n');
                app.editor.cursor_y += 1;
                app.editor.cursor_x = 0;
                return Ok(());
            }
            KeyCode::Char(c) => {
                app.editor.insert_char(c);
                return Ok(());
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Down if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.next() {
                let _ = app.editor.open(path);
            }
        }
        KeyCode::Up if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.previous() {
                let _ = app.editor.open(path);
            }
        }
        KeyCode::Enter if matches!(app.active_panel, Panel::Sidebar) => {
            if let Ok(Some(path)) = app.sidebar.toggle_selected() {
                let _ = app.editor.open(path);
                app.active_panel = Panel::Editor;
            }
        }
        _ => {}
    }

    Ok(())
}
