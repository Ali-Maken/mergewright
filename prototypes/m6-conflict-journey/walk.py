#!/usr/bin/env python3
"""PROTOTYPE — throwaway. M6 conflict journey, Wizard-of-Oz (design note m6-conflicts, "Prototype (pipeline step 5)").

Task, in the user's words: "merge `feature` into `main` and get it finished."
Repo pictured: /tmp/graft-conflict — notes.txt with 2 conflict blocks, gone.txt modified on main / deleted on feature.
Nothing here touches git. Frames are canned text; the script only steps on keys.

  python3 prototypes/m6-conflict-journey/walk.py                 # path A, in-spec keys only
  python3 prototypes/m6-conflict-journey/walk.py --proposed      # P1-P5 on
  python3 prototypes/m6-conflict-journey/walk.py --start shell   # path B: merge begun in the shell
  python3 prototypes/m6-conflict-journey/walk.py --capture OUT   # write every scripted frame at 120x40
q quits. Colour: none (plain text; reverse video etc. is not prototyped).
"""
import argparse
import os
import select
import sys

REPO = "/tmp/graft-conflict"
KEYNAMES = {"\r": "⏎", "\t": "⇥", "\x1b": "Esc", "\x1b[C": "→", "\x1b[D": "←", "\x0f": "Ctrl+O", "\x14": "Ctrl+T"}
NOTES = [  # (ours, theirs) per block, with disk line numbers of each side's first line
    {"ours": ["two from main"], "theirs": ["TWO from feature"]},
    {"ours": ["five from main"], "theirs": ["FIVE from feature"]},
]


def kname(k):
    return KEYNAMES.get(k, k)


class Walk:
    def __init__(self, proposed, start):
        self.p = proposed
        self.start = start
        self.keys = []
        self.stalls = 0
        self.msg = ""
        self.done = False
        self.outcome = ""
        if start == "shell":
            self.screen = "banner"
            self.banner = "launch"
        else:
            self.screen = "graph"
        self.sel = 0  # preview button: 0 Cancel, 1 Confirm
        self.file = 0  # 0 gone.txt, 1 notes.txt
        self.gone = None  # None / "ours" / "theirs"
        self.blocks = [None, None]
        self.cur = 0

    # ---------- transitions ----------
    def press(self, k):
        self.keys.append(kname(k))
        self.msg = ""
        ok = getattr(self, "on_" + self.screen)(k)
        if not ok:
            self.stalls += 1
            self.msg = f"STALL {self.stalls}: `{kname(k)}` does nothing here"

    def stop(self):
        if self.p:  # P5: a stop with anything unmerged lands in the resolver
            self.screen, self.file, self.cur = "resolver", 0, 0
        else:
            self.screen, self.banner = "banner", "stopped"

    def on_graph(self, k):
        if k == "m":
            self.screen, self.sel = "preview", 0
            return True
        return False

    def on_preview(self, k):
        if k == "\x1b[C":
            self.sel = 1
        elif k == "\x1b[D":
            self.sel = 0
        elif k == "\r":
            if self.sel == 1:
                self.stop()
            else:
                self.screen = "graph"
                self.msg = "merge cancelled (cursor was on Cancel)"
        elif k == "m" and self.p:  # P1
            self.stop()
        elif k == "\x1b":
            self.screen = "graph"
        else:
            return False
        return True

    def resolved(self):
        return self.gone is not None and all(self.blocks) and self.file == 2

    def on_banner(self, k):
        if k == "\r" and self.banner in ("stopped", "launch"):
            self.screen, self.file, self.cur = "resolver", 0, 0
        elif k == "c" and self.banner == "resolved":
            if self.start == "shell" and not self.p:
                self.banner = "refused"
                self.done, self.outcome = True, "left the app: `git commit` in the shell (in-spec refusal)"
            else:
                self.screen, self.done, self.outcome = "merged", True, "merged in the app"
        elif k == "\x1b" and self.banner != "launch":
            self.screen = "graph_live"
        elif k == "\t":
            self.screen = "graph_live"
        else:
            return False
        return True

    def on_graph_live(self, k):
        if k == "\t":
            self.screen = "banner"
            return True
        return False

    def on_resolver(self, k):
        if k == "\x1b":
            self.screen = "banner"
            return True
        if self.file == 0:  # gone.txt: modify/delete, one whole-file choice
            if k in ("o", "t"):
                self.gone = "ours" if k == "o" else "theirs"
            elif k == "\r" and self.gone:
                self.file, self.cur = 1, 0
            else:
                return False
            return True
        b = self.blocks
        if k in ("o", "t", "b"):
            if b[self.cur] is not None:
                return False
            b[self.cur] = {"o": "ours", "t": "theirs", "b": "both"}[k]
            if self.p:  # P2: advance to next open block
                nxt = [i for i in range(len(b)) if b[i] is None]
                if nxt:
                    self.cur = nxt[0]
        elif k in ("\x0f", "\x14") and self.p:  # P3
            for i in range(len(b)):
                if b[i] is None:
                    b[i] = "ours" if k == "\x0f" else "theirs"
        elif k in ("n", "N"):
            opens = [i for i in range(len(b)) if b[i] is None]
            pool = range(len(b)) if self.p else opens
            pool = [i for i in pool if i != self.cur]
            if not pool:
                self.msg = "no more conflicts"
                return False
            self.cur = pool[0]
        elif k == "\r" and all(b):
            self.file = 2
            self.screen, self.banner = "banner", "resolved"
        else:
            return False
        return True

    # ---------- drawing ----------
    def title(self, w):
        tag = "MERGING" if self.screen != "merged" else ""
        left = f" graft {REPO}   PROTOTYPE{'  --proposed' if self.p else ''}"
        return left + " " * max(1, w - len(left) - len(tag) - 1) + tag

    def graph_rows(self, cursor_on, merged=False):
        rows = []
        if merged:
            rows.append("◆─╮ ▏  [◆ main] Merge branch 'feature' into main                         MA now  4c1e2d0")
            rows.append("│ ●    [feature] feature                                               MA now  d709ba8")
            rows.append("●─╯    main                                                            MA now  e25730c")
            rows.append("◇      base                                                            MA now  055b111")
            return rows
        if self.screen != "graph" and self.screen != "preview":
            rows.append("◌      Uncommitted changes (2 unmerged)")
        mark = lambda n: "▏" if cursor_on == n else " "
        rows.append(f"  ● {mark('feature')}  [feature] feature                                               MA now  d709ba8")
        rows.append(f"◆ │ {mark('main')}  [◆ main] main                                                  MA now  e25730c")
        rows.append("◇─╯    base                                                            MA now  055b111")
        return rows

    def bottom(self):
        s = self.screen
        if s == "graph":
            return " . explain  m merge  r rebase  ⏎ open  b  t  P  c  p  f  u  S  /  q"
        if s == "graph_live":
            return " merge stopped · 2 files conflicted · ⇥ banner  ⏎ open  c commit  . more  …"
        if s == "banner":
            return {
                "stopped": " merge stopped · 2 files conflicted · ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph",
                "launch": " merge in progress · begun outside Graft · 2 files conflicted · ⏎ resolve  c continue  a abort  ⇥ graph",
                "resolved": " all conflicts resolved · c continue  a abort  ⇥ graph",
                "refused": " merge begun in the shell — finish it there: git commit · a abort  ⇥ graph",
            }[self.banner]
        if s == "merged":
            return " merged feature into main ✓ · ⏎ open  u undo  P push  …"
        return ""

    def resolver_rows(self, w):
        rows = []
        if self.file == 0:
            rows.append("gone.txt · file 1 of 2 · modified on main, deleted on feature")
            rows.append("ours = main · theirs = the branch being merged in")
            state = {"ours": "keep main's edit", "theirs": "take the deletion", None: "open"}[self.gone]
            keys = "o keep (ours)  t delete (theirs)" + ("  ⏎ stage" if self.gone else "")
            head = f"▏  ── gone.txt · {state} ── {keys} "
            rows.append(head + "─" * max(0, w - len(head)))
            rows.append("▏  OURS · HEAD (modified)                     │ THEIRS · feature")
            rows.append("▏    1 keep me, edited on main                │   (file deleted)")
            rows.append("▏ " + "─" * 44 + "┴" + "─" * max(0, w - 48))
            return rows
        b = self.blocks
        n_open = sum(1 for x in b if x is None)
        rows.append(f"notes.txt · file 2 of 2 · {n_open} of 2 conflicts open")
        rows.append("ours = main · theirs = the branch being merged in")
        disk = 1
        rows.append(f"   {disk:>3} one")
        disk += 1
        for i, blk in enumerate(b):
            cur = "▏" if i == self.cur else " "
            if blk is None:
                keys = "o ours  t theirs  b both"
                head = f"{cur}      ── conflict {i+1} of 2 · open ── {keys} "
                rows.append(head + "─" * max(0, w - len(head)))
                rows.append(f"{cur}       {'OURS · HEAD':<37}│     THEIRS · feature")
                # P-note: both sides carry their disk number (fix-resolver-line-numbers)
                # on disk: <<<, ours, ===, theirs, >>> ; markers carry no number
                o_line, t_line = disk + 1, disk + 3
                rows.append(f"{cur}   {o_line:>3} {NOTES[i]['ours'][0]:<37}│ {t_line:>3} {NOTES[i]['theirs'][0]}")
                rows.append(f"{cur}       " + "─" * 37 + "┴" + "─" * max(0, w - 46))
                disk += 5
            else:
                other = {"ours": "t theirs  b both", "theirs": "o ours  b both", "both": "o ours  t theirs"}[blk]
                head = f"{cur}      ── conflict {i+1} of 2 · took {blk} ── {other} "
                rows.append(head + "─" * max(0, w - len(head)))
                side = NOTES[i]["ours" if blk == "ours" else "theirs"]
                lines = side if blk != "both" else NOTES[i]["ours"] + NOTES[i]["theirs"]
                for ln in lines:
                    rows.append(f"{cur}   {disk:>3} {ln}")
                    disk += 1
            if i == 0:
                rows.append(f"   {disk:>3} three")
                rows.append(f"   {disk+1:>3} four")
                disk += 2
        rows.append("")
        if n_open == 0:
            rows.append("   all blocks taken · ⏎ stage notes.txt")
        return rows

    def resolver_bar(self):
        if self.file == 0:
            return " o ours  t theirs  ⏎ stage  e edit  esc banner"
        extra = "  Ctrl+O all ours  Ctrl+T all theirs" if self.p else ""
        return f" o ours  t theirs  b both  n/N next/prev{extra}  ⏎ stage  e edit  esc banner"

    def preview_box(self, w):
        inner = w - 2
        L = lambda t: "│" + (" " + t).ljust(inner)[:inner] + "│"
        top = "╭ Preview: merge feature into main "
        c = "▸[ Cancel ]   [ Confirm ]" if self.sel == 0 else " [ Cancel ]  ▸[ Confirm ]"
        foot = "╰ " + ("m merge  " if self.p else "") + "←→ choose  ⏎ act  esc cancel "
        return [
            top + "─" * (w - len(top) - 1) + "╮",
            L("1 merge commit"),
            L("main e25730c → 1 new commit"),
            L("working tree: 2 files updated"),
            L("may conflict: gone.txt, notes.txt"),
            L("undo: u takes this back"),
            L("git merge --no-edit feature"),
            L(c),
            foot + "─" * (w - len(foot) - 1) + "╯",
        ]

    def frame(self, w, h):
        out = [self.title(w)]
        s = self.screen
        foot = []
        if s in ("graph", "preview"):
            body = self.graph_rows("feature")
        elif s == "graph_live":
            body = self.graph_rows("main")
        elif s == "banner":
            body = self.graph_rows("main")
            body.append("")
            body.append("  [banner focused]  CONFLICTS in the changes pane: gone.txt, notes.txt"
                        if self.banner != "resolved" else "  [banner focused]  all conflicts staged")
        elif s == "resolver":
            body = self.resolver_rows(w)
        elif s == "merged":
            body = self.graph_rows(None, merged=True)
        out += body
        if s == "preview":
            foot = self.preview_box(w)
        bar = self.resolver_bar() if s == "resolver" else self.bottom()
        tail = foot + [(" " + self.msg) if self.msg else "", bar]
        while len(out) + len(tail) < h:
            out.append("")
        out += tail
        return [ln[:w].ljust(w) if len(ln) <= w else ln[: w - 1] + "…" for ln in out[:h]]

    def caption(self):
        s, p = self.screen, self.p
        if s == "graph":
            return "graph, cursor on `feature` · next: m"
        if s == "preview":
            if self.sel == 0:
                return "preview, cursor on Cancel · next: " + ("m (P1) or → then ⏎" if p else "→ then ⏎")
            return "preview, cursor on Confirm · next: ⏎"
        if s == "banner":
            return {
                "stopped": "stop landed on the banner; its keys own the bottom row · next: ⏎ (or Esc to leave)",
                "launch": "launched into a shell-begun merge: banner focused · next: ⏎",
                "resolved": "every file staged, banner focused · next: c",
                "refused": "in-spec: c refused for a shell-begun merge — journey leaves the app here · q",
            }[self.banner]
        if s == "graph_live":
            return "after Esc: graph focused, bar led by `⇥ banner` · next: ⇥"
        if s == "resolver":
            if self.file == 0:
                return "resolver on gone.txt (modify/delete) · next: " + ("⏎" if self.gone else "o (keep main's edit)")
            if all(self.blocks):
                return "notes.txt, both blocks taken · next: ⏎"
            if p and self.blocks == [None, None]:
                return "notes.txt card, both sides numbered, no number on the header · next: Ctrl+T (P3) or t (P2)"
            if p:
                return "after t: took theirs, cursor advanced to block 2 (P2) · next: t"
            if self.blocks[self.cur] is not None:
                return "after t: cursor stays on the taken block (in-spec) · next: n"
            return "notes.txt card, both sides numbered, no number on the header · next: t"
        if s == "merged":
            return "merged: graph focused, cursor on the new merge commit · done"
        return ""

    def note_count(self):
        if self.start == "shell":
            if self.p:
                return "b-launch-to-first-conflict 1 + gone 2 + notes P3:2 + b-finish P4:1 = 6"
            return "b-launch-to-first-conflict 1 + gone 2 + notes t n t ⏎ 4 + b-finish `-` (leaves app) = 7 + shell"
        if self.p:
            return "a-merge-to-stop P1:2 + a-stop-to-first-conflict P5:0 + gone 2 + notes P3:2 (P2:3) + a-finish 1 = 7 (P2 route 8)"
        return "a-merge-to-stop 3 + a-stop-to-first-conflict 1 + gone 2 + notes t n t ⏎ 4 + a-finish 1 = 11"

    def summary(self):
        return [
            f"PROTOTYPE walk · start={self.start} · {'proposed P1-P5' if self.p else 'in-spec'}",
            f"outcome: {self.outcome or 'quit before finishing'}",
            f"keys pressed: {len(self.keys)}  ({' '.join(self.keys)})",
            f"stalls: {self.stalls}",
            f"design note count: {self.note_count()}",
        ]


def pick_size():
    try:
        c, r = os.get_terminal_size()
    except OSError:
        return 120, 40
    return min([(120, 40), (200, 50)], key=lambda s: abs(s[0] - c) + abs(s[1] - r))


def read_key(fd):
    ch = os.read(fd, 1).decode(errors="replace")
    if ch == "\x1b":
        if select.select([fd], [], [], 0.05)[0]:
            seq = ch + os.read(fd, 8).decode(errors="replace")
            return seq
    return ch


def interactive(args):
    import termios
    import tty

    w, h = pick_size()
    walk = Walk(args.proposed, args.start)
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    sys.stdout.write("\x1b[?1049h\x1b[?25l")
    try:
        tty.setraw(fd)
        while True:
            lines = walk.frame(w, h)
            sys.stdout.write("\x1b[H\x1b[2J" + "\r\n".join(lines))
            sys.stdout.flush()
            if walk.done:
                k = read_key(fd)
                break
            k = read_key(fd)
            if k in ("q", "\x03"):
                break
            walk.press(k)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
        sys.stdout.write("\x1b[?25h\x1b[?1049l")
        sys.stdout.flush()
    print("\n".join(walk.summary()))


SCRIPTS = [
    ("A in-spec", False, "app", ["m", "\x1b[C", "\r", "\r", "o", "\r", "t", "n", "t", "\r", "c"]),
    ("A in-spec, Esc detour (J7)", False, "app", ["m", "\x1b[C", "\r", "\x1b", "\t", "\r"]),
    ("A proposed", True, "app", ["m", "m", "o", "\r", "\x14", "\r", "c"]),
    ("A proposed, P2 route", True, "app", ["m", "m", "o", "\r", "t", "t", "\r", "c"]),
    ("B in-spec", False, "shell", ["\r", "o", "\r", "t", "n", "t", "\r", "c"]),
    ("B proposed", True, "shell", ["\r", "o", "\r", "\x14", "\r", "c"]),
]


def capture(path):
    out = [
        "# M6 conflict journey — PROTOTYPE captures (walk.py --capture), 120x40",
        "# Wizard-of-Oz: canned frames, no git. Task: \"merge `feature` into `main` and get it finished.\"",
        "# Colour: none. These are plain text; reverse video / colour is not prototyped and not verifiable here.",
        "# Each frame sits under one line: what you should see / what key next.",
        "",
    ]
    for name, prop, start, keys in SCRIPTS:
        walk = Walk(prop, start)
        out.append("#" * 120)
        out.append(f"# RUN: {name}   (keys: {' '.join(kname(k) for k in keys)})")
        out.append("#" * 120)
        step = 0
        while True:
            out.append(f"=== PROTOTYPE {name} · frame {step} · {walk.caption()}")
            out += [ln.rstrip() for ln in walk.frame(120, 40)]
            if step >= len(keys):
                break
            walk.press(keys[step])
            step += 1
        out += ["", *("# " + s for s in walk.summary()), ""]
    with open(path, "w") as f:
        f.write("\n".join(out) + "\n")


def main():
    ap = argparse.ArgumentParser(description="PROTOTYPE: M6 conflict journey walk")
    ap.add_argument("--proposed", action="store_true", help="turn on P1-P5")
    ap.add_argument("--start", choices=["app", "shell"], default="app")
    ap.add_argument("--capture", metavar="OUT")
    args = ap.parse_args()
    if args.capture:
        capture(args.capture)
    else:
        interactive(args)


if __name__ == "__main__":
    main()
