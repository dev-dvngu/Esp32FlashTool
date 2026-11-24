pub mod detect;
pub mod esptool;
pub mod partition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInfo {
    pub name: String,
    /// USB product string when available, e.g. "CP2102 USB to UART Bridge".
    pub description: String,
}

pub fn list_ports() -> Vec<PortInfo> {
    let mut out: Vec<PortInfo> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| PortInfo {
            description: match p.port_type {
                serialport::SerialPortType::UsbPort(u) => u.product.unwrap_or_default(),
                _ => String::new(),
            },
            name: p.port_name,
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}
