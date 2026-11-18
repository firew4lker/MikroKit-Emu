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
  -h, --help     show this help";

fn main() -> eframe::Result {
    let mut opts = app::Options::default();
    let mut args = std::env::args_os().skip(1);
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
