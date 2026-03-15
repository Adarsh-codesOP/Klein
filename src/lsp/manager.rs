use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use super::actor::{ActorHandle, LspServerNotification, SpawnedActor};
use super::capabilities::LspFeatureFlags;
use super::doc_sync::DocSyncEngine;
use super::registry::{LspRegistry, ServerConfig};
use super::router;

/// High-level orchestrator for all LSP server connections.
///
/// Owns the registry, active server handles, document sync state, and
/// provides the public API that the rest of Klein calls.
pub struct LspManager {
    registry: LspRegistry,
    servers: HashMap<String, ServerState>,
    doc_sync: DocSyncEngine,
    event_tx: mpsc::UnboundedSender<LspServerNotification>,
}

struct ServerState {
    handle: ActorHandle,
    capabilities: LspFeatureFlags,
    _root_uri: url::Url,
    #[allow(dead_code)]
    join_handle: tokio::task::JoinHandle<()>,
}

impl LspManager {
    pub fn new(
        event_tx: mpsc::UnboundedSender<LspServerNotification>,
        config: &crate::config::AppConfig,
    ) -> Self {
        Self {
            registry: LspRegistry::new(config.enabled_lsps.as_ref()),
            servers: HashMap::new(),
            doc_sync: DocSyncEngine::new(),
            event_tx,
        }
    }

    /// Ensure a language server is running for the given file.
    /// Starts the server lazily if not already running.
    /// Returns the language ID if a server is available.
    pub async fn ensure_server_for_file(&mut self, path: &Path) -> Option<String> {
        let config = self.registry.find_server_for_file(path)?.clone();
        let lang_id = config.language_id.clone();

        if self.servers.contains_key(&lang_id) {
            return Some(lang_id);
        }

        match self.start_server(&config, path).await {
            Ok(()) => Some(lang_id),
            Err(e) => {
                log::error!("failed to start {} server: {}", lang_id, e);
                None
            }
        }
    }

    async fn start_server(
        &mut self,
        config: &ServerConfig,
        file_path: &Path,
    ) -> Result<(), String> {
        let root_dir = detect_project_root(file_path, &config.root_markers);
        let root_uri = router::path_to_uri(&root_dir)
            .ok_or_else(|| "failed to convert root path to URI".to_string())?;

        let spawned: SpawnedActor = super::actor::spawn_actor(
            &config.command,
            &config.args,
            &root_dir,
            &config.language_id,
            self.event_tx.clone(),
        )?;

        // Initialize handshake
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri.as_str(),
            "capabilities": client_capabilities(),
            "clientInfo": {
                "name": "Klein",
                "version": env!("CARGO_PKG_VERSION"),
            },
        });

        let result = spawned
            .handle
            .send_request("initialize", init_params)
            .await?;

        // Extract server capabilities
        let server_caps: lsp_types::ServerCapabilities =
            serde_json::from_value(result.get("capabilities").cloned().unwrap_or_default())
                .unwrap_or_default();

        let flags = LspFeatureFlags::from_capabilities(&server_caps);
        log::info!(
            "[{}] initialized — hover:{} completion:{} definition:{} references:{} formatting:{} rename:{}",
            config.language_id,
            flags.hover, flags.completion, flags.definition,
            flags.references, flags.formatting, flags.rename,
        );

        // Send initialized notification AFTER successful initialization response
        // This is a critical step in the LSP handshake
        spawned
            .handle
            .send_notification("initialized", serde_json::json!({}))?;

        self.servers.insert(
            config.language_id.clone(),
            ServerState {
                handle: spawned.handle,
                capabilities: flags,
                _root_uri: root_uri,
                join_handle: spawned.join_handle,
            },
        );

        Ok(())
    }

    pub fn notify_did_open(&mut self, path: &Path, content: &str) {
        let lang_id = match self.registry.language_id_for_file(path) {
            Some(id) => id.to_string(),
            None => return,
        };

        if self.doc_sync.is_open(path) {
            log::warn!(
                "LSP: document {} is already open in doc_sync, skipping didOpen",
                path.display()
            );
            return;
        }

        let (language_id, version) = self.doc_sync.open_document(path, &lang_id);
        let uri = match router::path_to_uri(path) {
            Some(u) => u,
            None => {
                log::error!("LSP: failed to convert path to URI: {}", path.display());
                return;
            }
        };

        log::warn!(
            "LSP: sending textDocument/didOpen for {} (lang: {})",
            path.display(),
            language_id
        );

        if let Some(server) = self.servers.get(&lang_id) {
            let _ = server.handle.send_notification(
                "textDocument/didOpen",
                serde_json::json!({
                    "textDocument": {
                        "uri": uri.as_str(),
                        "languageId": language_id,
                        "version": version,
                        "text": content,
                    }
                }),
            );
        }
    }

    pub fn notify_did_change(&mut self, path: &Path, content: &str) {
        let lang_id = match self.registry.language_id_for_file(path) {
            Some(id) => id.to_string(),
            None => return,
        };

        let version = match self.doc_sync.change_document(path) {
            Some(v) => v,
            None => return,
        };

        let uri = match router::path_to_uri(path) {
            Some(u) => u,
            None => return,
        };

        if let Some(server) = self.servers.get(&lang_id) {
            let _ = server.handle.send_notification(
                "textDocument/didChange",
                serde_json::json!({
                    "textDocument": {
                        "uri": uri.as_str(),
                        "version": version,
                    },
                    "contentChanges": [{
                        "text": content,
                    }],
                }),
            );
        }
    }

    pub fn notify_did_save(&mut self, path: &Path) {
        let lang_id = match self.registry.language_id_for_file(path) {
            Some(id) => id.to_string(),
            None => return,
        };

        let uri = match router::path_to_uri(path) {
            Some(u) => u,
            None => return,
        };

        if let Some(server) = self.servers.get(&lang_id) {
            let _ = server.handle.send_notification(
                "textDocument/didSave",
                serde_json::json!({
                    "textDocument": { "uri": uri.as_str() }
                }),
            );
        }
    }

    pub fn notify_did_close(&mut self, path: &Path) {
        let lang_id = match self.registry.language_id_for_file(path) {
            Some(id) => id.to_string(),
            None => return,
        };

        if !self.doc_sync.close_document(path) {
            return;
        }

        let uri = match router::path_to_uri(path) {
            Some(u) => u,
            None => return,
        };

        if let Some(server) = self.servers.get(&lang_id) {
            let _ = server.handle.send_notification(
                "textDocument/didClose",
                serde_json::json!({
                    "textDocument": { "uri": uri.as_str() }
                }),
            );
        }
    }

    // ─── LSP feature requests ──────────────────────────────────────

    pub fn get_capabilities(&self, path: &Path) -> Option<&LspFeatureFlags> {
        let lang_id = self.registry.language_id_for_file(path)?;
        self.servers.get(lang_id).map(|s| &s.capabilities)
    }

    pub fn server_handle_for_file(&self, path: &Path) -> Option<&ActorHandle> {
        let lang_id = self.registry.language_id_for_file(path)?;
        self.servers.get(lang_id).map(|s| &s.handle)
    }

    pub fn text_doc_position(
        &self,
        path: &Path,
        line: usize,
        col: usize,
        buffer: &ropey::Rope,
    ) -> Option<serde_json::Value> {
        let uri = router::path_to_uri(path)?;
        let pos = router::to_lsp_position(line, col, buffer);
        Some(serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": pos.line, "character": pos.character },
        }))
    }

    pub async fn request_completion(
        &self,
        path: &Path,
        line: usize,
        col: usize,
        buffer: &ropey::Rope,
    ) -> Option<serde_json::Value> {
        let caps = self.get_capabilities(path)?;
        if !caps.completion {
            return None;
        }

        let handle = self.server_handle_for_file(path)?;
        let params = self.text_doc_position(path, line, col, buffer)?;
        handle
            .send_request("textDocument/completion", params)
            .await
            .ok()
    }

    pub async fn request_hover(
        &self,
        path: &Path,
        line: usize,
        col: usize,
        buffer: &ropey::Rope,
    ) -> Option<serde_json::Value> {
        let caps = self.get_capabilities(path)?;
        if !caps.hover {
            return None;
        }

        let handle = self.server_handle_for_file(path)?;
        let params = self.text_doc_position(path, line, col, buffer)?;
        handle.send_request("textDocument/hover", params).await.ok()
    }

    pub async fn request_goto_definition(
        &self,
        path: &Path,
        line: usize,
        col: usize,
        buffer: &ropey::Rope,
    ) -> Option<serde_json::Value> {
        let caps = self.get_capabilities(path)?;
        if !caps.definition {
            return None;
        }

        let handle = self.server_handle_for_file(path)?;
        let params = self.text_doc_position(path, line, col, buffer)?;
        handle
            .send_request("textDocument/definition", params)
            .await
            .ok()
    }

    pub async fn request_references(
        &self,
        path: &Path,
        line: usize,
        col: usize,
        buffer: &ropey::Rope,
    ) -> Option<serde_json::Value> {
        let caps = self.get_capabilities(path)?;
        if !caps.references {
            return None;
        }

        let uri = router::path_to_uri(path)?;
        let pos = router::to_lsp_position(line, col, buffer);
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": pos.line, "character": pos.character },
            "context": { "includeDeclaration": true },
        });

        let handle = self.server_handle_for_file(path)?;
        handle
            .send_request("textDocument/references", params)
            .await
            .ok()
    }

    pub async fn request_formatting(&self, path: &Path) -> Option<serde_json::Value> {
        let caps = self.get_capabilities(path)?;
        if !caps.formatting {
            return None;
        }

        let uri = router::path_to_uri(path)?;
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "options": {
                "tabSize": 4,
                "insertSpaces": true,
            },
        });

        let handle = self.server_handle_for_file(path)?;
        handle
            .send_request("textDocument/formatting", params)
            .await
            .ok()
    }

    pub async fn request_code_action(
        &self,
        path: &Path,
        line: usize,
        col: usize,
        buffer: &ropey::Rope,
    ) -> Option<serde_json::Value> {
        let caps = self.get_capabilities(path)?;
        if !caps.code_action {
            return None;
        }

        let uri = router::path_to_uri(path)?;
        let pos = router::to_lsp_position(line, col, buffer);
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "range": {
                "start": { "line": pos.line, "character": pos.character },
                "end": { "line": pos.line, "character": pos.character },
            },
            "context": { "diagnostics": [] },
        });

        let handle = self.server_handle_for_file(path)?;
        handle
            .send_request("textDocument/codeAction", params)
            .await
            .ok()
    }

    pub async fn request_rename(
        &self,
        path: &Path,
        line: usize,
        col: usize,
        new_name: &str,
        buffer: &ropey::Rope,
    ) -> Option<serde_json::Value> {
        let caps = self.get_capabilities(path)?;
        if !caps.rename {
            return None;
        }

        let uri = router::path_to_uri(path)?;
        let pos = router::to_lsp_position(line, col, buffer);
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": pos.line, "character": pos.character },
            "newName": new_name,
        });

        let handle = self.server_handle_for_file(path)?;
        handle
            .send_request("textDocument/rename", params)
            .await
            .ok()
    }

    // ─── Lifecycle ─────────────────────────────────────────────────

    pub fn shutdown_all(&mut self) {
        for (lang_id, state) in self.servers.drain() {
            log::info!("shutting down {} server", lang_id);
            state.handle.request_shutdown();
        }
    }

    pub fn is_server_running(&self, path: &Path) -> bool {
        let lang_id = match self.registry.language_id_for_file(path) {
            Some(id) => id,
            None => return false,
        };
        self.servers.contains_key(lang_id)
    }

    pub fn running_server_count(&self) -> usize {
        self.servers.len()
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────

/// Walk up from the file's directory to find the project root.
fn detect_project_root(file_path: &Path, root_markers: &[String]) -> PathBuf {
    let start = if file_path.is_file() {
        file_path.parent().unwrap_or(file_path)
    } else {
        file_path
    };

    let mut dir = start.to_path_buf();
    loop {
        for marker in root_markers {
            if dir.join(marker).exists() {
                return dir;
            }
        }

        if !dir.pop() {
            break;
        }
    }

    // Fallback: use cwd
    std::env::current_dir().unwrap_or_else(|_| start.to_path_buf())
}

/// Client capabilities we advertise to the server.
fn client_capabilities() -> serde_json::Value {
    serde_json::json!({
        "textDocument": {
            "synchronization": {
                "dynamicRegistration": false,
                "willSave": false,
                "willSaveWaitUntil": false,
                "didSave": true,
            },
            "completion": {
                "completionItem": {
                    "snippetSupport": false,
                    "documentationFormat": ["plaintext", "markdown"],
                },
                "completionItemKind": {},
            },
            "hover": {
                "contentFormat": ["plaintext", "markdown"],
            },
            "definition": {
                "dynamicRegistration": false,
            },
            "references": {
                "dynamicRegistration": false,
            },
            "formatting": {
                "dynamicRegistration": false,
            },
            "codeAction": {
                "dynamicRegistration": false,
            },
            "rename": {
                "dynamicRegistration": false,
            },
            "publishDiagnostics": {
                "relatedInformation": false,
            },
        },
        "general": {
            "positionEncodings": ["utf-8", "utf-16"]
        },
        "window": {
            "workDoneProgress": false,
        },
    })
}
