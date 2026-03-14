use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use klein_ide::lsp::manager::LspManager;
use klein_ide::lsp::actor::LspServerNotification;
use klein_ide::config::AppConfig;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::unbounded_channel::<LspServerNotification>();
    let config = AppConfig {
        enabled_lsps: Some(vec!["rust".to_string()]),
        ..Default::default()
    };
    
    let mut manager = LspManager::new(tx, &config);
    let sample_path = PathBuf::from("src/main.rs");
    let absolute_path = std::fs::canonicalize(&sample_path).unwrap();
    
    println!("Starting rust-analyzer...");
    let lang_id = manager.ensure_server_for_file(&absolute_path).await;
    println!("Server started with lang_id: {:?}", lang_id);
    
    assert_eq!(lang_id.as_deref(), Some("rust"));

    // Wait a bit for initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // Open document
    let text = std::fs::read_to_string(&absolute_path).unwrap();
    manager.notify_did_open(&absolute_path, &text);
    println!("Sent didOpen");

    // Let's test hover at line 0, char 0
    let mut buffer = ropey::Rope::from_str(&text);
    if let Some(hover) = manager.request_hover(&absolute_path, 0, 0, &buffer).await {
        println!("Hover successful: {:?}", hover);
    } else {
        println!("Hover returned None or timed out");
    }

    // Graceful shutdown happens on drop
    println!("Test complete");
}
