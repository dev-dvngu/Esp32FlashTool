mod commands;
mod sidecar;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::FlashState::default())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::esptool_version,
            commands::get_chip_info,
            commands::list_serial_ports,
            commands::flash_start,
            commands::flash_cancel,
            commands::erase_flash,
            commands::read_flash,
            commands::read_local_file,
            commands::write_local_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
