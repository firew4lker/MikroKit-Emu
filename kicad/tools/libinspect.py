import sys, copy
sys.path.insert(0, 'tools')
from sexp import parse, find, find1
LIB = '/usr/share/kicad/symbols/'
_cache = {}
def lib(name):
    if name not in _cache:
        _cache[name] = parse(open(LIB + name + '.kicad_sym').read())
    return _cache[name]
def get(libname, sym):
    L = lib(libname)
    for s in find(L, 'symbol'):
        if s[1] == sym:
            return s
    raise KeyError(sym)
if __name__ == '__main__':
    for spec in sys.argv[1:]:
        l, s = spec.split(':')
        e = get(l, s)
        ext = find1(e, 'extends')
        print('==', spec, 'extends', ext[1] if ext else None)
        base = get(l, ext[1]) if ext else e
        for sub in find(base, 'symbol'):
            pins = find(sub, 'pin')
            if not pins: continue
            out = []
            for p in pins:
                at = find1(p, 'at'); num = find1(p, 'number')[1]; name = find1(p, 'name')[1]
                hid = 'H' if any(x == 'hide' for x in p) or (find1(p,'hide') is not None) else ''
                out.append(f"{num}:{name}:{p[1]}{hid}@{at[1]},{at[2]},{at[3]}")
            print(' ', sub[1], ' '.join(out))
