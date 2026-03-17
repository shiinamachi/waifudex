use std::{fs, io, path::PathBuf};

fn main() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_path = manifest_dir
        .parent()
        .expect("manifest has a parent directory")
        .join("src/lib/contracts/generated/runtime.ts");

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut contents = waifudex_lib::contracts::runtime::render_typescript_contract();
    contents.push('\n');
    fs::write(&output_path, contents)?;

    println!("exported runtime contracts to {}", output_path.display());
    Ok(())
}
