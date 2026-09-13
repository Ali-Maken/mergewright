import re, sys, bisect
p = sys.argv[1]; data = open(p, "rb").read(); ROWS = 45
reads = [tuple(l.split()) for l in open(p + ".reads").read().splitlines()]
ro = [int(o) for o, _ in reads]; rt = [float(t) for _, t in reads]
marks = [l.split(" ", 1) for l in open(p + ".marks").read().splitlines()]
def t_of(off): i = bisect.bisect_left(ro, off); return rt[min(i, len(rt) - 1)]
tok = re.compile(rb"\x1b_G([^;\x1b]*)(?:;[^\x1b]*)?\x1b\x5c|\x1b\[(\d*)(?:;(\d*))?H|\x1b\[\?2026l")
row = 0; places = {}; spans = []; cur = None; nframes = 0
for m in tok.finditer(data):
    if m.group(1) is not None:
        kv = dict(x.split(b"=", 1) for x in m.group(1).split(b",") if b"=" in x)
        a = kv.get(b"a", b"t")
        if a == b"p": places[(kv.get(b"i"), kv.get(b"p"))] = (row, int(kv.get(b"r", b"1")))
        elif a == b"d":
            i = kv.get(b"i")
            for k in [k for k in places if k[0] == i]: del places[k]
    elif m.group(0).startswith(b"\x1b[?"):
        nframes += 1
        cov = set(k for r, n in places.values() for k in range(r, r + n))
        miss = [r for r in range(1, ROWS - 3) if r not in cov]
        t = t_of(m.end())
        if miss and cur is None: cur = [t, t, len(miss), miss[:3], m.end()]
        elif miss: cur[1] = t; cur[2] = max(cur[2], len(miss))
        elif cur is not None: spans.append(cur); cur = None
    else:
        row = int(m.group(2) or 1) - 1
if cur: spans.append(cur + ["UNRECOVERED"])
print("frames", nframes, "marks", [n for _, n in marks])
for s in spans: print(f"blank {s[0]:.2f}-{s[1]:.2f}s max_missing={s[2]} first={s[3]} {s[5:] if len(s)>5 else ''}")
