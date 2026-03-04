use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::config;

pub struct Terminal {
    pub _pty_pair: PtyPair,
    pub writer: Box<dyn Write + Send>,
    pub output: Arc<Mutex<String>>,
}

impl Terminal {
    pub fn new(cwd: std::path::PathBuf) -> Self {
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        let mut cmd = CommandBuilder::new(config::TERMINAL_BASH_PATH); 
        cmd.args(&["--login", "-i"]);
        cmd.env("TERM", "xterm-256color");
        cmd.cwd(cwd);
        let _child = pty_pair.slave.spawn_command(cmd).unwrap();

        let writer = pty_pair.master.take_writer().unwrap();
        let mut reader = pty_pair.master.try_clone_reader().unwrap();
        let output = Arc::new(Mutex::new(String::new()));

        let output_clone = Arc::clone(&output);
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut out = output_clone.lock().unwrap();
                let text = String::from_utf8_lossy(&buf[..n]);
                if text.contains("\x1b[2J") || text.contains("\x1b[H") {
                    out.clear();
                }
                out.push_str(&text);
                // Limit output buffer size
                if out.len() > 10000 {
                    let split_idx = out.len() - 5000;
                    // Find a safe UTF-8 boundary
                    let safe_idx = out.char_indices()
                        .map(|(i, _)| i)
                        .filter(|&i| i >= split_idx)
                        .next()
                        .unwrap_or(out.len());
                    *out = out[safe_idx..].to_string();
                }
            }
        });

        Terminal {
            _pty_pair: pty_pair,
            writer,
            output,
        }
    }

    pub fn write(&mut self, data: &str) {
        let _ = self.writer.write_all(data.as_bytes());
        let _ = self.writer.flush(); // Crucial for PTY responsiveness
    }
}
