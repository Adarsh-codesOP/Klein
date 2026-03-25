use portable_pty::{native_pty_system, CommandBuilder, Child, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Terminal {
    // Keep master alive so the PTY isn't torn down while we still hold the writer/reader.
    _master: Box<dyn MasterPty + Send>,
    // Child is stored so we can poll try_wait() for reliable exit detection.
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    // Writer is Arc<Mutex<...>> so the reader thread can share it to respond to DA queries.
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub output: Arc<Mutex<String>>,
    cwd: std::path::PathBuf,
    shell: Option<String>,
}

impl Terminal {
    pub fn new(cwd: std::path::PathBuf, preferred_shell: Option<String>) -> Self {
        let (master, child, writer, output) = Self::spawn_shell(&cwd, &preferred_shell);
        Terminal { _master: master, child, writer, output, cwd, shell: preferred_shell }
    }

    fn spawn_shell(
        cwd: &std::path::PathBuf,
        preferred_shell: &Option<String>,
    ) -> (
        Box<dyn MasterPty + Send>,
        Arc<Mutex<Box<dyn Child + Send + Sync>>>,
        Arc<Mutex<Box<dyn Write + Send>>>,
        Arc<Mutex<String>>,
    ) {
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .unwrap();

        // Check if preferred shell exists and is usable
        let mut explicit_shell: Option<(String, Vec<&'static str>)> = None;
        if let Some(shell) = preferred_shell {
            if shell != "auto" {
                let test_arg = if shell.contains("powershell") { "-Command" } else { "--version" };
                let test_arg2 = if shell.contains("powershell") { "exit" } else { "" };
                let mut cmd = std::process::Command::new(shell);
                cmd.arg(test_arg);
                if !test_arg2.is_empty() { cmd.arg(test_arg2); }
                if cmd.output().is_ok() {
                    if shell == "bash" || shell.ends_with("bash.exe") {
                        explicit_shell = Some((shell.clone(), vec!["--login", "-i"]));
                    } else {
                        explicit_shell = Some((shell.clone(), vec![]));
                    }
                }
            }
        }

        let (shell_path, args) = if let Some(e) = explicit_shell {
            (e.0, e.1)
        } else if std::path::Path::new("C:\\Program Files\\Git\\bin\\bash.exe").exists() {
            ("C:\\Program Files\\Git\\bin\\bash.exe".to_string(), vec!["--login", "-i"])
        } else if std::path::Path::new("C:\\Program local\\Git\\bin\\bash.exe").exists() {
            ("C:\\Program local\\Git\\bin\\bash.exe".to_string(), vec!["--login", "-i"])
        } else if std::process::Command::new("bash").arg("--version").output().is_ok() {
            ("bash".to_string(), vec!["--login", "-i"])
        } else if std::process::Command::new("powershell").arg("-Command").arg("exit").output().is_ok() {
            ("powershell.exe".to_string(), vec![])
        } else {
            ("cmd.exe".to_string(), vec![])
        };

        let mut cmd = CommandBuilder::new(shell_path);
        cmd.args(&args);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.cwd(cwd);

        // Destructure so we can drop the slave after spawning.
        // Dropping the slave FD in our process is required: the kernel only signals
        // EOF/EIO to the master reader once ALL slave file descriptors are closed.
        // With the slave still open in our process the reader would block forever.
        let master = pty_pair.master;
        let slave  = pty_pair.slave;
        let child  = slave.spawn_command(cmd).unwrap();
        drop(slave); // closes our copy — child retains its own copy

        // Wrap writer in Arc<Mutex> so the reader thread can share it to respond
        // to terminal capability queries (e.g. fish's Primary Device Attribute ESC[c).
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(master.take_writer().unwrap()));
        let writer_for_thread = Arc::clone(&writer);

        let mut reader = master.try_clone_reader().unwrap();
        let output = Arc::new(Mutex::new(String::new()));
        let output_clone = Arc::clone(&output);

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]);

                        // Respond to Primary Device Attribute query (ESC[c or ESC[0c).
                        // Shells like fish send this to probe terminal capabilities and
                        // wait up to 2 seconds for a response. We reply as a VT220.
                        if text.contains("\x1b[c") || text.contains("\x1b[0c") {
                            if let Ok(mut w) = writer_for_thread.lock() {
                                let _ = w.write_all(b"\x1b[?62;22c");
                                let _ = w.flush();
                            }
                        }

                        let mut out = output_clone.lock().unwrap();
                        // Do NOT clear `out` here on ESC[2J — that would race with
                        // the render thread and wipe single-line command output before
                        // it is ever displayed.  ESC[2J is handled in-band inside
                        // process_output() where ordering is deterministic.
                        out.push_str(&text);
                        if out.len() > 10000 {
                            let split_idx = out.len() - 5000;
                            let safe_idx = out.char_indices()
                                .map(|(i, _)| i)
                                .filter(|&i| i >= split_idx)
                                .next()
                                .unwrap_or(out.len());
                            *out = out[safe_idx..].to_string();
                        }
                    }
                }
            }
        });

        (master, Arc::new(Mutex::new(child)), writer, output)
    }

    /// Returns true once the shell process has exited.
    pub fn is_exited(&self) -> bool {
        self.child.lock()
            .map(|mut c| matches!(c.try_wait(), Ok(Some(_))))
            .unwrap_or(false)
    }

    /// Spawn a fresh shell session, replacing all internal state.
    pub fn restart(&mut self) {
        let (master, child, writer, output) = Self::spawn_shell(&self.cwd, &self.shell);
        self._master = master;
        self.child   = child;
        self.writer  = writer;
        self.output  = output;
    }

    pub fn write(&mut self, data: &str) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(data.as_bytes());
            let _ = w.flush();
        }
    }

    /// Send a clear-screen command to the shell.  Called once after spawn so
    /// the terminal panel starts with a clean slate instead of showing shell
    /// startup noise (fish greeting, DA-query responses, etc.).
    pub fn send_clear(&mut self) {
        self.write("clear\n");
    }
}
