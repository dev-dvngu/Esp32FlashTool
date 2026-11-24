//! Fixtures are the manifests/partition tables of real builds (arduino-cli 1.4,
//! esp32 core 3.3.7, esp8266 core 3.1.2, ESP-IDF 5.5.2). Images are empty placeholders.

use flashcore::detect::{detect, detect_all, validate, Source};
use std::path::{Path, PathBuf};

fn fx(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(rel)
}

fn layout(p: &flashcore::detect::Project) -> Vec<(u32, String, bool)> {
    p.files
        .iter()
        .map(|f| {
            let name = f.path.as_ref().map_or("-".into(), |p| p.file_name().unwrap().to_string_lossy().into_owned());
            (f.offset, name, f.enabled)
        })
        .collect()
}

#[test]
fn idf_project_root() {
    let p = detect(&fx("idf-esp32")).unwrap();
    assert_eq!(p.source, Source::IdfFlasherArgs);
    assert_eq!(p.chip.as_deref(), Some("esp32"));
    assert_eq!(p.settings.size.as_deref(), Some("2MB"));
    assert_eq!(p.settings.freq.as_deref(), Some("40m"));
    assert_eq!(
        layout(&p),
        vec![
            (0x1000, "bootloader.bin".into(), true),
            (0x8000, "partition-table.bin".into(), true),
            (0x9000, "-".into(), false), // optional nvs image
            (0x10000, "blink.bin".into(), true),
        ]
    );
    // nvs / phy_init / factory from the binary table.
    let parts = p.partitions.as_ref().unwrap();
    assert_eq!(parts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["nvs", "phy_init", "factory"]);
    assert_eq!(p.files[3].max_size, Some(0x100000));
    assert_eq!(p.files[0].label, "Bootloader");
}

#[test]
fn arduino_esp32_sketch_root() {
    let p = detect(&fx("arduino-esp32")).unwrap();
    assert_eq!(p.source, Source::FlashArgs);
    assert_eq!(p.chip.as_deref(), Some("esp32"));
    assert_eq!(p.settings.mode.as_deref(), Some("dio"));
    assert_eq!(p.settings.size.as_deref(), Some("4MB"));
    assert!(p.merged.as_ref().unwrap().ends_with("blink.ino.merged.bin"));
    assert_eq!(
        layout(&p),
        vec![
            (0x1000, "blink.ino.bootloader.bin".into(), true),
            (0x8000, "blink.ino.partitions.bin".into(), true),
            (0x9000, "-".into(), false),  // nvs slot, optional
            (0xe000, "boot_app0.bin".into(), true),
            (0x10000, "blink.ino.bin".into(), true),
            (0x290000, "-".into(), false), // spiffs slot, optional
        ]
    );
    assert_eq!(p.files[4].max_size, Some(0x140000));
    // Optional empty slots don't block flashing.
    assert!(validate(&p.files).is_empty(), "{:?}", validate(&p.files));
}

#[test]
fn arduino_esp8266_single_bin() {
    let p = detect(&fx("arduino-esp8266")).unwrap();
    assert_eq!(p.source, Source::SingleBin);
    assert_eq!(p.chip.as_deref(), Some("esp8266"));
    assert_eq!(layout(&p), vec![(0, "blink.ino.bin".into(), true)]);
}

#[test]
fn partition_csv_alone_offers_slots_and_autofills() {
    let p = detect(&fx("arduino-esp32/build/esp32.esp32.esp32/partitions.csv")).unwrap();
    assert_eq!(p.source, Source::PartitionTable);
    let l = layout(&p);
    assert_eq!(l[0], (0x1000, "blink.ino.bootloader.bin".into(), true));
    // CSV isn't flashable; the compiled sibling table is picked up instead.
    assert_eq!(l[1], (0x8000, "blink.ino.partitions.bin".into(), true));
    assert_eq!(l[2], (0x9000, "-".into(), false)); // nvs
    assert_eq!(l[3], (0xe000, "boot_app0.bin".into(), true));
    assert_eq!(l[4], (0x10000, "blink.ino.bin".into(), true)); // app0 autofilled
    assert!(!l.iter().any(|f| f.0 == 0x150000), "ota_1 is not offered");
    assert!(validate(&p.files).is_empty());
}

#[test]
fn lone_csv_without_siblings_asks_for_everything() {
    let dir = tmp("lonecsv");
    std::fs::copy(fx("arduino-esp32/build/esp32.esp32.esp32/partitions.csv"), dir.join("p.csv")).unwrap();
    let p = detect(&dir.join("p.csv")).unwrap();
    let issues = validate(&p.files);
    for want in ["Bootloader", "Partition table", "OTA data", "app0 (app)"] {
        assert!(issues.iter().any(|i| i.starts_with(want)), "missing {want}: {issues:?}");
    }
}

#[test]
fn partition_bin_alone_is_recognised() {
    let p = detect(&fx("idf-esp32/build/partition_table/partition-table.bin")).unwrap();
    assert_eq!(p.source, Source::PartitionTable);
    assert!(p.files[1].path.as_ref().unwrap().ends_with("partition-table.bin"));
    assert!(p.files.iter().any(|f| f.label == "factory (app)" && f.enabled));
}

#[test]
fn multi_board_sketch_yields_one_project_per_fqbn() {
    // One sketch compiled for both cores: build/esp32.esp32.esp32 + build/esp8266.esp8266.nodemcuv2.
    let sketch = tmp("multi");
    for src in ["arduino-esp32", "arduino-esp8266"] {
        copy_dir(&fx(src), &sketch);
    }
    let all = detect_all(&sketch).unwrap();
    let chips: Vec<_> = all.iter().map(|p| p.chip.clone().unwrap()).collect();
    assert_eq!(chips, ["esp32", "esp8266"]);
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashcore-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn copy_dir(src: &Path, dst: &Path) {
    for e in std::fs::read_dir(src).unwrap().flatten() {
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            std::fs::create_dir_all(&to).unwrap();
            copy_dir(&e.path(), &to);
        } else {
            std::fs::copy(e.path(), to).unwrap();
        }
    }
}
