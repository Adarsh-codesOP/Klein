//! Document synchronization engine.
//!
//! Tracks which documents are open from the LSP perspective, manages version
//! numbers, and generates the appropriate `didOpen`, `didChange`, `didSave`,
//! and `didClose` notification parameters.
//!
//! Documents start with version 1 on `didOpen` and the version strictly
//! increases on every `didChange`. This is required by the LSP spec.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// State for a single document being tracked.
#[derive(Debug)]
struct DocumentState {
    /// The language ID reported to the server (e.g., "rust").
    language_id: String,
    /// Monotonically increasing version number.
    version: i32,
}

/// Manages document lifecycle for LSP synchronization.
///
/// This engine is LSP-transport-agnostic — it produces the data needed for
/// notifications but does not send them itself. The caller (LspManager)
/// is responsible for forwarding to the appropriate actor.
pub struct DocSyncEngine {
    /// Tracked documents keyed by canonical file path.
    documents: HashMap<PathBuf, DocumentState>,
}

impl DocSyncEngine {
    /// Create a new, empty sync engine.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Track a document as opened.
    ///
    /// Returns `(language_id, version)` for the `didOpen` notification.
    /// If the document was already tracked, it is re-opened with a fresh version.
    pub fn open_document(&mut self, path: &Path, language_id: &str) -> (String, i32) {
        let state = DocumentState {
            language_id: language_id.to_string(),
            version: 1,
        };
        let lang = state.language_id.clone();
        let ver = state.version;
        self.documents.insert(path.to_path_buf(), state);
        (lang, ver)
    }

    /// Mark a document as changed and bump its version.
    ///
    /// Returns the new version number for the `didChange` notification.
    /// Returns `None` if the document is not currently tracked.
    pub fn change_document(&mut self, path: &Path) -> Option<i32> {
        if let Some(state) = self.documents.get_mut(path) {
            state.version += 1;
            Some(state.version)
        } else {
            None
        }
    }

    /// Check if a document is currently tracked as open.
    pub fn is_open(&self, path: &Path) -> bool {
        self.documents.contains_key(path)
    }

    /// Get the current version of a tracked document.
    #[allow(dead_code)]
    pub fn version(&self, path: &Path) -> Option<i32> {
        self.documents.get(path).map(|s| s.version)
    }

    /// Get the language ID of a tracked document.
    #[allow(dead_code)]
    pub fn language_id(&self, path: &Path) -> Option<&str> {
        self.documents.get(path).map(|s| s.language_id.as_str())
    }

    /// Stop tracking a document (for `didClose`).
    ///
    /// Returns `true` if the document was tracked, `false` otherwise.
    pub fn close_document(&mut self, path: &Path) -> bool {
        self.documents.remove(path).is_some()
    }

    /// Get all currently tracked document paths.
    /// Used when restarting a server to re-send `didOpen` for all open files.
    pub fn open_documents(&self) -> Vec<(&PathBuf, &str)> {
        self.documents
            .iter()
            .map(|(path, state)| (path, state.language_id.as_str()))
            .collect()
    }
}

impl Default for DocSyncEngine {
    fn default() -> Self {
        Self::new()
    }
}
