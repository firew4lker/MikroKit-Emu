// Windows only:
// - the main thread gets an 8 MB stack like on Linux (Windows' default is
//   1 MB, and a stack overflow there ends the program silently);
// - the program icon (assets/icon.ico) is embedded, so Explorer and the
//   taskbar show it.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }
    let gnu = target.ends_with("-gnu") || target.ends_with("-gnullvm");
    if gnu {
        println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608");
    } else {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("icon.ico");
    let rc = out.join("icon.rc");
    let ico_str = ico.display().to_string().replace('\\', "\\\\");
    std::fs::write(&rc, format!("1 ICON \"{ico_str}\"\n")).unwrap();
    if gnu {
        // MinGW's resource compiler: prefixed when cross-compiling
        let obj = out.join("icon.o");
        let ok = ["x86_64-w64-mingw32-windres", "windres"].iter().any(|tool| {
            std::process::Command::new(tool)
                .arg(&rc)
                .args(["-O", "coff", "-o"])
                .arg(&obj)
                .status()
                .is_ok_and(|s| s.success())
        });
        if ok {
            println!("cargo:rustc-link-arg-bins={}", obj.display());
        } else {
            println!("cargo:warning=windres not found: the .exe gets no icon");
        }
    } else {
        // MSVC: rc.exe from the Windows SDK
        let res = out.join("icon.res");
        let ok = std::process::Command::new("rc")
            .arg("/nologo")
            .arg("/fo")
            .arg(&res)
            .arg(&rc)
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            println!("cargo:rustc-link-arg-bins={}", res.display());
        } else {
            println!("cargo:warning=rc.exe not found: the .exe gets no icon");
        }
    }
}
