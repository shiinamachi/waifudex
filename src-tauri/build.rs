fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("target os");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    if target_os == "linux" {
        let native_lib_dir = std::path::Path::new(&manifest_dir)
            .join("../third_party/inochi2d-c/out")
            .canonicalize()
            .expect("native lib dir");
        let so_path = std::path::Path::new(&manifest_dir)
            .join("../third_party/inochi2d-c/out/libinochi2d-c.so");
        println!("cargo:rerun-if-changed={}", so_path.display());
        if so_path.exists() {
            std::env::set_var(
                "TAURI_CONFIG",
                r#"{"bundle":{"resources":["../third_party/inochi2d-c/out/libinochi2d-c.so"]}}"#,
            );
        }
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            native_lib_dir.display()
        );
    } else if target_os == "windows" {
        let dll_path = std::path::Path::new(&manifest_dir)
            .join("../third_party/inochi2d-c/out/inochi2d-c.dll");
        println!("cargo:rerun-if-changed={}", dll_path.display());

        if dll_path.exists() {
            std::env::set_var(
                "TAURI_CONFIG",
                r#"{"bundle":{"resources":["../third_party/inochi2d-c/out/inochi2d-c.dll"]}}"#,
            );
            let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
            if let Some(profile_dir) = out_dir.ancestors().nth(3) {
                let staged_dll_path = profile_dir.join("inochi2d-c.dll");
                if let Err(error) = std::fs::copy(&dll_path, &staged_dll_path) {
                    panic!(
                        "failed to stage {} into {}: {error}",
                        dll_path.display(),
                        staged_dll_path.display()
                    );
                }
            }
        } else {
            println!(
                "cargo:warning=inochi2d-c.dll not found at {}; build it on Windows and copy it into third_party/inochi2d-c/out before packaging",
                dll_path.display()
            );
        }
    }
    tauri_build::build()
}
