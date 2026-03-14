pub mod app;
pub mod config;
pub mod editor;
pub mod events;
pub mod lsp;
pub mod search;
pub mod sidebar;
pub mod tabs;
pub mod terminal;
pub mod treesitter;
pub mod ui;

/// Initialize file-based logging.
pub fn init_logging() {
    use std::io::Write;

    let log_path = directories::ProjectDirs::from("", "", "Klein").map(|dirs| {
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
