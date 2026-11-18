#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;

const USAGE: &str = "\
Microkit 8085 trainer emulator

usage: microkit-emu [options] [program.hex|.asm|.bin]

options:
  --rom PATH     monitor ROM to use (.bin, .hex or .asm)
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

fn run_gui(all: Vec<std::ffi::OsString>) -> eframe::Result {
    let mut opts = app::Options::default();
    let mut args = all.into_iter();
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--rom") => opts.rom = args.next().map(Into::into),
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
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Microkit 8085 emulator")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([820.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("Microkit 8085 emulator", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, opts)))))
}
