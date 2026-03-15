use crate::theme::{Theme, ThemeConfig};
use include_dir::{include_dir, Dir};
use std::path::PathBuf;

pub static BUILTIN_THEMES: Dir = include_dir!("$CARGO_MANIFEST_DIR/themes");

pub fn get_user_theme_dir() -> Option<PathBuf> {
    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "Klein") {
        return Some(proj_dirs.config_dir().join("themes"));
    }
    None
}

fn load_theme_config(name: &str) -> Option<ThemeConfig> {
    let filename = format!("{}.toml", name);

    // 1. Check user themes
    if let Some(dir) = get_user_theme_dir() {
        let path = dir.join(&filename);
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return Some(config);
                } else {
                    log::error!("Failed to parse user theme: {}", path.display());
                }
            }
        }
    }

    // 2. Check built-in themes
    if let Some(file) = BUILTIN_THEMES.get_file(&filename) {
        if let Some(contents) = file.contents_utf8() {
            if let Ok(config) = toml::from_str(contents) {
                return Some(config);
            } else {
                log::error!("Failed to parse built-in theme: {}", filename);
            }
        }
    }

    None
}

pub fn load_theme(name: &str) -> Theme {
    let config_opt = load_theme_config(name);
    let mut base_theme = Theme::default();

    if let Some(config) = &config_opt {
        if let Some(ext) = &config.extends {
            base_theme = load_theme(ext);
        }
    } else {
        // If theme doesn't exist, try loading default dark theme if asking for something else
        if name != "dark" {
            return load_theme("dark");
        }
        return base_theme;
    }

    if let Some(config) = config_opt {
        base_theme.merge_config(&config);
        base_theme.name = name.to_string();
    }

    base_theme
}
