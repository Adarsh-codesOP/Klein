use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod app;
mod config;
mod editor;
mod events;
mod sidebar;
mod tabs;
mod terminal;
mod ui;

use crate::app::App;

#[derive(Parser)]
#[command(name = "klein")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A professional terminal-based text editor with IDE-like features")]
#[command(
    long_about = "Klein is a lightweight, terminal-based text editor built in Rust. It provides an IDE-like interface using ratatui for the user interface and portable-pty for terminal integration, giving developers a keyboard-centric coding environment directly in the command line."
)]
struct Cli {
    #[arg(help = "Optional file path to open on startup")]
    file: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().map(|a| if a == "-?" { "--help".to_string() } else { a }).collect();
    let cli = Cli::parse_from(args);

    let clipboard = arboard::Clipboard::new().ok();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(cli.file, clipboard);
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: io::Write + ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if !app.terminal_restarting {
            let mut child_exited = false;
            if let Ok(mut child) = app.terminal.child.lock() {
                if let Ok(Some(_status)) = child.try_wait() {
                    child_exited = true;
                }
            }
            if child_exited {
                app.terminal_restarting = true;
                app.show_quit_confirm = true;
            }
        } else if !app.show_quit_confirm && !app.should_quit {
            app.terminal.restart();
            app.terminal_restarting = false;
            app.active_panel = crate::app::Panel::Terminal;
            app.show_terminal = true;
            app.maximized = crate::app::Maximized::None;
        }

        if event::poll(std::time::Duration::from_millis(16))? {
            let ev = event::read()?;
            events::handle_event(app, ev)?;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
