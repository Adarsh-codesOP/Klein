use std::io;
use std::cell::Cell;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::sidebar::Sidebar;
use crate::editor::Editor;
use crate::terminal::Terminal;

pub enum Panel {
    Sidebar,
    Editor,
    Terminal,
}

pub struct App {
    pub active_panel: Panel,
    pub show_sidebar: bool,
    pub show_terminal: bool,
    pub should_quit: bool,
    pub sidebar: Sidebar,
    pub editor: Editor,
    pub terminal: Terminal,
    pub last_editor_height: Cell<usize>,
    pub show_help: bool,
    pub terminal_scroll: usize,
}

impl App {
    pub fn new() -> App {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        App {
            active_panel: Panel::Editor,
            show_sidebar: true,
            show_terminal: true,
            should_quit: false,
            sidebar: Sidebar::new(&current_dir),
            editor: Editor::new(),
            terminal: Terminal::new(current_dir),
            last_editor_height: Cell::new(20),
            show_help: true,
            terminal_scroll: 0,
        }
    }

    pub fn handle_event(&mut self, event: Event) -> io::Result<()> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                self.handle_key_event(key)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> io::Result<()> {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('b') => self.show_sidebar = !self.show_sidebar,
                KeyCode::Char('`') => self.show_terminal = !self.show_terminal,
                KeyCode::Char('s') => {
                    let _ = self.editor.save();
                }
                KeyCode::Char('r') => {
                    if let Some(path) = &self.editor.path {
                        let path_str: String = path.to_string_lossy().into_owned();
                        let cmd = if path_str.ends_with(".rs") {
                            "cargo run\r\n".to_string()
                        } else if path_str.ends_with(".py") {
                            format!("python {}\r\n", path_str)
                        } else if path_str.ends_with(".js") {
                            format!("node {}\r\n", path_str)
                        } else {
                            "".to_string()
                        };
                        if !cmd.is_empty() {
                            self.terminal.write(&cmd);
                            self.show_terminal = true;
                            self.active_panel = Panel::Terminal;
                        }
                    }
                }
                KeyCode::Char('f') => {
                    self.editor.is_searching = true;
                    self.editor.search_query.clear();
                }
                KeyCode::Char('c') => {
                    self.editor.copy();
                }
                KeyCode::Char('h') => self.show_help = !self.show_help,
                KeyCode::Right => {
                    self.active_panel = match self.active_panel {
                        Panel::Sidebar => Panel::Editor,
                        Panel::Editor => if self.show_terminal { Panel::Terminal } else { Panel::Sidebar },
                        Panel::Terminal => if self.show_sidebar { Panel::Sidebar } else { Panel::Editor },
                    };
                }
                KeyCode::Left => {
                    self.active_panel = match self.active_panel {
                        Panel::Sidebar => if self.show_terminal { Panel::Terminal } else { Panel::Editor },
                        Panel::Editor => if self.show_sidebar { Panel::Sidebar } else { Panel::Terminal },
                        Panel::Terminal => Panel::Editor,
                    };
                }
                _ => {}
            }
            return Ok(());
        }

        if self.editor.is_searching {
            match key.code {
                KeyCode::Char(c) => self.editor.search_query.push(c),
                KeyCode::Backspace => {
                    self.editor.search_query.pop();
                }
                KeyCode::Enter => {
                    self.editor.search(&self.editor.search_query.clone());
                    self.editor.is_searching = false;
                }
                KeyCode::Esc => {
                    self.editor.is_searching = false;
                }
                _ => {}
            }
            return Ok(());
        }

        if matches!(self.active_panel, Panel::Terminal) {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.terminal.write("\x03"); // Ctrl+C
                }
                KeyCode::Char(c) => {
                    self.terminal_scroll = 0;
                    self.terminal.write(&c.to_string());
                }
                KeyCode::Enter => {
                    self.terminal_scroll = 0;
                    self.terminal.write("\r");
                }
                KeyCode::Backspace => {
                    self.terminal_scroll = 0;
                    self.terminal.write("\x08");
                }
                KeyCode::Delete => self.terminal.write("\x1b[3~"),
                KeyCode::Up => self.terminal.write("\x1b[A"),
                KeyCode::Down => self.terminal.write("\x1b[B"),
                KeyCode::Right => self.terminal.write("\x1b[C"),
                KeyCode::Left => self.terminal.write("\x1b[D"),
                KeyCode::PageUp => {
                    self.terminal_scroll = self.terminal_scroll.saturating_add(5);
                }
                KeyCode::PageDown => {
                    self.terminal_scroll = self.terminal_scroll.saturating_sub(5);
                }
                KeyCode::Tab => {
                     // Terminal Tab support? 
                     // self.terminal.write("\t");
                }
                _ => {}
            }
            // Switch away from terminal with Esc? Or just Ctrl+Arrows
            if key.code == KeyCode::Esc {
                self.active_panel = Panel::Editor;
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                match self.active_panel {
                    Panel::Sidebar => {
                        if let Some(path) = self.sidebar.next() {
                            let _ = self.editor.open(path);
                        }
                    }
                    Panel::Editor => self.editor.move_cursor_down(self.last_editor_height.get()),
                    _ => {}
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.active_panel {
                    Panel::Sidebar => {
                        if let Some(path) = self.sidebar.previous() {
                            let _ = self.editor.open(path);
                        }
                    }
                    Panel::Editor => self.editor.move_cursor_up(),
                    _ => {}
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if matches!(self.active_panel, Panel::Editor) {
                    self.editor.move_cursor_left();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if matches!(self.active_panel, Panel::Editor) {
                    self.editor.move_cursor_right();
                }
            }
            KeyCode::Enter => {
                match self.active_panel {
                    Panel::Sidebar => {
                        if let Ok(Some(path)) = self.sidebar.toggle_selected() {
                            let _ = self.editor.open(path);
                            self.active_panel = Panel::Editor;
                        }
                    }
                    Panel::Editor => {
                        self.editor.insert_char('\n');
                        self.editor.cursor_y += 1;
                        self.editor.cursor_x = 0;
                    }
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                if matches!(self.active_panel, Panel::Editor) {
                    self.editor.delete_char();
                }
            }
            KeyCode::Char(c) => {
                if matches!(self.active_panel, Panel::Editor) {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.editor.insert_char(c);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn render(&self, f: &mut Frame<'_>) {
        let constraints = vec![
            if self.show_help { Constraint::Length(4) } else { Constraint::Length(0) }, // Help (Height 4 for 2 lines + borders)
            Constraint::Fill(1), // Main
            if self.show_terminal { Constraint::Length(10) } else { Constraint::Length(0) }, // Terminal
            Constraint::Length(1), // Status Bar
        ];
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(f.size());

        // Help Section
        if self.show_help {
            let help_text = "Navigation: [Ctrl+Arrows] Panels | [Arrows/hjkl] Move | [Enter] Select/Open | [Esc] Term Out | [Ctrl+H] Toggle Help\nShortcuts: [Ctrl+B] Sidebar | [Ctrl+`] Term | [Ctrl+S] Save | [Ctrl+R] Run | [Ctrl+Q] Quit";
            let help_block = Block::default()
                .title(" Klein IDE Help ")
                .borders(Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
            let help_widget = Paragraph::new(help_text)
                .block(help_block)
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(help_widget, chunks[0]);
        }

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                if self.show_sidebar {
                    [Constraint::Percentage(20), Constraint::Percentage(80)]
                } else {
                    [Constraint::Percentage(0), Constraint::Percentage(100)]
                }
                .as_ref(),
            )
            .split(chunks[1]);

        // Sidebar
        if self.show_sidebar {
            let mut list_items = Vec::new();
            for (i, (path, depth, is_dir)) in self.sidebar.flat_list.iter().enumerate() {
                let prefix = "  ".repeat(*depth);
                let icon = if *is_dir { "📁 " } else { "📄 " };
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let mut style = ratatui::style::Style::default();
                if i == self.sidebar.selected_index {
                    style = style.bg(ratatui::style::Color::DarkGray).fg(ratatui::style::Color::White);
                }
                list_items.push(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(prefix, style),
                    ratatui::text::Span::styled(icon, style),
                    ratatui::text::Span::styled(name, style),
                ]));
            }

            let sidebar_block = Block::default()
                .title(" File Explorer ")
                .borders(Borders::ALL)
                .border_style(if matches!(self.active_panel, Panel::Sidebar) {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
                } else {
                    ratatui::style::Style::default()
                });
            
            let sidebar_widget = Paragraph::new(list_items).block(sidebar_block);
            f.render_widget(sidebar_widget, main_chunks[0]);
        }

        // Editor
        let editor_rect = main_chunks[1];
        let editor_block = Block::default()
            .title(format!(
                " Editor - {} ",
                self.editor
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "No file".to_string())
            ))
            .borders(Borders::ALL)
            .border_style(if matches!(self.active_panel, Panel::Editor) {
                ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
            } else {
                ratatui::style::Style::default()
            });

        let inner_rect = editor_block.inner(editor_rect);
        self.last_editor_height.set(inner_rect.height as usize);
        let highlighted_lines = self.editor.get_highlighted_lines(
            inner_rect.width as usize,
            inner_rect.height as usize,
        );

        let editor_widget = Paragraph::new(highlighted_lines).block(editor_block);
        f.render_widget(editor_widget, editor_rect);

        // Render search box
        if self.editor.is_searching {
            let search_area = Rect::new(
                inner_rect.x + 2,
                inner_rect.y + inner_rect.height.saturating_sub(2),
                inner_rect.width.saturating_sub(4),
                1,
            );
            let search_block = Block::default()
                .title(" Search ")
                .borders(Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
            let search_text = format!("Find: {}", self.editor.search_query);
            f.render_widget(Paragraph::new(search_text).block(search_block), search_area);
        }

        // Show cursor in editor
        if matches!(self.active_panel, Panel::Editor) {
            f.set_cursor(
                inner_rect.x + self.editor.cursor_x as u16,
                inner_rect.y + (self.editor.cursor_y - self.editor.scroll_y) as u16,
            );
        }

        // Terminal
        if self.show_terminal {
            let output_raw = self.terminal.output.lock().unwrap();
            let output = strip_ansi(&output_raw);
            
            let lines: Vec<&str> = output.lines().collect();
            let height = chunks[2].height.saturating_sub(2) as usize;
            
            let max_scroll = lines.len().saturating_sub(height);
            let scroll = self.terminal_scroll.min(max_scroll);

            let start = lines.len().saturating_sub(height).saturating_sub(scroll);
            let end = lines.len().saturating_sub(scroll);
            
            let terminal_lines: Vec<ratatui::text::Line<'_>> = lines[start..end]
                .iter()
                .map(|l| ratatui::text::Line::from(l.to_string()))
                .collect();

            let terminal_block = Block::default()
                .title(" Terminal ")
                .borders(Borders::ALL)
                .border_style(if matches!(self.active_panel, Panel::Terminal) {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
                } else {
                    ratatui::style::Style::default()
                });
            
            let terminal_widget = Paragraph::new(terminal_lines).block(terminal_block);
            f.render_widget(terminal_widget, chunks[2]);

            // Show cursor in terminal if active and not scrolled back
            if matches!(self.active_panel, Panel::Terminal) && scroll == 0 {
                let last_line = lines.last().copied().unwrap_or("");
                let inner = Block::default().borders(Borders::ALL).inner(chunks[2]);
                f.set_cursor(
                    inner.x + last_line.len() as u16,
                    inner.y + lines.len().min(height).saturating_sub(1) as u16,
                );
            }
        }

        // Status Bar
        let status_bar = Block::default()
            .borders(Borders::TOP)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
        
        let status_text = format!(
            " {} | {} | Ln {}, Col {} | Ctrl+H: Help | Ctrl+Arrows: Panel ",
            if let Some(path) = &self.editor.path {
                path.file_name().unwrap_or_default().to_string_lossy().into_owned()
            } else {
                "No file".to_string()
            },
            if matches!(self.active_panel, Panel::Editor) { "Mode: EDIT" } 
            else if matches!(self.active_panel, Panel::Sidebar) { "Mode: EXPLORE" }
            else { "Mode: TERM" },
            self.editor.cursor_y + 1,
            self.editor.cursor_x + 1,
        );
        
        let status_paragraph = Paragraph::new(status_text)
            .block(status_bar)
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::Gray));
        
        f.render_widget(status_paragraph, chunks[3]);
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' {
            let _start = i;
            i += 1;
            if i >= chars.len() { break; } 
            match chars[i] {
                '[' => { // CSI
                    i += 1;
                    let mut found = false;
                    while i < chars.len() {
                        let c = chars[i];
                        i += 1;
                        if (c as u32) >= 0x40 && (c as u32) <= 0x7E {
                            found = true;
                            break;
                        }
                    }
                    if !found { break; } // Truncate partial
                }
                ']' => { // OSC (Window title etc)
                    i += 1;
                    let mut found = false;
                    while i < chars.len() {
                        if chars[i] == '\x07' {
                            i += 1;
                            found = true;
                            break;
                        }
                        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i+1] == '\\' {
                            found = true;
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    if !found { break; }
                }
                '(' | ')' | '*' | '+' | '-' | '.' | '/' => { // Charset
                    if i + 1 >= chars.len() { break; }
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        } else {
            let c = chars[i];
            if c == '\r' { i += 1; continue; } // Skip \r for clean display in TUI
            if (c as u32) >= 32 || c == '\n' || c == '\t' {
                result.push(c);
            }
            i += 1;
        }
    }
    result
}
