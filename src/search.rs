use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, Sink, SinkMatch};
use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    File,
    Grep,
    Lsp,
    CodeAction,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub content: Option<String>,
}

pub struct PickerState {
    pub active: bool,
    pub mode: SearchMode,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub scroll: usize,
    pub preview: Option<Vec<String>>,
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
            preview: None,
        }
    }
}

pub fn run_grep(query: &str) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let Ok(matcher) = RegexMatcher::new_line_matcher(&format!("(?i){}", query)) else {
        return Vec::new();
    };

    let results = Arc::new(Mutex::new(Vec::new()));

    // Create a parallel walker
    let walker = WalkBuilder::new("./")
        .hidden(true) // Skip .git and hidden folders
        .git_ignore(true) // Respect .gitignore
        .build_parallel();

    walker.run(|| {
        let results = results.clone();
        let matcher = matcher.clone();
        let mut searcher = Searcher::new();

        Box::new(move |result| {
            let Ok(entry) = result else {
                return WalkState::Continue;
            };
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                return WalkState::Continue;
            }

            let path = entry.path().to_path_buf();
            let mut local_results = Vec::new();

            let _ = searcher.search_path(
                &matcher,
                &path,
                SearchSink {
                    path: &path,
                    results: &mut local_results,
                },
            );

            if !local_results.is_empty() {
                let mut global = results.lock().unwrap();
                global.extend(local_results);
                if global.len() > 2000 {
                    return WalkState::Quit;
                }
            }

            WalkState::Continue
        })
    });

    let mut final_results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
    final_results.truncate(2000);
    final_results
}

struct SearchSink<'a> {
    path: &'a Path,
    results: &'a mut Vec<SearchResult>,
}

impl<'a> Sink for SearchSink<'a> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, line: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        self.results.push(SearchResult {
            path: self.path.to_path_buf(),
            line: Some(line.line_number().unwrap_or(1).saturating_sub(1) as usize),
            content: None,
        });
        Ok(true)
    }
}

pub fn run_file_search(query: &str) -> Vec<SearchResult> {
    let matcher = SkimMatcherV2::default();

    // Step 1: Walk files (efficiently skip hidden and gitignored)
    let walker = WalkBuilder::new("./")
        .hidden(true) // IMPORTANT: This prevents .git objects from appearing
        .git_ignore(true) // Respect .gitignore
        .build();

    let mut file_paths = Vec::new();
    for result in walker {
        let Ok(entry) = result else { continue };
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            file_paths.push(entry.path().to_path_buf());
        }
        if file_paths.len() > 10000 {
            break;
        } // Limit for UI sanity
    }

    if query.is_empty() {
        return file_paths
            .into_iter()
            .take(1000)
            .map(|path| SearchResult { path, line: None, content: None })
            .collect();
    }

    // Step 2: Fuzzy Match in parallel using Rayon
    let mut scored_files: Vec<(i64, SearchResult)> = file_paths
        .into_par_iter()
        .filter_map(|path| {
            let path_str = path.to_string_lossy().to_string();
            matcher
                .fuzzy_match(&path_str, query)
                .map(|score| (score, SearchResult { path, line: None, content: None }))
        })
        .collect();

    // Step 3: Sort by score
    scored_files.par_sort_by(|a, b| b.0.cmp(&a.0));

    scored_files.into_iter().take(1000).map(|f| f.1).collect()
}

pub fn load_preview_lines(path: &Path, line: usize, radius: usize) -> Option<Vec<String>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let start = line.saturating_sub(radius);
    let end = line + radius;

    let lines: Vec<String> = reader
        .lines()
        .enumerate()
        .skip(start)
        .take(end - start + 1)
        .map(|(i, l)| {
            let content = l.unwrap_or_default();
            if i == line {
                format!("> {}", content)
            } else {
                format!("  {}", content)
            }
        })
        .collect();

    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}
