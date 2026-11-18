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
  3-bit input port at 0FH (J16).
- The terminal holds typed characters until the monitor stops printing,
  because the monitor's output routine reads and discards input that arrives
  while it prints (on the real board, typing ahead loses characters).
  File sends go at full line speed.
- Not yet emulated: cassette tape (SID/SOD audio), the DB9-2 transmit side,
  the 8155 port connector (values are shown in the debug panel), memory
  options other than 2764 + 6264.

## Tests

```bash
cargo test
```

The tests assemble `../Monitor Program.asm` and check it matches the ROM
byte for byte, then boot the real ROM and drive it: keypad M/GO/R, `-Err`
on ROM writes, single step (including the manual's example), the VI key,
downloads over DB9-1 and DB9-2, and a terminal session.
