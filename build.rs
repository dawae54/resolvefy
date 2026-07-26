use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let input = format!("{}/ui/app.blp", manifest_dir);
    let out_ui = format!("{}/app.ui", out_dir);

    // Try blueprint-compiler first, then fallback to blueprint
    let try_cmd = |cmd: &str| {
        Command::new(cmd)
            .args(["compile", &input, "--output", &out_ui])
            .status()
            .ok()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    let compiled = try_cmd("blueprint-compiler") || try_cmd("blueprint");

    if !compiled {
        panic!("Failed to run 'blueprint-compiler' or 'blueprint'. Please install the GNOME blueprint compiler to build the UI: https://gitlab.gnome.org/GNOME/blueprint");
    }

    // Verify file was written
    if !fs::metadata(&out_ui).is_ok() {
        panic!("Blueprint compiler did not produce {}, build cannot continue", out_ui);
    }

    // Keep a copy in OUT_DIR (already there) so src can include_str!(concat!(env!("OUT_DIR"), "/app.ui"))
    println!("cargo:rerun-if-changed={}/ui/app.blp", manifest_dir);
}

