use std::path::PathBuf;

const TARGET: &str = env!("TAURI_ENV_TARGET_TRIPLE");

const EXE_SUFFIX: &str = if cfg!(windows) { ".exe" } else { "" };

pub fn resolve_esptool_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let dir = exe.parent().ok_or("Không lấy được thư mục exe")?;

    let candidates = [
        dir.join(format!("esptool{EXE_SUFFIX}")),
        dir.join(format!("esptool-{TARGET}{EXE_SUFFIX}")),
        dir.join("../lib/espflashtool")
            .join(format!("esptool-{TARGET}{EXE_SUFFIX}")),
    ];

    for p in &candidates {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    Err(format!(
        "Không tìm thấy esptool binary trong {} (tried: {})",
        dir.display(),
        candidates
            .iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
