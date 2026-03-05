use portable_pty::{native_pty_system, CommandBuilder, PtySize, MasterPty};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;
use vt100::Parser;

use crate::config;

pub struct MicroEditor {
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub parser: Arc<Mutex<Parser>>,
    pub width: u16,
    pub height: u16,
}

impl MicroEditor {
    pub fn new(width: u16, height: u16) -> Self {
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: height,
                cols: width,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        let mut cmd = CommandBuilder::new(config::MICRO_PATH);
        cmd.env("TERM", "xterm-256color");
        let _child = pty_pair.slave.spawn_command(cmd).unwrap();

        let writer = pty_pair.master.take_writer().unwrap();
        let mut reader = pty_pair.master.try_clone_reader().unwrap();
        let parser = Arc::new(Mutex::new(Parser::new(height, width, 0)));

        let parser_clone = Arc::clone(&parser);
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut p = parser_clone.lock().unwrap();
                p.process(&buf[..n]);
            }
        });

        MicroEditor {
            master: pty_pair.master,
            writer,
            parser,
            width,
            height,
        }
    }

    pub fn write(&mut self, data: &str) {
        let _ = self.writer.write_all(data.as_bytes());
        let _ = self.writer.flush();
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width == width && self.height == height {
            return;
        }
        let _ = self.master.resize(PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        });
        let mut p = self.parser.lock().unwrap();
        p.set_size(height, width);
        self.width = width;
        self.height = height;
    }
    
    pub fn open_file(&mut self, path: PathBuf) {
        // Send Ctrl-E to open command bar, then type 'open <path>'
        // \x05 is Ctrl-E in most terminals/micro
        let cmd = format!("\x05open {}\r", path.to_string_lossy());
        self.write(&cmd);
    }
}
