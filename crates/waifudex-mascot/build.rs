fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("target os");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let native_lib_dir =
        std::path::Path::new(&manifest_dir).join("../../third_party/inochi2d-c/out");
    println!("cargo:rerun-if-changed={}", native_lib_dir.display());

    if target_os == "linux" {
        let native_lib_dir = native_lib_dir.canonicalize().expect("native lib dir");
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            native_lib_dir.display()
        );
    }
}
