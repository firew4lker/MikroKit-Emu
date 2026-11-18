//! End-to-end tests that run the real Microkit monitor ROM.
//! They need "../Monitor Program.BIN" and ".asm" next to the crate.

use microkit_emu::asm;
use microkit_emu::loader::{write_ihex, Image};
use microkit_emu::machine::{Key, Machine};
use microkit_emu::segments::display_text;
use std::path::PathBuf;

fn kit_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

fn rom() -> Vec<u8> {
    std::fs::read(kit_file("Monitor Program.BIN")).expect("Monitor Program.BIN not found")
}

const MS: u64 = 3072; // cycles per millisecond

fn boot(terminal: bool) -> Machine {
    let mut m = Machine::new();
    m.load_rom(&rom());
    m.board.jumpers.j16_terminal = terminal;
    m.apply_jumpers();
    m.reset();
    m.run_for(100 * MS, &[]);
    m
}

fn keys(m: &mut Machine, ks: &[Key]) {
    for &k in ks {
        m.press(k);
        m.run_for(20 * MS, &[]);
    }
}

fn hex(s: &str) -> Vec<Key> {
    s.chars().map(|c| Key::Hex(c.to_digit(16).unwrap() as u8)).collect()
}

fn term_out(m: &mut Machine) -> String {
    m.board.uart.tx_out.drain(..).map(|b| b as char).collect()
}

fn type_line(m: &mut Machine, s: &str) -> String {
    m.board.uart.type_chars(s.as_bytes());
    m.run_for(300 * MS, &[]);
    term_out(m)
}

#[test]
fn asm_source_reproduces_rom() {
    let src = std::fs::read_to_string(kit_file("Monitor Program.asm")).unwrap();
    let out = asm::assemble(&src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out.image.to_rom(), rom());
    assert_eq!(out.symbols["WR_LEAD"], 0x10B3);
}

#[test]
fn keypad_boot_shows_sign_on() {
    let m = boot(false);
    assert_eq!(display_text(&m.display()), "- 80|85");
}

#[test]
fn keypad_m_go_r_cycle() {
    let mut m = boot(false);
    // M 2000 N 3E N 47 N CF EX   (MVI A,47H ; RST 1)
    let mut ks = vec![Key::Mem];
    ks.extend(hex("2000"));
    ks.push(Key::Next);
    ks.extend(hex("3E"));
    ks.push(Key::Next);
    ks.extend(hex("47"));
    ks.push(Key::Next);
    ks.extend(hex("CF"));
    ks.push(Key::Ex);
    keys(&mut m, &ks);
    assert_eq!(&m.board.ram[..3], &[0x3E, 0x47, 0xCF]);
    assert_eq!(display_text(&m.display()), "-   |  ");

    let mut ks = vec![Key::Go];
    ks.extend(hex("2000"));
    ks.push(Key::Ex);
    keys(&mut m, &ks);
    assert_eq!(display_text(&m.display()), "- 80|85");
    assert_eq!(m.board.peek(0x40E7), 0x47, "saved A");
    assert_eq!(m.board.peek(0x40EB), 0x03, "saved PC low");

    keys(&mut m, &[Key::Reg]);
    assert_eq!(display_text(&m.display()), "   A|47");
}

#[test]
fn writing_rom_gives_err() {
    let mut m = boot(false);
    let mut ks = vec![Key::Mem];
    ks.extend(hex("0000"));
    ks.push(Key::Next);
    ks.extend(hex("12"));
    ks.push(Key::Next);
    keys(&mut m, &ks);
    assert_eq!(display_text(&m.display()), "-Err|  ");
}

#[test]
fn single_step_executes_one_instruction() {
    let mut m = boot(false);
    // 2000: MVI A,01 ; INR A ; INR A ; RST 1
    for (i, b) in [0x3E, 0x01, 0x3C, 0x3C, 0xCF].iter().enumerate() {
        m.board.poke(0x2000 + i as u16, *b);
    }
    let mut ks = vec![Key::Step];
    ks.extend(hex("2000"));
    ks.push(Key::Next);
    keys(&mut m, &ks);
    assert_eq!(display_text(&m.display()), "2002|3C");
    keys(&mut m, &[Key::Next]);
    assert_eq!(display_text(&m.display()), "2003|3C");
    assert_eq!(m.board.peek(0x40E7), 0x02, "A after one INR");
    keys(&mut m, &[Key::Ex]);
    assert_eq!(display_text(&m.display()), "-   |  ");
}

#[test]
fn single_step_through_monitor_example() {
    // Manual p.38: SS 0008 N N N ... steps through the warm start code
    let mut m = boot(false);
    let mut ks = vec![Key::Step];
    ks.extend(hex("0008"));
    ks.push(Key::Next);
    keys(&mut m, &ks);
    assert_eq!(display_text(&m.display()), "000b|E1");
    keys(&mut m, &[Key::Next]);
    assert_eq!(display_text(&m.display()), "000C|22");
    keys(&mut m, &[Key::Next]);
    assert_eq!(display_text(&m.display()), "000F|F5");
}

#[test]
fn vi_key_jumps_through_40df() {
    let mut m = boot(false);
    // 40DF: JMP 2100 ; 2100: MVI A,99 ; RST 1 ; 2000: MVI A,0B ; SIM ; EI ; JMP 2005
    for (a, b) in [(0x40DF, 0xC3), (0x40E0, 0x00), (0x40E1, 0x21)] {
        m.board.poke(a, b);
    }
    for (i, b) in [0x3E, 0x99, 0xCF].iter().enumerate() {
        m.board.poke(0x2100 + i as u16, *b);
    }
    for (i, b) in [0x3E, 0x0B, 0x30, 0xFB, 0x00, 0xC3, 0x05, 0x20].iter().enumerate() {
        m.board.poke(0x2000 + i as u16, *b);
    }
    let mut ks = vec![Key::Go];
    ks.extend(hex("2000"));
    ks.push(Key::Ex);
    keys(&mut m, &ks);
    assert_eq!(display_text(&m.display()), "E   |  ");
    keys(&mut m, &[Key::Vi]);
    assert_eq!(display_text(&m.display()), "- 80|85");
    assert_eq!(m.board.peek(0x40E7), 0x99);
}

#[test]
fn keypad_download_over_db9_2() {
    let mut m = boot(false);
    let mut ks = vec![Key::Go];
    ks.extend(hex("1600"));
    ks.push(Key::Ex);
    keys(&mut m, &ks);
    assert_eq!(display_text(&m.display()), "L0Ad|  ");
    let data: Vec<u8> = (0..40u8).map(|i| i.wrapping_mul(7)).collect();
    m.board.db9_2.send(write_ihex(0x2100, &data).as_bytes());
    m.run_for(1500 * MS, &[]);
    assert_eq!(&m.board.ram[0x100..0x128], &data[..]);
    assert_eq!(display_text(&m.display()), "End |  ");
}

#[test]
fn crt_monitor_session() {
    let mut m = boot(true);
    let out = term_out(&mut m);
    assert!(out.contains("MICROKIT MONITOR Ver. 5.12"), "got {out:?}");
    assert_eq!(display_text(&m.display()), "Crt |  ");

    let img = Image::from_bin(&[0x3E, 0x47, 0xCF], 0x2000);
    for (&a, &b) in &img.bytes {
        m.board.poke(a, b);
    }
    let out = type_line(&mut m, "D2000,2002\r");
    assert!(out.contains("2000 3E 47 CF"), "got {out:?}");
    let out = type_line(&mut m, "G2000\r");
    assert!(out.contains("PC="), "got {out:?}");
    let out = type_line(&mut m, "X\r");
    assert!(out.contains("A=47"), "got {out:?}");
    let out = type_line(&mut m, "Q");
    assert!(out.contains("Q*"), "got {out:?}");
}

#[test]
fn crt_trace_command() {
    let mut m = boot(true);
    term_out(&mut m);
    for (i, b) in [0x3E, 0x01, 0x3C, 0xCF].iter().enumerate() {
        m.board.poke(0x2000 + i as u16, *b);
    }
    let out = type_line(&mut m, "T2000,");
    assert!(out.contains("PC=2002 3C"), "got {out:?}");
    let out = type_line(&mut m, ",");
    assert!(out.contains("PC=2003 CF"), "got {out:?}");
}

#[test]
fn crt_load_over_db9_1() {
    let mut m = boot(true);
    term_out(&mut m);
    let out = type_line(&mut m, "P1");
    assert!(out.contains("SELECT"), "got {out:?}");
    m.run_for(50 * MS, &[]);
    term_out(&mut m);
    type_line(&mut m, "L");
    let data: Vec<u8> = (0..64u8).collect();
    m.board.uart.receive(write_ihex(0x2400, &data).as_bytes());
    m.run_for(1500 * MS, &[]);
    let out = term_out(&mut m);
    assert!(out.contains("END OF TRANSMISSION"), "got {out:?}");
    assert_eq!(&m.board.ram[0x400..0x440], &data[..]);
}
