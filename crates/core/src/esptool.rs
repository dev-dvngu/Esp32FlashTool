//! Drive the esptool v5 binary as a child process and stream its output as events.

use crate::detect::FlashSettings;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Connecting,
    Erasing,
    Writing,
    Reading,
    Done,
    Error,
}

#[derive(Debug, Clone)]
pub enum Event {
    Log { text: String, error: bool },
    Progress(u8),
    Status(Status),
    Finished { success: bool },
}

#[derive(Debug, Clone)]
pub struct Conn {
    pub port: String,
    pub baud: u32,
    /// `None` lets esptool auto-detect.
    pub chip: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Op {
    Write {
        settings: FlashSettings,
        erase_before: bool,
        files: Vec<(u32, PathBuf)>,
        extra_args: Vec<String>,
    },
    Erase,
    ReadFlash { offset: u32, size: u32, out: PathBuf },
    /// `flash-id`: chip type, features, MAC, flash size.
    ChipInfo,
}

/// The esptool invocations for an operation, in order. Pure, so it is testable.
pub fn commands(conn: &Conn, op: &Op) -> Vec<(Status, Vec<String>)> {
    let mut base = vec!["--port".into(), conn.port.clone(), "--baud".into(), conn.baud.to_string()];
    if let Some(chip) = &conn.chip {
        base.extend(["--chip".into(), chip.clone()]);
    }
    let with = |extra: &[String]| [base.clone(), extra.to_vec()].concat();
    match op {
        Op::Erase => vec![(Status::Erasing, with(&["erase-flash".into()]))],
        Op::ChipInfo => vec![(Status::Connecting, with(&["flash-id".into()]))],
        Op::ReadFlash { offset, size, out } => vec![(
            Status::Reading,
            with(&[
                "read-flash".into(),
                format!("{offset:#x}"),
                format!("{size:#x}"),
                out.display().to_string(),
            ]),
        )],
        Op::Write { settings, erase_before, files, extra_args } => {
            let mut cmds = Vec::new();
            if *erase_before {
                cmds.push((Status::Erasing, with(&["erase-flash".into()])));
            }
            let mut w = vec!["write-flash".to_string()];
            for (flag, val) in [
                ("--flash-mode", &settings.mode),
                ("--flash-freq", &settings.freq),
                ("--flash-size", &settings.size),
            ] {
                if let Some(v) = val.as_deref().filter(|v| !v.eq_ignore_ascii_case("keep")) {
                    w.extend([flag.into(), v.into()]);
                }
            }
            for (off, path) in files {
                w.extend([format!("{off:#x}"), path.display().to_string()]);
            }
            w.extend(extra_args.iter().cloned());
            cmds.push((Status::Writing, with(&w)));
            cmds
        }
    }
}

/// Handle to a running operation.
#[derive(Clone)]
pub struct Task {
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
}

impl Task {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(c) = self.child.lock().unwrap().as_mut() {
            let _ = c.kill();
        }
    }
}

pub type Sink = Arc<dyn Fn(Event) + Send + Sync>;

/// Run `op` on a background thread; every line, progress tick and status goes to `sink`.
/// A `Finished` event is always the last one sent.
pub fn spawn(esptool: PathBuf, conn: Conn, op: Op, sink: Sink) -> Task {
    let task = Task { child: Arc::default(), cancelled: Arc::default() };
    let t = task.clone();
    std::thread::spawn(move || {
        let ok = run_all(&esptool, &commands(&conn, &op), &t, &sink);
        let ok = ok && !t.cancelled.load(Ordering::SeqCst);
        if t.cancelled.load(Ordering::SeqCst) {
            sink(Event::Log { text: "Cancelled.".into(), error: true });
        }
        sink(Event::Status(if ok { Status::Done } else { Status::Error }));
        sink(Event::Finished { success: ok });
    });
    task
}

fn run_all(esptool: &Path, cmds: &[(Status, Vec<String>)], task: &Task, sink: &Sink) -> bool {
    for (status, args) in cmds {
        if task.cancelled.load(Ordering::SeqCst) {
            return false;
        }
        sink(Event::Status(*status));
        sink(Event::Log { text: format!("$ {} {}", esptool.display(), args.join(" ")), error: false });
        match run_one(esptool, args, task, sink) {
            Ok(true) => {}
            Ok(false) => return false,
            Err(e) => {
                sink(Event::Log { text: format!("[ERROR] {e}"), error: true });
                return false;
            }
        }
    }
    true
}

fn run_one(esptool: &Path, args: &[String], task: &Task, sink: &Sink) -> Result<bool, String> {
    let mut cmd = Command::new(esptool);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let mut child = cmd.spawn().map_err(|e| format!("cannot start {}: {e}", esptool.display()))?;
    let readers: Vec<_> = [
        child.stdout.take().map(|o| Box::new(o) as Box<dyn Read + Send>),
        child.stderr.take().map(|e| Box::new(e) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(i, r)| {
        let sink = sink.clone();
        r.map(|r| std::thread::spawn(move || pump(r, i == 1, &sink)))
    })
    .collect();
    *task.child.lock().unwrap() = Some(child);

    // Poll rather than block in wait() so cancel() can take the lock and kill.
    let status = loop {
        let polled = task.child.lock().unwrap().as_mut().map(|c| c.try_wait());
        match polled {
            Some(Ok(Some(s))) => break Some(s),
            Some(Ok(None)) => std::thread::sleep(Duration::from_millis(50)),
            _ => break None,
        }
    };
    task.child.lock().unwrap().take();
    for r in readers {
        let _ = r.join();
    }
    Ok(status.is_some_and(|s| s.success()))
}

/// Split on `\n` / `\r` (esptool redraws its progress bar with `\r`): every
/// segment may carry a percentage, only `\n`-terminated ones are logged.
fn pump(mut r: impl Read, is_stderr: bool, sink: &Sink) {
    let mut buf = [0u8; 1024];
    let mut line = Vec::new();
    while let Ok(n) = r.read(&mut buf) {
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            if b != b'\n' && b != b'\r' {
                line.push(b);
                continue;
            }
            if line.is_empty() {
                continue;
            }
            let text = String::from_utf8_lossy(&line).into_owned();
            if let Some(p) = parse_percent(&text) {
                sink(Event::Progress(p));
            }
            if b == b'\n' {
                sink(Event::Log { text, error: is_stderr });
            }
            line.clear();
        }
    }
    if !line.is_empty() {
        sink(Event::Log { text: String::from_utf8_lossy(&line).into_owned(), error: is_stderr });
    }
}

/// Integer part of the last `NN.N %` in a line.
pub fn parse_percent(line: &str) -> Option<u8> {
    let before = line[..line.rfind('%')?].trim_end();
    let num_start = before
        .rfind(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map_or(0, |i| i + 1);
    let int = before[num_start..].split('.').next()?;
    int.parse::<u8>().ok().filter(|p| *p <= 100)
}

// ---------------------------------------------------------------- discovery

const EXE: &str = if cfg!(windows) { ".exe" } else { "" };

/// Look for esptool: `$ESPTOOL_PATH`, next to our executable (bundled sidecar,
/// any `esptool*` name), the repo's `src-tauri/binaries` during development,
/// then `PATH`.
pub fn find_esptool() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("ESPTOOL_PATH").map(PathBuf::from).filter(|p| p.is_file()) {
        return Ok(p);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut dirs: Vec<PathBuf> = exe.ancestors().skip(1).take(4).map(Path::to_path_buf).collect();
    dirs = dirs
        .iter()
        .flat_map(|d| [d.clone(), d.join("binaries"), d.join("src-tauri/binaries")])
        .collect();
    for d in &dirs {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        let mut hits: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let n = p.file_name().unwrap_or_default().to_string_lossy();
                p.is_file() && n.starts_with("esptool") && !n.contains("placeholder") && n.ends_with(EXE)
            })
            .collect();
        hits.sort();
        if let Some(p) = hits.into_iter().next() {
            return Ok(p);
        }
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for d in std::env::split_paths(&path) {
        for name in [format!("esptool{EXE}"), "esptool.py".into()] {
            let p = d.join(name);
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err("esptool not found. Run scripts/download-esptool.sh, put esptool next to the app, \
         or set ESPTOOL_PATH."
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent() {
        assert_eq!(parse_percent("Writing at 0x00010000 [====>   ]  23.4% 65536/270000"), Some(23));
        assert_eq!(parse_percent("100 %"), Some(100));
        assert_eq!(parse_percent("no progress"), None);
        assert_eq!(parse_percent("%"), None);
    }

    #[test]
    fn write_command_shape() {
        let conn = Conn { port: "/dev/ttyUSB0".into(), baud: 460800, chip: Some("esp32".into()) };
        let op = Op::Write {
            settings: FlashSettings { mode: Some("dio".into()), freq: None, size: Some("keep".into()) },
            erase_before: true,
            files: vec![(0x1000, "b.bin".into()), (0x10000, "a.bin".into())],
            extra_args: vec![],
        };
        let cmds = commands(&conn, &op);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].1.last().unwrap(), "erase-flash");
        assert_eq!(
            cmds[1].1.join(" "),
            "--port /dev/ttyUSB0 --baud 460800 --chip esp32 write-flash --flash-mode dio \
             0x1000 b.bin 0x10000 a.bin"
        );
    }
}
