use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/opentrack.rc");
    println!("cargo:rerun-if-changed=assets/opentrack-icon.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    let architecture = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => "x64",
        Ok("x86") => "x86",
        Ok("aarch64") => "arm64",
        other => panic!("unsupported Windows icon resource target: {other:?}"),
    };
    let resource_compiler = find_resource_compiler(architecture)
        .expect("Windows SDK rc.exe is required to embed the OpenTrack application icon");
    let resource = output.join("opentrack-icon.res");
    let status = Command::new(&resource_compiler)
        .current_dir(&manifest)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg("assets/opentrack.rc")
        .status()
        .unwrap_or_else(|error| panic!("could not run {}: {error}", resource_compiler.display()));
    assert!(
        status.success(),
        "rc.exe failed to compile the OpenTrack icon resource"
    );
    println!("cargo:rustc-link-arg-bins={}", resource.display());
}

fn find_resource_compiler(architecture: &str) -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Some(sdk) = env::var_os("WindowsSdkDir").filter(|path| !path.is_empty()) {
        roots.push(PathBuf::from(sdk).join("bin"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files).join("Windows Kits/10/bin"));
    }
    roots.push(Path::new("C:/Program Files (x86)/Windows Kits/10/bin").to_path_buf());

    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        let mut versions = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        versions.sort_by(|left, right| right.cmp(left));
        for version in versions {
            let compiler = version.join(architecture).join("rc.exe");
            if compiler.is_file() {
                return Some(compiler);
            }
        }
    }
    None
}
