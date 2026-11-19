"""Build the manuals: Markdown -> HTML -> PDF (with LibreOffice).

usage: python3 docs/build.py
"""
import base64
import os
import re
import subprocess

import markdown
from markdown.extensions.toc import slugify_unicode

HERE = os.path.dirname(os.path.abspath(__file__))

CSS = """
body { font-family: 'DejaVu Sans', 'Noto Sans', sans-serif; font-size: 10.5pt; line-height: 1.45;
       max-width: 46em; margin: 2em auto; padding: 0 1em; color: #1a1a18; }
h1 { font-size: 22pt; color: #123b2a; border-bottom: 2px solid #123b2a; padding-bottom: .2em; }
h2 { font-size: 15pt; color: #123b2a; margin-top: 1.6em; border-bottom: 1px solid #c9c6bd; }
h3 { font-size: 12pt; color: #1f5a40; margin-top: 1.2em; }
code { font-family: 'DejaVu Sans Mono', monospace; font-size: 9.5pt; background: #eeece6; padding: 0 .2em; }
pre { background: #eeece6; padding: .6em .8em; border-left: 3px solid #1f5a40; font-size: 9pt; }
pre code { background: none; padding: 0; }
table { border-collapse: collapse; margin: .8em 0; font-size: 9.5pt; }
th, td { border: 1px solid #b8b5ac; padding: .25em .5em; vertical-align: top; text-align: left; }
th { background: #e3e8df; }
img { max-width: 100%; border: 1px solid #c9c6bd; }
table.fig { margin: .6em 0; }
hr { border: none; border-top: 1px solid #c9c6bd; margin: 1.5em 0; }
a { color: #0b5cad; }
"""

# image widths in pixels for the HTML/PDF (LibreOffice uses the attribute)
def img_width(src):
    if any(k in src for k in ('keypad-memory', 'keypad-register', 'keypad-error')):
        return 300
    if 'keypad-board' in src:
        return 300
    if 'lcd-menu' in src or 'lcd-16x2' in src:
        return 360
    if 'rom-menu' in src:
        return 460
    return 600


def build(lang):
    src = os.path.join(HERE, f'manual-{lang}.md')
    with open(src, encoding='utf-8') as f:
        text = f.read()
    body = markdown.markdown(text, extensions=['tables', 'fenced_code', 'toc'],
                             extension_configs={'toc': {'slugify': slugify_unicode}})
    def embed(m):
        # images go inside the HTML, so it is one self-contained file
        # (and LibreOffice doesn't load linked images when making the PDF)
        with open(os.path.join(HERE, m.group(2)), 'rb') as f:
            data = base64.b64encode(f.read()).decode()
        # in a one-cell table: LibreOffice would otherwise flow the next text beside the image
        return ('<table class="fig" border="0" cellpadding="0" cellspacing="0" style="border:none"><tr>'
                '<td style="border:none;padding:0">'
                f'<img alt="{m.group(1)}" src="data:image/png;base64,{data}" width="{img_width(m.group(2))}" />'
                '</td></tr></table><p style="margin:0;font-size:2pt">&nbsp;</p>')

    body = body.replace('<table>', '<table border="1" cellpadding="4" cellspacing="0">')
    body = re.sub(r'<p>(<img [^>]+/>)</p>', r'\1', body)
    body = re.sub(r'<img alt="([^"]*)" src="([^"]+)" />', embed, body)
    title = re.search(r'^# (.+)$', text, re.M).group(1)
    html = (f'<!doctype html>\n<html lang="{lang}">\n<head>\n<meta charset="utf-8">\n'
            f'<meta name="viewport" content="width=device-width, initial-scale=1">\n'
            f'<title>{title}</title>\n<style>{CSS}</style>\n</head>\n<body>\n{body}\n</body>\n</html>\n')
    out = os.path.join(HERE, f'manual-{lang}.html')
    with open(out, 'w', encoding='utf-8') as f:
        f.write(html)
    print('wrote', out)
    r = subprocess.run(['soffice', '--headless', '--convert-to', 'pdf:writer_web_pdf_Export', '--outdir', HERE, out],
                       capture_output=True, text=True, timeout=300)
    print(r.stdout.strip() or r.stderr.strip())


if __name__ == '__main__':
    for lang in ('en', 'el'):
        build(lang)
