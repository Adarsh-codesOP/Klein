use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;

pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
    pub is_expanded: bool,
}

impl FileNode {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let is_dir = path.is_dir();
        FileNode {
            path,
            name,
            is_dir,
            children: None,
            is_expanded: false,
        }
    }

    pub fn expand(&mut self) -> Result<()> {
        if self.is_dir && self.children.is_none() {
            let mut children = Vec::new();
            for entry in fs::read_dir(&self.path)? {
                let entry = entry?;
                children.push(FileNode::new(entry.path()));
            }
            children.sort_by(|a, b| {
                if a.is_dir == b.is_dir {
                    a.name.cmp(&b.name)
                } else {
                    b.is_dir.cmp(&a.is_dir)
                }
            });
            self.children = Some(children);
        }
        self.is_expanded = true;
        Ok(())
    }

    pub fn collapse(&mut self) {
        self.is_expanded = false;
    }

    /// Re-read this directory's children from disk, preserving expanded state of
    /// subdirectories, then recursively refresh any that were expanded.
    pub fn refresh(&mut self) -> Result<()> {
        if !self.is_dir || !self.is_expanded {
            return Ok(());
        }
        let mut new_children = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let child_path = entry.path();
            let mut child = FileNode::new(child_path.clone());
            // Preserve was-expanded flag from the previous tree
            if let Some(existing) = self.children.as_ref()
                .and_then(|cs| cs.iter().find(|c| c.path == child_path))
            {
                child.is_expanded = existing.is_expanded;
            }
            new_children.push(child);
        }
        new_children.sort_by(|a, b| {
            if a.is_dir == b.is_dir { a.name.cmp(&b.name) } else { b.is_dir.cmp(&a.is_dir) }
        });
        self.children = Some(new_children);
        // Recurse into any still-expanded subdirectories
        if let Some(children) = &mut self.children {
            for child in children.iter_mut() {
                if child.is_expanded {
                    let _ = child.refresh();
                }
            }
        }
        Ok(())
    }
}

pub struct Sidebar {
    pub root: FileNode,
    pub selected_index: usize,
    pub flat_list: Vec<(PathBuf, usize, bool)>, // (path, depth, is_dir)
    pub offset: usize,
    pub last_height: std::cell::Cell<usize>,
    pub show_hidden: bool,
}

impl Sidebar {
    pub fn new(root_path: &Path) -> Self {
        let mut root = FileNode::new(root_path.to_path_buf());
        let _ = root.expand(); // Try to expand root by default
        let mut sidebar = Sidebar {
            root,
            selected_index: 0,
            flat_list: Vec::new(),
            offset: 0,
            last_height: std::cell::Cell::new(20),
            show_hidden: false,
        };
        sidebar.update_flat_list();
        sidebar
    }

    pub fn update_flat_list(&mut self) {
        let mut list = Vec::new();
        self.flatten(&self.root, 0, &mut list);
        self.flat_list = list;
    }

    fn flatten(&self, node: &FileNode, depth: usize, list: &mut Vec<(PathBuf, usize, bool)>) {
        // Filter hidden entries (names starting with '.') at non-root depth
        if depth > 0 && !self.show_hidden && node.name.starts_with('.') {
            return;
        }

        list.push((node.path.clone(), depth, node.is_dir));

        if node.is_expanded {
            if let Some(children) = &node.children {
                for child in children {
                    self.flatten(child, depth + 1, list);
                }
            }
        } else if depth == 0 {
            // If root is collapsed but we are at depth 0, we still want to show its children if it's the "workspace"
            if let Some(children) = &node.children {
                for child in children {
                    self.flatten(child, depth + 1, list);
                }
            }
        }
    }

    pub fn go_to_first(&mut self) -> Option<PathBuf> {
        if !self.flat_list.is_empty() {
            self.selected_index = 0;
            self.offset = 0;
            let (path, _, is_dir) = &self.flat_list[0];
            if !*is_dir {
                return Some(path.clone());
            }
        }
        None
    }

    pub fn go_to_last(&mut self) -> Option<PathBuf> {
        if !self.flat_list.is_empty() {
            self.selected_index = self.flat_list.len() - 1;
            self.adjust_scroll();
            let (path, _, is_dir) = &self.flat_list[self.selected_index];
            if !*is_dir {
                return Some(path.clone());
            }
        }
        None
    }

    /// Re-read all expanded directories and rebuild the flat list.
    /// Call this after any filesystem change (e.g. saving a new file).
    pub fn refresh(&mut self) {
        let _ = self.root.refresh();
        self.update_flat_list();
        // Clamp selection in case entries were removed
        if !self.flat_list.is_empty() && self.selected_index >= self.flat_list.len() {
            self.selected_index = self.flat_list.len() - 1;
        }
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.selected_index = 0;
        self.offset = 0;
        self.update_flat_list();
    }

    pub fn page_next(&mut self) -> Option<PathBuf> {
        if !self.flat_list.is_empty() {
            let height = self.last_height.get().saturating_sub(2).max(1);
            self.selected_index = (self.selected_index + height).min(self.flat_list.len() - 1);
            self.adjust_scroll();
            let (path, _, is_dir) = &self.flat_list[self.selected_index];
            if !*is_dir {
                return Some(path.clone());
            }
        }
        None
    }

    pub fn page_previous(&mut self) -> Option<PathBuf> {
        if !self.flat_list.is_empty() {
            let height = self.last_height.get().saturating_sub(2).max(1);
            self.selected_index = self.selected_index.saturating_sub(height);
            self.adjust_scroll();
            let (path, _, is_dir) = &self.flat_list[self.selected_index];
            if !*is_dir {
                return Some(path.clone());
            }
        }
        None
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        if !self.flat_list.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.flat_list.len();
            self.adjust_scroll();
            let (path, _, is_dir) = &self.flat_list[self.selected_index];
            if !*is_dir {
                return Some(path.clone());
            }
        }
        None
    }

    pub fn previous(&mut self) -> Option<PathBuf> {
        if !self.flat_list.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.flat_list.len() - 1;
            }
            self.adjust_scroll();
            let (path, _, is_dir) = &self.flat_list[self.selected_index];
            if !*is_dir {
                return Some(path.clone());
            }
        }
        None
    }

    fn adjust_scroll(&mut self) {
        let height = self.last_height.get().saturating_sub(2); // Account for borders
        if height == 0 {
            return;
        }

        if self.selected_index >= self.offset + height {
            self.offset = self.selected_index.saturating_sub(height).saturating_add(1);
        } else if self.selected_index < self.offset {
            self.offset = self.selected_index;
        }
    }

    pub fn toggle_selected(&mut self) -> Result<Option<PathBuf>> {
        if self.flat_list.is_empty() {
            return Ok(None);
        }

        let (path, _, is_dir) = &self.flat_list[self.selected_index];
        let path_clone = path.clone();

        if *is_dir {
            Self::toggle_node(&mut self.root, &path_clone)?;
            self.update_flat_list();
            Ok(None)
        } else {
            Ok(Some(path_clone))
        }
    }

    fn toggle_node(node: &mut FileNode, target_path: &Path) -> Result<bool> {
        if node.path == target_path {
            if node.is_expanded {
                node.collapse();
            } else {
                node.expand()?;
            }
            return Ok(true);
        }

        if let Some(children) = &mut node.children {
            for child in children {
                if Self::toggle_node(child, target_path)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}
