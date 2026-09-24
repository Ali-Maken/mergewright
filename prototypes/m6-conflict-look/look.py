#!/usr/bin/env python3
"""PROTOTYPE — throwaway. Deleted when the M6 look-and-finish specs land.

Pipeline step 5 for docs/design-notes/m6-conflicts.md, "Amendment 2026-09-24: look and finish",
subsection "Prototype (pipeline step 5)". Wizard of Oz: every frame is canned text, nothing reads git.

  python3 prototypes/m6-conflict-look/look.py              # task first, then the ten scenes
  python3 prototypes/m6-conflict-look/look.py --no-task    # straight to the scenes
  python3 prototypes/m6-conflict-look/look.py --width 120  # draw every scene at 120 (also 80)

Keys, the same in every scene:
  ] / [   next / previous scene      0   "What you're deciding"
  1-4     cycle the scene's knobs    < / >   redraw at 80 · 100 · 120 · 160 · 187 columns
  *       mark what is on screen as your pick      Backspace   restart the scene
  q       quit: prints the picks, appends them to .scratch/m6-conflicts/look-picks.txt
  Ctrl-C  quits from anywhere (q is typed text while a text field has focus)
  any other key is the app's own key, acted out on canned frames.

Colour values (the user judges these; nothing here is verified by a test — CLAUDE.md rules 5, 6):
  * bg, text, text-muted, border, accent, selection, added, removed: the dark theme,
    crates/graft-ui/src/theme/chrome.rs:39-52 (`DARK`). Their 256 / 16 entries:
    crates/graft-ui/src/theme/snapshots/graft_ui__theme__tests__resolved_values.snap:7-18.
  * ours, theirs, open and the three tints: the Design table of the amendment (m6-conflicts.md:941-948);
    tint = mix(fg, bg, alpha) (:950). "256 nearest" ports nearest_256 from
    crates/graft-ui/src/theme/depth.rs:29,131 (OKLab, chroma weighted x2, entries 16-255).
  * The prototype paints bg #14161B at truecolor and 256, as the resolver and summary do
    (crates/graft-ui/src/finish_summary.rs:160-163); the tints are mixed against that bg.

As-built strings are copied from the captures and sources named beside them. Words not built yet are
tagged `draft` on the knob row. Approximations (said here, not hidden):
  * the key-row ladder is a port of crates/graft-ui/src/graph_screen/hints/fitting.rs:50-100;
  * the stop line's rung choice (scene 6) is inferred from crates/graft/src/update/banner/stop.rs:190-215
    and docs/shots/m6/fix-landing-b-launch-187x50.txt:50, not ported;
  * the 187-column file column cuts long names with `…` at 32 cells (COLUMN_MAX), not ported;
  * scene 10 uses the 12-file shape of fix-file-list-many-120x40.txt (its strip reads `… 7 more`),
    not the note's 9 files, so the strip is the as-built one byte for byte;
  * the notes.txt words on the summary for mixed takes, and the graph after a commit, are guesses.
"""
import argparse
import codecs
import datetime
import functools
import os
import re
import select
import signal
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))
PICKS_DEFAULT = os.path.join(REPO, ".scratch", "m6-conflicts", "look-picks.txt")

# ------------------------------------------------------------------ colour

CHROME = {  # crates/graft-ui/src/theme/chrome.rs:39-52
    "bg": "#14161B", "selection": "#5A6478", "border": "#333944", "text": "#D7DBE0",
    "text-muted": "#8A919C", "accent": "#4FA3E3", "added": "#5DB87A", "removed": "#E06C75",
}
C256 = {"bg": 233, "selection": 60, "border": 237, "text": 253, "text-muted": 246, "accent": 75,
        "added": 71, "removed": 167}  # resolved_values.snap:7-18
C16 = {"bg": 0, "selection": 8, "border": 8, "text": 7, "text-muted": 8, "accent": 6, "added": 2,
       "removed": 9, "ours": 6, "theirs": 5, "open": 3}  # snap, and the amendment's table
OPEN = "#E69F00"
PAIRS = [("okabe", "#56B4E9", "#CC79A7"), ("teal-magenta", "#2AA198", "#D33682"),
         ("vs code", "#40C8AE", "#40A6FF")]
FIXED256 = {"ours": 74, "theirs": 175, "open": 214, "ours-tint": 23, "theirs-tint": 53, "open-tint": 58}
ALPHAS = [0.20, 0.30, 0.40]
DEPTHS = ["truecolor", "256 fixed", "256 nearest", "16", "no colour"]
ANSI16 = [(0, 0, 0), (205, 0, 0), (0, 205, 0), (205, 205, 0), (0, 0, 238), (205, 0, 205), (0, 205, 205),
          (229, 229, 229), (127, 127, 127), (255, 0, 0), (0, 255, 0), (255, 255, 0), (92, 92, 255),
          (255, 0, 255), (0, 255, 255), (255, 255, 255)]


def hx(s):
    return tuple(int(s[i:i + 2], 16) for i in (1, 3, 5))


def mix(fg, bg, a):
    return tuple(round(f * a + b * (1 - a)) for f, b in zip(fg, bg))


def x256(i):
    if i < 16:
        return ANSI16[i]
    if i < 232:
        i -= 16
        lv = [0, 95, 135, 175, 215, 255]
        return (lv[i // 36], lv[i // 6 % 6], lv[i % 6])
    g = 8 + 10 * (i - 232)
    return (g, g, g)


def oklab(c):
    def lin(v):
        v /= 255
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(v) for v in c)
    l_ = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) ** (1 / 3)
    m_ = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) ** (1 / 3)
    s_ = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) ** (1 / 3)
    return (0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_)


@functools.lru_cache(maxsize=None)
def nearest(c, lo, hi):
    """Nearest xterm entry in lo..hi-1: OKLab, chroma axes weighted x2 (depth.rs:29, :131)."""
    a = oklab(c)

    def d(i):
        b = oklab(x256(i))
        return (a[0] - b[0]) ** 2 + (2 * (a[1] - b[1])) ** 2 + (2 * (a[2] - b[2])) ** 2
    return min(range(lo, hi), key=d)


def lum(c):
    def lin(v):
        v /= 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(v) for v in c)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


class Pal:
    """Turns a style (fg token, bg token, attrs) into SGR at one colour depth.

    attrs: b bold, r reverse, d dim, N not-taken (DIM at no colour), O open (BOLD at no colour),
    R reverse at no colour only (the finish row's `selection` fill, the summary's cursor row)."""

    def __init__(self, depth, pair, alpha):
        self.depth, self.pair, self.alpha = depth, pair, alpha

    def rgb(self, tok):
        if tok in CHROME:
            return hx(CHROME[tok])
        if tok == "ours":
            return hx(PAIRS[self.pair][1])
        if tok == "theirs":
            return hx(PAIRS[self.pair][2])
        if tok == "open":
            return hx(OPEN)
        if tok.endswith("-tint"):
            return mix(self.rgb(tok[:-5]), hx(CHROME["bg"]), self.alpha)
        raise KeyError(tok)

    def idx256(self, tok):
        if tok in C256:
            return C256[tok]
        if self.depth == "256 fixed":
            if self.pair == 0 and tok in FIXED256:
                return FIXED256[tok]
            # No fixed entries were chosen for this pair: the cube only, never the grey ramp.
            return nearest(self.rgb(tok), 16, 232)
        return nearest(self.rgb(tok), 16, 256)

    def shown_rgb(self, tok):
        if self.depth == "truecolor":
            return self.rgb(tok)
        if self.depth.startswith("256"):
            return x256(self.idx256(tok))
        return None

    def sgr(self, st):
        fg, bg, at = st
        p = ["0"]
        if "b" in at:
            p.append("1")
        if "d" in at:
            p.append("2")
        if "r" in at:
            p.append("7")
        d = self.depth
        if d == "no colour":
            if "N" in at:
                p.append("2")
            if "O" in at:
                p.append("1")
            if "R" in at:
                p.append("7")
            return "\x1b[" + ";".join(p) + "m"
        if d == "16":
            if fg:
                n = C16[fg[:-5]] if fg.endswith("-tint") else C16[fg]
                p.append(str(30 + n) if n < 8 else str(90 + n - 8))
            if bg:
                n = C16["bg"] if bg.endswith("-tint") else C16[bg]  # no tints at 16 colours
                p.append(str(40 + n) if n < 8 else str(100 + n - 8))
            return "\x1b[" + ";".join(p) + "m"
        if fg:
            p.append(self.code(fg, 38))
        if bg:
            p.append(self.code(bg, 48))
        return "\x1b[" + ";".join(p) + "m"

    def code(self, tok, base):
        if self.depth == "truecolor":
            r, g, b = self.rgb(tok)
            return f"{base};2;{r};{g};{b}"
        return f"{base};5;{self.idx256(tok)}"

    def info(self):
        """The scene-1 bottom row numbers: weakest tint against bg, weakest text-on-tint."""
        if self.depth in ("16", "no colour"):
            return "tint none"
        bg = self.shown_rgb("bg")
        tints = [self.shown_rgb(t) for t in ("ours-tint", "theirs-tint", "open-tint")]
        tb = min(contrast(t, bg) for t in tints)
        tt = min(contrast(self.shown_rgb("text"), t) for t in tints)
        return f"{tb:.1f}:1 · text {tt:.1f}:1"


BASE = ("text", "bg", "")


def S(fg="text", bg=None, at=""):
    return (fg, bg, at)


# ------------------------------------------------------------------ canvas

class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.c = [[[" ", BASE] for _ in range(w)] for _ in range(h)]

    def put(self, x, y, text, st=S()):
        if not 0 <= y < self.h:
            return x + len(text)
        for i, ch in enumerate(text):
            if 0 <= x + i < self.w:
                cell = self.c[y][x + i]
                bg = st[1] if st[1] is not None else cell[1][1]
                cell[0], cell[1] = ch, (st[0], bg, st[2])
        return x + len(text)

    def paint(self, x, y, w, bg=None, add=""):
        if not 0 <= y < self.h:
            return
        for i in range(max(0, x), min(self.w, x + w)):
            fg, b, at = self.c[y][i][1]
            self.c[y][i][1] = (fg, bg if bg is not None else b, at + add)

    def render(self, pal, top):
        out = []
        for y, row in enumerate(self.c):
            out.append(f"\x1b[{top + y};1H")
            last = None
            for ch, st in row:
                if st != last:
                    out.append(pal.sgr(st))
                    last = st
                out.append(ch)
            out.append("\x1b[0m\x1b[K")
        return "".join(out)


def cut(s, n):
    if n <= 0:
        return ""
    return s if len(s) <= n else s[:n - 1] + "…"


# ------------------------------------------------------------------ key rows (the ladder)

SHORT_BUILT = {  # crates/graft-ui/src/graph_screen/hints.rs:169-182
    "first-parent": "first", "continue": "cont", "discard": "disc", "hide stash": "hide",
    "overview": "over", "renderer": "rend", "status": "stat", "revert": "rev", "lanes": "lane"}
SHORT_DRAFT = [  # .scratch/m5-history-and-ghost-preview/issues/24-the-ladder-and-the-stale-banner.md:24-36
    ("first-parent", "first"), ("continue", "cont"), ("mergetool", "tool"), ("discard", "disc"),
    ("hide stash", "hide"), ("journal", "log"), ("overview", "over"), ("renderer", "rend"),
    ("explain", "why"), ("search", "find"), ("changes", "diff"), ("status", "stat"),
    ("compare", "cmp"), ("resize", "size"), ("commit", "cmt"), ("revert", "rev"), ("stage", "stg"),
    ("abort", "stop"), ("apply", "use"), ("graph", "grph"), ("close", "shut"), ("lanes", "lane")]


def htext(h, short):
    key, verb, rung = h
    if rung == "key":
        return key
    return key + " " + (short[verb] if rung == "short" else verb)


def hints_text(hs, short):
    return "".join((" " if i == 0 else "  ") + htext(h, short) for i, h in enumerate(hs))


def fit_hints(lst, room, short, trailer=None):
    """Port of fit_list (fitting.rs:77-100): the floor of three, then bare keys, then raises."""
    def rungs(v):
        return ["key", "short", "full"] if v in short else ["key", "full"]

    def fits(d):
        m = d + ([(trailer[0], trailer[1], "full")] if trailer else [])
        return len(hints_text(m, short)) <= room
    floor = min(3, len(lst))
    drawn = [(k, v, "short" if v in short else "full") for k, v in lst[:floor]]
    if not fits(drawn):
        if trailer:
            for r in reversed(rungs(trailer[1])):
                if len(hints_text([(trailer[0], trailer[1], r)], short)) <= room:
                    return [(trailer[0], trailer[1], r)]
        return []
    steps = [("raise", i, "full") for i in range(floor) if lst[i][1] in short]
    steps += [("add",)] * (len(lst) - floor)
    for i in range(floor, len(lst)):
        steps += [("raise", i, r) for r in rungs(lst[i][1])[1:]]
    for st in steps:
        cand = list(drawn)
        if st[0] == "add":
            k, v = lst[len(cand)]
            cand.append((k, v, "key"))
        else:
            k, v, _ = cand[st[1]]
            cand[st[1]] = (k, v, st[2])
        if not fits(cand):
            break
        drawn = cand
    if trailer:
        pos = next((i for i, h in enumerate(drawn) if h[2] == "key"), len(drawn))
        drawn.insert(pos, (trailer[0], trailer[1], "full"))
    return drawn


def bar(lst, room, short=None, trailer=None):
    short = SHORT_BUILT if short is None else short
    return hints_text(fit_hints(lst, room, short, trailer), short)


def resolver_list(base, restart, roles):  # crates/graft-ui/src/graph_screen/hints/lists.rs:291-317
    lst = [("o", roles[0]), ("t", roles[1]), ("b", "both"), ("⌃O/⌃T", "all"), ("O/T", "file")]
    if base:
        lst.append(("B", "base"))
    lst += [("e", "edit"), ("n", "next"), ("N", "prev"), ("⏎", "done"), ("j/k", "file"), ("M", "mergetool")]
    if restart:
        lst.append(("d", "start over"))
    lst.append(("esc", "close"))
    return lst


# ------------------------------------------------------------------ the conflict document

MERGE = {"ours": "yours · main", "theirs": "theirs · feature", "roles": ("yours", "theirs"), "head": "MERGING"}
REBASE = {"ours": "upstream · main", "theirs": "yours · feature", "roles": ("upstream", "yours"),
          "head": "REBASING"}


class Doc:
    def __init__(self, segs):
        self.segs = segs
        self.state = [None for s in segs if s[0] == "b"]

    def blocks(self):
        return [s for s in self.segs if s[0] == "b"]

    def open_left(self):
        return sum(1 for s in self.state if s is None)

    def result_lines(self):
        out, bi = [], 0
        for s in self.segs:
            if s[0] == "c":
                out += s[1]
            else:
                st = self.state[bi]
                out += s[1] if st == "ours" else s[2] if st == "theirs" else s[1] + s[2] if st == "both" else []
                bi += 1
        return out

    def rows(self):
        """Three-pane rows, level across the panes (ADR-0043 §2)."""
        rows, ny, nr, nt, bi = [], 1, 1, 1, 0
        for s in self.segs:
            if s[0] == "c":
                for ln in s[1]:
                    rows.append({"blk": None, "y": (ny, ln), "r": (nr, ln), "t": (nt, ln)})
                    ny, nr, nt = ny + 1, nr + 1, nt + 1
                continue
            ours, theirs, st = s[1], s[2], self.state[bi]
            res = [] if st is None else ours if st == "ours" else theirs if st == "theirs" else ours + theirs
            span = max(len(ours), len(theirs), len(res) if st else 1)
            rows.append({"blk": bi, "rule": True, "i": -1})
            for i in range(span):
                r = ("open",) if st is None and i == 0 else ((nr + i, res[i]) if st and i < len(res) else None)
                rows.append({"blk": bi, "i": i, "last": i == span - 1,
                             "y": (ny + i, ours[i]) if i < len(ours) else None,
                             "t": (nt + i, theirs[i]) if i < len(theirs) else None, "r": r})
            ny, nt = ny + len(ours), nt + len(theirs)
            nr += len(res) if st else max(len(ours), len(theirs))
            bi += 1
        return rows


def notes_doc():  # docs/shots/m6/m6-12-after-t-187x50.txt:7-16
    return Doc([("c", ["one"]), ("b", ["two from main"], ["TWO from feature"]),
                ("c", ["three", "four", "five", "six"]), ("b", ["seven from main"], ["SEVEN from feature"]),
                ("c", ["eight"])])


class F:
    def __init__(self, name, doc=None, card=None, done=False, visited=False, kept=None):
        self.name, self.doc, self.card, self.done, self.visited, self.kept = name, doc, card, done, visited, kept

    def words(self, mark="…"):
        if self.done:
            return "done ✓"
        if self.card:
            return "keep or delete"
        if not self.visited:
            return mark
        if self.doc.open_left() == 0:
            return "⏎ marks done"
        return f"{self.doc.open_left()} of {len(self.doc.state)} open"

    def short_words(self):  # docs/shots/m6/m6-12-after-t-80x24.txt:4
        w = self.words()
        if w == "keep or delete":
            return "keep?"
        if w.endswith(" open"):
            return w.replace(" of ", "/")
        return w


class Res:
    """The resolver, acted out: takes, n/N, ⏎ done, the notice row, the bar."""

    def __init__(self, files, cur=0, names=MERGE, base=False, restart=True):
        self.files, self.cur, self.names, self.base, self.restart = files, cur, names, base, restart
        self.cursor = 0
        self.notice = None  # (text, kind) kind: plain | finish
        self.status = None
        self.finish_first = False
        self.all_done_head = False
        self.mark = "…"
        f = self.f()
        f.visited = True
        if f.doc:
            self.cursor = next((i for i, s in enumerate(f.doc.state) if s is None), 0)

    def f(self):
        return self.files[self.cur]

    def all_done(self):
        return all(f.done for f in self.files)

    def side_name(self, side):
        return {"ours": self.names["ours"], "theirs": self.names["theirs"], "both": "both"}[side]

    def take(self, side):
        f = self.f()
        if f.done or not f.doc:
            return False
        d = f.doc
        d.state[self.cursor] = side
        # crates/graft/src/update/resolver/answers/taken.rs:169-172, Status::done adds ` ✓`
        self.status = f"took {self.side_name(side)} in {f.name} · {d.open_left()} of {len(d.state)} open ✓"
        n = len(d.state)
        nxt = next((j % n for j in range(self.cursor + 1, self.cursor + n + 1) if d.state[j % n] is None), None)
        if nxt is None:
            self.notice = (f"all blocks taken · ⏎ marks {f.name} done", "plain")  # take.rs:218
        else:
            self.cursor, self.notice = nxt, None
        return True

    def walk(self, forward, no_more=None):
        """n / N, crates/graft/src/update/resolver/walk.rs:66-84 (no cross: the other files are done)."""
        f = self.f()
        if f.done or not f.doc:
            return False
        n = len(f.doc.state)
        j = self.cursor + (1 if forward else -1)
        if 0 <= j < n:
            self.cursor = j
            return True
        opens = [i for i, s in enumerate(f.doc.state) if s is None]
        if opens:
            self.cursor = opens[0] if forward else opens[-1]
            self.notice = (f"wrapped · {len(opens)} still open", "plain")
            return True
        self.status = no_more or f"no more conflicts · ⏎ marks {f.name} done"  # walk.rs:88-99
        return True

    def enter(self):
        """⏎: refuse while open (operation.rs:648-650), else the file is done — the caller's event."""
        f = self.f()
        if f.done or not f.doc:
            return None
        left = f.doc.open_left()
        if left:
            noun = "conflict" if left == 1 else "conflicts"
            self.status = f"{left} {noun} still open in {f.name} · n next"
            return None
        f.done = True
        self.notice = None
        return "done"

    def hints(self):
        f = self.f()
        roles = self.names["roles"]
        if f.done:  # lists.rs:343-351 and offering_c :108-119
            lst = ([("d", "start over")] if self.restart else []) + [("e", "edit")]
            if self.all_done():
                c = ("c", "finish" if self.names is MERGE else "continue")
                if self.finish_first:
                    lst = [c] + lst
                else:
                    lst.append(c)
            return lst + [("esc", "close")]
        if f.card:
            return [("o", roles[0]), ("t", roles[1]), ("e", "edit"), ("n", "next"), ("M", "mergetool"),
                    ("esc", "close")]
        return resolver_list(self.base, self.restart, roles)


# ------------------------------------------------------------------ drawing the resolver

def state_style(words):
    if words in ("done ✓", "✓"):
        return S("text-muted")
    if words == "…" or words == "":
        return S()
    return S("open")


def draw_bar_row(cv, y, text, st=S()):
    cv.put(0, y, text[:cv.w], st)


def draw_resolver(cv, res, look, bar_text=None):
    W, H = cv.w, cv.h
    if W >= 180:
        draw_three(cv, res, look)
    else:
        draw_card_mode(cv, res, look)
    if bar_text is None:
        bar_text = " " + res.status if res.status else bar(res.hints(), W)
    draw_bar_row(cv, H - 1, bar_text)


def identity(res):
    f = res.f()
    if f.done:
        return f"▸ {f.name} · done ✓" + (" · d starts it over" if res.restart else "")
    if f.card:
        return f"▸ {f.name} · modified on {res.names['ours']}, deleted on {res.names['theirs']}"
    n = len(f.doc.state)
    return f"▸ {f.name} · {n} conflict{'s' if n != 1 else ''} · {f.doc.open_left()} open"


def draw_notice(cv, x, y, res):
    if not res.notice:
        return
    text, kind = res.notice
    if kind == "finish":  # the finish row: accent BOLD on a full-width selection fill; NO_COLOR reversed
        cv.paint(x, y, cv.w - x, "selection", "R")
        cv.put(x, y, text, S("accent", None, "bR"))
    else:
        cv.put(x, y, text)


def draw_three(cv, res, look):
    W, H = cv.w, cv.h
    head =(f"FILES · all {len(res.files)} done ✓" if res.all_done() and res.all_done_head
            else f"FILES · {len(res.files)} conflicted")
    nlen = max(len(f.name) for f in res.files) + 2
    entries = []
    for i, f in enumerate(res.files):
        pre = "▏▸ " if i == res.cur else "   "
        entries.append((pre, f.name.ljust(nlen), f.words(res.mark)))
    lw = max([len(head)] + [len(a + b + c) for a, b, c in entries])
    lw = min(32, max(22, lw))
    cv.put(0, 0, res.names["head"])
    cv.put(0, 1, head, S("text-muted" if res.all_done() and res.all_done_head else "text"))
    for y in range(1, H - 1):
        cv.put(lw, y, "│", S("border"))
    for i, (pre, name, words) in enumerate(entries):
        y = 3 + i
        if y >= H - 1:
            break
        # A name too long for the column is cut with `…` so its words still show (COLUMN_MAX 32).
        room = lw - 3 - (len(words) if words else 0)
        if len(name) > room:
            name = cut(name.rstrip(), room - 1) + " "
        if pre.strip():
            cv.put(0, y, "▏", S("accent"))
            x = cv.put(1, y, "▸ " + name)
        else:
            x = cv.put(0, y, "   " + name)
        cv.put(x, y, words, state_style(words))
    x0 = lw + 1
    cv.put(x0, 1, identity(res))
    draw_notice(cv, x0, 2, res)
    f = res.f()
    if f.done:
        for i, ln in enumerate(f.doc.result_lines() if f.doc else [f.kept or ""]):
            cv.put(x0, 3 + i, f"  {i + 1} {ln}")
        return
    if f.card:
        pw = (W - x0 - 2) // 3
        xs = [x0, x0 + pw + 1, x0 + 2 * pw + 2]
        titles = [" " + res.names["ours"], " result", " " + res.names["theirs"]]
        for x, t, tok in zip(xs, titles, ("ours", "text", "theirs")):
            cv.put(x, 3, t, S(tok, None, "b"))
        for x in xs[1:]:
            cv.put(x - 1, 3, "│", S("border"))
        cv.put(x0, 4, f"deleted by them · o keeps the file  t deletes it")
        cv.put(xs[0], 5, " " + f"1 {f.card}")
        cv.put(xs[1], 5, "   ? keep or delete · o t", S("open", None, "bO"))
        cv.put(xs[2], 5, f"   (deleted on {res.names['theirs']})", S("text-muted"))
        for x in xs[1:]:
            cv.put(x - 1, 5, "│", S("border"))
        return
    draw_panes(cv, res, look, x0)


def draw_panes(cv, res, look, x0):
    W = cv.w
    pw = (W - x0 - 2) // 3
    xs = [x0, x0 + pw + 1, x0 + 2 * pw + 2]
    ws = [pw, pw, W - xs[2]]
    seps = [x0 + pw, x0 + 2 * pw + 1]
    lines = look.get("lines", 0)
    names = res.names
    for x, t, tok in zip(xs, (" " + names["ours"], " result", " " + names["theirs"]), ("ours", "text", "theirs")):
        cv.put(x, 3, t, S(tok, None, "b"))
    for s in seps:
        cv.put(s, 3, "│", S("border"))
    doc = res.f().doc
    n = len(doc.state)
    for k, row in enumerate(doc.rows()):
        y = 4 + k
        if y >= cv.h - 1:
            break
        b = row["blk"]
        if b is None:
            for p, key in enumerate(("y", "r", "t")):
                num, ln = row[key]
                cv.put(xs[p], y, f" {num} {ln}"[:ws[p]])
            for s in seps:
                cv.put(s, y, "│", S("border"))
            continue
        st = doc.state[b]
        cur = b == res.cursor
        rule = row.get("rule", False)
        i = row["i"]
        blk = doc.blocks()[b]
        tint_y = "ours-tint" if st in (None, "ours", "both") else None
        tint_t = "theirs-tint" if st in (None, "theirs", "both") else None
        if st is None:
            tint_r = "open-tint"
        elif st == "both":
            tint_r = "ours-tint" if rule or i < len(blk[1]) else "theirs-tint"
        else:
            tint_r = st + "-tint"
        tints = [tint_y, tint_r, tint_t]
        for p in range(3):
            if tints[p]:
                cv.paint(xs[p], y, ws[p], tints[p])
        kept_y = st in ("ours", "both")
        kept_t = st in ("theirs", "both")
        gut = "▏" if cur else " "
        if rule:
            wy = ("── o take", "ours", "") if st is None else (
                ("── taken ✓", "ours", "") if kept_y else ("── not taken", "text-muted", "N"))
            wt = ("── t take", "theirs", "") if st is None else (
                ("── taken ✓", "theirs", "") if kept_t else ("── not taken", "text-muted", "N"))
            what = "open" if st is None else ("took both" if st == "both" else "took " + res.side_name(st))
            rt = "open" if st is None else ("text" if st == "both" else st)
            wr = (f"── {b + 1} of {n} · {what}", rt, "O" if st is None else "")
            for p, (txt, tok, at) in enumerate((wy, wr, wt)):
                lead = gut if p == 0 else " "
                cv.put(xs[p], y, (lead + txt)[:ws[p]], S(tok, None, at))
                if cur:  # the cursor block's rule row, reversed across its pane (panes/draw.rs:116-121)
                    cv.paint(xs[p], y, ws[p], None, "r")
            if cur:
                cv.put(xs[0], y, "▏", S("accent", None, "r"))
        else:
            if row["y"]:
                num, ln = row["y"]
                cv.put(xs[0], y, f"{gut}{num} {ln}"[:ws[0]],
                       S("text") if st is None or kept_y else S("text-muted", None, "N"))
            elif cur:
                cv.put(xs[0], y, gut)
            if cur:
                cv.put(xs[0], y, "▏", S("accent"))
            if row["t"]:
                num, ln = row["t"]
                cv.put(xs[2], y, f" {num} {ln}"[:ws[2]],
                       S("text") if st is None or kept_t else S("text-muted", None, "N"))
            if row["r"] == ("open",):
                cv.put(xs[1], y, "   ? open · o t b", S("open", None, "bO"))
            elif row["r"]:
                num, ln = row["r"]
                cv.put(xs[1], y, f" {num} {ln}"[:ws[1]])
        # separators and brackets
        if lines == 0:  # A: glyphs and the band
            if st is None:
                left = right = ("?" if rule else "┃", "open", "open-tint")
            else:
                left = ("▶" if rule else "┃", "ours", "ours-tint") if kept_y else ("┊", "text-muted", None)
                right = ("◀" if rule else "┃", "theirs", "theirs-tint") if kept_t else ("┊", "text-muted", None)
            for s, (g, tok, bg) in zip(seps, (left, right)):
                cv.put(s, y, g, (tok, bg, "b" if cur else ""))
        else:
            for s in seps:
                cv.put(s, y, "│", S("border"))
            if lines == 1:  # B: VS Code's gutter bracket beside each side's block
                last = row.get("last", False)
                gy = "┐" if rule else ("┘" if last else "│")
                gt = "┌" if rule else ("└" if last else "│")
                ty = "open" if st is None else ("ours" if kept_y else "text-muted")
                tt = "open" if st is None else ("theirs" if kept_t else "text-muted")
                cv.put(xs[0] + ws[0] - 1, y, gy, S(ty, None, "b" if cur else ""))
                cv.put(xs[2], y, gt, S(tt, None, "b" if cur else ""))
    for y in range(4 + len(doc.rows()), cv.h - 1):
        for s in seps:
            cv.put(s, y, " ")


def draw_strip(cv, res, y):
    W = cv.w
    if len(res.files) > 2:
        return draw_strip_many(cv, res, y)
    label = "files · all done ✓" if res.all_done() and res.all_done_head else "files"
    x = cv.put(0, y, label + "   ")
    for i, f in enumerate(res.files):
        pre = "▸ " if i == res.cur else ""
        if W >= 100:
            x = cv.put(x, y, pre + f.name + " · ")
            words = f.words(res.mark)
            x = cv.put(x, y, words, state_style(words))
            x = cv.put(x, y, "    ")
        else:
            x = cv.put(x, y, pre + f.name + " ")
            words = f.short_words()
            x = cv.put(x, y, words, state_style(words))
            x = cv.put(x, y, "   ")
    return y + 1


def draw_strip_many(cv, res, y):
    """fix-file-list-many-120x40.txt:2-4: two columns, three rows, `… n more` in the last slot."""
    n = len(res.files)
    cv.put(0, y, f"files · {n}")
    shown = res.files[:5] if n > 6 else res.files
    ents = []
    for i, f in enumerate(shown):
        w = f.words(res.mark)
        ents.append(("▸ " if res.files.index(f) == res.cur else "  ", f.name, w))
    if n > 6:
        ents.append(("", f"… {n - 5} more", None))
    colA = max(len(p + nm + (" " + w if w else "")) for p, nm, w in ents[0::2])
    xb = 13 + colA + 3
    for k, (pre, nm, w) in enumerate(ents):
        x = 13 if k % 2 == 0 else xb
        yy = y + k // 2
        if w is None:
            cv.put(x, yy, nm)
            continue
        x = cv.put(x, yy, pre + nm)
        if w:
            cv.put(x, yy, " " + w, state_style(w))
    return y + (len(ents) + 1) // 2


def draw_card_mode(cv, res, look):
    W, H = cv.w, cv.h
    cv.put(0, 0, res.names["head"])
    y = draw_strip(cv, res, 1)
    cv.put(0, y, "─" * W, S("border"))
    cv.put(0, y + 1, identity(res))
    draw_notice(cv, 0, y + 2, res)
    y += 3
    f = res.f()
    if f.done:
        for i, ln in enumerate(f.doc.result_lines() if f.doc else [f.kept or ""]):
            cv.put(0, y + i, f"  {i + 1} {ln}")
        return
    if f.card:  # m6-02-whole-file-card-120x40.txt:10-11
        cv.put(0, y, "deleted by them · o keeps the file  t deletes it")
        cv.put(0, y + 1, f"  1 {f.card}")
        return
    doc = f.doc
    mid = W // 2
    paired = W >= 100
    nr, bi = 1, 0
    names = res.names
    for s in doc.segs:
        if y >= H - 1:
            break
        if s[0] == "c":
            for ln in s[1]:
                cv.put(0, y, f"  {nr} {ln}")
                nr, y = nr + 1, y + 1
            continue
        st = doc.state[bi]
        cur = bi == res.cursor
        gut = "▏ " if cur else "  "
        roles = names["roles"]
        if st is None:
            what, wtok, keys = "open", "open", f"o {roles[0]}  t {roles[1]}  b both"
        elif st == "both":
            what, wtok, keys = "took both", "text", f"o {roles[0]}  t {roles[1]}"
        else:
            what = "took " + res.side_name(st)
            wtok = st
            keys = f"t {roles[1]}  b both" if st == "ours" else f"o {roles[0]}  b both"
        # crates/graft-ui/src/conflict_resolver/draw/block.rs:40-45
        x = cv.put(2, y, f"── conflict {bi + 1} of {len(doc.state)} · ")
        x = cv.put(x, y, what, S(wtok, None, "O" if st is None else ""))
        x = cv.put(x, y, f" ── {keys} ")
        cv.put(x, y, "─" * max(0, W - x))
        if cur:
            cv.paint(2, y, W - 2, None, "r")
            cv.put(0, y, "▏", S("accent"))
        y += 1
        ours, theirs = s[1], s[2]
        if st is None and paired:
            rows = [(names["ours"], names["theirs"], True)] + [
                (f"{nr + i} {ours[i]}" if i < len(ours) else "", f"{nr + i} {theirs[i]}" if i < len(theirs) else "",
                 False) for i in range(max(len(ours), len(theirs)))]
            for a, b, lab in rows:
                cv.paint(2, y, mid - 2, "ours-tint")
                cv.paint(mid + 1, y, W - mid - 1, "theirs-tint")
                cv.put(0, y, gut, S("accent"))
                cv.put(2, y, a[:mid - 2], S("ours", None, "b") if lab else S())
                cv.put(mid, y, "│", S("border"))
                cv.put(mid + 1, y, b[:W - mid - 1], S("theirs", None, "b") if lab else S())
                y += 1
        elif st is None:
            for lab, lines, tok in ((names["ours"], ours, "ours"), (names["theirs"], theirs, "theirs")):
                for k, txt in enumerate([lab] + [f"{nr + i} {ln}" for i, ln in enumerate(lines)]):
                    cv.paint(2, y, W - 2, tok + "-tint")
                    cv.put(0, y, gut, S("accent"))
                    cv.put(2, y, txt, S(tok, None, "b") if k == 0 else S())
                    y += 1
        else:
            res_lines = [(ln, "ours") for ln in ours] * (st in ("ours", "both")) + \
                        [(ln, "theirs") for ln in theirs] * (st in ("theirs", "both"))
            for i, (ln, side) in enumerate(res_lines):
                cv.paint(2, y, W - 2, side + "-tint")
                cv.put(0, y, gut, S("accent"))
                cv.put(2, y, f"{nr + i} {ln}")
                y += 1
        close = "─" * (W - 2)
        if paired:
            close = close[:mid - 2] + "┴" + close[mid - 1:]
        cv.put(0, y, gut, S("accent"))
        cv.put(2, y, close, S("border"))
        y += 1
        if st is None:
            nr += max(len(ours), len(theirs))
        else:
            nr += len(ours) * (st in ("ours", "both")) + len(theirs) * (st in ("theirs", "both"))
        bi += 1


# ------------------------------------------------------------------ the finish summary

class Summary:
    def __init__(self, combined, resolved, mark_all=False):
        self.combined = combined  # [(name, how, plus, minus)]
        self.resolved = resolved  # [(name, words)]
        self.focus = "files"
        self.cursor = 0
        self.top = 0
        self.open = set()
        self.subject = "Merge branch 'feature' into main"
        self.body = ""
        self.status = None


README_DIFF = [("      1   1   # Project", None), ("      2   2", None),
               ("          3 + line from feature", "added")]


def resolved_words(res):
    out = []
    for f in res.files:
        if f.card or not f.doc:
            out.append((f.name, "kept · " + MERGE["ours"]))
            continue
        n = len(f.doc.state)
        sides = {}
        for s in f.doc.state:
            sides[s] = sides.get(s, 0) + 1
        parts = [f"{res.side_name(k) if k != 'both' else 'both'} in {v} of {n} conflicts" for k, v in sides.items()]
        out.append((f.name, ", ".join(parts)))
    return out


def summary_hints(sm, ycommit):
    if sm.focus == "subject":
        return [("⏎", "commit"), ("⇥", "body"), ("⌃E", "editor"), ("esc", "back")]
    if sm.focus == "body":
        return [("⌃⏎", "commit"), ("⇥", "files"), ("⌃E", "editor"), ("esc", "back")]
    if ycommit:
        return [("y", "commit"), ("⏎", "diff"), ("j/k", "move"), ("⇥", "message"), ("esc", "back")]
    return [("⏎", "commit"), ("space", "diff"), ("j/k", "move"), ("⇥", "message"), ("esc", "back")]


def draw_summary(cv, sm, framed, ycommit):
    W, H = cv.w, cv.h
    cv.put(0, 0, "MERGING")
    key = "y" if ycommit else "⏎"
    if framed:
        title = "Preview: finish merging feature into main"
        if len(title) + 4 > W - 2:
            title = cut("Preview: merge feature → main", W - 6)
        cv.put(0, 1, "╭ ", S("accent"))
        x = cv.put(2, 1, title, S("accent", None, "b"))
        cv.put(x, 1, " " + "─" * (W - x - 2) + "╮", S("accent"))
        for y in range(2, H - 1):
            cv.put(0, y, "│", S("accent"))
            cv.put(W - 1, y, "│", S("accent"))
        hs = summary_hints(sm, ycommit)
        text = hints_text(fit_hints(hs, W - 4, SHORT_BUILT), SHORT_BUILT).lstrip()
        if sm.status:
            # The frame's footer keeps its keys; the status goes beside the header (a placement guess:
            # the note's mockup leaves no status row).
            cv.put(len("MERGING") + 3, 0, sm.status)
        cv.put(0, H - 1, "╰ ", S("accent"))
        x = cv.put(2, H - 1, text)
        if text.startswith("y "):
            cv.put(2, H - 1, "y", S("accent", None, "b"))
        cv.put(x, H - 1, " " + "─" * max(0, W - x - 2) + "╯", S("accent"))
        cv.put(2, 2, f"1 merge commit on main · nothing is committed until {key}")
        cv.put(2, 3, "undo: u takes this back")
        body(cv, sm, 2, 4, W - 2, H - 1, ycommit)
    else:
        cv.put(0, 1, f"Finish · merging feature → main · nothing is written until {key}", S("accent", None, "b"))
        body(cv, sm, 0, 2, W, H - 1, ycommit)
        text = " " + sm.status if sm.status else bar(summary_hints(sm, ycommit), W)
        draw_bar_row(cv, H - 1, text)


def body(cv, sm, x0, y0, x1, y1, ycommit):
    w = x1 - x0
    two = w >= 160 - (4 if x0 else 0)
    mx = x0 + round(w * 0.61) if two else None
    lw = (mx - x0 - 2) if two else w
    y = y0
    head = S("text-muted", None, "b")
    cv.put(x0, y, cut(f"RESOLVED BY YOU · {len(sm.resolved)}", lw), head)
    y += 1
    nlen = max(len(n) for n, _ in sm.resolved) + 2
    for name, words in sm.resolved:
        x = cv.put(x0, y, cut("  " + name.ljust(nlen), lw))
        for part, tok in split_sides(words):
            x = cv.put(x, y, part[:max(0, x0 + lw - x)], S(tok))
        y += 1
    verb = "⏎" if ycommit else "space"
    cv.put(x0, y, cut(f"COMBINED BY GIT ON ITS OWN · {len(sm.combined)} · {verb} shows the change", lw), head)
    y += 1
    msg_rows = 0 if two else 3
    room = y1 - y - msg_rows
    rows = []
    for i, c in enumerate(sm.combined):
        rows.append(("file", i))
        if i in sm.open:
            rows += [("diff", d) for d in README_DIFF]
    cur_row = next(k for k, r in enumerate(rows) if r == ("file", sm.cursor))
    if cur_row < sm.top:
        sm.top = cur_row
    if cur_row >= sm.top + room:
        sm.top = cur_row - room + 1
    nl = max(len(c[0]) for c in sm.combined) + 2
    for r in rows[sm.top:sm.top + max(0, room)]:
        if r[0] == "file":
            name, how, plus, minus = sm.combined[r[1]]
            on = r[1] == sm.cursor
            if on and sm.focus == "files":
                cv.paint(x0, y, lw, "selection", "R")
            x = cv.put(x0, y, ("▸ " if on else "  ") + name.ljust(nl) + how + "  ")
            x = cv.put(x, y, plus, S("added"))
            cv.put(x, y, " " + minus, S("removed"))
        else:
            txt, tok = r[1]
            cv.put(x0, y, txt[:lw], S(tok) if tok else S("text-muted"))
        y += 1
    if two:
        my = y0
        cv.put(mx, my, "MESSAGE · ⇥ edit", head)
        subj_x, subj_y, body_y, sw = mx, my + 1, my + 2, x1 - mx
    else:
        my = y1 - 3
        cv.put(x0, my, "MESSAGE · ⇥ edit", head)
        subj_x, subj_y, body_y, sw = x0, my + 1, my + 2, w
    if sm.focus == "subject":
        cv.paint(subj_x, subj_y, sw, "selection", "R")
    if sm.focus == "body":
        cv.paint(subj_x, body_y, sw, "selection", "R")
    x = cv.put(subj_x, subj_y, " Subject ", S("text-muted"))
    cv.put(x, subj_y, sm.subject[:50])
    cv.put(x + 50, subj_y, "┊", S("border"))
    cnt = f"{len(sm.subject)}/50"
    cv.put(subj_x + sw - len(cnt) - (1 if x0 else 0), subj_y, cnt, S("text-muted"))
    x = cv.put(subj_x, body_y, " Body ", S("text-muted"))
    cv.put(x, body_y, sm.body[:sw - 7])


def split_sides(words):
    """`yours · main` in `ours`, `theirs · feature` in `theirs`: the rows tie back to the panes."""
    out = []
    for part in re.split(r"(yours · main|theirs · feature)", words):
        if part == MERGE["ours"]:
            out.append((part, "ours"))
        elif part == MERGE["theirs"]:
            out.append((part, "theirs"))
        elif part:
            out.append((part, "text"))
    return out


def summary_key(sm, k, ycommit, shift_tab=False, arrows=False, pages=False, page=10):
    """Returns 'commit', 'back', True (handled) or False (does nothing)."""
    if sm.focus in ("subject", "body"):
        if k == "\r":
            return "commit" if sm.focus == "subject" else True
        if k == "\t":
            sm.focus = {"subject": "body", "body": "files"}[sm.focus]
            return True
        if k == "\x1b[Z" and shift_tab:
            sm.focus = {"subject": "files", "body": "subject"}[sm.focus]
            return True
        if k == "\x1b":
            return "back"
        if k in ("\x7f", "\x08"):
            if sm.focus == "subject":
                sm.subject = sm.subject[:-1]
            else:
                sm.body = sm.body[:-1]
            return True
        if len(k) == 1 and k.isprintable():
            if sm.focus == "subject":
                sm.subject += k
            else:
                sm.body += k
            return True
        return False
    n = len(sm.combined)
    if (k == "y" and ycommit) or (k == "\r" and not ycommit):
        return "commit"
    if k == " " or (k == "\r" and ycommit):
        sm.open ^= {sm.cursor}
        return True
    if k == "j" or (arrows and k == "\x1b[B"):
        sm.cursor = min(n - 1, sm.cursor + 1)
        return True
    if k == "k" or (arrows and k == "\x1b[A"):
        sm.cursor = max(0, sm.cursor - 1)
        return True
    if pages and k == "\x1b[6~":
        sm.cursor = min(n - 1, sm.cursor + page)
        return True
    if pages and k == "\x1b[5~":
        sm.cursor = max(0, sm.cursor - page)
        return True
    if k == "\t":
        sm.focus = "subject"
        return True
    if k == "\x1b[Z" and shift_tab:
        sm.focus = "body"
        return True
    if k == "\x1b":
        return "back"
    return False


def draw_landed(cv, status):
    """A canned graph after the merge commit (a guess at the rows; the status words are the note's :930)."""
    cv.put(0, 0, " graft /code/api  9 commits")
    rows = [("●─╮ ▏  ", "[main] Merge branch 'feature' into main", "GF now  3c1e9a0"),
            ("│ ●    ", "[feature] finish the feature", "GF 2m   8d6cd28"),
            ("● │    ", "main moves on", "GF 4m   37926a7"),
            ("●─╯    ", "add the base", "GF 5m   ed332bc"),
            ("●      ", "Add the first note", "GF 1y   fadcfb3")]
    for i, (g, t, m) in enumerate(rows):
        cv.put(0, i + 1, g, S("accent"))
        cv.put(7, i + 1, t)
        cv.put(cv.w - len(m) - 2, i + 1, m, S("text-muted"))
    cv.paint(0, 1, cv.w, "selection", "R")  # the cursor lands on the merge commit (note :930)
    draw_bar_row(cv, cv.h - 1, " " + status)


# ------------------------------------------------------------------ scenes

KEYNAMES = {"\r": "⏎", "\t": "⇥", "\x1b": "Esc", "\x1b[A": "↑", "\x1b[B": "↓", "\x1b[C": "→", "\x1b[D": "←",
            "\x1b[5~": "PgUp", "\x1b[6~": "PgDn", "\x1b[Z": "⇧⇥", " ": "space", "\x7f": "Backspace",
            "\x0f": "Ctrl+O", "\x14": "Ctrl+T"}


def kname(k):
    return KEYNAMES.get(k, k)


class Scene:
    question = ""
    knobs = []  # [(name, [(label, tag)])]
    default_width = None  # None: the window's width
    widths = [80, 100, 120, 160, 187]

    def __init__(self, app):
        self.app = app
        self.kv = [0] * len(self.knobs)
        self.width = None
        self.reset()

    def reset(self):
        pass

    def text_mode(self):
        return False

    def key(self, k):
        return False

    def draw(self, cv):
        pass

    def info(self):
        return None

    def note(self):
        return ""

    def pick(self):
        parts = [f"{self.knobs[i][0]}={self.knobs[i][1][v][0]}" for i, v in enumerate(self.kv)]
        return " · ".join(parts)

    def knob_changed(self, i):
        pass


def two_file_res(state=(None, None), cursor=None):
    doc = notes_doc()
    doc.state = list(state)
    gone = F("gone.txt", card="gone, changed on main", done=True, visited=True, kept="gone, changed on main")
    res = Res([gone, F("notes.txt", doc=doc)], cur=1)
    if cursor is not None:
        res.cursor = cursor
    return res


class Colours(Scene):
    question = "colours and lines between the panes"
    knobs = [("tint", [("20%", "draft"), ("30%", "draft"), ("40%", "draft")]),
             ("depth", [(d, "draft") for d in DEPTHS]),
             ("lines", [("A glyphs and band", "draft"), ("B VS Code bracket", "draft"),
                        ("C colour only", "draft")]),
             ("pair", [("okabe #56B4E9/#CC79A7", "draft"), ("teal-magenta #2AA198/#D33682", "draft"),
                       ("VS Code #40C8AE/#40A6FF", "draft")])]

    def __init__(self, app):
        super().__init__(app)
        self.kv = [1, 0, 0, 0]

    def reset(self):
        self.res = two_file_res()
        self.res.cursor = 0

    def key(self, k):
        r = self.res
        r.status = None
        if k in "otb" and len(k) == 1:
            return r.take({"o": "ours", "t": "theirs", "b": "both"}[k])
        if k in ("n", "N"):
            return r.walk(k == "n")
        if k == "\r":
            if r.enter() == "done":
                self.app.to_finish(self.res)
            return True
        return False

    def draw(self, cv):
        draw_resolver(cv, self.res, {"lines": self.kv[2]})

    def info(self):
        pal = self.app.pal()
        lines = "ABC"[self.kv[2]] + ("" if self.app.width_now() >= 180 else " (three panes only, ≥180)")
        return (f"tint {round(ALPHAS[self.kv[0]] * 100)}% · {pal.info()} · {DEPTHS[self.kv[1]]} · lines {lines}"
                f" · pair {PAIRS[self.kv[3]][0]}")


class Finishing(Scene):
    question = "finishing the merge"
    knobs = [("after the last file", [("as built: `stage notes.txt ✓` until the next key", "as built"),
                                      ("`c finish` at once", "draft"), ("the summary opens by itself", "draft")]),
             ("commit key", [("⏎ commits", "as built"), ("y commits · ⏎ shows the change", "draft")]),
             ("summary look", [("as built, no frame", "as built"), ("inside the preview frame", "draft")])]

    def __init__(self, app):
        super().__init__(app)
        self.kv = [2, 1, 1]

    def reset(self, res=None):
        self.res = res or two_file_res(("theirs", None), cursor=1)
        self.res.notice = None
        self.mode = "resolver"
        self.sm = None
        self.landed = None

    def text_mode(self):
        return self.mode == "summary" and self.sm.focus != "files"

    def apply_look(self):
        r = self.res
        r.finish_first = self.kv[0] > 0
        r.all_done_head = self.kv[0] > 0

    def last_file_done(self):
        """Where the last ⏎ lands him, by knob 1."""
        r = self.res
        self.apply_look()
        self.app.event("last file done")
        if self.kv[0] == 0:  # fix-finish-from-the-resolver-120x40.txt:5, :40
            r.notice = ("every file done ✓ · c finishes the merge", "plain")
            r.status = "stage notes.txt ✓"
        elif self.kv[0] == 1:
            r.notice = ("✓ every file done — " + ("c review and finish the merge" if cv_width(self) >= 120
                                                 else "c review & finish"), "finish")
            r.status = "notes.txt done ✓ · every file done · c finishes the merge"
        else:
            self.open_summary(status="notes.txt done ✓ · every file done")

    def open_summary(self, status=None):
        self.sm = Summary([("README.md", "modified", "+1", "−0")], resolved_words(self.res))
        self.sm.status = status
        self.mode = "summary"
        self.app.event("summary open")

    def back_to_resolver(self):
        r = self.res
        self.mode = "resolver"
        self.apply_look()
        if self.kv[0] == 0:
            r.notice = ("every file done ✓ · c finishes the merge", "plain")
        else:
            r.notice = ("✓ every file done — " + ("c review and finish the merge" if cv_width(self) >= 120
                                                 else "c review & finish"), "finish")

    def knob_changed(self, i):
        """Knob 1 redraws the resolver at once when every file is already done."""
        if i == 0 and self.mode == "resolver" and self.res.all_done():
            self.back_to_resolver()

    def key(self, k):
        if self.mode == "landed":
            return False
        if self.mode == "summary":
            self.sm.status = None
            got = summary_key(self.sm, k, self.kv[1] == 1)
            if got == "commit":
                self.mode = "landed"
                self.app.event("committed")
            elif got == "back":
                self.back_to_resolver()
            return bool(got)
        r = self.res
        r.status = None
        if k in ("o", "t", "b"):
            return r.take({"o": "ours", "t": "theirs", "b": "both"}[k])
        if k in ("n", "N"):
            return r.walk(k == "n")
        if k == "\r":
            if r.enter() == "done":
                self.last_file_done()
            return True
        if k == "c" and r.all_done():
            self.open_summary()
            return True
        if k in ("d", "e"):
            self.app.flash(f"{kname(k)}: not acted out in this scene")
            return True
        return False

    def draw(self, cv):
        if self.mode == "landed":
            draw_landed(cv, "merged feature into main ✓")
        elif self.mode == "summary":
            draw_summary(cv, self.sm, self.kv[2] == 1, self.kv[1] == 1)
        else:
            draw_resolver(cv, self.res, {"lines": self.app.colours.kv[2]})


def cv_width(scene):
    return scene.app.width_now()


class SummaryKeys(Scene):
    question = "keys on the “what will land” screen"
    knobs = [("↑/↓", [("off", "as built"), ("on", "draft")]),
             ("PgUp/PgDn", [("off", "as built"), ("on", "draft")]),
             ("⇧⇥ back", [("off", "as built"), ("on", "draft")])]

    def reset(self):
        names = [f"src/{d}/{n}.rs" for d in ("api", "db", "ui") for n in
                 ("auth", "cache", "config", "errors", "handlers", "limits", "models", "routes", "state", "views")]
        combined = [(n, "modified", f"+{(i * 7) % 13 + 1}", f"−{(i * 5) % 9}") for i, n in enumerate(names)]
        self.sm = Summary(combined, [("gone.txt", "kept · yours · main"),
                                     ("notes.txt", "theirs · feature in 2 of 2 conflicts")])
        self.mode = "summary"

    def text_mode(self):
        return self.mode == "summary" and self.sm.focus != "files"

    def key(self, k):
        if self.mode != "summary":
            return False
        fin = self.app.finishing
        got = summary_key(self.sm, k, fin.kv[1] == 1, shift_tab=self.kv[2] == 1, arrows=self.kv[0] == 1,
                          pages=self.kv[1] == 1, page=max(1, self.app.height_now() - 12))
        if got == "commit":
            self.mode = "landed"
        elif got == "back":
            self.app.flash("esc: back to the resolver (not acted out here) · Backspace restarts")
        elif not got:
            self.app.flash("does nothing here")
        return bool(got)

    def draw(self, cv):
        fin = self.app.finishing
        if self.mode == "landed":
            draw_landed(cv, "merged feature into main ✓")
        else:
            draw_summary(cv, self.sm, fin.kv[2] == 1, fin.kv[1] == 1)

    def note(self):
        return "look and commit key follow scene 2's knobs"


class KeyRow120(Scene):
    question = "the conflict screen's key row at 120 columns"
    knobs = [("key row", [("C as built", "as built"), ("A drop words from everyday keys first", "draft"),
                          ("B shorter words", "draft")]),
             ("operation", [("merge", "as built"), ("rebase", "as built")])]
    default_width = 120

    def reset(self):
        pass

    def res(self):
        names = MERGE if self.kv[1] == 0 else REBASE
        doc = Doc([("b", ["our plan"], ["their plan"])])  # fix-stop-lands-in-resolver-120x40.txt:6-9
        other = F("shared.rs", doc=Doc([("b", ["a"], ["b"])]))
        r = Res([F("notes.md", doc=doc), other], cur=0, names=names, base=True, restart=True)
        return r

    def bar_text(self, W):
        roles = (MERGE if self.kv[1] == 0 else REBASE)["roles"]
        lst = resolver_list(True, True, roles)
        if self.kv[0] == 0:
            return bar(lst, W)
        if self.kv[0] == 2:
            lst = [(k, {"mergetool": "tool", "start over": "restart"}.get(v, v)) for k, v in lst]
            return bar(lst, W)
        # A (draft): everyday keys go bare first, `j/k` and `M` before the rest; `d start over` and
        # `esc close` are never the ones to lose their words.
        full = [(k, v, "full") for k, v in lst]
        for everyday in ("j/k", "M", "N", "n", "e", "O/T", "⌃O/⌃T", "B", "⏎", "b"):
            if len(hints_text(full, SHORT_BUILT)) <= W:
                break
            full = [(k, v, "key" if k == everyday else r) for k, v, r in full]
        if len(hints_text(full, SHORT_BUILT)) <= W:
            return hints_text(full, SHORT_BUILT)
        return bar(lst, W)

    def draw(self, cv):
        r = self.res()
        draw_resolver(cv, r, {"lines": self.app.colours.kv[2]}, bar_text=self.bar_text(cv.w))

    def note(self):
        roles = (MERGE if self.kv[1] == 0 else REBASE)["roles"]
        lst = resolver_list(True, True, roles)
        if self.kv[0] == 2:
            lst = [(k, {"mergetool": "tool", "start over": "restart"}.get(v, v)) for k, v in lst]
        need = len(hints_text([(k, v, "full") for k, v in lst], SHORT_BUILT))
        return f"every word needs {need} columns · this screen {self.app.width_now()}"


class ShortWords(Scene):
    question = "short key words on narrow screens"
    knobs = [("short words", [("as built: the 9 kept", "as built"), ("the whole draft", "draft"),
                              ("none: words drop whole", "draft")])]
    widths = list(range(187, 59, -10))

    def __init__(self, app):
        self.pairs = [[f, s, False, None] for f, s in SHORT_DRAFT]  # full, short, struck, retyped
        self.sel = 0
        self.typing = None
        super().__init__(app)

    def text_mode(self):
        return self.typing is not None

    def short(self):
        if self.kv[0] == 0:
            return SHORT_BUILT
        if self.kv[0] == 2:
            return {}
        return {f: (r or s) for f, s, struck, r in self.pairs if not struck}

    def key(self, k):
        if self.typing is not None:
            if k == "\r":
                if self.typing:
                    self.pairs[self.sel][3] = self.typing
                self.typing = None
            elif k == "\x1b":
                self.typing = None
            elif k in ("\x7f", "\x08"):
                self.typing = self.typing[:-1]
            elif len(k) == 1 and k.isprintable():
                self.typing += k
            return True
        if k == "j":
            self.sel = min(len(self.pairs) - 1, self.sel + 1)
        elif k == "k":
            self.sel = max(0, self.sel - 1)
        elif k == "x":
            self.pairs[self.sel][2] = not self.pairs[self.sel][2]
        elif k == "w":
            self.typing = ""
        else:
            return False
        return True

    def draw(self, cv):
        W = cv.w
        short = self.short()
        lists = [("a conflicted file (the longest)",  # lists.rs:257-266
                  [("⏎", "resolve"), ("M", "mergetool"), ("e", "edit"), ("space", "resolved"), ("c", "commit"),
                   ("⇥", "graph")]),
                 ("a changed file", [("space", "stage"), ("d", "discard"), ("c", "commit"), ("⌃A", "all"),
                                     ("e", "edit"), ("⇥", "graph")])]
        y = 0
        cv.put(0, y, f"the changes pane's key row at {W} columns", S("text-muted", None, "b"))
        y += 2
        for label, lst in lists:
            cv.put(0, y, "on " + label + ":", S("text-muted"))
            cv.put(0, y + 1, "─" * W, S("border"))
            cv.put(0, y + 2, bar(lst, W, short, (".", "more")))
            cv.put(0, y + 3, "─" * W, S("border"))
            y += 5
        cv.put(0, y, "the 2026-09-21 draft · j/k move · x strike · w retype · live only on “the whole draft”",
               S("text-muted", None, "b"))
        y += 1
        for i, (f, s, struck, r) in enumerate(self.pairs):
            if y + (i % 11) >= cv.h:
                continue
            col = 0 if i < 11 else 40
            yy = y + (i % 11)
            if i == self.sel:
                cv.paint(col, yy, 38, "selection", "R")
            txt = f"{'▸' if i == self.sel else ' '} {f:<13} → {(r or s):<6}"
            tag = " struck" if struck else (" retyped" if r else "")
            if self.typing is not None and i == self.sel:
                txt = f"▸ {f:<13} → {self.typing}_"
                tag = ""
            cv.put(col, yy, txt, S("text-muted", None, "N") if struck else S())
            cv.put(col + len(txt), yy, tag, S("open"))

    def pick(self):
        base = super().pick()
        edits = [f"{f}→{'STRUCK' if st else r}" for f, s, st, r in self.pairs if st or r]
        return base + (" · edits: " + ", ".join(edits) if edits else " · edits: none")


class StopLine(Scene):
    question = "the stop line with the changes pane open"
    knobs = [("origin", [("begun outside Graft", "as built"), ("from a previous run", "as built")]),
             ("stop line", [("as built: origin dropped so the count fits", "as built"),
                            ("keep the origin, cut the count with …", "draft"), ("shorten both", "draft")])]
    default_width = 187

    FILES = [("added-by-them.txt", "added by them"), ("added-by-us.txt", "added by us"),
             ("both-added.txt", "both added"), ("both-deleted.txt", "both deleted"),
             ("both.txt", "both modified"), ("crlf.txt", "both modified"), ("md.md", "both modified"),
             ("nofinal.txt", "both modified"), ("ours-deleted.txt", "deleted by us"), ("pic.bin", "both modified"),
             ("theirs-deleted.txt", "deleted by them"), ("three.txt", "both modified"),
             ("zdiff3.txt", "both modified")]  # fix-landing-b-launch-187x50.txt:7-19

    def wordings(self):  # crates/graft/src/update/banner/stop.rs:241-273
        if self.kv[0] == 0:
            return ["merge in progress · begun outside Graft · 13 files conflicted",
                    "merge in progress · 13 files conflicted", "merge in progress"]
        return ["merge still in progress from a previous run · 13 files conflicted",
                "merge still in progress · 13 files conflicted", "merge still in progress"]

    def row(self, gw):
        hints = [("⏎", "resolve"), ("c", "continue"), ("a", "abort"), ("M", "mergetool"), ("e", "edit"),
                 ("⇥", "graph")]
        full = hints_text([(k, v, "full") for k, v in hints], SHORT_BUILT).lstrip()
        ws = self.wordings()
        if self.kv[1] == 2:
            ws = ["merge in progress · from the shell · 13 conflicted" if self.kv[0] == 0
                  else "merge in progress · from last run · 13 conflicted"]
        if self.kv[1] == 1:  # draft: the origin stays whole; the count is cut with `…` first
            reason = ws[0]
            keep = len(ws[0]) - len("13 files conflicted") + 2  # the origin plus `1…` at least
            room = gw - len(" · " + full) - 1
            reason = cut(reason, max(room, keep))
            return " " + reason + " ·" + bar(hints, gw - len(reason) - 3)
        for r in ws:
            if len(" " + r + " · " + full) <= gw:
                return " " + r + " · " + full
        r = ws[-1]
        return " " + r + " ·" + bar(hints, gw - len(r) - 3)

    def draw(self, cv):
        W = cv.w
        gw = round(W * 109 / 187)  # the graph side at 187 is 109 cells (fix-landing-b-launch-187x50.txt:6)
        cv.put(0, 0, " graft /code/api    MERGING")
        cv.put(W - 12, 0, "UNCOMMITTED", S("text-muted"))
        graph = [("◌   ▏  ", "Uncommitted changes (13 unmerged)"), ("◆      ", "[◆ main] Change every file on main"),
                 ("│ ●  ⓘ ", "[feature] Change every file on feature"),
                 ("◇─╯    ", "Add the files both sides will change")]
        for i, (g, t) in enumerate(graph):
            cv.put(0, 1 + i, g, S("accent"))
            cv.put(7, 1 + i, t[:gw - 8])
        cv.paint(0, 1, gw, "selection", "R")
        px = gw + 1
        cv.put(px, 5, "CONFLICTS (13)", S("text-muted", None, "b"))
        for i, (n, how) in enumerate(self.FILES):
            cv.put(px + 2, 6 + i, n)
            cv.put(W - len(how) - 1, 6 + i, how, S("open"))
        draw_bar_row(cv, cv.h - 1, self.row(gw)[:gw])

    def note(self):
        return "the footer is budgeted on the graph side, not the whole row"


class AfterLastBlock(Scene):
    question = "what `n` says after the last block"
    knobs = [("n past the last block", [
        ("today: n wraps · `wrapped · 1 still open`", "as built"),
        ("then: `no more conflicts · ⏎ marks zdiff3.txt done`", "as built"),
        ("`no more conflicts this way`", "draft"),
        ("`last block · 1 still open above · N goes back`", "draft"),
        ("your own: w, type, ⏎", "draft")])]

    def __init__(self, app):
        self.custom = "no more conflicts · (your words here: w)"
        self.typing = None
        super().__init__(app)

    def reset(self):
        doc = Doc([("c", ["start"]), ("b", ["middle ours"], ["middle theirs"]), ("c", ["between"]),
                   ("b", ["end ours"], ["end theirs"]), ("c", ["tail"])])
        doc.state = [None, "theirs"]
        three = F("three.txt", doc=Doc([("b", ["x"], ["y"])]), done=True, visited=True)
        self.res = Res([three, F("zdiff3.txt", doc=doc)], cur=1)
        self.res.cursor = 1

    def text_mode(self):
        return self.typing is not None

    def key(self, k):
        if self.typing is not None:
            if k == "\r":
                self.custom = self.typing or self.custom
                self.typing = None
            elif k == "\x1b":
                self.typing = None
            elif k in ("\x7f", "\x08"):
                self.typing = self.typing[:-1]
            elif len(k) == 1 and k.isprintable():
                self.typing += k
            return True
        r = self.res
        r.status = None
        if k == "w":
            self.typing = ""
            self.kv[0] = 4
            return True
        if k in ("o", "t", "b"):
            return r.take({"o": "ours", "t": "theirs", "b": "both"}[k])
        if k == "n" and r.cursor == len(r.f().doc.state) - 1 and self.kv[0] > 0:
            left = r.f().doc.open_left()
            words = {1: f"no more conflicts · ⏎ marks {r.f().name} done",
                     2: "no more conflicts this way",
                     3: f"last block · {left} still open above · N goes back",
                     4: self.custom}[self.kv[0]]
            r.status = words
            return True
        if k in ("n", "N"):
            return r.walk(k == "n")
        if k == "\r":
            r.enter()
            return True
        return False

    def draw(self, cv):
        draw_resolver(cv, self.res, {"lines": self.app.colours.kv[2]})
        if self.typing is not None:
            draw_bar_row(cv, cv.h - 1, " type your words, ⏎ keeps them: " + self.typing + "_")

    def note(self):
        return ("docs/log/m6.md:78's case no longer happens: n now wraps (graft/src/update/resolver/walk.rs:78-80,"
                " read in source, not run) · the other values act it out")


class StartOver(Scene):
    question = "starting over a file one side deleted"
    knobs = [("d on gone.txt", [("as built: no d, refused in words", "as built"),
                                ("offered: d start over", "draft")]),
             ("merge begun", [("in the app", "as built"), ("in the shell", "as built")])]

    def reset(self):
        gone = F("gone.txt", card=None, done=True, visited=True, kept="gone, changed on main")
        notes = F("notes.txt", doc=notes_doc(), visited=True)
        self.res = Res([gone, notes], cur=0)
        self.box = None

    def offered(self):
        return self.kv[0] == 1 and self.kv[1] == 0

    def key(self, k):
        r = self.res
        r.status = None
        r.restart = self.offered()
        if self.box is not None:
            if k in ("\x1b[C", "\x1b[D", "l", "h"):
                self.box = 1 - self.box if k in ("\x1b[C", "\x1b[D") else self.box
                return True
            if k == "\r":
                if self.box == 1:
                    r.files[0] = F("gone.txt", card="gone, changed on main", visited=True)
                    r.status = None
                self.box = None
                return True
            if k == "\x1b":
                self.box = None
                return True
            return False
        if k == "d" and r.f().done:
            if self.offered():
                self.box = 0
            else:  # crates/graft/src/update/resolver/restart.rs:96 · docs/shots/m6/fix-done-start-over-cannot:81
                r.status = "can't start over · gone.txt: deleted by them — no markers to take from · esc dismiss"
            return True
        if k in ("o", "t") and r.f().card:
            r.files[0].done = True
            r.files[0].card = None
            r.status = ("kept gone.txt ✓" if k == "o" else "deleted gone.txt ✓")
            return True
        return False

    def draw(self, cv):
        self.res.restart = self.offered()
        draw_resolver(cv, self.res, {"lines": self.app.colours.kv[2]})
        if self.box is not None:
            draw_box(cv, "Start gone.txt over?",
                     ["the choice comes back: keep or delete",
                      "takes and edits in this file are lost · not undoable while",
                      "the merge is open", "", "git checkout --merge -- gone.txt", ""],
                     ["Cancel", "Start over"], self.box, None)
            draw_bar_row(cv, cv.h - 1, " ←/→ move  ⏎ choose  esc cancel")

    def note(self):
        return "in the shell: refused either way, words as built (a shell-only reason is not prototyped)"


def draw_box(cv, title, lines, buttons, sel, foot):
    """The box of m6-09-start-over-box-120x40.txt:18-26 and the merge preview (fix-preview-verb:44-52)."""
    w = min(cv.w - 4, 64)
    h = len(lines) + 3
    x0 = (cv.w - w) // 2
    y0 = max(2, (cv.h - h) // 2)
    for y in range(y0, y0 + h):
        cv.put(x0, y, " " * w, S("text", "bg"))
    cv.put(x0, y0, "╭ ", S("accent"))
    x = cv.put(x0 + 2, y0, title, S("accent", None, "b"))
    cv.put(x, y0, " " + "─" * (x0 + w - x - 2) + "╮", S("accent"))
    for i, ln in enumerate(lines):
        cv.put(x0, y0 + 1 + i, "│", S("accent"))
        cv.put(x0 + 2, y0 + 1 + i, ln[:w - 4])
        cv.put(x0 + w - 1, y0 + 1 + i, "│", S("accent"))
    yb = y0 + 1 + len(lines)
    cv.put(x0, yb, "│", S("accent"))
    x = x0 + 2
    for i, b in enumerate(buttons):
        x = cv.put(x, yb, ("▸" if i == sel else " ") + f"[ {b} ]   ", S("accent", None, "b") if i == sel else S())
    cv.put(x0 + w - 1, yb, "│", S("accent"))
    if foot:
        x = cv.put(x0, yb + 1, "╰ " + foot + " ", S("accent"))
        cv.put(x, yb + 1, "─" * (x0 + w - x - 1) + "╯", S("accent"))
    else:
        cv.put(x0, yb + 1, "╰" + "─" * (w - 2) + "╯", S("accent"))


class ShellUndo(Scene):
    question = "undo and abort on a merge begun in the shell"
    knobs = [("u and a", [("keep the shell step", "as built"), ("abort only", "draft"), ("both", "draft")])]

    def reset(self):
        self.box = None
        self.aborted = False
        self.status = None

    def key(self, k):
        self.status = None
        if self.aborted:
            return False
        if self.box is not None:
            if k in ("\x1b[C", "\x1b[D"):
                self.box = 1 - self.box
            elif k == "y" or (k == "\r" and self.box == 1):
                self.box, self.aborted = None, True
            elif k in ("\x1b",) or (k == "\r" and self.box == 0):
                self.box = None
            else:
                return False
            return True
        if k == "u":
            if self.kv[0] == 2:
                self.status = "undo: take theirs on notes.txt · your edit is back ✓"
            else:  # crates/graft/src/update/undo.rs:103-108
                self.status = "finish or abort the merge first · ⇥ banner"
            return True
        if k == "a":
            if self.kv[0] == 0:  # crates/graft-ops/src/operation.rs:598-601
                self.status = ("nothing Graft started is in progress; an operation begun in the shell is "
                               "finished there")
            else:
                self.box = 0
            return True
        return False

    def draw(self, cv):
        W = cv.w
        cv.put(0, 0, " graft /code/api    MERGING")
        graph = [("◌      ", "Uncommitted changes (1 staged, 1 unmerged)"), ("◆   ▏  ", "[◆ main] main's take"),
                 ("│ ●  ⓘ ", "[feature] the feature's take"), ("●─╯    ", "the shared files")]
        if self.aborted:
            graph = graph[1:]
        for i, (g, t) in enumerate(graph):
            cv.put(0, 1 + i, g, S("accent"))
            cv.put(7, 1 + i, t)
        if self.aborted:
            draw_bar_row(cv, cv.h - 1, " merge aborted ✓")
            return
        banner = (" merge in progress · begun outside Graft · 1 file conflicted · ⏎ resolve  c continue  a abort"
                  "  M mergetool  e edit  ⇥ graph")
        cv.paint(0, cv.h - 2, W, "selection", "R")
        cv.put(0, cv.h - 2, banner[:W], S("text", None, "b"))
        if self.status:
            draw_bar_row(cv, cv.h - 1, " " + self.status)
        if self.box is not None:
            draw_box(cv, "Preview: abort the merge · main goes back to before the pull",
                     ["1 file your t overwrote comes back from its backup", "undo: u takes this back", ""],
                     ["Cancel", "Confirm"], self.box, "y confirm  ←→ choose  ⏎ act  esc cancel")

    def note(self):
        return "the banner has focus · a `git pull` stopped in the shell · in Graft, t overwrote a hand edit"


class UnvisitedMark(Scene):
    question = "the mark on a file you haven't opened"
    knobs = [("mark", [("…", "as built"), ("○", "draft"), ("new", "draft"), ("no mark", "draft")])]
    default_width = 120

    def res(self):
        files = [F(f"src/handlers/handler_{i:02}.rs", doc=Doc([("b", [f"fn handle() {{ ours() }}"],
                                                                     [f"fn handle() {{ theirs() }}"])]))
                 for i in range(1, 13)]
        r = Res(files, cur=0)
        files[1].visited = True
        r.mark = ["…", "○", "new", ""][self.kv[0]]
        return r

    def draw(self, cv):
        draw_resolver(cv, self.res(), {"lines": self.app.colours.kv[2]})

    def note(self):
        return "12 files, 2 opened (the as-built many-files shape) · > shows the column at 187"


class CommitWhileMerging(Scene):
    """`c` on the main screen while a merge is stopped (docs/reviews/m6-usability.md M3)."""
    question = "c on the main screen while a merge is stopped"
    knobs = [("c", [("opens the editor, which refuses", "as built"),
                    ("goes where the banner's c goes", "draft")])]

    def reset(self):
        self.where = "pane"  # pane · editor · banner · summary · landed

    def key(self, k):
        w = self.where
        if w == "pane":
            if k == "c":
                # as built: commit_editor::open ignores in_progress (crates/graft/src/update/commit_editor.rs:289-318)
                self.where = "summary" if self.kv[0] == 1 else "editor"
            elif k == "\t":
                self.where = "banner"
            else:
                return False
            return True
        if w == "editor":
            if k == "\x1b":
                self.where = "pane"
            return True  # anything else: the editor stays and keeps refusing
        if w == "banner":
            if k == "c":
                self.where = "summary"
            elif k == "\t":
                self.where = "pane"
            else:
                return False
            return True
        if w == "summary":
            if k == "\r":  # the summary's ⏎ commits (crates/graft/src/update/finish/keys.rs:105-107)
                self.where = "landed"
            elif k == "\x1b":
                self.where = "banner"
            else:
                return False
            return True
        return False

    def draw(self, cv):
        W = cv.w
        cv.put(0, 0, " graft /code/api    " + ("" if self.where == "landed" else "MERGING"))
        if self.where == "landed":
            cv.put(0, 1, "◆─╮    ", S("accent"))
            cv.put(7, 1, "[◆ main] Merge branch 'feature'")
            draw_bar_row(cv, cv.h - 1, " committed ✓ · merge finished · Backspace to try again")
            return
        rows = [("◌      ", "Uncommitted changes (1 staged)"), ("◆   ▏  ", "[◆ main] main's take"),
                ("│ ●    ", "[feature] the feature's take")]
        for i, (g, t) in enumerate(rows):
            cv.put(0, 1 + i, g, S("accent"))
            cv.put(7, 1 + i, t)
        cv.put(0, 5, ("▸ " if self.where == "pane" else "  ") + "changes · staged", S("text", None, "b"))
        cv.put(4, 6, "M notes.txt  (resolved)")
        banner = " merge in progress · 0 files conflicted · c continue  a abort  ⇥ graph"
        on = self.where == "banner"
        if on:
            cv.paint(0, cv.h - 2, W, "selection", "R")
        cv.put(0, cv.h - 2, ("▸" if on else " ") + banner[1:W], S("text", None, "b"))
        if self.where == "editor":
            w = min(W - 4, 76)
            x0, y0 = 2, 8
            for y in range(y0, y0 + 8):
                cv.put(x0, y, " " * w, S("text", "bg"))
            cv.put(x0, y0, "╭ Subject " + "─" * (w - 11) + "╮", S("accent"))
            cv.put(x0 + 2, y0 + 1, "▏", S("accent"))
            cv.put(x0, y0 + 2, "├ Body " + "─" * (w - 8) + "┤", S("accent"))
            # options row: crates/graft-ui/src/commit_editor.rs:309-316
            cv.put(x0 + 2, y0 + 5, "  [ ] amend  [ ] sign-off  [ ] no-verify  [ ] allow empty  [ Commit ]")
            # the refusal: crates/graft/src/update/commit_editor.rs:224-227 (a merge begun in Graft)
            cv.put(x0 + 2, y0 + 6, "  merge in progress — esc, ⇥ banner, then c continue  a abort", S("removed"))
            cv.put(x0, y0 + 7, "╰" + "─" * (w - 2) + "╯", S("accent"))
            # crates/graft-ui/src/commit_editor.rs:171
            draw_bar_row(cv, cv.h - 1, " ⌃⏎/Commit commit  ⇥ next  ⌃E $EDITOR  ⌃S sign-off  esc cancel")
        elif self.where == "summary":
            draw_box(cv, "What will land · merge feature into main",
                     ["Subject: Merge branch 'feature'", "", "notes.txt   resolved by hand", ""],
                     [], 0, "⏎ commit  esc back")
        else:
            draw_bar_row(cv, cv.h - 1, " c commit  ⇥ banner" if self.where == "pane" else " c continue  ⇥ changes")

    def note(self):
        return "changes pane has focus · notes.txt resolved outside the resolver · goal: the merge committed"


class PushBox(Scene):
    """The push box (docs/reviews/m6-usability.md M1)."""
    question = "the push box: which keys push"
    knobs = [("y", [("closes the box, nothing pushed", "as built"), ("pushes", "draft")]),
             ("box", [("plain push", "as built"), ("force push", "as built")])]

    def reset(self):
        self.box = None
        self.status = None
        self.pushed = False

    def force(self):
        return self.kv[1] == 1

    def y_acts(self):
        return self.kv[0] == 1 and not self.force()  # the note recommends: never in the force box

    def key(self, k):
        self.status = None
        if self.box is None:
            if k == "P" and not self.pushed:
                self.box = 0  # the cursor starts on Cancel (ADR-0038 §1)
                return True
            return False
        done = "force-pushed main → origin/main ✓" if self.force() else "pushed main → origin/main ✓"
        if k in ("\x1b[C", "\x1b[D"):
            self.box = 1 if k == "\x1b[C" else 0
        elif k == "\r":  # crates/graft/src/update/confirm.rs:103-111: only ⏎ acts, on the cursor's button
            if self.box == 1:
                self.pushed, self.status = True, done
            self.box = None
        elif k == "\x1b":
            self.box = None
        elif k == "y" and self.y_acts():
            self.box, self.pushed, self.status = None, True, done
        elif self.kv[0] == 0:
            self.box = None  # crates/graft/src/update/confirm.rs:173 `_ => next.confirm = None`: no word said
        # else: the fix's rule, an unbound key is swallowed and the box stays
        return True

    def draw(self, cv):
        cv.put(0, 0, " graft /code/api")
        cv.put(0, 1, "●      ", S("accent"))
        cv.put(7, 1, "[◆ main" + ("" if self.pushed else "  ↑2") + "] fix the parser")
        cv.put(0, 2, "●      ", S("accent"))
        cv.put(7, 2, "add a test")
        cv.put(0, 3, "●      ", S("accent"))
        cv.put(7, 3, "[origin/main] the shared files")
        draw_bar_row(cv, cv.h - 1, " " + (self.status or "P push  p pull  f fetch"))
        if self.box is None:
            return
        foot = "y push  ←→ choose  ⏎ act  esc cancel" if self.y_acts() else None  # the note's footer
        if self.force():  # crates/graft/src/update/remote/push_box.rs:157-184
            draw_box(cv, "Force-push main → origin/main?",
                     ["origin/main 3e1a9c0 → 7b2d4f1 · refused if origin moved since your fetch (lease)",
                      "rewrites history for everyone who has 3e1a9c0", "",
                      "git push --force-with-lease=main:3e1a9c0 origin main", ""],
                     ["Cancel", "Force push"], self.box, foot)
        else:  # push_box.rs:134-155
            draw_box(cv, "Push main → origin/main",
                     ["2 commits · can't be undone from here — origin keeps what you send", "",
                      "git push origin main", ""],
                     ["Cancel", "Push"], self.box, foot)

    def note(self):
        return "graph focus · main is 2 ahead · goal: pushed · force box: y never pushes (recommended)"


class CommitSubject(Scene):
    """The commit editor's subject (docs/reviews/m6-usability.md M2)."""
    question = "⏎ in the commit editor's subject"
    knobs = [("⏎ in the subject", [("moves to the body", "as built"), ("commits; ⇥ to the body", "draft")])]
    FIELDS = 4  # subject → body → amend → [ Commit ] (crates/graft-ui/src/commit_editor.rs:90-99, options folded)

    def reset(self):
        self.focus = 0
        self.subject = ""
        self.body = [""]
        self.landed = None
        self.open = True

    def text_mode(self):
        return self.open and self.landed is None and self.focus in (0, 1)

    def key(self, k):
        if self.landed is not None:
            return False
        if not self.open:
            if k == "c":
                self.open = True
                return True
            return False
        f = self.focus
        if k == "\t":
            self.focus = (f + 1) % self.FIELDS
        elif k == "\x1b[Z":
            self.focus = (f - 1) % self.FIELDS
        elif k == "\x1b":
            self.open = False  # the draft is kept, as esc keeps it in the app
        elif k == "\r":
            if f == 0:
                if self.kv[0] == 1:
                    self.commit()
                else:  # crates/graft/src/update/commit_editor.rs:601 `Enter | Down => focus = Body`
                    self.focus = 1
            elif f == 1:
                self.body.append("")
            elif f == 3:
                self.commit()
        elif k == "\x1b[B" and f == 0:
            self.focus = 1
        elif k in ("\x7f", "\x08"):
            if f == 0:
                self.subject = self.subject[:-1]
            elif f == 1:
                self.body[-1] = self.body[-1][:-1]
        elif len(k) == 1 and k.isprintable():
            if f == 0:
                self.subject += k
            elif f == 1:
                self.body[-1] += k
        else:
            return False
        return True

    def commit(self):
        body = [b for b in self.body if b]
        self.landed = self.subject or "(no subject)"
        if body:
            self.landed += f"  (+{len(body)} body line{'s' if len(body) > 1 else ''})"

    def draw(self, cv):
        W = cv.w
        cv.put(0, 0, " graft /code/api")
        if self.landed is not None:
            cv.put(0, 1, "●      ", S("accent"))
            cv.put(7, 1, "[◆ main] " + self.landed)
            draw_bar_row(cv, cv.h - 1, " committed ✓ · Backspace to try again")
            return
        if not self.open:
            draw_bar_row(cv, cv.h - 1, " c commit · the prototype's keys work here (1 knob, ] next, q quit)")
            return
        w = min(W - 4, 72)
        x0, y0 = 2, 2
        for y in range(y0, y0 + 9):
            cv.put(x0, y, " " * w, S("text", "bg"))

        def mk(i):
            return "▸" if self.focus == i else " "
        cv.put(x0, y0, "╭ " + mk(0) + "Subject " + "─" * (w - 12) + "╮", S("accent"))
        cv.put(x0 + 2, y0 + 1, self.subject + ("▏" if self.focus == 0 else ""))
        cv.put(x0, y0 + 2, "├ " + mk(1) + "Body " + "─" * (w - 9) + "┤", S("accent"))
        shown = self.body[-4:]
        for i, ln in enumerate(shown):
            cv.put(x0 + 2, y0 + 3 + i, ln + ("▏" if self.focus == 1 and i == len(shown) - 1 else ""))
        # crates/graft-ui/src/commit_editor.rs:309-316 (amend stands in for the four boxes)
        cv.put(x0 + 2, y0 + 7, f"{mk(2)} [ ] amend    {mk(3)} [ Commit ]")
        cv.put(x0, y0 + 8, "╰" + "─" * (w - 2) + "╯", S("accent"))
        if self.kv[0] == 1:
            foot = " ⏎ commit  ⇥ body  ⌃E $EDITOR  ⌃S sign-off  esc cancel"  # the draft
        else:
            foot = " ⌃⏎/Commit commit  ⇥ next  ⌃E $EDITOR  ⌃S sign-off  esc cancel"  # commit_editor.rs:171
        draw_bar_row(cv, cv.h - 1, foot)

    def note(self):
        return "in Zed ⌃⏎ never arrives · type a subject, commit it · esc closes the editor (then 1 flips, c reopens)"


class Deciding(Scene):
    question = "what you're deciding"

    LINES = [
        ("1. Colours on the conflict screen: which blue and purple, how strong, and what the line between",
         "   the panes looks like.", "scene 1 · 1 strength · 2 colour depth · 3 lines · 4 blue/purple pair"),
        ("2. When the last file is done: stay where you are, show `c finish` straight away, or go straight",
         "   to the screen showing what will land.", "scene 2 · knob 1"),
        ("3. On that screen: `y` commits and `⏎` shows a file's change, or `⏎` commits as now.", "",
         "scene 2 · knob 2"),
        ("4. How that screen looks: framed like the merge preview, or as now.", "", "scene 2 · knob 3"),
        ("5. Arrow keys, Page Up / Page Down and Shift+Tab on that screen: all, some, or none.", "",
         "scene 3 · knobs 1, 2, 3"),
        ("6. The key row at 120 columns: drop words from keys you use every day, use shorter words, or",
         "   leave it.", "scene 4 · knob 1 (2: merge / rebase)"),
        ("7. Short key words on narrow screens: keep the draft, change some, or drop the idea.", "",
         "scene 5 · knob 1 · j/k x w · < > width"),
        ("8. The line after a stop, with the changes pane open: drop “begun outside Graft”, keep it, or",
         "   shorten both.", "scene 6 · knob 2 (1: where it began)"),
        ("9. What `n` says after the last conflict in a file.", "", "scene 7 · knob 1 · w your own"),
        ("10. Starting over a file where one side deleted it or added it whole: yes or no.", "",
         "scene 8 · knob 1 (2: begun in the shell)"),
        ("11. Undo and abort on a merge you started in the shell: both, abort only, or keep doing it in",
         "   the shell.", "scene 9 · knob 1"),
        ("12. The mark on a file you haven't opened yet.", "", "scene 10 · knob 1"),
        ("13. `c` on the main screen while a merge is stopped: open the commit editor (which then says",
         "   no), or go straight to the screen showing what will land.", "scene 11 · knob 1"),
        ("14. The push box: `y` pushes as well as ⏎, or only ⏎ (today `y` closes it and nothing is",
         "   pushed). The force-push box keeps ⏎ only either way.", "scene 12 · knob 1 (2: force box)"),
        ("15. The commit message's first line: ⏎ commits (Tab reaches the longer text), or ⏎ moves",
         "   on to the longer text as now.", "scene 13 · knob 1"),
    ]

    def draw(self, cv):
        y = 0
        cv.put(0, y, "What you're deciding", S("accent", None, "b"))
        y += 2
        wide = cv.w >= 150
        for a, b, how in self.LINES:
            cv.put(0, y, a)
            if b:
                y += 1
                cv.put(0, y, b)
            if wide:
                cv.put(104, y, "→ " + how, S("accent"))
            else:
                y += 1
                cv.put(3, y, "→ " + how, S("accent"))
            y += 1
        y += 1
        cv.put(0, y, "Then two answers: does the conflict screen look right? Go on to the next milestone?")
        y += 2
        for ln in ["] / [ scenes · 1-4 knobs · < > width · * mark your pick · Backspace restart · q quit",
                   "In every scene: try it with the app's own keys; the row above counts your keys.",
                   "Picks go to .scratch/m6-conflicts/look-picks.txt when you press q."]:
            cv.put(0, y, ln, S("text-muted"))
            y += 1


# ------------------------------------------------------------------ the app

class App:
    def __init__(self, args):
        self.args = args
        self.colours = Colours(self)
        self.finishing = Finishing(self)
        self.scenes = [Deciding(self), self.colours, self.finishing, SummaryKeys(self), KeyRow120(self),
                       ShortWords(self), StopLine(self), AfterLastBlock(self), StartOver(self), ShellUndo(self),
                       UnvisitedMark(self), CommitWhileMerging(self), PushBox(self),
                       CommitSubject(self)]
        self.idx = 1
        self.task = not args.no_task
        self.splash = self.task
        self.keys = 0
        self.flash_msg = ""
        self.picks = {}
        self.log = []
        self.watch = []
        self.last_key_at = None
        self.done = False
        if self.task:
            self.colours.kv = [1, 0, 0, 0]
            self.finishing.kv = [2, 1, 1]

    # -- sizes
    def term_size(self):
        c, r = os.get_terminal_size(sys.stdout.fileno())
        return c, r

    def scene(self):
        return self.scenes[self.idx]

    def width_now(self):
        """`<`/`>` first, then --width, then the scene's own start width, then the window's."""
        tw, _ = self.term_size()
        sc = self.scene()
        w = sc.width or self.args.width or sc.default_width or tw
        return max(20, min(w, tw))

    def height_now(self):
        _, th = self.term_size()
        return th - self.header_rows() - (1 if self.idx == 1 and not self.task else 0)

    def header_rows(self):
        return 0 if self.task else 2

    def pal(self):
        c = self.colours.kv
        return Pal(DEPTHS[c[1]], c[3], ALPHAS[c[0]])

    # -- events
    def flash(self, msg):
        self.flash_msg = msg

    def event(self, what):
        if not self.task:
            return
        self.log.append(f"{self.stamp()} event: {what}")
        if what in ("last file done", "summary open"):
            self.watch.append(what)
        if what == "committed":
            self.end_task("the commit landed")

    def stamp(self):
        return time.strftime("%H:%M:%S")

    def end_task(self, why):
        self.log.append(f"{self.stamp()} task over: {why}")
        self.task = False
        self.flash("task done · the rows are on · ] next scene")

    def to_finish(self, res):
        """Scene 1's ⏎ once every block is taken. In the task it carries straight on into scene 2."""
        if self.task:
            self.idx = 2
            self.finishing.reset(res)
            self.finishing.last_file_done()
        else:
            self.goto(2)

    def goto(self, i):
        self.idx = i % len(self.scenes)
        self.scene().reset()
        self.keys = 0

    # -- keys
    def handle(self, k):
        now = time.monotonic()
        if self.splash:
            self.splash = False
            self.last_key_at = now
            self.done = k == "q"
            return
        sc = self.scene()
        if self.task:
            if self.last_key_at is not None and now - self.last_key_at > 3:
                self.log.append(f"{self.stamp()} pause {now - self.last_key_at:.1f}s")
            self.last_key_at = now
            for what in self.watch:
                self.log.append(f"{self.stamp()} FIRST KEY after “{what}”: {kname(k)}")
            self.watch = []
            self.log.append(f"{self.stamp()} key {kname(k)}")
            if k == "]" and not sc.text_mode():
                self.end_task("] pressed")
                self.goto(1)
                return
            if k == "q" and not sc.text_mode():
                self.done = True
                return
            self.keys += 1
            sc.key(k)
            return
        self.flash_msg = ""
        if not sc.text_mode():
            if k == "q":
                self.done = True
                return
            if k == "]":
                return self.goto(self.idx + 1)
            if k == "[":
                return self.goto(self.idx - 1)
            if k == "0":
                return self.goto(0)
            if k in "1234" and len(k) == 1 and self.idx:
                i = int(k) - 1
                if i < len(sc.kv):
                    sc.kv[i] = (sc.kv[i] + 1) % len(sc.knobs[i][1])
                    sc.knob_changed(i)
                return
            if k in ("<", ">"):
                ws = sc.widths
                tw, _ = self.term_size()
                cur = self.width_now()
                if k == "<":
                    nxt = [w for w in ws if w < cur]
                    sc.width = max(nxt) if nxt else cur
                else:
                    nxt = [w for w in ws if cur < w <= tw]
                    sc.width = min(nxt) if nxt else cur
                    if not nxt:
                        self.flash(f"the window is {tw} columns wide")
                return
            if k == "*":
                self.picks[self.idx] = f"scene {self.idx} ({sc.question}): {sc.pick()} · width {self.width_now()}"
                self.flash("marked as your pick")
                return
            if k in ("\x7f", "\x08"):
                sc.reset()
                self.keys = 0
                return
        self.keys += 1
        if not sc.key(k) and self.idx == 0:
            self.flash("press ] to start the scenes")

    # -- frame
    def frame(self):
        tw, th = self.term_size()
        out = ["\x1b[0m\x1b[H\x1b[2J"]
        if self.splash:
            lines = ["", "  PROTOTYPE · the look and finish of the conflict screen",
                     "", "  the merge stopped on notes.txt — take theirs on what's left and get it committed.",
                     "", "  press any key to start"]
            for i, ln in enumerate(lines):
                out.append(f"\x1b[{i + 1};1H{ln[:tw]}")
            return "".join(out)
        sc = self.scene()
        top = 1
        if not self.task:
            n = self.idx
            head = (f"scene {n}/{len(self.scenes) - 1} · {sc.question} · keys {self.keys}" if n else
                    "What you're deciding · ] first scene · q quit")
            out.append(f"\x1b[1;1H\x1b[0;1m{head[:tw]}\x1b[0m\x1b[K")
            out.append("\x1b[2;1H" + self.knob_row(sc, tw) + "\x1b[0m\x1b[K")
            top = 3
        w = self.width_now()
        h = th - (top - 1) - (1 if (self.idx == 1 and not self.task) else 0)
        cv = Canvas(w, max(1, h))
        sc.draw(cv)
        out.append(cv.render(self.pal(), top))
        if self.idx == 1 and not self.task:
            out.append(f"\x1b[{th};1H\x1b[0m{(sc.info() or '')[:tw]}\x1b[K")
        return "".join(out)

    def knob_row(self, sc, tw):
        """`1 <name>: <value>` with the value reversed and tagged; then the flash or the scene's note.
        Cut to the window by visible width, so the row never wraps."""
        segs = []
        for i, (name, opts) in enumerate(sc.knobs):
            label, tag = opts[sc.kv[i]]
            segs += [(f"{i + 1} {name}: ", ""), (label, "7"), (" " + tag, "2"), ("  ", "")]
        extra = self.flash_msg or sc.note()
        if extra:
            segs.append((extra, "33"))
        out, used = [], 0
        for text, sgr in segs:
            text = text[:max(0, tw - used)]
            if not text:
                break
            out.append(f"\x1b[0;{sgr}m{text}" if sgr else f"\x1b[0m{text}")
            used += len(text)
        return "".join(out)

    def summary(self):
        lines = [f"== look.py {datetime.datetime.now().isoformat(timespec='seconds')} =="]
        lines += ["task log:"] + (["  " + ln for ln in self.log] or ["  (no task run)"])
        lines += ["picks:"] + (["  " + self.picks[k] for k in sorted(self.picks)] or ["  (none marked)"])
        return lines


# ------------------------------------------------------------------ terminal

def read_keys(fd, decoder):
    """Split raw bytes into keys: escape sequences whole, UTF-8 characters whole (a character split
    across two reads waits in `decoder` for its tail)."""
    keys = []
    text = decoder.decode(os.read(fd, 64))
    i = 0
    while i < len(text):
        ch = text[i]
        if ch == "\x1b":
            if i + 1 < len(text) and text[i + 1] == "[":
                j = i + 2
                while j < len(text) and not ("@" <= text[j] <= "~"):
                    j += 1
                keys.append(text[i:j + 1])
                i = j + 1
                continue
            if i + 1 < len(text) and text[i + 1] == "O" and i + 2 < len(text):
                keys.append("\x1b[" + text[i + 2])
                i += 3
                continue
            keys.append("\x1b")
            i += 1
            continue
        keys.append(ch)
        i += 1
    return keys


def interactive(args):
    import termios
    import tty

    app = App(args)
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)

    def on_term(signum, frame):
        raise SystemExit(128 + signum)  # unwinds through the finally below, which restores the terminal
    signal.signal(signal.SIGTERM, on_term)
    signal.signal(signal.SIGHUP, on_term)
    sys.stdout.write("\x1b[?1049h\x1b[?25l\x1b[?7l")  # alt screen, no cursor, no autowrap
    sys.stdout.flush()
    decoder = codecs.getincrementaldecoder("utf-8")()
    try:
        tty.setraw(fd)
        last_size = None
        dirty = True
        while not app.done:
            size = app.term_size()
            if dirty or size != last_size:
                sys.stdout.write(app.frame())
                sys.stdout.flush()
                last_size, dirty = size, False
            ready = select.select([fd], [], [], 0.25)[0]
            if not ready:
                continue
            for k in read_keys(fd, decoder):
                if k == "\x03":  # Ctrl-C: raw mode delivers it as a byte
                    app.done = True
                    break
                app.handle(k)
                if app.done:
                    break
            dirty = True
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
        sys.stdout.write("\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l")
        sys.stdout.flush()
    lines = app.summary()
    print("\n".join(lines))
    path = os.path.abspath(args.picks)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "a", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n\n")
    print(f"written to {os.path.relpath(path, REPO) if path.startswith(REPO) else path}")


def main():
    ap = argparse.ArgumentParser(description="PROTOTYPE: M6 conflict look and finish (throwaway).")
    ap.add_argument("--width", type=int, choices=[80, 100, 120, 160, 187], help="draw every scene at this width")
    ap.add_argument("--no-task", action="store_true", help="skip task mode, open on scene 1")
    ap.add_argument("--picks", default=PICKS_DEFAULT, help="where q appends the picks")
    args = ap.parse_args()
    if not sys.stdin.isatty():
        sys.exit("look.py needs a terminal")
    interactive(args)


if __name__ == "__main__":
    main()
