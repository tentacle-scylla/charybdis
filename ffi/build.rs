//! Generate C header using cbindgen

fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let config = cbindgen::Config::from_file(format!("{}/cbindgen.toml", crate_dir))
        .expect("Failed to read cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file("../include/charybdis.h");
}
