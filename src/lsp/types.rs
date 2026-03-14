//! Klein-native LSP types.
//!
//! These types are Klein's internal representation of LSP concepts.
//! They decouple Klein's UI and state management from the raw LSP protocol types,
//! making the codebase easier to evolve independently of LSP spec changes.

use std::collections::HashMap;
use std::path::PathBuf;

// ─── Diagnostics ───────────────────────────────────────────────────────

/// Severity level for a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// Hint — lowest severity.
    Hint,
    /// Informational message.
    Info,
    /// Warning.
    Warning,
    /// Error — highest severity.
    Error,
}

impl DiagnosticSeverity {
    /// Short display string for the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }

    /// Gutter icon character.
    pub fn icon(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "●",
            DiagnosticSeverity::Warning => "▲",
            DiagnosticSeverity::Info => "ℹ",
            DiagnosticSeverity::Hint => "·",
        }
    }
}

/// A diagnostic message tied to a specific range in a file.
#[derive(Clone, Debug)]
pub struct KleinDiagnostic {
    /// Zero-indexed line number.
    pub line: usize,
    /// Zero-indexed start column (char offset, not UTF-16).
    pub col_start: usize,
    /// Zero-indexed end column (char offset, not UTF-16).
    pub col_end: usize,
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Human-readable message.
    pub message: String,
    /// Source of the diagnostic (e.g., "rustc", "clippy").
    pub source: Option<String>,
    /// Diagnostic code (e.g., "E0308").
    pub code: Option<String>,
}

// ─── Completions ───────────────────────────────────────────────────────

/// The kind of a completion item, used to select an icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Variable,
    Struct,
    Field,
    Module,
    Keyword,
    Snippet,
    Constant,
    Enum,
    Interface,
    Property,
    Other,
}

impl CompletionKind {
    /// TUI icon for this completion kind.
    pub fn icon(&self) -> &'static str {
        match self {
            CompletionKind::Function => "ƒ",
            CompletionKind::Variable => "ν",
            CompletionKind::Struct => "τ",
            CompletionKind::Field => "◆",
            CompletionKind::Module => "Μ",
            CompletionKind::Keyword => "κ",
            CompletionKind::Snippet => "⊞",
            CompletionKind::Constant => "π",
            CompletionKind::Enum => "ε",
            CompletionKind::Interface => "ι",
            CompletionKind::Property => "◇",
            CompletionKind::Other => "·",
        }
    }
}

/// A single completion suggestion.
#[derive(Clone, Debug)]
pub struct KleinCompletion {
    /// The primary display label.
    pub label: String,
    /// Optional short detail (type signature, etc.).
    pub detail: Option<String>,
    /// Optional documentation string.
    pub documentation: Option<String>,
    /// Kind of completion item (for icon display).
    pub kind: CompletionKind,
    /// The text to insert when accepted.
    pub insert_text: String,
    /// LSP range to replace on accepting the completion.
    pub replace_range: Option<lsp_types::Range>,
    /// Sort priority (lower string = higher priority).
    pub sort_text: Option<String>,
}

// ─── Hover ─────────────────────────────────────────────────────────────

/// Hover information for the symbol under the cursor.
#[derive(Clone, Debug)]
pub struct KleinHoverInfo {
    /// The hover content (rendered to plain text from markdown).
    pub contents: String,
    /// Optional range the hover applies to: (start_line, start_col, end_line, end_col).
    pub range: Option<(usize, usize, usize, usize)>,
}

// ─── Completion Popup State ────────────────────────────────────────────

/// State for the active completion popup in the UI.
#[derive(Clone, Debug)]
pub struct CompletionState {
    /// Available completion items.
    pub items: Vec<KleinCompletion>,
    /// Currently selected index.
    pub selected_index: usize,
    /// Scroll offset for the visible window.
    pub scroll: usize,
    /// The (line, col) where completion was triggered.
    pub trigger_position: (usize, usize),
}

// ─── Rename State ──────────────────────────────────────────────────────

/// State for an active rename operation.
#[derive(Clone, Debug)]
pub struct RenameState {
    /// The (line, col) where rename was triggered.
    pub trigger_position: (usize, usize),
    /// The canonical path of the file where rename was triggered.
    pub path: PathBuf,
    /// The current user-inputted new name.
    pub new_name: String,
    /// Whether the prompt is active.
    pub active: bool,
}

impl Default for RenameState {
    fn default() -> Self {
        Self {
            trigger_position: (0, 0),
            path: PathBuf::new(),
            new_name: String::new(),
            active: false,
        }
    }
}

// ─── Server Status ─────────────────────────────────────────────────────

/// Status of a language server instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerStatus {
    /// Server is in the process of starting / initializing.
    Starting,
    /// Server is running and responsive.
    Running,
    /// Server encountered an error.
    Error(String),
    /// Server has been stopped (clean shutdown or crash).
    Stopped,
}

// ─── LspState ──────────────────────────────────────────────────────────

/// Aggregated LSP state made available to the UI layer.
///
/// This struct is owned by `App` and updated when LSP notifications arrive.
/// The UI reads from it to render diagnostics, completions, hover, etc.
#[derive(Default)]
pub struct LspState {
    /// Per-file diagnostics. Key is the canonical file path.
    pub diagnostics: HashMap<PathBuf, Vec<KleinDiagnostic>>,

    /// The currently active completion popup, if any.
    pub completion: Option<CompletionState>,

    /// The currently displayed hover tooltip, if any.
    pub hover: Option<KleinHoverInfo>,

    /// Status of each language server, keyed by language ID (e.g., "rust", "python").
    pub server_status: HashMap<String, ServerStatus>,
    /// Active rename operation, if any.
    pub rename: Option<RenameState>,
}

