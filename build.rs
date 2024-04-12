use std::{env, fs::File, io::Write as _, path::Path, process::Command};

const WEB_OUT: &str = "/web/dist";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    embuild::espidf::sysenv::output();

    let bun_result = Command::new("bun")
        .current_dir("web")
        .arg("run")
        .arg("build")
        .arg("--emptyOutDir")
        .arg("--outDir")
        .arg(env::var("OUT_DIR")? + WEB_OUT)
        .status()
        .expect("web build failed");
    assert!(bun_result.success());

    println!("cargo:rerun-if-changed=web/src");

    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("web_files.rs");
    let mut web_files = File::create(dest_path)?;

    writeln!(&mut web_files, r##"["##,)?;

    for f in walkdir::WalkDir::new(env::var("OUT_DIR")? + WEB_OUT) {
        let f = f?;

        if f.file_type().is_dir() {
            continue;
        }

        writeln!(
            &mut web_files,
            r##"("/{name}", include_bytes!(r#"{path}"#)),"##,
            name = f
                .path()
                .display()
                .to_string()
                .split_off(env::var("OUT_DIR")?.len() + WEB_OUT.len() + 1),
            path = f.path().display()
        )?;
    }

    writeln!(&mut web_files, r##"]"##,)?;

    Ok(())
}
