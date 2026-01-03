//! Simplified build script for library mode
//! Generates version_info.rs without requiring git repo

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("version_info.rs");

    let pkg_version = env!("CARGO_PKG_VERSION");

    let content = format!(
        r#"
pub const PKG_VERSION: &str = "{}";
pub const COMMIT_DATE: &str = "library-build";
pub const GIT_SHA: &str = "library-build";
pub const SCYLLA_DRIVER_VERSION: &str = "1.4.1";
pub const SCYLLA_DRIVER_RELEASE_DATE: &str = "2024";
pub const SCYLLA_DRIVER_SHA: &str = "crates.io";
"#,
        pkg_version
    );

    fs::write(&dest_path, content).unwrap();
}
