fn main() {
    println!("cargo:rerun-if-changed=ui/window.blp");
    println!("cargo:rerun-if-changed=ui/window.ui");

    let status = std::process::Command::new("blueprint-compiler")
        .args(["compile", "ui/window.blp", "--output", "ui/window.ui"])
        .status()
        .expect("failed to run blueprint-compiler (install it with: pip install blueprint-compiler)");

    assert!(status.success(), "blueprint-compiler failed to compile ui/window.blp");
}