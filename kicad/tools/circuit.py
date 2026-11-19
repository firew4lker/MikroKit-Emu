"""The Mikrokit V2.0 circuit, read from the scanned schematic (Micro Design,
7-3-1985, sheets 1-3). One entry per symbol unit: pin number -> net.
Pins left out are marked no-connect. Net names follow the original
labels; ~{X} is KiCad's overbar (active low).
"""

GND, VCC = 'GND', '+5V'
FP = {
    'R': 'Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P10.16mm_Horizontal',
    'C': 'Capacitor_THT:C_Disc_D5.0mm_W2.5mm_P5.00mm',
    'D': 'Diode_THT:D_DO-35_SOD27_P7.62mm_Horizontal',
    'Q': 'Package_TO_SOT_THT:TO-92_Inline',
    'J2': 'Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical',
    'J3': 'Connector_PinHeader_2.54mm:PinHeader_1x03_P2.54mm_Vertical',
    'SW': 'Button_Switch_THT:SW_PUSH_6mm',
    'DIP14': 'Package_DIP:DIP-14_W7.62mm',
    'DIP16': 'Package_DIP:DIP-16_W7.62mm',
    'DIP20': 'Package_DIP:DIP-20_W7.62mm',
}


class P:
    def __init__(self, ref, lib, value, conns, unit=1, fp=None):
        self.ref, self.lib, self.value, self.unit, self.fp = ref, lib, value, unit, fp
        self.conns = {str(k): v for k, v in conns.items()}


def R(ref, value, a, b):
    return P(ref, 'Device:R', value, {1: a, 2: b}, fp=FP['R'])


def C(ref, value, a, b):
    return P(ref, 'Device:C', value, {1: a, 2: b}, fp=FP['C'])


def J2(ref, a, b, value='Jumper'):
    return P(ref, 'Jumper:Jumper_2_Open', value, {1: a, 2: b}, fp=FP['J2'])


def J3(ref, p1, p2, p3, value='Jumper'):
    # pin 2 is the middle pin (the common one) on the board and in the symbol
    return P(ref, 'Jumper:Jumper_3_Open', value, {1: p1, 2: p2, 3: p3}, fp=FP['J3'])


def gate_power(ref, lib, unit, vcc=14, gnd=7, fp=FP['DIP14']):
    return P(ref, lib, lib.split(':')[1], {vcc: VCC, gnd: GND}, unit=unit, fp=fp)


AD = [f'AD{i}' for i in range(8)]
A = [f'A{i}' for i in range(16)]
AB = {i: f'AB{i}' for i in range(8, 16)}

# ----------------------------------------------------------------------------
# Sheet 1: CPU, address latch and buffer, control decoder, memory, X1
# ----------------------------------------------------------------------------
cpu = {1: 'XTAL1', 2: 'XTAL2', 3: 'RESET', 4: 'SOD', 5: 'SID', 6: 'CPU_TRAP', 7: 'RST7.5', 8: 'RST6.5', 9: 'RST5.5',
       10: 'INTR', 30: 'ALE', 31: '~{WR}', 32: '~{RD}', 34: 'IO~{M}', 35: VCC, 36: '~{RSTIN}', 37: 'CLK', 39: 'HOLD',
       20: GND, 40: VCC}
cpu.update({12 + i: AD[i] for i in range(8)})
cpu.update({21 + i: AB[8 + i] for i in range(8)})

latch = {1: GND, 11: 'ALE', 10: GND, 20: VCC}
for d, q, i in zip([3, 4, 7, 8, 13, 14, 17, 18], [2, 5, 6, 9, 12, 15, 16, 19], range(8)):
    latch[d] = AD[i]
    latch[q] = A[i]

buf = {1: GND, 19: GND, 10: GND, 20: VCC}
for i_pin, o_pin, n in [(2, 18, 15), (4, 16, 14), (6, 14, 13), (8, 12, 12), (11, 9, 8), (13, 7, 9), (15, 5, 10), (17, 3, 11)]:
    buf[i_pin] = AB[n]
    buf[o_pin] = A[n]


def mem_addr(pins):
    # A0..A10 and A12 (A11 goes through a jumper)
    return {p: A[i] for i, p in enumerate(pins) if i != 11}


ADDR28 = [10, 9, 8, 7, 6, 5, 4, 3, 25, 24, 21, 23, 2]
DATA28 = [11, 12, 13, 15, 16, 17, 18, 19]
rom = mem_addr(ADDR28) | {23: 'ROM_A11', 20: '~{CS_ROM}', 22: '~{MEMR}', 27: VCC, 1: VCC, 26: VCC, 14: GND, 28: VCC}
rom.update({p: AD[i] for i, p in enumerate(DATA28)})
ram = mem_addr(ADDR28) | {23: 'RAM_A11', 20: '~{CS_RAM}', 26: VCC, 22: '~{MEMR}', 27: '~{MEMW}', 1: VCC, 14: GND, 28: VCC}
ram.update({p: AD[i] for i, p in enumerate(DATA28)})
ADDR24 = [8, 7, 6, 5, 4, 3, 2, 1, 23, 22, 19, 21]
sc24 = {p: A[i] for i, p in enumerate(ADDR24) if i != 11} | {21: 'SC24_A11', 18: '~{CS_SC24}', 20: '~{MEMR}', 12: GND, 24: VCC}
sc24.update({p: AD[i] for i, p in enumerate([9, 10, 11, 13, 14, 15, 16, 17])})

x1 = {33: 'AD0', 34: 'AD1', 31: 'AD2', 32: 'AD3', 29: 'AD4', 30: 'AD5', 27: 'AD6', 28: 'AD7',
      25: 'A0', 26: 'A1', 23: 'A2', 24: 'A3', 21: 'A4', 22: 'A5', 19: 'A6', 20: 'A7',
      17: 'A8', 18: 'A9', 15: 'A10', 16: 'A11', 13: 'A12', 14: 'A13', 11: 'A14', 12: 'A15',
      8: 'IO~{M}', 7: 'ALE', 5: 'RESET', 10: '~{RD}', 9: '~{WR}', 6: 'CLK', 3: 'RST6.5', 1: GND, 2: GND, 4: GND}

SHEET1 = [
    P('IC1', 'mikrokit:8085A', '8085A', cpu),
    P('XTAL1', 'Device:Crystal', '6.144MHz', {1: 'XTAL1', 2: 'XTAL2'}, fp='Crystal:Crystal_HC49-U_Vertical'),
    J2('J13', 'TRAP', 'CPU_TRAP', 'J13 TRAP enable'),
    J2('J1', GND, 'CPU_TRAP'), J2('J2', GND, 'RST6.5'), J2('J3', GND, 'INTR'), J2('J4', GND, 'HOLD'),
    P('S1', 'Switch:SW_Push', 'RS (RESET)', {1: GND, 2: 'RS_SW'}, fp=FP['SW']),
    R('R1', '1K', 'RS_SW', '~{RSTIN}'), R('R2', '47K', VCC, '~{RSTIN}'), C('C1', '100n', '~{RSTIN}', GND),
    P('S2', 'Switch:SW_Push', 'VI (RST7.5)', {1: 'VI_SW', 2: 'RST7.5'}, fp=FP['SW']),
    R('R4', '200', 'VI_SW', GND), R('R3', '3.9K', VCC, 'RST7.5'), C('C2', '100n', 'RST7.5', GND),
    P('IC5', '74xx:74LS373', '74LS373', latch, fp=FP['DIP20']),
    P('IC4', '74xx:74LS244', '74LS244', buf, fp=FP['DIP20']),
    P('IC6', '74xx:74LS138', '74LS138', {1: '~{WR}', 2: '~{RD}', 3: 'IO~{M}', 4: GND, 5: 'RESET', 6: VCC,
                                         14: '~{MEMR}', 13: '~{MEMW}', 10: '~{IOR}', 9: '~{IOW}', 8: GND, 16: VCC}, fp=FP['DIP16']),
    P('IC7', '74xx:74LS02', '74LS02', {2: 'A11', 3: 'A11', 1: '~{A11}'}, 1, FP['DIP14']),
    P('IC7', '74xx:74LS02', '74LS02', {5: 'A12', 6: 'A12', 4: '~{A12}'}, 2, FP['DIP14']),
    P('IC7', '74xx:74LS02', '74LS02', {8: 'A13', 9: 'A13', 10: '~{A13}'}, 3, FP['DIP14']),
    P('IC7', '74xx:74LS02', '74LS02', {11: 'A14', 12: 'A15', 13: 'A14_A15_LOW'}, 4, FP['DIP14']),
    gate_power('IC7', '74xx:74LS02', 5),
    J3('J7', VCC, 'J7_SEL', '~{A11}'), J3('J8', VCC, 'J8_SEL', '~{A12}'),
    J3('J9', VCC, 'J9_SEL', '~{A11}'), J3('J10', VCC, 'J10_SEL', '~{A12}'), J3('J11', VCC, 'J11_SEL', '~{A11}'),
    P('IC8', '74xx:74LS20', '74LS20', {1: 'J7_SEL', 2: 'J8_SEL', 4: '~{A13}', 5: 'A14_A15_LOW', 6: '~{CS_ROM}'}, 1, FP['DIP14']),
    P('IC8', '74xx:74LS20', '74LS20', {9: 'A13', 10: 'A14_A15_LOW', 12: 'J10_SEL', 13: 'J9_SEL', 8: '~{CS_RAM}'}, 2, FP['DIP14']),
    gate_power('IC8', '74xx:74LS20', 3),
    P('IC19', '74xx:74LS20', '74LS20', {9: 'A14_A15_LOW', 10: '~{A13}', 12: 'A12', 13: 'J11_SEL', 8: '~{CS_SC24}'}, 2, FP['DIP14']),
    gate_power('IC19', '74xx:74LS20', 3),
    J3('J5', VCC, 'ROM_A11', 'A11'), J3('J6', '~{MEMW}', 'RAM_A11', 'A11'), J3('J12', VCC, 'SC24_A11', 'A11'),
    P('IC2', 'mikrokit:2764', '2764', rom),
    P('IC3', 'mikrokit:6264', '6264', ram),
    P('SC24', 'mikrokit:2732', '2732 (socket SC24)', sc24),
    P('X1', 'Connector_Generic:Conn_02x17_Odd_Even', 'X1 expansion', x1, fp='Connector_IDC:IDC-Header_2x17_P2.54mm_Vertical'),
]

# ----------------------------------------------------------------------------
# Sheet 2: serial ports, baud rate generator, 8155, input port, tape, power
# ----------------------------------------------------------------------------
usart = {3: 'RXD1_TTL', 4: GND, 9: 'BAUD_CLK', 25: 'BAUD_CLK', 10: '~{IOW}', 11: '~{CS3}', 12: 'A8', 13: '~{IOR}',
         17: 'CTS1_TTL', 19: 'TXD1_TTL', 20: 'CLK', 21: 'RESET', 23: 'RTS1_TTL', 26: VCC}
usart.update({p: AD[i] for i, p in enumerate([27, 28, 1, 2, 5, 6, 7, 8])})

pio = {8: '~{CS1}', 4: 'RESET', 9: '~{RD}', 10: '~{WR}', 7: 'IO~{M}', 11: 'ALE', 3: 'CLK', 6: 'TRAP', 20: GND, 40: VCC}
pio.update({12 + i: AD[i] for i in range(8)})
pio.update({21 + i: f'PA{i}' for i in range(8)})
pio.update({29 + i: f'PB{i}' for i in range(8)})
pio.update({37: 'PC0', 38: 'PC1', 39: 'PC2', 1: 'PC3', 2: 'PC4', 5: 'PC5'})

x2 = {25: 'PA0', 26: 'PA1', 23: 'PA2', 24: 'PA3', 21: 'PA4', 22: 'PA5', 19: 'PA6', 20: 'PA7',
      17: 'PB0', 18: 'PB1', 15: 'PB2', 16: 'PB3', 13: 'PB4', 14: 'PB5', 11: 'PB6', 12: 'PB7',
      9: 'PC0', 10: 'PC1', 7: 'PC2', 8: 'PC3', 5: 'PC4', 6: 'PC5', 2: VCC, 4: VCC, 1: GND, 3: GND}

# CD4040 outputs Q1..Q11 (KiCad names them Q0..Q10) -> J17 pins 1..11
ctr = {10: 'CLK_DIV10', 11: GND, 8: GND, 16: VCC}
for k, p in enumerate([9, 7, 6, 5, 3, 2, 4, 13, 12, 14, 15]):
    ctr[p] = f'BAUD_Q{k + 1}'
j17 = {k + 1: f'BAUD_Q{k + 1}' for k in range(11)}
j17.update({12 + k: 'BAUD_CLK' for k in range(11)})

LM = 'Amplifier_Operational:LM324'
SHEET2 = [
    P('IC13', 'mikrokit:8251A', '8251A', usart),
    P('IC9', 'mikrokit:MC1488', 'MC1488', {2: 'SOD', 3: 'TXD2_RS', 4: 'TXD1_TTL', 5: 'TXD1_TTL', 6: 'TXD1_RS',
                                          9: 'RTS1_TTL', 10: 'RTS1_TTL', 8: 'RTS1_RS', 14: '+12V', 1: '-12V', 7: GND}),
    P('IC10', 'mikrokit:MC1489', 'MC1489', {1: 'RXD2_RS', 3: 'RXD2_TTL', 10: 'RXD1_RS', 8: 'RXD1_TTL',
                                           13: 'CTS1_RS', 11: 'CTS1_TTL', 14: VCC, 7: GND}),
    P('DB1', 'Connector:DE9_Socket', 'DB9-1 (serial port 1)', {3: 'TXD1_RS', 5: 'RTS1_RS', 2: 'RXD1_RS', 4: 'CTS1_RS', 7: GND},
      fp='Connector_Dsub:DSUB-9_Socket_Horizontal_P2.77x2.84mm_EdgePinOffset7.70mm_Housed_MountingHolesOffset9.12mm'),
    J2('J15', 'CTS1_RS', 'RTS1_RS', 'J15 RTS-CTS'),
    P('DB2', 'Connector:DE9_Socket', 'DB9-2 (serial port 2)', {2: 'RXD2_RS', 3: 'TXD2_RS', 7: GND},
      fp='Connector_Dsub:DSUB-9_Socket_Horizontal_P2.77x2.84mm_EdgePinOffset7.70mm_Housed_MountingHolesOffset9.12mm'),
    P('IC16', '74xx:74LS90', '74LS90', {14: 'IC16_QD', 11: 'IC16_QD', 1: 'CLK', 12: 'CLK_DIV10', 2: GND, 3: GND, 6: GND, 7: GND,
                                        5: VCC, 10: GND}, fp=FP['DIP14']),
    P('IC15', '4xxx:4040', 'CD4040', ctr, fp=FP['DIP16']),
    P('J17', 'Connector_Generic:Conn_02x11_Top_Bottom', 'J17 baud rate', j17,
      fp='Connector_PinHeader_2.54mm:PinHeader_2x11_P2.54mm_Vertical'),
    P('IC12', 'mikrokit:8155', '8155', pio),
    P('X2', 'Connector_Generic:Conn_02x13_Odd_Even', 'X2 parallel I/O', x2, fp='Connector_IDC:IDC-Header_2x13_P2.54mm_Vertical'),
    P('IC21', '74xx:74LS125', '74LS125', {1: '~{CS2}', 2: 'J16_IN', 3: 'AD0'}, 1, FP['DIP14']),
    P('IC21', '74xx:74LS125', '74LS125', {4: '~{CS5}', 5: 'X3_1', 6: 'AD0'}, 2, FP['DIP14']),
    P('IC21', '74xx:74LS125', '74LS125', {10: '~{CS5}', 9: 'X3_2', 8: 'AD1'}, 3, FP['DIP14']),
    P('IC21', '74xx:74LS125', '74LS125', {13: '~{CS5}', 12: 'X3_3', 11: 'AD2'}, 4, FP['DIP14']),
    gate_power('IC21', '74xx:74LS125', 5),
    R('R40', '2.7K', VCC, 'J16_IN'), J2('J16', 'J16_IN', GND, 'J16 monitor select'),
    P('X3', 'Connector_Generic:Conn_01x03', 'X3 inputs', {1: 'X3_1', 2: 'X3_2', 3: 'X3_3'}, fp=FP['J3']),
    # cassette interface
    R('R28', '10K', VCC, 'SOD'), R('R29', '20K', 'SOD', 'REC_SUM'),
    P('IC11', LM, 'LM324', {1: 'REC_OUT', 2: 'REC_SUM', 3: 'VREF'}, 1, FP['DIP14']),
    R('R30', '10K', 'REC_SUM', 'REC_OUT'), C('C4', '1n', 'REC_SUM', 'REC_OUT'),
    C('C5', '100n', 'REC_OUT', 'TAPE_MIC'), R('R31', '1K', 'TAPE_MIC', GND),
    R('R39', '1.2K', VCC, 'VREF'), R('R27', '100', 'VREF', GND),
    P('DIN1', 'Connector:DIN-5', 'TAPE', {1: 'TAPE_EAR', 2: 'TAPE_EAR', 3: GND, 4: 'TAPE_MIC', 5: 'TAPE_MIC'},
      fp=''),  # no 5-pin audio DIN footprint in the KiCad library
    R('R38', '100', 'TAPE_EAR', GND), C('C7', '100n', 'TAPE_EAR', 'PB_IN'), R('R37', '1M', 'PB_IN', 'VREF'),
    P('IC11', LM, 'LM324', {5: 'PB_IN', 6: 'PB_FB', 7: 'PB_AMP'}, 2, FP['DIP14']),
    R('R36', '10K', 'PB_FB', 'VREF'), R('R35', '10K', 'PB_FB', 'PB_AMP'),
    R('R34', '10K', 'PB_AMP', 'PB_INV'), R('R33', '10K', 'PB_INV', 'PB_RECT'),
    P('IC11', LM, 'LM324', {9: 'PB_INV', 10: 'VREF', 8: 'PB_RECT'}, 3, FP['DIP14']),
    P('D1', 'Diode:1N4148', '1N4148', {2: 'PB_AMP', 1: 'ENV'}, fp=FP['D']),
    P('D2', 'Diode:1N4148', '1N4148', {2: 'PB_RECT', 1: 'ENV'}, fp=FP['D']),
    R('R32', '12K', 'ENV', GND), C('C6', '100n', 'ENV', GND),
    P('P1', 'Device:R_Potentiometer', '1K', {1: VCC, 2: 'THRESH', 3: GND}, fp='Potentiometer_THT:Potentiometer_Piher_PT-10-V10_Vertical'),
    P('IC11', LM, 'LM324', {12: 'ENV', 13: 'THRESH', 14: 'TAPE_SID'}, 4, FP['DIP14']),
    P('IC11', LM, 'LM324', {4: '+12V', 11: '-12V'}, 5, FP['DIP14']),
    J3('J18', 'TAPE_SID', 'SID', 'RXD2_TTL', 'J18 SID source'),
    P('X4', 'Connector:Screw_Terminal_01x04', 'X4 power supply', {1: VCC, 2: GND, 3: '+12V', 4: '-12V'},
      fp='TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-4-5.08_1x04_P5.08mm_Horizontal'),
    P('#FLG01', 'power:PWR_FLAG', 'PWR_FLAG', {1: VCC}),
    P('#FLG02', 'power:PWR_FLAG', 'PWR_FLAG', {1: GND}),
    P('#FLG03', 'power:PWR_FLAG', 'PWR_FLAG', {1: '+12V'}),
    P('#FLG04', 'power:PWR_FLAG', 'PWR_FLAG', {1: '-12V'}),
]

# ----------------------------------------------------------------------------
# Sheet 3: I/O decoding, 8279, display and keypad
# ----------------------------------------------------------------------------
kbd = {21: 'A8', 22: '~{CS4}', 10: '~{IOR}', 11: '~{IOW}', 3: 'CLK', 9: 'RESET', 4: 'RST5.5', 36: GND, 37: GND,
       32: 'SL0', 33: 'SL1', 34: 'SL2', 20: GND, 40: VCC}
kbd.update({12 + i: AD[i] for i in range(8)})
kbd.update({p: f'DSP{i}' for i, p in enumerate([27, 26, 25, 24, 31, 30, 29, 28])})
kbd.update({p: f'KEY{i}' for i, p in enumerate([38, 39, 1, 2, 5, 6, 7, 8])})

SEGS = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'DP']
SEG_PIN = {'A': 14, 'B': 13, 'C': 8, 'D': 7, 'E': 6, 'F': 1, 'G': 2, 'DP': 9}

SHEET3 = [
    P('IC18', '74xx:74LS138', '74LS138', {1: 'A12', 2: 'A13', 3: 'A14', 6: 'IC18_G1', 4: 'A15', 5: 'A11', 11: '~{CS1}',
                                          8: GND, 16: VCC}, fp=FP['DIP16']),
    J3('J14', VCC, 'IC18_G1', 'IO~{M}', 'J14 8155 RAM enable'),
    P('IC17', '74xx:74LS138', '74LS138', {1: 'A12', 2: 'A13', 3: 'A14', 6: 'IO~{M}', 4: 'A15', 5: '~{A9A10A11}',
                                          15: '~{CS2}', 14: '~{CS3}', 13: '~{CS4}', 12: '~{CS5}', 8: GND, 16: VCC}, fp=FP['DIP16']),
    P('IC19', '74xx:74LS20', '74LS20', {1: VCC, 2: 'A9', 4: 'A10', 5: 'A11', 6: '~{A9A10A11}'}, 1, FP['DIP14']),
    P('IC14', 'mikrokit:8279', '8279', kbd),
    P('IC20', '74xx:74LS156', '74LS156', {13: 'SL0', 3: 'SL1', 1: 'SL2', 15: 'SL2', 2: 'RESET', 14: 'RESET',
                                          7: 'SCAN4', 6: 'SCAN5', 9: 'SCAN0', 10: 'SCAN1', 11: 'SCAN2', 12: 'SCAN3',
                                          8: GND, 16: VCC}, fp=FP['DIP16']),
]
# segment drivers: DSP0..7 -> Q1..Q8 -> segments a..g, dp
for i, seg in enumerate(SEGS):
    q = i + 1
    SHEET3 += [
        R(f'R{q + 4}', '2.7K', f'DSP{i}', f'Q{q}_B'),
        P(f'Q{q}', 'Transistor_BJT:BC557', 'BC557', {2: f'Q{q}_B', 3: VCC, 1: f'Q{q}_C'}, fp=FP['Q']),
        R(f'R{q + 18}', '11', f'Q{q}_C', f'SEG_{seg}'),
    ]
# digit drivers: scan line k -> Q9..Q14 -> DS1..DS6 cathodes
for k in range(6):
    q = 9 + k
    SHEET3 += [
        R(f'R{13 + k}', '100', f'SCAN{k}', f'Q{q}_B'),
        P(f'Q{q}', 'Transistor_BJT:BC557', 'BC557', {2: f'Q{q}_B', 3: f'DIG{k + 1}', 1: GND}, fp=FP['Q']),
        P(f'DS{k + 1}', 'mikrokit:7SEG_CC', '7-segment', {SEG_PIN[s]: f'SEG_{s}' for s in SEGS} | {4: f'DIG{k + 1}', 12: f'DIG{k + 1}'}),
    ]
# keypad: rows are scan lines 0..2, columns are return lines KEY0..7
KEYS = [['0', '1', '2', '3', '4', '5', '6', '7'], ['8', '9', 'A', 'B', 'C', 'D', 'E', 'F'], ['EX', 'N', 'GO', 'M', 'R', 'SS']]
n = 3
for row, names in enumerate(KEYS):
    for col, name in enumerate(names):
        SHEET3.append(P(f'S{n}', 'Switch:SW_Push', f'"{name}"', {1: f'KEY{col}', 2: f'SCAN{row}'}, fp=FP['SW']))
        n += 1

SHEETS = [
    ('cpu_memory', 'CPU and memory', 'sheet 1 of 3', SHEET1),
    ('serial_io_tape', 'Serial ports, 8155 I/O, tape and power', 'sheet 2 of 3', SHEET2),
    ('keyboard_display', 'I/O decoding, keyboard and display', 'sheet 3 of 3', SHEET3),
]
