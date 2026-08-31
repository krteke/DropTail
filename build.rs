use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    const SCHEMA_DIR: &str = "data";
    const SCHEMA_FILE: &str = "data/io.github.krteke.DropTail.gschema.xml";

    println!("cargo:rerun-if-changed={SCHEMA_FILE}");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"));
    let status = Command::new("glib-compile-schemas")
        .args(["--strict", "--targetdir"])
        .arg(&out_dir)
        .arg(SCHEMA_DIR)
        .status()
        .expect("glib-compile-schemas is required to build DropTail");

    assert!(status.success(), "failed to compile the GSettings schema");
    println!(
        "cargo:rustc-env=DROPTAIL_GSETTINGS_SCHEMA_DIR={}",
        out_dir.display()
    );
}
