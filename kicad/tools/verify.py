"""Checks for the generated project.

1. The netlist KiCad exports has exactly the connections in circuit.py.
2. The address decoding, simulated from those connections, gives the
   memory and I/O map the emulator (and the original monitor ROM) use.
"""
import os
import subprocess
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(__file__))
import circuit  # noqa: E402

HERE = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
fails = 0


def check(ok, msg):
    global fails
    if not ok:
        fails += 1
    print(('ok    ' if ok else 'FAIL  ') + msg)


# ----------------------------------------------------------------------------
# 1. netlist
# ----------------------------------------------------------------------------
xml = os.path.join(HERE, 'microkit-netlist.xml')
subprocess.run(['kicad-cli', 'sch', 'export', 'netlist', '--format', 'kicadxml', '-o', xml,
                os.path.join(HERE, 'microkit.kicad_sch')], check=True, capture_output=True)
root = ET.parse(xml).getroot()
os.remove(xml)
kicad = {}
for net in root.iter('net'):
    nodes = frozenset((n.get('ref'), n.get('pin')) for n in net.iter('node') if not n.get('ref').startswith('#'))
    if nodes and not net.get('name').startswith('unconnected-'):  # no-connect pins
        kicad[net.get('name')] = nodes

want = {}
for _, _, _, parts in circuit.SHEETS:
    for p in parts:
        if p.ref.startswith('#'):
            continue
        for pin, net in p.conns.items():
            want.setdefault(net, set()).add((p.ref, pin))
want = {n: frozenset(s) for n, s in want.items()}

kicad_groups = set(kicad.values())
missing = [n for n, s in want.items() if s not in kicad_groups]
extra = [n for n, s in kicad.items() if s not in set(want.values())]
check(not missing and not extra, f'netlist: {len(want)} nets in circuit.py, {len(kicad)} in KiCad, '
      f'{len(missing)} missing, {len(extra)} unexpected')
for n in missing[:10]:
    print('      missing', n, sorted(want[n])[:6])
for n in extra[:10]:
    print('      unexpected', n, sorted(kicad[n])[:6])
single = [n for n, s in want.items() if len(s) < 2]
check(not single, f'no net with a single pin {single[:8]}')

# ----------------------------------------------------------------------------
# 2. decoding, simulated from circuit.py
# ----------------------------------------------------------------------------
parts = {}
for _, _, _, ps in circuit.SHEETS:
    for p in ps:
        parts.setdefault(p.ref, []).append(p)


def pin_net(ref, pin, unit=None):
    for p in parts[ref]:
        if str(pin) in p.conns and (unit is None or p.unit == unit):
            return p.conns[str(pin)]
    raise KeyError((ref, pin))


# jumper positions: 'A' = pins 1-2, 'B' = pins 2-3 (for 2-pin jumpers: True = fitted)
JUMPERS = {'J7': 'A', 'J8': 'A', 'J9': 'A', 'J10': 'A', 'J11': 'A', 'J14': 'A',
           'J5': 'B', 'J6': 'B', 'J12': 'B'}


def evaluate(addr, iom, rd=0, wr=1, reset=0):
    v = {'+5V': 1, 'GND': 0, 'IO~{M}': iom, '~{RD}': rd, '~{WR}': wr, 'RESET': reset}
    for i in range(16):
        v[f'A{i}'] = (addr >> i) & 1
    for _ in range(6):  # settle
        for ref, ps in parts.items():
            for p in ps:
                lib = p.lib.split(':')[1]
                c = p.conns
                g = lambda pin: v.get(c.get(str(pin)))  # noqa: E731
                if lib == 'Jumper_3_Open' and ref in JUMPERS:
                    src = c['1'] if JUMPERS[ref] == 'A' else c['3']
                    if src in v:
                        v[c['2']] = v[src]
                elif lib == '74LS02':
                    for o, a, b in [(1, 2, 3), (4, 5, 6), (10, 8, 9), (13, 11, 12)]:
                        if str(o) in c and g(a) is not None and g(b) is not None:
                            v[c[str(o)]] = int(not (g(a) or g(b)))
                elif lib == '74LS20':
                    for o, ins in [(6, (1, 2, 4, 5)), (8, (9, 10, 12, 13))]:
                        if str(o) in c and all(g(i) is not None for i in ins):
                            v[c[str(o)]] = int(not all(g(i) for i in ins))
                elif lib == '74LS138':
                    if all(g(i) is not None for i in (1, 2, 3, 4, 5, 6)):
                        en = g(6) and not g(4) and not g(5)
                        sel = g(1) | g(2) << 1 | g(3) << 2
                        for k, o in enumerate([15, 14, 13, 12, 11, 10, 9, 7]):
                            if str(o) in c:
                                v[c[str(o)]] = 0 if en and sel == k else 1
    return v


def selected(net, iom, lo, hi, step=1, **kw):
    return [a for a in range(lo, hi + 1, step) if evaluate(a, iom, **kw).get(net) == 0]


def span(addrs):
    return f'{addrs[0]:04X}-{addrs[-1]:04X}' if addrs else 'none'


mem = range(0, 0x10000, 0x100)
for net, want_lo, want_hi, what in [('~{CS_ROM}', 0x0000, 0x1FFF, '2764 ROM (IC2)'),
                                     ('~{CS_RAM}', 0x2000, 0x3FFF, '6264 RAM (IC3)'),
                                     ('~{CS_SC24}', 0x1000, 0x1FFF, '2732 socket SC24'),
                                     ('~{CS1}', 0x4000, 0x47FF, '8155 RAM (IC12)')]:
    got = [a for a in mem if evaluate(a, 0).get(net) == 0]
    exp = list(range(want_lo, want_hi + 1, 0x100))
    check(got == exp, f'{what}: selected at {span(got)} (expected {want_lo:04X}-{want_hi:04X})')

# I/O cycles put the port number on A8-A15 (and A0-A7)
ports = {}
for net in ['~{CS1}', '~{CS2}', '~{CS3}', '~{CS4}', '~{CS5}']:
    ports[net] = [p for p in range(256) if evaluate(p << 8 | p, 1).get(net) == 0]
for net, exp, what in [('~{CS1}', list(range(0x40, 0x48)), '8155 ports'),
                       ('~{CS2}', [0x0E, 0x0F], 'J16 input (IC21)'),
                       ('~{CS3}', [0x1E, 0x1F], '8251'),
                       ('~{CS4}', [0x2E, 0x2F], '8279'),
                       ('~{CS5}', [0x3E, 0x3F], 'X3 inputs (IC21)')]:
    got = ports[net]
    check(got == exp, f'{what}: ports {" ".join(f"{p:02X}" for p in got)}')

# control strobes from IC6
for iom, rd, wr, net in [(0, 0, 1, '~{MEMR}'), (0, 1, 0, '~{MEMW}'), (1, 0, 1, '~{IOR}'), (1, 1, 0, '~{IOW}')]:
    v = evaluate(0x2000, iom, rd=rd, wr=wr)
    others = [n for n in ['~{MEMR}', '~{MEMW}', '~{IOR}', '~{IOW}'] if n != net and v.get(n) == 0]
    check(v.get(net) == 0 and not others, f'IC6: IO/M={iom} RD={rd} WR={wr} gives only {net}')
v = evaluate(0x2000, 0, rd=0, wr=1, reset=1)
check(all(v.get(n) == 1 for n in ['~{MEMR}', '~{MEMW}', '~{IOR}', '~{IOW}']), 'IC6: no strobes during RESET')

print('\nall checks passed' if not fails else f'\n{fails} check(s) failed')
sys.exit(1 if fails else 0)
