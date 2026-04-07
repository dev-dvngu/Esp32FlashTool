use crate::sidecar;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// Loại bỏ #[tauri::command] ở đây vì đã có ở mod.rs
pub async fn esptool_version() -> Result<String, String> {
    let esptool_path = sidecar::resolve_esptool_path()?;
    let output = Command::new(esptool_path)
        .arg("version")
        .output()
        .map_err(|e| format!("Failed to get version: {e}"))?;
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn get_chip_info(port: String, baud: u32) -> Result<String, String> {
    let esptool_path = sidecar::resolve_esptool_path()?;
    let mut cmd = Command::new(esptool_path);
    
    cmd.args([
        "--port", &port,
        "--baud", &baud.to_string(),
        "chip-id"
    ]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd.output().map_err(|e| format!("Failed to get chip info: {e}"))?;
    
    if !output.status.success() {
        let mut cmd2 = Command::new(sidecar::resolve_esptool_path()?);
        cmd2.args(["--port", &port, "--baud", &baud.to_string(), "flash-id"]);
        #[cfg(target_os = "windows")]
        cmd2.creation_flags(0x08000000);
        let output2 = cmd2.output().map_err(|e| format!("Retry flash-id failed: {e}"))?;
        return Ok(String::from_utf8_lossy(&output2.stdout).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
