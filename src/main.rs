use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod app;
mod config;
mod editor;
mod events;
mod lsp;
mod search;
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
    let args: Vec<String> = std::env::args()
        .map(|a| if a == "-?" { "--help".to_string() } else { a })
        .collect();
    let cli = Cli::parse_from(args);

    init_logging();
    log::info!(
        "Klein {} starting in {}",
        env!("CARGO_PKG_VERSION"),
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "<unknown>".into())
    );

    let clipboard = arboard::Clipboard::new().ok();

    // Unified event channel — all sources funnel into this.
    let (event_tx, event_rx) =
        tokio::sync::mpsc::unbounded_channel::<events::klein_event::KleinEvent>();

    // LSP notification sender (forwarded into KleinEvent::Lsp by bridge below).
    let (lsp_notification_tx, mut lsp_notification_rx) =
        tokio::sync::mpsc::unbounded_channel::<lsp::actor::LspServerNotification>();

    // Bridge: LSP notifications → unified channel
    let lsp_event_tx = event_tx.clone();
    tokio::spawn(async move {
        while let Some(notif) = lsp_notification_rx.recv().await {
            let _ = lsp_event_tx.send(events::klein_event::KleinEvent::Lsp(notif));
        }
    });

    // Bridge: crossterm terminal events → unified channel (runs on blocking thread)
    let term_event_tx = event_tx.clone();
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(ev) => {
                    if term_event_tx
                        .send(events::klein_event::KleinEvent::Terminal(ev))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

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

    let timer_manager = events::timers::TimerManager::new(event_tx.clone());

    let mut app = App::new(cli.file, clipboard, lsp_notification_tx, event_tx);
    app.timer_manager = Some(timer_manager);

    let res = run_app(&mut terminal, &mut app, event_rx).await;

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
    mut event_rx: tokio::sync::mpsc::UnboundedReceiver<events::klein_event::KleinEvent>,
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

        // Drain all pending events (non-blocking)
        while let Ok(klein_event) = event_rx.try_recv() {
            match klein_event {
                events::klein_event::KleinEvent::Terminal(ev) => {
                    events::handle_event(app, ev)?;
                }
                events::klein_event::KleinEvent::Lsp(notification) => {
                    events::handle_lsp_notification(app, notification);
                }
                events::klein_event::KleinEvent::Timer(kind) => {
                    events::handle_timer_event(app, kind).await;
                }
                events::klein_event::KleinEvent::InitLsp(path) => {
                    // Try to start server for this file
                    if app.lsp_manager.ensure_server_for_file(&path).await.is_some() {
                        // Once server is up, send didOpen for the file that triggered it
                        // This handles the case where the first didOpen was ignored
                        // because the server was still starting.
                        app.notify_lsp_did_open_for_path(&path);
                    }
                }
            }
        }

        // Yield briefly so the channel can accumulate events
        tokio::time::sleep(std::time::Duration::from_millis(8)).await;

        if app.should_quit {
            log::info!("Klein shutting down");
            return Ok(());
        }
    }
}

/// Initialize file-based logging.
///
/// Logs are written to the Klein config directory (e.g., `~/.config/Klein/klein.log`
/// on Linux, `%APPDATA%/Klein/klein.log` on Windows). The log level defaults to
/// `warn` but can be overridden via the `KLEIN_LOG` environment variable.
fn init_logging() {
    use std::io::Write;

    let log_path = directories::ProjectDirs::from("", "", "Klein")
        .map(|dirs| {
            let log_dir = dirs.config_dir().to_path_buf();
            let _ = std::fs::create_dir_all(&log_dir);
            log_dir.join("klein.log")
        });

    if let Some(path) = log_path {
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let env_filter = std::env::var("KLEIN_LOG").unwrap_or_else(|_| "warn".to_string());
            let _ = env_logger::Builder::new()
                .parse_filters(&env_filter)
                .target(env_logger::Target::Pipe(Box::new(file)))
                .format(|buf, record| {
                    writeln!(
                        buf,
                        "[{} {} {}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                        record.level(),
                        record.target(),
                        record.args()
                    )
                })
                .try_init();
        }
    }
}
