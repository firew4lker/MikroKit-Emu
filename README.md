# Microkit 8085 emulator

A cross-platform emulator of the Micro Design **Microkit** 8085 trainer
(Athens, 1985). It runs the original monitor ROM v5.12 unchanged and behaves
like the board: 6-digit 7-segment display, 24-key keypad, the DB9-1 serial
terminal, single step, the VI interrupt key, and HEX downloads over either
serial port.

Written in Rust with [egui](https://github.com/emilk/egui) / eframe, so it
builds on Linux, Windows and macOS.

## Build and run

```bash
cargo run --release
```

The emulator looks for `Monitor Program.BIN` next to the executable, in its
parent folders, and in the working folder (so running it from this folder
finds `../Monitor Program.BIN`). Otherwise use **File → Load monitor ROM**
(`.bin`, `.hex`, or the `.asm` source, which is assembled on the fly).

```
microkit-emu [--rom PATH] [--terminal | --keypad] [--debug] [program.hex|.asm|.bin]
```

## Using it

- **Keypad monitor** (J16 off, the default): click the board, then type
  `0-F`, `M` `R` `G` `S` for M/R/GO/SS, `N` or Space for N, `.` or Enter for
  EX. F9 is RS (reset), F10 is VI. You can also click the keys.
- **Terminal monitor**: switch on *J16 terminal mode* in the bottom bar and
  press RS. Click the terminal and type; commands must be capitals, and
  *Serial → Type in capitals* (on by default) does that for you.
- **Loading programs**: *Load program* (toolbar or File menu), drag and drop,
  or a file on the command line.
  - `.hex` Intel HEX, `.asm` 8085 source (built-in assembler), `.bin` raw
    bytes at an address you choose.
  - The monitor's saved PC is set to the program start, so **GO EX** runs it.
  - End programs with `RST 1` (`CF`) to return to the monitor.
  - **Reload** re-reads the last file after you edit it.
- **The kit's own download**: `GO 1600 EX` (keypad) or `L` (terminal), then
  *Send HEX → DB9-2* (the default port) or *→ DB9-1* after selecting port 1
  with `GO 1800` / `P`.
- **Tape recorder** (📼 Tape): set J18 to tape, then record with
  `GO 1000 EX` (keypad) or `W` (terminal) — press *Record* just before the
  last EX / Return. Read back with `GO 1300 EX` or `R`, pressing *Play* the
  same way. Tapes are saved and loaded as WAV files, so you can also play
  them into a real Microkit, or record a real kit's tape to load here.
- **I/O ports** (🔌 I/O ports): the 8155's ports A, B and C on connector X2
  shown as LEDs (outputs) or clickable switches (inputs), following the
  8155's command register, plus the X3 inputs on port 0FH and the timer.
  Try `examples/ports.asm`.
- **Debug panel**: registers, disassembly, breakpoints, memory view, the
  monitor's saved user registers, 8155 state. Pause and step the CPU.
- **File → Save memory range** writes RAM out as `.hex` or `.bin`.

See `examples/` for small programs.

## How faithful is it

- The CPU counts clock cycles exactly at 3.072 MHz, so the monitor's
  timing-dependent code works unchanged: single step / trace (8155 timer →
  TRAP after one instruction), the 9600-baud software serial port on SID, and
  the tape routines' delay loops.
- Chips emulated: 8085 (including RIM/SIM, all interrupts and the
  undocumented instructions), 8279, 8155 (RAM, ports, timer), 8251, the
  3-bit input port at 0FH (J16 and X3), and the cassette interface.
- Tape: recording turns the SOD square wave into audio through a high-pass
  filter, like the recorder's AC-coupled input. Playback models the board's
  LM324 circuit as an envelope detector (SID = 1 while a tone is present).
  When recording stops, the recorder's stop click is written too: the
  monitor's read routine waits for the next sound after the checksum byte,
  so a perfectly silent tape would leave it waiting forever.
- The terminal holds typed characters until the monitor stops printing,
  because the monitor's output routine reads and discards input that arrives
  while it prints (on the real board, typing ahead loses characters).
  File sends go at full line speed.
- Not yet emulated: the DB9-2 transmit side (SOD as a serial port), the
  8155 handshake modes (ALT3/ALT4) and port interrupts, memory options
  other than 2764 + 6264.

## Tests

```bash
cargo test
```

The tests assemble `../Monitor Program.asm` and check it matches the ROM
byte for byte, then boot the real ROM and drive it: keypad M/GO/R, `-Err`
on ROM writes, single step (including the manual's example), the VI key,
downloads over DB9-1 and DB9-2, a terminal session, and tape write → WAV
→ read round trips from both the keypad and the terminal monitor.
