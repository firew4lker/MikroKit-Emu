"""Generate the Mikrokit KiCad project from circuit.py.

Every pin gets a short wire and a net label (or a power symbol, or a
no-connect flag), so the drawing is a labelled-net schematic: parts are
laid out in rows and the labels carry the connections, like the bus
labels on the original sheets.

Output is written in the KiCad 8 file format and then upgraded with
kicad-cli to whatever this KiCad writes.
"""
import copy
import math
import os
import subprocess
import sys
import uuid

sys.path.insert(0, os.path.dirname(__file__))
from sexp import Sym as S, dump, find, find1, parse  # noqa: E402
from libinspect import get as stock_get  # noqa: E402
import circuit  # noqa: E402
import symbols  # noqa: E402

OUT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
PROJECT = 'mikrokit'
NS = uuid.UUID('5b1f7c1e-2f43-4f8e-9a7e-8085a0000000')
G = 2.54
STUB = 2.54
POWER = {'+5V', 'GND', '+12V', '-12V'}
CUSTOM = {s[1]: s for s in symbols.SYMBOLS}


def uid(*key):
    return str(uuid.uuid5(NS, '/'.join(str(k) for k in key)))


def snap(v):
    return round(v / G) * G


def font(*extra):
    return [S('effects'), [S('font'), [S('size'), 1.27, 1.27]]] + list(extra)


# ----------------------------------------------------------------------------
# library symbols
# ----------------------------------------------------------------------------
_flat = {}


def flat_symbol(lib_id):
    """The symbol with `extends` resolved, named like the library name (no prefix)."""
    if lib_id in _flat:
        return _flat[lib_id]
    lib, name = lib_id.split(':')
    if lib == PROJECT:
        sym = copy.deepcopy(CUSTOM[name])
    else:
        e = stock_get(lib, name)
        ext = find1(e, 'extends')
        if ext:
            base = flat_symbol(f'{lib}:{ext[1]}')
            sym = copy.deepcopy(base)
            sym[1] = name
            for sub in find(sym, 'symbol'):
                sub[1] = name + sub[1][len(ext[1]):]
            props = {p[1]: p for p in find(e, 'property')}
            for i, x in enumerate(sym):
                if isinstance(x, list) and x and x[0] == 'property' and x[1] in props:
                    sym[i] = copy.deepcopy(props.pop(x[1]))
            sym.extend(copy.deepcopy(list(props.values())))
        else:
            sym = copy.deepcopy(e)
    _flat[lib_id] = sym
    return sym


def embedded(lib_id):
    s = copy.deepcopy(flat_symbol(lib_id))
    s[1] = lib_id
    return s


def is_hidden(p):
    return any(x == 'hide' for x in p) or any(isinstance(x, list) and x and x[0] == 'hide' and (len(x) == 1 or x[1] == 'yes') for x in p)


def unit_pins(lib_id, unit):
    sym = flat_symbol(lib_id)
    name = sym[1]
    pins = []
    for sub in find(sym, 'symbol'):
        u, st = sub[1][len(name) + 1:].split('_')
        u, st = int(u), int(st)
        if st == 2 or u not in (0, unit):
            continue
        for p in find(sub, 'pin'):
            at = find1(p, 'at')
            pins.append(dict(num=find1(p, 'number')[1], type=str(p[1]), x=float(at[1]), y=float(at[2]),
                             ang=int(float(at[3])) if len(at) > 3 else 0, hidden=is_hidden(p)))
    return pins


def graphic_extent(lib_id, unit):
    sym = flat_symbol(lib_id)
    name = sym[1]
    xs, ys = [0.0], [0.0]

    def walk(e):
        if isinstance(e, list) and e:
            if e[0] in ('xy', 'start', 'end', 'center', 'mid') and len(e) >= 3:
                xs.append(float(e[1])); ys.append(float(e[2]))
            if e[0] == 'circle':
                c = find1(e, 'center'); r = find1(e, 'radius')
                if c and r:
                    xs.extend([float(c[1]) - float(r[1]), float(c[1]) + float(r[1])])
                    ys.extend([float(c[2]) - float(r[1]), float(c[2]) + float(r[1])])
                return
            if e[0] in ('pin', 'property', 'text'):
                return
            for x in e[1:]:
                walk(x)

    for sub in find(sym, 'symbol'):
        u, st = sub[1][len(name) + 1:].split('_')
        if int(st) == 2 or int(u) not in (0, unit):
            continue
        walk(sub)
    return min(xs), max(xs), min(ys), max(ys)


# ----------------------------------------------------------------------------
# geometry of a placed unit
# ----------------------------------------------------------------------------
OUTWARD = {0: (-1, 0), 90: (0, 1), 180: (1, 0), 270: (0, -1)}  # screen dx, dy (y down) for pin angle


def text_len(t):
    t = t.replace('~{', '').replace('}', '')
    return len(t) * 1.15 + 1.5


class Placed:
    def __init__(self, part, global_nets):
        self.p = part
        self.pins = [pn for pn in unit_pins(part.lib, part.unit) if not (pn['hidden'] and pn['num'] not in part.conns)]
        self.global_nets = global_nets
        x0, x1, y0, y1 = graphic_extent(part.lib, part.unit)
        # screen coords relative to origin: (x, -y)
        bx = [x0, x1]
        by = [-y1, -y0]
        for pn in self.pins:
            px, py = pn['x'], -pn['y']
            dx, dy = OUTWARD[pn['ang']]
            net = part.conns.get(pn['num'])
            ext = STUB
            if net is None:
                ext = 1.5
            elif net in POWER:
                ext = STUB + (5.5 if dy else 4 + text_len(net))
            else:
                ext = STUB + text_len(net) + (3 if net in global_nets else 0)
            bx += [px, px + dx * ext]
            by += [py, py + dy * ext]
            if dx:  # label text height
                by += [py - 1.8, py + 1.0]
            else:
                bx += [px - 1.8, px + 1.0]
        # visible reference/value text
        sym = flat_symbol(part.lib)
        for prop in find(sym, 'property'):
            if prop[1] not in ('Reference', 'Value') or part.ref.startswith('#'):
                continue
            eff = find1(prop, 'effects')
            if eff and (any(x == 'hide' for x in eff) or any(isinstance(x, list) and x and x[0] == 'hide' for x in eff)):
                continue
            at = find1(prop, 'at')
            tx, ty = float(at[1]), -float(at[2])
            ang = float(at[3]) if len(at) > 3 else 0
            text = part.ref if prop[1] == 'Reference' else part.value
            L = text_len(text)
            just = find1(eff, 'justify') if eff else None
            j = str(just[1]) if just else 'center'
            lo, hi = {'left': (0, L), 'right': (-L, 0)}.get(j, (-L / 2, L / 2))
            if ang in (90, 270):
                by += [ty - hi, ty - lo]
                bx += [tx - 1.2, tx + 1.2]
            else:
                bx += [tx + lo, tx + hi]
                by += [ty - 1.2, ty + 1.2]
        self.bbox = (min(bx) - 1, max(bx) + 1, min(by) - 1, max(by) + 1)

    @property
    def w(self):
        return self.bbox[1] - self.bbox[0]

    @property
    def h(self):
        return self.bbox[3] - self.bbox[2]


# ----------------------------------------------------------------------------
# emitting one sheet
# ----------------------------------------------------------------------------
PAPERS = [('A3', 420, 297), ('A2', 594, 420), ('A1', 841, 594), ('A0', 1189, 841)]
MARGIN = 15
TITLE_H = 40


def flow(placed, width):
    """Flow units left to right in rows within `width`. Returns offsets of
    each unit's bbox corner and the block size."""
    gap_x, gap_y = 3.81, 3.81
    x = y = 0.0
    row_h = 0.0
    w_used = 0.0
    pos = []
    for pl in placed:
        if x > 0 and x + pl.w > width:
            x, y, row_h = 0.0, y + row_h + gap_y, 0.0
        pos.append((x, y))
        x += pl.w + gap_x
        w_used = max(w_used, x - gap_x)
        row_h = max(row_h, pl.h)
    return pos, w_used, y + row_h


def pack(blocks, page_w):
    """Bottom-left skyline packing of (w, h) blocks in order. Returns positions and height."""
    cols = int(page_w // G)
    sky = [0.0] * cols
    gap = 3.81
    out = []
    for w, h in blocks:
        n = min(cols, int(math.ceil((w + gap) / G)))
        best = None
        for c in range(0, cols - n + 1):
            y = max(sky[c:c + n])
            if best is None or y < best[0] - 0.01:
                best = (y, c)
        y, c = best
        out.append((c * G, y))
        for k in range(c, c + n):
            sky[k] = y + h + gap
    return out, max(sky)


class Sheet:
    def __init__(self, name, title, note, parts, path, global_nets, counters):
        self.name, self.title, self.note, self.parts = name, title, note, parts
        self.path = path
        self.global_nets = global_nets
        self.counters = counters
        self.items = []
        self.lib_ids = set()

    def ref_power(self):
        self.counters['pwr'] += 1
        return f"#PWR{self.counters['pwr']:03d}"

    def build(self):
        import layout as L
        units = {}
        for p in self.parts:
            units.setdefault(p.ref, []).append(p)
        blocks, seen = [], set()
        for width, refs in L.BLOCKS.get(self.name, []):
            items = []
            for r in refs:
                ref, _, unit = r.partition('.')
                for p in units.get(ref, []):
                    if (not unit or p.unit == int(unit)) and id(p) not in seen:
                        items.append(p); seen.add(id(p))
            blocks.append((width, items))
        rest = [p for p in self.parts if id(p) not in seen]
        if rest:
            blocks.append((150, rest))
        placed = [(w, [Placed(p, self.global_nets) for p in items]) for w, items in blocks if items]
        flows = [(flow(pls, w), pls) for w, pls in placed]
        sizes = [(bw, bh) for (_, bw, bh), _ in flows]
        orders = [list(range(len(sizes))),
                  sorted(range(len(sizes)), key=lambda i: -sizes[i][1]),
                  sorted(range(len(sizes)), key=lambda i: -sizes[i][0] * sizes[i][1]),
                  sorted(range(len(sizes)), key=lambda i: -sizes[i][0])]
        import random
        rnd = random.Random(8085)
        for _ in range(400):
            o = list(range(len(sizes)))
            rnd.shuffle(o)
            orders.append(o)
        done = False
        for paper, pw, ph in PAPERS:
            for order in orders:
                p_pos, used_h = pack([sizes[i] for i in order], pw - 2 * MARGIN)
                if used_h + 2 * MARGIN + TITLE_H <= ph:
                    pos = [None] * len(sizes)
                    for i, xy in zip(order, p_pos):
                        pos[i] = xy
                    done = True
                    break
            if done:
                break
        self.paper = paper
        for (bx, by), ((offs, _, _), pls) in zip(pos, flows):
            for (ux, uy), pl in zip(offs, pls):
                ox = snap(MARGIN + bx + ux - pl.bbox[0] + G)
                oy = snap(MARGIN + by + uy - pl.bbox[2] + G)
                self.emit_unit(pl, ox, oy)

    def emit_unit(self, pl, ox, oy):
        p = pl.p
        self.lib_ids.add(p.lib)
        key = (self.name, p.ref, p.unit)
        sym = flat_symbol(p.lib)
        props = []
        for prop in find(sym, 'property'):
            pname = prop[1]
            if isinstance(pname, S) or pname.startswith('ki_'):
                continue  # library-only properties (ki_*, private ...)
            at = find1(prop, 'at')
            val = {'Reference': p.ref, 'Value': p.value, 'Footprint': p.fp if p.fp is not None else prop[2]}.get(pname, prop[2])
            eff = copy.deepcopy(find1(prop, 'effects')) or font()
            hide = pname not in ('Reference', 'Value') or p.ref.startswith('#')
            if hide and not any(isinstance(x, list) and x[0] == 'hide' for x in eff):
                eff.append([S('hide'), S('yes')])
            for x in list(prop):
                if isinstance(x, list) and x and x[0] == 'hide':
                    eff = [y for y in eff if not (isinstance(y, list) and y and y[0] == 'hide')] + [[S('hide'), S('yes')]]
            props.append([S('property'), pname, val,
                          [S('at'), round(ox + float(at[1]), 4), round(oy - float(at[2]), 4), float(at[3]) if len(at) > 3 else 0],
                          eff])
        # every pin of every unit, so kicad-cli doesn't add random UUIDs for the other units
        nums = []
        for sub in find(sym, 'symbol'):
            for pp in find(sub, 'pin'):
                n = find1(pp, 'number')[1]
                if n not in nums:
                    nums.append(n)
        pins = [[S('pin'), n, [S('uuid'), uid(*key, 'pin', n)]] for n in nums]
        self.items.append([S('symbol'), [S('lib_id'), p.lib], [S('at'), ox, oy, 0], [S('unit'), p.unit],
                           [S('exclude_from_sim'), S('no')], [S('in_bom'), S('no') if p.ref.startswith('#') else S('yes')],
                           [S('on_board'), S('no') if p.ref.startswith('#') else S('yes')], [S('dnp'), S('no')],
                           [S('uuid'), uid(*key)]] + props + pins + [
            [S('instances'), [S('project'), PROJECT, [S('path'), self.path, [S('reference'), p.ref], [S('unit'), p.unit]]]]])
        for pn in pl.pins:
            px, py = ox + pn['x'], oy - pn['y']
            net = p.conns.get(pn['num'])
            if net is None:
                if pn['type'] != 'no_connect':
                    self.items.append([S('no_connect'), [S('at'), px, py], [S('uuid'), uid(*key, 'nc', pn['num'])]])
                continue
            dx, dy = OUTWARD[pn['ang']]
            ex, ey = round(px + dx * STUB, 4), round(py + dy * STUB, 4)
            self.items.append([S('wire'), [S('pts'), [S('xy'), px, py], [S('xy'), ex, ey]],
                               [S('stroke'), [S('width'), 0], [S('type'), S('default')]], [S('uuid'), uid(*key, 'w', pn['num'])]])
            if net in POWER:
                self.power_symbol(net, ex, ey, (dx, dy), key + (pn['num'],))
            else:
                self.label(net, ex, ey, (dx, dy), key + (pn['num'],))

    def label(self, net, x, y, d, key):
        ang, just = {(1, 0): (0, 'left'), (-1, 0): (180, 'right'), (0, -1): (90, 'left'), (0, 1): (270, 'right')}[d]
        if net in self.global_nets:
            self.items.append([S('global_label'), net, [S('shape'), S('passive')], [S('at'), x, y, ang],
                               [S('fields_autoplaced'), S('yes')], font([S('justify'), S(just)]), [S('uuid'), uid(*key, 'gl')],
                               [S('property'), 'Intersheetrefs', '${INTERSHEET_REFS}', [S('at'), x, y, 0], font([S('hide'), S('yes')])]])
        else:
            self.items.append([S('label'), net, [S('at'), x, y, ang], [S('fields_autoplaced'), S('yes')],
                               font([S('justify'), S(just), S('bottom')]), [S('uuid'), uid(*key, 'l')]])

    def power_symbol(self, net, x, y, d, key):
        lib_id = f'power:{net}'
        self.lib_ids.add(lib_id)
        pin = unit_pins(lib_id, 1)[0]
        out_ang = {(1, 0): 0, (0, -1): 90, (-1, 0): 180, (0, 1): 270}[d]
        rot = (out_ang - pin['ang']) % 360
        ref = self.ref_power()
        sym = flat_symbol(lib_id)
        props = []
        for prop in find(sym, 'property'):
            if isinstance(prop[1], S) or prop[1].startswith('ki_'):
                continue
            at = find1(prop, 'at')
            px, py = float(at[1]), float(at[2])
            t = math.radians(rot)
            rx, ry = px * math.cos(t) - py * math.sin(t), px * math.sin(t) + py * math.cos(t)
            val = {'Reference': ref, 'Value': net}.get(prop[1], prop[2])
            eff = font() if prop[1] == 'Value' else font([S('hide'), S('yes')])
            tx, ty = round(x + rx, 4), round(y - ry, 4)
            if prop[1] == 'Value' and d[0]:
                # side pin: put the text past the symbol's tip, on the pin's row
                tx, ty = x + d[0] * 3.8, y
                eff = font([S('justify'), S('left' if d[0] > 0 else 'right')])
            # field angles are stored relative to the symbol's rotation
            props.append([S('property'), prop[1], val, [S('at'), tx, ty, 90 if rot in (90, 270) else 0], eff])
        self.items.append([S('symbol'), [S('lib_id'), lib_id], [S('at'), x, y, rot], [S('unit'), 1],
                           [S('exclude_from_sim'), S('no')], [S('in_bom'), S('yes')], [S('on_board'), S('yes')], [S('dnp'), S('no')],
                           [S('uuid'), uid(*key, 'pwr')]] + props + [
            [S('pin'), '1', [S('uuid'), uid(*key, 'pwrpin')]],
            [S('instances'), [S('project'), PROJECT, [S('path'), self.path, [S('reference'), ref], [S('unit'), 1]]]]])

    def file(self, sheet_uuid, page_no, total):
        tb = [S('title_block'), [S('title'), f'Mikrokit V2.0 - {self.title}'], [S('date'), '1985-03-07'], [S('rev'), '2.0'],
              [S('company'), 'Micro Design Ltd (original design)'],
              [S('comment'), 1, f'Redrawn in KiCad from the scanned schematic, {self.note}'],
              [S('comment'), 2, 'Connections are by net label; see NOTES.md for parts that could not be read with certainty']]
        libs = [embedded(l) for l in sorted(self.lib_ids)]
        return [S('kicad_sch'), [S('version'), 20231120], [S('generator'), 'eeschema'], [S('generator_version'), '8.0'],
                [S('uuid'), sheet_uuid], [S('paper'), self.paper], tb, [S('lib_symbols')] + libs] + self.items


class _Break:
    def __init__(self, p):
        self.p = p


def BREAK():
    class B:
        lib = 'BREAK'
    return B()


def main():
    # nets used on more than one sheet become global labels
    where = {}
    for name, _, _, parts in circuit.SHEETS:
        for p in parts:
            for net in p.conns.values():
                where.setdefault(net, set()).add(name)
    global_nets = {n for n, s in where.items() if len(s) > 1 and n not in POWER}

    root_uuid = uid('root')
    counters = {'pwr': 0}
    sheets = []
    for i, (name, title, note, parts) in enumerate(circuit.SHEETS):
        su = uid('sheet', name)
        sh = Sheet(name, title, note, parts, f'/{root_uuid}/{su}', global_nets, counters)
        sh.build()
        sheets.append((sh, su))

    for i, (sh, su) in enumerate(sheets):
        with open(os.path.join(OUT, f'{sh.name}.kicad_sch'), 'w') as f:
            f.write(dump(sh.file(uid('file', sh.name), i + 2, len(sheets) + 1)) + '\n')

    # root sheet: one sheet symbol per page
    items = []
    for i, (sh, su) in enumerate(sheets):
        x, y = 30 + i * 90, 60
        items.append([S('sheet'), [S('at'), x, y], [S('size'), 70, 30], [S('fields_autoplaced'), S('yes')],
                      [S('stroke'), [S('width'), 0.1524], [S('type'), S('solid')]], [S('fill'), [S('color'), 0, 0, 0, 0.0]],
                      [S('uuid'), su],
                      [S('property'), 'Sheetname', f'{i + 1}. {sh.title}', [S('at'), x, y - 0.7, 0], font([S('justify'), S('left'), S('bottom')])],
                      [S('property'), 'Sheetfile', f'{sh.name}.kicad_sch', [S('at'), x, y + 30.6, 0], font([S('justify'), S('left'), S('top')])],
                      [S('instances'), [S('project'), PROJECT, [S('path'), f'/{root_uuid}', [S('page'), str(i + 2)]]]]])
    items.append([S('text'), 'Mikrokit V2.0 8085 trainer (Micro Design Ltd, Athens, 1985).\n'
                             'Redrawn in KiCad from the three scanned schematic sheets dated 7-3-1985.\n'
                             'Reference designators, jumper numbers and connector pin numbers follow the original.\n'
                             'See NOTES.md for what was inferred and what to check on a real board.',
                  [S('exclude_from_sim'), S('no')], [S('at'), 30, 120, 0], font([S('justify'), S('left'), S('bottom')]),
                  [S('uuid'), uid('root', 'text')]])
    root = [S('kicad_sch'), [S('version'), 20231120], [S('generator'), 'eeschema'], [S('generator_version'), '8.0'],
            [S('uuid'), root_uuid], [S('paper'), 'A4'],
            [S('title_block'), [S('title'), 'Mikrokit V2.0'], [S('date'), '1985-03-07'], [S('rev'), '2.0'],
             [S('company'), 'Micro Design Ltd (original design)'], [S('comment'), 1, 'KiCad redraw of the scanned schematic']],
            [S('lib_symbols')]] + items + [[S('sheet_instances'), [S('path'), '/', [S('page'), '1']]]]
    with open(os.path.join(OUT, f'{PROJECT}.kicad_sch'), 'w') as f:
        f.write(dump(root) + '\n')

    # project library, library table and project file
    lib = [S('kicad_symbol_lib'), [S('version'), 20231120], [S('generator'), 'mikrokit_gen']] + [copy.deepcopy(s) for s in symbols.SYMBOLS]
    with open(os.path.join(OUT, f'{PROJECT}.kicad_sym'), 'w') as f:
        f.write(dump(lib) + '\n')
    with open(os.path.join(OUT, 'sym-lib-table'), 'w') as f:
        f.write('(sym_lib_table\n  (version 7)\n  (lib (name "mikrokit")(type "KiCad")(uri "${KIPRJMOD}/mikrokit.kicad_sym")'
                '(options "")(descr "Mikrokit parts missing from the KiCad libraries"))\n)\n')
    pro = os.path.join(OUT, f'{PROJECT}.kicad_pro')
    if not os.path.exists(pro):
        with open(pro, 'w') as f:
            f.write('{\n  "meta": {\n    "filename": "mikrokit.kicad_pro",\n    "version": 3\n  }\n}\n')

    def run(*args):
        r = subprocess.run(['kicad-cli', *args], capture_output=True, text=True)
        if r.returncode != 0:
            print(r.stdout, r.stderr)
            raise SystemExit(f'kicad-cli {args[0]} {args[1]} failed')

    run('sym', 'upgrade', os.path.join(OUT, f'{PROJECT}.kicad_sym'))
    for sh, _ in sheets:
        run('sch', 'upgrade', os.path.join(OUT, f'{sh.name}.kicad_sch'))
    run('sch', 'upgrade', os.path.join(OUT, f'{PROJECT}.kicad_sch'))
    for sh, _ in sheets:
        print(f'{sh.name}: {sh.paper}, {sum(1 for x in sh.items if x[0] == "symbol")} symbols')


if __name__ == '__main__':
    main()
