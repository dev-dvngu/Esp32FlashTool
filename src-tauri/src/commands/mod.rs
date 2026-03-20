mod greet;
mod esptool;
mod serial;
pub mod flash;

pub use flash::FlashState;

#[tauri::command]
pub fn greet(name: &str) -> String {
    greet::greet(name)
}

#[tauri::command]
pub fn esptool_version() -> Result<String, String> {
    esptool::esptool_version()
}

#[tauri::command]
pub fn list_serial_ports() -> Result<Vec<serial::SerialPortInfo>, String> {
    serial::list_serial_ports()
}

#[tauri::command]
pub fn flash_start(
    app: tauri::AppHandle,
    payload: flash::FlashStartPayload,
) -> Result<(), String> {
    flash::flash_start(app, payload)
}

#[tauri::command]
pub fn flash_cancel(app: tauri::AppHandle) -> Result<(), String> {
    flash::flash_cancel(app)
}
