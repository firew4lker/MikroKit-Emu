"""Minimal S-expression reader/writer for KiCad files."""
import re

class Sym(str):
    """An unquoted atom."""

_tok = re.compile(r'\s*(?:(\()|(\))|"((?:[^"\\]|\\.)*)"|([^\s()"]+))')

def parse(text):
    stack, cur = [], []
    pos = 0
    n = len(text)
    while True:
        m = _tok.match(text, pos)
        if not m:
            if text[pos:].strip():
                raise ValueError(f"bad token at {pos}: {text[pos:pos+40]!r}")
            break
        pos = m.end()
        if m.group(1):
            stack.append(cur); cur = []
        elif m.group(2):
            done = cur; cur = stack.pop(); cur.append(done)
        elif m.group(3) is not None:
            cur.append(m.group(3).replace('\\n', '\n').replace('\\"', '"').replace('\\\\', '\\'))
        else:
            cur.append(Sym(m.group(4)))
        if pos >= n:
            break
    return cur[0] if len(cur) == 1 else cur

def q(s):
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n') + '"'

def dump(e, ind=0):
    if isinstance(e, list):
        simple = all(not isinstance(x, list) for x in e)
        if simple or len(e) <= 2 and all(not isinstance(x, list) or all(not isinstance(y, list) for y in x) for x in e):
            return '(' + ' '.join(dump(x) for x in e) + ')'
        head = []
        rest = []
        for i, x in enumerate(e):
            if isinstance(x, list):
                rest = e[i:]; break
            head.append(x)
        pad = '\t' * (ind + 1)
        return '(' + ' '.join(dump(x) for x in head) + '\n' + '\n'.join(pad + dump(x, ind + 1) for x in rest) + '\n' + '\t' * ind + ')'
    if isinstance(e, Sym):
        return str(e)
    if isinstance(e, bool):
        return 'yes' if e else 'no'
    if isinstance(e, (int, float)):
        return fmt(e)
    return q(e)

def fmt(v):
    if isinstance(v, int):
        return str(v)
    s = f"{v:.4f}".rstrip('0').rstrip('.')
    return '0' if s in ('-0', '') else s

def find(e, key):
    return [x for x in e if isinstance(x, list) and x and x[0] == key]

def find1(e, key):
    r = find(e, key)
    return r[0] if r else None
