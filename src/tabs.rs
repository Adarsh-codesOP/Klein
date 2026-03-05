use crate::editor::Editor;

pub struct TabState {
    pub editor: Editor,
}

impl TabState {
    pub fn new() -> Self {
        TabState {
            editor: Editor::new(),
        }
    }
}
