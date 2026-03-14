use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Parser};

pub struct TSManager {
    registry: HashMap<String, Language>,
}

impl TSManager {
    pub fn new() -> Self {
        let mut registry = HashMap::new();

        // Register grammars
        registry.insert("rs".to_string(), tree_sitter_rust::language());
        registry.insert("py".to_string(), tree_sitter_python::language());
        registry.insert("js".to_string(), tree_sitter_javascript::language());
        registry.insert(
            "ts".to_string(),
            tree_sitter_typescript::language_typescript(),
        );
        registry.insert("tsx".to_string(), tree_sitter_typescript::language_tsx());
        registry.insert("c".to_string(), tree_sitter_c::language());
        registry.insert("h".to_string(), tree_sitter_c::language());
        registry.insert("cpp".to_string(), tree_sitter_cpp::language());
        registry.insert("hpp".to_string(), tree_sitter_cpp::language());
        registry.insert("go".to_string(), tree_sitter_go::language());
        registry.insert("json".to_string(), tree_sitter_json::language());
        registry.insert("toml".to_string(), tree_sitter_toml::language());
        registry.insert("md".to_string(), tree_sitter_md::language());
        registry.insert("markdown".to_string(), tree_sitter_md::language());
        registry.insert("java".to_string(), tree_sitter_java::language());
        registry.insert("html".to_string(), tree_sitter_html::language());
        registry.insert("css".to_string(), tree_sitter_css::language());
        registry.insert("yaml".to_string(), tree_sitter_yaml::language());
        registry.insert("yml".to_string(), tree_sitter_yaml::language());

        Self { registry }
    }

    pub fn get_language_for_file(&self, path: &Path) -> Option<Language> {
        let ext = path.extension()?.to_str()?;
        self.registry.get(ext).cloned()
    }

    pub fn create_parser_for_file(&self, path: &Path) -> Option<Parser> {
        let lang = self.get_language_for_file(path)?;
        let mut parser = Parser::new();
        parser.set_language(lang).ok()?;
        Some(parser)
    }
}

impl Default for TSManager {
    fn default() -> Self {
        Self::new()
    }
}
