//! The monitor + Tiny BASIC ROM (roms/monitor-basic.asm).

use mikrokit_emu::asm;
use mikrokit_emu::machine::{Key, Machine};
use mikrokit_emu::segments::display_text;
use std::path::PathBuf;

const MS: u64 = 3072;

fn rom() -> (Vec<u8>, std::collections::HashMap<String, u16>) {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("roms/monitor-basic.asm");
    let out = asm::assemble(&std::fs::read_to_string(p).unwrap()).unwrap_or_else(|e| panic!("{e}"));
    (out.image.to_rom(), out.symbols)
}

fn boot(terminal: bool) -> Machine {
    let mut m = Machine::new();
    m.load_rom(&rom().0);
    m.board.jumpers.j16_terminal = terminal;
    m.apply_jumpers();
    m.reset();
    m.run_for(100 * MS, &[]);
    m
}

fn out(m: &mut Machine) -> String {
    m.board.uart.tx_out.drain(..).map(|b| b as char).collect()
}

/// Type a line (like a person would) and return what the kit printed.
fn say(m: &mut Machine, s: &str, ms: u64) -> String {
    m.board.uart.type_chars(s.as_bytes());
    let mut t = String::new();
    for _ in 0..ms / 10 {
        m.run_for(10 * MS, &[]);
        t.push_str(&out(m));
    }
    t
}

fn basic() -> Machine {
    let mut m = boot(true);
    out(&mut m);
    let t = say(&mut m, "B", 300);
    assert!(t.contains("TINY BASIC 2.0"), "{t:?}");
    assert!(t.ends_with("OK\r\n>"), "{t:?}");
    m
}

fn program(m: &mut Machine, lines: &[&str]) {
    for l in lines {
        say(m, &format!("{l}\r"), 150);
    }
}

#[test]
fn rom_layout() {
    let (rom, sym) = rom();
    assert_eq!(rom.len(), 0x2000);
    assert!(sym["TBENDA"] <= 0x2000, "part 1 ends at {:04X}", sym["TBENDA"]);
    assert!(sym["TBENDB"] <= 0x1300, "part 2 ends at {:04X}", sym["TBENDB"]);
    // the keypad-monitor programs stay at their documented addresses
    assert_eq!(sym["KWRITE"], 0x1000);
    assert_eq!(sym["KREAD"], 0x1300);
    assert_eq!(sym["KDNLD"], 0x1600);
    assert_eq!(sym["KEYBR_PORT_CMD"], 0x1800);
    assert_eq!(sym["WR_LEAD"], 0x10B3);
}

#[test]
fn keypad_monitor_unchanged() {
    let mut m = boot(false);
    assert_eq!(display_text(&m.display()), "- 80|85");
    for (i, b) in [0x3E, 0x01, 0x3C, 0xCF].iter().enumerate() {
        m.board.poke(0x2000 + i as u16, *b);
    }
    for k in [Key::Step, Key::Hex(2), Key::Hex(0), Key::Hex(0), Key::Hex(0), Key::Next] {
        m.press(k);
        m.run_for(20 * MS, &[]);
    }
    assert_eq!(display_text(&m.display()), "2002|3C", "single step still works");
}

#[test]
fn terminal_monitor_unchanged_and_help_lists_b() {
    let mut m = boot(true);
    let t = out(&mut m);
    assert!(t.contains("MIKROKIT MONITOR Ver. 5.12 + TINY BASIC"), "{t:?}");
    m.board.poke(0x2000, 0x3E);
    m.board.poke(0x2001, 0x47);
    m.board.poke(0x2002, 0xCF);
    let t = say(&mut m, "D2000,2002\r", 300);
    assert!(t.contains("2000 3E 47 CF"), "{t:?}");
    let t = say(&mut m, "G2000\r", 300);
    assert!(t.contains("PC="), "{t:?}");
    let t = say(&mut m, "X\r", 300);
    assert!(t.contains("A=47"), "{t:?}");
    let t = say(&mut m, "H", 1500);
    let t = t + &say(&mut m, " ", 2500);
    assert!(t.contains("Tiny BASIC (BYE returns):  B"), "{t:?}");
    assert!(t.ends_with("\n\n."), "help ends cleanly, got {:?}", &t[t.len().saturating_sub(40)..]);
}

#[test]
fn basic_loops_and_print() {
    let mut m = basic();
    program(&mut m, &["10 FOR I=1 TO 5", "20 PRINT I*I,", "30 NEXT I", "40 PRINT", "50 PRINT -7/2, 7-10, (2+3)*4"]);
    let t = say(&mut m, "RUN\r", 1500);
    assert!(t.contains("     1     4     9    16    25"), "{t:?}");
    assert!(t.contains("    -3    -3    20"), "{t:?}");
    let t = say(&mut m, "LIST\r", 1500);
    assert!(t.contains("  20 PRINT I*I,"), "{t:?}");
}

#[test]
fn basic_gosub_if_input() {
    let mut m = basic();
    program(
        &mut m,
        &[
            "10 INPUT N",
            "20 GOSUB 100",
            "30 PRINT 'SQUARE IS ',S",
            "40 IF N>5 PRINT 'BIG'",
            "50 STOP",
            "100 S=N*N",
            "110 RETURN",
        ],
    );
    let t = say(&mut m, "RUN\r", 500);
    assert!(t.contains("N:"), "{t:?}");
    let t = say(&mut m, "12\r", 1000);
    assert!(t.contains("SQUARE IS    144"), "{t:?}");
    assert!(t.contains("BIG"), "{t:?}");
}

#[test]
fn basic_peek_poke_in_out() {
    let mut m = basic();
    say(&mut m, "POKE 12288,77\r", 500);
    assert_eq!(m.board.peek(0x3000), 77);
    let t = say(&mut m, "PRINT PEEK(12288)+1\r", 500);
    assert!(t.contains("78"), "{t:?}");
    // 8155: port A as output, write AAH, read it back
    say(&mut m, "OUT 64,1\r", 500);
    say(&mut m, "OUT 65,170\r", 500);
    assert_eq!(m.board.pio.out[0], 170);
    let t = say(&mut m, "PRINT IN(65)\r", 500);
    assert!(t.contains("170"), "{t:?}");
    // port B is an input: switches read as FFH
    let t = say(&mut m, "PRINT IN(66)\r", 500);
    assert!(t.contains("255"), "{t:?}");
}

#[test]
fn basic_errors_ctrl_c_and_bye() {
    let mut m = basic();
    let t = say(&mut m, "PRINT 1+\r", 500);
    assert!(t.contains("WHAT?"), "{t:?}");
    let t = say(&mut m, "PRINT 1/0\r", 500);
    assert!(t.contains("HOW?"), "{t:?}");
    program(&mut m, &["10 GOTO 10"]);
    say(&mut m, "RUN\r", 500);
    let t = say(&mut m, "\x03", 500);
    assert!(t.contains("OK"), "Ctrl-C stops the program: {t:?}");
    let t = say(&mut m, "BYE\r", 500);
    assert!(t.ends_with("\r\n."), "back at the monitor prompt: {t:?}");
    let t = say(&mut m, "B", 500);
    assert!(t.contains("OK"), "{t:?}");
    let t = say(&mut m, "LIST\r", 500);
    assert!(t.contains("  10 GOTO 10"), "program kept after BYE: {t:?}");
    let t = say(&mut m, "NEW\r", 300) + &say(&mut m, "LIST\r", 300);
    assert!(!t.contains("GOTO"), "{t:?}");
}

#[test]
fn basic_rnd_and_size() {
    let mut m = basic();
    program(&mut m, &["10 FOR I=1 TO 20", "20 R=RND(6)", "30 IF R<1 PRINT 'LOW'", "40 IF R>6 PRINT 'HIGH'", "50 NEXT I", "60 PRINT SIZE"]);
    let t = say(&mut m, "RUN\r", 2000);
    assert!(!t.contains("LOW") && !t.contains("HIGH"), "{t:?}");
    let size: i32 = t.split_whitespace().filter_map(|w| w.parse().ok()).last().unwrap();
    assert!(size > 7000 && size < 7936, "free bytes {size}");
}

#[test]
fn downloads_and_tape_still_work() {
    use mikrokit_emu::loader::write_ihex;
    // keypad GO 1600 over DB9-2
    let mut m = boot(false);
    for k in [Key::Go, Key::Hex(1), Key::Hex(6), Key::Hex(0), Key::Hex(0), Key::Ex] {
        m.press(k);
        m.run_for(20 * MS, &[]);
    }
    let data: Vec<u8> = (0..40u8).collect();
    m.board.db9_2.send(write_ihex(0x2100, &data).as_bytes());
    m.run_for(1500 * MS, &[]);
    assert_eq!(&m.board.ram[0x100..0x128], &data[..]);
    assert_eq!(display_text(&m.display()), "End |  ");

    // terminal W then R through the tape
    let mut m = boot(true);
    m.board.jumpers.j18_db9_2 = false;
    out(&mut m);
    for i in 0..16u16 {
        m.board.poke(0x2200 + i, 0xA0 + i as u8);
    }
    let step = |m: &mut Machine, s: &str, ms: u64| {
        m.board.uart.type_chars(s.as_bytes());
        for _ in 0..ms / 10 {
            m.run_for(10 * MS, &[]);
            m.periodic();
        }
    };
    step(&mut m, "W", 300);
    step(&mut m, "2200\r", 300);
    step(&mut m, "220F\r", 300);
    m.tape_record();
    step(&mut m, "\r", 6000);
    m.tape_stop();
    for i in 0..16u16 {
        m.board.poke(0x2200 + i, 0);
    }
    m.board.tape.rewind(m.now());
    out(&mut m);
    step(&mut m, "R", 300);
    m.tape_play();
    step(&mut m, "\r", 9000);
    let t = out(&mut m);
    assert!(!t.contains("READING ERROR") && t.contains("END ADDRESS   :220F"), "{t:?}");
    assert_eq!(m.board.ram[0x20F], 0xAF);
}

#[test]
fn committed_bin_matches_source() {
    let bin = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("roms/monitor-basic.bin")).unwrap();
    assert!(bin == rom().0, "roms/monitor-basic.bin is out of date: rebuild it with --assemble");
}

#[test]
fn leds_example_drives_port_a() {
    let mut m = basic();
    let src = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/leds.bas")).unwrap();
    for line in src.lines() {
        say(&mut m, &format!("{line}\r"), 150);
    }
    say(&mut m, "RUN\r", 300);
    let a = m.board.pio.out[0];
    m.run_for(300 * MS, &[]);
    let b = m.board.pio.out[0];
    assert_eq!(m.board.pio.csr, 1);
    assert_ne!(a, b, "the LED count moves");
    let t = say(&mut m, "\x03", 300);
    assert!(t.contains("OK"), "{t:?}");
}
