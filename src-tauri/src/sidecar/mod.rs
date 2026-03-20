use std::path::PathBuf;

const TARGET: &str = env!("TAURI_ENV_TARGET_TRIPLE");

pub fn resolve_esptool_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let dir = exe.parent().ok_or("Không lấy được thư mục exe")?;

    // Dev mode: Tauri build copies sidecar stripped of directory prefix and target triple
    let p = dir.join("esptool");
    if p.exists() {
        return Ok(p);
    }

    // Production: sidecar with target triple suffix next to exe
    let p = dir.join(format!("esptool-{TARGET}"));
    if p.exists() {
        return Ok(p);
    }

    // Production: resource dir might differ — try ../lib/{app}/
    let p = dir.join("../lib/espflashtool").join(format!("esptool-{TARGET}"));
    if p.exists() {
        return Ok(p);
    }

    Err(format!(
        "Không tìm thấy esptool binary trong {}",
        dir.display()
    ))
}
