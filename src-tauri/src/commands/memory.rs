use std::fs::File;
use std::io::{Read, Write};

pub fn read_local_file(path: String, offset: u64, size: u64) -> Result<Vec<u8>, String> {
    let mut file = File::open(&path).map_err(|e| format!("Cannot open file: {e}"))?;
    
    // Đọc toàn bộ file hoặc đọc theo chunk nếu cần, ở đây đơn giản đọc toàn bộ giới hạn bởi size
    let file_size = file.metadata().map_err(|e| e.to_string())?.len();
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;

    if offset >= file_size {
        return Ok(Vec::new());
    }

    let start = offset as usize;
    let end = std::cmp::min((offset + size) as usize, file_size as usize);

    Ok(buffer[start..end].to_vec())
}

pub fn write_local_file(path: String, data: Vec<u8>) -> Result<(), String> {
    let mut file = File::create(&path).map_err(|e| format!("Cannot create file: {e}"))?;
    file.write_all(&data).map_err(|e| format!("Cannot write file: {e}"))?;
    Ok(())
}
