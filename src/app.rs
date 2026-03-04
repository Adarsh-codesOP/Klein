use std::io;
use std::cell::Cell;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

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
            terminal: Terminal::new(),
            last_editor_height: Cell::new(20),
        }
    }

    pub fn handle_event(&mut self, event: Event) -> io::Result<()> {
        if let Event::Key(key) = event {
            self.handle_key_event(key)?;
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
                KeyCode::Char('v') => {
                    self.editor.paste();
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
                KeyCode::Char(c) => self.terminal.write(&c.to_string()),
                KeyCode::Enter => self.terminal.write("\r\n"),
                KeyCode::Backspace => self.terminal.write("\x08"), // Standard backspace for many PTYs
                KeyCode::Up => self.terminal.write("\x1b[A"),
                KeyCode::Down => self.terminal.write("\x1b[B"),
                KeyCode::Right => self.terminal.write("\x1b[C"),
                KeyCode::Left => self.terminal.write("\x1b[D"),
                KeyCode::Tab => {
                     // We use Tab for panel switching, but what if terminal needs it?
                     // Let's keep Tab for panel switching but maybe Ctrl+Tab for terminal Tab?
                     // For now, let's allow Esc to switch panel or just stay as is.
                }
                _ => {}
            }
            // Switch away from terminal with Tab
            if key.code == KeyCode::Tab {
                self.active_panel = if self.show_sidebar { Panel::Sidebar } else { Panel::Editor };
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Tab => {
                self.active_panel = match self.active_panel {
                    Panel::Sidebar => Panel::Editor,
                    Panel::Editor => {
                        if self.show_terminal {
                            Panel::Terminal
                        } else if self.show_sidebar {
                            Panel::Sidebar
                        } else {
                            Panel::Editor
                        }
                    }
                    Panel::Terminal => {
                        if self.show_sidebar {
                            Panel::Sidebar
                        } else {
                            Panel::Editor
                        }
                    }
                };
            }
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                if self.show_terminal {
                    [
                        Constraint::Min(3),
                        Constraint::Length(10),
                        Constraint::Length(1), // Status Bar
                    ]
                } else {
                    [
                        Constraint::Min(3),
                        Constraint::Length(0),
                        Constraint::Length(1), // Status Bar
                    ]
                }
                .as_ref(),
            )
            .split(f.size());

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                if self.show_sidebar {
                    [Constraint::Percentage(20), Constraint::Percentage(80)]
                } else {
                    [Constraint::Percentage(0), Constraint::Percentage(100)]
                }
                .as_ref(),
            )
            .split(chunks[0]);

        // Sidebar
        if self.show_sidebar {
            let mut list_items = Vec::new();
            for (i, (path, depth, is_dir)) in self.sidebar.flat_list.iter().enumerate() {
                let prefix = "  ".repeat(*depth - 1);
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
            f.render_widget(sidebar_widget, top_chunks[0]);
        }

        // Editor
        let editor_rect = top_chunks[1];
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
            let stripped = strip_ansi_escapes::strip(&*output_raw).unwrap_or_else(|_| output_raw.as_bytes().to_vec());
            let output = String::from_utf8_lossy(&stripped);
            
            let terminal_lines: Vec<ratatui::text::Line<'_>> = output
                .lines()
                .rev() // Show last lines
                .take(chunks[1].height.saturating_sub(2) as usize)
                .map(|l: &str| ratatui::text::Line::from(l.to_string()))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
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
            f.render_widget(terminal_widget, chunks[1]);
        }

        // Status Bar
        let status_bar = Block::default()
            .borders(Borders::TOP)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
        
        let status_text = format!(
            " {} | {} | Ln {}, Col {} | Ctrl+S: Save | Ctrl+R: Run | Ctrl+F: Search ",
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
        
        f.render_widget(status_paragraph, chunks[2]);
    }
}
