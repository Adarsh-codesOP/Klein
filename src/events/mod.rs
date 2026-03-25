use std::io;
use std::path::PathBuf;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use crate::app::{App, Maximized, Panel, SaveAsContext, SaveAsState, default_save_filename};

/// Save the active editor if it has a path, otherwise open the Save As dialog.
/// Executes `context` immediately if the file is named; stores it for later if untitled.
fn try_save_or_show_save_as(app: &mut App, context: SaveAsContext) {
    if app.editor().path.is_some() {
        let _ = app.editor_mut().save();
        execute_save_context(app, context);
    } else {
        // Dismiss any active confirm dialog — save_as takes over input
        app.show_quit_confirm = false;
        app.show_close_confirm = false;
        app.show_unsaved_confirm = false;
        app.save_as = Some(SaveAsState {
            folder: app.cwd.to_string_lossy().into_owned(),
            filename: default_save_filename(),
            active_field: 1, // filename focused by default
            context,
        });
    }
}

/// Carry out the post-save action (quit, close tab, switch file, or nothing).
fn execute_save_context(app: &mut App, context: SaveAsContext) {
    match context {
        SaveAsContext::JustSave => {}
        SaveAsContext::SaveAndQuit => {
            app.should_quit = true;
        }
        SaveAsContext::SaveAndClose => {
            app.show_close_confirm = false;
            app.close_tab();
        }
        SaveAsContext::SaveAndSwitch => {
            app.show_unsaved_confirm = false;
            if let Some(path) = app.pending_open_path.take() {
                app.open_in_new_tab(path);
                app.active_panel = Panel::Editor;
            }
        }
    }
}

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
        // Bracketed paste: terminal sends clipboard text directly (e.g. via Ctrl+Shift+V).
        // Insert it at the cursor just like Ctrl+V would.
        Event::Paste(text) => {
            if !matches!(app.active_panel, crate::app::Panel::Terminal) {
                let h = app.last_editor_height.get();
                app.editor_mut().insert_paste(&text, h);
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

    let tarea = app.terminal_area.get();
    let is_in_terminal = tarea.width > 0
        && mouse.column >= tarea.x
        && mouse.column < tarea.x + tarea.width
        && mouse.row >= tarea.y
        && mouse.row < tarea.y + tarea.height;

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

        // ── Terminal mouse selection ──────────────────────────────────────
        MouseEventKind::Down(crossterm::event::MouseButton::Left) if is_in_terminal => {
            app.active_panel = Panel::Terminal;
            app.terminal_sel = None; // clear previous selection on new click
            let col = mouse.column - tarea.x;
            let row = mouse.row - tarea.y;
            app.terminal_sel = Some(((col, row), (col, row)));
        }
        MouseEventKind::Drag(crossterm::event::MouseButton::Left) if is_in_terminal || app.terminal_sel.is_some() && matches!(app.active_panel, Panel::Terminal) => {
            if let Some(((sc, sr), _)) = app.terminal_sel {
                let col = mouse.column.saturating_sub(tarea.x).min(tarea.width.saturating_sub(1));
                let row = mouse.row.saturating_sub(tarea.y).min(tarea.height.saturating_sub(1));
                app.terminal_sel = Some(((sc, sr), (col, row)));
            }
        }
        MouseEventKind::Up(crossterm::event::MouseButton::Left) if app.terminal_sel.is_some() && matches!(app.active_panel, Panel::Terminal) => {
            if let Some(((c1, r1), (c2, r2))) = app.terminal_sel {
                // Copy selection to clipboard if it spans more than a single point
                if (c1, r1) != (c2, r2) {
                    copy_terminal_selection(app, c1, r1, c2, r2);
                } else {
                    // Single click — clear selection
                    app.terminal_sel = None;
                }
            }
        }

        // ── Editor mouse selection ────────────────────────────────────────
        MouseEventKind::Down(crossterm::event::MouseButton::Left) if is_in_editor => {
            app.active_panel = Panel::Editor;
            app.terminal_sel = None;
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

                app.editor_mut().cursor_y = new_y;
                app.editor_mut().cursor_x = new_x;
                app.editor_mut().clamp_cursor_x();
            }
        }
        MouseEventKind::Drag(crossterm::event::MouseButton::Left) if matches!(app.active_panel, Panel::Editor) => {
            if app.editor().selection_start.is_none() {
                app.editor_mut().toggle_selection();
            }

            let new_x = (mouse.column.saturating_sub(area.x)) as usize;

            if mouse.row < area.y {
                let scroll_y = app.editor().scroll_y;
                app.editor_mut().scroll_y = scroll_y.saturating_sub(1);
                let scroll_y = app.editor().scroll_y;
                app.editor_mut().cursor_y = scroll_y;
            } else if mouse.row >= area.y + area.height {
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
                let scroll_y = app.editor().scroll_y;
                app.editor_mut().cursor_y = (mouse.row - area.y) as usize + scroll_y;
            }

            app.editor_mut().cursor_x = new_x;
            app.editor_mut().clamp_cursor_x();
        }
        _ => {}
    }
    Ok(())
}

/// Extract the selected text from the terminal output and copy it to the clipboard.
fn copy_terminal_selection(app: &mut App, c1: u16, r1: u16, c2: u16, r2: u16) {
    use crate::ui::terminal::strip_ansi;

    let output_raw = app.terminal.output.lock().unwrap().clone();
    let output = strip_ansi(&output_raw);
    let all_lines: Vec<&str> = output.lines().collect();

    let tarea = app.terminal_area.get();
    let height = tarea.height as usize;
    let start_line = all_lines.len().saturating_sub(height).saturating_sub(app.terminal_scroll);
    let end_line = all_lines.len().saturating_sub(app.terminal_scroll);
    let visible: Vec<&str> = if start_line < end_line {
        all_lines[start_line..end_line.min(all_lines.len())].to_vec()
    } else {
        vec![]
    };

    // Normalise to top-left → bottom-right
    let ((sc, sr), (ec, er)) = if (r1, c1) <= (r2, c2) {
        ((c1, r1), (c2, r2))
    } else {
        ((c2, r2), (c1, r1))
    };

    let mut result = String::new();
    for row in sr..=er {
        let line = visible.get(row as usize).copied().unwrap_or("");
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len() as u16;
        let col_start = if row == sr { sc.min(len) } else { 0 };
        let col_end   = if row == er { ec.min(len) } else { len };
        let chunk: String = chars[col_start as usize..col_end as usize].iter().collect();
        if !result.is_empty() { result.push('\n'); }
        result.push_str(&chunk);
    }

    if !result.is_empty() {
        if let Some(cb) = &mut app.editor_mut().clipboard {
            let _ = cb.set_text(result);
        }
    }
}

fn load_preview(app: &mut App, path: std::path::PathBuf) {
    let mut preview_editor = crate::editor::Editor::new();
    let _ = preview_editor.open(path);
    app.preview = Some(preview_editor);
}

fn open_tab_from_path(app: &mut App, path: std::path::PathBuf) {
    if app.editor().is_dirty {
        app.pending_open_path = Some(path);
        app.show_unsaved_confirm = true;
    } else {
        app.open_in_new_tab(path);
        app.active_panel = Panel::Editor;
    }
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> io::Result<()> {
    // Help overlay steals all input when shown
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
            KeyCode::Up => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::Down => {
                app.help_scroll += 1;
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                app.help_scroll += 10;
            }
            _ => {}
        }
        return Ok(());
    }

    // "File doesn't exist — create it?" prompt
    if app.create_file_prompt.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(path) = app.create_file_prompt.take() {
                    // Create parent directories if needed, then create the empty file.
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::write(&path, "").is_ok() {
                        app.open_in_current_tab(path);
                        app.active_panel = Panel::Editor;
                        app.sidebar.refresh();
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.create_file_prompt = None;
                // Open as normal with no file — sidebar stays focused.
            }
            _ => {}
        }
        return Ok(());
    }

    // Save As dialog steals all input when shown
    if app.save_as.is_some() {
        // Block all Ctrl shortcuts so they don't fire global actions
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Esc => {
                    app.save_as = None;
                }
                KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                    if let Some(sa) = &mut app.save_as {
                        sa.active_field = 1 - sa.active_field;
                    }
                }
                KeyCode::Enter => {
                    if let Some(sa) = app.save_as.take() {
                        let path = PathBuf::from(&sa.folder).join(&sa.filename);
                        let content = app.editor().buffer.to_string();
                        if std::fs::write(&path, content).is_ok() {
                            app.editor_mut().path = Some(path);
                            app.editor_mut().is_dirty = false;
                            app.sidebar.refresh(); // show the new file in the tree
                            execute_save_context(app, sa.context);
                        }
                        // If write failed, dialog closes silently — file stays untitled
                    }
                }
                KeyCode::Backspace => {
                    if let Some(sa) = &mut app.save_as {
                        if sa.active_field == 0 {
                            sa.folder.pop();
                        } else {
                            sa.filename.pop();
                        }
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(sa) = &mut app.save_as {
                        if sa.active_field == 0 {
                            sa.folder.push(c);
                        } else {
                            sa.filename.push(c);
                        }
                    }
                }
                _ => {}
            }
        }
        return Ok(());
    }

    // Handle Quit Confirmation
    if app.show_quit_confirm {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                try_save_or_show_save_as(app, SaveAsContext::SaveAndQuit);
                return Ok(());
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.should_quit = true;
                return Ok(());
            }
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') => {
                app.show_quit_confirm = false;
                if app.terminal_triggered_quit {
                    app.terminal_triggered_quit = false;
                    app.terminal.restart();
                    app.show_terminal = true;
                    app.active_panel = Panel::Terminal;
                }
                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    // Handle Close Tab Confirm (Ctrl+W with unsaved changes)
    if app.show_close_confirm {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                try_save_or_show_save_as(app, SaveAsContext::SaveAndClose);
                return Ok(());
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.editor_mut().is_dirty = false;
                app.close_tab();
                app.show_close_confirm = false;
                return Ok(());
            }
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
                app.show_close_confirm = false;
                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    // Handle Unsaved Changes Confirm (file switch)
    if app.show_unsaved_confirm {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                try_save_or_show_save_as(app, SaveAsContext::SaveAndSwitch);
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

    // Escape restores from maximized view
    if key.code == KeyCode::Esc && app.maximized != Maximized::None {
        app.maximized = Maximized::None;
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
            KeyCode::Char('q') => {
                if app.tabs.iter().any(|t| t.editor.is_dirty) {
                    app.show_quit_confirm = true;
                } else {
                    app.should_quit = true;
                }
            }
            KeyCode::Char('b') => app.show_sidebar = !app.show_sidebar,
            KeyCode::Char('j') => app.show_terminal = !app.show_terminal,
            KeyCode::Char('s') => {
                try_save_or_show_save_as(app, SaveAsContext::JustSave);
            }
            KeyCode::Char('e') => {
                if matches!(app.active_panel, Panel::Editor) {
                    app.maximized = if app.maximized == Maximized::Editor {
                        Maximized::None
                    } else {
                        Maximized::Editor
                    };
                } else {
                    app.preview = None;
                    app.active_panel = Panel::Editor;
                    app.maximized = Maximized::None;
                }
            }
            KeyCode::Char('r') => {
                app.active_panel = Panel::Sidebar;
                app.show_sidebar = true;
                app.maximized = Maximized::None;
            }
            KeyCode::Char('t') => {
                if matches!(app.active_panel, Panel::Terminal) {
                    app.maximized = if app.maximized == Maximized::Terminal {
                        Maximized::None
                    } else {
                        Maximized::Terminal
                    };
                } else {
                    app.preview = None;
                    app.active_panel = Panel::Terminal;
                    app.show_terminal = true;
                    app.maximized = Maximized::None;
                }
            }
            KeyCode::Char('x') => {
                app.editor_mut().cut();
            }
            KeyCode::Char('c') => {
                app.editor_mut().copy();
            }
            KeyCode::Char('v') => {
                let h = app.last_editor_height.get();
                app.editor_mut().paste(h);
            }
            KeyCode::Char('a') => {
                app.editor_mut().select_all();
            }
            KeyCode::Char('w') => {
                if app.tabs[app.active_tab].editor.is_dirty {
                    app.show_close_confirm = true;
                } else {
                    app.close_tab();
                }
            }
            KeyCode::Char('.') => {
                app.sidebar.toggle_hidden();
            }
            KeyCode::Char('d') | KeyCode::Char('D') if matches!(app.active_panel, Panel::Sidebar) => {
                if let Some(path) = app.sidebar.page_next() {
                    load_preview(app, path);
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U') if matches!(app.active_panel, Panel::Sidebar) => {
                if let Some(path) = app.sidebar.page_previous() {
                    load_preview(app, path);
                }
            }
            KeyCode::Char('z') => {
                let h = app.last_editor_height.get();
                app.editor_mut().undo(h);
            }
            KeyCode::Char('h') => app.show_help = !app.show_help,
            KeyCode::Home if matches!(app.active_panel, Panel::Editor) => {
                let h = app.last_editor_height.get();
                app.editor_mut().move_to_file_start();
                let _ = h; // scroll_y already reset inside move_to_file_start
            }
            KeyCode::End if matches!(app.active_panel, Panel::Editor) => {
                let h = app.last_editor_height.get();
                app.editor_mut().move_to_file_end(h);
            }
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
            app.preview = None;
            app.active_panel = Panel::Editor;
        }
        return Ok(());
    }

    if matches!(app.active_panel, Panel::Editor) {
        let is_selecting = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Down => {
                if is_selecting { app.editor_mut().toggle_selection(); }
                else { app.editor_mut().clear_selection(); }
                let h = app.last_editor_height.get();
                app.editor_mut().move_cursor_down(h);
                return Ok(());
            }
            KeyCode::Up => {
                if is_selecting { app.editor_mut().toggle_selection(); }
                else { app.editor_mut().clear_selection(); }
                app.editor_mut().move_cursor_up();
                return Ok(());
            }
            KeyCode::Left => {
                if is_selecting { app.editor_mut().toggle_selection(); }
                else { app.editor_mut().clear_selection(); }
                app.editor_mut().move_cursor_left();
                return Ok(());
            }
            KeyCode::Right => {
                if is_selecting { app.editor_mut().toggle_selection(); }
                else { app.editor_mut().clear_selection(); }
                app.editor_mut().move_cursor_right();
                return Ok(());
            }
            KeyCode::Home => {
                if is_selecting { app.editor_mut().toggle_selection(); }
                else { app.editor_mut().clear_selection(); }
                app.editor_mut().move_to_line_start();
                return Ok(());
            }
            KeyCode::End => {
                if is_selecting { app.editor_mut().toggle_selection(); }
                else { app.editor_mut().clear_selection(); }
                app.editor_mut().move_to_line_end();
                return Ok(());
            }
            KeyCode::PageUp => {
                let h = app.last_editor_height.get();
                app.editor_mut().page_up(h);
                return Ok(());
            }
            KeyCode::PageDown => {
                let h = app.last_editor_height.get();
                app.editor_mut().page_down(h);
                return Ok(());
            }
            KeyCode::Tab => {
                app.editor_mut().insert_tab();
                return Ok(());
            }
            KeyCode::Char('c') if app.editor().selection_start.is_some() => {
                app.editor_mut().copy();
                app.editor_mut().clear_selection();
                return Ok(());
            }
            KeyCode::Char('v') if app.editor().selection_start.is_some() => {
                let h = app.last_editor_height.get();
                app.editor_mut().paste(h);
                return Ok(());
            }
            KeyCode::Backspace => {
                app.editor_mut().delete_char();
                return Ok(());
            }
            KeyCode::Delete => {
                app.editor_mut().delete_char_forward();
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

    if matches!(app.active_panel, Panel::Sidebar) {
        match key.code {
            KeyCode::Down => {
                if let Some(path) = app.sidebar.next() {
                    load_preview(app, path);
                }
            }
            KeyCode::Up => {
                if let Some(path) = app.sidebar.previous() {
                    load_preview(app, path);
                }
            }
            KeyCode::PageDown => {
                if let Some(path) = app.sidebar.page_next() {
                    load_preview(app, path);
                }
            }
            KeyCode::PageUp => {
                if let Some(path) = app.sidebar.page_previous() {
                    load_preview(app, path);
                }
            }
            KeyCode::Home => {
                if let Some(path) = app.sidebar.go_to_first() {
                    load_preview(app, path);
                }
            }
            KeyCode::End => {
                if let Some(path) = app.sidebar.go_to_last() {
                    load_preview(app, path);
                }
            }
            KeyCode::Char('.') => {
                // Toggle hidden files — plain '.' used because terminals don't
                // reliably report Ctrl modifier for punctuation keys
                app.sidebar.toggle_hidden();
            }
            KeyCode::Enter => {
                if let Ok(Some(path)) = app.sidebar.toggle_selected() {
                    app.preview = None;
                    open_tab_from_path(app, path);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
