// Windows gives a program's main thread a 1 MB stack (Linux: 8 MB), and a
// stack overflow there ends the program silently. Give it 8 MB too.
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        if target.ends_with("-gnu") || target.ends_with("-gnullvm") {
            println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608");
        } else {
            println!("cargo:rustc-link-arg-bins=/STACK:8388608");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
