//! Position and type conversions between Klein's internal types and LSP types.
//!
//! Klein uses `(cursor_y, cursor_x)` where both are zero-indexed char (Unicode scalar)
//! offsets. LSP uses `Position { line: u32, character: u32 }` where `character` is
//! a **UTF-16 code unit offset**. This module handles the conversion correctly,
//! including for non-BMP characters (emoji, CJK, etc.).

use crate::lsp::types::{CompletionKind, DiagnosticSeverity, KleinCompletion, KleinDiagnostic};
use lsp_types::{CompletionItem, Diagnostic, Position};
use ropey::Rope;
use std::path::Path;
use url::Url;

/// Convert a Klein cursor position (line, char-column) to an LSP Position.
///
/// The character offset is converted from Unicode scalar index to UTF-16 code unit index.
pub fn to_lsp_position(line: usize, col: usize, buffer: &Rope) -> Position {
    if line >= buffer.len_lines() {
        return Position {
            line: line as u32,
            character: col as u32,
        };
    }

    let rope_line = buffer.line(line);
    let mut utf16_offset: u32 = 0;

    for (i, ch) in rope_line.chars().enumerate() {
        if i >= col {
            break;
        }
        utf16_offset += ch.len_utf16() as u32;
    }

    Position {
        line: line as u32,
        character: utf16_offset,
    }
}

/// Convert an LSP Position to a Klein cursor position (line, char-column).
///
/// The UTF-16 character offset is converted back to a Unicode scalar (char) index.
pub fn from_lsp_position(pos: &Position, buffer: &Rope) -> (usize, usize) {
    let line = pos.line as usize;
    if line >= buffer.len_lines() {
        return (line, 0);
    }

    let rope_line = buffer.line(line);
    let mut utf16_count: u32 = 0;

    for (i, ch) in rope_line.chars().enumerate() {
        if utf16_count >= pos.character {
            return (line, i);
        }
        utf16_count += ch.len_utf16() as u32;
    }

    // character offset is at or past end of line
    let mut max_col = rope_line.len_chars();
    while max_col > 0 && (rope_line.char(max_col - 1) == '\n' || rope_line.char(max_col - 1) == '\r') {
        max_col -= 1;
    }
    (line, max_col)
}

/// Convert a filesystem path to an LSP document URI.
///
/// Example: `C:\project\main.rs` → `file:///C:/project/main.rs`
pub fn path_to_uri(path: &Path) -> Option<Url> {
    // Canonicalize to ensure consistent path format
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    Url::from_file_path(&canonical).ok()
}

/// Convert an LSP document URI back to a filesystem path.
///
/// Example: `file:///C:/project/main.rs` → `C:\project\main.rs`
pub fn uri_to_path(uri: &Url) -> Option<std::path::PathBuf> {
    uri.to_file_path().ok()
}

/// Convert an LSP Diagnostic to a KleinDiagnostic.
/// Uses the buffer to convert UTF-16 characters to char offsets.
pub fn to_klein_diagnostic(diag: &Diagnostic, buffer: &Rope) -> KleinDiagnostic {
    let (line, col_start) = from_lsp_position(&diag.range.start, buffer);
    let (_, col_end) = from_lsp_position(&diag.range.end, buffer);

    let severity = match diag.severity {
        Some(lsp_types::DiagnosticSeverity::ERROR) => DiagnosticSeverity::Error,
        Some(lsp_types::DiagnosticSeverity::WARNING) => DiagnosticSeverity::Warning,
        Some(lsp_types::DiagnosticSeverity::INFORMATION) => DiagnosticSeverity::Info,
        Some(lsp_types::DiagnosticSeverity::HINT) => DiagnosticSeverity::Hint,
        _ => DiagnosticSeverity::Info,
    };

    KleinDiagnostic {
        line,
        col_start,
        col_end,
        severity,
        message: diag.message.clone(),
        source: diag.source.clone(),
        code: diag.code.as_ref().map(|c| match c {
            lsp_types::NumberOrString::Number(n) => n.to_string(),
            lsp_types::NumberOrString::String(s) => s.clone(),
        }),
    }
}

pub fn to_klein_completion(item: &CompletionItem) -> KleinCompletion {
    let mut insert_text = item
        .insert_text
        .clone()
        .unwrap_or_else(|| item.label.clone());
    let mut replace_range = None;

    if let Some(edit) = &item.text_edit {
        match edit {
            lsp_types::CompletionTextEdit::Edit(e) => {
                insert_text = e.new_text.clone();
                replace_range = Some(e.range);
            }
            lsp_types::CompletionTextEdit::InsertAndReplace(e) => {
                insert_text = e.new_text.clone();
                replace_range = Some(e.replace);
            }
        }
    }

    KleinCompletion {
        label: item.label.clone(),
        detail: item.detail.clone(),
        documentation: item.documentation.as_ref().map(|d| match d {
            lsp_types::Documentation::String(s) => s.clone(),
            lsp_types::Documentation::MarkupContent(m) => m.value.clone(),
        }),
        kind: map_completion_kind(item.kind),
        insert_text,
        replace_range,
        sort_text: item.sort_text.clone(),
    }
}

fn map_completion_kind(kind: Option<lsp_types::CompletionItemKind>) -> CompletionKind {
    use lsp_types::CompletionItemKind as LspKind;
    match kind {
        Some(LspKind::FUNCTION) | Some(LspKind::METHOD) | Some(LspKind::CONSTRUCTOR) => {
            CompletionKind::Function
        }
        Some(LspKind::VARIABLE) => CompletionKind::Variable,
        Some(LspKind::STRUCT) | Some(LspKind::CLASS) => CompletionKind::Struct,
        Some(LspKind::FIELD) => CompletionKind::Field,
        Some(LspKind::MODULE) => CompletionKind::Module,
        Some(LspKind::KEYWORD) => CompletionKind::Keyword,
        Some(LspKind::SNIPPET) => CompletionKind::Snippet,
        Some(LspKind::CONSTANT) => CompletionKind::Constant,
        Some(LspKind::ENUM) => CompletionKind::Enum,
        Some(LspKind::INTERFACE) => CompletionKind::Interface,
        Some(LspKind::PROPERTY) => CompletionKind::Property,
        _ => CompletionKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_roundtrip() {
        let buffer = Rope::from_str("hello world\n");
        let pos = to_lsp_position(0, 5, &buffer);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5);

        let (line, col) = from_lsp_position(&pos, &buffer);
        assert_eq!(line, 0);
        assert_eq!(col, 5);
    }

    #[test]
    fn test_position_at_start() {
        let buffer = Rope::from_str("abc\n");
        let pos = to_lsp_position(0, 0, &buffer);
        assert_eq!(pos.character, 0);

        let (line, col) = from_lsp_position(&pos, &buffer);
        assert_eq!((line, col), (0, 0));
    }

    #[test]
    fn test_empty_buffer() {
        let buffer = Rope::from_str("");
        // Past-end positions should not panic
        let pos = to_lsp_position(5, 3, &buffer);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 3);
    }
}
