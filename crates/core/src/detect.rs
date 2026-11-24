//! Turn whatever the user dropped (a project dir, build dir, manifest, partition
//! table or lone .bin) into a flash plan.
//!
//! Priority, highest first:
//!   1. ESP-IDF `flasher_args.json`  — offsets, flash settings, chip
//!   2. `flash_args`                 — Arduino-ESP32 (and IDF) offset list
//!   3. partition table (csv / bin)  — offsets from the table, user picks images
//!   4. lone `.bin`                  — 0x0 (ESP8266 Arduino, merged images)

use crate::partition::{self, Partition, Slot};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    IdfFlasherArgs,
    FlashArgs,
    PartitionTable,
    SingleBin,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlashSettings {
    pub mode: Option<String>,
    pub freq: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashFile {
    pub offset: u32,
    /// `None` = a slot the user still has to fill.
    pub path: Option<PathBuf>,
    pub label: String,
    /// Size of the partition this file lands in, for overflow checks.
    pub max_size: Option<u32>,
    /// Pre-ticked in the UI. Optional slots (filesystem, nvs) start unticked.
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub source: Source,
    /// The file or directory the plan was read from (shown in the UI).
    pub origin: PathBuf,
    /// esptool `--chip` name, e.g. `esp32`, `esp32s3`, `esp8266`.
    pub chip: Option<String>,
    pub settings: FlashSettings,
    pub files: Vec<FlashFile>,
    pub partitions: Option<Vec<Partition>>,
    /// Arduino `*.merged.bin`: a single-file alternative written at 0x0.
    pub merged: Option<PathBuf>,
}

/// Bootloader offset per chip family (esptool/IDF conventions).
pub fn bootloader_offset(chip: Option<&str>) -> u32 {
    match chip {
        Some("esp32" | "esp32s2") | None => 0x1000,
        Some("esp32p4" | "esp32c5") => 0x2000,
        Some(_) => 0x0,
    }
}

pub fn detect(path: &Path) -> Result<Project, String> {
    detect_all(path)?
        .into_iter()
        .next()
        .ok_or_else(|| format!("No ESP build output found in {}", path.display()))
}

/// Every build found. A sketch compiled for several boards has one build dir
/// per FQBN, so a directory can yield more than one project.
pub fn detect_all(path: &Path) -> Result<Vec<Project>, String> {
    if !path.is_dir() {
        return detect_file(path).map(|p| vec![p]);
    }
    let mut out = Vec::new();
    for dir in candidate_dirs(path) {
        if let Some(p) = detect_build_dir(&dir)? {
            out.push(p);
        }
    }
    if out.is_empty() {
        // Plain folder with a single image in it.
        let bins = bins_in(path);
        if let [only] = bins.as_slice() {
            out.push(single_bin(only));
        }
    }
    Ok(out)
}

/// `dir`, `dir/build`, and the immediate children of both (Arduino puts each
/// FQBN in `build/<vendor.arch.board>/`).
fn candidate_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![dir.to_path_buf()];
    let build = dir.join("build");
    if build.is_dir() {
        out.push(build.clone());
    }
    for parent in [dir.to_path_buf(), build] {
        let Ok(rd) = fs::read_dir(&parent) else { continue };
        let mut kids: Vec<PathBuf> =
            rd.flatten().map(|e| e.path()).filter(|p| p.is_dir() && !out.contains(p)).collect();
        kids.sort();
        out.extend(kids);
    }
    out
}

fn detect_build_dir(dir: &Path) -> Result<Option<Project>, String> {
    let json = dir.join("flasher_args.json");
    if json.is_file() {
        return from_idf_json(&json).map(Some);
    }
    let args = dir.join("flash_args");
    if args.is_file() {
        return from_flash_args(&args).map(Some);
    }
    // Arduino ESP8266 writes no manifest: just `<sketch>.ino.bin` in `build/<fqbn>/`.
    let ino: Vec<PathBuf> =
        bins_in(dir).into_iter().filter(|b| file_name(b).ends_with(".ino.bin")).collect();
    if let [only] = ino.as_slice() {
        return Ok(Some(single_bin(only)));
    }
    Ok(None)
}

fn detect_file(path: &Path) -> Result<Project, String> {
    let name = file_name(path);
    match name.as_str() {
        "flasher_args.json" => return from_idf_json(path),
        "flash_args" => return from_flash_args(path),
        "build.options.json" | "sdkconfig" => {
            let dir = path.parent().unwrap_or(Path::new("."));
            return detect_build_dir(dir)?
                .ok_or_else(|| format!("No flashable output next to {}", path.display()));
        }
        _ => {}
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "csv" => {
            let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let parts = partition::parse_csv(&text, partition::DEFAULT_TABLE_OFFSET)?;
            Ok(from_table(path, parts, None))
        }
        "bin" => {
            let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            if partition::is_binary_table(&bytes) {
                let parts = partition::parse_bin(&bytes)?;
                Ok(from_table(path, parts, Some(path.to_path_buf())))
            } else {
                Ok(single_bin(path))
            }
        }
        _ => Err(format!("Don't know how to flash {name}")),
    }
}

// ---------------------------------------------------------------- manifests

fn from_idf_json(path: &Path) -> Result<Project, String> {
    let dir = parent(path);
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let s = |k: &str| v["flash_settings"][k].as_str().map(str::to_owned);
    let settings = FlashSettings { mode: s("flash_mode"), freq: s("flash_freq"), size: s("flash_size") };
    let chip = v["extra_esptool_args"]["chip"].as_str().map(str::to_owned);

    let obj = v["flash_files"].as_object().ok_or("flasher_args.json: missing flash_files")?;
    let mut files = Vec::new();
    for (off, rel) in obj {
        let offset = partition::parse_num(off).ok_or_else(|| format!("bad offset {off}"))?;
        let rel = rel.as_str().ok_or("flash_files: path is not a string")?;
        // Named sections ("bootloader", "app", "partition-table", "otadata", ...) give labels.
        let label = v
            .as_object()
            .into_iter()
            .flatten()
            .find(|(_, s)| s["offset"].as_str().and_then(partition::parse_num) == Some(offset))
            .map(|(k, _)| pretty_label(k))
            .unwrap_or_else(|| label_for_file(rel));
        files.push(FlashFile { offset, path: Some(dir.join(rel)), label, max_size: None, enabled: true });
    }
    files.sort_by_key(|f| f.offset);

    let partitions = v["partition-table"]["file"]
        .as_str()
        .and_then(|rel| fs::read(dir.join(rel)).ok())
        .and_then(|b| partition::parse_bin(&b).ok());

    let mut p = Project {
        source: Source::IdfFlasherArgs,
        origin: path.to_path_buf(),
        chip,
        settings,
        files,
        partitions,
        merged: None,
    };
    add_partition_slots(&mut p);
    Ok(p)
}

fn from_flash_args(path: &Path) -> Result<Project, String> {
    let dir = parent(path);
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut settings = FlashSettings::default();
    let mut files = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line.starts_with("--") {
            let mut toks = line.split_whitespace();
            while let Some(flag) = toks.next() {
                // IDF writes `--flash_mode`, esptool v5 wants `--flash-mode`.
                let slot = match flag.replace('_', "-").as_str() {
                    "--flash-mode" => &mut settings.mode,
                    "--flash-freq" => &mut settings.freq,
                    "--flash-size" => &mut settings.size,
                    _ => continue,
                };
                *slot = toks.next().map(str::to_owned);
            }
        } else if let Some((off, rel)) = line.split_once(char::is_whitespace) {
            let offset = partition::parse_num(off)
                .ok_or_else(|| format!("{}: bad line `{line}`", path.display()))?;
            let rel = rel.trim();
            files.push(FlashFile {
                offset,
                path: Some(resolve_arduino(&dir, rel)),
                label: label_for_file(rel),
                max_size: None,
                enabled: true,
            });
        }
    }
    files.sort_by_key(|f| f.offset);

    // Arduino drops the source CSV next to the build; otherwise read the binary table we flash.
    let partitions = fs::read_to_string(dir.join("partitions.csv"))
        .ok()
        .and_then(|t| partition::parse_csv(&t, partition::DEFAULT_TABLE_OFFSET).ok())
        .or_else(|| {
            files.iter().filter_map(|f| fs::read(f.path.as_ref()?).ok()).find_map(|b| {
                partition::is_binary_table(&b).then(|| partition::parse_bin(&b).ok()).flatten()
            })
        });

    let mut p = Project {
        source: Source::FlashArgs,
        origin: path.to_path_buf(),
        chip: chip_near(&dir),
        settings,
        files,
        partitions,
        merged: bins_in(&dir).into_iter().find(|b| file_name(b).ends_with(".merged.bin")),
    };
    add_partition_slots(&mut p);
    Ok(p)
}

/// Only a partition table was given: build the standard ESP32 layout around it
/// and offer one picker per partition that takes an image.
fn from_table(path: &Path, parts: Vec<Partition>, table_bin: Option<PathBuf>) -> Project {
    let dir = parent(path);
    let chip = chip_near(&dir);
    let mut files = vec![
        FlashFile {
            offset: bootloader_offset(chip.as_deref()),
            path: None,
            label: "Bootloader".into(),
            max_size: None,
            enabled: true,
        },
        FlashFile {
            offset: partition::DEFAULT_TABLE_OFFSET,
            // A CSV can't be flashed directly; the user must pick the compiled table.
            path: table_bin,
            label: "Partition table".into(),
            max_size: Some(0x1000),
            enabled: true,
        },
    ];
    if let Some(ota) = parts.iter().find(|p| p.ptype == partition::TYPE_DATA && p.subtype == 0x00) {
        files.push(FlashFile {
            offset: ota.offset,
            path: None,
            label: "OTA data (boot_app0)".into(),
            max_size: Some(ota.size),
            enabled: true,
        });
    }
    let mut p = Project {
        source: Source::PartitionTable,
        origin: path.to_path_buf(),
        chip,
        settings: FlashSettings::default(),
        files,
        partitions: Some(parts),
        merged: None,
    };
    add_partition_slots(&mut p);
    autofill(&dir, &mut p.files);
    p
}

fn single_bin(path: &Path) -> Project {
    let dir = parent(path);
    Project {
        source: Source::SingleBin,
        origin: path.to_path_buf(),
        chip: chip_near(&dir).or_else(|| chip_from_image(path)),
        settings: FlashSettings::default(),
        files: vec![FlashFile {
            offset: 0,
            path: Some(path.to_path_buf()),
            label: "Firmware".into(),
            max_size: None,
            enabled: true,
        }],
        partitions: None,
        merged: None,
    }
}

// ---------------------------------------------------------------- helpers

/// Annotate files with their partition size, and add an (unticked) picker for
/// every app / filesystem / nvs partition nothing is written to yet.
fn add_partition_slots(p: &mut Project) {
    let Some(parts) = &p.partitions else { return };
    // Everything below the table (the bootloader) must end before it; the table is 0xC00 max.
    let table = p.files.iter().find(|f| f.label == "Partition table").map(|f| f.offset);
    for f in &mut p.files {
        match table {
            Some(t) if f.offset == t => _ = f.max_size.get_or_insert(0xC00),
            Some(t) if f.offset < t => _ = f.max_size.get_or_insert(t - f.offset),
            _ => {}
        }
    }
    for f in &mut p.files {
        if let Some(part) = parts.iter().find(|pt| pt.offset == f.offset) {
            f.max_size.get_or_insert(part.size);
        }
    }
    // Only the first app slot (factory / ota_0) is needed; ota_1.. are filled by OTA.
    let mut need_app = !parts
        .iter()
        .any(|pt| pt.slot() == Slot::App && p.files.iter().any(|f| f.offset == pt.offset));
    for part in parts {
        if p.files.iter().any(|f| f.offset == part.offset) {
            continue;
        }
        let (kind, enabled) = match part.slot() {
            Slot::App if need_app => {
                need_app = false;
                ("app", true)
            }
            Slot::Filesystem => ("filesystem", false),
            Slot::Nvs => ("nvs", false),
            _ => continue,
        };
        p.files.push(FlashFile {
            offset: part.offset,
            path: None,
            label: if part.name == kind { part.name.clone() } else { format!("{} ({kind})", part.name) },
            max_size: Some(part.size),
            enabled,
        });
    }
    p.files.sort_by_key(|f| f.offset);
}

/// Fill empty slots from obviously-named siblings in `dir`.
fn autofill(dir: &Path, files: &mut [FlashFile]) {
    let bins = bins_in(dir);
    let used = |b: &PathBuf, files: &[FlashFile]| files.iter().any(|f| f.path.as_ref() == Some(b));
    for i in 0..files.len() {
        if files[i].path.is_some() {
            continue;
        }
        let label = files[i].label.to_ascii_lowercase();
        let pick = bins.iter().find(|b| {
            let n = file_name(b);
            !used(b, files)
                && if label.starts_with("bootloader") {
                    n.contains("bootloader")
                } else if label.starts_with("ota data") {
                    n == "boot_app0.bin" || n == "ota_data_initial.bin"
                } else if label == "partition table" {
                    fs::read(b).is_ok_and(|d| partition::is_binary_table(&d))
                } else {
                    false
                }
        });
        files[i].path = pick.cloned();
    }
    // A single remaining unclaimed image goes into the (single) empty app slot.
    let leftovers: Vec<&PathBuf> = bins
        .iter()
        .filter(|b| {
            let n = file_name(b);
            !used(b, files)
                && !n.ends_with(".merged.bin")
                && !n.contains("partition")
                && fs::read(b).map_or(true, |d| !partition::is_binary_table(&d))
        })
        .collect();
    let empty_apps: Vec<usize> = (0..files.len())
        .filter(|&i| files[i].path.is_none() && files[i].label.ends_with("(app)"))
        .collect();
    if let ([only], [slot]) = (leftovers.as_slice(), empty_apps.as_slice()) {
        files[*slot].path = Some((*only).clone());
    }
}

/// Chip from `sdkconfig` (Arduino-ESP32 copies it into the build dir), or the
/// Arduino FQBN: the build dir name (`esp8266.esp8266.nodemcuv2`) or `build.options.json`.
/// `flash_args` names `boot_app0.bin` but arduino-cli with `--output-dir` doesn't
/// copy it; it lives in the core's `tools/partitions/` (listed in build.options.json).
fn resolve_arduino(dir: &Path, rel: &str) -> PathBuf {
    let local = dir.join(rel);
    if local.exists() {
        return local;
    }
    fs::read_to_string(dir.join("build.options.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| {
            v["hardwareFolders"].as_str()?.split(',').map(|hw| Path::new(hw).join("tools/partitions").join(rel)).find(|p| p.exists())
        })
        .unwrap_or(local)
}

/// Chip from an app/bootloader image header. ESP32-family images carry an
/// extended header: wp_pin (0xEE) at byte 8 and chip id at 12; ESP8266 has none.
pub fn chip_from_image(path: &Path) -> Option<String> {
    let mut h = [0u8; 24];
    std::io::Read::read_exact(&mut fs::File::open(path).ok()?, &mut h).ok()?;
    if h[0] != 0xE9 {
        return None;
    }
    if h[8] != 0xEE {
        return Some("esp8266".into());
    }
    let chip = match u16::from_le_bytes([h[12], h[13]]) {
        0 => "esp32",
        2 => "esp32s2",
        5 => "esp32c3",
        9 => "esp32s3",
        12 => "esp32c2",
        13 => "esp32c6",
        16 => "esp32h2",
        18 => "esp32p4",
        23 => "esp32c5",
        _ => return None,
    };
    Some(chip.into())
}

fn chip_near(dir: &Path) -> Option<String> {
    if file_name(dir).starts_with("esp8266.") {
        return Some("esp8266".into());
    }
    if let Ok(cfg) = fs::read_to_string(dir.join("sdkconfig")) {
        let chip = cfg
            .lines()
            .find_map(|l| l.strip_prefix("CONFIG_IDF_TARGET=\""))
            .map(|s| s.trim_end_matches('"').to_owned());
        if chip.is_some() {
            return chip;
        }
    }
    let opts = fs::read_to_string(dir.join("build.options.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&opts).ok()?;
    let fqbn = v["fqbn"].as_str()?;
    fqbn.starts_with("esp8266:").then(|| "esp8266".to_owned())
}

fn bins_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bin")))
        .collect();
    v.sort();
    v
}

fn label_for_file(rel: &str) -> String {
    let n = rel.rsplit(['/', '\\']).next().unwrap_or(rel).to_ascii_lowercase();
    if n.contains("bootloader") {
        "Bootloader"
    } else if n.contains("partition") {
        "Partition table"
    } else if n == "boot_app0.bin" || n.contains("ota_data") {
        "OTA data (boot_app0)"
    } else if n.ends_with(".merged.bin") {
        "Merged image"
    } else {
        "Application"
    }
    .into()
}

fn pretty_label(key: &str) -> String {
    match key {
        "bootloader" => "Bootloader".into(),
        "app" => "Application".into(),
        "partition-table" => "Partition table".into(),
        "otadata" => "OTA data".into(),
        other => other.into(),
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn parent(p: &Path) -> PathBuf {
    p.parent().map(Path::to_path_buf).unwrap_or_default()
}

// ---------------------------------------------------------------- checks

/// Human-readable problems that should block or warn before flashing.
pub fn validate(files: &[FlashFile]) -> Vec<String> {
    let mut issues = Vec::new();
    let mut spans = Vec::new();
    for f in files.iter().filter(|f| f.enabled) {
        let Some(path) = &f.path else {
            issues.push(format!("{} @ {:#x}: no file selected", f.label, f.offset));
            continue;
        };
        let len = match fs::metadata(path) {
            Ok(m) => m.len(),
            Err(_) => {
                issues.push(format!("{}: file not found: {}", f.label, path.display()));
                continue;
            }
        };
        if let Some(max) = f.max_size {
            if len > max as u64 {
                issues.push(format!(
                    "{}: {} bytes does not fit its partition ({} bytes)",
                    f.label, len, max
                ));
            }
        }
        spans.push((f.offset as u64, f.offset as u64 + len, &f.label));
    }
    spans.sort();
    for w in spans.windows(2) {
        if w[0].1 > w[1].0 {
            issues.push(format!("{} overlaps {} at {:#x}", w[0].2, w[1].2, w[1].0));
        }
    }
    issues
}
