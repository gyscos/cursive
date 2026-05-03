use std::env;
use std::path::Path;

fn main() {
    // Declare the custom cfg so rustc's check-cfg doesn't warn about it.
    println!("cargo:rustc-check-cfg=cfg(has_ncurses_button5)");

    // The ncurses crate (v6+) generates a `raw_constants.rs` file from the
    // system ncurses headers. On systems where the headers define BUTTON5_*
    // (Linux with ncurses 6+), those constants appear in the generated file.
    // On systems where they don't (macOS with its old system ncurses), they
    // are absent.
    //
    // We scan that generated file to determine whether BUTTON5 support is
    // available, and set cfg(has_ncurses_button5) accordingly. This avoids
    // needing a C compiler probe in our own build script.
    if let Ok(dep_dir) = env::var("DEP_NCURSES_OUT_DIR") {
        let raw_constants = Path::new(&dep_dir).join("raw_constants.rs");
        if let Ok(contents) = std::fs::read_to_string(&raw_constants) {
            if contents.contains("BUTTON5_PRESSED") {
                println!("cargo:rustc-cfg=has_ncurses_button5");
                return;
            }
        }
    }

    // Fallback: scan the ncurses build output directory from cargo's OUT_DIR.
    // The ncurses crate's build artifacts are in a sibling directory under
    // target/<profile>/build/ncurses-<hash>/out/.
    if let Ok(out_dir) = env::var("OUT_DIR") {
        // OUT_DIR is something like target/debug/build/cursive-ncurses-<hash>/out
        // We need target/debug/build/ncurses-<hash>/out/raw_constants.rs
        if let Some(build_dir) = Path::new(&out_dir).parent().and_then(|p| p.parent()) {
            if let Ok(entries) = std::fs::read_dir(build_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    if name.to_string_lossy().starts_with("ncurses-") {
                        let raw_constants = entry.path().join("out/raw_constants.rs");
                        if let Ok(contents) = std::fs::read_to_string(&raw_constants) {
                            if contents.contains("BUTTON5_PRESSED") {
                                println!("cargo:rustc-cfg=has_ncurses_button5");
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    // If we couldn't find the file or BUTTON5 isn't in it, leave the cfg unset.
}
