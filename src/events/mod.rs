pub mod klein_event;
pub mod timers;

use crate::app::{App, Panel};
use crate::lsp::actor::LspServerNotification;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use std::io;

pub fn handle_lsp_notification(app: &mut App, notification: LspServerNotification) {
    match notification.method.as_str() {
        "textDocument/publishDiagnostics" => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(notification.params)
            {
                if let Some(path) = crate::lsp::router::uri_to_path(&params.uri) {
                    // Find the buffer for this file to do position conversion
                    let mut diagnostics = Vec::new();
                    let mut buffer = None;

                    for tab in &app.tabs {
                        if tab.editor.path.as_ref() == Some(&path) {
                            buffer = Some(tab.editor.buffer.clone());
                            break;
                        }
                    }

                    if let Some(buf) = buffer {
                        for diag in params.diagnostics {
                            diagnostics.push(crate::lsp::router::to_klein_diagnostic(&diag, &buf));
                        }
                        app.lsp_state.diagnostics.insert(path, diagnostics);
                    }
                }
            }
        }
        _ => {
            log::trace!("unhandled LSP notification: {}", notification.method);
        }
    }
}

pub fn handle_timer_event(app: &mut App, kind: klein_event::TimerKind) {
    match kind {
        klein_event::TimerKind::DocumentSync => {
            app.notify_lsp_did_change();
        }
        klein_event::TimerKind::CompletionTrigger => {
            app.trigger_completion();
        }
        klein_event::TimerKind::HoverTrigger => {
            app.trigger_hover();
        }
    }
}

fn schedule_document_sync(app: &mut App) {
    if let Some(ref mut tm) = app.timer_manager {
        log::warn!("LSP: scheduling document sync timer");
        tm.schedule(
            klein_event::TimerKind::DocumentSync,
            std::time::Duration::from_millis(150),
        );
    }
}

fn schedule_completion(app: &mut App) {
    if let Some(ref mut tm) = app.timer_manager {
        log::warn!("LSP: scheduling completion timer");
        tm.schedule(
            klein_event::TimerKind::CompletionTrigger,
            std::time::Duration::from_millis(50),
        );
    }
}

#[allow(dead_code)]
fn schedule_hover(app: &mut App) {
    if let Some(ref mut tm) = app.timer_manager {
        tm.schedule(
            klein_event::TimerKind::HoverTrigger,
            std::time::Duration::from_millis(400),
        );
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

fn point_in_rect(col: u16, row: u16, r: ratatui::layout::Rect) -> bool {
    r.width > 0 && r.height > 0
        && col >= r.x && col < r.x + r.width
        && row >= r.y && row < r.y + r.height
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent) -> io::Result<()> {
    let area = app.editor_area.get();
    let is_in_editor = point_in_rect(mouse.column, mouse.row, area);

    let term_area = app.terminal_area.get();
    let is_in_terminal = point_in_rect(mouse.column, mouse.row, term_area);

    let sidebar_area = app.sidebar_area.get();
    let is_in_sidebar = point_in_rect(mouse.column, mouse.row, sidebar_area);

    let top_bar_area = app.top_bar_area.get();
    let is_in_top_bar = point_in_rect(mouse.column, mouse.row, top_bar_area);

    let dropdown_area = app.dropdown_area.get();
    let is_in_dropdown = dropdown_area
        .map(|r| point_in_rect(mouse.column, mouse.row, r))
        .unwrap_or(false);

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if is_in_terminal || matches!(app.active_panel, Panel::Terminal) {
                app.terminal_scroll = app.terminal_scroll.saturating_add(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if is_in_terminal || matches!(app.active_panel, Panel::Terminal) {
                app.terminal_scroll = app.terminal_scroll.saturating_sub(3);
            }
        }
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            // Click on dropdown item
            if is_in_dropdown {
                if let Some(dd) = dropdown_area {
                    // Row within dropdown content (subtract border)
                    let item_row = mouse.row.saturating_sub(dd.y + 1) as usize;
                    let items_len = if let Some(menu) = app.top_bar.active_menu {
                        crate::ui::top_bar::get_menu_items(menu, app).len()
                    } else {
                        0
                    };
                    if item_row < items_len {
                        app.top_bar.selected_index = item_row;
                        app.execute_top_bar_action();
                    }
                }
            }
            // Click on top bar menu label
            else if is_in_top_bar {
                let positions = app.top_bar_positions.take();
                let menus = [
                    crate::app::TopBarMenu::Navigation,
                    crate::app::TopBarMenu::Edit,
                    crate::app::TopBarMenu::Files,
                    crate::app::TopBarMenu::Panels,
                    crate::app::TopBarMenu::Sidebar,
                    crate::app::TopBarMenu::Code,
                    crate::app::TopBarMenu::Help,
                    crate::app::TopBarMenu::Theme,
                ];
                let mut clicked_menu = None;
                for (i, &(start, end)) in positions.iter().enumerate() {
                    if mouse.column >= start && mouse.column < end {
                        clicked_menu = Some(menus[i]);
                        break;
                    }
                }
                app.top_bar_positions.set(positions);
                if let Some(menu) = clicked_menu {
                    if app.top_bar.active_menu == Some(menu) {
                        app.close_menu();
                    } else {
                        app.top_bar.active_menu = Some(menu);
                        app.top_bar.selected_index = 0;
                    }
                }
            }
            // Click in sidebar
            else if is_in_sidebar {
                // Close any open menu
                if app.top_bar.active_menu.is_some() {
                    app.close_menu();
                }
                app.active_panel = Panel::Sidebar;
                app.terminal_sel = None;
                // Calculate which sidebar entry was clicked
                let clicked_row = (mouse.row - sidebar_area.y) as usize;
                let target_index = app.sidebar.offset + clicked_row;
                if target_index < app.sidebar.flat_list.len() {
                    app.sidebar.selected_index = target_index;
                }
            }
            // Click in editor
            else if is_in_editor {
                if app.top_bar.active_menu.is_some() {
                    app.close_menu();
                }
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

                    app.editor_mut().cursor_y =
                        new_y.min(app.editor().buffer.len_lines().saturating_sub(1));
                    app.editor_mut().cursor_x = new_x;
                    app.editor_mut().clamp_cursor_x();
                }
            }
            // Click in terminal
            else if is_in_terminal {
                if app.top_bar.active_menu.is_some() {
                    app.close_menu();
                }
                app.active_panel = Panel::Terminal;
                app.terminal_sel = None;
            }
            // Click elsewhere closes menu
            else if app.top_bar.active_menu.is_some() {
                app.close_menu();
            }
        }
        MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
            if is_in_terminal {
                let term_y = mouse.row.saturating_sub(term_area.y).saturating_sub(1) as usize;
                let term_x = mouse.column.saturating_sub(term_area.x).saturating_sub(1) as usize;

                let abs_y = term_y;

                if let Some((sel_start, _)) = app.terminal_sel {
                    app.terminal_sel = Some((sel_start, (abs_y, term_x)));
                } else {
                    app.terminal_sel = Some(((abs_y, term_x), (abs_y, term_x)));
                }

                copy_terminal_selection(app);
            } else if is_in_editor {
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
    let _ = preview_editor.open(path, &app.ts_manager);
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
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.toggle_menu(crate::app::TopBarMenu::Navigation);
                return Ok(());
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                app.toggle_menu(crate::app::TopBarMenu::Edit);
                return Ok(());
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                app.toggle_menu(crate::app::TopBarMenu::Files);
                return Ok(());
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                app.toggle_menu(crate::app::TopBarMenu::Panels);
                return Ok(());
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.toggle_menu(crate::app::TopBarMenu::Sidebar);
                return Ok(());
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                app.toggle_menu(crate::app::TopBarMenu::Code);
                return Ok(());
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                app.toggle_menu(crate::app::TopBarMenu::Help);
                return Ok(());
            }
            _ => {}
        }
    }

    if app.top_bar.active_menu.is_some() {
        match key.code {
            KeyCode::Esc => {
                app.close_menu();
                return Ok(());
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let items_len =
                    crate::ui::top_bar::get_menu_items(app.top_bar.active_menu.unwrap(), app).len();
                app.top_bar.selected_index = (app.top_bar.selected_index + 1) % items_len;
                return Ok(());
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let items_len =
                    crate::ui::top_bar::get_menu_items(app.top_bar.active_menu.unwrap(), app).len();
                if app.top_bar.selected_index == 0 {
                    app.top_bar.selected_index = items_len - 1;
                } else {
                    app.top_bar.selected_index -= 1;
                }
                return Ok(());
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let next = match app.top_bar.active_menu.unwrap() {
                    crate::app::TopBarMenu::Navigation => crate::app::TopBarMenu::Edit,
                    crate::app::TopBarMenu::Edit => crate::app::TopBarMenu::Files,
                    crate::app::TopBarMenu::Files => crate::app::TopBarMenu::Panels,
                    crate::app::TopBarMenu::Panels => crate::app::TopBarMenu::Sidebar,
                    crate::app::TopBarMenu::Sidebar => crate::app::TopBarMenu::Code,
                    crate::app::TopBarMenu::Code => crate::app::TopBarMenu::Help,
                    crate::app::TopBarMenu::Help => crate::app::TopBarMenu::Theme,
                    crate::app::TopBarMenu::Theme => crate::app::TopBarMenu::Navigation,
                };
                app.top_bar.active_menu = Some(next);
                app.top_bar.selected_index = 0;
                return Ok(());
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let prev = match app.top_bar.active_menu.unwrap() {
                    crate::app::TopBarMenu::Navigation => crate::app::TopBarMenu::Theme,
                    crate::app::TopBarMenu::Edit => crate::app::TopBarMenu::Navigation,
                    crate::app::TopBarMenu::Files => crate::app::TopBarMenu::Edit,
                    crate::app::TopBarMenu::Panels => crate::app::TopBarMenu::Files,
                    crate::app::TopBarMenu::Sidebar => crate::app::TopBarMenu::Panels,
                    crate::app::TopBarMenu::Code => crate::app::TopBarMenu::Sidebar,
                    crate::app::TopBarMenu::Help => crate::app::TopBarMenu::Code,
                    crate::app::TopBarMenu::Theme => crate::app::TopBarMenu::Help,
                };
                app.top_bar.active_menu = Some(prev);
                app.top_bar.selected_index = 0;
                return Ok(());
            }
            KeyCode::Enter => {
                app.execute_top_bar_action();
                return Ok(());
            }
            _ => {
                return Ok(());
            } // Block other keys while menu is open
        }
    }

    // 0. Handle g_mode
    if app.g_mode {
        app.g_mode = false;
        match key.code {
            KeyCode::Char('d') => {
                let _ = app.event_tx.send(klein_event::KleinEvent::GotoDefinition);
                return Ok(());
            }
            KeyCode::Char('h') => {
                app.trigger_hover();
                return Ok(());
            }
            KeyCode::Char('r') => {
                let _ = app.event_tx.send(klein_event::KleinEvent::FindReferences);
                return Ok(());
            }
            KeyCode::Char('f') => {
                let _ = app.event_tx.send(klein_event::KleinEvent::FormatDocument);
                return Ok(());
            }
            KeyCode::Char('n') => {
                app.trigger_rename();
                return Ok(());
            }
            KeyCode::Char('a') => {
                let _ = app.event_tx.send(klein_event::KleinEvent::CodeAction);
                return Ok(());
            }
            _ => {}
        }
    }

    if app.lsp_state.rename.is_some() && handle_rename_keys(app, key)? {
        return Ok(());
    }

    // 1. If completion popup is open, it gets first dibs
    if app.lsp_state.completion.is_some() && handle_completion_keys(app, key)? {
        return Ok(());
    }

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

                    if app.picker.mode == crate::search::SearchMode::CodeAction {
                        app.apply_code_action(app.picker.selected_index);
                        return Ok(());
                    }

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
                    crate::search::SearchMode::Lsp | crate::search::SearchMode::CodeAction => {}
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
                    crate::search::SearchMode::Lsp | crate::search::SearchMode::CodeAction => {}
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
    // Let Ctrl(+Shift)+Home/End and Ctrl+Shift+Left/Right fall through to editor handler
    let ctrl_nav_to_editor = key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(app.active_panel, Panel::Editor)
        && (matches!(key.code, KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown)
            || (key.modifiers.contains(KeyModifiers::SHIFT)
                && matches!(key.code, KeyCode::Left | KeyCode::Right)));
    if key.modifiers.contains(KeyModifiers::CONTROL) && !ctrl_nav_to_editor {
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
            KeyCode::Char('n') => {
                app.new_untitled_tab();
                return Ok(());
            }
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
                return Ok(());
            }
            KeyCode::Char('v') => {
                let h = app.last_editor_height.get();
                app.paste_clipboard(h);
                return Ok(());
            }
            KeyCode::Char('a') => {
                app.editor_mut().select_all();
                return Ok(());
            }
            KeyCode::Char('h') => {
                app.show_help = !app.show_help;
                return Ok(());
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
                if key.modifiers.contains(KeyModifiers::ALT) {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.editor_mut().move_block(true);
                        schedule_document_sync(app);
                        return Ok(());
                    }
                    app.editor_mut().shrink_selection();
                    return Ok(());
                }
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                let h = app.last_editor_height.get();
                app.editor_mut().move_cursor_down(h);
                app.lsp_state.hover = None;
                return Ok(());
            }
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.editor_mut().move_block(false);
                        schedule_document_sync(app);
                        return Ok(());
                    }
                    app.editor_mut().expand_selection();
                    return Ok(());
                }
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                app.editor_mut().move_cursor_up();
                app.lsp_state.hover = None;
                return Ok(());
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    app.editor_mut().swap_nodes(false);
                    schedule_document_sync(app);
                    return Ok(());
                }
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Word-left: skip whitespace then non-whitespace
                    let editor = app.editor_mut();
                    let line = editor.buffer.line(editor.cursor_y);
                    let text: String = line.chars().collect();
                    if editor.cursor_x == 0 {
                        if editor.cursor_y > 0 {
                            editor.cursor_y -= 1;
                            editor.cursor_x = editor.get_max_cursor_x(editor.cursor_y);
                        }
                    } else {
                        let mut x = editor.cursor_x;
                        let chars: Vec<char> = text.chars().collect();
                        // Skip whitespace backwards
                        while x > 0 && chars.get(x - 1).map_or(false, |c| c.is_whitespace()) {
                            x -= 1;
                        }
                        // Skip word chars backwards
                        while x > 0 && chars.get(x - 1).map_or(false, |c| !c.is_whitespace()) {
                            x -= 1;
                        }
                        editor.cursor_x = x;
                    }
                } else {
                    app.editor_mut().move_cursor_left();
                }
                app.lsp_state.hover = None;
                return Ok(());
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    app.editor_mut().swap_nodes(true);
                    schedule_document_sync(app);
                    return Ok(());
                }
                if is_selecting {
                    app.editor_mut().toggle_selection();
                } else {
                    app.editor_mut().clear_selection();
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Word-right: skip non-whitespace then whitespace
                    let editor = app.editor_mut();
                    let line = editor.buffer.line(editor.cursor_y);
                    let text: String = line.chars().collect();
                    let max_x = editor.get_max_cursor_x(editor.cursor_y);
                    if editor.cursor_x >= max_x {
                        let total_lines = editor.buffer.len_lines();
                        if editor.cursor_y + 1 < total_lines {
                            editor.cursor_y += 1;
                            editor.cursor_x = 0;
                        }
                    } else {
                        let mut x = editor.cursor_x;
                        let chars: Vec<char> = text.chars().collect();
                        // Skip word chars forward
                        while x < max_x && chars.get(x).map_or(false, |c| !c.is_whitespace()) {
                            x += 1;
                        }
                        // Skip whitespace forward
                        while x < max_x && chars.get(x).map_or(false, |c| c.is_whitespace()) {
                            x += 1;
                        }
                        editor.cursor_x = x;
                    }
                } else {
                    app.editor_mut().move_cursor_right();
                }
                app.lsp_state.hover = None;
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
                schedule_document_sync(app);
                return Ok(());
            }
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.cut_selection();
                schedule_document_sync(app);
                return Ok(());
            }
            KeyCode::Backspace => {
                app.editor_mut().delete_char();
                schedule_document_sync(app);
                app.lsp_state.completion = None;
                app.lsp_state.hover = None;
                return Ok(());
            }
            KeyCode::Null | KeyCode::Char(' ')
                if key.modifiers.contains(KeyModifiers::CONTROL) || key.code == KeyCode::Null =>
            {
                app.last_completion_trigger_char = None;
                schedule_completion(app);
                return Ok(());
            }
            KeyCode::Enter => {
                app.editor_mut().insert_char('\n');
                app.editor_mut().cursor_y += 1;
                app.editor_mut().cursor_x = 0;
                let h = app.last_editor_height.get();
                app.editor_mut().ensure_cursor_visible(h);
                schedule_document_sync(app);
                return Ok(());
            }
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::ALT) => {
                app.g_mode = true;
                return Ok(());
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                app.trigger_hover();
                return Ok(());
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::ALT) => {
                let _ = app.event_tx.send(klein_event::KleinEvent::FormatDocument);
                return Ok(());
            }
            KeyCode::Char(c) => {
                log::warn!("LSP: KeyCode::Char('{}') pressed", c);
                app.editor_mut().insert_char(c);
                schedule_document_sync(app);
                app.lsp_state.hover = None;
                if c == '.' || c == ':' || app.lsp_state.completion.is_some() {
                    log::warn!("LSP: Triggering schedule_completion for '{}'", c);
                    app.last_completion_trigger_char = Some(c);
                    schedule_completion(app);
                }
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
            if let Some(path) = app.sidebar.select_next() {
                load_preview(app, path);
            }
        }
        KeyCode::Up if matches!(app.active_panel, Panel::Sidebar) => {
            if let Some(path) = app.sidebar.select_previous() {
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

fn handle_completion_keys(app: &mut App, key: KeyEvent) -> io::Result<bool> {
    let mut state = match app.lsp_state.completion.take() {
        Some(s) => s,
        None => return Ok(false),
    };

    let mut handled = true;
    match key.code {
        KeyCode::Esc => {
            // Already taken out of state, so it closes.
        }
        KeyCode::Up => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            } else {
                state.selected_index = state.items.len().saturating_sub(1);
            }
            app.lsp_state.completion = Some(state);
        }
        KeyCode::Down => {
            if !state.items.is_empty() && state.selected_index < state.items.len() - 1 {
                state.selected_index += 1;
            } else {
                state.selected_index = 0;
            }
            app.lsp_state.completion = Some(state);
        }
        KeyCode::Enter | KeyCode::Tab => {
            if let Some(item) = state.items.get(state.selected_index) {
                let (start_char, end_char) = if let Some(range) = &item.replace_range {
                    let buffer = &app.editor().buffer;
                    let (sl, sc) = crate::lsp::router::from_lsp_position(&range.start, buffer);
                    let (el, ec) = crate::lsp::router::from_lsp_position(&range.end, buffer);

                    let start =
                        buffer.line_to_char(sl.min(buffer.len_lines().saturating_sub(1))) + sc;
                    let end =
                        buffer.line_to_char(el.min(buffer.len_lines().saturating_sub(1))) + ec;
                    (start, end)
                } else {
                    let buffer = &app.editor().buffer;
                    let cursor_y = app.editor().cursor_y;
                    let cursor_x = app.editor().cursor_x;

                    // Token-aware: find the start of the current word prefix
                    let line_idx = cursor_y.min(buffer.len_lines().saturating_sub(1));
                    let line = buffer.line(line_idx);
                    let mut start_col = cursor_x;

                    while start_col > 0 {
                        let ch = line.char(start_col - 1);
                        if ch.is_alphanumeric() || ch == '_' {
                            start_col -= 1;
                        } else {
                            break;
                        }
                    }

                    let start = buffer.line_to_char(line_idx) + start_col;
                    let end = buffer.line_to_char(line_idx) + cursor_x;
                    (start, end)
                };
                let buffer_len = app.editor().buffer.len_chars();
                app.editor_mut().replace_range(
                    start_char.min(buffer_len),
                    end_char.min(buffer_len),
                    &item.insert_text,
                );
                schedule_document_sync(app);
            }
        }
        _ => {
            app.lsp_state.completion = Some(state);
            handled = false;
        }
    }

    Ok(handled)
}
fn handle_rename_keys(app: &mut App, key: KeyEvent) -> io::Result<bool> {
    let mut state = match app.lsp_state.rename.take() {
        Some(s) => s,
        None => return Ok(false),
    };

    match key.code {
        KeyCode::Esc => {
            state.active = false;
            app.lsp_state.rename = None;
            return Ok(true);
        }
        KeyCode::Enter => {
            state.active = true; // Still active but we are sending
            app.lsp_state.rename = Some(state);
            let _ = app.event_tx.send(klein_event::KleinEvent::Rename);
            return Ok(true);
        }
        KeyCode::Backspace => {
            state.new_name.pop();
        }
        KeyCode::Char(c) => {
            state.new_name.push(c);
        }
        _ => {}
    }

    app.lsp_state.rename = Some(state);
    Ok(true)
}
