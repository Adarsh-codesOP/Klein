use crate::editor::Editor;

pub struct TabState {
    pub editor: Editor,
}

impl Default for TabState {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState {
    pub fn new() -> Self {
        TabState {
            editor: Editor::new(),
        }
    }
}
