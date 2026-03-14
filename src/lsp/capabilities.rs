//! Server capability detection and feature flags.
//!
//! After the LSP initialize handshake, the server reports its capabilities.
//! This module extracts those into a simple set of boolean flags that the
//! rest of Klein can check before making LSP requests.

use lsp_types::ServerCapabilities;

/// Simplified feature flags derived from `ServerCapabilities`.
///
/// Instead of passing raw capabilities everywhere, Klein checks these flags
/// to decide whether to enable specific IDE features.
#[derive(Debug, Clone, Default)]
pub struct LspFeatureFlags {
    /// Server supports textDocument/hover.
    pub hover: bool,
    /// Server supports textDocument/completion.
    pub completion: bool,
    /// Characters that trigger auto-completion (e.g., '.', ':', '/').
    pub completion_trigger_chars: Vec<char>,
    /// Server supports textDocument/definition.
    pub definition: bool,
    /// Server supports textDocument/references.
    pub references: bool,
    /// Server supports textDocument/formatting.
    pub formatting: bool,
    /// Server supports textDocument/rename.
    pub rename: bool,
    /// Server supports textDocument/codeAction.
    pub code_action: bool,
    /// Server supports textDocument/signatureHelp.
    pub signature_help: bool,
    /// Server supports textDocument/documentSymbol.
    pub document_symbols: bool,
    /// Server supports workspace/symbol.
    pub workspace_symbols: bool,
    /// Server supports textDocument/semanticTokens.
    pub semantic_tokens: bool,
    /// Server supports textDocument/inlayHint.
    pub inlay_hints: bool,
}

impl LspFeatureFlags {
    /// Extract feature flags from raw server capabilities.
    pub fn from_capabilities(caps: &ServerCapabilities) -> Self {
        let mut completion = false;
        let mut completion_trigger_chars = Vec::new();
        if let Some(ref provider) = caps.completion_provider {
            completion = true;
            if let Some(ref triggers) = provider.trigger_characters {
                completion_trigger_chars =
                    triggers.iter().filter_map(|s| s.chars().next()).collect();
            }
        }

        Self {
            hover: caps.hover_provider.is_some(),
            completion,
            completion_trigger_chars,
            definition: caps.definition_provider.is_some(),
            references: caps.references_provider.is_some(),
            formatting: caps.document_formatting_provider.is_some(),
            rename: caps.rename_provider.is_some(),
            code_action: caps.code_action_provider.is_some(),
            signature_help: caps.signature_help_provider.is_some(),
            document_symbols: caps.document_symbol_provider.is_some(),
            workspace_symbols: caps.workspace_symbol_provider.is_some(),
            semantic_tokens: caps.semantic_tokens_provider.is_some(),
            inlay_hints: caps.inlay_hint_provider.is_some(),
        }
    }
}
