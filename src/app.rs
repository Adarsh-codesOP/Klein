use crate::editor::Editor;
use crate::events::klein_event::KleinEvent;
use crate::lsp::actor::LspServerNotification;
use crate::lsp::{LspManager, LspState};
use crate::sidebar::Sidebar;
use crate::tabs::TabState;
use crate::terminal::Terminal;
use std::cell::Cell;
use std::path::PathBuf;

pub enum Panel {
    Sidebar,
    Editor,
    Terminal,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SaveAsContext {
    SaveOnly,
    QuitAfter,
    CloseTabAfter,
    SwitchFileAfter(PathBuf),
}

pub struct SaveAsState {
    pub active: bool,
    pub filename: String,
    pub cur_dir: PathBuf,
    pub focus_filename: bool,
    pub context: SaveAsContext,
    pub is_edited: bool,
}

impl Default for SaveAsState {
    fn default() -> Self {
        Self {
            active: false,
            filename: String::new(),
            cur_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            focus_filename: true,
            context: SaveAsContext::SaveOnly,
            is_edited: false,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Maximized {
    None,
    Editor,
    Terminal,
}

pub struct App {
    pub active_panel: Panel,
    pub show_sidebar: bool,
    pub show_terminal: bool,
    pub should_quit: bool,
    pub sidebar: Sidebar,
    pub tabs: Vec<TabState>,
    pub active_tab: usize,
    pub preview: Option<Editor>,
    pub terminal: Terminal,
    pub last_editor_height: Cell<usize>,
    pub editor_area: Cell<ratatui::layout::Rect>,
    pub show_help: bool,
    pub help_scroll: usize,
    pub terminal_scroll: usize,
    pub terminal_restarting: bool,
    pub terminal_area: Cell<ratatui::layout::Rect>,
    pub terminal_sel: Option<((usize, usize), (usize, usize))>,
    pub show_quit_confirm: bool,
    pub show_unsaved_confirm: bool,
    pub show_create_file_prompt: bool,
    pub pending_open_path: Option<PathBuf>,
    pub maximized: Maximized,
    pub save_as_state: SaveAsState,
    pub picker: crate::search::PickerState,
    pub clipboard: Option<arboard::Clipboard>,
    pub lsp_state: LspState,
    pub lsp_notification_tx: tokio::sync::mpsc::UnboundedSender<LspServerNotification>,
    pub lsp_manager: LspManager,
    pub timer_manager: Option<crate::events::timers::TimerManager>,
    pub event_tx: tokio::sync::mpsc::UnboundedSender<KleinEvent>,
    pub g_mode: bool,
    pub code_actions: Vec<lsp_types::CodeActionOrCommand>,
    pub ts_manager: crate::treesitter::TSManager,
}

impl App {
    pub fn new(
        cli_file: Option<PathBuf>,
        clipboard: Option<arboard::Clipboard>,
        lsp_notification_tx: tokio::sync::mpsc::UnboundedSender<LspServerNotification>,
        event_tx: tokio::sync::mpsc::UnboundedSender<KleinEvent>,
    ) -> App {
        let config = crate::config::AppConfig::load();

        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        let mut app = App {
            active_panel: Panel::Sidebar,
            show_sidebar: true,
            show_terminal: true,
            should_quit: false,
            sidebar: Sidebar::new(&current_dir),
            tabs: vec![TabState::new()],
            active_tab: 0,
            preview: None,
            terminal: Terminal::new(current_dir.clone(), config.shell.clone()),
            last_editor_height: Cell::new(20),
            editor_area: Cell::new(ratatui::layout::Rect::default()),
            show_help: false,
            help_scroll: 0,
            terminal_scroll: 0,
            terminal_restarting: false,
            terminal_area: Cell::new(ratatui::layout::Rect::default()),
            terminal_sel: None,
            show_quit_confirm: false,
            show_unsaved_confirm: false,
            show_create_file_prompt: false,
            pending_open_path: None,
            maximized: Maximized::None,
            save_as_state: SaveAsState {
                cur_dir: current_dir.clone(),
                ..Default::default()
            },
            picker: crate::search::PickerState::default(),
            clipboard,
            lsp_state: LspState::default(),
            lsp_manager: LspManager::new(lsp_notification_tx.clone(), &config),
            lsp_notification_tx,
            timer_manager: None,
            event_tx,
            g_mode: false,
            code_actions: Vec::new(),
            ts_manager: crate::treesitter::TSManager::new(),
        };

        if let Some(file) = cli_file {
            let path = current_dir.join(&file);
            if path.exists() {
                app.open_in_current_tab(path);
                app.active_panel = Panel::Editor;
            } else {
                app.pending_open_path = Some(path);
                app.show_create_file_prompt = true;
            }
        }

        app
    }

    /// Get a reference to the editor that should be displayed.
    /// Returns preview editor when sidebar is focused and preview exists,
    /// otherwise returns the active tab's editor.
    pub fn active_editor(&self) -> &Editor {
        if matches!(self.active_panel, Panel::Sidebar) {
            if let Some(preview) = &self.preview {
                return preview;
            }
        }
        self.editor()
    }

    /// Get a reference to the current tab's editor
    pub fn editor(&self) -> &Editor {
        &self.tabs[self.active_tab].editor
    }

    /// Get a mutable reference to the current tab's editor
    pub fn editor_mut(&mut self) -> &mut Editor {
        &mut self.tabs[self.active_tab].editor
    }

    pub fn copy_selection(&mut self) {
        let mut cb = self.clipboard.take();
        self.editor_mut().copy(&mut cb);
        self.clipboard = cb;
    }

    pub fn cut_selection(&mut self) {
        let mut cb = self.clipboard.take();
        self.editor_mut().cut(&mut cb);
        self.clipboard = cb;
    }

    pub fn paste_clipboard(&mut self, height: usize) {
        let mut cb = self.clipboard.take();
        self.editor_mut().paste(&mut cb, height);
        self.clipboard = cb;
    }

    pub fn insert_paste(&mut self, text: &str, height: usize) {
        self.editor_mut().insert_paste(text, height);
    }

    /// Find a tab with the given path
    pub fn find_tab_by_path(&self, path: &std::path::Path) -> Option<usize> {
        self.tabs
            .iter()
            .position(|t| t.editor.path.as_ref().map(|p| p == path).unwrap_or(false))
    }

    /// Open a file: switch to it if open, otherwise open in new tab
    pub fn open_file(&mut self, path: PathBuf) {
        if let Some(idx) = self.find_tab_by_path(&path) {
            self.active_tab = idx;
        } else {
            self.open_in_new_tab(path);
        }
    }

    pub fn jump_to_location(&mut self, path: PathBuf, line: usize, col: usize) {
        self.open_file(path);
        let h = self.last_editor_height.get();
        let editor = self.editor_mut();
        editor.cursor_y = line;
        editor.cursor_x = col;
        editor.ensure_cursor_visible(h);
    }

    pub fn open_in_new_tab(&mut self, path: PathBuf) {
        let mut tab = TabState::new();
        let _ = tab.editor.open(path, &self.ts_manager);
        self.lsp_did_open_for_editor(&tab.editor);
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    #[allow(dead_code)]
    pub fn open_in_current_tab(&mut self, path: PathBuf) {
        let _ = self.tabs[self.active_tab]
            .editor
            .open(path, &self.ts_manager);
        let p = self.tabs[self.active_tab].editor.path.clone();
        if let Some(path) = p {
            let _ = self.event_tx.send(crate::events::klein_event::KleinEvent::InitLsp(path.clone()));
        }
    }

    /// Switch to the next tab (wraps around)
    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn close_tab(&mut self) {
        // Send didClose before removing
        if let Some(path) = self.tabs[self.active_tab].editor.path.clone() {
            self.lsp_manager.notify_did_close(&path);
        }

        if self.tabs.len() == 1 {
            self.tabs[0] = TabState::new();
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn save_current_file(&mut self) {
        let active_tab = self.active_tab;
        let tab = &mut self.tabs[active_tab];
        if tab.editor.path.is_some() {
            let _ = tab.editor.save();
            if let Some(path) = tab.editor.path.clone() {
                self.lsp_manager.notify_did_save(&path);
            }
        }
    }

    pub fn try_save_or_show_save_as(&mut self, context: SaveAsContext) -> bool {
        let tab = &mut self.tabs[self.active_tab];
        if tab.editor.path.is_some() {
            self.save_current_file();
            true
        } else {
            self.save_as_state.active = true;
            self.save_as_state.context = context;

            let now = chrono::Local::now();
            let proposed_name = now.format("untitled-%d-%m-%y-%H%M%S.txt").to_string();

            self.save_as_state.filename = proposed_name;
            self.save_as_state.focus_filename = true;
            self.save_as_state.is_edited = false;
            false
        }
    }

    pub fn execute_save_as(&mut self) {
        let path = self
            .save_as_state
            .cur_dir
            .join(&self.save_as_state.filename);
        let tab = &mut self.tabs[self.active_tab];
        tab.editor.path = Some(path);
        self.save_current_file();
        self.save_as_state.active = false;

        self.sidebar.refresh();

        match self.save_as_state.context.clone() {
            SaveAsContext::QuitAfter => {
                self.should_quit = true;
            }
            SaveAsContext::CloseTabAfter => {
                self.close_tab();
            }
            SaveAsContext::SwitchFileAfter(path) => {
                self.open_in_new_tab(path);
                self.active_panel = Panel::Editor;
            }
            SaveAsContext::SaveOnly => {}
        }
    }

    // ─── LSP helpers ───────────────────────────────────────────────

    fn lsp_did_open_for_editor(&mut self, editor: &Editor) {
        if let Some(path) = &editor.path {
            // Trigger server start
            let _ = self.event_tx.send(KleinEvent::InitLsp(path.clone()));
        }
    }

    pub fn notify_lsp_did_change(&mut self) {
        let (path, content) = {
            let editor = &self.tabs[self.active_tab].editor;
            if let Some(path) = editor.path.clone() {
                (path, editor.buffer.to_string())
            } else {
                return;
            }
        };
        self.lsp_manager.notify_did_change(&path, &content);
    }

    pub fn notify_lsp_did_open_for_path(&mut self, path: &std::path::Path) {
        log::warn!("LSP: notify_lsp_did_open_for_path called for {}", path.display());
        // Find if this path is open in any tab
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(p) = &tab.editor.path {
                log::warn!("LSP: Checking tab {}, tab path: {}", i, p.display());
                if p == path {
                    let content = tab.editor.buffer.to_string();
                    log::warn!("LSP: Found matching tab {}, sending didOpen", i);
                    self.lsp_manager.notify_did_open(path, &content);
                    return;
                }
            } else {
                log::warn!("LSP: Tab {} has no path", i);
            }
        }
        log::warn!("LSP: No matching tab found for {}", path.display());
    }

    pub fn trigger_completion(&mut self) {
        let (path, line, col, buffer) = {
            let editor = &self.tabs[self.active_tab].editor;
            match &editor.path {
                Some(p) => (
                    p.clone(),
                    editor.cursor_y,
                    editor.cursor_x,
                    editor.buffer.clone(),
                ),
                None => return,
            }
        };

        log::warn!("LSP: requesting completions for {} at Ln {}, Col {}", path.display(), line + 1, col + 1);
        
        let handle = match self.lsp_manager.server_handle_for_file(&path) {
            Some(h) => h.clone(),
            None => return,
        };
        let caps = match self.lsp_manager.get_capabilities(&path) {
            Some(c) => c,
            None => return,
        };
        if !caps.completion { return; }

        let params = match self.lsp_manager.text_doc_position(&path, line, col, &buffer) {
            Some(p) => p,
            None => return,
        };

        let mut params_obj = params.as_object().unwrap().clone();
        params_obj.insert(
            "context".to_string(),
            serde_json::json!({
                "triggerKind": 1, // Invoked
            }),
        );
        let params_with_context = serde_json::Value::Object(params_obj);

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            let response = handle.send_request("textDocument/completion", params_with_context).await.ok();
            let _ = tx.send(crate::events::klein_event::KleinEvent::CompletionResponse(response, path, (line, col)));
        });
    }

    pub fn handle_completion_response(&mut self, response: Option<serde_json::Value>, _path: std::path::PathBuf, trigger_position: (usize, usize)) {
        let response = match response {
            Some(r) => r,
            None => {
                self.lsp_state.completion = None;
                return;
            }
        };

        // Parse response (can be Array or List or null)
        let items: Vec<crate::lsp::types::KleinCompletion> =
            match serde_json::from_value::<Option<lsp_types::CompletionResponse>>(response.clone()) {
                Ok(Some(lsp_types::CompletionResponse::Array(arr))) => arr
                    .into_iter()
                    .map(|i| crate::lsp::router::to_klein_completion(&i))
                    .collect(),
                Ok(Some(lsp_types::CompletionResponse::List(list))) => list
                    .items
                    .into_iter()
                    .map(|i| crate::lsp::router::to_klein_completion(&i))
                    .collect(),
                Ok(None) => Vec::new(),
                Err(e) => {
                    log::error!("failed to parse completion response: {}. Raw response: {}", e, response);
                    Vec::new()
                }
            };

        let items: Vec<crate::lsp::types::KleinCompletion> = if !items.is_empty() {
            let editor = self.editor();
            let (line_idx, col_idx) = (editor.cursor_y, editor.cursor_x);
            let (start_line, start_col) = trigger_position;
            
            if start_line != line_idx || start_col > col_idx {
                items
            } else {
                let line_slice = editor.buffer.line(line_idx);
                let prefix = if start_col < col_idx {
                    line_slice.slice(start_col..col_idx).to_string()
                } else {
                    "".to_string()
                };
                crate::search::fuzzy_filter(&prefix, items)
            }
        } else {
            Vec::new()
        };

        if !items.is_empty() {
            log::warn!("LSP: received {} filtered completion items", items.len());
            self.lsp_state.completion = Some(crate::lsp::types::CompletionState {
                items,
                selected_index: 0,
                scroll: 0,
                trigger_position,
            });
        } else {
            log::warn!("LSP: no completion items found");
            self.lsp_state.completion = None;
        }
    }

    pub fn trigger_hover(&mut self) {
        let (path, line, col, buffer) = {
            let editor = &self.tabs[self.active_tab].editor;
            match &editor.path {
                Some(p) => (
                    p.clone(),
                    editor.cursor_y,
                    editor.cursor_x,
                    editor.buffer.clone(),
                ),
                None => return,
            }
        };

        let handle = match self.lsp_manager.server_handle_for_file(&path) {
            Some(h) => h.clone(),
            None => return,
        };
        let caps = match self.lsp_manager.get_capabilities(&path) {
            Some(c) => c,
            None => return,
        };
        if !caps.hover { return; }

        let params = match self.lsp_manager.text_doc_position(&path, line, col, &buffer) {
            Some(p) => p,
            None => return,
        };

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            let response = handle.send_request("textDocument/hover", params).await.ok();
            let _ = tx.send(crate::events::klein_event::KleinEvent::HoverResponse(response, path, (line, col)));
        });
    }

    pub fn handle_hover_response(&mut self, response: Option<serde_json::Value>, _path: std::path::PathBuf, _pos: (usize, usize)) {
        let response = match response {
            Some(r) => r,
            None => {
                self.lsp_state.hover = None;
                return;
            }
        };
        // Parse response (can be Hover or null)
        match serde_json::from_value::<Option<lsp_types::Hover>>(response.clone()) {
            Ok(Some(hover)) => {
                let contents = match hover.contents {
                    lsp_types::HoverContents::Scalar(m) => match m {
                        lsp_types::MarkedString::String(s) => s,
                        lsp_types::MarkedString::LanguageString(ls) => ls.value,
                    },
                    lsp_types::HoverContents::Array(arr) => arr
                        .into_iter()
                        .map(|m| match m {
                            lsp_types::MarkedString::String(s) => s,
                            lsp_types::MarkedString::LanguageString(ls) => ls.value,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    lsp_types::HoverContents::Markup(m) => m.value,
                };

                if !contents.is_empty() {
                    self.lsp_state.hover = Some(crate::lsp::types::KleinHoverInfo {
                        contents,
                        range: hover.range.map(|r| (r.start.line as usize, r.start.character as usize, r.end.line as usize, r.end.character as usize)),
                    });
                } else {
                    self.lsp_state.hover = None;
                }
            }
            Ok(None) => {
                self.lsp_state.hover = None;
            }
            Err(e) => {
                log::error!("failed to parse hover response: {}. Raw response: {}", e, response);
                self.lsp_state.hover = None;
            }
        }
    }


    pub fn trigger_goto_definition(&mut self) {
        let (path, line, col, buffer) = {
            let editor = &self.tabs[self.active_tab].editor;
            match &editor.path {
                Some(p) => (
                    p.clone(),
                    editor.cursor_y,
                    editor.cursor_x,
                    editor.buffer.clone(),
                ),
                None => return,
            }
        };

        let handle = match self.lsp_manager.server_handle_for_file(&path) {
            Some(h) => h.clone(),
            None => return,
        };
        let caps = match self.lsp_manager.get_capabilities(&path) {
            Some(c) => c,
            None => return,
        };
        if !caps.definition { return; }

        let params = match self.lsp_manager.text_doc_position(&path, line, col, &buffer) {
            Some(p) => p,
            None => return,
        };

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            let response = handle.send_request("textDocument/definition", params).await.ok();
            let _ = tx.send(crate::events::klein_event::KleinEvent::DefinitionResponse(response, path));
        });
    }

    pub fn handle_definition_response(&mut self, response: Option<serde_json::Value>, _path: std::path::PathBuf) {
        let resp = match response {
            Some(r) => r,
            None => return,
        };

        let loc = match serde_json::from_value::<Option<lsp_types::GotoDefinitionResponse>>(resp.clone()) {
            Ok(Some(lsp_types::GotoDefinitionResponse::Scalar(l))) => Some(l),
            Ok(Some(lsp_types::GotoDefinitionResponse::Array(a))) => a.into_iter().next(),
            Ok(Some(lsp_types::GotoDefinitionResponse::Link(l))) => {
                l.into_iter().next().map(|link| lsp_types::Location {
                    uri: link.target_uri,
                    range: link.target_range,
                })
            }
            Ok(None) => None,
            Err(e) => {
                log::error!("failed to parse definition response: {}. Raw response: {}", e, resp);
                None
            }
        };

        if let Some(loc) = loc {
            if let Some(target_path) = crate::lsp::router::uri_to_path(&loc.uri) {
                let target_line = loc.range.start.line as usize;
                let target_col = if let Some(idx) = self.find_tab_by_path(&target_path) {
                    let (_, col) = crate::lsp::router::from_lsp_position(
                        &loc.range.start,
                        &self.tabs[idx].editor.buffer,
                    );
                    col
                } else {
                    loc.range.start.character as usize
                };
                self.jump_to_location(target_path, target_line, target_col);
            }
        }
    }


    pub fn trigger_find_references(&mut self) {
        let (path, line, col, buffer) = {
            let editor = &self.tabs[self.active_tab].editor;
            match &editor.path {
                Some(p) => (
                    p.clone(),
                    editor.cursor_y,
                    editor.cursor_x,
                    editor.buffer.clone(),
                ),
                None => return,
            }
        };

        let handle = match self.lsp_manager.server_handle_for_file(&path) {
            Some(h) => h.clone(),
            None => return,
        };
        let caps = match self.lsp_manager.get_capabilities(&path) {
            Some(c) => c,
            None => return,
        };
        if !caps.references { return; }

        let params = match self.lsp_manager.text_doc_position(&path, line, col, &buffer) {
            Some(p) => p,
            None => return,
        };

        let mut params_obj = params.as_object().unwrap().clone();
        params_obj.insert(
            "context".to_string(),
            serde_json::json!({ "includeDeclaration": true }),
        );
        let params_with_context = serde_json::Value::Object(params_obj);

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            let response = handle.send_request("textDocument/references", params_with_context).await.ok();
            let _ = tx.send(crate::events::klein_event::KleinEvent::ReferencesResponse(response, path));
        });
    }

    pub fn handle_references_response(&mut self, response: Option<serde_json::Value>, _path: std::path::PathBuf) {
        let resp = match response {
            Some(r) => r,
            None => return,
        };

        let locs: Vec<lsp_types::Location> = serde_json::from_value(resp).unwrap_or_default();

        if locs.is_empty() {
            return;
        }

        let mut results = Vec::new();
        for loc in locs {
            if let Some(p) = crate::lsp::router::uri_to_path(&loc.uri) {
                results.push(crate::search::SearchResult {
                    path: p,
                    line: Some(loc.range.start.line as usize),
                    content: None, // Could fetch line content for preview
                });
            }
        }

        if !results.is_empty() {
            self.picker.active = true;
            self.picker.mode = crate::search::SearchMode::Lsp;
            self.picker.results = results;
            self.picker.query = "References".to_string();
            self.picker.selected_index = 0;
        }
    }

    pub fn trigger_format_document(&mut self) {
        let path = {
            let editor = &self.tabs[self.active_tab].editor;
            match &editor.path {
                Some(p) => p.clone(),
                None => return,
            }
        };

        let handle = match self.lsp_manager.server_handle_for_file(&path) {
            Some(h) => h.clone(),
            None => return,
        };
        let caps = match self.lsp_manager.get_capabilities(&path) {
            Some(c) => c,
            None => return,
        };
        if !caps.formatting { return; }

        let uri = match crate::lsp::router::path_to_uri(&path) {
            Some(u) => u,
            None => return,
        };
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        });

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            let response = handle.send_request("textDocument/formatting", params).await.ok();
            let _ = tx.send(crate::events::klein_event::KleinEvent::FormatResponse(response, path));
        });
    }

    pub fn handle_format_response(&mut self, response: Option<serde_json::Value>, path: std::path::PathBuf) {
        let resp = match response {
            Some(r) => r,
            None => return,
        };
        let edits: Vec<lsp_types::TextEdit> = serde_json::from_value(resp).unwrap_or_default();
        if edits.is_empty() { return; }

        let mut sorted_edits = edits;
        sorted_edits.sort_by(|a, b| b.range.start.cmp(&a.range.start));

        if let Some(tab_idx) = self.find_tab_by_path(&path) {
            let editor = &mut self.tabs[tab_idx].editor;
            editor.save_undo_state();
            for edit in sorted_edits {
                let (start_line, start_col) =
                    crate::lsp::router::from_lsp_position(&edit.range.start, &editor.buffer);
                let (end_line, end_col) =
                    crate::lsp::router::from_lsp_position(&edit.range.end, &editor.buffer);

                let start_char = editor.buffer.line_to_char(start_line) + start_col;
                let end_char = editor.buffer.line_to_char(end_line) + end_col;

                if start_char <= end_char && end_char <= editor.buffer.len_chars() {
                    editor.buffer.remove(start_char..end_char);
                    editor.buffer.insert(start_char, &edit.new_text);
                }
            }
            editor.is_dirty = true;
        }
    }

    pub fn trigger_rename(&mut self) {
        let editor = self.editor();
        let path = match &editor.path {
            Some(p) => p.clone(),
            None => return,
        };

        self.lsp_state.rename = Some(crate::lsp::types::RenameState {
            trigger_position: (editor.cursor_y, editor.cursor_x),
            path,
            new_name: String::new(),
            active: true,
        });
    }

    pub fn execute_rename(&mut self) {
        let state = match self.lsp_state.rename.take() {
            Some(s) if s.active && !s.new_name.is_empty() => s,
            _ => return,
        };

        let buffer = match self.find_tab_by_path(&state.path) {
            Some(idx) => self.tabs[idx].editor.buffer.clone(),
            None => return, // Should not happen if active
        };

        let handle = match self.lsp_manager.server_handle_for_file(&state.path) {
            Some(h) => h.clone(),
            None => return,
        };
        let caps = match self.lsp_manager.get_capabilities(&state.path) {
            Some(c) => c,
            None => return,
        };
        if !caps.rename { return; }

        let params = match self.lsp_manager.text_doc_position(&state.path, state.trigger_position.0, state.trigger_position.1, &buffer) {
            Some(p) => p,
            None => return,
        };

        let mut params_obj = params.as_object().unwrap().clone();
        params_obj.insert("newName".to_string(), serde_json::Value::String(state.new_name.clone()));
        let params_with_name = serde_json::Value::Object(params_obj);

        let tx = self.event_tx.clone();
        let path_clone = state.path.clone();
        tokio::spawn(async move {
            let response = handle.send_request("textDocument/rename", params_with_name).await.ok();
            let _ = tx.send(crate::events::klein_event::KleinEvent::RenameResponse(response, path_clone, state.new_name));
        });
    }

    pub fn handle_rename_response(&mut self, response: Option<serde_json::Value>, _path: std::path::PathBuf, _new_name: String) {
        let resp = match response {
            Some(r) => r,
            None => return,
        };

        let edit = match serde_json::from_value::<Option<lsp_types::WorkspaceEdit>>(resp.clone()) {
            Ok(Some(v)) => v,
            Ok(None) => return,
            Err(e) => {
                log::error!("failed to parse rename response: {}. Raw response: {}", e, resp);
                return;
            }
        };

        self.apply_workspace_edit(edit);
    }

    fn apply_workspace_edit(&mut self, edit: lsp_types::WorkspaceEdit) {
        if let Some(changes) = edit.changes {
            for (uri, edits) in changes {
                if let Some(path) = crate::lsp::router::uri_to_path(&uri) {
                    self.apply_workspace_edits_to_file(path, edits);
                }
            }
        } else if let Some(doc_changes) = edit.document_changes {
            match doc_changes {
                lsp_types::DocumentChanges::Edits(edits) => {
                    for edit in edits {
                        if let Some(path) = crate::lsp::router::uri_to_path(&edit.text_document.uri) {
                            let text_edits = edit.edits.into_iter().map(|e| match e {
                                lsp_types::OneOf::Left(te) => te,
                                lsp_types::OneOf::Right(ae) => ae.text_edit,
                            }).collect();
                            self.apply_workspace_edits_to_file(path, text_edits);
                        }
                    }
                }
                lsp_types::DocumentChanges::Operations(_) => {
                    log::warn!("Workspace file operations are not supported yet");
                }
            }
        }
    }

    fn apply_workspace_edits_to_file(
        &mut self,
        path: PathBuf,
        mut edits: Vec<lsp_types::TextEdit>,
    ) {
        // Sort in reverse
        edits.sort_by(|a, b| b.range.start.cmp(&a.range.start));

        if let Some(tab_idx) = self.find_tab_by_path(&path) {
            let editor = &mut self.tabs[tab_idx].editor;
            editor.save_undo_state();
            for edit in edits {
                let (start_line, start_col) =
                    crate::lsp::router::from_lsp_position(&edit.range.start, &editor.buffer);
                let (end_line, end_col) =
                    crate::lsp::router::from_lsp_position(&edit.range.end, &editor.buffer);
                let start_char = editor.buffer.line_to_char(start_line) + start_col;
                let end_char = editor.buffer.line_to_char(end_line) + end_col;
                if start_char <= end_char && end_char <= editor.buffer.len_chars() {
                    editor.buffer.remove(start_char..end_char);
                    editor.buffer.insert(start_char, &edit.new_text);
                }
            }
            editor.is_dirty = true;
        } else {
            // File not open, apply to disk
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut rope = ropey::Rope::from_str(&content);
                for edit in edits {
                    let (start_line, start_col) =
                        crate::lsp::router::from_lsp_position(&edit.range.start, &rope);
                    let (end_line, end_col) =
                        crate::lsp::router::from_lsp_position(&edit.range.end, &rope);
                    let start_char = rope.line_to_char(start_line) + start_col;
                    let end_char = rope.line_to_char(end_line) + end_col;
                    if start_char <= end_char && end_char <= rope.len_chars() {
                        rope.remove(start_char..end_char);
                        rope.insert(start_char, &edit.new_text);
                    }
                }
                let _ = std::fs::write(&path, rope.to_string());
            }
        }
    }

    pub fn trigger_code_action(&mut self) {
        let (path, line, col, buffer) = {
            let editor = &self.tabs[self.active_tab].editor;
            match &editor.path {
                Some(p) => (
                    p.clone(),
                    editor.cursor_y,
                    editor.cursor_x,
                    editor.buffer.clone(),
                ),
                None => return,
            }
        };

        let handle = match self.lsp_manager.server_handle_for_file(&path) {
            Some(h) => h.clone(),
            None => return,
        };
        let caps = match self.lsp_manager.get_capabilities(&path) {
            Some(c) => c,
            None => return,
        };
        if !caps.code_action { return; }

        let params = match self.lsp_manager.text_doc_position(&path, line, col, &buffer) {
            Some(p) => p,
            None => return,
        };

        let mut params_obj = params.as_object().unwrap().clone();
        let range = params_obj.get("position").unwrap().clone();
        params_obj.insert(
            "range".to_string(),
            serde_json::json!({
                "start": range,
                "end": range,
            }),
        );
        params_obj.remove("position");
        params_obj.insert(
            "context".to_string(),
            serde_json::json!({
                "diagnostics": []
            }),
        );
        let action_params = serde_json::Value::Object(params_obj);

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            let response = handle.send_request("textDocument/codeAction", action_params).await.ok();
            let _ = tx.send(crate::events::klein_event::KleinEvent::CodeActionResponse(response, path, (line, col)));
        });
    }

    pub fn handle_code_action_response(&mut self, response: Option<serde_json::Value>, path: std::path::PathBuf, pos: (usize, usize)) {
        let resp = match response {
            Some(r) => r,
            None => return,
        };

        let actions: Vec<lsp_types::CodeActionOrCommand> =
            serde_json::from_value(resp).unwrap_or_default();

        if actions.is_empty() {
            return;
        }

        self.code_actions = actions;
        let mut results = Vec::new();
        for action in &self.code_actions {
            let title = match action {
                lsp_types::CodeActionOrCommand::Command(c) => &c.title,
                lsp_types::CodeActionOrCommand::CodeAction(a) => &a.title,
            };
            results.push(crate::search::SearchResult {
                path: path.clone(),
                line: Some(pos.0),
                content: Some(title.to_string()),
            });
        }

        self.picker.active = true;
        self.picker.mode = crate::search::SearchMode::CodeAction;
        self.picker.results = results;
        self.picker.query = "Code Actions".to_string();
        self.picker.selected_index = 0;
    }
    pub fn apply_code_action(&mut self, index: usize) {
        let action = match self.code_actions.get(index) {
            Some(a) => a.clone(),
            None => return,
        };

        match action {
            lsp_types::CodeActionOrCommand::CodeAction(a) => {
                if let Some(edit) = &a.edit {
                    self.apply_workspace_edit(edit.clone());
                }
            }
            lsp_types::CodeActionOrCommand::Command(c) => {
                log::warn!("Executing LSP command '{}' is not supported yet", c.command);
            }
        }
    }
}
