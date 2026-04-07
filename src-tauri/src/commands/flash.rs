use crate::sidecar;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::Mutex,
};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::{AppHandle, Emitter, Manager};

pub struct FlashState {
    pub child: Mutex<Option<Child>>,
}

impl Default for FlashState {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LogLineEvent {
    pub text: String,
    pub level: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusChangeEvent {
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletedEvent {
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlashItem {
    pub offset: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlashStartPayload {
    pub port: String,
    pub baud: u32,
    pub flash_mode: Option<String>,
    pub flash_freq: Option<String>,
    pub flash_size: Option<String>,
    pub erase_before: bool,
    pub verify_after: bool,
    pub items: Vec<FlashItem>,
    pub extra_args: Option<String>,
}

fn emit<T: Serialize + Clone>(app: &AppHandle, event: &str, payload: &T) {
    let _ = app.emit(event, payload.clone());
}

pub fn flash_cancel(app: AppHandle) -> Result<(), String> {
    let state = app.state::<FlashState>();
    let mut guard = state
        .child
        .lock()
        .map_err(|_| "Lock flash state failed".to_string())?;
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
    }
    Ok(())
}

pub fn flash_start(app: AppHandle, payload: FlashStartPayload) -> Result<(), String> {
    {
        let state = app.state::<FlashState>();
        let guard = state
            .child
            .lock()
            .map_err(|_| "Lock flash state failed".to_string())?;
        if guard.is_some() {
            return Err("Đang có flash đang chạy, hãy Cancel trước".to_string());
        }
    }

    if payload.items.is_empty() {
        return Err("Cần ít nhất 1 file để flash".to_string());
    }

    let esptool_path = sidecar::resolve_esptool_path()?;

    emit(
        &app,
        "status_change",
        &StatusChangeEvent {
            status: "connecting".into(),
        },
    );

    std::thread::spawn(move || {
        if let Err(e) = flash_worker(app.clone(), payload, esptool_path) {
            emit(
                &app,
                "log_line",
                &LogLineEvent {
                    text: format!("[ERROR] {e}"),
                    level: "error".into(),
                },
            );
            emit(
                &app,
                "status_change",
                &StatusChangeEvent {
                    status: "error".into(),
                },
            );
            emit(&app, "completed", &CompletedEvent { success: false });
        }
    });

    Ok(())
}

fn handle_stream<R: Read>(mut reader: R, app: AppHandle, is_stderr: bool) {
    let re = Regex::new(r"\((\d+)\s*%\)").unwrap();
    let mut buffer = [0; 1024];
    let mut line_buffer = Vec::new();
    
    while let Ok(n) = reader.read(&mut buffer) {
        if n == 0 { break; }
        for &byte in &buffer[..n] {
            if byte == b'\n' || byte == b'\r' || byte == 0x08 {
                if !line_buffer.is_empty() {
                    let line = String::from_utf8_lossy(&line_buffer).to_string();
                    if byte == b'\n' {
                        emit(&app, "log_line", &LogLineEvent {
                            text: line.clone(),
                            level: if is_stderr { "error".into() } else { "info".into() },
                        });
                    }
                    if let Some(caps) = re.captures(&line) {
                        if let Ok(p) = caps.get(1).unwrap().as_str().parse::<u8>() {
                            emit(&app, "progress", &ProgressEvent { percent: p });
                        }
                    }
                    line_buffer.clear();
                }
            } else {
                line_buffer.push(byte);
            }
        }
    }
}

fn flash_worker(
    app: AppHandle,
    payload: FlashStartPayload,
    esptool_path: std::path::PathBuf,
) -> Result<(), String> {
    let base_args: Vec<String> = vec![
        "--port".into(),
        payload.port.clone(),
        "--baud".into(),
        payload.baud.to_string(),
    ];

    if payload.erase_before {
        emit(&app, "status_change", &StatusChangeEvent { status: "erasing".into() });
        let mut erase_args = base_args.clone();
        erase_args.push("erase_flash".into());
        emit(&app, "log_line", &LogLineEvent {
            text: format!("$ {} {}", esptool_path.display(), erase_args.join(" ")),
            level: "info".into(),
        });

        let mut erase_cmd = Command::new(&esptool_path);
        erase_cmd.args(&erase_args).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        erase_cmd.creation_flags(0x08000000);

        let mut erase_child = erase_cmd.spawn().map_err(|e| format!("Erase spawn error: {e}"))?;
        let e_out = erase_child.stdout.take();
        let e_err = erase_child.stderr.take();

        let app_c = app.clone();
        let t1 = e_out.map(|o| std::thread::spawn(move || handle_stream(o, app_c, false)));
        let app_c2 = app.clone();
        let t2 = e_err.map(|e| std::thread::spawn(move || handle_stream(e, app_c2, true)));

        let _ = erase_child.wait();
        if let Some(t) = t1 { let _ = t.join(); }
        if let Some(t) = t2 { let _ = t.join(); }
    }

    emit(&app, "status_change", &StatusChangeEvent { status: "writing".into() });
    let mut write_args = base_args;
    write_args.push("write_flash".into());
    if let Some(m) = payload.flash_mode.as_ref() { write_args.push("--flash_mode".into()); write_args.push(m.clone()); }
    if let Some(f) = payload.flash_freq.as_ref() { write_args.push("--flash_freq".into()); write_args.push(f.clone()); }
    if let Some(s) = payload.flash_size.as_ref() { if s.to_lowercase() != "keep" { write_args.push("--flash_size".into()); write_args.push(s.clone()); } }
    if payload.verify_after { write_args.push("--verify".into()); }
    for it in &payload.items { write_args.push(it.offset.clone()); write_args.push(it.file_path.clone()); }
    if let Some(extra) = payload.extra_args.as_ref() {
        let extra = extra.trim();
        if !extra.is_empty() { write_args.extend(extra.split_whitespace().map(|s| s.to_string())); }
    }

    emit(&app, "log_line", &LogLineEvent {
        text: format!("$ {} {}", esptool_path.display(), write_args.join(" ")),
        level: "info".into(),
    });

    let mut write_cmd = Command::new(&esptool_path);
    write_cmd.args(&write_args).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    write_cmd.creation_flags(0x08000000);

    let mut child = write_cmd.spawn().map_err(|e| format!("Spawn error: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    {
        let state = app.state::<FlashState>();
        let mut guard = state.child.lock().map_err(|_| "Lock flash state failed".to_string())?;
        *guard = Some(child);
    }

    let app_c = app.clone();
    let t_out = stdout.map(|o| std::thread::spawn(move || handle_stream(o, app_c, false)));
    let app_c2 = app.clone();
    let t_err = stderr.map(|e| std::thread::spawn(move || handle_stream(e, app_c2, true)));

    let result = {
        let state = app.state::<FlashState>();
        let mut guard = state.child.lock().map_err(|_| "Lock failed".to_string())?;
        match guard.take() {
            Some(mut c) => c.wait().ok(),
            None => None,
        }
    };

    if let Some(t) = t_out { let _ = t.join(); }
    if let Some(t) = t_err { let _ = t.join(); }

    match result {
        Some(s) if s.success() => {
            emit(&app, "status_change", &StatusChangeEvent { status: "done".into() });
            emit(&app, "completed", &CompletedEvent { success: true });
        }
        _ => {
            emit(&app, "status_change", &StatusChangeEvent { status: "error".into() });
            emit(&app, "completed", &CompletedEvent { success: false });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EraseStartPayload {
    pub port: String,
    pub baud: u32,
}

pub fn erase_flash(app: AppHandle, payload: EraseStartPayload) -> Result<(), String> {
    {
        let state = app.state::<FlashState>();
        let guard = state.child.lock().map_err(|_| "Lock flash state failed".to_string())?;
        if guard.is_some() { return Err("Đang có tiến trình chạy, hãy Cancel trước".to_string()); }
    }
    let esptool_path = sidecar::resolve_esptool_path()?;
    emit(&app, "status_change", &StatusChangeEvent { status: "connecting".into() });
    std::thread::spawn(move || {
        if let Err(e) = erase_worker(app.clone(), payload, esptool_path) {
            emit(&app, "log_line", &LogLineEvent { text: format!("[ERROR] {e}"), level: "error".into() });
            emit(&app, "status_change", &StatusChangeEvent { status: "error".into() });
            emit(&app, "completed", &CompletedEvent { success: false });
        }
    });
    Ok(())
}

fn erase_worker(
    app: AppHandle,
    payload: EraseStartPayload,
    esptool_path: std::path::PathBuf,
) -> Result<(), String> {
    emit(&app, "status_change", &StatusChangeEvent { status: "erasing".into() });
    let erase_args: Vec<String> = vec![
        "--port".into(), payload.port, "--baud".into(), payload.baud.to_string(), "erase_flash".into(),
    ];
    emit(&app, "log_line", &LogLineEvent {
        text: format!("$ {} {}", esptool_path.display(), erase_args.join(" ")),
        level: "info".into(),
    });

    let mut erase_cmd = Command::new(&esptool_path);
    erase_cmd.args(&erase_args).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    erase_cmd.creation_flags(0x08000000);

    let mut child = erase_cmd.spawn().map_err(|e| format!("Erase spawn error: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    {
        let state = app.state::<FlashState>();
        let mut guard = state.child.lock().map_err(|_| "Lock flash state failed".to_string())?;
        *guard = Some(child);
    }

    let app_c = app.clone();
    let t_out = stdout.map(|o| std::thread::spawn(move || handle_stream(o, app_c, false)));
    let app_c2 = app.clone();
    let t_err = stderr.map(|e| std::thread::spawn(move || handle_stream(e, app_c2, true)));

    let result = {
        let state = app.state::<FlashState>();
        let mut guard = state.child.lock().map_err(|_| "Lock failed".to_string())?;
        match guard.take() { Some(mut c) => c.wait().ok(), None => None }
    };

    if let Some(t) = t_out { let _ = t.join(); }
    if let Some(t) = t_err { let _ = t.join(); }

    match result {
        Some(s) if s.success() => {
            emit(&app, "status_change", &StatusChangeEvent { status: "done".into() });
            emit(&app, "completed", &CompletedEvent { success: true });
        }
        _ => {
            emit(&app, "status_change", &StatusChangeEvent { status: "error".into() });
            emit(&app, "completed", &CompletedEvent { success: false });
        }
    }
    Ok(())
}
