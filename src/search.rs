use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    File,
    Grep,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub content: String,
}

pub struct PickerState {
    pub active: bool,
    pub mode: SearchMode,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll: usize,
}

impl Default for PickerState {
    fn default() -> Self {
        Self {
            active: false,
            mode: SearchMode::File,
            query: String::new(),
            results: Vec::new(),
            selected_index: 0,
            scroll: 0,
        }
    }
}

pub fn run_grep(query: &str) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let output = Command::new("rg")
        .arg("--line-number")
        .arg("--column")
        .arg("--color")
        .arg("never")
        .arg("--no-heading")
        .arg("--smart-case")
        .arg(query)
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, ':').collect();
                if parts.len() >= 3 {
                    let path = PathBuf::from(parts[0]);
                    let line_num = parts[1].parse::<usize>().ok()?.saturating_sub(1);
                    let col_num = parts[2].parse::<usize>().ok()?.saturating_sub(1);
                    let content = parts.get(3).unwrap_or(&"").trim().to_string();
                    Some(SearchResult {
                        path,
                        line: Some(line_num),
                        column: Some(col_num),
                        content,
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}

pub fn run_file_search(query: &str) -> Vec<SearchResult> {
    let rg_output = Command::new("rg").arg("--files").output();

    if let Ok(rg_output) = rg_output {
        let files = String::from_utf8_lossy(&rg_output.stdout);

        if query.is_empty() {
            return files
                .lines()
                .map(|f| SearchResult {
                    path: PathBuf::from(f),
                    line: None,
                    column: None,
                    content: f.to_string(),
                })
                .collect();
        }

        let mut child = if let Ok(c) = Command::new("fzf")
            .arg("-f")
            .arg(query)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
        {
            c
        } else {
            return Vec::new();
        };

        {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(files.as_bytes());
            }
        }

        if let Ok(output) = child.wait_with_output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .map(|f| SearchResult {
                    path: PathBuf::from(f),
                    line: None,
                    column: None,
                    content: f.to_string(),
                })
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}
