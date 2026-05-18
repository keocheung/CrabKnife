use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }

    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_owned());
    let (major, minor, patch, build) = version_numbers(&version);
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
    let rc_path = out_dir.join("version.rc");
    let res_path = out_dir.join("version.res");

    fs::write(
        &rc_path,
        format!(
            r#"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},{build}
PRODUCTVERSION {major},{minor},{patch},{build}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "CrabKnife\0"
            VALUE "FileDescription", "CrabKnife\0"
            VALUE "FileVersion", "{version}\0"
            VALUE "InternalName", "crab-knife.exe\0"
            VALUE "OriginalFilename", "crab-knife.exe\0"
            VALUE "ProductName", "CrabKnife\0"
            VALUE "ProductVersion", "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
        ),
    )
    .expect("failed to write version resource");

    compile_resource(&target, &rc_path, &res_path);
    println!("cargo:rerun-if-changed=Cargo.toml");
}

fn version_numbers(version: &str) -> (u16, u16, u16, u16) {
    let mut parts = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u16>().unwrap_or(0));

    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn compile_resource(target: &str, rc_path: &Path, res_path: &Path) {
    if target.contains("msvc") {
        let rc_exe = find_rc_exe(target).unwrap_or_else(|| PathBuf::from("rc.exe"));
        let mut output_arg = OsString::from("/fo");
        output_arg.push(res_path);

        let status = Command::new(&rc_exe)
            .arg("/nologo")
            .arg(output_arg)
            .arg(rc_path)
            .status()
            .unwrap_or_else(|err| panic!("failed to run {}: {err}", rc_exe.display()));

        assert!(status.success(), "{} failed", rc_exe.display());
        println!("cargo:rustc-link-arg-bins={}", res_path.display());
        return;
    }

    let obj_path = res_path.with_extension("o");
    let status = Command::new("windres")
        .arg(rc_path)
        .arg(&obj_path)
        .status()
        .expect("failed to run windres");

    assert!(status.success(), "windres failed");
    println!("cargo:rustc-link-arg-bins={}", obj_path.display());
}

fn find_rc_exe(target: &str) -> Option<PathBuf> {
    let arch = if target.contains("aarch64") {
        "arm64"
    } else if target.contains("i686") {
        "x86"
    } else {
        "x64"
    };

    let kits_root = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
        .join("Windows Kits")
        .join("10")
        .join("bin");

    let mut candidates = fs::read_dir(kits_root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join(arch).join("rc.exe"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    candidates.sort();
    candidates.pop()
}
