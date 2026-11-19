"""Take the manual's screenshots by driving MicroKit-Emu in a virtual X display.

Needs Xvfb running on :99 (Xvfb :99 -screen 0 1280x800x24 &), xdotool,
ImageMagick's `import`, Pillow, and a release build (cargo build --release).

usage: python3 docs/screenshots.py [scene ...]   (no names = all scenes)
       LANGS="el" python3 docs/screenshots.py basic
Afterwards run docs/build.py. The numbered main-window images are made
by numbered() at the end.
"""
import json
import os
import shutil
import subprocess
import sys
import time

import tempfile

S = os.path.join(tempfile.gettempdir(), 'microkit-shots')
os.makedirs(S, exist_ok=True)
EMU = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
OUT = os.path.join(EMU, 'docs', 'images')
os.makedirs(OUT, exist_ok=True)
ENV = dict(os.environ, DISPLAY=':99')
ENV.pop('WAYLAND_DISPLAY', None)

# menu x positions per language
MENU = {
    'en': {'file': 20, 'lcd': 197, 'view': 287},
    'el': {'file': 30, 'lcd': 260, 'view': 428},
}


def xdo(*args):
    subprocess.run(['xdotool', *map(str, args)], env=ENV, check=False)


def settings_ron(lang, extra):
    base = {
        'theme': 'Light',
        'language': 'Greek' if lang == 'el' else 'English',
    }
    base.update(extra)
    return '(' + ','.join(f'{k}:{v}' for k, v in base.items()) + ')'


def jumpers(terminal=False, j18_db9_2=True):
    return f'(j16_terminal:{str(terminal).lower()},j13_trap:true,j18_db9_2:{str(j18_db9_2).lower()},j17:1,j14_8155_ram:true)'


class Run:
    def __init__(self, lang, settings, args=()):
        self.lang = lang
        x = os.path.join(S, 'xdgshot')
        shutil.rmtree(x, ignore_errors=True)
        os.makedirs(os.path.join(x, 'microkit-emu'))
        ron = settings_ron(lang, settings)
        with open(os.path.join(x, 'microkit-emu', 'app.ron'), 'w') as f:
            f.write('{\n    "settings": ' + json.dumps(ron) + ',\n}\n')
        env = dict(ENV, XDG_DATA_HOME=x)
        self.p = subprocess.Popen([os.path.join(EMU, 'target/release/microkit-emu'), *args], cwd=EMU, env=env,
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            r = subprocess.run(['xwininfo', '-root', '-tree'], env=ENV, capture_output=True, text=True)
            if 'MicroKit-Emu' in r.stdout:
                break
            time.sleep(0.25)
        time.sleep(2)
        xdo('search', '--name', 'MicroKit-Emu', 'windowfocus')

    def click(self, x, y, wait=0.3):
        xdo('mousemove', x, y, 'click', 1)
        time.sleep(wait)

    def drag(self, x0, y0, x1, y1):
        xdo('mousemove', x0, y0)
        time.sleep(0.2)
        xdo('mousedown', 1)
        time.sleep(0.3)
        for k in range(1, 21):
            xdo('mousemove', x0 + (x1 - x0) * k // 20, y0 + (y1 - y0) * k // 20)
            time.sleep(0.08)
        time.sleep(0.3)
        xdo('mouseup', 1)
        time.sleep(0.3)

    def move(self, x, y):
        xdo('mousemove', x, y)
        time.sleep(0.2)

    def type(self, text, wait=0.2):
        """Type text; spaces and newlines go as key presses (xdotool drops spaces here)."""
        parts = text.replace('\n', '\r').split(' ')
        for i, part in enumerate(parts):
            for chunk in part.split('\r'):
                pass
            sub = part.split('\r')
            for j, chunk in enumerate(sub):
                if chunk:
                    xdo('type', '--delay', 90, chunk)
                if j < len(sub) - 1:
                    xdo('key', 'Return')
                    time.sleep(0.25)
            if i < len(parts) - 1:
                xdo('key', 'space')
        time.sleep(wait)

    def key(self, k, wait=0.2):
        xdo('key', k)
        time.sleep(wait)

    def wait(self, s):
        time.sleep(s)

    def shot(self, name, crop=None):
        path = os.path.join(OUT, f'{name}-{self.lang}.png')
        tmp = os.path.join(S, 'shot_tmp.png')
        subprocess.run(['import', '-window', 'root', '-crop', '1180x760+0+0', tmp], env=ENV, check=True)
        from PIL import Image
        im = Image.open(tmp)
        if crop:
            im = im.crop(crop)
        im.save(path, optimize=True)
        print('wrote', path, im.size)

    def close(self):
        # the program runs its window in a child process (see supervise in main.rs)
        subprocess.run(['pkill', '-P', str(self.p.pid)], check=False)
        self.p.kill()
        self.p.wait()
        time.sleep(0.5)


def board_focus(r):
    r.click(185, 393)  # empty part of the board, under the keypad


def term_focus(r):
    r.click(780, 500)


# ----------------------------------------------------------------------------
def main_window(lang):
    r = Run(lang, {'jumpers': jumpers()})
    r.wait(0.5)
    r.shot('main-window')
    r.close()


def keypad_memory(lang):
    r = Run(lang, {'jumpers': jumpers()}, ['examples/outputs.asm'])
    board_focus(r)
    r.type('m2000n', 0.5)
    r.shot('keypad-memory', (8, 64, 364, 160))
    r.type('n', 0.5)
    r.shot('keypad-memory-next', (8, 64, 364, 160))
    r.type('.r', 0.5)
    r.shot('keypad-register', (8, 64, 364, 160))
    r.type('.', 0.3)
    r.close()


def keypad_boot(lang):
    r = Run(lang, {'jumpers': jumpers()})
    r.shot('keypad-board', (0, 55, 372, 410))
    board_focus(r)
    r.type('3', 0.4)
    r.shot('keypad-error', (8, 64, 364, 160))
    r.close()


def terminal(lang):
    r = Run(lang, {'jumpers': jumpers(terminal=True)}, ['examples/outputs.asm'])
    term_focus(r)
    r.type('D2000,203F\r', 1.2)
    r.type('X\r', 1.0)
    r.shot('terminal-session')
    r.type('H', 1.5)
    r.shot('terminal-help', (372, 55, 1180, 694))
    r.key('Escape')
    r.close()


def rom_menu(lang):
    r = Run(lang, {'jumpers': jumpers(terminal=True)})
    m = MENU[lang]
    r.click(m['file'], 12, 0.4)
    r.move(m['file'] + 40, 117)
    r.move(m['file'] + 60, 117)
    r.wait(0.5)
    r.shot('rom-menu', (0, 0, 560, 240))
    r.close()


def basic(lang):
    r = Run(lang, {'jumpers': jumpers(terminal=True), 'rom': 'Builtin("basic")'})
    term_focus(r)
    r.type('B', 1.0)
    for line in ['10 FOR I=1 TO 8', '20 PRINT I, I*I, 1000/I', '30 NEXT I', 'LIST', 'RUN']:
        r.type(line + '\r', 0.8)
    r.wait(1.5)
    r.shot('basic', (372, 55, 1180, 694))
    r.close()


def lcd(lang):
    r = Run(lang, {'jumpers': jumpers(), 'lcd': 'L20x4', 'show_lcd': 'true'}, ['examples/lcd.asm'])
    board_focus(r)
    r.type('g.', 1.5)
    r.shot('lcd-20x4')
    r.close()
    r = Run(lang, {'jumpers': jumpers(), 'lcd': 'L16x2', 'show_lcd': 'true', 'lcd_blue': 'true'}, ['examples/lcd.asm'])
    board_focus(r)
    r.type('g.', 1.5)
    r.shot('lcd-16x2-blue', (420, 90, 900, 270))
    r.close()


def lcd_menu(lang):
    r = Run(lang, {'jumpers': jumpers()})
    r.click(MENU[lang]['lcd'], 12, 0.6)
    r.shot('lcd-menu', (MENU[lang]['lcd'] - 20, 0, MENU[lang]['lcd'] + 300, 240))
    r.close()


def ports(lang):
    r = Run(lang, {'jumpers': jumpers(), 'show_ports': 'true'}, ['examples/outputs.asm'])
    board_focus(r)
    r.type('g.', 1.0)
    r.shot('io-ports')
    r.close()


def debug(lang):
    r = Run(lang, {'jumpers': jumpers(), 'show_debug': 'true'}, ['examples/count.asm'])
    board_focus(r)
    r.type('g.', 1.5)
    r.shot('debug-panel')
    r.close()


RECORD = {'en': (830, 576), 'el': (895, 576)}


def tape(lang):
    r = Run(lang, {'jumpers': jumpers(terminal=True, j18_db9_2=False), 'show_tape': 'true'}, ['examples/outputs.asm'])
    r.drag(590, 137, 880, 470)
    term_focus(r)
    r.type('W', 0.8)
    r.type('2000\r', 0.8)
    r.type('201F\r', 0.8)
    if RECORD[lang] is None:
        r.shot('tape-before')
        r.close()
        return
    r.click(*RECORD[lang])
    term_focus(r)
    r.key('Return', 8.0)
    r.shot('tape-recorded')
    r.close()


def dark(lang):
    r = Run(lang, {'jumpers': jumpers(terminal=True), 'theme': 'Dark'}, ['examples/outputs.asm'])
    term_focus(r)
    r.type('D2000,201F\r', 1.2)
    r.shot('dark-theme')
    r.close()


ALL = [main_window, keypad_boot, keypad_memory, terminal, rom_menu, basic, lcd, lcd_menu, ports, debug, tape, dark]

def numbered():
    """Red numbered markers on the main-window screenshot (see section 3)."""
    from PIL import Image, ImageDraw, ImageFont
    font = ImageFont.truetype('DejaVuSans-Bold.ttf', 18)
    marks = {
        'en': [(1, 445, 12), (2, 905, 37), (3, 360, 108), (4, 360, 280), (5, 1150, 120), (6, 535, 711), (7, 300, 743)],
        'el': [(1, 600, 12), (2, 1125, 37), (3, 360, 108), (4, 360, 280), (5, 1150, 120), (6, 640, 711), (7, 560, 743)],
    }
    for lang, ms in marks.items():
        src = os.path.join(OUT, f'main-window-{lang}.png')
        if not os.path.exists(src):
            continue
        im = Image.open(src).convert('RGB')
        d = ImageDraw.Draw(im)
        for n, x, y in ms:
            d.ellipse((x - 13, y - 13, x + 13, y + 13), fill=(200, 30, 30), outline='white', width=2)
            d.text((x, y), str(n), fill='white', font=font, anchor='mm')
        im.save(os.path.join(OUT, f'main-window-numbered-{lang}.png'), optimize=True)


if __name__ == '__main__':
    want = sys.argv[1:]
    for lang in (['en', 'el'] if not os.environ.get('LANGS') else os.environ['LANGS'].split()):
        for f in ALL:
            if not want or f.__name__ in want:
                f(lang)
    numbered()
