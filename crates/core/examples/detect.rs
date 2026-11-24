//! `cargo run -p flashtool-core --example detect -- <path>`: print the flash plan.
fn main() {
    for arg in std::env::args().skip(1) {
        match flashcore::detect::detect_all(arg.as_ref()) {
            Ok(all) => for p in all {
                println!("{:?} chip={:?} {:?} merged={:?}", p.source, p.chip, p.settings, p.merged.is_some());
                for f in &p.files {
                    println!("  [{}] {:#08x} {:<22} {}", if f.enabled {'x'} else {' '}, f.offset, f.label,
                        f.path.as_ref().map_or("<pick>".into(), |p| p.display().to_string()));
                }
                for i in flashcore::detect::validate(&p.files) { println!("  ! {i}"); }
            },
            Err(e) => println!("{arg}: {e}"),
        }
    }
}
