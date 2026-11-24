//! ESP32 partition tables, in both CSV and binary (`partition-table.bin`) form.

pub const DEFAULT_TABLE_OFFSET: u32 = 0x8000;
const ENTRY_LEN: usize = 32;
const MAGIC: [u8; 2] = [0xAA, 0x50];
const MD5_MAGIC: [u8; 2] = [0xEB, 0xEB];

pub const TYPE_APP: u8 = 0x00;
pub const TYPE_DATA: u8 = 0x01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub name: String,
    pub ptype: u8,
    pub subtype: u8,
    pub offset: u32,
    pub size: u32,
    pub flags: u32,
}

/// What the user should be offered for a partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Application image (`factory`, `ota_N`, `test`).
    App,
    /// Filesystem image (spiffs / littlefs / fat).
    Filesystem,
    /// Pre-generated NVS image (`nvs_partition_gen`).
    Nvs,
    /// Managed by the bootloader/firmware; never picked by hand (otadata, phy, coredump...).
    None,
}

impl Partition {
    pub fn end(&self) -> u32 {
        self.offset.saturating_add(self.size)
    }

    pub fn slot(&self) -> Slot {
        match (self.ptype, self.subtype) {
            (TYPE_APP, _) => Slot::App,
            (TYPE_DATA, 0x81..=0x83) => Slot::Filesystem,
            (TYPE_DATA, 0x02) => Slot::Nvs,
            _ => Slot::None,
        }
    }

    pub fn type_name(&self) -> String {
        match self.ptype {
            TYPE_APP => "app".into(),
            TYPE_DATA => "data".into(),
            t => format!("{t:#04x}"),
        }
    }

    pub fn subtype_name(&self) -> String {
        let known = match (self.ptype, self.subtype) {
            (TYPE_APP, 0x00) => "factory",
            (TYPE_APP, 0x20) => "test",
            (TYPE_APP, s @ 0x10..=0x1F) => return format!("ota_{}", s - 0x10),
            (TYPE_DATA, s) => match DATA_SUBTYPES.iter().find(|(_, v)| *v == s) {
                Some((n, _)) => n,
                None => return format!("{s:#04x}"),
            },
            (_, s) => return format!("{s:#04x}"),
        };
        known.into()
    }
}

const DATA_SUBTYPES: &[(&str, u8)] = &[
    ("ota", 0x00),
    ("phy", 0x01),
    ("nvs", 0x02),
    ("coredump", 0x03),
    ("nvs_keys", 0x04),
    ("efuse", 0x05),
    ("undefined", 0x06),
    ("esphttpd", 0x80),
    ("fat", 0x81),
    ("spiffs", 0x82),
    ("littlefs", 0x83),
];

/// Parse either form; binary is recognised by its leading magic.
pub fn parse(bytes: &[u8]) -> Result<Vec<Partition>, String> {
    if bytes.starts_with(&MAGIC) {
        parse_bin(bytes)
    } else {
        let text = std::str::from_utf8(bytes).map_err(|_| "not a partition table".to_string())?;
        parse_csv(text, DEFAULT_TABLE_OFFSET)
    }
}

pub fn is_binary_table(bytes: &[u8]) -> bool {
    bytes.starts_with(&MAGIC)
}

pub fn parse_bin(bytes: &[u8]) -> Result<Vec<Partition>, String> {
    let mut out = Vec::new();
    for e in bytes.chunks_exact(ENTRY_LEN) {
        if e[..2] == MD5_MAGIC || e[..2] == [0xFF, 0xFF] {
            break;
        }
        if e[..2] != MAGIC {
            return Err(format!("bad partition entry magic at index {}", out.len()));
        }
        let u32_at = |i: usize| u32::from_le_bytes([e[i], e[i + 1], e[i + 2], e[i + 3]]);
        let name_raw = &e[12..28];
        let name_len = name_raw.iter().position(|&b| b == 0).unwrap_or(16);
        out.push(Partition {
            name: String::from_utf8_lossy(&name_raw[..name_len]).into_owned(),
            ptype: e[2],
            subtype: e[3],
            offset: u32_at(4),
            size: u32_at(8),
            flags: u32_at(28),
        });
    }
    if out.is_empty() {
        return Err("empty partition table".into());
    }
    Ok(out)
}

/// Parse `gen_esp32part.py`-style CSV. Blank offsets are laid out the same way
/// the IDF tool does: after the table sector, apps aligned to 64 KiB, data to 4 KiB.
pub fn parse_csv(text: &str, table_offset: u32) -> Result<Vec<Partition>, String> {
    let mut out = Vec::new();
    let mut cursor = table_offset + 0x1000;
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let err = |m: &str| format!("partitions.csv line {}: {m}", lineno + 1);
        let cols: Vec<&str> = line.split(',').map(str::trim).collect();
        if cols.len() < 5 {
            return Err(err("expected name,type,subtype,offset,size"));
        }
        let ptype = match cols[1] {
            "app" => TYPE_APP,
            "data" => TYPE_DATA,
            t => parse_num(t).map(|v| v as u8).ok_or_else(|| err("bad type"))?,
        };
        let subtype = parse_subtype(ptype, cols[2]).ok_or_else(|| err("bad subtype"))?;
        let align = if ptype == TYPE_APP { 0x10000 } else { 0x1000 };
        let offset = if cols[3].is_empty() {
            cursor.div_ceil(align) * align
        } else {
            parse_num(cols[3]).ok_or_else(|| err("bad offset"))?
        };
        let size = parse_num(cols[4]).ok_or_else(|| err("bad size"))?;
        let flags = cols.get(5).map_or(0, |f| {
            f.split(':').fold(0, |acc, flag| match flag.trim() {
                "encrypted" => acc | 1,
                "readonly" => acc | 2,
                _ => acc,
            })
        });
        cursor = offset + size;
        out.push(Partition { name: cols[0].into(), ptype, subtype, offset, size, flags });
    }
    if out.is_empty() {
        return Err("empty partition table".into());
    }
    Ok(out)
}

fn parse_subtype(ptype: u8, s: &str) -> Option<u8> {
    if let Some(n) = parse_num(s) {
        return Some(n as u8);
    }
    match ptype {
        TYPE_APP => match s {
            "factory" => Some(0x00),
            "test" => Some(0x20),
            _ => s.strip_prefix("ota_")?.parse::<u8>().ok().filter(|n| *n < 16).map(|n| 0x10 + n),
        },
        TYPE_DATA => DATA_SUBTYPES.iter().find(|(n, _)| *n == s).map(|(_, v)| *v),
        _ => None,
    }
}

/// `0x1000`, `4096`, `64K`, `1M`.
pub fn parse_num(s: &str) -> Option<u32> {
    let s = s.trim();
    let (digits, mul) = match s.as_bytes().last()? {
        b'K' | b'k' => (&s[..s.len() - 1], 1024),
        b'M' | b'm' => (&s[..s.len() - 1], 1024 * 1024),
        _ => (s, 1),
    };
    let v = match digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };
    v.checked_mul(mul)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_explicit_and_auto_offsets() {
        let csv = "# Name, Type, SubType, Offset, Size\n\
                   nvs, data, nvs, , 0x6000,\n\
                   phy_init, data, phy, , 4K,\n\
                   factory, app, factory, , 1M,\n\
                   storage, data, littlefs, 0x200000, 64K, readonly\n";
        let p = parse_csv(csv, DEFAULT_TABLE_OFFSET).unwrap();
        assert_eq!(p[0].offset, 0x9000);
        assert_eq!(p[1].offset, 0xF000);
        assert_eq!(p[2].offset, 0x10000); // 64K aligned
        assert_eq!(p[2].size, 0x100000);
        assert_eq!(p[3].offset, 0x200000);
        assert_eq!(p[3].flags, 2);
        assert_eq!(p[3].slot(), Slot::Filesystem);
        assert_eq!(p[2].slot(), Slot::App);
        assert_eq!(p[0].slot(), Slot::Nvs);
        assert_eq!(p[1].slot(), Slot::None);
    }

    #[test]
    fn names_roundtrip() {
        let p = parse_csv("a, app, ota_3, 0x10000, 1M", 0x8000).unwrap();
        assert_eq!(p[0].subtype, 0x13);
        assert_eq!(p[0].subtype_name(), "ota_3");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_csv("x, app, bogus, 0, 1", 0x8000).is_err());
        assert!(parse_bin(&[0u8; 32]).is_err());
        assert_eq!(parse_num("0x10"), Some(16));
        assert_eq!(parse_num("2M"), Some(0x200000));
        assert_eq!(parse_num("zz"), None);
    }
}
