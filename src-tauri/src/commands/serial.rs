use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SerialPortInfo {
    pub name: String,
    pub port_type: String,
}

pub fn list_serial_ports() -> Result<Vec<SerialPortInfo>, String> {
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;

    let mut out = ports
        .into_iter()
        .map(|p| SerialPortInfo {
            name: p.port_name,
            port_type: format!("{:?}", p.port_type),
        })
        .collect::<Vec<_>>();

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

