use std::{env, path::PathBuf};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("target os");
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let project_root = crate_dir
        .parent()
        .and_then(|parent| parent.parent())
        .expect("project root");
    let header_dir = project_root.join("third_party/inochi2d-c");
    let header_path = header_dir.join("inochi2d.h");
    let wrapper_path = crate_dir.join("wrapper.h");
    let out_dir = header_dir.join("out");

    println!("cargo:rerun-if-changed={}", wrapper_path.display());
    println!("cargo:rerun-if-changed={}", header_path.display());

    if out_dir.exists() {
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        if target_os == "linux" {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", out_dir.display());
        }
    } else {
        println!(
            "cargo:warning=inochi2d-c build output not found at {}; run scripts/build-inochi2d.sh before enabling native integration",
            out_dir.display()
        );
    }
    if target_os == "linux" {
        println!("cargo:rustc-link-lib=dylib=inochi2d-c");
        println!("cargo:rustc-link-lib=dylib=z");
        println!("cargo:rustc-link-lib=dylib=EGL");
        println!("cargo:rustc-link-lib=dylib=GL");
    } else if target_os == "windows" {
        println!("cargo:rustc-link-lib=dylib=inochi2d-c");
    }

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.display().to_string())
        .clang_arg(format!("-I{}", header_dir.display()))
        .allowlist_function("in.*")
        .allowlist_type("In.*")
        .allowlist_var("INOCHI2D_.*")
        .blocklist_function("inPuppetGetName")
        .generate()
        .expect("generate inochi2d bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("write bindings");
}
