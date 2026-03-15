//! LSP subsystem for Klein IDE.
//!
//! This module implements the Language Server Protocol client using an
//! **actor-based architecture** inspired by Helix and Zed:
//!
//! - Each language server runs as a dedicated Tokio task (the "actor")
//!   that exclusively owns the server's stdin/stdout.
//! - Klein communicates with actors through `mpsc` channels — no locks.
//! - Server notifications flow back to Klein's event loop via a shared
//!   notification channel.
//!
//! # Module Layout
//!
//! - `actor`        — Actor message types, handle, and (later) the actor loop.
//! - `codec`        — LSP wire protocol (Content-Length framing).
//! - `registry`     — Language → server configuration mapping.
//! - `router`       — Klein ↔ LSP position/type conversions.
//! - `doc_sync`     — Document lifecycle tracking (version numbers).
//! - `types`        — Klein-native LSP types (diagnostics, completions, etc.).
//! - `capabilities` — Server capability detection / feature flags.

pub mod actor;
pub mod capabilities;
pub mod codec;
pub mod doc_sync;
pub mod manager;
pub mod registry;
pub mod router;
pub mod types;

pub use manager::LspManager;
pub use types::LspState;
