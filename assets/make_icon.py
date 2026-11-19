"""Draw the MicroKit-Emu icon: an 8085 chip (DIP package) in 3D.

usage: python3 assets/make_icon.py   (writes assets/icon-*.png and assets/icon.ico)
"""
import math
import os

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
N = 1024  # drawn large, then scaled down

# chip body in chip units: x along the length, y across, z up
L, W, H = 6.6, 3.2, 1.05
PINS = 8  # per side (a 16-pin look reads better than 40 at icon size)


def project(p):
    """3D -> 2D: turn about z, tilt towards the viewer, orthographic."""
    x, y, z = p
    a = math.radians(-33)
    xr = x * math.cos(a) - y * math.sin(a)
    yr = x * math.sin(a) + y * math.cos(a)
    t = math.radians(50)
    sy = yr * math.cos(t) - z * math.sin(t)
    return (xr, sy)


def fit(points3d):
    pts = [project(p) for p in points3d]
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), max(xs), min(ys), max(ys)


def leg_shapes(side):
    """Each pin: a short flat part out from the body, then bent down."""
    shapes = []
    pitch = L / (PINS + 0.6)
    pw = pitch * 0.42
    y0 = -W / 2 if side < 0 else W / 2
    out = 0.45 * side
    for i in range(PINS):
        cx = -L / 2 + pitch * (i + 0.8)
        x0, x1 = cx - pw / 2, cx + pw / 2
        zb = 0.18  # leaves the body a little above the bottom
        flat = [(x0, y0, zb), (x1, y0, zb), (x1, y0 + out, zb), (x0, y0 + out, zb)]
        down = [(x0, y0 + out, zb), (x1, y0 + out, zb), (x1 + pw * 0.15, y0 + out * 1.05, -1.55),
                (x0 - pw * 0.15, y0 + out * 1.05, -1.55)]
        shapes.append((flat, down))
    return shapes


def main():
    body_top = [(-L / 2, -W / 2, H), (L / 2, -W / 2, H), (L / 2, W / 2, H), (-L / 2, W / 2, H)]
    body_front = [(-L / 2, W / 2, H), (L / 2, W / 2, H), (L / 2, W / 2, 0), (-L / 2, W / 2, 0)]
    body_right = [(L / 2, -W / 2, H), (L / 2, W / 2, H), (L / 2, W / 2, 0), (L / 2, -W / 2, 0)]
    allpts = body_top + body_front + body_right + [(0, 0, -1.3), (L / 2 + .3, W / 2 + .5, -1.3), (-L / 2, -W / 2 - .5, -1.3)]
    x0, x1, y0, y1 = fit(allpts)
    span = max(x1 - x0, y1 - y0)
    scale = N * 0.86 / span
    ox = N / 2 - (x0 + x1) / 2 * scale
    oy = N / 2 - (y0 + y1) / 2 * scale

    def P(p):
        x, y = project(p)
        return (ox + x * scale, oy + y * scale)

    img = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # soft shadow under the chip
    sh = Image.new('L', (N, N), 0)
    ImageDraw.Draw(sh).polygon([P((-L / 2 - .2, -W / 2 - .7, -1.3)), P((L / 2 + .2, -W / 2 - .7, -1.3)),
                                P((L / 2 + .2, W / 2 + .9, -1.3)), P((-L / 2 - .2, W / 2 + .9, -1.3))], fill=110)
    sh = sh.filter(ImageFilter.GaussianBlur(28))
    img.paste((0, 0, 0, 255), (0, 0), sh)

    def pin(flat, down, dark):
        d.polygon([P(p) for p in down], fill=(150, 154, 160) if dark else (205, 209, 214), outline=(90, 94, 100))
        d.polygon([P(p) for p in flat], fill=(225, 228, 232), outline=(110, 114, 120))
        # highlight along the bent leg
        a, b = P(down[0]), P(down[3])
        c, e = P(down[1]), P(down[2])
        d.line([((a[0] * 2 + c[0]) / 3, (a[1] * 2 + c[1]) / 3), ((b[0] * 2 + e[0]) / 3, (b[1] * 2 + e[1]) / 3)],
               fill=(245, 247, 250), width=max(2, N // 200))

    for flat, down in leg_shapes(-1):  # back row first
        pin(flat, down, True)
    # body
    d.polygon([P(p) for p in body_right], fill=(28, 29, 32))
    d.polygon([P(p) for p in body_front], fill=(40, 41, 45))
    d.polygon([P(p) for p in body_top], fill=(58, 60, 64))
    # gloss on the top face: lighter towards the back
    gloss = Image.new('L', (N, N), 0)
    gd = ImageDraw.Draw(gloss)
    for k in range(40):
        f = k / 40
        q = [(-L / 2, -W / 2 + W * f, H), (L / 2, -W / 2 + W * f, H), (L / 2, -W / 2 + W * (f + 1 / 40), H),
             (-L / 2, -W / 2 + W * (f + 1 / 40), H)]
        gd.polygon([P(p) for p in q], fill=int(70 * (1 - f) ** 1.6))
    img.paste((255, 255, 255, 255), (0, 0), gloss)
    d = ImageDraw.Draw(img)
    # edges
    for face in (body_top, body_front, body_right):
        d.line([P(p) for p in face] + [P(face[0])], fill=(14, 14, 16), width=max(2, N // 260))
    # pin-1 notch and dot
    nx, ny = P((-L / 2, 0, H))
    r = scale * 0.32
    d.chord((nx - r, ny - r * 0.55, nx + r, ny + r * 0.55), 270, 90, fill=(30, 31, 34))
    dx, dy = P((-L / 2 + 0.55, W / 2 - 0.55, H))
    rd = scale * 0.13
    d.ellipse((dx - rd, dy - rd * 0.55, dx + rd, dy + rd * 0.55), fill=(36, 37, 40))

    # lettering, drawn flat and mapped onto the top face (affine)
    TW, TH = 1600, 680
    tex = Image.new('RGBA', (TW, TH), (0, 0, 0, 0))
    td = ImageDraw.Draw(tex)
    font = ImageFont.truetype('DejaVuSans-Bold.ttf', 330)
    td.text((TW / 2, TH / 2), '8085 µP', font=font, fill=(242, 244, 247, 255), anchor='mm')
    # texture (0,0)-(TW,TH) covers the top face; find the affine map image->texture
    A, B, C = P(body_top[0]), P(body_top[1]), P(body_top[3])  # tex (0,0), (TW,0), (0,TH)
    ux, uy = (B[0] - A[0]) / TW, (B[1] - A[1]) / TW
    vx, vy = (C[0] - A[0]) / TH, (C[1] - A[1]) / TH
    det = ux * vy - uy * vx
    ia, ib = vy / det, -vx / det
    ic, idd = -uy / det, ux / det
    coeffs = (ia, ib, -(ia * A[0] + ib * A[1]), ic, idd, -(ic * A[0] + idd * A[1]))
    mapped = tex.transform((N, N), Image.AFFINE, coeffs, resample=Image.BICUBIC)
    img.alpha_composite(mapped)

    d = ImageDraw.Draw(img)
    for flat, down in leg_shapes(1):  # front row last
        pin(flat, down, False)

    sizes = [256, 128, 64, 48, 32, 24, 16]
    for s in sizes:
        im = img.resize((s, s), Image.LANCZOS)
        if s <= 32:
            im = im.filter(ImageFilter.UnsharpMask(radius=1, percent=60, threshold=2))
        im.save(os.path.join(HERE, f'icon-{s}.png'), optimize=True)
    img.resize((256, 256), Image.LANCZOS).save(os.path.join(HERE, 'icon.ico'),
                                               sizes=[(s, s) for s in sizes])
    img.save(os.path.join(HERE, 'icon-1024.png'), optimize=True)
    print('icons written')


if __name__ == '__main__':
    main()
