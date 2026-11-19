"""Draw the MikroKit-Emu icon: an 8085 in a DIP package, in 3D.

A small orthographic renderer: every surface is a 3D polygon, shaded with
a gradient, culled when it faces away, and painted from back to front.

usage: python3 assets/make_icon.py   (writes assets/icon-*.png and assets/icon.ico)
"""
import math
import os

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
N = 1024  # drawn large, then scaled down

# package, in chip units: x along the length, y across (+y towards the viewer), z up
L, W = 6.4, 2.7          # bottom of the body
H = 1.15                 # body height
DRAFT = 0.13             # the top is smaller than the bottom (moulded package)
Z_SHOULDER = 0.42        # where the pins leave the body
REACH = 0.34             # how far the shoulder sticks out
Z_TIP = -1.55            # bottom of the pins
PINS = 8                 # per side
ROT, TILT = -21.0, 38.0  # view: turn about z, then tilt towards the viewer


def view(p):
    """3D point -> (screen x, screen y, depth towards the viewer)."""
    x, y, z = p
    a, t = math.radians(ROT), math.radians(TILT)
    xr = x * math.cos(a) - y * math.sin(a)
    yr = x * math.sin(a) + y * math.cos(a)
    return xr, yr * math.sin(t) - z * math.cos(t), yr * math.cos(t) + z * math.sin(t)


class Canvas:
    def __init__(self, pts3d):
        vs = [view(p) for p in pts3d]
        x0, x1 = min(v[0] for v in vs), max(v[0] for v in vs)
        y0, y1 = min(v[1] for v in vs), max(v[1] for v in vs)
        self.s = N * 0.90 / max(x1 - x0, y1 - y0)
        self.ox = N / 2 - (x0 + x1) / 2 * self.s
        self.oy = N / 2 - (y0 + y1) / 2 * self.s
        self.img = Image.new('RGBA', (N, N), (0, 0, 0, 0))

    def P(self, p):
        x, y, _ = view(p)
        return (self.ox + x * self.s, self.oy + y * self.s)

    def gradient_fill(self, poly2d, top_rgb, bottom_rgb, highlight=None):
        """Fill a polygon with a vertical gradient (screen y)."""
        ys = [p[1] for p in poly2d]
        ya, yb = min(ys), max(ys)
        mask = Image.new('L', (N, N), 0)
        ImageDraw.Draw(mask).polygon(poly2d, fill=255)
        grad = Image.new('RGBA', (1, N))
        for y in range(N):
            f = 0.0 if yb == ya else min(1.0, max(0.0, (y - ya) / (yb - ya)))
            c = [int(top_rgb[i] + (bottom_rgb[i] - top_rgb[i]) * f) for i in range(3)]
            if highlight is not None:
                # a bright band (metal sheen)
                h = math.exp(-((f - highlight[0]) / highlight[1]) ** 2)
                c = [min(255, int(v + (255 - v) * 0.75 * h)) for v in c]
            grad.putpixel((0, y), (*c, 255))
        self.img.paste(grad.resize((N, N)), (0, 0), mask)


# outward normals of the body's side faces
NORMALS = {'front': (0, 1, 0), 'back': (0, -1, 0), 'right': (1, 0, 0), 'left': (-1, 0, 0)}


def faces_viewer(name):
    """True if the side face with this name is turned towards the viewer."""
    a, t = math.radians(ROT), math.radians(TILT)
    v = (math.sin(a) * math.cos(t), math.cos(a) * math.cos(t), math.sin(t))  # towards the viewer
    n = NORMALS[name]
    return n[0] * v[0] + n[1] * v[1] + n[2] * v[2] > 0


def depth(poly3d):
    return sum(view(p)[2] for p in poly3d) / len(poly3d)


def pin_polys(i, side):
    """A DIP pin: a shoulder out of the body, then a leg that narrows to a tip."""
    pitch = (L - 0.9) / (PINS - 1)
    cx = -L / 2 + 0.45 + i * pitch
    wide, narrow, thick = pitch * 0.44, pitch * 0.20, 0.07
    y_body = side * (W / 2 - 0.02)
    y_out = side * (W / 2 + REACH)
    zs = Z_SHOULDER
    zt = zs + 0.07
    polys = []
    # shoulder (top surface, horizontal)
    polys.append(('shoulder', [(cx - wide / 2, y_body, zt), (cx + wide / 2, y_body, zt),
                               (cx + wide / 2, y_out, zt), (cx - wide / 2, y_out, zt)]))
    # leg outline in the vertical plane y = y_out (outer face)
    def leg(y):
        return [(cx - wide / 2, y, zt), (cx + wide / 2, y, zt), (cx + wide / 2, y, zs - 0.38),
                (cx + narrow / 2, y, zs - 0.62), (cx + narrow / 2, y, Z_TIP),
                (cx - narrow / 2, y, Z_TIP), (cx - narrow / 2, y, zs - 0.62), (cx - wide / 2, y, zs - 0.38)]
    outer = leg(y_out + side * thick / 2)
    inner = leg(y_out - side * thick / 2)
    polys.append(('leg_back', inner))
    # the visible edge (thickness) on the right side of the leg
    for a, b in [(1, 2), (2, 3), (3, 4)]:
        polys.append(('leg_edge', [outer[a], outer[b], inner[b], inner[a]]))
    polys.append(('leg', outer))
    return polys


def main():
    d = DRAFT
    bl = [(-L / 2, -W / 2, 0), (L / 2, -W / 2, 0), (L / 2, W / 2, 0), (-L / 2, W / 2, 0)]
    tp = [(-L / 2 + d, -W / 2 + d, H), (L / 2 - d, -W / 2 + d, H), (L / 2 - d, W / 2 - d, H), (-L / 2 + d, W / 2 - d, H)]
    body = {
        'top': tp,
        'front': [tp[3], tp[2], bl[2], bl[3]],
        'back': [tp[1], tp[0], bl[0], bl[1]],
        'right': [tp[2], tp[1], bl[1], bl[2]],
        'left': [tp[0], tp[3], bl[3], bl[0]],
    }
    extent = bl + tp + [(x, y * (W / 2 + REACH + 0.3) / (W / 2), Z_TIP - 0.1) for x, y, _ in bl]
    cv = Canvas(extent)
    P = cv.P

    # soft shadow on the ground
    sh = Image.new('L', (N, N), 0)
    g = Z_TIP
    ImageDraw.Draw(sh).polygon([P((-L / 2 - .1, -W / 2 - .55, g)), P((L / 2 + .2, -W / 2 - .55, g)),
                                P((L / 2 + .35, W / 2 + .75, g)), P((-L / 2 + .05, W / 2 + .75, g))], fill=120)
    sh = sh.filter(ImageFilter.GaussianBlur(N / 40))
    cv.img.paste((0, 0, 0, 255), (0, 0), sh)

    items = []  # (depth, kind, poly3d)
    for side in (-1, 1):
        for i in range(PINS):
            for kind, poly in pin_polys(i, side):
                items.append((depth(poly) + (0 if side > 0 else -100), kind, poly, side))
    for name, poly in body.items():
        if name == 'top':
            continue
        if faces_viewer(name):
            items.append((depth(poly), 'body_' + name, poly, 0))
    # back pins, then the body sides, then the front pins
    order = sorted(items, key=lambda it: (0 if it[3] < 0 else 2 if it[3] > 0 else 1, it[0]))

    METAL = {
        'shoulder': ((236, 238, 241), (205, 208, 213), None),
        'leg': ((215, 218, 223), (150, 154, 161), (0.25, 0.22)),
        'leg_back': ((120, 123, 129), (85, 88, 94), None),
        'leg_edge': ((150, 153, 159), (105, 108, 114), None),
    }
    for _, kind, poly, side in order:
        p2 = [P(p) for p in poly]
        if kind.startswith('body_'):
            if kind == 'body_front':
                cv.gradient_fill(p2, (52, 53, 57), (20, 20, 23))
            else:
                cv.gradient_fill(p2, (40, 41, 45), (14, 14, 16))
            continue
        top, bottom, hl = METAL[kind]
        if side < 0:  # back pins are in the body's shadow
            top = tuple(int(c * 0.62) for c in top)
            bottom = tuple(int(c * 0.62) for c in bottom)
            hl = None
        cv.gradient_fill(p2, top, bottom, hl)
        ImageDraw.Draw(cv.img).line(p2 + [p2[0]], fill=(70, 72, 78, 255), width=max(1, N // 512))

    # top face: a texture (sheen, notch, dimple, lettering) mapped onto it
    TW, TH = 1700, int(1700 * (W - 2 * d) / (L - 2 * d))
    tex = Image.new('RGBA', (TW, TH), (0, 0, 0, 0))
    td = ImageDraw.Draw(tex)
    for yy in range(TH):  # the back of the top catches more light
        f = yy / TH
        c = int(78 - 26 * f)
        td.line([(0, yy), (TW, yy)], fill=(c, c + 1, c + 4, 255))
    nr = TH * 0.17
    td.pieslice((-nr, TH / 2 - nr, nr, TH / 2 + nr), 270, 90, fill=(30, 31, 34, 255))  # pin-1 notch
    dr = TH * 0.075
    td.ellipse((TW * 0.06 - dr, TH * 0.78 - dr, TW * 0.06 + dr, TH * 0.78 + dr), fill=(38, 39, 43, 255))
    font = ImageFont.truetype('DejaVuSans-Bold.ttf', int(TH * 0.42))
    td.text((TW * 0.53, TH * 0.50), '8085 µP', font=font, fill=(236, 238, 242, 255), anchor='mm')
    A, B, C = P(tp[0]), P(tp[1]), P(tp[3])  # texture (0,0), (TW,0), (0,TH)
    ux, uy = (B[0] - A[0]) / TW, (B[1] - A[1]) / TW
    vx, vy = (C[0] - A[0]) / TH, (C[1] - A[1]) / TH
    det = ux * vy - uy * vx
    ia, ib, ic, idd = vy / det, -vx / det, -uy / det, ux / det
    coeffs = (ia, ib, -(ia * A[0] + ib * A[1]), ic, idd, -(ic * A[0] + idd * A[1]))
    cv.img.alpha_composite(tex.transform((N, N), Image.AFFINE, coeffs, resample=Image.BICUBIC))

    # crisp edges, with a light catch along the top's front edge
    dr_ = ImageDraw.Draw(cv.img)
    w = max(2, N // 300)
    dr_.line([P(tp[3]), P(tp[2])], fill=(120, 122, 128, 255), width=w)
    dr_.line([P(tp[2]), P(tp[1])], fill=(95, 97, 103, 255), width=w)
    for name in ('front', 'right', 'left'):
        poly = body[name]
        if faces_viewer(name):
            dr_.line([P(poly[2]), P(poly[3])], fill=(8, 8, 9, 255), width=w)

    img = cv.img
    sizes = [256, 128, 64, 48, 32, 24, 16]
    for s in sizes:
        im = img.resize((s, s), Image.LANCZOS)
        if s <= 32:
            im = im.filter(ImageFilter.UnsharpMask(radius=1, percent=70, threshold=2))
        im.save(os.path.join(HERE, f'icon-{s}.png'), optimize=True)
    img.resize((256, 256), Image.LANCZOS).save(os.path.join(HERE, 'icon.ico'), sizes=[(s, s) for s in sizes])
    img.save(os.path.join(HERE, 'icon-1024.png'), optimize=True)
    print('icons written')


if __name__ == '__main__':
    main()
