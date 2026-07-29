fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    slint_build::compile_with_config(
        format!("{}/ui/app.slint", manifest_dir),
        slint_build::CompilerConfiguration::new().with_style("cosmic".into()),
    )
    .unwrap();
}
