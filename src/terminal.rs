use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Terminal {
    pub master_pty: Box<dyn portable_pty::MasterPty + Send>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    pub cwd: std::path::PathBuf,
    pub shell: Option<String>,
}

impl Terminal {
    pub fn new(cwd: std::path::PathBuf, preferred_shell: Option<String>) -> Self {
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        // Check if preferred shell exists and is usable
        let mut explicit_shell: Option<(String, Vec<&str>)> = None;
        if let Some(ref shell) = preferred_shell {
            if shell != "auto" {
                // Determine test arguments based on the shell
                let test_arg = if shell.contains("powershell") {
                    "-Command"
                } else {
                    "--version"
                };
                let test_arg2 = if shell.contains("powershell") {
                    "exit"
                } else {
                    ""
                };

                let mut cmd = std::process::Command::new(&shell);
                cmd.arg(test_arg);
                if !test_arg2.is_empty() {
                    cmd.arg(test_arg2);
                }

                if cmd.output().is_ok() {
                    if shell == "bash" || shell.ends_with("bash.exe") {
                        explicit_shell = Some((shell.clone(), vec!["--login", "-i"]));
                    } else {
                        explicit_shell = Some((shell.clone(), vec![]));
                    }
                }
            }
        }

        // Dynamically resolve the best available shell
        let (shell_path, args) = if let Some(explicit) = explicit_shell {
            (explicit.0, explicit.1)
        } else if std::path::Path::new("C:\\Program Files\\Git\\bin\\bash.exe").exists() {
            (
                "C:\\Program Files\\Git\\bin\\bash.exe".to_string(),
                vec!["--login", "-i"],
            )
        } else if std::path::Path::new("C:\\Program local\\Git\\bin\\bash.exe").exists() {
            (
                "C:\\Program local\\Git\\bin\\bash.exe".to_string(),
                vec!["--login", "-i"],
            )
        } else if std::process::Command::new("bash")
            .arg("--version")
            .output()
            .is_ok()
        {
            ("bash".to_string(), vec!["--login", "-i"])
        } else if std::process::Command::new("powershell")
            .arg("-Command")
            .arg("exit")
            .output()
            .is_ok()
        {
            ("powershell.exe".to_string(), vec![])
        } else {
            // Ultimate fallback
            ("cmd.exe".to_string(), vec![])
        };

        let mut cmd = CommandBuilder::new(shell_path);
        cmd.args(&args);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.cwd(&cwd);
        let child = pty_pair.slave.spawn_command(cmd).unwrap();
        
        // Drop slave proactively to ensure EOF reaches master when child exits
        drop(pty_pair.slave);

        let writer = pty_pair.master.take_writer().unwrap();
        let writer_arc = Arc::new(Mutex::new(writer));
        let mut reader = pty_pair.master.try_clone_reader().unwrap();
        
        let parser = Arc::new(Mutex::new(vt100::Parser::new(24, 80, 10000)));
        let parser_clone = Arc::clone(&parser);
        let writer_clone = Arc::clone(&writer_arc);

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                
                let text = String::from_utf8_lossy(&buf[..n]);
                // DA Query Response for shells like Fish
                if text.contains("\x1b[c") || text.contains("\x1b[0c") {
                    if let Ok(mut w) = writer_clone.lock() {
                        let _ = w.write_all(b"\x1b[?62;22c");
                        let _ = w.flush();
                    }
                }
                
                let mut p = parser_clone.lock().unwrap();
                
                // Many shells on Windows/Portable-PTY fail to emit \r with \n in raw mode.
                // We inject \r before \n if missing to prevent staircasing in the VT100 grid.
                let mut processed = Vec::with_capacity(n * 2);
                for &b in &buf[..n] {
                    if b == b'\n' {
                        // Idempotent even if \r was already sent, as VT100 \r just moves to col 0.
                        processed.push(b'\r');
                    }
                    processed.push(b);
                }
                
                p.process(&processed);
            }
        });

        Terminal {
            master_pty: pty_pair.master,
            writer: writer_arc,
            parser,
            child: Arc::new(Mutex::new(child)),
            cwd: cwd.clone(),
            shell: preferred_shell.clone(),
        }
    }

    pub fn restart(&mut self) {
        *self = Terminal::new(self.cwd.clone(), self.shell.clone());
    }

    pub fn write(&mut self, data: &str) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(data.as_bytes());
            let _ = w.flush(); // Crucial for PTY responsiveness
        }
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.master_pty.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        if let Ok(mut p) = self.parser.lock() {
            p.set_size(rows, cols);
        }
    }
}
