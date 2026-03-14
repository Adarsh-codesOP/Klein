use crate::lsp::actor::LspServerNotification;
use crossterm::event::Event as CrosstermEvent;

/// Unified event type for Klein's event loop.
///
/// All async sources (terminal input, LSP, timers) are funneled into this
/// single enum so the event loop has one place to dispatch from.
pub enum KleinEvent {
    /// A crossterm terminal event (key, mouse, paste, resize).
    Terminal(CrosstermEvent),

    /// A notification from an LSP server (diagnostics, progress, etc.).
    Lsp(LspServerNotification),

    /// A debounce timer fired.
    Timer(TimerKind),

    /// Trigger LSP server initialization for a file.
    InitLsp(std::path::PathBuf),
    /// Request definition for symbol at cursor.
    GotoDefinition,
    /// Request references for symbol at cursor.
    FindReferences,
    /// Request formatting for current document.
    FormatDocument,
    /// Request rename for symbol at cursor.
    Rename,
    /// Request code actions for cursor position.
    CodeAction,
    /// Response from a completion request.
    CompletionResponse(Option<serde_json::Value>, std::path::PathBuf, (usize, usize)),
    /// Response from a hover request.
    HoverResponse(Option<serde_json::Value>, std::path::PathBuf, (usize, usize)),
    /// Response from a definition request.
    DefinitionResponse(Option<serde_json::Value>, std::path::PathBuf),
    /// Response from a references request.
    ReferencesResponse(Option<serde_json::Value>, std::path::PathBuf),
    /// Response from a formatting request.
    FormatResponse(Option<serde_json::Value>, std::path::PathBuf),
    /// Response from a rename request.
    RenameResponse(Option<serde_json::Value>, std::path::PathBuf, String),
    /// Response from a code action request.
    CodeActionResponse(Option<serde_json::Value>, std::path::PathBuf, (usize, usize)),
}

/// Identifies which debounce timer fired.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TimerKind {
    /// Send document changes to the LSP server.
    DocumentSync,
    /// Trigger autocompletion after a typing pause.
    CompletionTrigger,
    /// Trigger hover info after cursor stops moving.
    HoverTrigger,
}
