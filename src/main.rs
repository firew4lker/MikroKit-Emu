#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod i18n;
mod system_theme;

const USAGE: &str = "\
MicroKit-Emu — Micro Design Microkit 8085 trainer emulator

usage: microkit-emu [options] [program.hex|.asm|.bin]

options:
  --rom ROM      monitor ROM: original (the default), basic
                 (monitor + Tiny BASIC), or a file (.bin, .hex, .asm)
  --terminal     start with J16 on (terminal monitor)
  --keypad       start with J16 off (keypad monitor)
  --debug        show the debug panel
  -h, --help     show this help

assembler (no window):
  microkit-emu --assemble SOURCE.asm [-o OUT.bin|OUT.hex] [--symbols]
    .bin output is the 8 KB ROM image 0000-1FFF (gaps filled with 00);
    .hex output contains every assembled byte.";

fn assemble_cli(args: &[std::ffi::OsString]) -> std::process::ExitCode {
    use microkit_emu::{asm, loader};
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
    match run_gui(all) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}

const APP_NAME: &str = "MicroKit-Emu";
/// What the app was called before it was named; its settings folder.
const OLD_APP_NAME: &str = "Microkit 8085 emulator";

/// Carry the settings over from the folder used before the rename.
fn migrate_settings() {
    let (Some(new), Some(old)) = (eframe::storage_dir(APP_NAME), eframe::storage_dir(OLD_APP_NAME)) else {
        return;
    };
    if new.exists() || !old.exists() {
        return;
    }
    if std::fs::create_dir_all(&new).is_ok() {
        if let Ok(dir) = std::fs::read_dir(&old) {
            for f in dir.flatten() {
                let _ = std::fs::copy(f.path(), new.join(f.file_name()));
            }
        }
    }
}

fn run_gui(all: Vec<std::ffi::OsString>) -> eframe::Result {
    let mut opts = app::Options::default();
    let mut args = all.into_iter();
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--rom") => {
                opts.rom = args.next().map(|r| match r.to_str() {
                    Some(n) if microkit_emu::roms::BUILTIN.iter().any(|(b, _, _)| *b == n) => {
                        app::RomChoice::Builtin(n.into())
                    }
                    _ => app::RomChoice::File(r.into()),
                })
            }
            Some("--terminal") => opts.terminal = Some(true),
            Some("--keypad") => opts.terminal = Some(false),
            Some("--debug") => opts.debug = true,
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
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, opts)))))
}
