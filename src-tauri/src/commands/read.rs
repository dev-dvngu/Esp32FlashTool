use crate::sidecar;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::{AppHandle, Emitter, Manager};
use super::flash::{CompletedEvent, FlashState, LogLineEvent, ProgressEvent, StatusChangeEvent};

// Hàm tiện ích emit
fn emit<T: Serialize + Clone>(app: &AppHandle, event: &str, payload: &T) {
    let _ = app.emit(event, payload.clone());
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadFlashPayload {
    pub port: String,
    pub baud: u32,
    pub offset: String,
    pub size: String,
    pub output_file: String,
}

pub fn read_flash(app: AppHandle, payload: ReadFlashPayload) -> Result<(), String> {
    {
        let state = app.state::<FlashState>();
        let guard = state
            .child
            .lock()
            .map_err(|_| "Lock flash state failed".to_string())?;
        if guard.is_some() {
            return Err("Đang có tiến trình chạy, hãy Cancel trước".to_string());
        }
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
        if let Err(e) = read_worker(app.clone(), payload, esptool_path) {
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

fn read_worker(
    app: AppHandle,
    payload: ReadFlashPayload,
    esptool_path: std::path::PathBuf,
) -> Result<(), String> {
    emit(
        &app,
        "status_change",
        &StatusChangeEvent {
            status: "reading".into(),
        },
    );

    let read_args: Vec<String> = vec![
        "--port".into(),
        payload.port,
        "--baud".into(),
        payload.baud.to_string(),
        "read_flash".into(),
        payload.offset,
        payload.size,
        payload.output_file,
    ];

    emit(
        &app,
        "log_line",
        &LogLineEvent {
            text: format!("$ {} {}", esptool_path.display(), read_args.join(" ")),
            level: "info".into(),
        },
    );

    let mut read_cmd = Command::new(&esptool_path);
    read_cmd
        .args(&read_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    read_cmd.creation_flags(0x08000000);

    let mut child = read_cmd
        .spawn()
        .map_err(|e| format!("Read spawn error: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    {
        let state = app.state::<FlashState>();
        let mut guard = state
            .child
            .lock()
            .map_err(|_| "Lock flash state failed".to_string())?;
        *guard = Some(child);
    }

    let app_out = app.clone();
    let re = Regex::new(r"\((\d+)\s*%\)").unwrap();
    
    let t_out = if let Some(mut out) = stdout {
        Some(std::thread::spawn(move || {
            let mut buffer = [0; 1024];
            let mut line_buffer = Vec::new();
            
            while let Ok(n) = out.read(&mut buffer) {
                if n == 0 { break; }
                
                for &byte in &buffer[..n] {
                    // Nếu gặp ký tự kết thúc dòng hoặc ký tự điều khiển (Backspace, Carriage Return)
                    if byte == b'\n' || byte == b'\r' || byte == 0x08 {
                        if !line_buffer.is_empty() {
                            let line = String::from_utf8_lossy(&line_buffer).to_string();
                            
                            // Chỉ gửi log lên UI nếu là dòng mới (newline) hoặc chứa thông tin quan trọng
                            if byte == b'\n' {
                                emit(
                                    &app_out,
                                    "log_line",
                                    &LogLineEvent {
                                        text: line.clone(),
                                        level: "info".into(),
                                    },
                                );
                            }
                            
                            // Trích xuất phần trăm tiến độ từ bất kỳ chuỗi nào (kể cả giữa dòng)
                            if let Some(caps) = re.captures(&line) {
                                if let Ok(p) = caps.get(1).unwrap().as_str().parse::<u8>() {
                                    emit(&app_out, "progress", &ProgressEvent { percent: p });
                                }
                            }
                            
                            line_buffer.clear();
                        }
                    } else {
                        line_buffer.push(byte);
                    }
                }
            }
        }))
    } else {
        None
    };

    let app_err = app.clone();
    let t_err = if let Some(err) = stderr {
        Some(std::thread::spawn(move || {
            for line in BufReader::new(err).lines().flatten() {
                emit(
                    &app_err,
                    "log_line",
                    &LogLineEvent {
                        text: line,
                        level: "error".into(),
                    },
                );
            }
        }))
    } else {
        None
    };

    let result = {
        let state = app.state::<FlashState>();
        let mut guard = state.child.lock().map_err(|_| "Lock failed".to_string())?;
        match guard.take() {
            Some(mut c) => {
                let s = c.wait().ok();
                s
            },
            None => None,
        }
    };

    if let Some(t) = t_out {
        let _ = t.join();
    }
    if let Some(t) = t_err {
        let _ = t.join();
    }

    match result {
        Some(s) if s.success() => {
            emit(
                &app,
                "status_change",
                &StatusChangeEvent {
                    status: "done".into(),
                },
            );
            emit(&app, "completed", &CompletedEvent { success: true });
        }
        _ => {
            emit(
                &app,
                "status_change",
                &StatusChangeEvent {
                    status: "error".into(),
                },
            );
            emit(&app, "completed", &CompletedEvent { success: false });
        }
    }

    Ok(())
}
