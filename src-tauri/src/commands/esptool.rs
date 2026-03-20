use crate::sidecar;
use std::process::Command;

pub fn esptool_version() -> Result<String, String> {
    let path = sidecar::resolve_esptool_path()?;

    let output = Command::new(&path)
        .arg("version")
        .output()
        .map_err(|e| format!("Không chạy được esptool tại {}: {e}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("esptool version lỗi: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
