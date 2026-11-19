//! The egui front end: board (display + keypad), terminal, menus, dialogs,
//! debug panel and status bar.

use eframe::egui::{
    self, Align2, Color32, Event, FontId, Key as EKey, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind,
    Vec2, pos2, vec2,
};
use microkit_emu::asm;
use microkit_emu::disasm::disasm;
use microkit_emu::loader::{self, Image, parse_addr};
use microkit_emu::machine::{CPU_HZ, J17_CLOCKS, Jumpers, Key, Machine, monitor};
use crate::i18n::{tr, trf, Language};
use microkit_emu::roms;
use microkit_emu::segments;
use microkit_emu::tape::{Tape, TapeState};
use microkit_emu::terminal::Terminal;
use std::path::{Path, PathBuf};
use std::time::Instant;

const PCB: Color32 = Color32::from_rgb(0x12, 0x3b, 0x2a);
const PCB_TEXT: Color32 = Color32::from_rgb(0x9f, 0xe1, 0xcb);
const LED_ON: Color32 = Color32::from_rgb(0xff, 0x3b, 0x30);
const LED_OFF: Color32 = Color32::from_rgb(0x3a, 0x1c, 0x1c);
const KEY_FACE: Color32 = Color32::from_rgb(0xec, 0xe9, 0xe0);
const KEY_TEXT: Color32 = Color32::from_rgb(0x1d, 0x1d, 0x1b);
const TERM_BG: Color32 = Color32::from_rgb(0x0e, 0x11, 0x10);
const TERM_FG: Color32 = Color32::from_rgb(0x9f, 0xe1, 0xcb);

const KEYPAD: [[Key; 6]; 4] = [
    [Key::Reset, Key::Vi, Key::Hex(0xC), Key::Hex(0xD), Key::Hex(0xE), Key::Hex(0xF)],
    [Key::Step, Key::Go, Key::Hex(8), Key::Hex(9), Key::Hex(0xA), Key::Hex(0xB)],
    [Key::Mem, Key::Reg, Key::Hex(4), Key::Hex(5), Key::Hex(6), Key::Hex(7)],
    [Key::Next, Key::Ex, Key::Hex(0), Key::Hex(1), Key::Hex(2), Key::Hex(3)],
];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Settings {
    rom: RomChoice,
    /// The last external ROM file, kept so the menu can switch back to it.
    rom_file: Option<PathBuf>,
    last_program: Option<PathBuf>,
    last_bin_addr: String,
    jumpers: Jumpers,
    turbo: bool,
    force_upper: bool,
    set_pc_after_load: bool,
    show_debug: bool,
    show_tape: bool,
    show_ports: bool,
    theme: Theme,
    language: Language,
    lcd: LcdChoice,
    show_lcd: bool,
    lcd_blue: bool,
}

/// Which LCD module is plugged into port B.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
enum LcdChoice {
    #[default]
    None,
    L16x2,
    L20x4,
}

impl LcdChoice {
    fn size(self) -> Option<microkit_emu::lcd::LcdSize> {
        match self {
            LcdChoice::None => None,
            LcdChoice::L16x2 => Some(microkit_emu::lcd::LCD_16X2),
            LcdChoice::L20x4 => Some(microkit_emu::lcd::LCD_20X4),
        }
    }
}

/// Which monitor ROM is in the socket.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RomChoice {
    /// One of `roms::BUILTIN`, by its short name.
    Builtin(String),
    /// A file the user picked (.bin, .hex or .asm).
    File(PathBuf),
}

impl Default for RomChoice {
    fn default() -> Self {
        RomChoice::Builtin("original".into())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

impl Theme {
    fn preference(self) -> egui::ThemePreference {
        match self {
            Theme::System => egui::ThemePreference::System,
            Theme::Dark => egui::ThemePreference::Dark,
            Theme::Light => egui::ThemePreference::Light,
        }
    }
}

#[derive(Clone, Copy)]
enum TermAction {
    Copy,
    CopyAll,
    Paste,
    CtrlC,
    Clear,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            rom: RomChoice::Builtin("original".into()),
            rom_file: None,
            last_program: None,
            last_bin_addr: "2000".into(),
            jumpers: Jumpers::default(),
            turbo: false,
            force_upper: true,
            set_pc_after_load: true,
            show_debug: false,
            show_tape: false,
            show_ports: false,
            theme: Theme::System,
            language: Language::English,
            lcd: LcdChoice::None,
            show_lcd: true,
            lcd_blue: false,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Target {
    Keypad,
    Terminal,
}

struct BinDialog {
    path: PathBuf,
    data: Vec<u8>,
    addr: String,
}

enum Confirm {
    BlankTape,
    LoadWav(PathBuf),
}

struct SaveDialog {
    start: String,
    end: String,
}

pub struct App {
    m: Machine,
    term: Terminal,
    settings: Settings,
    rom_loaded: bool,
    running: bool,
    last_tick: Option<Instant>,
    speed_window: (Instant, u64),
    measured_mhz: f64,
    target: Target,
    key_flash: Vec<(Key, f64)>,
    status: String,
    message: Option<(String, String)>,
    bin_dialog: Option<BinDialog>,
    save_dialog: Option<SaveDialog>,
    show_help: bool,
    confirm: Option<Confirm>,
    /// The terminal text the user last selected (egui reports it when the
    /// selection settles); used by the right-click tr("Copy").
    selected_text: std::sync::Arc<std::sync::Mutex<Selection>>,
    /// Desktop light/dark preference (winit doesn't report it on Linux).
    system_theme: crate::system_theme::SystemTheme,
    /// CPU state shown in the status bar / debug panel. While running it is
    /// refreshed a few times a second; at 60 Hz it is just a blur.
    shown_cpu: (microkit_emu::cpu::Cpu, f64),
    started: Instant,
    breakpoints: Vec<u16>,
    bp_text: String,
    mem_addr: String,
    last_loaded: Option<String>,
}

/// Command-line options.
#[derive(Default)]
pub struct Options {
    pub rom: Option<RomChoice>,
    pub program: Option<PathBuf>,
    pub terminal: Option<bool>,
    pub debug: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Options) -> Self {
        let mut settings: Settings =
            cc.storage.and_then(|s| eframe::get_value(s, "settings")).unwrap_or_default();
        if let Some(t) = opts.terminal {
            settings.jumpers.j16_terminal = t;
        }
        if opts.debug {
            settings.show_debug = true;
        }
        crate::i18n::set(settings.language);
        let mut app = App {
            m: Machine::new(),
            term: Terminal::default(),
            target: if settings.jumpers.j16_terminal { Target::Terminal } else { Target::Keypad },
            settings,
            rom_loaded: false,
            running: true,
            last_tick: None,
            speed_window: (Instant::now(), 0),
            measured_mhz: 0.0,
            key_flash: Vec::new(),
            status: String::new(),
            message: None,
            bin_dialog: None,
            save_dialog: None,
            show_help: false,
            confirm: None,
            selected_text: Default::default(),
            system_theme: crate::system_theme::SystemTheme::watch(cc.egui_ctx.clone()),
            shown_cpu: (microkit_emu::cpu::Cpu::new(), f64::NEG_INFINITY),
            started: Instant::now(),
            breakpoints: Vec::new(),
            bp_text: String::new(),
            mem_addr: "2000".into(),
            last_loaded: None,
        };
        cc.egui_ctx.set_visuals_of(egui::Theme::Light, light_visuals());
        cc.egui_ctx.set_theme(app.settings.theme.preference());
        cc.egui_ctx.add_plugin(SelectionWatcher(app.selected_text.clone()));
        app.m.board.jumpers = app.settings.jumpers.clone();
        app.m.apply_jumpers();
        app.m.board.lcd = app.settings.lcd.size().map(microkit_emu::lcd::Hd44780::new);
        let rom = opts.rom.clone().unwrap_or_else(|| app.settings.rom.clone());
        app.select_rom(rom);
        if !app.rom_loaded {
            // a saved external file that has gone away: fall back to the original
            app.select_rom(RomChoice::default());
        }
        if let Some(p) = opts.program {
            // let the monitor finish its cold start first, so the GO address sticks
            app.m.run_for(CPU_HZ as u64 / 10, &[]);
            app.load_program(&p);
        }
        app
    }

    // ----- loading ----------------------------------------------------------

    fn error(&mut self, title: &str, text: String) {
        self.status = format!("{title}: {}", text.lines().next().unwrap_or(""));
        self.message = Some((title.to_string(), text));
    }

    fn select_rom(&mut self, choice: RomChoice) {
        match &choice {
            RomChoice::Builtin(name) => {
                let Some((_, label, image)) = roms::BUILTIN.iter().find(|(n, _, _)| n == name) else {
                    return self.error(tr("Unknown built-in ROM"), name.clone());
                };
                self.install_rom(image, choice.clone());
                self.status = trf!("Monitor ROM: {} (built in)", tr(label));
            }
            RomChoice::File(p) => {
                let p = p.clone();
                self.load_rom(&p);
            }
        }
    }

    fn install_rom(&mut self, image: &[u8], choice: RomChoice) {
        self.m.load_rom(image);
        self.m.reset();
        self.rom_loaded = true;
        self.running = true;
        if let RomChoice::File(p) = &choice {
            self.settings.rom_file = Some(p.clone());
        }
        self.settings.rom = choice;
    }

    fn load_rom(&mut self, path: &Path) {
        let result: Result<Vec<u8>, String> = (|| {
            let ext = ext_of(path);
            if ext == "asm" || ext == "a85" || ext == "s" {
                let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                let out = asm::assemble(&src).map_err(|e| e.to_string())?;
                Ok(out.image.to_rom())
            } else if ext == "hex" || ext == "ihx" {
                let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                Ok(loader::parse_ihex(&src)?.to_rom())
            } else {
                let data = std::fs::read(path).map_err(|e| e.to_string())?;
                if data.len() > 0x2000 {
                    return Err(trf!("file is {} bytes; the ROM socket holds 8 KB", data.len()));
                }
                Ok(data)
            }
        })();
        match result {
            Ok(rom) => {
                self.install_rom(&rom, RomChoice::File(path.to_path_buf()));
                self.status = trf!("Monitor ROM loaded from {}", file_name(path));
            }
            Err(e) => self.error(tr("Couldn't load the ROM"), e),
        }
    }

    /// File → Monitor ROM: the built-in ROMs, the last external file, and
    /// tr("Load from file…"). Choosing one puts it in the socket and resets.
    fn rom_menu(&mut self, ui: &mut egui::Ui) {
        let mut pick = None;
        for (name, label, _) in roms::BUILTIN {
            let this = RomChoice::Builtin(name.into());
            if ui.radio(self.settings.rom == this, tr(label)).clicked() {
                pick = Some(this);
            }
        }
        if let Some(f) = self.settings.rom_file.clone() {
            let this = RomChoice::File(f.clone());
            if ui
                .radio(self.settings.rom == this, trf!("File: {}", file_name(&f)))
                .on_hover_text(f.display().to_string())
                .clicked()
            {
                pick = Some(this);
            }
        }
        ui.separator();
        if ui.button(tr("Load from file…")).clicked() {
            ui.close();
            if let Some(p) = rfd::FileDialog::new()
                .set_title(tr("Load monitor ROM"))
                .add_filter(tr("ROM image or source"), &["bin", "rom", "hex", "asm", "a85"])
                .add_filter(tr("All files"), &["*"])
                .pick_file()
            {
                self.load_rom(&p);
            }
        }
        if let Some(c) = pick {
            ui.close();
            self.select_rom(c);
        }
    }

    fn load_program(&mut self, path: &Path) {
        let ext = ext_of(path);
        let img = match ext.as_str() {
            "hex" | "ihx" => std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|t| loader::parse_ihex(&t)),
            "asm" | "a85" | "s" | "src" => std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|t| asm::assemble(&t).map(|o| o.image).map_err(|e| e.to_string())),
            _ => match std::fs::read(path) {
                Ok(data) => {
                    self.bin_dialog = Some(BinDialog {
                        path: path.to_path_buf(),
                        data,
                        addr: self.settings.last_bin_addr.clone(),
                    });
                    return;
                }
                Err(e) => Err(e.to_string()),
            },
        };
        match img {
            Ok(img) => self.apply_image(path, img),
            Err(e) => self.error(&trf!("Couldn't load {}", file_name(path)), e),
        }
    }

    fn apply_image(&mut self, path: &Path, img: Image) {
        if img.is_empty() {
            return self.error(tr("Nothing to load"), trf!("{} contains no bytes.", file_name(path)));
        }
        let rom: Vec<_> = img.bytes.keys().filter(|&&a| a < 0x2000).collect();
        if !rom.is_empty() {
            return self.error(
                tr("Program overlaps the ROM"),
                trf!(
                    "Address {} is in the EPROM (0000–1FFF). Load user programs into RAM (2000–3FFF), or use File → Monitor ROM → Load from file… to replace the monitor.",
                    format!("{:04X}", rom[0])
                ),
            );
        }
        let mut warn = String::new();
        for (&a, &b) in &img.bytes {
            if !self.m.board.poke(a, b) {
                return self.error(
                    tr("Program doesn't fit"),
                    trf!("There's no memory at {}. RAM is 2000–3FFF (and 4000–40FF in the 8155).", format!("{a:04X}")),
                );
            }
            if (0x4000..=0x40FF).contains(&a) && warn.is_empty() {
                warn = tr(" — warning: it writes into the monitor's area 4000–40FF").into();
            }
        }
        let start = img.start().unwrap();
        let mut msg = trf!("Loaded {} → {}", file_name(path), img.describe());
        if self.settings.set_pc_after_load && monitor::USER_RAM.contains(&start) {
            let [lo, hi] = start.to_le_bytes();
            self.m.board.poke(monitor::PSAV, lo);
            self.m.board.poke(monitor::PSAV + 1, hi);
            msg.push_str(&trf!(", GO address {}", format!("{start:04X}")));
        }
        msg.push_str(&warn);
        self.status = msg;
        self.settings.last_program = Some(path.to_path_buf());
        self.last_loaded = Some(img.describe());
    }

    fn send_file(&mut self, port: u8) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(trf!("Send file to DB9-{}", port))
            .add_filter(tr("Intel HEX, 8085 source or BASIC text"), &["hex", "ihx", "asm", "a85", "bas", "txt"])
            .add_filter(tr("All files"), &["*"])
            .pick_file()
        else {
            return;
        };
        let text = match ext_of(&path).as_str() {
            "asm" | "a85" => match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| {
                asm::assemble(&t).map_err(|e| e.to_string())
            }) {
                Ok(o) => image_to_hex(&o.image),
                Err(e) => return self.error(tr("Couldn't assemble"), e),
            },
            _ => match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => return self.error(tr("Couldn't read the file"), e.to_string()),
            },
        };
        let typed = matches!(ext_of(&path).as_str(), "bas" | "txt");
        if typed && port == 1 {
            // text for BASIC (or the monitor): type it at a person's pace so
            // every line gets stored before the next one arrives
            let t = text.replace("\r\n", "\n").replace('\n', "\r");
            let t = if self.settings.force_upper { t.to_ascii_uppercase() } else { t };
            self.send_terminal(t.as_bytes());
            self.status = trf!("Typing {} into the terminal…", file_name(&path));
            return;
        }
        let bytes: Vec<u8> = text.replace("\r\n", "\n").replace('\n', "\r\n").into_bytes();
        if port == 1 {
            self.m.board.uart.receive(&bytes);
        } else {
            if !self.m.board.jumpers.j18_db9_2 {
                self.status = tr("Note: J18 is set to tape, so DB9-2 isn't connected to SID.").into();
            }
            self.m.board.db9_2.send(&bytes);
        }
        self.status = trf!("Sending {} ({} bytes) to DB9-{}…", file_name(&path), bytes.len(), port);
    }

    fn save_memory(&mut self, start: u16, end: u16) {
        if end < start {
            return self.error(tr("Wrong range"), tr("The end address is before the start address.").into());
        }
        let data: Vec<u8> = (start..=end).map(|a| self.m.board.peek(a)).collect();
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr("Save memory"))
            .add_filter("Intel HEX", &["hex"])
            .add_filter(tr("Binary"), &["bin"])
            .set_file_name(format!("mem_{start:04X}_{end:04X}.hex"))
            .save_file()
        else {
            return;
        };
        let res = if ext_of(&path) == "bin" {
            std::fs::write(&path, &data)
        } else {
            std::fs::write(&path, loader::write_ihex(start, &data))
        };
        match res {
            Ok(()) => self.status = trf!("Saved {}–{} to {}", format!("{start:04X}"), format!("{end:04X}"), file_name(&path)),
            Err(e) => self.error(tr("Couldn't save"), e.to_string()),
        }
    }

    // ----- emulation ---------------------------------------------------------

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = self.last_tick.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0).min(0.1);
        self.last_tick = Some(now);
        if self.running && self.rom_loaded {
            let hit = if self.settings.turbo {
                let deadline = now + std::time::Duration::from_millis(14);
                let mut hit = false;
                while Instant::now() < deadline && !hit {
                    hit = self.m.run_for(50_000, &self.breakpoints);
                }
                hit
            } else {
                self.m.run_for((dt * CPU_HZ as f64) as u64, &self.breakpoints)
            };
            if hit {
                self.running = false;
                self.status = trf!("Stopped at breakpoint {}", format!("{:04X}", self.m.cpu.pc));
            }
        }
        self.m.periodic();
        while let Some(b) = self.m.board.uart.tx_out.pop_front() {
            self.term.feed(b);
        }
        let (t0, c0) = self.speed_window;
        let el = now.duration_since(t0).as_secs_f64();
        if el >= 0.5 {
            self.measured_mhz = (self.m.cpu.cycles - c0) as f64 / el / 1e6;
            self.speed_window = (now, self.m.cpu.cycles);
        }
    }

    fn press(&mut self, key: Key, time: f64) {
        if std::env::var_os("MICROKIT_DEBUG").is_some() {
            eprintln!("key {:?} at {:.3}", key, time);
        }
        self.m.press(key);
        if key == Key::Reset {
            self.running = self.rom_loaded;
        }
        self.key_flash.retain(|(k, _)| *k != key);
        self.key_flash.push((key, time));
    }

    fn set_jumpers(&mut self, j: Jumpers) {
        let terminal_changed = j.j16_terminal != self.settings.jumpers.j16_terminal;
        self.settings.jumpers = j.clone();
        self.m.board.jumpers = j;
        self.m.apply_jumpers();
        if terminal_changed {
            self.target = if self.settings.jumpers.j16_terminal { Target::Terminal } else { Target::Keypad };
            self.status = tr("J16 changed — press RS (reset) for the monitor to switch.").into();
        }
    }

    fn send_terminal(&mut self, bytes: &[u8]) {
        self.m.board.uart.type_chars(bytes);
    }

    // ----- keyboard routing ----------------------------------------------------

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.memory(|m| m.focused().is_some()) || self.bin_dialog.is_some() || self.save_dialog.is_some() {
            return;
        }
        let (events, time) = ctx.input(|i| (i.events.clone(), i.time));
        for ev in events {
            if std::env::var_os("MICROKIT_DEBUG").is_some() {
                eprintln!("event {ev:?}");
            }
            match (self.target, ev) {
                (_, Event::Key { key: EKey::F9, pressed: true, .. }) => self.press(Key::Reset, time),
                (_, Event::Key { key: EKey::F10, pressed: true, .. }) => self.press(Key::Vi, time),
                (Target::Keypad, Event::Text(t)) => {
                    for ch in t.chars() {
                        let k = match ch.to_ascii_uppercase() {
                            c @ ('0'..='9' | 'A'..='F') => Some(Key::Hex(c.to_digit(16).unwrap() as u8)),
                            'N' | ' ' | ',' => Some(Key::Next),
                            '.' => Some(Key::Ex),
                            'G' => Some(Key::Go),
                            'M' => Some(Key::Mem),
                            'R' => Some(Key::Reg),
                            'S' => Some(Key::Step),
                            _ => None,
                        };
                        if let Some(k) = k {
                            self.press(k, time);
                        }
                    }
                }
                (Target::Keypad, Event::Key { key: EKey::Enter, pressed: true, .. }) => self.press(Key::Ex, time),
                (Target::Terminal, Event::Text(t)) => {
                    let t = if self.settings.force_upper { t.to_ascii_uppercase() } else { t };
                    self.send_terminal(t.as_bytes());
                }
                (Target::Terminal, Event::Paste(t)) => {
                    let t = if self.settings.force_upper { t.to_ascii_uppercase() } else { t };
                    let t = t.replace("\r\n", "\r").replace('\n', "\r");
                    self.send_terminal(t.as_bytes());
                }
                (Target::Terminal, Event::Copy) => {
                    // with text selected, Ctrl+C copies it (egui does that);
                    // otherwise it is ^C for the kit
                    if !has_text_selection(ctx) {
                        self.send_terminal(&[0x03]);
                    }
                }
                (Target::Terminal, Event::Key { key, pressed: true, modifiers, .. }) => {
                    let b = match key {
                        EKey::Enter => Some(0x0D),
                        EKey::Backspace => Some(0x08),
                        EKey::Escape => Some(0x1B),
                        EKey::Tab => Some(0x09),
                        EKey::C if modifiers.ctrl => Some(0x03),
                        EKey::S if modifiers.ctrl => Some(0x13),
                        EKey::Q if modifiers.ctrl => Some(0x11),
                        _ => None,
                    };
                    if let Some(b) = b {
                        self.send_terminal(&[b]);
                    }
                }
                _ => {}
            }
        }
    }

    // ----- UI pieces -------------------------------------------------------------

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(tr("File"), |ui| {
                if ui.button(tr("Load program…")).clicked() {
                    ui.close();
                    if let Some(p) = pick_program() {
                        self.load_program(&p);
                    }
                }
                let last = self.settings.last_program.clone();
                if ui.add_enabled(last.is_some(), egui::Button::new(tr("Reload last program"))).clicked() {
                    ui.close();
                    if let Some(p) = last {
                        self.load_program(&p);
                    }
                }
                if ui.button(tr("Save memory range…")).clicked() {
                    ui.close();
                    self.save_dialog = Some(SaveDialog { start: "2000".into(), end: "20FF".into() });
                }
                ui.separator();
                ui.menu_button(tr("Monitor ROM"), |ui| self.rom_menu(ui));
                ui.separator();
                if ui.button(tr("Quit")).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(tr("Machine"), |ui| {
                let label = if self.running { tr("Pause") } else { tr("Run") };
                if ui.button(label).clicked() {
                    self.running = !self.running;
                    ui.close();
                }
                if ui.button(tr("Reset (RS)   F9")).clicked() {
                    let t = ui.input(|i| i.time);
                    self.press(Key::Reset, t);
                    ui.close();
                }
                if ui.button(tr("Vector interrupt (VI)   F10")).clicked() {
                    let t = ui.input(|i| i.time);
                    self.press(Key::Vi, t);
                    ui.close();
                }
                ui.separator();
                ui.checkbox(&mut self.settings.turbo, tr("Turbo speed"));
                ui.separator();
                let mut j = self.settings.jumpers.clone();
                jumper_controls(ui, &mut j);
                if j != self.settings.jumpers {
                    self.set_jumpers(j);
                }
            });
            ui.menu_button(tr("Serial"), |ui| {
                if ui.button(tr("Send HEX file to DB9-1 (8251)…")).clicked() {
                    ui.close();
                    self.send_file(1);
                }
                if ui.button(tr("Send HEX file to DB9-2 (SID)…")).clicked() {
                    ui.close();
                    self.send_file(2);
                }
                ui.separator();
                ui.checkbox(&mut self.settings.force_upper, tr("Type in capitals (the monitor needs them)"));
                if ui.button(tr("Clear terminal")).clicked() {
                    self.term.clear();
                    ui.close();
                }
                if ui.button(tr("Cancel pending input")).clicked() {
                    self.m.board.uart.clear_rx();
                    self.m.board.db9_2.clear();
                    ui.close();
                }
            });
            ui.menu_button(tr("Tape"), |ui| {
                ui.checkbox(&mut self.settings.show_tape, tr("Show tape recorder"));
                if ui.button(tr("Load WAV…")).clicked() {
                    ui.close();
                    self.pick_wav();
                }
                if ui.add_enabled(!self.m.board.tape.is_empty(), egui::Button::new(tr("Save WAV…"))).clicked() {
                    ui.close();
                    self.save_wav();
                }
            });
            ui.menu_button(tr("LCD"), |ui| self.lcd_menu(ui));
            ui.menu_button(tr("Debug"), |ui| {
                ui.checkbox(&mut self.settings.show_ports, tr("Show I/O ports"));
                ui.checkbox(&mut self.settings.show_debug, tr("Show debug panel"));
                ui.checkbox(&mut self.settings.set_pc_after_load, tr("Set GO address after loading a program"));
            });
            ui.menu_button(tr("View"), |ui| {
                ui.label(RichText::new(tr("Theme")).weak());
                let before = self.settings.theme;
                ui.radio_value(&mut self.settings.theme, Theme::System, tr("Follow the system"));
                ui.radio_value(&mut self.settings.theme, Theme::Dark, tr("Dark (light text on dark)"));
                ui.radio_value(&mut self.settings.theme, Theme::Light, tr("Light (dark text on light)"));
                if self.settings.theme != before {
                    ui.ctx().set_theme(self.settings.theme.preference());
                }
                ui.separator();
                ui.checkbox(&mut self.settings.show_tape, tr("Tape recorder"));
                ui.checkbox(&mut self.settings.show_ports, tr("I/O ports"));
                ui.checkbox(&mut self.settings.show_debug, tr("Debug panel"));
            });
            ui.menu_button(tr("Language"), |ui| {
                let before = self.settings.language;
                // each language is named in itself
                ui.radio_value(&mut self.settings.language, Language::English, "English");
                ui.radio_value(&mut self.settings.language, Language::Greek, "Ελληνικά");
                if self.settings.language != before {
                    crate::i18n::set(self.settings.language);
                    ui.close();
                    self.status = tr("Language: English").into();
                }
            });
            ui.menu_button(tr("Help"), |ui| {
                if ui.button(tr("Keys and usage")).clicked() {
                    self.show_help = true;
                    ui.close();
                }
                ui.separator();
                ui.label(format!("MicroKit-Emu {}", env!("CARGO_PKG_VERSION")));
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let time = ui.input(|i| i.time);
            let (icon, tip) = if self.running { (tr("⏸ Pause"), tr("Pause the CPU")) } else { (tr("▶ Run"), tr("Resume the CPU")) };
            if ui.button(icon).on_hover_text(tip).clicked() {
                self.running = !self.running;
            }
            if ui.button(tr("⟲ Reset (RS)")).on_hover_text(tr("Same as the RS key (F9)")).clicked() {
                self.press(Key::Reset, time);
            }
            if ui.button(tr("📂 Load program")).on_hover_text(tr(".hex, .bin or .asm into RAM")).clicked() {
                if let Some(p) = pick_program() {
                    self.load_program(&p);
                }
            }
            let last = self.settings.last_program.clone();
            if ui
                .add_enabled(last.is_some(), egui::Button::new(tr("↻ Reload")))
                .on_hover_text(last.as_deref().map(file_name).unwrap_or_default())
                .clicked()
            {
                if let Some(p) = last {
                    self.load_program(&p);
                }
            }
            ui.separator();
            ui.label(tr("Speed"));
            egui::ComboBox::from_id_salt("speed")
                .selected_text(if self.settings.turbo { "Turbo" } else { tr("3.072 MHz (real)") })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.turbo, false, tr("3.072 MHz (real)"));
                    ui.selectable_value(&mut self.settings.turbo, true, "Turbo");
                });
            ui.label(RichText::new(format!("{:.2} MHz", self.measured_mhz)).weak());
            ui.separator();
            ui.toggle_value(&mut self.settings.show_tape, tr("📼 Tape"));
            ui.toggle_value(&mut self.settings.show_ports, tr("🔌 I/O ports"));
            ui.checkbox(&mut self.settings.show_debug, tr("Debug"));
        });
    }

    fn board(&mut self, ui: &mut egui::Ui) {
        let time = ui.input(|i| i.time);
        let focused = self.target == Target::Keypad;
        let frame = egui::Frame::new()
            .fill(PCB)
            .corner_radius(8)
            .inner_margin(egui::Margin::same(12))
            .stroke(if focused { Stroke::new(2.0, PCB_TEXT) } else { Stroke::NONE });
        let resp = frame
            .show(ui, |ui| {
                ui.set_width(330.0);
                // display
                let (rect, _) = ui.allocate_exact_size(vec2(330.0, 74.0), Sense::hover());
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 6.0, Color32::from_rgb(0x1a, 0x0f, 0x0f));
                let digits = self.m.display();
                let dw = 40.0;
                let gap = 6.0;
                let total = 6.0 * dw + 5.0 * gap + 18.0;
                let mut x = rect.center().x - total / 2.0;
                for (i, &raw) in digits.iter().enumerate() {
                    if i == 4 {
                        x += 18.0;
                    }
                    let r = Rect::from_min_size(pos2(x, rect.top() + 8.0), vec2(dw, 58.0));
                    draw_digit(&p, r, segments::decode(raw));
                    x += dw + gap;
                }
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr("address")).color(PCB_TEXT).small());
                    ui.add_space(190.0);
                    ui.label(RichText::new(tr("data")).color(PCB_TEXT).small());
                });
                ui.add_space(6.0);
                // keypad
                let kw = 50.0;
                let kh = 40.0;
                let kgap = (330.0 - 6.0 * kw) / 5.0;
                for row in KEYPAD {
                    let (rect, _) = ui.allocate_exact_size(vec2(330.0, kh), Sense::hover());
                    for (i, key) in row.iter().enumerate() {
                        let r = Rect::from_min_size(pos2(rect.left() + i as f32 * (kw + kgap), rect.top()), vec2(kw, kh));
                        let id = ui.scope_id().with(("key", key.label()));
                        let resp = ui.interact(r, id, Sense::click());
                        let flash = self
                            .key_flash
                            .iter()
                            .any(|(k, t)| k == key && time - t < 0.12);
                        let down = resp.is_pointer_button_down_on() || flash;
                        let face = match key {
                            Key::Reset => Color32::from_rgb(0xf5, 0xc4, 0xb3),
                            _ if resp.hovered() => Color32::from_rgb(0xff, 0xfd, 0xf5),
                            _ => KEY_FACE,
                        };
                        let r2 = if down { r.shrink(2.0) } else { r };
                        let p = ui.painter();
                        p.rect_filled(r2.translate(vec2(0.0, 2.0)), 5.0, Color32::from_rgb(0x08, 0x20, 0x16));
                        p.rect_filled(r2, 5.0, if down { Color32::from_rgb(0xc8, 0xc4, 0xb8) } else { face });
                        p.rect_stroke(r2, 5.0, Stroke::new(1.0, Color32::from_rgb(0x9b, 0x98, 0x8f)), StrokeKind::Inside);
                        p.text(r2.center(), Align2::CENTER_CENTER, key.label(), FontId::proportional(17.0), KEY_TEXT);
                        if resp.clicked() {
                            self.press(*key, time);
                            self.target = Target::Keypad;
                        }
                        resp.on_hover_text(key_hint(*key));
                    }
                    ui.add_space(kgap);
                }
                ui.label(
                    RichText::new(tr("MICROKIT V2.0 · click here, then type: 0-F, M R G S N . (Enter = EX)"))
                        .color(PCB_TEXT)
                        .small(),
                );
            })
            .response;
        if clicked_inside(ui, resp.rect) {
            self.target = Target::Keypad;
        }
    }

    fn terminal(&mut self, ui: &mut egui::Ui) {
        let focused = self.target == Target::Terminal;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("Clear")).clicked() {
                    self.term.clear();
                }
                if ui
                    .button(tr("Send HEX → DB9-2"))
                    .on_hover_text(tr("Software serial on SID: the default port of GO 1600 and L"))
                    .clicked()
                {
                    self.send_file(2);
                }
                if ui
                    .button(tr("Send HEX → DB9-1"))
                    .on_hover_text(tr("The 8251 port (select it with P → 1, or GO 1800 → 1)"))
                    .clicked()
                {
                    self.send_file(1);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let baud = self.m.board.uart.baud();
                    let mut text = trf!("Terminal — DB9-1 · {} baud", baud);
                    let pending = self.m.board.uart.rx_pending() + self.m.board.db9_2.pending();
                    if pending > 0 {
                        text.push_str(&trf!(" · sending {} bytes", pending));
                    } else if !self.settings.jumpers.j16_terminal {
                        text.push_str(tr(" · J16 off: keypad monitor active"));
                    }
                    ui.add(egui::Label::new(RichText::new(text).strong()).truncate());
                });
            });
        });
        let (bg, fg) = term_colors(ui.visuals().dark_mode);
        let frame = egui::Frame::new()
            .fill(bg)
            .corner_radius(8)
            .inner_margin(egui::Margin::same(10))
            .stroke(if focused {
                Stroke::new(2.0, if ui.visuals().dark_mode { fg } else { Color32::from_rgb(0x18, 0x5f, 0xa5) })
            } else {
                Stroke::new(1.0, Color32::from_gray(120))
            });
        let avail = ui.available_height() - 22.0;
        let selecting = has_text_selection(ui.ctx());
        let mut action: Option<TermAction> = None;
        let mut menu_open = false;
        let resp = frame
            .show(ui, |ui| {
                ui.set_min_height(avail.max(120.0));
                ui.set_width(ui.available_width());
                // background: right-click on empty space opens the menu too
                let back = ui.interact(ui.max_rect(), ui.scope_id().with("term_back"), Sense::click());
                back.context_menu(|ui| term_menu(ui, selecting, &mut action));
                menu_open |= back.context_menu_opened();
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut text = self.term.text();
                        let blink = (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0;
                        if focused && blink && !selecting {
                            // block cursor at the current column of the last line
                            let last_len = self.term.lines.last().map(|l| l.len()).unwrap_or(0);
                            if self.term.col >= last_len {
                                text.push_str(&" ".repeat(self.term.col - last_len));
                                text.push('█');
                            }
                        }
                        let label = egui::Label::new(RichText::new(text).monospace().size(13.0).color(fg))
                            .selectable(true);
                        let lr = ui.add(label).on_hover_cursor(egui::CursorIcon::Text);
                        lr.context_menu(|ui| term_menu(ui, selecting, &mut action));
                        menu_open |= lr.context_menu_opened();
                    });
            })
            .response;
        if clicked_inside(ui, resp.rect) {
            self.target = Target::Terminal;
        }
        if menu_open {
            // egui re-reports the selection while the menu is up; keep ours
            if let Ok(mut s) = self.selected_text.lock() {
                s.ignore = true;
            }
        }
        match action {
            Some(TermAction::Copy) => {
                let t = self.selected_text.lock().map(|s| s.text.clone()).unwrap_or_default();
                ui.ctx().copy_text(t);
                self.status = tr("Selection copied").into();
            }
            Some(TermAction::CopyAll) => {
                ui.ctx().copy_text(self.term.text());
                self.status = tr("Terminal text copied").into();
            }
            Some(TermAction::Paste) => self.paste_into_terminal(),
            Some(TermAction::CtrlC) => self.send_terminal(&[0x03]),
            Some(TermAction::Clear) => self.term.clear(),
            None => {}
        }
    }

    fn paste_into_terminal(&mut self) {
        match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
            Ok(t) => {
                let t = if self.settings.force_upper { t.to_ascii_uppercase() } else { t };
                let t = t.replace("\r\n", "\r").replace('\n', "\r");
                self.send_terminal(t.as_bytes());
                self.target = Target::Terminal;
            }
            Err(e) => self.status = trf!("Nothing to paste ({})", e),
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Jumpers")).weak());
            let mut j = self.settings.jumpers.clone();
            ui.toggle_value(&mut j.j16_terminal, tr("J16 terminal mode"))
                .on_hover_text(tr("ON: terminal (CRT) monitor on DB9-1. OFF: keypad monitor. Press RS after changing."));
            ui.toggle_value(&mut j.j13_trap, tr("J13 single step"))
                .on_hover_text(tr("8155 timer drives TRAP (needed for SS and T)"));
            let mut db92 = j.j18_db9_2;
            let j18_label = if db92 { "J18 SID: DB9-2" } else { tr("J18 SID: tape") };
            ui.toggle_value(&mut db92, j18_label)
                .on_hover_text(tr("Route SID/SOD to the DB9-2 serial port or to the tape connector"));
            j.j18_db9_2 = db92;
            egui::ComboBox::from_id_salt("j17")
                .selected_text(format!("J17 {}", J17_CLOCKS[j.j17.clamp(1, 11) as usize - 1] / 16))
                .show_ui(ui, |ui| {
                    for pos in 1..=11u8 {
                        let baud = J17_CLOCKS[pos as usize - 1] / 16;
                        ui.selectable_value(&mut j.j17, pos, trf!("pos {}: {} baud", pos, baud));
                    }
                })
                .response
                .on_hover_text(tr("Baud rate of DB9-1 (8251 in x16 mode)"));
            if j != self.settings.jumpers {
                self.set_jumpers(j);
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            let c = self.shown_cpu().clone();
            ui.label(RichText::new(if self.running { tr("running") } else { tr("paused") }).small());
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "PC={:04X} SP={:04X} A={:02X} {} IE={} SOD={}",
                        c.pc,
                        c.sp,
                        c.a,
                        flag_string(c.f),
                        c.ie as u8,
                        c.sod as u8
                    ))
                    .monospace()
                    .small(),
                )
                .on_hover_text(tr("CPU registers (updated 4 times a second while running)"));
                ui.separator();
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(&self.status).small()).truncate());
                });
            });
        });
    }

    /// The CPU state to display: live when paused, sampled every 0.25 s
    /// while running.
    fn shown_cpu(&mut self) -> &microkit_emu::cpu::Cpu {
        let clock = self.clock_secs();
        if !self.running || clock - self.shown_cpu.1 >= 0.25 {
            self.shown_cpu = (self.m.cpu.clone(), clock);
        }
        &self.shown_cpu.0
    }

    fn clock_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("CPU");
        let c = self.shown_cpu().clone();
        egui::Grid::new("regs").num_columns(4).spacing([10.0, 2.0]).show(ui, |ui| {
            let mono = |s: String| RichText::new(s).monospace();
            ui.label(mono(format!("A  {:02X}", c.a)));
            ui.label(mono(format!("F  {:02X}", c.f)));
            ui.label(mono(format!("BC {:04X}", c.bc())));
            ui.label(mono(format!("DE {:04X}", c.de())));
            ui.end_row();
            ui.label(mono(format!("HL {:04X}", c.hl())));
            ui.label(mono(format!("SP {:04X}", c.sp)));
            ui.label(mono(format!("PC {:04X}", c.pc)));
            ui.label(mono(flag_string(c.f)));
            ui.end_row();
        });
        ui.label(
            RichText::new(format!(
                "IE={} masks 7.5/6.5/5.5={}{}{} I7.5={} cycles={}",
                c.ie as u8, c.m75 as u8, c.m65 as u8, c.m55 as u8, c.i75 as u8, c.cycles
            ))
            .monospace()
            .small(),
        );
        ui.horizontal(|ui| {
            let label = if self.running { tr("Pause") } else { tr("Run") };
            if ui.button(label).clicked() {
                self.running = !self.running;
            }
            if ui.add_enabled(!self.running, egui::Button::new(tr("Step"))).clicked() {
                self.m.step();
            }
            if ui.add_enabled(!self.running, egui::Button::new(tr("Step ×100"))).clicked() {
                for _ in 0..100 {
                    self.m.step();
                }
            }
        });
        ui.separator();
        ui.label(RichText::new(tr("Disassembly")).strong());
        let mut pc = c.pc;
        for _ in 0..10 {
            let (text, len) = disasm(|a| self.m.board.peek(a), pc);
            let bytes: String = (0..len).map(|i| format!("{:02X} ", self.m.board.peek(pc.wrapping_add(i)))).collect();
            let mark = if pc == c.pc { "▶" } else if self.breakpoints.contains(&pc) { "●" } else { " " };
            ui.label(RichText::new(format!("{mark}{pc:04X}  {bytes:<9} {text}")).monospace().small());
            pc = pc.wrapping_add(len);
        }
        ui.separator();
        ui.label(RichText::new(tr("Breakpoints")).strong());
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.bp_text).desired_width(60.0).hint_text("2000"));
            if ui.button(tr("Add")).clicked() {
                if let Some(a) = parse_addr(&self.bp_text) {
                    if !self.breakpoints.contains(&a) {
                        self.breakpoints.push(a);
                    }
                    self.bp_text.clear();
                }
            }
        });
        let mut remove = None;
        for (i, bp) in self.breakpoints.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{bp:04X}")).monospace());
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            self.breakpoints.remove(i);
        }
        ui.separator();
        ui.label(RichText::new(tr("Memory")).strong());
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.mem_addr).desired_width(60.0));
            for (lbl, a) in [("RAM", "2000"), ("8155", "40C0"), ("ROM", "0000")] {
                if ui.small_button(lbl).clicked() {
                    self.mem_addr = a.into();
                }
            }
        });
        let base = parse_addr(&self.mem_addr).unwrap_or(0x2000) & 0xFFF0;
        for row in 0..8u16 {
            let a = base.wrapping_add(row * 8);
            let bytes: String = (0..8).map(|i| format!("{:02X} ", self.m.board.peek(a.wrapping_add(i)))).collect();
            let ascii: String = (0..8)
                .map(|i| {
                    let b = self.m.board.peek(a.wrapping_add(i));
                    if (0x20..0x7F).contains(&b) { b as char } else { '.' }
                })
                .collect();
            ui.label(RichText::new(format!("{a:04X}  {bytes} {ascii}")).monospace().small());
        }
        ui.separator();
        ui.label(RichText::new(tr("Monitor's saved user registers")).strong());
        let pk = |a: u16| self.m.board.peek(a);
        ui.label(
            RichText::new(format!(
                "A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} F={:02X}\nH={:02X} L={:02X} I={:02X} PC={:04X} SP={:04X}",
                pk(0x40E7),
                pk(0x40E5),
                pk(0x40E4),
                pk(0x40E3),
                pk(0x40E2),
                pk(0x40E6),
                pk(0x40E9),
                pk(0x40E8),
                pk(0x40EA),
                u16::from_le_bytes([pk(0x40EB), pk(0x40EC)]),
                u16::from_le_bytes([pk(0x40ED), pk(0x40EE)])
            ))
            .monospace()
            .small(),
        );
        ui.separator();
        let pio = &self.m.board.pio;
        ui.label(
            RichText::new(format!(
                "8155 CSR={:02X} PA={:02X} PB={:02X} PC={:02X} {} {}",
                pio.csr,
                pio.out[0],
                pio.out[1],
                pio.out[2],
                tr("timer"),
                if pio.timer_running() { tr("running") } else { tr("stopped") }
            ))
            .monospace()
            .small(),
        );
    }

    // ----- tape recorder ------------------------------------------------------

    fn pick_wav(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .set_title(tr("Load tape (WAV)"))
            .add_filter(tr("WAV audio"), &["wav"])
            .pick_file()
        {
            if self.m.board.tape.modified {
                self.confirm = Some(Confirm::LoadWav(p));
            } else {
                self.load_wav(&p);
            }
        }
    }

    fn load_wav(&mut self, p: &Path) {
        match std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| Tape::from_wav(&b)) {
            Ok(mut t) => {
                t.name = Some(file_name(p));
                let secs = t.seconds(t.len() as f64);
                self.m.board.tape = t;
                self.status = trf!("Tape loaded: {} ({} s)", file_name(p), format!("{secs:.1}"));
            }
            Err(e) => self.error(tr("Couldn't load the tape"), e),
        }
    }

    fn save_wav(&mut self) {
        self.m.tape_stop();
        let name = self.m.board.tape.name.clone().unwrap_or_else(|| "tape.wav".into());
        if let Some(p) = rfd::FileDialog::new()
            .set_title(tr("Save tape as WAV"))
            .add_filter(tr("WAV audio"), &["wav"])
            .set_file_name(name)
            .save_file()
        {
            match std::fs::write(&p, self.m.board.tape.to_wav()) {
                Ok(()) => {
                    self.m.board.tape.modified = false;
                    self.m.board.tape.name = Some(file_name(&p));
                    self.status = trf!("Tape saved to {}", file_name(&p));
                }
                Err(e) => self.error(tr("Couldn't save the tape"), e.to_string()),
            }
        }
    }

    fn tape_ui(&mut self, ui: &mut egui::Ui) {
        if self.m.board.jumpers.j18_db9_2 {
            ui.horizontal(|ui| {
                let warn = ui.visuals().warn_fg_color;
                ui.label(RichText::new(tr("J18 routes SID/SOD to DB9-2, so the recorder isn't connected.")).color(warn));
                if ui.button(tr("Set J18 to tape")).clicked() {
                    let mut j = self.settings.jumpers.clone();
                    j.j18_db9_2 = false;
                    self.set_jumpers(j);
                }
            });
        }
        let now = self.m.now();
        let tape = &self.m.board.tape;
        let pos = tape.position(now).min(tape.len() as f64);
        let len_s = tape.seconds(tape.len() as f64);
        let pos_s = tape.seconds(pos);
        let state = tape.state;
        ui.horizontal(|ui| {
            let name = tape.name.clone().unwrap_or_else(|| tr("(blank tape)").into());
            let star = if tape.modified { tr(" • unsaved") } else { "" };
            ui.label(RichText::new(format!("{name}{star}")).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{} / {}", mmss(pos_s), mmss(len_s))).monospace());
            });
        });
        // waveform strip with the head position
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(300.0), 46.0), Sense::click());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 4.0, TERM_BG);
        let n = tape.len();
        if n > 0 {
            let cols = rect.width() as usize;
            for c in 0..cols {
                let a = c * n / cols;
                let b = ((c + 1) * n / cols).max(a + 1).min(n);
                let step = ((b - a) / 64).max(1);
                let peak = (a..b).step_by(step).map(|i| (tape.samples[i] as i32).abs()).max().unwrap_or(0);
                let h = (peak as f32 / 32767.0) * (rect.height() / 2.0 - 2.0);
                let x = rect.left() + c as f32 + 0.5;
                p.line_segment([pos2(x, rect.center().y - h), pos2(x, rect.center().y + h)], Stroke::new(1.0, Color32::from_rgb(0x1d, 0x9e, 0x75)));
            }
            let hx = rect.left() + (pos / n as f64) as f32 * rect.width();
            p.line_segment([pos2(hx, rect.top()), pos2(hx, rect.bottom())], Stroke::new(2.0, LED_ON));
        } else {
            p.text(rect.center(), Align2::CENTER_CENTER, tr("empty — press Record to record"), FontId::proportional(12.0), Color32::GRAY);
        }
        if resp.clicked() && state == TapeState::Stopped && n > 0 {
            if let Some(pt) = resp.interact_pointer_pos() {
                let frac = ((pt.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
                self.m.board.tape.seek(frac * len_s);
            }
        }
        ui.horizontal(|ui| {
            let playing = matches!(state, TapeState::Playing { .. });
            let recording = matches!(state, TapeState::Recording { .. });
            if ui.button("⏮").on_hover_text(tr("Rewind")).clicked() {
                let now = self.m.now();
                self.m.board.tape.rewind(now);
            }
            if ui.selectable_label(playing, tr("▶ Play")).clicked() && !playing {
                self.m.tape_play();
            }
            if ui
                .selectable_label(recording, RichText::new(tr("⏺ Record")).color(if recording { LED_ON } else { Color32::PLACEHOLDER }))
                .on_hover_text(tr("Records from the current position, over anything already there"))
                .clicked()
                && !recording
            {
                self.m.tape_record();
            }
            if ui.button(tr("⏹ Stop")).clicked() {
                self.m.tape_stop();
            }
            ui.separator();
            let now = self.m.now();
            let tape = &self.m.board.tape;
            let tone_out = recording && now.saturating_sub(tape.last_sod_edge) < CPU_HZ as u64 / 1000;
            led(ui, tone_out, "MIC");
            led(ui, tape.sid(now), "EAR");
        });
        ui.horizontal(|ui| {
            if ui.button(tr("Load WAV…")).clicked() {
                self.pick_wav();
            }
            if ui.add_enabled(!self.m.board.tape.is_empty(), egui::Button::new(tr("Save WAV…"))).clicked() {
                self.save_wav();
            }
            if ui.add_enabled(!self.m.board.tape.is_empty(), egui::Button::new(tr("Blank tape"))).clicked() {
                if self.m.board.tape.modified {
                    self.confirm = Some(Confirm::BlankTape);
                } else {
                    self.m.board.tape.clear();
                }
            }
        });
        ui.label(
            RichText::new(
                tr("Write: GO 1000 EX (keypad) or W (terminal); press Record, then the last EX / Return.\nRead: rewind, GO 1300 EX or R; press Play, then the last EX / Return."),
            )
            .weak()
            .small(),
        );
    }

    // ----- I/O ports -------------------------------------------------------------

    fn ports_ui(&mut self, ui: &mut egui::Ui) {
        let csr = self.m.board.pio.csr;
        let pc_mode = match (csr >> 2) & 3 {
            0 => tr("input (ALT1)"),
            3 => tr("output (ALT2)"),
            1 => tr("handshake for A (ALT3)"),
            _ => tr("handshake for A and B (ALT4)"),
        };
        ui.label(RichText::new(tr("8155 (IC12) — connector X2")).strong());
        ui.label(RichText::new(format!("CSR = {csr:02X}H · PC: {pc_mode}")).small().weak());
        egui::Grid::new("ports").num_columns(3).spacing([8.0, 6.0]).show(ui, |ui| {
            for (idx, name, bits) in [(0usize, "PA (41H)", 8u8), (1, "PB (42H)", 8), (2, "PC (43H)", 6)] {
                let output = match idx {
                    0 => csr & 1 != 0,
                    1 => csr & 2 != 0,
                    _ => csr & 0x0C == 0x0C,
                };
                ui.label(RichText::new(name).monospace());
                let value = if output { self.m.board.pio.out[idx] } else { self.m.board.pio.ext_in[idx] };
                ui.horizontal(|ui| {
                    for bit in (0..bits).rev() {
                        let on = value & (1 << bit) != 0;
                        if output {
                            led(ui, on, &bit.to_string());
                        } else if bit_switch(ui, on, &bit.to_string()) {
                            self.m.board.pio.ext_in[idx] ^= 1 << bit;
                        }
                    }
                });
                let mask = if bits == 8 { 0xFF } else { 0x3F };
                ui.label(
                    RichText::new(format!("{:02X}H {}", value & mask, if output { tr("out") } else { tr("in") })).monospace(),
                );
                ui.end_row();
            }
        });
        ui.label(RichText::new(tr("Outputs are LEDs; inputs are switches you can click.")).weak().small());
        ui.separator();
        ui.label(RichText::new(tr("Input ports (IC21)")).strong());
        egui::Grid::new("inports").num_columns(3).spacing([8.0, 6.0]).show(ui, |ui| {
            ui.label(RichText::new("3EH").monospace()).on_hover_text(tr("Connector X3, pins 1-3 → D0-D2"));
            ui.horizontal(|ui| {
                for bit in [2u8, 1, 0] {
                    let on = self.m.board.x3_in & (1 << bit) != 0;
                    if bit_switch(ui, on, &format!("D{bit}")) {
                        self.m.board.x3_in ^= 1 << bit;
                    }
                }
            });
            ui.label(RichText::new(tr("X3 pins 1-3")).weak().small());
            ui.end_row();
            ui.label(RichText::new("0FH").monospace());
            ui.horizontal(|ui| {
                ui.add_space(60.0);
                led(ui, !self.settings.jumpers.j16_terminal, "D0");
            });
            ui.label(RichText::new(tr("D0 = J16 (0 = terminal monitor)")).weak().small());
            ui.end_row();
        });
        ui.separator();
        let (count, mode) = self.m.board.pio.timer_setting();
        let mode_s = [tr("single square wave"), tr("square wave"), tr("single pulse"), tr("continuous pulses")][mode as usize & 3];
        let run = if self.m.board.pio.timer_running() { tr("running") } else { tr("stopped") };
        ui.label(
            RichText::new(trf!("Timer: count {} ({}H), {}, {} → TRAP via J13", count, format!("{count:04X}"), mode_s, run))
                .small(),
        );
    }

    // ----- LCD ---------------------------------------------------------------------

    fn lcd_menu(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(tr("HD44780 LCD on port B (4-bit)")).weak());
        let before = self.settings.lcd;
        ui.radio_value(&mut self.settings.lcd, LcdChoice::None, tr("Not connected"));
        ui.radio_value(&mut self.settings.lcd, LcdChoice::L16x2, "16x2");
        ui.radio_value(&mut self.settings.lcd, LcdChoice::L20x4, "20x4");
        if self.settings.lcd != before {
            self.m.board.lcd = self.settings.lcd.size().map(microkit_emu::lcd::Hd44780::new);
            if self.m.board.lcd.is_some() {
                self.settings.show_lcd = true;
            }
            ui.close();
        }
        ui.separator();
        let connected = self.m.board.lcd.is_some();
        ui.add_enabled_ui(connected, |ui| {
            ui.checkbox(&mut self.settings.show_lcd, tr("Show LCD window"));
            ui.checkbox(&mut self.settings.lcd_blue, tr("Blue backlight"));
            if ui.button(tr("Power cycle the LCD")).clicked() {
                self.settings.lcd.size().map(|s| self.m.board.lcd = Some(microkit_emu::lcd::Hd44780::new(s)));
                ui.close();
            }
        });
        ui.separator();
        ui.label(RichText::new(tr("Wiring: PB2 = RS, PB3 = E, PB4-PB7 = D4-D7;\nD0-D3 and R/W to ground. Example: examples/lcd.asm")).weak().small());
    }

    fn lcd_ui(&mut self, ui: &mut egui::Ui) {
        let now = self.m.now();
        let Some(lcd) = &self.m.board.lcd else { return };
        let (bg, on, off, frame) = if self.settings.lcd_blue {
            (Color32::from_rgb(0x1f, 0x45, 0xd8), Color32::from_rgb(0xec, 0xf3, 0xff), Color32::from_rgb(0x2a, 0x52, 0xe0), Color32::from_rgb(0x10, 0x1a, 0x40))
        } else {
            (Color32::from_rgb(0x9c, 0xc2, 0x2c), Color32::from_rgb(0x1c, 0x2a, 0x0c), Color32::from_rgb(0x92, 0xb8, 0x27), Color32::from_rgb(0x1a, 0x24, 0x12))
        };
        let (cols, rows) = (lcd.size.cols, lcd.size.rows);
        // Work in whole screen pixels so every dot and every gap is the same
        // size (fractional positions round to uneven 1- and 2-pixel gaps).
        let ppp = ui.ctx().pixels_per_point();
        let dot_px = (3.0 * ppp).round().max(2.0);
        let gap_px = (1.0 * ppp).round().max(1.0);
        let pitch_px = dot_px + gap_px;
        // between characters: one dot's width more than between dots
        let cell_w_px = 5.0 * pitch_px + dot_px;
        let cell_h_px = 8.0 * pitch_px + dot_px;
        let (dot, pitch, cell_w, cell_h) = (dot_px / ppp, pitch_px / ppp, cell_w_px / ppp, cell_h_px / ppp);
        let pad = 14.0;
        let size = vec2(cols as f32 * cell_w + 2.0 * pad, rows as f32 * cell_h + 2.0 * pad);
        let (rect, _) = ui.allocate_exact_size(size + vec2(12.0, 12.0), Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 6.0, frame);
        let glass = rect.shrink(6.0);
        p.rect_filled(glass, 3.0, bg);
        let blink_full = lcd.blink_on && (now as f64 / CPU_HZ as f64 / 0.2048) as u64 % 2 == 0;
        let cursor = if lcd.display_on { lcd.cursor_at() } else { None };
        for r in 0..rows {
            for c in 0..cols {
                let base = ((glass.min + vec2(pad, pad)).to_vec2() * ppp).round() / ppp;
                let origin = pos2(base.x, base.y) + vec2(c as f32 * cell_w, r as f32 * cell_h);
                let mut g = match lcd.char_at(r, c) {
                    Some(code) => lcd.glyph(code),
                    None => [0; 8],
                };
                if !lcd.initialized && r % 2 == 0 {
                    g = [0x1F; 8]; // a module that hasn't been set up shows dark blocks
                }
                if cursor == Some((r, c)) {
                    if lcd.cursor_on {
                        g[7] = 0x1F;
                    }
                    if blink_full {
                        g = [0x1F; 8];
                    }
                }
                for (y, bits) in g.iter().enumerate() {
                    for x in 0..5 {
                        let lit = bits & (0x10 >> x) != 0;
                        let d = Rect::from_min_size(origin + vec2(x as f32 * pitch, y as f32 * pitch), vec2(dot, dot));
                        p.rect_filled(d, 0.0, if lit { on } else { off });
                    }
                }
            }
        }
        let mut info = vec![
            if !lcd.initialized { tr("not initialized").to_string() } else if lcd.four_bit { tr("4-bit mode").to_string() } else { tr("8-bit mode").to_string() },
            if lcd.display_on { tr("display on").to_string() } else { tr("display off").to_string() },
            format!("AC={:02X}H", lcd.ac),
        ];
        if !lcd.two_lines && lcd.initialized {
            info.push(tr("1-line mode").to_string());
        }
        ui.label(RichText::new(info.join(" · ")).small().weak());
        if lcd.early_writes > 0 {
            ui.label(
                RichText::new(trf!("{} bytes arrived while the LCD was busy; a real module may lose them. Wait longer between writes.", lcd.early_writes))
                    .small()
                    .color(ui.visuals().warn_fg_color),
            );
        }
        if self.m.board.pio.csr & 0x02 == 0 {
            ui.label(RichText::new(tr("Port B is an input, so the LCD sees nothing.")).small().weak());
        }
    }

    fn tool_windows(&mut self, ctx: &egui::Context) {
        let mut show = self.settings.show_tape;
        if show {
            egui::Window::new(tr("📼 Tape recorder"))
                .open(&mut show)
                .resizable(false)
                .default_width(380.0)
                .default_pos(pos2(400.0, 120.0))
                .show(ctx, |ui| self.tape_ui(ui));
            self.settings.show_tape = show;
        }
        if self.m.board.lcd.is_some() && self.settings.show_lcd {
            let mut show = true;
            let title = match self.settings.lcd {
                LcdChoice::L20x4 => "LCD 20x4",
                _ => "LCD 16x2",
            };
            egui::Window::new(title)
                .id(egui::Id::unique("lcd_window"))
                .open(&mut show)
                .resizable(false)
                .default_pos(pos2(420.0, 90.0))
                .show(ctx, |ui| self.lcd_ui(ui));
            self.settings.show_lcd = show;
        }
        let mut show = self.settings.show_ports;
        if show {
            egui::Window::new(tr("🔌 I/O ports"))
                .open(&mut show)
                .resizable(false)
                .default_pos(pos2(420.0, 360.0))
                .show(ctx, |ui| self.ports_ui(ui));
            self.settings.show_ports = show;
        }
        if let Some(c) = &self.confirm {
            let what = match c {
                Confirm::BlankTape => tr("Erase the tape?"),
                Confirm::LoadWav(_) => tr("Replace the tape?"),
            };
            let mut answer = None;
            egui::Window::new(what).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label(tr("The current tape has a recording you haven't saved."));
                ui.horizontal(|ui| {
                    if ui.button(tr("Discard recording")).clicked() {
                        answer = Some(true);
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        answer = Some(false);
                    }
                });
            });
            if let Some(yes) = answer {
                let c = self.confirm.take().unwrap();
                if yes {
                    match c {
                        Confirm::BlankTape => self.m.board.tape.clear(),
                        Confirm::LoadWav(p) => self.load_wav(&p),
                    }
                }
            }
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some((title, text)) = self.message.clone() {
            let mut open = true;
            egui::Window::new(title).collapsible(false).resizable(false).open(&mut open).show(ctx, |ui| {
                ui.label(text);
                if ui.button("OK").clicked() {
                    self.message = None;
                }
            });
            if !open {
                self.message = None;
            }
        }
        if let Some(mut d) = self.bin_dialog.take() {
            let mut keep = true;
            egui::Window::new(tr("Load binary")).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label(trf!("{} ({} bytes)", file_name(&d.path), d.data.len()));
                ui.horizontal(|ui| {
                    ui.label(tr("Load address (hex):"));
                    ui.add(egui::TextEdit::singleline(&mut d.addr).desired_width(70.0));
                });
                ui.horizontal(|ui| {
                    if ui.button(tr("Load")).clicked() {
                        match parse_addr(&d.addr) {
                            Some(a) => {
                                self.settings.last_bin_addr = d.addr.clone();
                                let img = Image::from_bin(&d.data, a);
                                let path = d.path.clone();
                                self.apply_image(&path, img);
                                keep = false;
                            }
                            None => self.status = tr("Enter a hex address like 2000").into(),
                        }
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        keep = false;
                    }
                });
            });
            if keep {
                self.bin_dialog = Some(d);
            }
        }
        if let Some(mut d) = self.save_dialog.take() {
            let mut keep = true;
            egui::Window::new(tr("Save memory range")).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("From"));
                    ui.add(egui::TextEdit::singleline(&mut d.start).desired_width(60.0));
                    ui.label(tr("to"));
                    ui.add(egui::TextEdit::singleline(&mut d.end).desired_width(60.0));
                });
                ui.horizontal(|ui| {
                    if ui.button(tr("Save…")).clicked() {
                        match (parse_addr(&d.start), parse_addr(&d.end)) {
                            (Some(s), Some(e)) => {
                                keep = false;
                                self.save_memory(s, e);
                            }
                            _ => self.status = tr("Enter hex addresses like 2000 and 20FF").into(),
                        }
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        keep = false;
                    }
                });
            });
            if keep {
                self.save_dialog = Some(d);
            }
        }
        if self.show_help {
            egui::Window::new(tr("MicroKit-Emu — keys and usage")).open(&mut self.show_help).show(ctx, |ui| {
                ui.label(if self.settings.language == Language::Greek { HELP_TEXT_EL } else { HELP_TEXT });
            });
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // tr("Follow the system"): egui uses the fallback theme when the backend
        // can't tell, so feed it what the desktop told us (light if unsure).
        let want = ctx.input(|i| i.raw.system_theme).or(self.system_theme.get()).unwrap_or(egui::Theme::Light);
        if ctx.options(|o| o.fallback_theme) != want {
            ctx.options_mut(|o| o.fallback_theme = want);
        }
        self.tick();
        if self.running {
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // drag and drop
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        for p in dropped {
            self.load_program(&p);
        }
        // a right-click on the terminal makes egui report a new selection;
        // don't let that replace what the user dragged over
        let right = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary) || i.pointer.button_released(egui::PointerButton::Secondary));
        if let Ok(mut s) = self.selected_text.lock() {
            s.ignore = right;
        }
        self.handle_keyboard(&ctx);

        egui::Panel::top("menu").show(ui, |ui| {
            self.menu_bar(ui);
            ui.add_space(2.0);
            self.toolbar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.add_space(4.0);
            self.status_bar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::left("board").resizable(false).exact_size(370.0).show(ui, |ui| {
            ui.add_space(8.0);
            self.board(ui);
        });
        if self.settings.show_debug {
            egui::Panel::right("debug").default_size(330.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.debug_panel(ui));
            });
        }
        egui::CentralPanel::default_margins().show(ui, |ui| self.terminal(ui));
        self.dialogs(&ctx);
        self.tool_windows(&ctx);

        if !self.running {
            // keep the cursor blinking and the display live
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
    }
}

// ----- helpers ------------------------------------------------------------------

const HELP_TEXT: &str = "\
Keypad (click the board first):
  0-9, A-F   hex digits
  M          M  - examine/modify memory
  R          R  - examine/modify registers
  G          GO - run a program
  S          SS - single step
  N, Space   N  - next
  . , Enter  EX - execute / end
  F9         RS - reset
  F10        VI - vector interrupt (RST 7.5 → 40DFH)

Built-in ROM programs (GO address EX):
  1000 tape write   1300 tape read
  1600 download HEX 1800 select download port

Terminal (set J16 on, then RS): click the terminal and type.
  Commands must be capitals; 'Type in capitals' does that for you.
  Enter = CR, Esc = ESC, Ctrl+C = ^C, Ctrl+S/Q = XOFF/XON.
  Drag over text to select it; then Ctrl+C copies it instead of
  sending ^C. Right-click: Copy, Copy all text, Paste (Ctrl+V),
  Send Ctrl+C, Clear terminal.

Theme: View → Follow the system / Dark / Light.
Language: Language → English / Ελληνικά.

Monitor ROM: File → Monitor ROM chooses the original monitor
  5.12 (default), the monitor + Tiny BASIC, or a file of your own.

Tiny BASIC (ROM 'Monitor 5.12 + Tiny BASIC', J16 on):
  press RS, type B in the terminal.
  BYE returns to the monitor; B again keeps your program.
  Send HEX → DB9-1 with a .bas file types the program in.

Loading programs:
  File → Load program, the toolbar, or drag a file onto the window.
  .hex  Intel HEX (addresses from the file)
  .asm  8085 source, assembled by the built-in assembler
  .bin  raw bytes, you choose the load address
  The GO address is set to the program's start, so GO EX runs it.
  End your programs with RST 1 (CF) to return to the monitor.

  Or use the kit's own download: GO 1600 EX (keypad) or L (terminal),
  then Serial → Send HEX file to DB9-2 (or DB9-1 after P → 1).";

const HELP_TEXT_EL: &str = "\
Πληκτρολόγιο (κάντε πρώτα κλικ στην πλακέτα):
  0-9, A-F   δεκαεξαδικά ψηφία
  M          M  - εξέταση/αλλαγή μνήμης
  R          R  - εξέταση/αλλαγή καταχωρητών
  G          GO - εκτέλεση προγράμματος
  S          SS - βηματική εκτέλεση
  N, Space   N  - επόμενο
  . , Enter  EX - εκτέλεση / τέλος
  F9         RS - επαναφορά (reset)
  F10        VI - διακοπή (RST 7.5 → 40DFH)

Προγράμματα της ROM (GO διεύθυνση EX):
  1000 εγγραφή κασέτας    1300 ανάγνωση κασέτας
  1600 λήψη HEX           1800 επιλογή θύρας λήψης

Τερματικό (J16 on και μετά RS): κάντε κλικ στο τερματικό και
  πληκτρολογήστε. Οι εντολές γράφονται με κεφαλαία· η επιλογή
  «Πληκτρολόγηση με κεφαλαία» το κάνει για εσάς.
  Enter = CR, Esc = ESC, Ctrl+C = ^C, Ctrl+S/Q = XOFF/XON.
  Σύρετε πάνω στο κείμενο για να το επιλέξετε· τότε το Ctrl+C
  αντιγράφει αντί να στείλει ^C. Δεξί κλικ: Αντιγραφή, Αντιγραφή
  όλου του κειμένου, Επικόλληση (Ctrl+V), Αποστολή Ctrl+C,
  Καθαρισμός τερματικού.

Θέμα: Προβολή → Όπως το σύστημα / Σκούρο / Ανοιχτό.
Γλώσσα: Γλώσσα → English / Ελληνικά.

ROM του monitor: Αρχείο → ROM του monitor επιλέγει το αρχικό
  monitor 5.12 (προεπιλογή), το monitor + Tiny BASIC ή δικό σας
  αρχείο.

Tiny BASIC (ROM «Monitor 5.12 + Tiny BASIC», J16 on):
  πατήστε RS και γράψτε B στο τερματικό.
  Το BYE επιστρέφει στο monitor· με B ξανά το πρόγραμμα μένει.
  Αποστολή HEX → DB9-1 με αρχείο .bas πληκτρολογεί το πρόγραμμα.

Φόρτωση προγραμμάτων:
  Αρχείο → Φόρτωση προγράμματος, η γραμμή εργαλείων ή σύρετε ένα
  αρχείο στο παράθυρο.
  .hex  Intel HEX (οι διευθύνσεις από το αρχείο)
  .asm  πηγαίος κώδικας 8085, με τον ενσωματωμένο assembler
  .bin  σκέτα byte, εσείς διαλέγετε τη διεύθυνση φόρτωσης
  Η διεύθυνση GO γίνεται η αρχή του προγράμματος, οπότε GO EX το
  εκτελεί. Τελειώστε τα προγράμματα με RST 1 (CF) για επιστροφή
  στο monitor.

  Ή με τη λήψη του ίδιου του κιτ: GO 1600 EX (πληκτρολόγιο) ή L
  (τερματικό) και μετά Σειριακή → Αποστολή αρχείου HEX στη DB9-2
  (ή στη DB9-1 μετά από P → 1).";

/// Remembers the last settled text selection (egui sends it to the
/// backend at the end of each pass, after our UI code has run).
#[derive(Default)]
struct Selection {
    text: String,
    /// Set for frames with a right-click (see `App::ui`).
    ignore: bool,
}

struct SelectionWatcher(std::sync::Arc<std::sync::Mutex<Selection>>);

impl egui::Plugin for SelectionWatcher {
    fn debug_name(&self) -> &'static str {
        "SelectionWatcher"
    }
    fn output_hook(&mut self, _ctx: &egui::Context, output: &mut egui::FullOutput) {
        for c in &output.platform_output.commands {
            if let egui::OutputCommand::TextSelectionSettled(t) = c {
                if std::env::var_os("MICROKIT_DEBUG").is_some() {
                    eprintln!("selection settled: {t:?} ignore={}", self.0.lock().map(|s| s.ignore).unwrap_or(false));
                }
                if let Ok(mut s) = self.0.lock() {
                    if !s.ignore {
                        s.text = t.clone();
                    }
                }
            }
        }
    }
}

/// egui's stock light theme is low-contrast (grey 80 text, thin glyphs,
/// pale buttons). This one keeps the same layout with darker text, heavier
/// glyph rendering, a warm grey background and visible control outlines.
fn light_visuals() -> egui::Visuals {
    use egui::epaint::FontColorTransferFunction;
    let rgb = Color32::from_rgb;
    let ink = rgb(0x1a, 0x1a, 0x18);
    let mut v = egui::Visuals::light();
    v.text_options.color_transfer_function = FontColorTransferFunction::Gamma(0.7);
    v.panel_fill = rgb(0xe8, 0xe6, 0xdf);
    v.window_fill = rgb(0xf4, 0xf3, 0xee);
    v.window_stroke = Stroke::new(1.0, rgb(0x9a, 0x97, 0x8e));
    v.faint_bg_color = rgb(0xdd, 0xda, 0xd2);
    v.extreme_bg_color = Color32::WHITE;
    v.code_bg_color = rgb(0xdc, 0xd9, 0xd0);
    v.weak_text_color = Some(rgb(0x55, 0x54, 0x4f));
    v.warn_fg_color = rgb(0xa8, 0x4a, 0x00);
    v.error_fg_color = rgb(0xb4, 0x14, 0x14);
    v.hyperlink_color = rgb(0x0b, 0x5c, 0xad);
    v.selection.bg_fill = rgb(0x9c, 0xc3, 0xee);
    v.selection.stroke = Stroke::new(1.0, rgb(0x0a, 0x36, 0x6a));
    let w = &mut v.widgets;
    w.noninteractive.weak_bg_fill = v.panel_fill;
    w.noninteractive.bg_fill = v.panel_fill;
    w.noninteractive.bg_stroke = Stroke::new(1.0, rgb(0xa8, 0xa5, 0x9c));
    w.noninteractive.fg_stroke = Stroke::new(1.0, ink);
    w.inactive.weak_bg_fill = rgb(0xd6, 0xd3, 0xca);
    w.inactive.bg_fill = rgb(0xd6, 0xd3, 0xca);
    w.inactive.bg_stroke = Stroke::new(1.0, rgb(0x9a, 0x97, 0x8e));
    w.inactive.fg_stroke = Stroke::new(1.0, ink);
    w.hovered.weak_bg_fill = rgb(0xc8, 0xc4, 0xb9);
    w.hovered.bg_fill = rgb(0xc8, 0xc4, 0xb9);
    w.hovered.bg_stroke = Stroke::new(1.0, rgb(0x55, 0x54, 0x4f));
    w.hovered.fg_stroke = Stroke::new(1.5, Color32::BLACK);
    w.active.weak_bg_fill = rgb(0xb5, 0xb0, 0xa4);
    w.active.bg_fill = rgb(0xb5, 0xb0, 0xa4);
    w.open.weak_bg_fill = rgb(0xcf, 0xcb, 0xc1);
    w.open.bg_fill = rgb(0xcf, 0xcb, 0xc1);
    w.open.bg_stroke = Stroke::new(1.0, rgb(0x7a, 0x77, 0x6f));
    v
}

fn term_colors(dark: bool) -> (Color32, Color32) {
    if dark {
        (TERM_BG, TERM_FG)
    } else {
        (Color32::from_rgb(0xff, 0xfe, 0xfa), Color32::from_rgb(0x10, 0x10, 0x0e))
    }
}

fn has_text_selection(ctx: &egui::Context) -> bool {
    ctx.with_plugin(|s: &mut egui::text_selection::LabelSelectionState| s.has_selection()).unwrap_or(false)
}

fn term_menu(ui: &mut egui::Ui, selecting: bool, action: &mut Option<TermAction>) {
    if ui.add_enabled(selecting, egui::Button::new(tr("Copy")).shortcut_text("Ctrl+C")).clicked() {
        *action = Some(TermAction::Copy);
        ui.close();
    }
    if ui.button(tr("Copy all text")).clicked() {
        *action = Some(TermAction::CopyAll);
        ui.close();
    }
    if ui.add(egui::Button::new(tr("Paste")).shortcut_text("Ctrl+V")).clicked() {
        *action = Some(TermAction::Paste);
        ui.close();
    }
    ui.separator();
    if ui.button(tr("Send Ctrl+C")).clicked() {
        *action = Some(TermAction::CtrlC);
        ui.close();
    }
    if ui.button(tr("Clear terminal")).clicked() {
        *action = Some(TermAction::Clear);
        ui.close();
    }
}

/// True if the primary button was clicked inside `rect` this frame (without
/// registering a widget that would steal clicks from the children).
fn clicked_inside(ui: &egui::Ui, rect: Rect) -> bool {
    let Some(p) = ui.input(|i| if i.pointer.primary_clicked() { i.pointer.interact_pos() } else { None }) else {
        return false;
    };
    // ignore clicks on windows floating above this area
    rect.contains(p) && ui.ctx().layer_id_at(p).is_none_or(|l| l == ui.layer_id())
}

fn mmss(secs: f64) -> String {
    format!("{:02}:{:04.1}", (secs / 60.0) as u32, secs % 60.0)
}

/// A small LED with a caption underneath.
fn led(ui: &mut egui::Ui, on: bool, caption: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(22.0, 30.0), Sense::hover());
    let c = pos2(rect.center().x, rect.top() + 8.0);
    ui.painter().circle_filled(c, 6.5, if on { LED_ON } else { LED_OFF });
    ui.painter().text(pos2(c.x, rect.bottom() - 6.0), Align2::CENTER_CENTER, caption, FontId::proportional(10.0), ui.visuals().weak_text_color());
}

/// A clickable bit switch; returns true when clicked.
fn bit_switch(ui: &mut egui::Ui, on: bool, caption: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 30.0), Sense::click());
    let r = Rect::from_center_size(pos2(rect.center().x, rect.top() + 8.0), vec2(12.0, 16.0));
    let p = ui.painter();
    p.rect_filled(r, 3.0, if on { Color32::from_rgb(0x37, 0x8a, 0xdd) } else { Color32::from_gray(70) });
    let knob = if on { Rect::from_min_size(r.min, vec2(12.0, 8.0)) } else { Rect::from_min_size(pos2(r.min.x, r.center().y), vec2(12.0, 8.0)) };
    p.rect_filled(knob.shrink(1.5), 2.0, Color32::from_gray(230));
    p.text(pos2(rect.center().x, rect.bottom() - 6.0), Align2::CENTER_CENTER, caption, FontId::proportional(10.0), ui.visuals().weak_text_color());
    resp.on_hover_text(if on { tr("1 — click to set 0") } else { tr("0 — click to set 1") }).clicked()
}

fn key_hint(k: Key) -> &'static str {
    match k {
        Key::Reset => tr("RS: reset (F9)"),
        Key::Vi => tr("VI: vector interrupt, RST 7.5 (F10)"),
        Key::Step => tr("SS: single step (S)"),
        Key::Go => tr("GO: execute program (G)"),
        Key::Mem => tr("M: examine/modify memory (M); inside M it steps back"),
        Key::Reg => tr("R: examine/modify registers (R)"),
        Key::Next => tr("N: next (N or Space)"),
        Key::Ex => tr("EX: execute / end (. or Enter)"),
        Key::Hex(_) => tr("hex digit"),
    }
}

fn jumper_controls(ui: &mut egui::Ui, j: &mut Jumpers) {
    ui.checkbox(&mut j.j16_terminal, tr("J16 terminal monitor (press RS after)"));
    ui.checkbox(&mut j.j13_trap, tr("J13 timer → TRAP (single step)"));
    ui.checkbox(&mut j.j18_db9_2, tr("J18 SID/SOD on DB9-2 (off = tape)"));
    ui.checkbox(&mut j.j14_8155_ram, tr("J14 8155 RAM enabled"));
}

fn flag_string(f: u8) -> String {
    let names = [(0x80, 'S'), (0x40, 'Z'), (0x10, 'A'), (0x04, 'P'), (0x01, 'C')];
    names.iter().map(|&(m, c)| if f & m != 0 { c } else { '-' }).collect()
}

fn draw_digit(p: &egui::Painter, r: Rect, segs: [bool; 8]) {
    let slant = 0.12;
    let w = 6.5;
    let x0 = r.left() + 6.0;
    let x1 = r.right() - 8.0;
    let y0 = r.top() + 3.0;
    let y2 = r.bottom() - 3.0;
    let y1 = (y0 + y2) / 2.0;
    let sk = |x: f32, y: f32| pos2(x + (y1 - y) * slant, y);
    let ends = [
        (sk(x0, y0), sk(x1, y0)), // a
        (sk(x1, y0), sk(x1, y1)), // b
        (sk(x1, y1), sk(x1, y2)), // c
        (sk(x0, y2), sk(x1, y2)), // d
        (sk(x0, y1), sk(x0, y2)), // e
        (sk(x0, y0), sk(x0, y1)), // f
        (sk(x0, y1), sk(x1, y1)), // g
    ];
    for (i, (a, b)) in ends.iter().enumerate() {
        let col = if segs[i] { LED_ON } else { LED_OFF };
        p.add(Shape::convex_polygon(segment_poly(*a, *b, w), col, Stroke::NONE));
    }
    let dp = pos2(r.right() - 2.0, y2);
    p.circle_filled(dp, 3.5, if segs[7] { LED_ON } else { LED_OFF });
}

fn segment_poly(a: Pos2, b: Pos2, w: f32) -> Vec<Pos2> {
    let d = (b - a).normalized();
    let n = Vec2::new(-d.y, d.x);
    let h = w / 2.0;
    let gap = 1.2;
    let a = a + d * gap;
    let b = b - d * gap;
    vec![a, a + d * h + n * h, b - d * h + n * h, b, b - d * h - n * h, a + d * h - n * h]
}

fn ext_of(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn pick_program() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(tr("Load program into RAM"))
        .add_filter(tr("Programs"), &["hex", "ihx", "bin", "asm", "a85"])
        .add_filter(tr("All files"), &["*"])
        .pick_file()
}

fn image_to_hex(img: &Image) -> String {
    let mut s = String::new();
    for (start, end) in img.ranges() {
        let data: Vec<u8> = (start..=end).map(|a| img.bytes[&a]).collect();
        let part = loader::write_ihex(start, &data);
        s.push_str(part.trim_end_matches(":00000001FF\r\n"));
    }
    s.push_str(":00000001FF\r\n");
    s
}
