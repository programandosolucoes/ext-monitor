#[path = "../tools/ext-tool/src/splash.rs"]
mod splash;

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../tools/ext-tool/src/splash.rs");

    // 100% Pure Rust Splash Generator during cargo build:
    // Ensures splash_loading.raw.gz, splash_ready.raw.gz, and splash_miracast.raw.gz
    // are automatically generated with the exact current release version
    // before ext-receiver compiles and bakes them via include_bytes!
    let project_root = Path::new("..");
    if let Err(e) = splash::generate_all_splashes(project_root) {
        eprintln!("cargo:warning=Pure-Rust splash generation warning: {}", e);
    }
}
