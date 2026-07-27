fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    slint_build::compile(format!("{}/ui/app.slint", manifest_dir)).unwrap();
}
