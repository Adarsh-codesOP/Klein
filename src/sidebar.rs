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
}

pub struct Sidebar {
    pub root: FileNode,
    pub selected_index: usize,
    pub flat_list: Vec<(PathBuf, usize, bool)>, // (path, depth, is_dir)
}

impl Sidebar {
    pub fn new(root_path: &Path) -> Self {
        let mut root = FileNode::new(root_path.to_path_buf());
        let _ = root.expand(); // Try to expand root by default
        let mut sidebar = Sidebar {
            root,
            selected_index: 0,
            flat_list: Vec::new(),
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
        // Skip root itself if it's the current project dir? No, let's show it or its children.
        // Usually we show children of root.
        if depth > 0 {
            list.push((node.path.clone(), depth, node.is_dir));
        }

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

    pub fn next(&mut self) {
        if !self.flat_list.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.flat_list.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.flat_list.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.flat_list.len() - 1;
            }
        }
    }

    pub fn toggle_selected(&mut self) -> Result<Option<PathBuf>> {
        if self.flat_list.is_empty() {
            return Ok(None);
        }

        let (path, _, is_dir) = &self.flat_list[self.selected_index];
        let path_clone = path.clone();

        if *is_dir {
            self.toggle_node(&mut self.root, &path_clone)?;
            self.update_flat_list();
            Ok(None)
        } else {
            Ok(Some(path_clone))
        }
    }

    fn toggle_node(&mut self, node: &mut FileNode, target_path: &Path) -> Result<bool> {
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
                if self.toggle_node(child, target_path)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}
