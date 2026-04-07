use crate::sidecar;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn esptool_version() -> Result<String, String> {
    let path = sidecar::resolve_esptool_path()?;

    let mut cmd = Command::new(&path);
    cmd.arg("version");

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd
        .output()
        .map_err(|e| format!("Không chạy được esptool tại {}: {e}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("esptool version lỗi: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_chip_info(port: String, baud: u32) -> Result<String, String> {
    let path = sidecar::resolve_esptool_path()?;

    let mut cmd = Command::new(&path);
    cmd.args(&["--port", &port, "--baud", &baud.to_string(), "chip_id"]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd
        .output()
        .map_err(|e| format!("Lỗi thực thi esptool: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Không thể lấy thông tin chip: {stderr}"));
    }

    // esptool often prints diagnostic/device info to stderr, so combine both streams.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(format!("{stdout}\n{stderr}"))
}
