#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod asm_highlight;
mod i18n;
mod system_theme;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
const USAGE: &str = "\
MikroKit-Emu — Micro Design Mikrokit 8085 trainer emulator

usage: mikrokit-emu [options] [program.hex|.asm|.bin]

options:
  --rom ROM      monitor ROM: original (the default), basic
                 (monitor + Tiny BASIC), or a file (.bin, .hex, .asm)
  --terminal     start with J16 on (terminal monitor)
  --keypad       start with J16 off (keypad monitor)
  --debug        show the debug panel
  --renderer wgpu|glow
                 drawing method: Vulkan/Direct3D 12/Metal or OpenGL
                 (default: automatic, with a fallback to OpenGL)
  -h, --help     show this help

assembler (no window):
  mikrokit-emu --assemble SOURCE.asm [-o OUT.bin|OUT.hex] [--symbols]
    .bin output is the 8 KB ROM image 0000-1FFF (gaps filled with 00);
    .hex output contains every assembled byte.";

#[cfg(not(target_arch = "wasm32"))]
fn assemble_cli(args: &[std::ffi::OsString]) -> std::process::ExitCode {
    use mikrokit_emu::{asm, loader};
    let mut src = None;
    let mut out: Option<std::path::PathBuf> = None;
    let mut symbols = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.to_str() {
            Some("-o") => out = it.next().map(Into::into),
            Some("--symbols") => symbols = true,
            _ => src = Some(std::path::PathBuf::from(a)),
        }
    }
    let Some(src) = src else {
        eprintln!("--assemble needs a source file");
        return std::process::ExitCode::FAILURE;
    };
    let text = match std::fs::read_to_string(&src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", src.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    let res = match asm::assemble(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: {e}", src.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    println!("{}: {} bytes in {}", src.display(), res.image.bytes.len(), res.image.describe());
    if symbols {
        let mut v: Vec<_> = res.symbols.iter().collect();
        v.sort_by_key(|(k, v)| (**v, (*k).clone()));
        for (k, v) in v {
            println!("{v:04X} {k}");
        }
    }
    if let Some(out) = out {
        let is_hex = out.extension().is_some_and(|e| e.eq_ignore_ascii_case("hex"));
        let data = if is_hex {
            let mut s = String::new();
            for (a, b) in res.image.ranges() {
                let bytes: Vec<u8> = (a..=b).map(|x| res.image.bytes[&x]).collect();
                s.push_str(loader::write_ihex(a, &bytes).trim_end_matches(":00000001FF\r\n"));
            }
            s.push_str(":00000001FF\r\n");
            s.into_bytes()
        } else {
            res.image.to_rom()
        };
        if let Err(e) = std::fs::write(&out, data) {
            eprintln!("{}: {e}", out.display());
            return std::process::ExitCode::FAILURE;
        }
        println!("wrote {}", out.display());
    }
    std::process::ExitCode::SUCCESS
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    let all: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    // A Windows GUI program has no console; use the one it was started from
    // (if any) so --help and --assemble can print.
    #[cfg(windows)]
    if !all.is_empty() {
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS);
        }
    }
    if all.first().and_then(|a| a.to_str()) == Some("--assemble") {
        return assemble_cli(&all[1..]);
    }
    // `--child --renderer R …`: this process is the window (see supervise)
    if all.first().is_some_and(|a| a == "--child") {
        // a crash should end this process at once (no Windows Error
        // Reporting dialog or debugger), so the supervisor can react
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Diagnostics::Debug::{SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SetErrorMode};
            SetErrorMode(SEM_NOGPFAULTERRORBOX | SEM_FAILCRITICALERRORS);
        }
        let mut args: Vec<std::ffi::OsString> = all[1..].to_vec();
        let mut renderer = String::from("wgpu");
        if let Some(i) = args.iter().position(|a| a == "--renderer") {
            if let Some(r) = args.get(i + 1).and_then(|r| r.to_str()) {
                renderer = r.to_string();
            }
            args.drain(i..(i + 2).min(args.len()));
        }
        if std::env::var_os("MIKROKIT_SIMULATE_CRASH").is_some_and(|v| v == renderer.as_str()) {
            // test hook: crash like a broken graphics driver does
            unsafe { std::ptr::null_mut::<u8>().write_volatile(0) };
        }
        return run_child(args, &renderer);
    }
    if all.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return std::process::ExitCode::SUCCESS;
    }
    supervise(all)
}

#[cfg(not(target_arch = "wasm32"))]
const APP_NAME: &str = "MikroKit-Emu";
/// Earlier names of the app (newest first): their settings folders.
#[cfg(not(target_arch = "wasm32"))]
const OLD_APP_NAMES: &[&str] = &["MicroKit-Emu", "Microkit 8085 emulator"];

/// Carry the settings over from the folder used before a rename.
#[cfg(not(target_arch = "wasm32"))]
fn migrate_settings() {
    let Some(new) = eframe::storage_dir(APP_NAME) else { return };
    if new.exists() {
        return;
    }
    let Some(old) = OLD_APP_NAMES.iter().filter_map(|n| eframe::storage_dir(n)).find(|d| d.exists()) else {
        return;
    };
    if std::fs::create_dir_all(&new).is_ok() {
        if let Ok(dir) = std::fs::read_dir(&old) {
            for f in dir.flatten() {
                let _ = std::fs::copy(f.path(), new.join(f.file_name()));
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_gui(all: Vec<std::ffi::OsString>) -> eframe::Result {
    let mut opts = app::Options::default();
    let mut renderer: Option<String> = None;
    let mut args = all.into_iter();
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--rom") => {
                opts.rom = args.next().map(|r| match r.to_str() {
                    Some(n) if mikrokit_emu::roms::BUILTIN.iter().any(|(b, _, _)| *b == n) => {
                        app::RomChoice::Builtin(n.into())
                    }
                    _ => app::RomChoice::File(r.into()),
                })
            }
            Some("--terminal") => opts.terminal = Some(true),
            Some("--keypad") => opts.terminal = Some(false),
            Some("--debug") => opts.debug = true,
            Some("--renderer") => renderer = args.next().and_then(|r| r.into_string().ok()),
            Some("-h") | Some("--help") => {
                println!("{USAGE}");
                return Ok(());
            }
            _ => opts.program = Some(a.into()),
        }
    }
    migrate_settings();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")).expect("icon is a valid PNG"),
            )
            .with_app_id(APP_NAME)
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([820.0, 560.0]),
        renderer: if renderer.as_deref() == Some("glow") { eframe::Renderer::Glow } else { eframe::Renderer::Wgpu },
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, opts)))))
}

/// Set once the first frame has been drawn.
pub static FIRST_FRAME: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// What a child process prints on stdout once its first frame is drawn.
pub const READY_LINE: &str = "MIKROKIT-EMU-READY";

/// Called from the first frame: tell the supervisor the window works.
pub fn signal_ready() {
    use std::io::Write;
    if !FIRST_FRAME.swap(true, std::sync::atomic::Ordering::Relaxed) {
        let mut out = std::io::stdout();
        let _ = writeln!(out, "{READY_LINE}");
        let _ = out.flush();
    }
}

/// The window runs in a child process (`--child`) and this process watches
/// it. A graphics driver can crash the process outright (on Windows an
/// access violation, 0xC0000005) before any error handling runs, so only a
/// separate process can notice and try again with OpenGL. The renderer that
/// worked is remembered, so a PC with a broken Vulkan/Direct3D 12 driver
/// goes straight to OpenGL next time.
#[cfg(not(target_arch = "wasm32"))]
fn supervise(all: Vec<std::ffi::OsString>) -> std::process::ExitCode {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    // before the remembered renderer is read from the settings folder
    migrate_settings();

    // a --renderer chosen by the user: use only that one
    let mut user_args = Vec::new();
    let mut forced: Option<String> = None;
    let mut it = all.into_iter();
    while let Some(a) = it.next() {
        if a == "--renderer" {
            forced = it.next().and_then(|r| r.into_string().ok());
        } else {
            user_args.push(a);
        }
    }
    let remembered = remembered_renderer();
    let order: Vec<String> = match (&forced, remembered.as_deref()) {
        (Some(r), _) => vec![r.clone()],
        (None, Some("glow")) => vec!["glow".into()],
        _ => vec!["wgpu".into(), "glow".into()],
    };
    let Ok(exe) = std::env::current_exe() else {
        return run_child(user_args, forced.as_deref().unwrap_or("wgpu"));
    };

    let mut attempts: Vec<String> = Vec::new();
    for renderer in &order {
        let child = Command::new(&exe)
            .arg("--child")
            .arg("--renderer")
            .arg(renderer)
            .args(&user_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                attempts.push(format!("{}: could not start: {e}", renderer_name(renderer)));
                continue;
            }
        };
        // pass the child's error output through, keeping the end of it
        let stderr = child.stderr.take();
        let err_thread = std::thread::spawn(move || {
            // pass each line on as it comes; keep the end for an error report
            let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
            if let Some(e) = stderr {
                for line in BufReader::new(e).lines().map_while(Result::ok) {
                    eprintln!("{line}");
                    tail.push_back(line);
                    if tail.len() > 40 {
                        tail.pop_front();
                    }
                }
            }
            tail.into_iter().collect::<Vec<_>>().join("\n")
        });
        let mut ready = false;
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if line.trim() == READY_LINE {
                    ready = true;
                    // the window works: remember OpenGL if the default failed
                    if renderer == "glow" && forced.is_none() && !attempts.is_empty() {
                        remember_renderer(Some("glow"));
                    } else if renderer == "wgpu" {
                        remember_renderer(None);
                    }
                } else {
                    println!("{line}");
                }
            }
        }
        let status = child.wait();
        let stderr_text = err_thread.join().unwrap_or_default();
        let detail = last_lines(&stderr_text, 12);
        match status {
            Ok(st) if st.success() => return std::process::ExitCode::SUCCESS,
            Ok(st) if ready => {
                report_error(
                    "MikroKit-Emu stopped unexpectedly.",
                    &format!("{}\n\n{detail}", explain_status(&st)),
                    false,
                );
                return std::process::ExitCode::FAILURE;
            }
            Ok(st) => attempts.push(
                format!("{}: {}\n{detail}", renderer_name(renderer), explain_status(&st)).trim_end().to_string(),
            ),
            Err(e) => attempts.push(format!("{}: {e}", renderer_name(renderer))),
        }
    }
    report_error("MikroKit-Emu could not open its window.", &attempts.join("\n\n"), true);
    std::process::ExitCode::FAILURE
}

#[cfg(not(target_arch = "wasm32"))]
fn renderer_name(r: &str) -> &'static str {
    if r == "glow" { "OpenGL" } else { "Vulkan / Direct3D 12" }
}

#[cfg(not(target_arch = "wasm32"))]
fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// The exit status in words, with the Windows crash codes people meet.
#[cfg(not(target_arch = "wasm32"))]
fn explain_status(st: &std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = st.signal() {
            let name = match sig {
                11 => " (SIGSEGV: a crash, usually in the graphics driver)",
                6 => " (SIGABRT)",
                _ => "",
            };
            return format!("killed by signal {sig}{name}");
        }
    }
    match st.code() {
        Some(101) => "stopped with an error (exit code 101)".into(),
        Some(c) => {
            let hex = c as u32;
            let what = match hex {
                0xC0000005 => " (access violation: a crash, usually in the graphics driver)",
                0xC00000FD => " (stack overflow)",
                0xC0000135 => " (a DLL is missing)",
                0xC0000409 => " (the program was stopped by a security check)",
                _ => "",
            };
            if hex >= 0xC0000000 { format!("exit code 0x{hex:08X}{what}") } else { format!("exit code {c}{what}") }
        }
        None => "stopped".into(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn renderer_file() -> Option<std::path::PathBuf> {
    eframe::storage_dir(APP_NAME).map(|d| d.join("renderer"))
}

#[cfg(not(target_arch = "wasm32"))]
fn remembered_renderer() -> Option<String> {
    let s = std::fs::read_to_string(renderer_file()?).ok()?;
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

#[cfg(not(target_arch = "wasm32"))]
fn remember_renderer(r: Option<&str>) {
    let Some(f) = renderer_file() else { return };
    match r {
        Some(r) => {
            if let Some(d) = f.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(f, r);
        }
        None => {
            let _ = std::fs::remove_file(f);
        }
    }
}

/// The child: open the window with the given renderer. Errors go to stderr
/// (the supervisor shows them); a panic exits with code 101.
#[cfg(not(target_arch = "wasm32"))]
fn run_child(args: Vec<std::ffi::OsString>, renderer: &str) -> std::process::ExitCode {
    let mut all: Vec<std::ffi::OsString> = vec!["--renderer".into(), renderer.into()];
    all.extend(args);
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_gui(all))) {
        Ok(Ok(())) => std::process::ExitCode::SUCCESS,
        Ok(Err(e)) => {
            eprintln!("{e}");
            std::process::ExitCode::from(101)
        }
        Err(_) => std::process::ExitCode::from(101), // the panic message is already on stderr
    }
}

/// Tell the user why the program stopped: a message box on Windows (it has
/// no console), stderr elsewhere, and a log file next to the settings.
#[cfg(not(target_arch = "wasm32"))]
fn report_error(what: &str, detail: &str, no_window: bool) {
    let mut text = format!("{what}\n\n{detail}");
    if no_window {
        text.push_str(
            "\n\nUpdating the graphics driver usually helps; in a virtual machine, enable 3D acceleration.",
        );
    }
    if let Some(dir) = eframe::storage_dir(APP_NAME) {
        let log = dir.join("error.log");
        if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&log, &text).is_ok() {
            text.push_str(&format!("\n\nThis report was saved to {}", log.display()));
        }
    }
    eprintln!("{text}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (t, c) = (wide(&text), wide(APP_NAME));
        unsafe {
            MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), MB_OK | MB_ICONERROR);
        }
    }
}
