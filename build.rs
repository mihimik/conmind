use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let dest_path = Path::new(&manifest_dir)
        .join("target")
        .join(profile)
        .join("config.toml");
    let _ = fs::copy("config.toml", dest_path);

    println!("cargo:rerun-if-changed=config.toml");
}