//! The Microkit board: memory map, I/O decoding, jumpers and the glue that
//! connects the peripherals to the CPU's interrupt pins.

use crate::cpu::{Bus, Cpu, IntLines};
use crate::periph::{I8155, I8251, I8279, SerialLine};
use crate::tape::Tape;

pub const CPU_HZ: u32 = 3_072_000;

/// Clock produced by the J17 divider for each jumper position (1..=11).
pub const J17_CLOCKS: [u32; 11] =
    [153_600, 76_800, 38_400, 19_200, 9_600, 4_800, 2_400, 1_200, 600, 300, 150];

/// Keypad keys and their 8279 codes (scan line * 8 + return line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Hex(u8),
    Ex,
    Next,
    Go,
    Mem,
    Reg,
    Step,
    Reset,
    Vi,
}

impl Key {
    pub fn code(self) -> Option<u8> {
        Some(match self {
            Key::Hex(d) => d & 0x0F,
            Key::Ex => 0x10,
            Key::Next => 0x11,
            Key::Go => 0x12,
            Key::Mem => 0x13,
            Key::Reg => 0x14,
            Key::Step => 0x15,
            Key::Reset | Key::Vi => return None,
        })
    }
    pub fn label(self) -> String {
        match self {
            Key::Hex(d) => format!("{:X}", d),
            Key::Ex => "EX".into(),
            Key::Next => "N".into(),
            Key::Go => "GO".into(),
            Key::Mem => "M".into(),
            Key::Reg => "R".into(),
            Key::Step => "SS".into(),
            Key::Reset => "RS".into(),
            Key::Vi => "VI".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Jumpers {
    /// J16 ON: terminal (CRT) monitor; OFF: keypad monitor.
    pub j16_terminal: bool,
    /// J13 ON (and J1 OFF): 8155 timer drives TRAP, single step works.
    pub j13_trap: bool,
    /// J18 2-3: SID/SOD go to DB9-2; 1-2: tape.
    pub j18_db9_2: bool,
    /// J17 position 1..=11.
    pub j17: u8,
    /// J14 1-2: 8155 RAM enabled at 4000H.
    pub j14_8155_ram: bool,
}

impl Default for Jumpers {
    fn default() -> Self {
        Jumpers { j16_terminal: false, j13_trap: true, j18_db9_2: true, j17: 1, j14_8155_ram: true }
    }
}

pub struct Board {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub kbd: I8279,
    pub pio: I8155,
    pub uart: I8251,
    /// Bytes arriving on DB9-2 RxD, routed to SID when J18 = 2-3.
    pub db9_2: SerialLine,
    pub jumpers: Jumpers,
    pub sod: bool,
    /// Last SID level the CPU read (for the activity LED).
    pub sid_level: bool,
    pub tape: Tape,
    /// Inputs on connector X3 → port 0FH bits 1 and 2 (bit 0 is J16).
    pub x3_in: u8,
    trap_edge: bool,
}

impl Board {
    fn new() -> Self {
        Board {
            rom: vec![0xFF; 0x2000],
            ram: vec![0; 0x2000],
            kbd: I8279::default(),
            pio: I8155::default(),
            uart: I8251::new(CPU_HZ, J17_CLOCKS[0]),
            db9_2: SerialLine::new((CPU_HZ / 9600) as u64),
            jumpers: Jumpers::default(),
            sod: false,
            sid_level: true,
            tape: Tape::default(),
            x3_in: 0b110,
            trap_edge: false,
        }
    }

    /// Debugger/loader view of memory (no side effects).
    pub fn peek(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x1FFF => self.rom[a as usize],
            0x2000..=0x3FFF => self.ram[a as usize - 0x2000],
            0x4000..=0x43FF if self.jumpers.j14_8155_ram => self.pio.ram[(a & 0xFF) as usize],
            _ => 0xFF,
        }
    }

    /// Returns false if nothing writable is at that address.
    pub fn poke(&mut self, a: u16, v: u8) -> bool {
        match a {
            0x2000..=0x3FFF => {
                self.ram[a as usize - 0x2000] = v;
                true
            }
            0x4000..=0x43FF if self.jumpers.j14_8155_ram => {
                self.pio.ram[(a & 0xFF) as usize] = v;
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, now: u64) {
        if self.pio.update(now) && self.jumpers.j13_trap {
            self.trap_edge = true;
        }
        self.uart.update(now);
    }
}

impl Bus for Board {
    fn read(&mut self, a: u16) -> u8 {
        self.peek(a)
    }
    fn write(&mut self, a: u16, v: u8) {
        self.poke(a, v);
    }
    fn input(&mut self, port: u8, now: u64) -> u8 {
        self.uart.update(now);
        match port & 0xF8 {
            0x08 => 0xF8 | (self.x3_in & 0b110) | (!self.jumpers.j16_terminal) as u8,
            0x18 => {
                if port & 1 != 0 {
                    self.uart.read_status(now)
                } else {
                    self.uart.read_data()
                }
            }
            0x28 => {
                if port & 1 != 0 {
                    self.kbd.read_status()
                } else {
                    self.kbd.read_data()
                }
            }
            0x40 => self.pio.read_io(port & 7, now),
            _ => 0xFF,
        }
    }
    fn output(&mut self, port: u8, v: u8, now: u64) {
        match port & 0xF8 {
            0x18 => {
                if port & 1 != 0 {
                    self.uart.write_ctl(v)
                } else {
                    self.uart.write_data(v, now)
                }
            }
            0x28 => {
                if port & 1 != 0 {
                    self.kbd.write_cmd(v)
                } else {
                    self.kbd.write_data(v)
                }
            }
            0x40 => self.pio.write_io(port & 7, v, now),
            _ => {}
        }
    }
    fn sid(&mut self, now: u64) -> bool {
        self.sid_level = if self.jumpers.j18_db9_2 { self.db9_2.level(now) } else { self.tape.sid(now) };
        self.sid_level
    }
    fn sod(&mut self, level: bool, now: u64) {
        self.sod = level;
        if !self.jumpers.j18_db9_2 {
            self.tape.sod(level, now);
        }
    }
}

pub struct Machine {
    pub cpu: Cpu,
    pub board: Board,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        Machine { cpu: Cpu::new(), board: Board::new() }
    }

    pub fn load_rom(&mut self, image: &[u8]) {
        self.board.rom = vec![0xFF; 0x2000];
        let n = image.len().min(0x2000);
        self.board.rom[..n].copy_from_slice(&image[..n]);
    }

    /// The RS key: resets the CPU and every chip on the RESET OUT line.
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.board.kbd.reset();
        self.board.pio.reset();
        self.board.uart.reset();
        self.board.trap_edge = false;
    }

    pub fn apply_jumpers(&mut self) {
        let pos = self.board.jumpers.j17.clamp(1, 11) as usize;
        self.board.uart.clock_hz = J17_CLOCKS[pos - 1];
    }

    pub fn press(&mut self, key: Key) {
        match key {
            Key::Reset => self.reset(),
            Key::Vi => self.cpu.pulse_rst75(),
            k => self.board.kbd.key_press(k.code().unwrap()),
        }
    }

    /// Run one instruction (or interrupt acknowledge).
    pub fn step(&mut self) -> u64 {
        let lines = IntLines {
            trap: self.board.trap_edge,
            rst65: false,
            rst55: self.board.kbd.irq(),
            intr: false,
        };
        self.board.trap_edge = false;
        let n = self.cpu.step(&mut self.board, lines);
        let now = self.cpu.cycles;
        self.board.update(now);
        n
    }

    /// Housekeeping that doesn't need to run every instruction.
    pub fn periodic(&mut self) {
        let now = self.cpu.cycles;
        self.board.tape.update(now);
    }

    /// Run for at least `cycles` clock cycles, stopping early at a breakpoint.
    /// Returns true if a breakpoint was hit.
    pub fn run_for(&mut self, cycles: u64, breakpoints: &[u16]) -> bool {
        let end = self.cpu.cycles + cycles;
        while self.cpu.cycles < end {
            self.step();
            if !breakpoints.is_empty() && breakpoints.contains(&self.cpu.pc) {
                return true;
            }
        }
        false
    }

    pub fn now(&self) -> u64 {
        self.cpu.cycles
    }

    pub fn tape_play(&mut self) {
        let now = self.cpu.cycles;
        self.board.tape.play(now);
    }

    pub fn tape_record(&mut self) {
        let now = self.cpu.cycles;
        let sod = self.board.sod;
        self.board.tape.record(now, sod);
    }

    pub fn tape_stop(&mut self) {
        let now = self.cpu.cycles;
        self.board.tape.stop(now);
    }

    /// The six digits of the display: raw 8279 RAM bytes 0..6.
    pub fn display(&self) -> [u8; 6] {
        let mut d = [0u8; 6];
        d.copy_from_slice(&self.board.kbd.display[..6]);
        d
    }
}

/// Monitor RAM locations (from the listing).
pub mod monitor {
    pub const PSAV: u16 = 0x40EB;
    pub const SSAV: u16 = 0x40ED;
    pub const USER_RAM: std::ops::RangeInclusive<u16> = 0x2000..=0x3FFF;
}
