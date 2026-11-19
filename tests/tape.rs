//! Tape write → read round trips through the monitor's own routines.

use mikrokit_emu::machine::{Key, Machine};
use mikrokit_emu::segments::display_text;
use mikrokit_emu::tape::Tape;

const MS: u64 = 3072;

fn boot(terminal: bool) -> Machine {
    let mut m = Machine::new();
    m.load_rom(mikrokit_emu::roms::ORIGINAL);
    m.board.jumpers.j16_terminal = terminal;
    m.board.jumpers.j18_db9_2 = false; // J18 1-2: tape
    m.apply_jumpers();
    m.reset();
    m.run_for(100 * MS, &[]);
    m
}

fn run(m: &mut Machine, ms: u64) {
    for _ in 0..ms / 10 {
        m.run_for(10 * MS, &[]);
        m.periodic();
    }
}

fn keys(m: &mut Machine, s: &str) {
    for c in s.chars() {
        let k = match c {
            'M' => Key::Mem,
            'G' => Key::Go,
            'N' => Key::Next,
            '.' => Key::Ex,
            d => Key::Hex(d.to_digit(16).unwrap() as u8),
        };
        m.press(k);
        run(m, 20);
    }
}

#[test]
fn keypad_tape_write_then_read() {
    let mut m = boot(false);
    let data: Vec<u8> = (0..32u8).map(|i| i.wrapping_mul(37) ^ 0x5A).collect();
    for (i, &b) in data.iter().enumerate() {
        m.board.poke(0x2000 + i as u16, b);
    }
    keys(&mut m, "G1000.");
    assert_eq!(display_text(&m.display()), "5 Ad|  ");
    keys(&mut m, "2000.");
    assert_eq!(display_text(&m.display()), "E Ad|  ");
    keys(&mut m, "201F.");
    assert_eq!(display_text(&m.display()), "5 rC|  ");
    m.tape_record();
    keys(&mut m, ".");
    run(&mut m, 14000);
    m.tape_stop();
    assert_eq!(display_text(&m.display()), "-   |  ", "back at the prompt after writing");
    let secs = m.board.tape.seconds(m.board.tape.len() as f64);
    assert!(secs > 4.0 && secs < 20.0, "tape length {secs}");

    // through a WAV file, like saving and loading it
    let wav = m.board.tape.to_wav();
    m.board.tape = Tape::from_wav(&wav).unwrap();

    for i in 0..32 {
        m.board.poke(0x2000 + i, 0);
    }
    m.board.tape.rewind(m.now());
    keys(&mut m, "G1300.");
    assert_eq!(display_text(&m.display()), "5 rC|  ");
    m.tape_play();
    keys(&mut m, ".");
    run(&mut m, 25000);
    assert_eq!(&m.board.ram[..32], &data[..], "data read back from tape");
    assert_eq!(display_text(&m.display()), "-   |  ", "no -Err after reading");
}

#[test]
fn terminal_tape_write_then_read() {
    let mut m = boot(true);
    let data: Vec<u8> = (0..16u8).map(|i| 0xF0 - i).collect();
    for (i, &b) in data.iter().enumerate() {
        m.board.poke(0x2100 + i as u16, b);
    }
    let mut out = String::new();
    fn typ(m: &mut Machine, out: &mut String, s: &str, ms: u64) {
        m.board.uart.type_chars(s.as_bytes());
        run(m, ms);
        out.extend(m.board.uart.tx_out.drain(..).map(|b| b as char));
    }
    typ(&mut m, &mut out, "W", 300);
    typ(&mut m, &mut out, "2100\r", 300);
    typ(&mut m, &mut out, "210F\r", 300);
    m.tape_record();
    typ(&mut m, &mut out, "\r", 8000);
    m.tape_stop();
    assert!(out.contains("END OF WRITING"), "{out}");
    for i in 0..16 {
        m.board.poke(0x2100 + i, 0);
    }
    m.board.tape.rewind(m.now());
    out.clear();
    typ(&mut m, &mut out, "R", 300);
    m.tape_play();
    typ(&mut m, &mut out, "\r", 10000);
    assert!(out.contains("READING FROM TAPE"), "{out}");
    assert!(out.contains("END ADDRESS   :210F"), "{out}");
    assert!(!out.contains("READING ERROR"), "{out}");
    assert_eq!(&m.board.ram[0x100..0x110], &data[..]);
}
