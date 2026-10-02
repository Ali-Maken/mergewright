#!/usr/bin/env python3
"""PROTOTYPE — throwaway. Deleted when the M8 specs land.

Pipeline step 5 for docs/design-notes/m8-polish-and-ship.md, § "Prototype brief (step 5)".
Fake data, real keys. Nothing reads or writes git.

  python3 prototypes/m8-polish-and-ship/polish.py 1    # scroll feel + terminal probe (do this one first)
  python3 prototypes/m8-polish-and-ship/polish.py 2    # ? and Ctrl-P
  python3 prototypes/m8-polish-and-ship/polish.py 3    # first run and the practice tour
  python3 prototypes/m8-polish-and-ship/polish.py 4    # themes, by eye
  python3 prototypes/m8-polish-and-ship/polish.py 4 --no-colour   # the fallback, colour off

Every launch probes the terminal first (DSR round trip, DEC 2026 support, OSC 11 background) and logs it with
$TERM_PROGRAM and $COLORTERM. Every key, pause over 3 s (STUCK over 30 s), measurement and answer is appended
to prototypes/m8-polish-and-ship/picks.txt (gitignored) as it happens.

Prototype keys (not the app's), named on screen: digits answer a question when one shows · n next scene once
a scene is done · Ctrl-C quits anywhere; q quits outside a text field. Scene 1 adds 1 / 2 (blind build),
s (wheel step), space (next task), Backspace (back to the top). Ctrl-\\ saves the current frame as plain text
when --capture is given (used by selfcheck.py).

Colour: real 24-bit colour in the live terminal; captures are plain text and never show colour (CLAUDE.md
rule 5), timing or feel.
"""
import argparse
import codecs
import collections
import datetime
import os
import random
import select
import signal
import sys
import termios
import time
import tty

HERE = os.path.dirname(os.path.abspath(__file__))
PICKS_DEFAULT = os.path.join(HERE, "picks.txt")
COLOUR = [True]

# ------------------------------------------------------------------ palettes (Graft's own tokens, chrome.rs)

PAL = {
    "dark": dict(bg="#14161B", surface="#1B1E24", raised="#232730", sel="#5A6478", border="#333944",
                 text="#D7DBE0", muted="#8A919C", accent="#4FA3E3", added="#5DB87A", removed="#E06C75",
                 warning="#E5A94E", info="#6FB3D2", ours="#56B4E9", theirs="#CC79A7", open="#E69F00",
                 ours_t="#284559", theirs_t="#4B3445", open_t="#533F13"),
    "light": dict(bg="#FBFBFC", surface="#F2F3F5", raised="#E8EAED", sel="#828CA0", border="#C9CDD4",
                  text="#1E222A", muted="#5F6773", accent="#15639F", added="#15703C", removed="#B4232B",
                  warning="#8A5A00", info="#1C6B8C", ours="#0072B2", theirs="#A8467F", open="#B36B00",
                  ours_t="#B0D2E6", theirs_t="#E2C5D7", open_t="#E5D0B0"),
    "hc": dict(bg="#000000", surface="#000000", raised="#1A1A1A", sel="#696969", border="#FFFFFF",
               text="#FFFFFF", muted="#D0D0D0", accent="#FFD400", added="#00E676", removed="#FF5252",
               warning="#FF9E1F", info="#40C4FF", ours="#56B4E9", theirs="#CC79A7", open="#E69F00",
               ours_t="#1A3646", theirs_t="#3D2432", open_t="#453000"),
}
# Two dark candidates for scene 4: A = ground darker, B = text brighter. Both clear the two floors
# (selection vs ground >= 3:1, text on selection >= 4.5:1); today's dark misses the second (4.28:1).
PAL["dark_a"] = dict(PAL["dark"], bg="#0B0C10", surface="#121419", raised="#1A1D24", sel="#565F72",
                     border="#2A2F38", ours_t="#203A4B", theirs_t="#3F2A39", open_t="#47350F")
PAL["dark_b"] = dict(PAL["dark"], sel="#5C667B", text="#F0F2F5", muted="#9AA1AC")
LANES = ["#0077BB", "#EE8866", "#AA3377", "#77AADD", "#DDCC77", "#009988", "#CCBB44", "#CC3311"]


def rgb_of(h):
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))


def luminance(h):
    out = []
    for c in rgb_of(h):
        c /= 255
        out.append(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4)
    return 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2]


def contrast(a, b):
    x, y = sorted([luminance(a), luminance(b)])
    return (y + 0.05) / (x + 0.05)


# ------------------------------------------------------------------ canvas

_SGR = {}


def sgr(st):
    key = (st, COLOUR[0])
    if key in _SGR:
        return _SGR[key]
    fg, bg, at = st
    parts = ["0"]
    for ch, code in (("b", "1"), ("d", "2"), ("u", "4"), ("r", "7")):
        if ch in at:
            parts.append(code)
    if COLOUR[0]:
        if fg:
            parts.append("38;2;%d;%d;%d" % rgb_of(fg))
        if bg:
            parts.append("48;2;%d;%d;%d" % rgb_of(bg))
    _SGR[key] = out = "\x1b[" + ";".join(parts) + "m"
    return out


class Canvas:
    def __init__(self, w, h, fg=None, bg=None):
        self.w, self.h = w, h
        self.fg, self.bg = fg, bg
        self.ch = [[" "] * w for _ in range(h)]
        self.st = [[(fg, bg, "")] * w for _ in range(h)]

    def put(self, x, y, text, fg=None, bg=None, at="", maxw=None):
        if y < 0 or y >= self.h:
            return x
        lim = self.w if maxw is None else min(self.w, x + maxw)
        for c in text:
            if x >= lim:
                break
            if x >= 0:
                old = self.st[y][x]
                self.ch[y][x] = c
                self.st[y][x] = (fg or self.fg, bg or old[1], at)
            x += 1
        return x

    def fill(self, x, y, w, h, bg, fg=None):
        for yy in range(max(0, y), min(self.h, y + h)):
            for xx in range(max(0, x), min(self.w, x + w)):
                self.ch[yy][xx] = " "
                self.st[yy][xx] = (fg or self.fg, bg, "")

    def rbg(self, y, bg, x=0, w=None):
        """Set the background of a row span, keeping its text."""
        if not 0 <= y < self.h:
            return
        w = self.w - x if w is None else w
        for xx in range(max(0, x), min(self.w, x + w)):
            fg, _, at = self.st[y][xx]
            self.st[y][xx] = (fg, bg, at)

    def box(self, x, y, w, h, fg, bg, title="", right=""):
        self.fill(x, y, w, h, bg)
        self.put(x, y, "╭" + "─" * (w - 2) + "╮", fg, bg)
        for yy in range(y + 1, y + h - 1):
            self.put(x, yy, "│", fg, bg)
            self.put(x + w - 1, yy, "│", fg, bg)
        self.put(x, y + h - 1, "╰" + "─" * (w - 2) + "╯", fg, bg)
        if title:
            self.put(x + 2, y, " " + title + " ", fg, bg, "b")
        if right:
            self.put(x + w - 3 - len(right), y, " " + right + " ", fg, bg)

    def lines(self):
        return ["".join(r).rstrip() for r in self.ch]

    def ansi(self):
        out = []
        for y in range(self.h):
            out.append("\x1b[%d;1H" % (y + 1))
            cur = None
            row_ch, row_st = self.ch[y], self.st[y]
            for x in range(self.w):
                st = row_st[x]
                if st != cur:
                    out.append(sgr(st))
                    cur = st
                out.append(row_ch[x])
        out.append("\x1b[0m")
        return "".join(out)


def cut(s, n):
    return s if len(s) <= n else s[:max(0, n - 1)] + "…"


# ------------------------------------------------------------------ log (picks.txt)

class Log:
    def __init__(self, path):
        self.fh = open(path, "a", encoding="utf-8")
        self.t0 = time.monotonic()
        self.scene = "-"
        self.run = None  # [name, n, start, end]
        self.last_input = time.monotonic()

    def _w(self, text):
        stamp = datetime.datetime.now().strftime("%H:%M:%S")
        self.fh.write("%s +%7.1fs [%s] %s\n" % (stamp, time.monotonic() - self.t0, self.scene, text))
        self.fh.flush()

    def line(self, text):
        self.flush_run()
        self._w(text)

    def flush_run(self):
        if self.run:
            name, n, a, b = self.run
            self._w("key %s" % name if n == 1 else "key %s ×%d over %.2fs" % (name, n, b - a))
            self.run = None

    def input(self, name):
        now = time.monotonic()
        gap = now - self.last_input
        if gap > 3:
            self.flush_run()
            self._w("pause %.1fs%s" % (gap, "  STUCK (over 30 s)" if gap > 30 else ""))
        self.last_input = now
        if self.run and self.run[0] == name:
            self.run[1] += 1
            self.run[3] = now
        else:
            self.flush_run()
            self.run = [name, 1, now, now]


# ------------------------------------------------------------------ terminal input

class Ev:
    __slots__ = ("kind", "v", "x", "y", "t")

    def __init__(self, kind, v=None, x=0, y=0, t=0.0):
        self.kind, self.v, self.x, self.y, self.t = kind, v, x, y, t

    def __repr__(self):
        return "Ev(%s,%r,%d,%d)" % (self.kind, self.v, self.x, self.y)


NAMES = {"\r": "enter", "\n": "enter", "\t": "tab", "\x7f": "bs", "\x08": "bs", "\x03": "ctrl-c",
         "\x10": "ctrl-p", "\x0e": "ctrl-n", "\x15": "ctrl-u", "\x04": "ctrl-d", "\x1c": "capture",
         "\x1a": "ctrl-z", "\x12": "ctrl-r", "\x01": "ctrl-a"}
CSI_KEYS = {"A": "up", "B": "down", "C": "right", "D": "left", "H": "home", "F": "end", "Z": "shift-tab"}
TILDE = {"1": "home", "7": "home", "4": "end", "8": "end", "5": "pgup", "6": "pgdn", "3": "delete"}


class Term:
    def __init__(self, fd_in, fd_out):
        self.fd, self.out = fd_in, fd_out
        self.dec = codecs.getincrementaldecoder("utf-8")("replace")
        self.buf = ""
        self.paste = None

    def write(self, data):
        b = data.encode("utf-8")
        while b:
            n = os.write(self.out, b)
            b = b[n:]

    def _wait(self, timeout):
        return bool(select.select([self.fd], [], [], timeout)[0])

    def read(self, timeout):
        if not self._wait(timeout):
            return []
        evs = []
        for _ in range(4):
            chunk = os.read(self.fd, 65536)
            if not chunk:
                raise SystemExit(0)  # the terminal went away
            self.buf += self.dec.decode(chunk)
            evs += self._parse()
            if not self.buf or not self._wait(0.03):
                break
        if self.buf == "\x1b":  # a lone Esc with nothing after it in 30 ms is the Esc key
            evs.append(Ev("key", "esc"))
            self.buf = ""
        now = time.monotonic()
        for e in evs:
            e.t = now
        return evs

    def _parse(self):
        buf, i, n, evs = self.buf, 0, len(self.buf), []
        while i < n:
            if self.paste is not None:
                j = buf.find("\x1b[201~", i)
                if j < 0:
                    self.paste += buf[i:]
                    i = n
                    break
                evs.append(Ev("paste", self.paste + buf[i:j]))
                self.paste = None
                i = j + 6
                continue
            c = buf[i]
            if c != "\x1b":
                evs.append(Ev("key", NAMES.get(c, c)))
                i += 1
                continue
            if i + 1 >= n:
                break
            d = buf[i + 1]
            if d == "[":
                j = i + 2
                while j < n and not ("\x40" <= buf[j] <= "\x7e"):
                    j += 1
                if j >= n:
                    break
                body, final = buf[i + 2:j], buf[j]
                i = j + 1
                if body == "200" and final == "~":
                    self.paste = ""
                    continue
                evs.append(csi_event(body, final))
                continue
            if d == "]":
                j1 = buf.find("\x07", i + 2)
                j2 = buf.find("\x1b\\", i + 2)
                ends = [(j1, 1), (j2, 2)]
                ends = [e for e in ends if e[0] >= 0]
                if not ends:
                    break
                j, ln = min(ends)
                evs.append(Ev("osc", buf[i + 2:j]))
                i = j + ln
                continue
            if d == "O":
                if i + 2 >= n:
                    break
                evs.append(Ev("key", CSI_KEYS.get(buf[i + 2], "ss3:" + buf[i + 2])))
                i += 3
                continue
            if d == "\x1b":
                evs.append(Ev("key", "esc"))
                i += 1
                continue
            evs.append(Ev("key", "alt-" + d))
            i += 2
        self.buf = buf[i:]
        return evs


def csi_event(body, final):
    if body.startswith("<") and final in "Mm":
        try:
            b, x, y = (int(p) for p in body[1:].split(";"))
        except ValueError:
            return Ev("key", "csi:" + body + final)
        x, y = x - 1, y - 1
        if b & 64:
            return Ev("mouse", ["wheel_up", "wheel_down", "wheel_left", "wheel_right"][b & 3], x, y)
        if b & 32:
            return Ev("mouse", "move" if (b & 3) == 3 else "drag", x, y)
        return Ev("mouse", "press" if final == "M" else "release", x, y)
    if final == "n" and body in ("0", "3"):
        return Ev("dsr", body)
    if final == "y" and body.startswith("?") and body.endswith("$"):
        parts = body[1:-1].split(";")
        if len(parts) == 2:
            return Ev("decrpm", parts[0], int(parts[1]) if parts[1].isdigit() else -1)
    if final == "R":
        return Ev("cpr", body)
    if final == "~":
        return Ev("key", TILDE.get(body.split(";")[0], "csi:" + body + "~"))
    if final in CSI_KEYS:
        mod = body.split(";")[1] if ";" in body else ""
        name = CSI_KEYS[final]
        return Ev("key", {"3": "alt-", "5": "ctrl-", "2": "shift-"}.get(mod, "") + name)
    return Ev("key", "csi:" + body + final)


# ------------------------------------------------------------------ fake repo screens shared by scenes 2 and 3

BILLING = [
    # graph, refs, subject, author, when, sha
    ("◌", [], "Uncommitted changes · 3 files", "", "", ""),
    ("●", ["main", "origin/main"], "Add invoice totals", "Ali", "2 hours ago", "9f2c1ab"),
    ("●", [], "Fix rounding in tax lines", "Lina Park", "5 hours ago", "41d0e7c"),
    ("●─╮", [], "Merge pull request #212 from acme/csv-export", "Tom Ruiz", "yesterday", "c8a1f02"),
    ("│ ●", ["feature/csv"], "Add CSV export", "Tom Ruiz", "yesterday", "77be3d9"),
    ("│ ●", [], "Escape commas in CSV", "Tom Ruiz", "2 days ago", "0d4c5aa"),
    ("●─╯", [], "Speed up ledger query", "Mei Chen", "2 days ago", "e19f6b4"),
    ("●", ["v2.3.0"], "Validate IBAN input", "Ali", "3 days ago", "5a7d2e1"),
    ("●", [], "Make refunds idempotent", "Priya Nair", "4 days ago", "b3c90f8"),
    ("●─╮", [], "Merge branch 'fix/rounding'", "Lina Park", "5 days ago", "2e6f1d3"),
    ("│ ●", ["fix/rounding"], "Round half to even", "Lina Park", "5 days ago", "9c0a4b7"),
    ("●─╯", [], "Add audit trail", "Sam Okafor", "6 days ago", "6d2b8e0"),
    ("●", [], "Log slow requests", "Mei Chen", "last week", "f01c3a9"),
    ("●", [], "Initial import of billing", "Ali", "2 months ago", "a1b2c3d"),
]
PAYMENTS = [
    ("●", ["main", "origin/main"], "Retry declined cards once", "Ali", "yesterday", "3e8d1f0"),
    ("●", [], "Add 3-D Secure fallback", "Ali", "yesterday", "c19a7b2"),
    ("●─╮", [], "Merge pull request #88 from acme/payouts", "Priya Nair", "2 days ago", "8b0e4c1"),
    ("│ ●", ["payouts"], "Batch payouts nightly", "Priya Nair", "3 days ago", "4f7a2d9"),
    ("●─╯", [], "Split payment service", "Ali", "4 days ago", "d2c6e88"),
    ("●", [], "Initial commit", "Ali", "3 months ago", "00a9f1e"),
]


def draw_graph_rows(cv, pal, rows, cur, x, y, w, h, hits=None, top=0):
    """rows: (graph, refs, subject, author, when, sha). Lane i is drawn in LANES[i]."""
    for i in range(h):
        idx = top + i
        if idx >= len(rows):
            break
        g, refs, subj, who, when, sha = rows[idx]
        yy = y + i
        sel = idx == cur
        cx = x
        cv.put(cx, yy, "▸" if sel else " ", pal["accent"], at="b")
        cx += 2
        for k, ch in enumerate(g.ljust(6)):
            lane = k // 2
            colour = pal["muted"] if ch == "◌" else LANES[lane % len(LANES)]
            if ch in "─╮╯":
                colour = LANES[(lane + (1 if ch == "─" else 0)) % len(LANES)]
            cv.put(cx + k, yy, ch, colour)
        cx += 7
        right = "%-11s %-12s %s" % (cut(who, 11), when, sha) if who else ""
        avail = w - (cx - x) - len(right) - 2
        for r in refs:
            if avail < len(r) + 4:
                break
            fg = pal["accent"] if not r.startswith("v") else pal["warning"]
            if hits is not None:
                hits.append((yy, cx, cx + len(r) + 2, r))
            cx = cv.put(cx, yy, "[" + r + "]", fg, at="b") + 1
            avail -= len(r) + 3
        cv.put(cx, yy, cut(subj, max(0, avail)), pal["muted"] if g == "◌" else pal["text"])
        if right:
            cv.put(x + w - len(right) - 1, yy, right, pal["muted"])
        if sel:
            cv.rbg(yy, pal["sel"], x, w)
            if not COLOUR[0]:
                for xx in range(x, min(cv.w, x + w)):
                    fg, bg, at = cv.st[yy][xx]
                    cv.st[yy][xx] = (fg, bg, at + "r")


# ------------------------------------------------------------------ key lists for ? and Ctrl-P

# (key, label, config name, extra search words)
HERE_GRAPH = [
    ("c", "commit", "graph.commit", ""),
    ("a", "amend the last commit", "graph.amend", "reword fix message"),
    ("m", "merge the selected branch into yours", "graph.merge", ""),
    ("r", "rebase yours onto the selected commit", "graph.rebase", ""),
    ("b", "new branch here", "graph.branch", "create"),
    ("t", "new tag here", "graph.tag", ""),
    ("x", "compare: mark this commit, then pick another", "graph.compare", "diff two"),
    ("S", "stash…", "graph.stash", "aside shelve save park"),
    ("C", "switch branch (branch picker)", "graph.branch_picker", "checkout"),
    ("o", "take this commit onto yours (cherry-pick)", "graph.cherry_pick", ""),
    ("R", "rewrite: squash, reorder, edit", "graph.rewrite", "interactive rebase"),
    ("f", "fetch", "graph.fetch", "download"),
    ("p", "pull", "graph.pull", "update"),
    ("P", "push", "graph.push", "upload"),
    ("⏎", "open the commit", "graph.open", "details"),
    ("y", "copy the commit id", "graph.copy_sha", "sha hash"),
    ("e", "open in your editor", "graph.edit", ""),
    ("M", "map of all branches", "graph.map", ""),
    ("F", "focus on this commit", "graph.focus", ""),
    ("1", "first-parent view", "graph.first_parent", ""),
]
EVERYWHERE = [
    ("⌃P", "find anything (palette)", "app.palette", "command"),
    ("?", "keys here", "app.help", "shortcuts"),
    ("u", "undo the last thing you did", "app.undo", "back revert"),
    ("U", "redo", "app.redo", ""),
    ("/", "find in this list", "app.find", "search"),
    (".", "actions for the selected row", "app.menu", ""),
    ("⇥", "next pane", "app.next_pane", ""),
    ("q", "quit", "app.quit", "exit"),
]
IN_PROGRESS = [
    ("c", "continue the rebase", "operation.continue", "go on resume next"),
    ("s", "skip this commit", "operation.skip", ""),
    ("a", "abort: back to where it started", "operation.abort", "cancel stop"),
    ("⏎", "open the stopped commit", "operation.open", ""),
    ("e", "open the file in your editor", "operation.edit", ""),
]
PALETTE_EXTRA = [
    ("BRANCHES", [("", "main", "", "branch"), ("", "feature/csv", "", "branch"), ("", "fix/rounding", "", "branch")]),
    ("TAGS", [("", "v2.3.0", "", "tag")]),
    ("FILES", [("", "src/invoice.rs", "", "file"), ("", "src/tax.rs", "", "file"), ("", "README.md", "", "file")]),
    ("COMMITS", [("", "Add invoice totals", "", "commit"), ("", "Fix rounding in tax lines", "", "commit")]),
    ("RECENT", [("", "payments-service", "", "repo"), ("", "web-dashboard", "", "repo")]),
]


def fuzzy(q, text):
    """Substring, or every typed word starts some word of the row (`sw br` finds `switch branch`)."""
    q, text = q.lower(), text.lower()
    if q in text:
        return 2
    words = text.replace(".", " ").replace("_", " ").split()
    return 1 if all(any(w.startswith(t) for w in words) for t in q.split()) else 0


class Finder:
    """`?` (help: IN PROGRESS → HERE → EVERYWHERE) or ⌃P (palette: those + BRANCHES … RECENT)."""

    def __init__(self, kind, groups):
        self.kind, self.groups = kind, groups
        self.query, self.cur = "", 0

    def rows(self):
        q = self.query.strip()
        out = []
        if len(q) == 1 or q in ("⏎", "⌃P"):  # a bound key typed alone ranks its row first
            for title, items in self.groups:
                for it in items:
                    if it[0] == q:
                        out.append(("KEY " + q, it))
        for title, items in self.groups:
            for it in items:
                if any(it is o[1] for o in out):
                    continue
                if not q or fuzzy(q, it[1] + " " + it[3] + " " + it[2]):
                    out.append((title, it))
        return out

    def key(self, k):
        if k == "esc":
            return ("close", None)
        if k in ("down", "ctrl-n"):
            self.cur += 1
        elif k in ("up",):
            self.cur = max(0, self.cur - 1)
        elif k == "bs":
            self.query = self.query[:-1]
            self.cur = 0
        elif k == "ctrl-u":
            self.query, self.cur = "", 0
        elif k == "enter":
            rows = self.rows()
            if rows:
                return ("run", rows[min(self.cur, len(rows) - 1)][1])
        elif len(k) == 1 and k.isprintable():
            self.query += k
            self.cur = 0
        rows = self.rows()
        self.cur = min(self.cur, max(0, len(rows) - 1))
        return None

    def draw(self, cv, pal, x0, y0, wmax, hmax):
        w = min(96, wmax) if cv.w >= 100 else cv.w
        x = x0 + max(0, (wmax - w) // 2) if cv.w >= 100 else 0
        rows = self.rows()
        lines = []
        last = None
        for title, it in rows:
            if title != last:
                lines.append(("head", title))
                last = title
            lines.append(("row", it))
        h = min(hmax, max(6, len(lines) + 4))
        cv.box(x, y0, w, h, pal["border"], pal["raised"], "keys here" if self.kind == "help" else "palette",
               "esc close")
        cv.put(x + 2, y0 + 1, "› ", pal["accent"], pal["raised"], "b")
        cv.put(x + 4, y0 + 1, self.query + "▏", pal["text"], pal["raised"])
        if not rows:
            cv.put(x + 2, y0 + 3, "nothing matches · esc close", pal["muted"], pal["raised"])
            return
        # keep the cursor row in view
        row_i = 0
        cur_line = 0
        for li, (kind, _) in enumerate(lines):
            if kind == "row":
                if row_i == self.cur:
                    cur_line = li
                row_i += 1
        room = h - 4
        start = max(0, cur_line - room + 1)
        row_i = sum(1 for k, _ in lines[:start] if k == "row")
        for i, (kind, v) in enumerate(lines[start:start + room]):
            yy = y0 + 2 + i
            if kind == "head":
                cv.put(x + 2, yy, v, pal["muted"], pal["raised"], "b")
                continue
            key, label, cfg, _ = v
            sel = row_i == self.cur
            bg = pal["sel"] if sel else pal["raised"]
            cv.put(x + 1, yy, "▸" if sel else " ", pal["accent"], bg, "b")
            cv.put(x + 2, yy, (key or "—").ljust(6), pal["accent"] if key else pal["muted"], bg, "b")
            cv.put(x + 8, yy, cut(label, w - 12 - (len(cfg) + 2 if cv.w >= 100 else 0)), pal["text"], bg)
            if cfg and cv.w >= 100:
                cv.put(x + w - 2 - len(cfg), yy, cfg, pal["muted"], bg)
            cv.rbg(yy, bg, x + 1, w - 2)
            if sel and not COLOUR[0]:
                for xx in range(x + 1, x + w - 1):
                    fg, b2, at = cv.st[yy][xx]
                    cv.st[yy][xx] = (fg, b2, at + "r")
            row_i += 1


# ------------------------------------------------------------------ scene base

class Scene:
    num = 0
    title = ""
    custom_loop = False

    def __init__(self, app):
        self.app = app
        self.pal = app.pal
        self.q = None          # (qid, text, [options]) while a question is on screen
        self.done = False      # question answered: the scene is over
        self.flash = ""
        self.flash_until = 0.0

    def say(self, text, secs=4.0):
        self.flash = text
        self.flash_until = time.monotonic() + secs

    def task(self):
        return ""

    def bar(self):
        return ""

    def body(self, cv, y0, h):
        pass

    def on(self, ev):
        pass

    def tick(self, now):
        return None  # seconds until the next timed change, or None

    def answered(self, n):
        pass

    def text_mode(self):
        return False

    def frame(self, w, h):
        pal = self.pal
        cv = Canvas(w, h, pal["text"], pal["bg"])
        task = self.task()
        cv.put(1, 0, "SCENE %d · %s" % (self.num, self.title), pal["muted"], at="b")
        cv.put(1, 1, cut(task, w - 2), pal["text"], at="b")
        cv.put(0, 2, "─" * w, pal["border"])
        self.body(cv, 3, h - 6)
        cv.put(0, h - 3, "─" * w, pal["border"])
        if self.q:
            qid, text, opts = self.q
            cv.fill(0, h - 6, w, 3, pal["raised"])
            cv.put(1, h - 6, "QUESTION  " + text, pal["warning"], pal["raised"], "b")
            cv.put(5, h - 5, "     ".join("%d  %s" % (i + 1, o) for i, o in enumerate(opts)), pal["text"],
                   pal["raised"])
        elif self.done:
            cv.fill(0, h - 6, w, 3, pal["raised"])
            nxt = "n next scene · " if self.num < 4 else ""
            cv.put(1, h - 5, "✓ Thank you, that's this scene. %sq quit" % nxt, pal["added"], pal["raised"], "b")
        msg = self.flash if time.monotonic() < self.flash_until else self.status()
        cv.put(1, h - 2, cut(msg, w - 2), pal["added"] if msg.startswith("✓") else pal["text"])
        cv.put(1, h - 1, cut(self.bar(), w - 2), pal["muted"])
        return cv

    def status(self):
        return ""


# ------------------------------------------------------------------ scene 1: scroll feel + terminal probe

SUBJECTS = ["Fix rounding in invoice totals", "Add retry to webhook sender", "Refactor billing client",
            "Update README", "Handle expired tokens", "Tidy logging", "Speed up ledger query",
            "Add tax region table", "Fix flaky export test", "Rename customer fields", "Add CSV export",
            "Guard against empty cart", "Log slow requests", "Cache exchange rates",
            "Fix typo in email template", "Split payment service", "Add pagination to invoices",
            "Remove dead feature flag", "Validate IBAN input", "Make refunds idempotent",
            "Add metrics for retries", "Fix timezone in reports", "Clean up migrations", "Add audit trail",
            "Handle partial refunds", "Move config to env", "Add healthcheck endpoint", "Drop unused index"]
PREFIX = ["", "", "", "feat: ", "fix: ", "refactor: ", "test: ", "docs: "]
AUTHORS = ["Ali", "Lina Park", "Tom Ruiz", "Mei Chen", "Sam Okafor", "Priya Nair", "Jonas Berg"]
TARGET_SUBJECT = "chore: bump deps"


def gen_history(n=17694, seed=8):
    rnd = random.Random(seed)
    when = datetime.datetime(2026, 10, 2, 16, 40)
    two_weeks = datetime.datetime(2026, 9, 18, 15, 0)
    lanes = [0]          # colour id per lane slot, None = free
    next_colour = 1
    rows = []
    target = None
    for i in range(n):
        cells = []
        active = [k for k, c in enumerate(lanes) if c is not None]
        kind = "plain"
        r = rnd.random()
        if r < 0.06 and len(active) < 6:
            kind = "open"
        elif r < 0.14 and len(active) > 1:
            kind = "close"
        if kind == "open":
            c = 0
            k = lanes.index(None) if None in lanes else len(lanes)
            if k == len(lanes):
                lanes.append(None)
            lanes[k] = next_colour
            next_colour += 1
        elif kind == "close":
            k = rnd.choice(active[1:])
            c = max(a for a in active if a < k)
        else:
            weights = [6 if a == 0 else 2 for a in active]
            c = rnd.choices(active, weights)[0]
        width = len(lanes)
        line = [(" ", None)] * (2 * width)
        for a in active:
            line[2 * a] = ("│", lanes[a])
        if kind == "open":
            for xx in range(2 * c + 1, 2 * k):
                line[xx] = ("┼" if line[xx][0] == "│" else "─", lanes[k])
            line[2 * k] = ("╮", lanes[k])
        elif kind == "close":
            for xx in range(2 * c + 1, 2 * k):
                line[xx] = ("┼" if line[xx][0] == "│" else "─", lanes[k])
            line[2 * k] = ("╯", lanes[k])
            lanes[k] = None
            while len(lanes) > 1 and lanes[-1] is None:
                lanes.pop()
        line[2 * c] = ("●", lanes[c] if lanes[c] is not None else 0)
        cells = line
        if kind == "open":
            subj = "Merge pull request #%d from acme/%s" % (9000 - i // 2, rnd.choice(
                ["csv-export", "payouts", "tax-fix", "retries", "audit", "refunds", "ledger"]))
        else:
            subj = rnd.choice(PREFIX) + rnd.choice(SUBJECTS)
        refs = []
        if i == 0:
            refs = ["main", "origin/main"]
        elif i % 437 == 0:
            refs = ["v3.%d.0" % (60 - i // 437)]
        if target is None and when <= two_weeks:
            target = i
            subj = TARGET_SUBJECT
        sha = "%07x" % rnd.getrandbits(28)
        rows.append((cells, refs, subj, rnd.choice(AUTHORS), when.strftime("%a %b %d %H:%M"), sha))
        when -= datetime.timedelta(minutes=rnd.expovariate(1 / 57.0))
    return rows, target


ATTEMPTS = [("keyboard", 0), ("keyboard", 1), ("trackpad", 0), ("trackpad", 1), ("wheel", 0), ("wheel", 1)]
TASK1 = {
    "keyboard": "Go back about two weeks, fast, and stop on the commit that says “bump deps”. Use the keyboard.",
    "trackpad": "Same again with the trackpad: flick back about two weeks and stop on “bump deps”.",
    "wheel": "Same again with a mouse wheel, if you have one (no mouse: skip it). Try both wheel steps.",
}


class ScrollScene(Scene):
    num, title, custom_loop = 1, "scroll feel", True

    def __init__(self, app):
        super().__init__(app)
        self.rows, self.target = gen_history()
        kinds = ["paced", "unpaced"]
        random.shuffle(kinds)
        self.kinds = kinds              # label "1" = kinds[0], "2" = kinds[1]
        self.label = 0
        self.attempt = 0
        self.ticked = set()
        self.cur = self.top = 0
        self.step = 3
        self.seq = 0                    # motion input counter, stamped into each frame
        self.last_motion = 0.0
        self.attempt_t0 = None
        self.attempt_keys = 0
        self.on_target_since = None
        self.reset_at = None
        self.view_h = 20
        app.log.line("blind builds: 1 = %s, 2 = %s (don't read this file until you're done)" % tuple(kinds))
        app.log.line("target 'bump deps' is row %d of %d" % (self.target, len(self.rows)))
        self.fence = {"paced": [], "unpaced": []}
        self.fence_timeouts = 0

    def build(self):
        return self.kinds[self.label]

    def task(self):
        if self.done:
            return "Done."
        if self.q:
            return "That's all six tries."
        return TASK1[ATTEMPTS[self.attempt][0]]

    def bar(self):
        if self.q or self.done:
            return ""
        return ("1 / 2 switch build · s wheel step: %d row%s · space next task · ⌫ back to the top · q quit"
                % (self.step, "" if self.step == 1 else "s"))

    def status(self):
        p = self.app.probe
        fence = ("answers the fence: yes, %.1f ms typical" % p["dsr_p50"]) if p.get("dsr_p50") is not None \
            else "answers the fence: no (16 ms fallback)"
        sync = {True: "yes", False: "no", None: "no answer"}[p.get("sync")]
        return "row %s of %s · %s · frame sync: %s · %s" % (
            "{:,}".format(self.cur + 1), "{:,}".format(len(self.rows)), fence, sync, p.get("who", ""))

    def body(self, cv, y0, h):
        pal = self.pal
        self.view_h = h - 1
        done = " ".join(("✓" if (k, l) in self.ticked else "·") + " %s %d" % (k, l + 1) for k, l in ATTEMPTS)
        cv.put(1, y0, "build %d" % (self.label + 1), pal["warning"], at="b")
        cv.put(10, y0, done, pal["muted"])
        w = cv.w
        for i in range(self.view_h):
            idx = self.top + i
            if idx >= len(self.rows):
                break
            yy = y0 + 1 + i
            cells, refs, subj, who, when, sha = self.rows[idx]
            sel = idx == self.cur
            cv.put(0, yy, "▸" if sel else " ", pal["accent"], at="b")
            x = 2
            for ch, col in cells:
                cv.put(x, yy, ch, LANES[col % len(LANES)] if col is not None else None)
                x += 1
            x = max(x, 14) + 1
            for r in refs:
                x = cv.put(x, yy, "[" + r + "]", pal["warning"] if r.startswith("v") else pal["accent"], at="b") + 1
            right = "%-10s %s  %s" % (who, when, sha)
            cv.put(x, yy, cut(subj, w - x - len(right) - 3), pal["text"])
            cv.put(w - len(right) - 1, yy, right, pal["muted"])
            if sel:
                cv.rbg(yy, pal["sel"])
                if not COLOUR[0]:
                    for xx in range(w):
                        fg, bg, at = cv.st[yy][xx]
                        cv.st[yy][xx] = (fg, bg, at + "r")

    # -------- motion

    def clamp(self):
        n = len(self.rows)
        self.cur = max(0, min(n - 1, self.cur))
        vh = max(1, self.view_h)
        if self.cur < self.top:
            self.top = self.cur
        elif self.cur >= self.top + vh:
            self.top = self.cur - vh + 1
        self.top = max(0, min(n - vh, self.top))

    def motion(self, ev, now):
        k = ev.v
        if ev.kind == "key":
            d = {"j": 1, "down": 1, "k": -1, "up": -1, "ctrl-d": self.view_h // 2, "ctrl-u": -(self.view_h // 2),
                 "pgdn": self.view_h, "pgup": -self.view_h}.get(k)
            if d is not None:
                self.cur += d
            elif k == "G":
                self.cur = len(self.rows) - 1
            elif k == "g" or k == "home":
                self.cur = 0
            else:
                return None
            self.clamp()
            return "key"
        if ev.kind == "mouse":
            if k in ("wheel_down", "wheel_up"):
                d = self.step if k == "wheel_down" else -self.step
                self.top = max(0, min(len(self.rows) - self.view_h, self.top + d))
                self.cur = max(self.top, min(self.top + self.view_h - 1, self.cur))
                return "wheel"
            if k == "press":
                row = ev.y - 4
                if 0 <= row < self.view_h:
                    self.cur = self.top + row
                    self.clamp()
                    return "click"
        return None

    def answered(self, n):
        self.app.log.line("wheel step left on: %d" % self.step)

    def next_attempt(self):
        self.attempt += 1
        self.cur = self.top = 0
        self.attempt_t0 = None
        self.attempt_keys = 0
        self.on_target_since = None
        self.reset_at = None
        if self.attempt >= len(ATTEMPTS):
            self.q = ("smooth", "Which felt smoother?", ["build 1", "build 2", "no difference"])
            self.app.log.line("question: which felt smoother")
        else:
            self.label = ATTEMPTS[self.attempt][1]
            self.app.log.line("task %d: %s, build %d (%s)" % (self.attempt + 1, ATTEMPTS[self.attempt][0],
                                                            self.label + 1, self.build()))

    def on(self, ev):
        """Non-motion keys. Returns True when the frame changed."""
        k = ev.v
        if ev.kind != "key" or self.q or self.done:
            return False
        if k in ("1", "2"):
            self.label = int(k) - 1
            self.app.log.line("switched to build %s (%s)" % (k, self.build()))
            return True
        if k == "s":
            self.step = 1 if self.step == 3 else 3
            self.app.log.line("wheel step now %d" % self.step)
            return True
        if k == " ":
            self.app.log.line("skipped task %d" % (self.attempt + 1))
            self.next_attempt()
            return True
        if k == "bs":
            self.cur = self.top = 0
            return True
        return False

    def check_target(self, now):
        """Ticks the attempt once the cursor rests on the target for 0.6 s."""
        if self.q or self.done or self.attempt >= len(ATTEMPTS):
            return None
        if self.reset_at is not None:
            if now >= self.reset_at:
                self.next_attempt()
                return 0
            return self.reset_at - now
        if self.cur != self.target:
            self.on_target_since = None
            return None
        if self.on_target_since is None:
            self.on_target_since = now
        if now - self.on_target_since >= 0.6 and now - self.last_motion >= 0.6:
            kind, lab = ATTEMPTS[self.attempt]
            self.ticked.add((kind, self.label))
            took = now - (self.attempt_t0 or now)
            self.app.log.line("REACHED bump deps: task %s, build %d (%s), %.1fs from first move, %d inputs" % (
                kind, self.label + 1, self.build(), took, self.attempt_keys))
            self.say("✓ You're on it. Next task in a moment…", 1.5)
            self.reset_at = now + 1.5
            return 1.5
        return max(0.01, 0.6 - (now - self.on_target_since), 0.6 - (now - self.last_motion))

    def loop(self):
        app, term = self.app, self.app.term
        have_dsr = app.probe.get("dsr_p50") is not None
        fence_timeout = 0.5 if have_dsr else 0.016
        inflight = collections.deque()   # (t_written, seq, build) per frame, when the terminal answers DSR
        awaiting = False
        fence_t = 0.0
        dirty = True
        burst = None
        answered_seq = -1
        while app.scene is self and not app.done:
            now = time.monotonic()
            paced = self.build() == "paced"
            if dirty and not (paced and awaiting):
                w, h = app.size()
                cv = self.frame(w, h)
                app.last_canvas = cv
                data = cv.ansi()
                if paced:
                    data = "\x1b[?2026h" + data + "\x1b[?2026l"
                if have_dsr:
                    data += "\x1b[5n"
                t0 = time.monotonic()
                term.write(data)
                if have_dsr:
                    inflight.append((t0, self.seq, self.build()))
                if paced:
                    awaiting, fence_t = True, t0
                dirty = False
            timeouts = []
            if paced and awaiting:
                timeouts.append(fence_t + fence_timeout - now)
            if burst:
                timeouts.append(burst["last"] + 0.3 - now)
            t = self.check_target(now)
            if t is not None:
                timeouts.append(t)
                if t == 0:
                    dirty = True
            if time.monotonic() < self.flash_until:
                timeouts.append(self.flash_until - now)
            timeout = max(0.0, min(timeouts)) if timeouts else None
            evs = app.next_events(timeout)
            now = time.monotonic()
            if self.flash and now >= self.flash_until and self.flash_until:
                self.flash_until = 0
                dirty = True
            for ev in evs:
                if ev.kind == "dsr":
                    if inflight:
                        tw, seq, b = inflight.popleft()
                        self.fence[b].append(ev.t - tw)
                        answered_seq = max(answered_seq, seq)
                        if burst and burst.get("settled") is None and seq >= burst["seq"]:
                            burst["settled"] = ev.t
                    if paced and not inflight:
                        awaiting = False
                    continue
                if ev.kind in ("decrpm", "osc", "cpr", "paste"):
                    continue
                if app.global_key(ev):
                    dirty = True
                    continue
                kind = self.motion(ev, now) if not (self.q or self.done) else None
                if kind:
                    self.seq += 1
                    self.last_motion = ev.t
                    self.attempt_keys += 1
                    if self.attempt_t0 is None:
                        self.attempt_t0 = ev.t
                    app.log.input(ev.v)
                    src = "wheel" if kind == "wheel" else "key"
                    if burst and (burst["src"] != src or ev.t - burst["last"] > 0.3):
                        self.end_burst(burst)
                        burst = None
                    if burst is None:
                        burst = {"src": src, "start": ev.t, "last": ev.t, "n": 0, "seq": 0, "settled": None,
                                 "build": self.build(), "label": self.label + 1, "buckets": collections.Counter()}
                    burst["n"] += 1
                    burst["last"] = ev.t
                    burst["seq"] = self.seq
                    burst["settled"] = None
                    burst["buckets"][int((ev.t - burst["start"]) * 10)] += 1
                    dirty = True
                elif ev.kind == "key" or ev.kind == "mouse" and ev.v in ("press", "release"):
                    if ev.kind == "key":
                        app.log.input(ev.v)
                    if self.on(ev):
                        dirty = True
            if paced and awaiting and time.monotonic() - fence_t > fence_timeout:
                awaiting = False
                if have_dsr:
                    self.fence_timeouts += 1
            if burst and time.monotonic() - burst["last"] > 0.3 and (burst["settled"] is not None or not have_dsr
                                                                   or time.monotonic() - burst["last"] > 3):
                self.end_burst(burst)
                burst = None
        if burst:
            self.end_burst(burst)
        for b, xs in self.fence.items():
            if xs:
                xs = sorted(xs)
                self.app.log.line("frames answered, build %s: %d, p50 %.1f ms, p99 %.1f ms" % (
                    b, len(xs), 1000 * xs[len(xs) // 2], 1000 * xs[min(len(xs) - 1, int(len(xs) * 0.99))]))
        if self.fence_timeouts:
            self.app.log.line("fence timeouts (no answer within 500 ms): %d" % self.fence_timeouts)

    def end_burst(self, b):
        dur = b["last"] - b["start"]
        rate = (b["n"] - 1) / dur if dur > 0 else 0
        peak = max(b["buckets"].values()) * 10 if b["buckets"] else 0
        settle = ("%.0f ms" % (1000 * (b["settled"] - b["last"]))) if b["settled"] is not None else "unknown"
        self.app.log.line("burst: %s, build %d (%s), %d events over %.2fs, %.0f/s (peak %d/s), "
                          "screen caught up %s after the last one" % (
                              b["src"], b["label"], b["build"], b["n"], dur, rate, peak, settle))


# ------------------------------------------------------------------ scene 2: ? and Ctrl-P

TASKS2 = [
    "Put your changes aside: you need a clean tree for a minute.",
    "What does the x key do here?",
    "Your rebase stopped. How do you go on?",
    "You want merge on another key. What would you write in your config file? Write it down when you know.",
]
TARGET2 = ["journey 6: 3 steps + 1 decision (find by name, run)", "journey 5: 2 steps + 1 decision",
           "journey 7: 1 step + 1 decision, then continue", "journey 8: read the config name from the list"]


class HelpScene(Scene):
    num, title = 2, "finding keys"

    def __init__(self, app):
        super().__init__(app)
        self.rows = [list(r) for r in BILLING]
        self.cur = 1
        self.finder = None
        self.overlay = None   # "stash" | "stash_prompt" | "answer"
        self.text = ""
        self.t = 0
        self.rebase = False
        self.start_task()

    def start_task(self):
        self.t0 = time.monotonic()
        self.keys = 0
        self.typed = 0
        self.path = []
        if self.t < len(TASKS2):
            self.app.log.line("task %d: %s" % (self.t + 1, TASKS2[self.t]))
        if self.t == 2:
            self.rebase = True

    def finish(self, how):
        self.app.log.line("task %d done: %d keys + %d typed chars, %.1fs, %s · path: %s · target %s" % (
            self.t + 1, self.keys, self.typed, time.monotonic() - self.t0, how, " ".join(self.path[-40:]),
            TARGET2[self.t]))
        self.say("✓ Done. Next task.", 2.5)
        self.t += 1
        if self.t >= len(TASKS2):
            self.q = ("eye", "When the list opened, where did your eye land first?",
                      ["the keys on the left", "the action names", "the grey names on the right"])
            self.app.log.line("question: where did your eye land first")
        else:
            self.start_task()

    def task(self):
        if self.done or self.q:
            return "That's all four."
        return TASKS2[self.t]

    def text_mode(self):
        return self.finder is not None or self.overlay in ("stash_prompt", "answer")

    def bar(self):
        if self.q or self.done:
            return ""
        if self.overlay == "answer":
            return "type your answer · ⏎ done · esc cancel"
        extra = " · = write your answer" if self.t == 3 else ""
        return "(the app's keys are yours to find) · q quit%s" % extra

    def groups(self, palette):
        g = []
        if self.rebase:
            g.append(("IN PROGRESS", IN_PROGRESS))
        # while an operation is live, c and a belong to it (keymap §2.3, §2.4): not listed twice
        here = [it for it in HERE_GRAPH if not (self.rebase and it[0] in ("c", "a"))]
        g += [("HERE", here), ("EVERYWHERE", EVERYWHERE)]
        if palette:
            g.insert(1 if self.rebase else 0, ("ACTIONS", [("S s", "stash: save your changes now", "stash.save",
                                                             "aside shelve park put")]))
            g += PALETTE_EXTRA
        return g

    def body(self, cv, y0, h):
        pal = self.pal
        cv.put(1, y0, "◆ billing-api", pal["accent"], at="b")
        cv.put(16, y0, "~/code/billing-api · main", pal["muted"])
        y = y0 + 1
        if self.rebase:
            cv.fill(0, y, cv.w, 1, pal["open_t"])
            cv.put(1, y, "REBASE paused · 3 of 5 · stopped to edit 9f2c1ab Add invoice totals · nothing conflicts",
                   pal["warning"], pal["open_t"], "b")
            y += 1
        draw_graph_rows(cv, pal, self.rows, self.cur, 0, y + 1, cv.w, h - (y - y0) - 1)
        if self.finder:
            self.finder.draw(cv, pal, 0, y0 + 2, cv.w, h - 3)
        elif self.overlay == "stash":
            cv.box(cv.w // 2 - 30, y0 + 4, 60, 6, pal["border"], pal["raised"], "stash", "esc close")
            cv.put(cv.w // 2 - 27, y0 + 6, "no stashes yet", pal["muted"], pal["raised"])
            cv.put(cv.w // 2 - 27, y0 + 8, "s save a new stash · ⏎ apply · p pop · esc close", pal["muted"],
                   pal["raised"])
        elif self.overlay == "stash_prompt":
            cv.box(cv.w // 2 - 30, y0 + 4, 60, 5, pal["border"], pal["raised"], "save a stash", "esc cancel")
            cv.put(cv.w // 2 - 27, y0 + 5, "message › " + self.text + "▏", pal["text"], pal["raised"])
            cv.put(cv.w // 2 - 27, y0 + 7, "⇥ staged only / everything · ⏎ save", pal["muted"], pal["raised"])
        elif self.overlay == "answer":
            cv.box(cv.w // 2 - 40, y0 + 4, 80, 5, pal["warning"], pal["raised"], "your answer (prototype)")
            cv.put(cv.w // 2 - 37, y0 + 5, "› " + self.text + "▏", pal["text"], pal["raised"])
            cv.put(cv.w // 2 - 37, y0 + 7, "the line(s) you would write in your config file", pal["muted"],
                   pal["raised"])

    def run_item(self, it):
        key, label, cfg, _ = it
        self.app.log.line("ran from the list: %r (%s)" % (label, cfg or "no config name"))
        if cfg == "stash.save":
            self.overlay, self.text = "stash_prompt", ""
        elif cfg == "graph.stash":
            self.overlay = "stash"
        elif cfg == "operation.continue":
            self.continue_rebase()
        elif cfg in ("operation.skip", "operation.abort"):
            self.say("(prototype: that would %s; this task wants the rebase to go on)" % label, 5)
        elif cfg == "app.palette":
            self.finder = Finder("palette", self.groups(True))
        elif cfg == "app.help":
            self.finder = Finder("help", self.groups(False))
        else:
            self.say("(prototype: “%s” isn't acted out here)" % label)

    def continue_rebase(self):
        if not self.rebase:
            self.say("(prototype: nothing to continue)")
            return
        self.rebase = False
        self.say("✓ rebase done · 5 commits on main", 4)
        if self.t == 2:
            self.finish("continued")

    def on(self, ev):
        if ev.kind == "paste" and self.text_mode():
            if self.finder:
                self.finder.query += ev.v
            else:
                self.text += ev.v
            self.typed += len(ev.v)
            return
        if ev.kind == "mouse":
            if ev.v == "press":
                self.keys += 1
                self.path.append("click")
            return
        k = ev.v
        if self.finder:
            if len(k) == 1:
                self.typed += 1
            else:
                self.keys += 1
            self.path.append(k if len(k) > 1 else repr(k))
            res = self.finder.key(k)
            if self.t == 1 and self.finder and self.finder.query.strip() == "x":
                self.app.log.line("finder query 'x' → top row: %s" % (self.finder.rows()[0][1][1]
                                                                       if self.finder.rows() else "none"))
                self.finish("typed x in the %s" % self.finder.kind)
            if res:
                f = self.finder
                self.app.log.line("%s closed, query %r, %s" % (f.kind, f.query, res[0]))
                self.finder = None
                if res[0] == "run":
                    self.run_item(res[1])
            return
        self.keys += 1
        self.path.append(k)
        if self.overlay in ("stash_prompt", "answer"):
            if k == "esc":
                self.overlay = None
            elif k == "bs":
                self.text = self.text[:-1]
            elif k == "enter":
                if self.overlay == "stash_prompt":
                    self.overlay = None
                    self.rows[0] = ["◌", [], "Uncommitted changes · none", "", "", ""]
                    self.say("✓ changes put aside: stash@{0} · u brings them back", 5)
                    if self.t == 0:
                        self.finish("stash saved")
                else:
                    self.app.log.line("ANSWER (config line): %r · uses graph.merge: %s" % (
                        self.text, "yes" if "graph.merge" in self.text else "no"))
                    self.overlay = None
                    if self.t == 3:
                        self.finish("wrote %r" % self.text)
            elif len(k) == 1 and k.isprintable():
                self.text += k
                self.typed += 1
                self.keys -= 1
            return
        if self.overlay == "stash":
            if k == "esc":
                self.overlay = None
            elif k == "s":
                self.overlay, self.text = "stash_prompt", ""
            else:
                self.say("(prototype: only saving is acted out here)")
            return
        if k == "?":
            self.finder = Finder("help", self.groups(False))
        elif k in ("ctrl-p", ":"):
            self.finder = Finder("palette", self.groups(True))
        elif k in ("j", "down"):
            self.cur = min(len(self.rows) - 1, self.cur + 1)
        elif k in ("k", "up"):
            self.cur = max(0, self.cur - 1)
        elif k == "S":
            self.overlay = "stash"
        elif k == "=" and self.t == 3:
            self.overlay, self.text = "answer", ""
        elif self.rebase and k == "c":
            self.continue_rebase()
        elif self.rebase and k in ("s", "a"):
            self.say("(prototype: that would %s; this task wants the rebase to go on)" % (
                "skip this commit" if k == "s" else "abort the rebase"), 5)
        elif k == "x":
            self.say("compare: %s marked · pick another commit and press x" % self.rows[self.cur][5])
            if self.t == 1:
                self.finish("pressed x itself")
        elif k == "u" and self.rows[0][2].endswith("none"):
            self.rows[0] = list(BILLING[0])
            self.say("✓ undone: your changes are back · U redoes")
        elif k == "esc":
            pass
        else:
            item = next((it for it in HERE_GRAPH + EVERYWHERE if it[0] == k), None)
            self.say("(prototype: %s isn't acted out here)" % (repr(item[1]) if item else repr(k)))

    def answered(self, n):
        pass


# ------------------------------------------------------------------ scene 3: first run and tour

TOUR = [
    ("stage a change", "⏎ on the top row opens your changes · space stages the change under the cursor", "stage"),
    ("commit it", "c commits what's staged · type a message, ⏎", "commit"),
    ("undo that commit", "u takes back the last thing you did — nothing is lost", "undo"),
    ("merge by dragging", "drag the feature badge onto main with the mouse · y goes ahead", "drag"),
    ("switch branch", "C opens the branch picker · type a name, ⏎", "switch"),
    ("pull", "p brings in what's new from the remote", "pull"),
    ("push", "P sends your branch up", "push"),
    ("find any key", "? lists the keys for wherever you are", "help"),
]
TASKS3 = [
    "Open the project you worked on yesterday.",
    "Get a copy of this project onto your machine: https://github.com/acme/billing-api.git",
    "You've never used this app before. Find somewhere safe to practise.",
    "Follow the practice card.",
    "Enough practice. Go back to your own work now.",
]


class TourScene(Scene):
    num, title = 3, "first run and the practice tour"

    def __init__(self, app):
        super().__init__(app)
        self.place = "empty"      # empty | clone | cloning | repo
        self.repo = None          # payments | billing | practice
        self.recent_cur = 0
        self.text = ""
        self.clone_err = ""
        self.clone_t = 0.0
        self.first_line = True
        self.finder = None
        self.cur = 0
        self.saved_cur = {}
        self.t = 0
        self.back_at = None
        self.hits = []
        self.drag = None
        self.preview = False
        self.tour_i = 0
        self.ticks = set()
        self.p = None
        self.start_task()

    # -------- tasks
    def start_task(self):
        self.t0 = time.monotonic()
        self.keys = 0
        self.path = []
        if self.t < len(TASKS3):
            self.app.log.line("task %d: %s" % (self.t + 1, TASKS3[self.t]))

    def finish(self, how):
        self.app.log.line("task %d done: %d inputs, %.1fs, %s · path: %s" % (
            self.t + 1, self.keys, time.monotonic() - self.t0, how, " ".join(self.path[-40:])))
        self.t += 1
        if self.t < len(TASKS3):
            self.start_task()

    def task(self):
        if self.q or self.done:
            return "Where are you now?"
        return TASKS3[min(self.t, len(TASKS3) - 1)]

    def text_mode(self):
        return self.finder is not None or self.place == "clone" or (self.p and self.p.get("prompt") is not None)

    def bar(self):
        if self.q or self.done:
            return ""
        if self.place == "empty":
            return "o open… · i init · c clone… · t practice tour · ⏎ open the selected · q quit"
        if self.place in ("clone", "cloning"):
            return "⏎ clone · esc cancel"
        return "(the app's keys are yours to find; the card names some) · q quit"

    # -------- practice repo model
    def practice_reset(self):
        self.p = {"hunks": [0, 0], "commit": None, "merged": False, "changes": False, "hcur": 0,
                  "prompt": None, "undone": False}
        self.cur = 0

    def practice_rows(self):
        p = self.p
        staged = p["hunks"].count(1)
        left = p["hunks"].count(0)
        rows = []
        if left or staged:
            rows.append(("◌", [], "Uncommitted changes · greet.rs (%d staged, %d not)" % (staged, left), "", "", ""))
        else:
            rows.append(("◌", [], "Uncommitted changes · none", "", "", ""))
        if p["merged"]:
            rows.append(("●─╮", ["main"], "Merge feature into main", "you", "just now", "7d1e2f0"))
        if p["commit"]:
            rows.append(("●" if not p["merged"] else "● │", [] if p["merged"] else ["main"], p["commit"], "you",
                         "just now", "a4c7e19"))
        mainrefs = [] if (p["commit"] or p["merged"]) else ["main"]
        rows += [
            ("● │" if p["merged"] else "●", mainrefs, "Say hello", "Practice", "1 min ago", "3b9d0c2"),
            ("│ ●", ["feature"], "Shout the greeting", "Practice", "1 min ago", "e5f6a71"),
            ("●─╯", [], "Add README", "Practice", "2 min ago", "8c2d4b3"),
            ("●", [], "Initial commit", "Practice", "2 min ago", "1f0a9e5"),
        ]
        return rows

    def rows(self):
        if self.repo == "practice":
            return self.practice_rows()
        return {"payments": PAYMENTS, "billing": BILLING[1:]}[self.repo]

    def tick_tour(self, what, how=""):
        name = TOUR[self.tour_i][2]
        if what != name:
            self.app.log.line("tour: did %s while the card asked for %s" % (what, name))
            return
        self.ticks.add(name)
        self.app.log.line("tour step %d (%s) ticked %s· %.1fs since the card showed it" % (
            self.tour_i + 1, name, how, time.monotonic() - self.card_t))
        self.tour_i += 1
        self.card_t = time.monotonic()
        if self.tour_i == 4:   # three tasks and a drag done: ask him to leave mid-task
            self.finish("four practice steps done")

    # -------- drawing
    def body(self, cv, y0, h):
        pal = self.pal
        self.hits = []
        if self.place in ("empty", "clone", "cloning"):
            self.draw_empty(cv, y0, h)
            return
        if self.repo == "practice":
            cv.put(1, y0, "◇ practice · /tmp/graft-practice-7f3a", pal["warning"], at="b")
            cv.put(40, y0, "a throwaway repo, not yours", pal["muted"])
            card = TOUR[self.tour_i] if self.tour_i < len(TOUR) else TOUR[-1]
            ticks = " ".join(("✓ " if t[2] in self.ticks else "") + t[2] for t in TOUR[:max(3, self.tour_i + 1)])
            title = "practice %d of %d · %s" % (self.tour_i + 1, len(TOUR), card[0])
            cv.box(0, y0 + 1, cv.w, 3, pal["warning"], pal["bg"], title, "esc leave")
            cv.put(2, y0 + 2, cut(card[1], cv.w - 4 - (len(ticks) + 2 if cv.w >= 100 else 0)), pal["text"])
            if cv.w >= 100:
                cv.put(cv.w - 2 - len(ticks), y0 + 2, ticks, pal["added"])
            gy = y0 + 4
        else:
            name = {"payments": "payments-service", "billing": "billing-api"}[self.repo]
            cv.put(1, y0, "◆ " + name, pal["accent"], at="b")
            cv.put(4 + len(name), y0, "~/code/%s · main" % name, pal["muted"])
            gy = y0 + 1
        rows = self.rows()
        gw = cv.w if not (self.repo == "practice" and self.p["changes"]) else cv.w // 2
        draw_graph_rows(cv, pal, rows, self.cur if not (self.p and self.p["changes"] and self.repo == "practice")
                        else -1, 0, gy + 1, gw, h - (gy - y0) - 1, self.hits)
        if self.repo == "practice" and self.p["changes"]:
            self.draw_changes(cv, gw, gy + 1, cv.w - gw, h - (gy - y0) - 1)
        if self.drag and self.drag.get("moved"):
            cv.put(self.drag["x"] + 1, self.drag["y"], "⇢ " + self.drag["name"], pal["accent"], pal["raised"], "b")
        if self.preview:
            bx, bw = max(0, cv.w // 2 - 32), 64
            cv.box(bx, y0 + 6, bw, 6, pal["accent"], pal["raised"], "Merge feature into main", "preview")
            cv.put(bx + 3, y0 + 7, "main gains 1 commit: Shout the greeting", pal["text"], pal["raised"])
            cv.put(bx + 3, y0 + 8, "no conflicts · u takes it back", pal["muted"], pal["raised"])
            cv.put(bx + 3, y0 + 10, "y go ahead · esc cancel", pal["accent"], pal["raised"], "b")
        if self.p and self.p.get("prompt") is not None:
            bx = max(0, cv.w // 2 - 32)
            cv.box(bx, y0 + 6, 64, 4, pal["border"], pal["raised"], "commit", "esc cancel")
            cv.put(bx + 3, y0 + 7, "message › " + self.p["prompt"] + "▏", pal["text"], pal["raised"])
            cv.put(bx + 3, y0 + 8, "⏎ commit what's staged", pal["muted"], pal["raised"])
        if self.finder:
            self.finder.draw(cv, pal, 0, y0 + 2, cv.w, h - 3)

    def draw_changes(self, cv, x, y, w, h):
        pal = self.pal
        cv.put(x + 1, y, "greet.rs", pal["text"], at="b")
        lines = [
            (0, "@@ fn greet @@", None), (0, "-    println!(\"hi\");", "removed"),
            (0, "+    println!(\"Hello, {name}!\");", "added"),
            (1, "@@ fn farewell @@", None), (1, "+fn farewell() { println!(\"bye\"); }", "added"),
        ]
        for i, (hk, text, tok) in enumerate(lines):
            yy = y + 2 + i
            head = tok is None
            mark = ""
            if head:
                mark = {0: "  ", 1: "✓ staged ", 2: "committed "}[self.p["hunks"][hk]]
            sel = head and hk == self.p["hcur"]
            cv.put(x + 1, yy, "▸" if sel else " ", pal["accent"], at="b")
            if head:
                cv.put(x + 3, yy, mark, pal["added"], at="b")
            cv.put(x + 3 + (len(mark) if head else 0), yy, text, pal[tok] if tok else pal["info"])
            if sel:
                cv.rbg(yy, pal["sel"], x, w)
        cv.put(x + 1, y + 8, "space stage / unstage · j k between changes · esc back", pal["muted"])

    def draw_empty(self, cv, y0, h):
        pal = self.pal
        cv.put(2, y0 + 1, "graft · no repository here", pal["text"], at="b")
        cv.put(2, y0 + 2, "~/code", pal["muted"])
        x = 2
        for k, lab in (("o", "Open…"), ("i", "Init"), ("c", "Clone…"), ("t", "practice tour")):
            x = cv.put(x, y0 + 4, k, pal["accent"], at="b") + 2
            x = cv.put(x, y0 + 4, lab, pal["text"]) + 5
        cv.put(2, y0 + 6, "RECENT", pal["muted"], at="b")
        recent = [("payments-service", "~/code/payments-service", "yesterday"),
                  ("web-dashboard", "~/code/web-dashboard", "3 days ago"),
                  ("dotfiles", "~/dotfiles", "last week")]
        for i, (name, path, when) in enumerate(recent):
            yy = y0 + 7 + i
            sel = i == self.recent_cur and self.place == "empty"
            cv.put(2, yy, "▸" if sel else " ", pal["accent"], at="b")
            cv.put(4, yy, name.ljust(20), pal["text"])
            cv.put(24, yy, path.ljust(28), pal["muted"])
            cv.put(52, yy, when, pal["muted"])
            if sel:
                cv.rbg(yy, pal["sel"], 2, 64)
                if not COLOUR[0]:
                    for xx in range(2, 66):
                        fg, bg, at = cv.st[yy][xx]
                        cv.st[yy][xx] = (fg, bg, at + "r")
            self.hits.append((yy, 2, 66, "recent:%d" % i))
        if self.place in ("clone", "cloning"):
            yy = y0 + 12
            cv.box(2, yy, min(cv.w - 4, 90), 5, pal["border"], pal["raised"], "clone")
            cv.put(5, yy + 1, "clone › " + self.text + ("▏" if self.place == "clone" else ""), pal["text"],
                   pal["raised"])
            name = self.text.rstrip("/").split("/")[-1].replace(".git", "") or "…"
            cv.put(5, yy + 2, "→ ~/code/" + name, pal["muted"], pal["raised"])
            if self.place == "cloning":
                pct = min(100, int((time.monotonic() - self.clone_t) / 1.6 * 100))
                cv.put(5, yy + 3, "receiving objects %d%%" % pct, pal["info"], pal["raised"])
            elif self.clone_err:
                cv.put(5, yy + 3, self.clone_err + " · ⏎ retry · esc cancel", pal["removed"], pal["raised"])

    def status(self):
        if self.place == "repo" and self.repo != "practice" and self.first_line:
            return "first time? ? keys here · ⌃P anything · ⌃P \"tour\" to practise"
        return ""

    def frame(self, w, h):
        cv = super().frame(w, h)
        if self.place == "repo" and self.repo != "practice" and self.first_line and not (
                time.monotonic() < self.flash_until):
            for xx in range(1, w):
                fg, bg, at = cv.st[h - 2][xx]
                cv.st[h - 2][xx] = (self.pal["muted"], bg, at)
        return cv

    # -------- input
    def open_repo(self, repo, how):
        self.place, self.repo, self.cur = "repo", repo, self.saved_cur.get(repo, 0)
        self.app.log.line("opened %s (%s)" % (repo, how))

    def start_tour(self, how):
        self.app.log.line("tour started (%s)" % how)
        self.came_from = (self.place, self.repo)
        if self.repo:
            self.saved_cur[self.repo] = self.cur
        self.place, self.repo = "repo", "practice"
        self.practice_reset()
        self.tour_i = 0
        self.ticks = set()
        self.card_t = time.monotonic()
        if self.t == 2:
            self.finish(how)

    def leave_tour(self):
        self.app.log.line("tour left with esc at step %d (%s)" % (self.tour_i + 1, TOUR[min(self.tour_i, 7)][0]))
        place, repo = self.came_from
        self.place, self.repo = place, repo
        self.p = None
        if repo:
            self.cur = self.saved_cur.get(repo, 0)
        if self.t == 4:
            self.finish("left the tour")
            self.q = ("where", "Whose repository is on screen now?",
                      ["my own work (billing-api)", "still the practice one", "not sure"])
            self.app.log.line("question: whose repository is on screen now")

    def tick(self, now):
        if self.place == "cloning":
            if now - self.clone_t >= 1.6:
                self.open_repo("billing", "clone")
                self.say("✓ cloned into ~/code/billing-api", 2)
                if self.t == 1:
                    self.finish("cloned %r" % self.text)
                return None
            return 0.1
        if self.back_at is not None:
            if now >= self.back_at:
                self.back_at = None
                self.place, self.repo = "empty", None
                self.say("(prototype: you're back outside any repo for the next task)", 5)
                return None
            return self.back_at - now
        return None

    def on(self, ev):
        if ev.kind == "mouse":
            self.mouse(ev)
            return
        if ev.kind == "paste":
            if self.place == "clone":
                self.text += ev.v.strip()
            elif self.finder:
                self.finder.query += ev.v
            return
        k = ev.v
        self.keys += 1
        self.path.append(k if len(k) > 1 else repr(k))
        if self.finder:
            res = self.finder.key(k)
            if res:
                f = self.finder
                self.app.log.line("%s closed, query %r, %s" % (f.kind, f.query, res[0]))
                self.finder = None
                if res[0] == "run":
                    lab, cfg = res[1][1], res[1][2]
                    if cfg == "app.tour":
                        self.start_tour("from the %s, query %r" % (f.kind, f.query))
                    elif cfg == "app.help":
                        self.open_finder("help")
                    else:
                        self.say("(prototype: “%s” isn't acted out here)" % lab)
            return
        if self.place == "empty":
            if k in ("j", "down"):
                self.recent_cur = min(2, self.recent_cur + 1)
            elif k in ("k", "up"):
                self.recent_cur = max(0, self.recent_cur - 1)
            elif k == "enter":
                self.pick_recent(self.recent_cur, "⏎")
            elif k == "c":
                self.place, self.text, self.clone_err = "clone", "", ""
            elif k == "t":
                self.start_tour("t on the empty screen")
            elif k in ("o", "i"):
                self.say("(prototype: %s isn't acted out here)" % ("a folder prompt" if k == "o" else "init"))
            return
        if self.place == "clone":
            if k == "esc":
                self.place = "empty"
            elif k == "bs":
                self.text = self.text[:-1]
            elif k == "ctrl-u":
                self.text = ""
            elif k == "enter":
                if "://" in self.text or self.text.startswith("git@"):
                    self.place, self.clone_t, self.clone_err = "cloning", time.monotonic(), ""
                    self.app.log.line("clone started: %r" % self.text)
                else:
                    self.clone_err = "clone: that isn't a URL git can use"
                    self.app.log.line("clone refused: %r" % self.text)
            elif len(k) == 1 and k.isprintable():
                self.text += k
            return
        if self.place != "repo":
            return
        if self.repo == "practice":
            self.practice_key(k)
            return
        rows = self.rows()
        if k == "?":
            self.open_finder("help")
        elif k in ("ctrl-p", ":"):
            self.open_finder("palette")
        elif k in ("j", "down"):
            self.cur = min(len(rows) - 1, self.cur + 1)
        elif k in ("k", "up"):
            self.cur = max(0, self.cur - 1)
        elif k == "esc":
            pass
        else:
            self.say("(prototype: %r isn't acted out in this repo)" % k)

    def open_finder(self, kind):
        self.first_line = False
        g = []
        tour = [("—", "practice tour", "app.tour", "tutorial learn practise practice help start")]
        if kind == "palette":
            g.append(("ACTIONS", tour + [("—", "Theme: dark", "app.theme", ""), ("—", "Theme: light", "app.theme", ""),
                                         ("—", "Theme: high contrast", "app.theme", "")]))
        g += [("HERE", HERE_GRAPH), ("EVERYWHERE", EVERYWHERE + (tour if kind == "help" else []))]
        if kind == "palette":
            g += PALETTE_EXTRA
        self.finder = Finder(kind, g)
        self.app.log.line("opened the %s" % kind)

    def pick_recent(self, i, how):
        name = ["payments", "web-dashboard", "dotfiles"][i]
        if name != "payments":
            self.app.log.line("opened recent row %d (%s), not yesterday's" % (i, name))
            self.say("(prototype: only payments-service is acted out)")
            return
        self.open_repo("payments", how)
        if self.t == 0:
            self.finish("opened payments-service by %s" % how)
            self.back_at = time.monotonic() + 2.5

    def practice_key(self, k):
        p = self.p
        if p["prompt"] is not None:
            if k == "esc":
                p["prompt"] = None
            elif k == "bs":
                p["prompt"] = p["prompt"][:-1]
            elif k == "enter":
                p["commit"] = p["prompt"].strip() or "Say hello properly"
                p["prompt"] = None
                p["hunks"] = [2 if h == 1 else h for h in p["hunks"]]
                self.app.log.line("practice: committed %r" % p["commit"])
                self.say("✓ committed “%s” · u takes it back" % p["commit"])
                self.tick_tour("commit")
            elif len(k) == 1 and k.isprintable():
                p["prompt"] += k
            return
        if self.preview:
            if k == "y":
                self.preview = False
                p["merged"] = True
                self.say("✓ merged feature into main · u takes it back")
                self.tick_tour("drag", "(%s) " % self.preview_how)
            elif k == "esc":
                self.preview = False
            return
        if k == "esc":
            if p["changes"]:
                p["changes"] = False
                return
            self.leave_tour()
            return
        if p["changes"]:
            if k in ("j", "down"):
                p["hcur"] = min(1, p["hcur"] + 1)
            elif k in ("k", "up"):
                p["hcur"] = max(0, p["hcur"] - 1)
            elif k in (" ", "s"):
                h = p["hunks"][p["hcur"]]
                if h == 2:
                    self.say("that change is already committed")
                    return
                p["hunks"][p["hcur"]] = 1 - h
                if h == 0:
                    self.say("✓ staged · c commits it")
                    self.tick_tour("stage")
            elif k == "c":
                self.commit_key()
            elif k == "u":
                self.undo()
            elif k == "tab":
                p["changes"] = False
            else:
                self.say("(prototype: %r isn't acted out in the practice)" % k)
            return
        rows = self.practice_rows()
        if k in ("j", "down"):
            self.cur = min(len(rows) - 1, self.cur + 1)
        elif k in ("k", "up"):
            self.cur = max(0, self.cur - 1)
        elif k == "enter" and self.cur == 0:
            p["changes"] = True
        elif k == "c":
            self.commit_key()
        elif k == "u":
            self.undo()
        elif k == "m":
            if rows[self.cur][1] == ["feature"]:
                self.preview, self.preview_how = True, "m on the feature row"
            else:
                self.say("(prototype: put the cursor on feature first)")
        elif k == "?":
            self.open_finder("help")
        elif k in ("ctrl-p", ":"):
            self.open_finder("palette")
        else:
            self.say("(prototype: %r isn't acted out in the practice)" % k)

    def commit_key(self):
        p = self.p
        if 1 not in p["hunks"]:
            self.say("nothing staged — space stages the change under the cursor")
            return
        p["prompt"] = ""
        p["changes"] = False

    def undo(self):
        p = self.p
        if p["merged"]:
            p["merged"] = False
            self.say("✓ undone: the merge · U redoes")
        elif p["commit"]:
            self.say("✓ undone: commit “%s” · your change is back, staged · U redoes" % p["commit"])
            p["commit"] = None
            p["hunks"] = [1 if h == 2 else h for h in p["hunks"]]
            self.tick_tour("undo")
        else:
            self.say("nothing to undo")

    def mouse(self, ev):
        if self.place == "empty" and ev.v == "press":
            for (y, x0, x1, name) in self.hits:
                if ev.y == y and x0 <= ev.x < x1 and name.startswith("recent:"):
                    self.keys += 1
                    self.path.append("click")
                    self.recent_cur = int(name.split(":")[1])
                    self.pick_recent(self.recent_cur, "click")
            return
        if self.place != "repo" or self.repo != "practice" or self.preview:
            return
        if ev.v == "press":
            self.keys += 1
            self.path.append("press")
            for (y, x0, x1, name) in self.hits:
                if ev.y == y and x0 <= ev.x < x1:
                    self.drag = {"name": name, "x": ev.x, "y": ev.y, "moved": False}
                    self.app.log.line("mouse down on the %s badge" % name)
                    return
            row = ev.y - 8
            if 0 <= row < len(self.practice_rows()):
                self.cur = row
        elif ev.v == "drag" and self.drag:
            self.drag.update(x=ev.x, y=ev.y, moved=True)
        elif ev.v == "release" and self.drag:
            d, self.drag = self.drag, None
            row = ev.y - 8
            rows = self.practice_rows()
            if d["moved"] and 0 <= row < len(rows):
                target = rows[row]
                self.app.log.line("dropped %s on row %r" % (d["name"], target[2]))
                if d["name"] == "feature" and ("main" in target[1] or target[0].startswith("●")) \
                        and target[1] != ["feature"]:
                    self.preview, self.preview_how = True, "drag"
                else:
                    self.say("(prototype: drop feature onto main)")


# ------------------------------------------------------------------ scene 4: themes

class ThemeScene(Scene):
    num, title = 4, "themes, by eye"
    VIEWS = [("dark_a", "dark A · ground darker"), ("dark_b", "dark B · text brighter"),
             ("light", "light"), ("hc", "high contrast"), ("dark", "today's dark, for reference")]

    def __init__(self, app):
        super().__init__(app)
        self.view = 0       # 0: A|B, 1: light|hc, 2: today's dark alone
        self.final = None   # free-text answer to the closing question

    def task(self):
        if self.final is not None:
            return "Last one, in your own words."
        if self.q or self.done:
            return "Which dark reads better?"
        return ["Two dark versions. Look at the highlighted row, the coloured line running through it, the conflict "
                "colours and the grey text. Which do you read more easily?",
                "Light and high contrast, for reference. Anything you can't read?",
                "Today's dark, for reference."][self.view]

    def text_mode(self):
        return self.final is not None

    def bar(self):
        if self.final is not None:
            return "type your answer · ⏎ done"
        if self.q or self.done:
            return ""
        return "→ / space next · ← back · q quit" + ("" if COLOUR[0] else "   (colour is off: --no-colour)")

    def panel(self, cv, x, y, w, h, key, title):
        pal = PAL[key]
        if COLOUR[0]:
            cv.fill(x, y, w, h, pal["bg"], pal["text"])
        cv.put(x + 1, y, title, pal["text"], at="b")
        rows = BILLING[:7]
        cv.fill(x, y + 1, w, 1, pal["surface"])
        cv.put(x + 1, y + 1, "◆ billing-api · main", pal["accent"], pal["surface"], "b")
        old = (cv.fg, cv.bg)
        cv.fg, cv.bg = pal["text"], pal["bg"]
        draw_graph_rows(cv, pal, rows, 3, x, y + 2, w, 7)
        cv.fg, cv.bg = old
        yy = y + 10
        for lab, tok, tint, text in (("ours  ", "ours", "ours_t", "let total = round(sum);"),
                                     ("theirs", "theirs", "theirs_t", "let total = sum.round(2);"),
                                     ("open  ", "open", "open_t", "<<<<<<< choose a side")):
            cv.fill(x + 1, yy, w - 2, 1, pal[tint])
            cv.put(x + 2, yy, lab, pal[tok], pal[tint], "b")
            cv.put(x + 10, yy, text, pal["text"], pal[tint])
            yy += 1
        cv.put(x + 2, yy, "+ fn farewell() { … }", pal["added"])
        cv.put(x + 2, yy + 1, "- println!(\"hi\");", pal["removed"])
        cv.put(x + 2, yy + 2, "3 files · 2 hours ago · ? keys here", pal["muted"])
        nums = [("cursor row vs ground", contrast(pal["sel"], pal["bg"]), 3.0),
                ("text on cursor row", contrast(pal["text"], pal["sel"]), 4.5),
                ("blue lane on cursor row", contrast(LANES[0], pal["sel"]), 3.0),
                ("grey text on ground", contrast(pal["muted"], pal["bg"]), 4.5)]
        for i, (lab, c, floor) in enumerate(nums):
            ok = c >= floor
            cv.put(x + 2, yy + 4 + i, "%-24s %5.2f:1 %s floor %.1f" % (lab, c, "≥" if ok else "<", floor),
                   pal["muted"] if ok else pal["warning"])

    def body(self, cv, y0, h):
        if self.final is not None:
            pal = self.pal
            cv.put(2, y0 + 2, "Where did two different things still look the same?", pal["text"], at="b")
            cv.put(2, y0 + 3, "(any scene; a few words are enough)", pal["muted"])
            cv.put(2, y0 + 5, "› " + self.final + "▏", pal["text"])
            return
        pairs = [[0, 1], [2, 3], [4]][self.view]
        if len(pairs) == 2 and cv.w >= 150:
            pw = (cv.w - 3) // 2
            for i, v in enumerate(pairs):
                self.panel(cv, 1 + i * (pw + 1), y0, pw, h, *self.VIEWS[v])
        else:
            for i, v in enumerate(pairs):
                ph = h // len(pairs)
                self.panel(cv, 1, y0 + i * ph, cv.w - 2, ph, *self.VIEWS[v])

    def on(self, ev):
        if ev.kind == "paste" and self.final is not None:
            self.final += ev.v
            return
        if ev.kind != "key":
            return
        k = ev.v
        if self.final is not None:
            if k == "enter":
                self.app.log.line("FINAL ANSWER (two things that looked the same): %r" % self.final)
                self.final = None
                self.done = True
            elif k == "bs":
                self.final = self.final[:-1]
            elif len(k) == 1 and k.isprintable():
                self.final += k
            return
        if k in ("right", " ", "l"):
            if self.view < 2:
                self.view += 1
                self.app.log.line("theme view %d" % (self.view + 1))
            else:
                self.q = ("dark", "Which dark reads better?", ["dark A (ground darker)", "dark B (text brighter)",
                                                               "no difference"])
                self.view = 0
                self.app.log.line("question: which dark reads better")
        elif k in ("left", "h"):
            self.view = max(0, self.view - 1)

    def answered(self, n):
        self.done = False
        self.final = ""


# ------------------------------------------------------------------ app

SCENES = [None, ScrollScene, HelpScene, TourScene, ThemeScene]


class App:
    def __init__(self, args, term):
        self.args, self.term = args, term
        self.log = Log(args.picks)
        self.inbox = collections.deque()
        self.done = False
        self.probe = {}
        self.pal = PAL["dark"]
        self.scene = None
        self.last_canvas = None
        self.captures = 0
        self.resized = False

    def size(self):
        try:
            s = os.get_terminal_size(self.term.out)
            return max(40, s.columns), max(16, s.lines)
        except OSError:
            return 187, 50

    def next_events(self, timeout):
        if self.inbox:
            out = list(self.inbox)
            self.inbox.clear()
            return out
        return self.term.read(timeout)

    def wait_for(self, kinds, timeout):
        """Reads until an event of one of `kinds` arrives; queues everything else. Returns it or None."""
        end = time.monotonic() + timeout
        while True:
            left = end - time.monotonic()
            if left <= 0:
                return None
            for i, ev in enumerate(self.term.read(left)):
                if ev.kind in kinds:
                    return ev
                self.inbox.append(ev)

    def run_probe(self):
        p = self.probe
        env = {k: os.environ.get(k, "") for k in ("TERM_PROGRAM", "TERM_PROGRAM_VERSION", "COLORTERM", "TERM")}
        p["who"] = (env["TERM_PROGRAM"] or env["TERM"] or "unknown terminal") + (
            " " + env["TERM_PROGRAM_VERSION"] if env["TERM_PROGRAM_VERSION"] else "")
        self.log.line("terminal: " + " ".join("%s=%r" % kv for kv in env.items()) + " size=%dx%d" % self.size())
        self.term.write("\x1b]11;?\x1b\\\x1b[?2026$p\x1b[5n")
        end = time.monotonic() + 1.0
        osc = decrpm = dsr = None
        while dsr is None and time.monotonic() < end:
            for ev in self.term.read(end - time.monotonic()):
                if ev.kind == "osc" and ev.v.startswith("11;"):
                    osc = ev.v[3:]
                elif ev.kind == "decrpm" and ev.v == "2026":
                    decrpm = ev  # the DECRPM value is in ev.x (csi_event)
                elif ev.kind == "dsr":
                    dsr = ev
                else:
                    self.inbox.append(ev)
        p["osc11"] = osc
        p["sync"] = None if decrpm is None else decrpm.x in (1, 2, 3)
        self.log.line("OSC 11 background reply: %r" % osc)
        self.log.line("DEC 2026 (synchronized output) DECRQM reply: %s" % (
            "none before the DSR answer" if decrpm is None else "mode 2026 value %d (%s)" % (
                decrpm.x, {0: "not recognised", 1: "set", 2: "reset, supported", 3: "permanently set",
                           4: "permanently reset"}.get(decrpm.x, "?"))))
        if osc and osc.startswith("rgb:"):
            try:
                parts = [int(c[:2], 16) for c in osc[4:].split("/")]
                lum = luminance("#%02X%02X%02X" % tuple(parts))
                p["bg_light"] = lum > 0.4
                self.log.line("background reads as %s (luminance %.2f)" % ("light" if lum > 0.4 else "dark", lum))
            except (ValueError, TypeError) as e:
                self.log.line("OSC 11 reply not parsed: %s" % e)
        if dsr is None:
            p["dsr_p50"] = None
            self.log.line("DSR (CSI 5 n): no answer within 1 s — the fence would run on its 16 ms fallback")
            return
        rtts = []
        for _ in range(60):
            t0 = time.monotonic()
            self.term.write("\x1b[5n")
            ev = self.wait_for(("dsr",), 0.5)
            if ev is None:
                break
            rtts.append(ev.t - t0)
        rtts.sort()
        if rtts:
            p["dsr_p50"] = 1000 * rtts[len(rtts) // 2]
            p["dsr_p99"] = 1000 * rtts[min(len(rtts) - 1, int(len(rtts) * 0.99))]
            self.log.line("DSR round trip, %d samples, idle: p50 %.2f ms, p99 %.2f ms, max %.2f ms" % (
                len(rtts), p["dsr_p50"], p["dsr_p99"], 1000 * rtts[-1]))
        else:
            p["dsr_p50"] = None

    def global_key(self, ev):
        """Prototype keys every scene shares. Returns True when it handled the event."""
        sc = self.scene
        if ev.kind != "key":
            return False
        k = ev.v
        if k == "ctrl-c":
            self.log.line("quit (Ctrl-C)")
            self.done = True
            return True
        if k == "capture" and self.args.capture:
            self.capture()
            return True
        if sc.q and len(k) == 1 and k.isdigit():
            n = int(k)
            qid, text, opts = sc.q
            if 1 <= n <= len(opts):
                self.log.line("ANSWER %s: %d (%s)" % (qid, n, opts[n - 1]))
                sc.q = None
                sc.done = True
                sc.answered(n)
                return True
            return True
        if k == "q" and not sc.text_mode():
            self.log.line("quit (q)")
            self.done = True
            return True
        if sc.done and k == "n":
            if sc.num < 4:
                self.set_scene(sc.num + 1)
            else:
                self.done = True
            return True
        if sc.q or sc.done:
            return True  # nothing else while a question is open
        return False

    def capture(self):
        if self.last_canvas is None:
            return
        self.captures += 1
        path = "%s-%d.txt" % (self.args.capture, self.captures)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write("\n".join(self.last_canvas.lines()).rstrip() + "\n")

    def set_scene(self, n):
        self.log.scene = "scene%d" % n
        self.log.line("scene %d started" % n)
        self.scene = SCENES[n](self)
        self.term.write("\x1b[2J")

    def run(self):
        self.run_probe()
        if self.probe.get("bg_light"):
            self.pal = PAL["light"]
        self.set_scene(self.args.scene)
        while not self.done:
            sc = self.scene
            if sc.custom_loop:
                sc.loop()
                continue
            w, h = self.size()
            cv = sc.frame(w, h)
            self.last_canvas = cv
            self.term.write("\x1b[?2026h" + cv.ansi() + "\x1b[?2026l")
            now = time.monotonic()
            waits = [t for t in (sc.tick(now), (sc.flash_until - now) if sc.flash_until > now else None)
                     if t is not None]
            evs = self.next_events(max(0.0, min(waits)) if waits else None)
            for ev in evs:
                if ev.kind in ("dsr", "decrpm", "osc", "cpr"):
                    continue
                if ev.kind == "key":
                    self.log.input(ev.v)
                elif ev.kind == "mouse" and ev.v in ("press", "release"):
                    self.log.input("mouse-" + ev.v)
                if self.global_key(ev):
                    if self.done or self.scene is not sc:
                        break
                    continue
                sc.on(ev)
            sc.tick(time.monotonic())


ENTER = "\x1b[?1049h\x1b[?25l\x1b[?7l\x1b[?1000h\x1b[?1002h\x1b[?1006h\x1b[?2004h"
LEAVE = "\x1b[?2026l\x1b[0m\x1b[?2004l\x1b[?1006l\x1b[?1002l\x1b[?1000l\x1b[?7h\x1b[?25h\x1b[?1049l"


def main():
    ap = argparse.ArgumentParser(description="PROTOTYPE: M8 polish (throwaway).")
    ap.add_argument("scene_pos", nargs="?", type=int, choices=[1, 2, 3, 4], help="scene number")
    ap.add_argument("--scene", type=int, choices=[1, 2, 3, 4])
    ap.add_argument("--no-colour", "--no-color", action="store_true", help="colour off (shapes only)")
    ap.add_argument("--picks", default=PICKS_DEFAULT, help="where keys, measurements and answers are appended")
    ap.add_argument("--capture", help="prefix: Ctrl-\\ writes the current frame to <prefix>-<n>.txt")
    args = ap.parse_args()
    args.scene = args.scene_pos or args.scene
    if args.no_colour:
        COLOUR[0] = False
    if not sys.stdin.isatty():
        sys.exit("polish.py needs a real terminal (it measures it). Run it in Ghostty, Zed or VS Code.")
    if not args.scene:
        sys.stdout.write("Which scene?  1 scroll feel   2 finding keys   3 first run + tour   4 themes  > ")
        sys.stdout.flush()
        ans = sys.stdin.readline().strip()
        if ans not in ("1", "2", "3", "4"):
            sys.exit("pick 1, 2, 3 or 4")
        args.scene = int(ans)
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    term = Term(fd, sys.stdout.fileno())

    def on_signal(signum, frame):
        raise SystemExit(128 + signum)
    for s in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(s, on_signal)
    app = None
    try:
        tty.setraw(fd)
        term.write(ENTER)
        app = App(args, term)
        app.run()
    finally:
        try:
            term.write(LEAVE)
            # Late terminal replies (a fence answer still in flight) must not land in his shell.
            end = time.monotonic() + 0.15
            while time.monotonic() < end and select.select([fd], [], [], end - time.monotonic())[0]:
                if not os.read(fd, 65536):
                    break
        finally:
            termios.tcflush(fd, termios.TCIFLUSH)
            termios.tcsetattr(fd, termios.TCSADRAIN, old)
    if app:
        app.log.line("left the prototype")
        print("keys, measurements and answers written to", os.path.relpath(args.picks))


if __name__ == "__main__":
    main()
