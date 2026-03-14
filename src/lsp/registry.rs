//! LSP server registry.
//!
//! Maps file extensions to language server configurations.
//! Provides built-in defaults for popular languages and supports
//! user overrides via config.toml.

use std::collections::HashMap;
use std::path::Path;

/// Configuration for a single language server.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// The executable command (e.g., "rust-analyzer", "pyright-langserver").
    pub command: String,
    /// Command-line arguments (e.g., ["--stdio"]).
    pub args: Vec<String>,
    /// The LSP language identifier (e.g., "rust", "python").
    pub language_id: String,
    /// Files whose presence marks the project root (e.g., ["Cargo.toml"]).
    pub root_markers: Vec<String>,
}

/// Registry that maps file extensions to language server configurations.
pub struct LspRegistry {
    /// Map from file extension (without leading dot) to server config.
    servers: HashMap<String, ServerConfig>,
}

impl LspRegistry {
    /// Create a new registry with built-in defaults.
    pub fn new(enabled_lsps: Option<&Vec<String>>) -> Self {
        let mut servers = HashMap::new();

        // If config.enabled_lsps is None, we assume no servers are enabled.
        // The user must explicitly opt-in to LSP servers via config.toml
        let is_enabled = |id: &str| {
            if let Some(enabled) = enabled_lsps {
                enabled
                    .iter()
                    .any(|s| s.to_lowercase() == id.to_lowercase())
            } else {
                false
            }
        };

        // Rust
        if is_enabled("rust") {
            let rust_config = ServerConfig {
                command: "rust-analyzer".into(),
                args: vec![],
                language_id: "rust".into(),
                root_markers: vec!["Cargo.toml".into()],
            };
            servers.insert("rs".into(), rust_config);
        }

        // Python
        if is_enabled("python") {
            let python_config = ServerConfig {
                command: "pyright-langserver".into(),
                args: vec!["--stdio".into()],
                language_id: "python".into(),
                root_markers: vec![
                    "pyproject.toml".into(),
                    "setup.py".into(),
                    "requirements.txt".into(),
                ],
            };
            servers.insert("py".into(), python_config);
        }

        // JavaScript / TypeScript
        if is_enabled("javascript") || is_enabled("typescript") {
            let js_config = ServerConfig {
                command: "typescript-language-server".into(),
                args: vec!["--stdio".into()],
                language_id: "javascript".into(),
                root_markers: vec!["package.json".into()],
            };
            servers.insert("js".into(), js_config.clone());
            servers.insert("jsx".into(), js_config);

            let ts_config = ServerConfig {
                command: "typescript-language-server".into(),
                args: vec!["--stdio".into()],
                language_id: "typescript".into(),
                root_markers: vec!["tsconfig.json".into(), "package.json".into()],
            };
            servers.insert("ts".into(), ts_config.clone());
            servers.insert("tsx".into(), ts_config);
        }

        // C / C++
        if is_enabled("c") {
            let c_config = ServerConfig {
                command: "clangd".into(),
                args: vec![],
                language_id: "c".into(),
                root_markers: vec!["compile_commands.json".into(), "CMakeLists.txt".into()],
            };
            servers.insert("c".into(), c_config.clone());
            servers.insert("h".into(), c_config);

            let cpp_config = ServerConfig {
                command: "clangd".into(),
                args: vec![],
                language_id: "cpp".into(),
                root_markers: vec!["compile_commands.json".into(), "CMakeLists.txt".into()],
            };
            servers.insert("cpp".into(), cpp_config.clone());
            servers.insert("hpp".into(), cpp_config.clone());
            servers.insert("cc".into(), cpp_config);
        }

        // Go
        if is_enabled("go") {
            let go_config = ServerConfig {
                command: "gopls".into(),
                args: vec!["serve".into()],
                language_id: "go".into(),
                root_markers: vec!["go.mod".into()],
            };
            servers.insert("go".into(), go_config);
        }

        // Java
        if is_enabled("java") {
            let java_config = ServerConfig {
                command: "jdtls".into(),
                args: vec![],
                language_id: "java".into(),
                root_markers: vec!["pom.xml".into(), "build.gradle".into()],
            };
            servers.insert("java".into(), java_config);
        }

        // HTML
        if is_enabled("html") {
            let html_config = ServerConfig {
                command: "vscode-html-languageserver".into(),
                args: vec!["--stdio".into()],
                language_id: "html".into(),
                root_markers: vec![],
            };
            servers.insert("html".into(), html_config);
        }

        // CSS
        if is_enabled("css") {
            let css_config = ServerConfig {
                command: "vscode-css-languageserver".into(),
                args: vec!["--stdio".into()],
                language_id: "css".into(),
                root_markers: vec![],
            };
            servers.insert("css".into(), css_config);
        }

        // JSON
        if is_enabled("json") {
            let json_config = ServerConfig {
                command: "vscode-json-languageserver".into(),
                args: vec!["--stdio".into()],
                language_id: "json".into(),
                root_markers: vec![],
            };
            servers.insert("json".into(), json_config);
        }

        // YAML
        if is_enabled("yaml") {
            let yaml_config = ServerConfig {
                command: "yaml-language-server".into(),
                args: vec!["--stdio".into()],
                language_id: "yaml".into(),
                root_markers: vec![],
            };
            servers.insert("yaml".into(), yaml_config.clone());
            servers.insert("yml".into(), yaml_config);
        }

        // Markdown
        if is_enabled("markdown") {
            let md_config = ServerConfig {
                command: "marksman".into(),
                args: vec!["server".into()],
                language_id: "markdown".into(),
                root_markers: vec![],
            };
            servers.insert("md".into(), md_config.clone());
            servers.insert("markdown".into(), md_config);
        }

        // TOML (Enable by default if needed, or if in list)
        if is_enabled("toml") {
            let toml_config = ServerConfig {
                command: "taplo".into(),
                args: vec!["lsp".into(), "stdio".into()],
                language_id: "toml".into(),
                root_markers: vec![],
            };
            servers.insert("toml".into(), toml_config);
        }

        LspRegistry { servers }
    }

    /// Look up the server configuration for a given file path.
    ///
    /// Returns `None` if the file's extension has no associated language server.
    pub fn find_server_for_file(&self, path: &Path) -> Option<&ServerConfig> {
        let ext = path.extension()?.to_str()?;
        self.servers.get(ext)
    }

    /// Get the language ID for a file path (e.g., "rust" for ".rs" files).
    pub fn language_id_for_file(&self, path: &Path) -> Option<&str> {
        self.find_server_for_file(path)
            .map(|c| c.language_id.as_str())
    }

    /// Override or add a server configuration for a given file extension.
    #[allow(dead_code)]
    pub fn set_server(&mut self, extension: String, config: ServerConfig) {
        self.servers.insert(extension, config);
    }

    /// Get a static list of all available language server identifiers currently supported.
    pub fn available_servers() -> &'static [&'static str] {
        &[
            "rust",
            "python",
            "javascript",
            "typescript",
            "c",
            "cpp",
            "go",
            "java",
            "html",
            "css",
            "json",
            "yaml",
            "markdown",
            "toml",
        ]
    }
}

impl Default for LspRegistry {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_servers() {
        let servers = LspRegistry::available_servers();
        assert!(servers.contains(&"rust"));
        assert!(servers.contains(&"python"));
        assert!(servers.contains(&"typescript"));
    }
}
