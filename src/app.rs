use std::io;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

mod sidebar;
mod editor;
use sidebar::Sidebar;
use editor::Editor;

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
                _ => {}
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
                    Panel::Sidebar => self.sidebar.next(),
                    Panel::Editor => self.editor.move_cursor_down(),
                    _ => {}
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.active_panel {
                    Panel::Sidebar => self.sidebar.previous(),
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

    pub fn render<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                if self.show_terminal {
                    [Constraint::Min(3), Constraint::Length(10)]
                } else {
                    [Constraint::Min(3), Constraint::Length(0)]
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
        let mut editor_content = Vec::new();
        for (i, line) in self.editor.buffer.lines().enumerate() {
            let line_str = line.to_string();
            // Simple coloring for now (syntax highlighting will come next)
            editor_content.push(ratatui::text::Line::from(line_str));
        }

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

        let editor_widget = Paragraph::new(editor_content).block(editor_block);
        f.render_widget(editor_widget, top_chunks[1]);

        // Show cursor in editor
        if matches!(self.active_panel, Panel::Editor) {
            f.set_cursor(
                top_chunks[1].x + self.editor.cursor_x as u16 + 1,
                top_chunks[1].y + self.editor.cursor_y as u16 + 1,
            );
        }

        // Terminal
        if self.show_terminal {
            let terminal = Block::default()
                .title(" Terminal ")
                .borders(Borders::ALL)
                .border_style(if matches!(self.active_panel, Panel::Terminal) {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
                } else {
                    ratatui::style::Style::default()
                });
            f.render_widget(terminal, chunks[1]);
        }
    }
}
