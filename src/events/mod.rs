use crate::app::{App, Panel};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use std::io;

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
        Event::Paste(text) => {
            if matches!(app.active_panel, Panel::Editor) {
                let h = app.last_editor_height.get();
                app.insert_paste(&text, h);
                schedule_document_sync(app);
            } else if matches!(app.active_panel, Panel::Terminal) {
                app.terminal.write(&text);
            }
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

    let term_area = app.terminal_area.get();
    let is_in_terminal = mouse.column >= term_area.x
        && mouse.column < term_area.x + term_area.width
        && mouse.row >= term_area.y
        && mouse.row < term_area.y + term_area.height;

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
        MouseEventKind::Down(crossterm::event::MouseButton::Left) if is_in_editor => {
            app.active_panel = Panel::Editor;
            let new_y = (mouse.row - area.y) as usize + app.editor().scroll_y;
            let new_x = (mouse.column - area.x) as usize;

            if new_y < app.editor().buffer.len_lines() {
                if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                    if app.editor().selection_start.is_none() {
                        app.editor_mut().toggle_selection();
                    }
                } else {
                    app.editor_mut().clear_selection();
                }

                app.editor_mut().cursor_y =
                    new_y.min(app.editor().buffer.len_lines().saturating_sub(1));
                app.editor_mut().cursor_x = new_x;
                app.editor_mut().clamp_cursor_x();
            }
        }
        MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
            if is_in_terminal {
                let term_y = mouse.row.saturating_sub(term_area.y).saturating_sub(1) as usize;
                let term_x = mouse.column.saturating_sub(term_area.x).saturating_sub(1) as usize;

                // For simplicity, we just use term_y as the absolute Y within the grid
                // This means selection highlights will be restricted to the active screen view.
                let abs_y = term_y;

                if let Some((sel_start, _)) = app.terminal_sel {
                    app.terminal_sel = Some((sel_start, (abs_y, term_x)));
                } else {
                    app.terminal_sel = Some(((abs_y, term_x), (abs_y, term_x)));
                }

                // Copy selection immediately on drag like most modern terminals
                copy_terminal_selection(app);
            } else if is_in_editor {
                if app.editor().selection_start.is_none() {
                    app.editor_mut().toggle_selection();
                }

                let new_x = (mouse.column.saturating_sub(area.x)) as usize;

                if mouse.row < area.y {
                    // Dragging above the editor area
                    let scroll_y = app.editor().scroll_y;
                    app.editor_mut().scroll_y = scroll_y.saturating_sub(1);
                    let scroll_y = app.editor().scroll_y;
                    app.editor_mut().cursor_y = scroll_y;
                } else if mouse.row >= area.y + area.height {
                    // Dragging below the editor area
                    let scroll_y = app.editor().scroll_y;
                    let buf_len = app.editor().buffer.len_lines();
                    if scroll_y + (area.height as usize) < buf_len {
                        app.editor_mut().scroll_y += 1;
                    }
                    let scroll_y = app.editor().scroll_y;
                    app.editor_mut().cursor_y = (scroll_y + area.height as usize)
                        .saturating_sub(1)
                        .min(buf_len.saturating_sub(1));
                } else {
                    // Within editor area y-bounds
                    let scroll_y = app.editor().scroll_y;
                    let target_y = (mouse.row - area.y) as usize + scroll_y;
                    app.editor_mut().cursor_y =
                        target_y.min(app.editor().buffer.len_lines().saturating_sub(1));
                }

                app.editor_mut().cursor_x = new_x;
                app.editor_mut().clamp_cursor_x();
            }
        }
        MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
            if app.terminal_sel.is_some() {
                copy_terminal_selection(app);
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn copy_terminal_selection(app: &mut App) {
    if let Some((sel_start, sel_end)) = app.terminal_sel {
        let (sy, sx) = if sel_start < sel_end {
            sel_start
        } else {
            sel_end
        };
        let (ey, ex) = if sel_start < sel_end {
            sel_end
        } else {
            sel_start
        };

        let parser_lock = app.terminal.parser.lock().unwrap();
        let mut screen = parser_lock.screen().clone();
        screen.set_scrollback(app.terminal_scroll);

        let selected_text = screen.contents_between(sy as u16, sx as u16, ey as u16, ex as u16);

        if let Some(clipboard) = &mut app.clipboard {
            let _ = clipboard.set_text(selected_text);
        }
    }
}

fn load_preview(app: &mut App, path: std::path::PathBuf) {
    if let Some(preview) = &app.preview {
        if preview.path.as_ref() == Some(&path) {
            return;
        }
    }
    let mut preview_editor = crate::editor::Editor::new();
    let _ = preview_editor.open(path);
    app.preview = Some(preview_editor);
}

fn open_tab_from_path(app: &mut App, path: std::path::PathBuf) {
    if app.editor().is_dirty {
        app.pending_open_path = Some(path);
        app.show_unsaved_confirm = true;
    } else {
        app.open_file(path);
        app.active_panel = Panel::Editor;
    }
}

fn trigger_picker_preview(app: &mut App) {
    if let Some(res) = app.picker.results.get(app.picker.selected_index) {
        let line = res.line.unwrap_or(0);
        app.picker.preview = crate::search::load_preview_lines(&res.path, line, 8);
    } else {
        app.picker.preview = None;
    }
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> io::Result<()> {
    if app.picker.active {
        match key.code {
            KeyCode::Esc => {
                app.picker.active = false;
                app.picker.preview = None;
            }
            KeyCode::Enter => {
                if let Some(res) = app.picker.results.get(app.picker.selected_index) {
                    let path = res.path.clone();
                    let line = res.line;
                    app.picker.active = false;
                    app.picker.preview = None;

                    // Open the file
                    app.open_file(path);
                    app.active_panel = Panel::Editor;

                    if let Some(l) = line {
                        let h = app.last_editor_height.get();
                        app.editor_mut().cursor_y = l;
                        app.editor_mut().ensure_cursor_visible(h);
                    }
                }
            }
            KeyCode::Up => {
                if app.picker.selected_index > 0 {
                    app.picker.selected_index -= 1;
                } else if !app.picker.results.is_empty() {
                    app.picker.selected_index = app.picker.results.len() - 1;
                }
                // Update scroll
                if app.picker.selected_index < app.picker.scroll {
                    app.picker.scroll = app.picker.selected_index;
                } else if app.picker.selected_index >= app.picker.scroll + 15 {
                    // basic scroll
                    app.picker.scroll = app.picker.selected_index.saturating_sub(14);
                }
                trigger_picker_preview(app);
            }
            KeyCode::Down => {
                if !app.picker.results.is_empty() {
                    app.picker.selected_index =
                        (app.picker.selected_index + 1) % app.picker.results.len();
                }
                // Update scroll
                if app.picker.selected_index < app.picker.scroll {
                    app.picker.scroll = app.picker.selected_index;
                } else if app.picker.selected_index >= app.picker.scroll + 15 {
                    app.picker.scroll = app.picker.selected_index.saturating_sub(14);
                }
                trigger_picker_preview(app);
            }
            KeyCode::Backspace => {
                app.picker.query.pop();
                // Trigger reactive search
                match app.picker.mode {
                    crate::search::SearchMode::File => {
                        app.picker.results = crate::search::run_file_search(&app.picker.query);
                    }
                    crate::search::SearchMode::Grep => {
                        app.picker.results = crate::search::run_grep(&app.picker.query);
                    }
                }
                app.picker.selected_index = 0;
                app.picker.scroll = 0;
                trigger_picker_preview(app);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.picker.query.clear();
                app.picker.results.clear();
                app.picker.selected_index = 0;
                app.picker.scroll = 0;
                app.picker.preview = None;
            }
            KeyCode::Char(c) => {
                app.picker.query.push(c);
                // Trigger reactive search
                match app.picker.mode {
                    crate::search::SearchMode::File => {
                        app.picker.results = crate::search::run_file_search(&app.picker.query);
                    }
                    crate::search::SearchMode::Grep => {
                        app.picker.results = crate::search::run_grep(&app.picker.query);
                    }
                }
                app.picker.selected_index = 0;
                app.picker.scroll = 0;
                trigger_picker_preview(app);
            }
            _ => {}
        }
        return Ok(());
    }

    if app.save_as_state.active {
        match key.code {
            KeyCode::Esc => {
                app.save_as_state.active = false;
            }
            KeyCode::Enter => {
                app.execute_save_as();
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                app.save_as_state.focus_filename = !app.save_as_state.focus_filename;
            }
            KeyCode::Backspace => {
                if app.save_as_state.focus_filename {
                    app.save_as_state.filename.pop();
                    app.save_as_state.is_edited = true;
                }
            }
            KeyCode::Delete => {
                // For a simple text field, delete can behave like backspace if we don't track cursor pos
                if app.save_as_state.focus_filename {
                    app.save_as_state.filename.pop();
                    app.save_as_state.is_edited = true;
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.save_as_state.focus_filename {
                    app.save_as_state.filename.clear();
                    app.save_as_state.is_edited = true;
                }
            }
            KeyCode::Char(c) => {
                if app.save_as_state.focus_filename
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    if !app.save_as_state.is_edited {
                        app.save_as_state.filename.clear();
                        app.save_as_state.is_edited = true;
                    }
                    app.save_as_state.filename.push(c);
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // Handle Quit Confirmation
    if app.show_quit_confirm {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if app.try_save_or_show_save_as(crate::app::SaveAsContext::QuitAfter) {
                    app.should_quit = true;
                }
                app.show_quit_confirm = false;
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

    // Handle Unsaved Changes Confirm (file switch)
    if app.show_unsaved_confirm {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let ctx = if let Some(path) = app.pending_open_path.take() {
                    crate::app::SaveAsContext::SwitchFileAfter(path)
                } else {
                    crate::app::SaveAsContext::CloseTabAfter
                };

                if app.try_save_or_show_save_as(ctx.clone()) {
                    match ctx {
                        crate::app::SaveAsContext::SwitchFileAfter(p) => {
                            app.open_in_new_tab(p);
                            app.active_panel = Panel::Editor;
                        }
                        crate::app::SaveAsContext::CloseTabAfter => {
                            app.close_tab();
                        }
                        _ => {}
                    }
                }
                app.show_unsaved_confirm = false;
                return Ok(());
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.editor_mut().is_dirty = false;
                if let Some(path) = app.pending_open_path.take() {
                    app.open_in_new_tab(path);
                    app.active_panel = Panel::Editor;
                }
                app.show_unsaved_confirm = false;
                return Ok(());
            }
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
                app.pending_open_path = None;
                app.show_unsaved_confirm = false;
                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    // Handle create file prompt
    if app.show_create_file_prompt {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(path) = app.pending_open_path.take() {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::File::create(&path).is_ok() {
                        app.open_in_current_tab(path);
                        app.active_panel = Panel::Editor;
                    }
                }
                app.show_create_file_prompt = false;
                return Ok(());
            }
            KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Esc
            | KeyCode::Char('c')
            | KeyCode::Char('C') => {
                app.pending_open_path = None;
                app.show_create_file_prompt = false;
                app.active_panel = Panel::Sidebar;
                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc => {
                app.show_help = false;
                app.help_scroll = 0;
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.show_help = false;
                app.help_scroll = 0;
            }
            KeyCode::Down => {
                app.help_scroll = app.help_scroll.saturating_add(1);
            }
            KeyCode::Up => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                app.help_scroll = app.help_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(10);
            }
            _ => {}
        }
        return Ok(());
    }

    if key.code == KeyCode::Esc && app.maximized != crate::app::Maximized::None {
        app.maximized = crate::app::Maximized::None;
        return Ok(());
    }

    // Global Control shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        // Ctrl+Shift combos
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            match key.code {
                KeyCode::Char('z') | KeyCode::Char('Z') => {
                    app.next_tab();
                    return Ok(());
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    app.close_tab();
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('g') => {
                app.picker.active = true;
                app.picker.mode = crate::search::SearchMode::Grep;
                app.picker.query.clear();
                app.picker.results.clear();
                app.picker.selected_index = 0;
                app.picker.scroll = 0;
                return Ok(());
            }
            KeyCode::Char('p') => {
                app.picker.active = true;
                app.picker.mode = crate::search::SearchMode::File;
                app.picker.query.clear();
                app.picker.results = crate::search::run_file_search("");
                app.picker.selected_index = 0;
                app.picker.scroll = 0;
                return Ok(());
            }
            KeyCode::Char('q') => {
                if app.tabs.iter().any(|t| t.editor.is_dirty) {
                    app.show_quit_confirm = true;
                } else {
                    app.should_quit = true;
                }
            }
            KeyCode::Char('z') => {
                app.editor_mut().undo();
                schedule_document_sync(app);
            }
            KeyCode::Char('b') => app.show_sidebar = !app.show_sidebar,
            KeyCode::Char('j') => app.show_terminal = !app.show_terminal,
            // Simple Ctrl+W for now (advanced save confirm flow later)
            KeyCode::Char('w') => app.close_tab(),
            KeyCode::Char('s') => {
                let _ = app.editor_mut().save();
            }
            KeyCode::Char('e') => {
                if app.maximized == crate::app::Maximized::Editor {
                    app.maximized = crate::app::Maximized::None;
                } else {
                    app.maximized = crate::app::Maximized::Editor;
                }
                app.preview = None;
                app.active_panel = Panel::Editor;
            }
            KeyCode::Char('f') => {
                app.active_panel = Panel::Sidebar;
                app.show_sidebar = true;
            }
            KeyCode::Char('t') => {
                if app.maximized == crate::app::Maximized::Terminal {
                    app.maximized = crate::app::Maximized::None;
                } else {
                    app.maximized = crate::app::Maximized::Terminal;
                }
                app.preview = None;
                app.active_panel = Panel::Terminal;
                app.show_terminal = true;
            }
            KeyCode::Char('d') if matches!(app.active_panel, Panel::Sidebar) => {
                if let Some(path) = app.sidebar.page_down() {
                    load_preview(app, path);
                }
            }
            KeyCode::Char('u') if matches!(app.active_panel, Panel::Sidebar) => {
                if let Some(path) = app.sidebar.page_up() {
                    load_preview(app, path);
                }
            }
            KeyCode::Char('c') => {
                app.copy_selection();
            }
            KeyCode::Char('v') => {
                let h = app.last_editor_height.get();
                app.paste_clipboard(h);
            }
            KeyCode::Char('a') => {
                app.editor_mut().select_all();
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

    if matches!(app.active_panel, Panel::Terminal) {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.terminal.write("\x03"); // Ctrl+C
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.terminal_scroll = 0;
                app.terminal.write("\x08");
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
                app.terminal.write("\x7f");
            }
            KeyCode::Tab => {
                app.terminal.write("\t");
            }
            KeyCode::Delete => app.terminal.write("\x1b[3~"),
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.terminal_scroll = app.terminal_scroll.saturating_add(1);
                } else {
                    app.terminal_scroll = 0;
                    let app_cursor = app
                        .terminal
                        .parser
                        .lock()
                        .unwrap()
                        .screen()
                        .application_cursor();
                    app.terminal
                        .write(if app_cursor { "\x1bOA" } else { "\x1b[A" });
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.terminal_scroll = app.terminal_scroll.saturating_sub(1);
                } else {
                    app.terminal_scroll = 0;
                    let app_cursor = app
                        .terminal
                        .parser
                        .lock()
                        .unwrap()
                        .screen()
                        .application_cursor();
                    app.terminal
                        .write(if app_cursor { "\x1bOB" } else { "\x1b[B" });
                }
            }
            KeyCode::Right => {
                let app_cursor = app
                    .terminal
                    .parser
                    .lock()
                    .unwrap()
                    .screen()
                    .application_cursor();
                app.terminal
                    .write(if app_cursor { "\x1bOC" } else { "\x1b[C" });
            }
            KeyCode::Left => {
                let app_cursor = app
                    .terminal
                    .parser
                    .lock()
                    .unwrap()
                    .screen()
                    .application_cursor();
                app.terminal
                    .write(if app_cursor { "\x1bOD" } else { "\x1b[D" });
            }
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
            app.preview = None;
            app.active_panel = Panel::Editor;
        }
        return Ok(());
    }

    if matches!(app.active_panel, Panel::Editor) {
        let is_selecting = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Down => {
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                let h = app.last_editor_height.get();
                app.editor_mut().move_cursor_down(h);
                return Ok(());
            }
            KeyCode::Up => {
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                app.editor_mut().move_cursor_up();
                return Ok(());
            }
            KeyCode::Left => {
                app.editor_mut().clear_selection();
                app.editor_mut().move_cursor_left();
                return Ok(());
            }
            KeyCode::Right => {
                app.editor_mut().clear_selection();
                app.editor_mut().move_cursor_right();
                return Ok(());
            }
            KeyCode::Tab => {
                app.editor_mut().insert_tab();
                schedule_document_sync(app);
                return Ok(());
            }
            KeyCode::Char('c') if app.editor().selection_start.is_some() => {
                app.copy_selection();
                app.editor_mut().clear_selection();
                return Ok(());
            }
            KeyCode::Char('v') if app.editor().selection_start.is_some() => {
                let h = app.last_editor_height.get();
                app.paste_clipboard(h);
                return Ok(());
            }
            KeyCode::Home => {
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.editor_mut().cursor_y = 0;
                }
                app.editor_mut().cursor_x = 0;
                let h = app.last_editor_height.get();
                app.editor_mut().ensure_cursor_visible(h);
                return Ok(());
            }
            KeyCode::End => {
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    let lines = app.editor().buffer.len_lines();
                    app.editor_mut().cursor_y = lines.saturating_sub(1);
                }
                let y = app.editor().cursor_y;
                let max_x = app.editor().get_max_cursor_x(y);
                app.editor_mut().cursor_x = max_x;
                let h = app.last_editor_height.get();
                app.editor_mut().ensure_cursor_visible(h);
                return Ok(());
            }
            KeyCode::PageUp => {
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                let h = app.last_editor_height.get();
                for _ in 0..h {
                    app.editor_mut().move_cursor_up();
                }
                return Ok(());
            }
            KeyCode::PageDown => {
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                let h = app.last_editor_height.get();
                for _ in 0..h {
                    app.editor_mut().move_cursor_down(h);
                }
                return Ok(());
            }
            KeyCode::Delete => {
                app.editor_mut().delete_forward_char();
                return Ok(());
            }
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.cut_selection();
                return Ok(());
            }
            KeyCode::Backspace => {
                app.editor_mut().delete_char();
                return Ok(());
            }
            KeyCode::Enter => {
                app.editor_mut().insert_char('\n');
                app.editor_mut().cursor_y += 1;
                app.editor_mut().cursor_x = 0;
                let h = app.last_editor_height.get();
                app.editor_mut().ensure_cursor_visible(h);
                return Ok(());
            }
            KeyCode::Char(c) => {
                app.editor_mut().insert_char(c);
                return Ok(());
            }
            _ => {}
        }
    }

    // Sidebar navigation
    match key.code {
        KeyCode::Char('.') if matches!(app.active_panel, Panel::Sidebar) => {
            app.sidebar.show_hidden = !app.sidebar.show_hidden;
            app.sidebar.update_flat_list();
            if app.sidebar.selected_index >= app.sidebar.flat_list.len()
                && !app.sidebar.flat_list.is_empty()
            {
                app.sidebar.selected_index = app.sidebar.flat_list.len() - 1;
            }
        }
        KeyCode::PageDown if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.page_down() {
                load_preview(app, path);
            }
        }
        KeyCode::PageUp if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.page_up() {
                load_preview(app, path);
            }
        }
        KeyCode::Home if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.start() {
                load_preview(app, path);
            }
        }
        KeyCode::End if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.end() {
                load_preview(app, path);
            }
        }
        KeyCode::Down if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.next() {
                load_preview(app, path);
            }
        }
        KeyCode::Up if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.previous() {
                load_preview(app, path);
            }
        }
        KeyCode::Enter if matches!(app.active_panel, Panel::Sidebar) => {
            // First try to expand dirs
            if let Ok(Some(path)) = app.sidebar.toggle_selected() {
                app.preview = None;
                open_tab_from_path(app, path);
            }
        }
        _ => {}
    }

    Ok(())
}
