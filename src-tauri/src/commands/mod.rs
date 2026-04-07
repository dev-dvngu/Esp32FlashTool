mod greet;
mod esptool;
mod serial;
pub mod flash;
pub mod memory;
pub mod read;

pub use flash::FlashState;

#[tauri::command]
pub fn read_local_file(path: String, offset: u64, size: u64) -> Result<Vec<u8>, String> {
    memory::read_local_file(path, offset, size)
}

#[tauri::command]
pub fn write_local_file(path: String, data: Vec<u8>) -> Result<(), String> {
    memory::write_local_file(path, data)
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    greet::greet(name)
}

#[tauri::command]
pub fn esptool_version() -> Result<String, String> {
    esptool::esptool_version()
}

#[tauri::command]
pub fn get_chip_info(port: String, baud: u32) -> Result<String, String> {
    esptool::get_chip_info(port, baud)
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

#[tauri::command]
pub fn erase_flash(
    app: tauri::AppHandle,
    payload: flash::EraseStartPayload,
) -> Result<(), String> {
    flash::erase_flash(app, payload)
}

#[tauri::command]
pub fn read_flash(
    app: tauri::AppHandle,
    payload: read::ReadFlashPayload,
) -> Result<(), String> {
    read::read_flash(app, payload)
}
