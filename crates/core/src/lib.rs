pub mod detect;
pub mod esptool;
pub mod monitor;
pub mod partition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInfo {
    pub name: String,
    /// USB product string when available, e.g. "CP2102 USB to UART Bridge".
    pub description: String,
}

/// Serial ports, USB first. Linux's legacy on-board UARTs (`/dev/ttyS*`, usually
/// 32 phantom entries) are hidden unless they are USB-backed: dev boards never are.
pub fn list_ports() -> Vec<PortInfo> {
    let mut out: Vec<(bool, PortInfo)> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            let (usb, description) = match p.port_type {
                serialport::SerialPortType::UsbPort(u) => (true, u.product.unwrap_or_default()),
                _ => (false, String::new()),
            };
            if !usb && p.port_name.starts_with("/dev/ttyS") {
                return None;
            }
            Some((usb, PortInfo { name: p.port_name, description }))
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| natural(&a.1.name).cmp(&natural(&b.1.name))));
    out.into_iter().map(|(_, p)| p).collect()
}

/// Sort key so `ttyUSB2` < `ttyUSB10` and `COM3` < `COM10`.
fn natural(name: &str) -> (String, u32) {
    let split = name.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    (name[..split].to_owned(), name[split..].parse().unwrap_or(0))
}
