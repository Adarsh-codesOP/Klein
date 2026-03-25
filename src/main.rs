use std::io;
use arboard;
use crossterm::{
    event::{self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use anyhow::Result;
use clap::Parser;

mod app;
mod sidebar;
mod editor;
mod terminal;
mod ui;
mod events;
mod config;
mod tabs;

use crate::app::App;

#[derive(Parser)]
#[command(name = "klein")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A professional terminal-based text editor with IDE-like features")]
#[command(long_about = "\
Klein is a lightweight, terminal-based text editor (TIDE) built in Rust.\n\
\n\
USAGE:\n\
  klein [FILE]\n\
\n\
ARGUMENTS:\n\
  [FILE]   Optional file to open on startup.\n\
           - Relative path: resolved from the directory you run klein in.\n\
             e.g.  klein README.md\n\
           - Absolute path: used as-is, sidebar and terminal start in that file's folder.\n\
             e.g.  klein /home/user/project/src/main.rs\n\
           If the file does not exist it is created (empty) when you save.\n\
\n\
With no FILE argument Klein opens in the current working directory.")]
struct Cli {
    /// File to open on startup (relative or absolute path)
    #[arg(value_name = "FILE")]
    file: Option<String>,

    /// Print help (alias for -h / --help)
    #[arg(short = '?', action = clap::ArgAction::Help, hide = true)]
    help_alias: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Resolve the optional file argument to an absolute path
    let initial_file = cli.file.map(|f| {
        let p = std::path::PathBuf::from(&f);
        if p.is_absolute() {
            p
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(p)
        }
    });

    // Initialize clipboard BEFORE entering raw mode.
    // On Wayland compositors the clipboard context must be created while the
    // terminal is still in normal mode; doing it afterwards can silently fail.
    let clipboard = arboard::Clipboard::new().ok();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(initial_file, clipboard);
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

        if event::poll(std::time::Duration::from_millis(16))? {
            let ev = event::read()?;
            events::handle_event(app, ev)?;
        }

        // Detect when the embedded shell exits (e.g. user typed `exit`)
        if app.terminal.is_exited() && !app.terminal_triggered_quit && !app.show_quit_confirm && !app.should_quit {
            app.terminal_triggered_quit = true;
            app.show_quit_confirm = true;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
