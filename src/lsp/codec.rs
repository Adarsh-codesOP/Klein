//! LSP wire protocol codec.
//!
//! Handles the Content-Length header framing that LSP uses on its JSON-RPC
//! transport. Every LSP message is preceded by HTTP-style headers:
//!
//! ```text
//! Content-Length: 42\r\n
//! \r\n
//! {"jsonrpc":"2.0", ...}
//! ```
//!
//! This module provides `encode` and `decode` functions for this framing.

use anyhow::{anyhow, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

/// Encode a JSON value into an LSP wire-protocol message.
///
/// Returns the byte sequence: `Content-Length: N\r\n\r\n<JSON body>`.
pub fn encode(msg: &serde_json::Value) -> Vec<u8> {
    let body = serde_json::to_string(msg).expect("failed to serialize JSON");
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut bytes = header.into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

/// Decode one LSP message from an async buffered reader.
///
/// Reads headers until the blank `\r\n` separator, extracts `Content-Length`,
/// then reads exactly that many bytes and deserializes the JSON body.
///
/// Returns `Err` on EOF, malformed headers, or invalid JSON.
pub async fn decode<R: AsyncBufReadExt + Unpin>(reader: &mut R) -> Result<serde_json::Value> {
    let mut content_length: Option<usize> = None;

    // Read headers
    loop {
        let mut header_line = String::new();
        let bytes_read = reader.read_line(&mut header_line).await?;
        if bytes_read == 0 {
            return Err(anyhow!("EOF: language server connection closed"));
        }

        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            // Empty line signals end of headers
            break;
        }

        // Parse Content-Length header (case-insensitive)
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| anyhow!("invalid Content-Length: {}", e))?,
            );
        }
        // Ignore other headers (e.g., Content-Type)
    }

    let content_length =
        content_length.ok_or_else(|| anyhow!("missing Content-Length header"))?;

    if content_length == 0 {
        return Err(anyhow!("Content-Length is 0"));
    }

    // Read exactly content_length bytes
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await?;

    let value: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| anyhow!("invalid JSON in LSP message: {}", e))?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        let msg = serde_json::json!({"jsonrpc": "2.0", "method": "initialized"});
        let encoded = encode(&msg);
        let s = String::from_utf8(encoded).unwrap();
        assert!(s.starts_with("Content-Length: "));
        assert!(s.contains("\r\n\r\n"));
        // The body after the blank line should be valid JSON
        let parts: Vec<&str> = s.splitn(2, "\r\n\r\n").collect();
        assert_eq!(parts.len(), 2);
        let body: serde_json::Value = serde_json::from_str(parts[1]).unwrap();
        assert_eq!(body["method"], "initialized");
    }

    #[tokio::test]
    async fn test_decode() {
        let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": null});
        let encoded = encode(&msg);
        let mut cursor = tokio::io::BufReader::new(&encoded[..]);
        let decoded = decode(&mut cursor).await.unwrap();
        assert_eq!(decoded["id"], 1);
    }

    #[tokio::test]
    async fn test_decode_eof() {
        let data: &[u8] = b"";
        let mut cursor = tokio::io::BufReader::new(data);
        assert!(decode(&mut cursor).await.is_err());
    }
}
