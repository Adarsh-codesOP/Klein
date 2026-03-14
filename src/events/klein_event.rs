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
