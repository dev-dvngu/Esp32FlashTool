//! Serial monitor: a reader thread on the port, commands in through a channel.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub enum MonEvent {
    Data(Vec<u8>),
    /// The reader stopped; `Some` carries the error (e.g. board unplugged).
    Closed(Option<String>),
}

enum Cmd {
    Write(Vec<u8>),
    Reset,
}

pub struct Monitor {
    stop: Arc<AtomicBool>,
    cmds: Sender<Cmd>,
    thread: Option<JoinHandle<()>>,
}

pub fn open(port: &str, baud: u32, sink: Arc<dyn Fn(MonEvent) + Send + Sync>) -> Result<Monitor, String> {
    let mut p = serialport::new(port, baud)
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|e| format!("{port}: {e}"))?;
    // DTR/RTS released = EN high, IO0 high on the usual auto-reset circuit: run mode.
    let _ = p.write_data_terminal_ready(false);
    let _ = p.write_request_to_send(false);
    let stop = Arc::new(AtomicBool::new(false));
    let (cmds, rx) = mpsc::channel::<Cmd>();
    let flag = stop.clone();
    let thread = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let err = loop {
            if flag.load(Ordering::SeqCst) {
                break None;
            }
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    Cmd::Write(b) => _ = p.write_all(&b),
                    Cmd::Reset => {
                        // RTS pulls EN low; DTR stays released so IO0 is high and it boots the app.
                        let _ = p.write_data_terminal_ready(false);
                        let _ = p.write_request_to_send(true);
                        thread::sleep(Duration::from_millis(100));
                        let _ = p.write_request_to_send(false);
                    }
                }
            }
            match p.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => sink(MonEvent::Data(buf[..n].to_vec())),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => break Some(e.to_string()),
            }
        };
        drop(p); // free the port before announcing it
        sink(MonEvent::Closed(err));
    });
    Ok(Monitor { stop, cmds, thread: Some(thread) })
}

impl Monitor {
    pub fn send(&self, bytes: Vec<u8>) {
        let _ = self.cmds.send(Cmd::Write(bytes));
    }

    pub fn reset(&self) {
        let _ = self.cmds.send(Cmd::Reset);
    }
}

/// Dropping blocks until the port is closed (≤ one 50 ms read), so esptool can open it next.
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Other,
}

/// Log level of a monitor line (ANSI already stripped). ESP-IDF: `E (123) tag: ..`;
/// Arduino-ESP32: `[   123][E][file.cpp:1] ..`.
pub fn level(line: &str) -> Level {
    let l = line.trim_start();
    let idf = |c: &str| l.starts_with(&format!("{c} ("));
    let ard = |c: &str| l.starts_with('[') && l.contains(&format!("][{c}]["));
    if idf("E") || ard("E") || l.starts_with("Guru Meditation") || l.starts_with("abort()") {
        Level::Error
    } else if idf("W") || ard("W") {
        Level::Warn
    } else if idf("I") || ard("I") {
        Level::Info
    } else {
        Level::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(level("E (312) wifi: oops"), Level::Error);
        assert_eq!(level("W (1) x: y"), Level::Warn);
        assert_eq!(level("I (27) boot: ESP-IDF v5.5.2"), Level::Info);
        assert_eq!(level("[   123][E][WiFi.cpp:10] fail"), Level::Error);
        assert_eq!(level("Guru Meditation Error: Core 0 panic'ed"), Level::Error);
        assert_eq!(level("Hello world"), Level::Other);
        assert_eq!(level("Ever (x)"), Level::Other);
    }
}

#[cfg(all(test, unix))]
mod pty_test {
    use super::*;
    use serialport::SerialPort;
    use std::sync::Mutex;

    /// Round trip through a pseudo-terminal pair: device output arrives, our sends reach the device.
    #[test]
    fn reads_and_writes_over_a_pty() {
        let (mut dev, host) = serialport::TTYPort::pair().unwrap();
        let name = host.name().unwrap();
        let got = Arc::new(Mutex::new(Vec::new()));
        let g = got.clone();
        let m = open(&name, 115200, Arc::new(move |e| {
            if let MonEvent::Data(b) = e {
                g.lock().unwrap().extend(b);
            }
        }))
        .unwrap();
        dev.write_all(b"I (1) boot: hi\n").unwrap();
        m.send(b"ping\n".to_vec());
        thread::sleep(Duration::from_millis(300));
        assert_eq!(&*got.lock().unwrap(), b"I (1) boot: hi\n");
        let mut buf = [0u8; 16];
        dev.set_timeout(Duration::from_millis(500)).unwrap();
        let n = dev.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"ping\n");
        drop(m); // must return promptly
    }
}
