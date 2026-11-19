"""How the parts are grouped on each sheet. Each block is a list of
references ("IC7" = all units of IC7, "IC7.2" = unit 2) flowed left to
right within `width` mm; blocks are then packed on the page.
"""


def rng(prefix, a, b):
    return [f'{prefix}{i}' for i in range(a, b + 1)]


BLOCKS = {
    'cpu_memory': [
        (90, ['IC1', 'XTAL1']),
        (60, ['J13', 'J1', 'J2', 'J3', 'J4']),
        (110, ['S1', 'R1', 'R2', 'C1', 'S2', 'R4', 'R3', 'C2']),
        (150, ['IC5', 'IC4']),
        (80, ['IC6']),
        (110, ['IC7']),
        (110, ['J7', 'J8', 'J9', 'J10', 'J11']),
        (120, ['IC8', 'IC19']),
        (60, ['J5', 'J6', 'J12']),
        (80, ['IC2']), (80, ['IC3']), (80, ['SC24']),
        (80, ['X1']),
    ],
    'serial_io_tape': [
        (90, ['IC13']),
        (130, ['IC9', 'IC10']),
        (130, ['DB1', 'J15', 'DB2']),
        (140, ['IC16', 'IC15', 'J17']),
        (90, ['IC12']),
        (80, ['X2']),
        (120, ['IC21', 'R40', 'J16', 'X3']),
        (130, ['R28', 'R29', 'IC11.1', 'R30', 'C4', 'C5', 'R31', 'R39', 'R27', 'DIN1']),
        (150, ['R38', 'C7', 'R37', 'IC11.2', 'R36', 'R35', 'R34', 'R33', 'IC11.3', 'D1', 'D2', 'R32', 'C6', 'P1', 'IC11.4', 'IC11.5', 'J18']),
        (90, ['X4', '#FLG01', '#FLG02', '#FLG03', '#FLG04']),
    ],
    'keyboard_display': [
        (110, ['IC18', 'J14']),
        (110, ['IC17', 'IC19']),
        (90, ['IC14']),
        (80, ['IC20']),
        (240, [x for q in range(1, 9) for x in (f'R{q + 4}', f'Q{q}', f'R{q + 18}')]),
        (190, [x for q in range(9, 15) for x in (f'R{q + 4}', f'Q{q}')]),
        (190, rng('DS', 1, 6)),
        (190, rng('S', 3, 24)),
    ],
}
