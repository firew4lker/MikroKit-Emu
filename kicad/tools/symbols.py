"""Custom symbols for the Mikrokit parts that KiCad's libraries lack.

Each IC is a box: pins listed top to bottom on the left and right, power
pins on the top and bottom. `None` leaves a gap.
"""
from sexp import Sym as S

G = 2.54
PIN_LEN = 5.08


def _font():
    return [S('effects'), [S('font'), [S('size'), 1.27, 1.27]]]


def _hidden():
    return [S('effects'), [S('font'), [S('size'), 1.27, 1.27]], [S('hide'), S('yes')]]


def _pin(num, name, typ, x, y, ang):
    return [S('pin'), S(typ), S('line'), [S('at'), x, y, ang], [S('length'), PIN_LEN],
            [S('name'), name, _font()], [S('number'), str(num), _font()]]


def box(name, left, right, top=(), bottom=(), width=20.32, footprint='', description='', ref='IC'):
    """left/right: lists of (number, name, type) or None. top/bottom: lists of (number, name, type)."""
    n = max(len(left), len(right))
    if n % 2 == 0:
        n += 1
    half = (n - 1) / 2
    ys = [(half - i) * G for i in range(n)]
    top_y = ys[0] + G
    bot_y = ys[-1] - G
    w2 = width / 2
    pins = []
    for i, p in enumerate(left):
        if p:
            pins.append(_pin(p[0], p[1], p[2], -(w2 + PIN_LEN), ys[i], 0))
    for i, p in enumerate(right):
        if p:
            pins.append(_pin(p[0], p[1], p[2], w2 + PIN_LEN, ys[i], 180))

    def spread(lst):
        k = len(lst)
        return [((j - (k - 1) / 2) * 2 * G) for j in range(k)]

    for (p, x) in zip(top, spread(top)):
        pins.append(_pin(p[0], p[1], p[2], x, top_y + PIN_LEN, 270))
    for (p, x) in zip(bottom, spread(bottom)):
        pins.append(_pin(p[0], p[1], p[2], x, bot_y - PIN_LEN, 90))
    body = [S('rectangle'), [S('start'), -w2, top_y], [S('end'), w2, bot_y],
            [S('stroke'), [S('width'), 0.254], [S('type'), S('default')]], [S('fill'), [S('type'), S('background')]]]
    ty = top_y + (PIN_LEN + 1.27 if top else 1.27)
    by = bot_y - (PIN_LEN + 1.27 if bottom else 1.27)
    return [S('symbol'), name,
            [S('pin_names'), [S('offset'), 1.016]],
            [S('exclude_from_sim'), S('no')], [S('in_bom'), S('yes')], [S('on_board'), S('yes')],
            [S('property'), 'Reference', ref, [S('at'), -w2, ty, 0], [S('effects'), [S('font'), [S('size'), 1.524, 1.524]], [S('justify'), S('left')]]],
            [S('property'), 'Value', name, [S('at'), -w2, by, 0], [S('effects'), [S('font'), [S('size'), 1.27, 1.27]], [S('justify'), S('left')]]],
            [S('property'), 'Footprint', footprint, [S('at'), 0, 0, 0], _hidden()],
            [S('property'), 'Datasheet', '', [S('at'), 0, 0, 0], _hidden()],
            [S('property'), 'Description', description, [S('at'), 0, 0, 0], _hidden()],
            [S('symbol'), f'{name}_0_1', body],
            [S('symbol'), f'{name}_1_1'] + pins]


IN, OUT, BI, TRI, PWR, PAS, OC = 'input', 'output', 'bidirectional', 'tri_state', 'power_in', 'passive', 'open_collector'


def addr(pins, typ=IN):
    return [(p, f'A{i}', typ) for i, p in enumerate(pins)]


def data(pins, prefix, typ):
    return [(p, f'{prefix}{i}', typ) for i, p in enumerate(pins)]


DIP = 'Package_DIP:DIP-{}_W{}mm'

SYMBOLS = [
    box('8085A',
        [(1, 'X1', IN), (2, 'X2', PAS), None, (36, '~{RESET_IN}', IN), (35, 'READY', IN), (39, 'HOLD', IN), (38, 'HLDA', OUT), None,
         (6, 'TRAP', IN), (7, 'RST7.5', IN), (8, 'RST6.5', IN), (9, 'RST5.5', IN), (10, 'INTR', IN), (11, '~{INTA}', OUT), None,
         (5, 'SID', IN), (4, 'SOD', OUT), None, (37, 'CLK_OUT', OUT), (3, 'RESET_OUT', OUT)],
        data(range(12, 20), 'AD', BI) + [None] + [(21 + i, f'A{8 + i}', TRI) for i in range(8)] + [None]
        + [(30, 'ALE', OUT), (32, '~{RD}', TRI), (31, '~{WR}', TRI), (34, 'IO/~{M}', TRI), (29, 'S0', OUT), (33, 'S1', OUT)],
        top=[(40, 'VCC', PWR)], bottom=[(20, 'VSS', PWR)], width=20.32,
        footprint=DIP.format(40, '15.24'), description='Intel 8085A 8-bit microprocessor'),
    box('8155',
        data(range(12, 20), 'AD', BI) + [None, (8, '~{CE}', IN), (7, 'IO/~{M}', IN), (11, 'ALE', IN), (9, '~{RD}', IN),
                                          (10, '~{WR}', IN), (4, 'RESET', IN), None, (3, 'TIMER_IN', IN), (6, '~{TIMER_OUT}', OUT)],
        data(range(21, 29), 'PA', BI) + [None] + data(range(29, 37), 'PB', BI) + [None]
        + [(37, 'PC0', BI), (38, 'PC1', BI), (39, 'PC2', BI), (1, 'PC3', BI), (2, 'PC4', BI), (5, 'PC5', BI)],
        top=[(40, 'VCC', PWR)], bottom=[(20, 'VSS', PWR)], width=20.32,
        footprint=DIP.format(40, '15.24'), description='Intel 8155 256-byte RAM, I/O ports and timer'),
    box('8279',
        data(range(12, 20), 'DB', BI) + [None, (21, 'A0', IN), (22, '~{CS}', IN), (10, '~{RD}', IN), (11, '~{WR}', IN), (3, 'CLK', IN),
                                          (9, 'RESET', IN), (4, 'IRQ', OUT), None, (36, 'SHIFT', IN), (37, 'CNTL/STB', IN), None]
        + [(p, f'RL{i}', IN) for i, p in enumerate([38, 39, 1, 2, 5, 6, 7, 8])],
        [(27, 'OUT_A0', OUT), (26, 'OUT_A1', OUT), (25, 'OUT_A2', OUT), (24, 'OUT_A3', OUT),
         (31, 'OUT_B0', OUT), (30, 'OUT_B1', OUT), (29, 'OUT_B2', OUT), (28, 'OUT_B3', OUT), None,
         (32, 'SL0', OUT), (33, 'SL1', OUT), (34, 'SL2', OUT), (35, 'SL3', OUT), None, (23, '~{BD}', OUT)],
        top=[(40, 'VCC', PWR)], bottom=[(20, 'VSS', PWR)], width=20.32,
        footprint=DIP.format(40, '15.24'), description='Intel 8279 keyboard/display interface'),
    box('8251A',
        [(p, f'D{i}', BI) for i, p in enumerate([27, 28, 1, 2, 5, 6, 7, 8])]
        + [None, (12, 'C/~{D}', IN), (11, '~{CS}', IN), (13, '~{RD}', IN), (10, '~{WR}', IN), (20, 'CLK', IN), (21, 'RESET', IN)],
        [(19, 'TXD', OUT), (23, '~{RTS}', OUT), (24, '~{DTR}', OUT), (15, 'TXRDY', OUT), (18, 'TXEMPTY', OUT), None,
         (3, 'RXD', IN), (17, '~{CTS}', IN), (22, '~{DSR}', IN), (14, 'RXRDY', OUT), (16, 'SYNDET/BD', BI), None,
         (9, '~{TXC}', IN), (25, '~{RXC}', IN)],
        top=[(26, 'VCC', PWR)], bottom=[(4, 'GND', PWR)], width=20.32,
        footprint=DIP.format(28, '15.24'), description='Intel 8251A USART'),
    box('2764',
        addr([10, 9, 8, 7, 6, 5, 4, 3, 25, 24, 21, 23, 2]) + [None, (20, '~{CE}', IN), (22, '~{OE}', IN), (27, '~{PGM}', IN),
                                                             (1, 'VPP', IN), (26, 'NC', PAS)],
        data([11, 12, 13, 15, 16, 17, 18, 19], 'O', TRI),
        top=[(28, 'VCC', PWR)], bottom=[(14, 'GND', PWR)], width=15.24,
        footprint=DIP.format(28, '15.24'), description='8K x 8 EPROM'),
    box('6264',
        addr([10, 9, 8, 7, 6, 5, 4, 3, 25, 24, 21, 23, 2]) + [None, (20, '~{CS1}', IN), (26, 'CS2', IN), (22, '~{OE}', IN),
                                                             (27, '~{WE}', IN), (1, 'NC', PAS)],
        data([11, 12, 13, 15, 16, 17, 18, 19], 'I/O', BI),
        top=[(28, 'VCC', PWR)], bottom=[(14, 'GND', PWR)], width=15.24,
        footprint=DIP.format(28, '15.24'), description='8K x 8 static RAM'),
    box('2732',
        addr([8, 7, 6, 5, 4, 3, 2, 1, 23, 22, 19, 21]) + [None, (18, '~{CE}', IN), (20, '~{OE}', IN)],
        data([9, 10, 11, 13, 14, 15, 16, 17], 'O', TRI),
        top=[(24, 'VCC', PWR)], bottom=[(12, 'GND', PWR)], width=15.24,
        footprint=DIP.format(24, '15.24'), description='4K x 8 EPROM (socket SC24)'),
    box('MC1488',
        [(2, 'IN1', IN), None, (4, 'IN2A', IN), (5, 'IN2B', IN), None, (9, 'IN3A', IN), (10, 'IN3B', IN), None, (12, 'IN4A', IN), (13, 'IN4B', IN)],
        [(3, 'OUT1', OUT), None, None, (6, 'OUT2', OUT), None, None, (8, 'OUT3', OUT), None, None, (11, 'OUT4', OUT)],
        top=[(14, 'VCC', PWR)], bottom=[(7, 'GND', PWR), (1, 'VEE', PWR)], width=12.7,
        footprint=DIP.format(14, '7.62'), description='Quad RS-232 line driver'),
    box('MC1489',
        [(1, 'IN1', IN), (2, 'RC1', IN), None, (4, 'IN2', IN), (5, 'RC2', IN), None, (10, 'IN3', IN), (9, 'RC3', IN), None, (13, 'IN4', IN), (12, 'RC4', IN)],
        [(3, 'OUT1', OUT), None, None, (6, 'OUT2', OUT), None, None, (8, 'OUT3', OUT), None, None, (11, 'OUT4', OUT)],
        top=[(14, 'VCC', PWR)], bottom=[(7, 'GND', PWR)], width=12.7,
        footprint=DIP.format(14, '7.62'), description='Quad RS-232 line receiver'),
    box('7SEG_CC',
        [(14, 'A', PAS), (13, 'B', PAS), (8, 'C', PAS), (7, 'D', PAS), (6, 'E', PAS), (1, 'F', PAS), (2, 'G', PAS), (9, 'DP', PAS)],
        [None, None, None, (4, 'K1', PAS), (12, 'K2', PAS)],
        width=10.16, ref='DS', description='7-segment LED display, common cathode (pinout as drawn on the Mikrokit schematic)'),
]
