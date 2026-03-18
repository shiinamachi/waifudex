fn main() {
    #[cfg(target_os = "linux")]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
        let native_lib_dir = std::path::Path::new(&manifest_dir)
            .join("../third_party/inochi2d-c/out")
            .canonicalize()
            .expect("native lib dir");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            native_lib_dir.display()
        );
    }
    tauri_build::build()
}
