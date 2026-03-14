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

use klein_ide::app::App;
use klein_ide::events;
use klein_ide::init_logging;
use klein_ide::ui;

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
        tokio::sync::mpsc::unbounded_channel::<klein_ide::lsp::actor::LspServerNotification>();

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
        while let Ok(ev) = event::read() {
            if term_event_tx
                .send(events::klein_event::KleinEvent::Terminal(ev))
                .is_err()
            {
                break;
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
            app.active_panel = klein_ide::app::Panel::Terminal;
            app.show_terminal = true;
            app.maximized = klein_ide::app::Maximized::None;
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
                    events::handle_timer_event(app, kind);
                }
                events::klein_event::KleinEvent::InitLsp(path) => {
                    log::info!("received InitLsp for {}", path.display());
                    if app
                         .lsp_manager
                         .ensure_server_for_file(&path)
                         .await
                         .is_some()
                     {
                         log::info!("LSP server confirmed for {}", path.display());
                         app.notify_lsp_did_open_for_path(&path);
                     } else {
                         log::warn!("LSP server NOT found or failed to start for {}", path.display());
                     }
                }
                events::klein_event::KleinEvent::GotoDefinition => {
                    app.trigger_goto_definition();
                }
                events::klein_event::KleinEvent::FindReferences => {
                    app.trigger_find_references();
                }
                events::klein_event::KleinEvent::FormatDocument => {
                    app.trigger_format_document();
                }
                events::klein_event::KleinEvent::Rename => {
                    app.execute_rename();
                }
                events::klein_event::KleinEvent::CodeAction => {
                    app.trigger_code_action();
                }
                events::klein_event::KleinEvent::CompletionResponse(resp, path, pos) => {
                    app.handle_completion_response(resp, path, pos);
                }
                events::klein_event::KleinEvent::HoverResponse(resp, path, pos) => {
                    app.handle_hover_response(resp, path, pos);
                }
                events::klein_event::KleinEvent::DefinitionResponse(resp, path) => {
                    app.handle_definition_response(resp, path);
                }
                events::klein_event::KleinEvent::ReferencesResponse(resp, path) => {
                    app.handle_references_response(resp, path);
                }
                events::klein_event::KleinEvent::FormatResponse(resp, path) => {
                    app.handle_format_response(resp, path);
                }
                events::klein_event::KleinEvent::RenameResponse(resp, path, new_name) => {
                    app.handle_rename_response(resp, path, new_name);
                }
                events::klein_event::KleinEvent::CodeActionResponse(resp, path, pos) => {
                    app.handle_code_action_response(resp, path, pos);
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
