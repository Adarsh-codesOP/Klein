use crate::theme_loader::{get_user_theme_dir, BUILTIN_THEMES};
use std::collections::HashSet;

pub fn list_themes() -> Vec<String> {
    let mut themes = HashSet::new();

    // User themes
    if let Some(dir) = get_user_theme_dir() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".toml") {
                        themes.insert(name.trim_end_matches(".toml").to_string());
                    }
                }
            }
        }
    }

    // Built-in themes
    for entry in BUILTIN_THEMES.files() {
        if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".toml") {
                themes.insert(name.trim_end_matches(".toml").to_string());
            }
        }
    }

    let mut themes: Vec<String> = themes.into_iter().collect();
    themes.sort();
    themes
}
