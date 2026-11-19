# Notes on the KiCad redraw

The project was redrawn from the three scanned sheets of the original
schematic ("MIKROKIT V2.0", Micro Design Ltd, 7-3-1985). Reference
designators, jumper numbers and connector pin numbers follow the original.

The drawing is a **labelled-net** schematic: every pin has a short wire and
a net label, so connections are made by name (as the original does with
its address, data and control buses), not by long wires. Labels with a box
are global (they continue on another sheet); plain labels stay on their
sheet.

## How it was checked

- KiCad's electrical rules check: 0 errors, 0 warnings.
- `tools/verify.py` exports the netlist from KiCad and compares it with
  the connections entered in `tools/circuit.py` (207 nets, all match).
- The same script simulates the address decoding from those connections.
  It gives the memory and I/O map that the original monitor ROM uses:

  | Selected                 | Addresses           | Monitor ROM uses     |
  |--------------------------|---------------------|----------------------|
  | 2764 EPROM (IC2)         | 0000–1FFF           | ROM at 0000–1FFF     |
  | 6264 RAM (IC3)           | 2000–3FFF           | user RAM             |
  | 2732 socket SC24         | 1000–1FFF           |                      |
  | 8155 RAM (IC12)          | 4000–47FF (8 copies of 256 bytes) | 4000–40FF |
  | 8155 ports               | I/O 40–47           | 40–45                |
  | J16 input (/CS2)         | I/O 0E–0F, D0       | MONSEL = 0FH         |
  | 8251 (/CS3)              | I/O 1E–1F           | 1EH data, 1FH status |
  | 8279 (/CS4)              | I/O 2E–2F           | 2EH data, 2FH control|
  | X3 inputs (/CS5)         | I/O 3E–3F, D0–D2    |                      |

  (Jumpers as fitted for a 2764 + 6264: J7–J11 and J14 on 1-2,
  J5, J6, J12 on 2-3.) With a 2764 in IC2, a chip in SC24 would clash
  with it at 1000–1FFF; SC24 is meant for use with a smaller IC2
  (for example a 2732 at 0000–0FFF with J8 on 2-3).

## Read with less than full certainty

These spots were hard to read on the scan, or were inferred. They are the
first things to check on a real board with a continuity tester.

1. **IC6 (74LS138) inputs C and G2B.** The two lines are hard to follow
   on the scan. They are drawn here as C = IO/M (pin 3) and
   G2B = RESET OUT (pin 5); this is the only assignment that produces
   /MEMR, /MEMW, /IOR and /IOW on Y1, Y2, Y5 and Y6 as labelled.
2. **NAND input order.** IC8 pins 12/13 (J10/J9) and IC19 pins 12/13
   (A12/J11) may be swapped. NAND inputs are interchangeable, so the logic
   is the same either way.
3. **RS-232 chip supplies.** The power pins of IC9 (MC1488) and IC10
   (MC1489) are not drawn. Assumed: MC1488 pin 14 = +12V, pin 1 = −12V;
   MC1489 pin 14 = +5V; pin 7 = GND on both.
4. **MC1488 gate for DB9-2.** The original draws this gate with pin 3 as
   the input and pin 2 as the output. On the MC1488, pin 2 is the input and
   pin 3 the output of that gate, so here SOD drives pin 2 and pin 3 drives
   DB9-2 pin 3 (TXD2).
5. **Power pins of all ICs.** Not drawn on the original; the standard
   pinouts are used. No decoupling capacitors are drawn on the original
   either, so none are included.
6. **RXD2 to J18.** Part of the line from IC10 pin 3 to J18 pin 3 is hidden
   behind other lines; it is the only line that reaches J18 pin 3.
7. **7-segment displays.** The part number is not given. The pinout is
   taken from the drawing: segments A=14, B=13, C=8, D=7, E=6, F=1, G=2,
   DP=9, cathodes on 4 and 12 (common cathode). No footprint is assigned.
8. **DB9 pin numbers** are as printed on the schematic (TXD on 3, RXD on 2,
   /RTS on 5, /CTS on 4, ground on 7). This is not the usual PC DB9 pinout,
   so check it before connecting a PC.
9. **J17** is drawn as two columns: pins 1–11 go to the CD4040 outputs
   Q1–Q11, and pins 12–22 are all joined to the 8251 clock inputs (TXC, RXC).
10. **DIN1 (tape)**: pins 1+2 are the playback input (EAR), 4+5 the record
    output (MIC), 3 is ground. No footprint is assigned (the KiCad library
    has no 5-pin audio DIN socket).
11. **Footprints** are standard through-hole parts chosen to match the
    usual packages (DIP, TO-92, axial resistors). They are not taken from
    the original board.

## Differences from the emulator (fixed)

The redraw showed two details that MikroKit-Emu did differently; both are
fixed in the emulator now:

- The X3 inputs are read through IC21 at **ports 3EH/3FH**, bits D0–D2
  (enabled by /CS5). The emulator had them on port 0FH, which on the board
  has only J16 on D0.
- The 8155 RAM is decoded at **4000–47FF** (A8–A10 not decoded), so it
  appears 8 times. The emulator repeated it only up to 43FF.

The emulator's I/O decoding now follows the schematic exactly. The monitor
ROM uses neither detail, so it ran correctly before too.
