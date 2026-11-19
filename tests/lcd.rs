//! The HD44780 LCD on port B, driven by the example programs.

use mikrokit_emu::asm;
use mikrokit_emu::lcd::{Hd44780, LCD_16X2, LCD_20X4};
use mikrokit_emu::machine::{Key, Machine};
use mikrokit_emu::roms;
use mikrokit_emu::segments::display_text;
use std::path::PathBuf;

const MS: u64 = 3072;

fn example(name: &str) -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join(name)).unwrap()
}

fn machine(rom: &[u8], terminal: bool) -> Machine {
    let mut m = Machine::new();
    m.load_rom(rom);
    m.board.jumpers.j16_terminal = terminal;
    m.apply_jumpers();
    m.reset();
    m.run_for(100 * MS, &[]);
    m
}

fn run_asm_example(size: mikrokit_emu::lcd::LcdSize) -> Machine {
    let mut m = machine(roms::ORIGINAL, false);
    m.board.lcd = Some(Hd44780::new(size));
    for (&a, &b) in &asm::assemble(&example("lcd.asm")).unwrap().image.bytes {
        m.board.poke(a, b);
    }
    for k in [Key::Go, Key::Hex(2), Key::Hex(0), Key::Hex(0), Key::Hex(0), Key::Ex] {
        m.press(k);
        m.run_for(20 * MS, &[]);
    }
    m.run_for(300 * MS, &[]);
    m
}

#[test]
fn asm_example_on_16x2() {
    let m = run_asm_example(LCD_16X2);
    let lcd = m.board.lcd.as_ref().unwrap();
    assert_eq!(lcd.row_text(0), "MikroKit-Emu    ");
    assert_eq!(lcd.row_text(1), "8085 + HD44780  ");
    assert_eq!(lcd.early_writes, 0, "the delays are long enough");
    assert_eq!(display_text(&m.display()), "- 80|85", "returned to the monitor");
}

#[test]
fn asm_example_on_20x4() {
    let m = run_asm_example(LCD_20X4);
    let lcd = m.board.lcd.as_ref().unwrap();
    assert_eq!(lcd.row_text(0), "MikroKit-Emu        ");
    assert_eq!(lcd.row_text(1), "8085 + HD44780      ");
    assert_eq!(lcd.row_text(2), "Row 3 of a 20x4     ");
    assert_eq!(lcd.row_text(3), "Row 4: 0123456789   ");
    assert_eq!(lcd.early_writes, 0);
}

#[test]
fn nothing_reaches_the_lcd_while_port_b_is_an_input() {
    let mut m = machine(roms::ORIGINAL, false);
    m.board.lcd = Some(Hd44780::new(LCD_16X2));
    // the monitor leaves port B as an input; writes to PB don't drive the pins
    use mikrokit_emu::cpu::Bus;
    m.board.output(0x40, 0x00, 0);
    for v in [0x38, 0x30] {
        m.board.output(0x42, v, 0);
    }
    assert!(!m.board.lcd.as_ref().unwrap().initialized);
}

#[test]
fn basic_example() {
    let mut m = machine(roms::BASIC, true);
    m.board.lcd = Some(Hd44780::new(LCD_16X2));
    let say = |m: &mut Machine, s: &str, ms: u64| {
        m.board.uart.type_chars(s.as_bytes());
        m.run_for(ms * MS, &[]);
    };
    say(&mut m, "B", 300);
    for line in example("lcd.bas").lines() {
        say(&mut m, &format!("{line}\r"), 150);
    }
    say(&mut m, "RUN\r", 4000);
    let lcd = m.board.lcd.as_ref().unwrap();
    assert_eq!(&lcd.row_text(0)[..8], "MIKROKIT");
    let n: u32 = lcd.row_text(1).trim().parse().unwrap_or_else(|_| panic!("{:?}", lcd.row_text(1)));
    m.run_for(2000 * MS, &[]);
    let n2: u32 = m.board.lcd.as_ref().unwrap().row_text(1).trim().parse().unwrap();
    assert!(n2 > n, "the counter runs: {n} then {n2}");
    say(&mut m, "\x03", 300);
}
