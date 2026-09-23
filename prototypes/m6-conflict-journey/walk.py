#!/usr/bin/env python3
"""PROTOTYPE — throwaway. M6 conflict journey, Wizard-of-Oz (design note m6-conflicts, "Prototype (pipeline step 5)").

Task, in the user's words: "merge `feature` into `main` and get it finished."
Repo pictured: /tmp/graft-conflict — notes.txt with 2 conflict blocks, gone.txt modified on main / deleted on feature.
Nothing here touches git. Frames are canned text; the script only steps on keys.

  python3 prototypes/m6-conflict-journey/walk.py                 # v2: --layout jb (file list, 3 panes >=180 cols)
  python3 prototypes/m6-conflict-journey/walk.py --layout card   # v1, path A, in-spec keys only
  python3 prototypes/m6-conflict-journey/walk.py --layout card --proposed   # v1 with P1-P5 on
  python3 prototypes/m6-conflict-journey/walk.py --start shell   # path B: merge begun in the shell
  python3 prototypes/m6-conflict-journey/walk.py --capture OUT   # write every scripted frame (jb 200x50 + 120x40, v1 120x40)
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
        tag = "" if self.screen in ("graph", "preview", "merged") else "MERGING"
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

    def resolver_bar(self, w=0):
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
        if s == "preview":
            foot = self.preview_box(w)
        bar = self.resolver_bar(w) if s == "resolver" else self.bottom()
        tail = foot + [(" " + self.msg) if self.msg else "", bar]
        out += body[: max(0, h - 1 - len(tail))]  # never push the key bar off the bottom
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


YOURS, THEIRS = "main (yours)", "feature (theirs)"  # one name per side, everywhere (audit C)
FILES = ["gone.txt", "notes.txt"]
# --auto: files git merged cleanly on its own (faked; the canned repo has none). (name, +, -, diff lines)
AUTO = [
    ("README.md", 3, 1, ["@@ -4,3 +4,5 @@", "   ## Usage", " - run: make", " + run: make run", " + test: make test",
                         " + lint: make lint"]),
    ("src/app.rs", 5, 2, ["@@ -10,4 +10,7 @@ fn main() {", "   let cfg = load();", " - run(cfg);", " - exit(0);",
                          " + let app = App::new(cfg);", " + app.run()?;", " + log::info!(\"bye\");",
                          " + Ok(())", " + // from main", "   }"]),
]
# notes.txt as each side has it on disk: context lines and the 2 blocks. Both sides number 1-6 contiguously.
NOTES_LAYOUT = ["one", 0, "three", "four", 1, "six"]


def fit(s, n):
    return s.ljust(n) if len(s) <= n else s[: n - 1] + "…"


class JbWalk(Walk):
    """--layout jb: JetBrains-style resolver. File list always on screen; 3 panes at >=180 cols, card below.

    Keys are --proposed's (P1-P5) plus two list keys: j/k move between files, O/T take a whole file
    as main (yours) / feature (theirs) and mark it done (audit A: 1 key per file)."""

    def __init__(self, start, auto=False):
        super().__init__(True, start)
        self.staged = [False, False]
        self.auto = AUTO if auto else []  # files git combined on its own (canned repo: none)
        self.message = "Merge branch 'feature' into main"
        self.editing = None  # None, or the line being edited
        self.srow = -1  # summary cursor: -1 message line, i >= 0 an auto-merged file
        self.diff = None  # index of the auto-merged file whose diff is open

    # ---------- finish summary (field note 2026-09-23: approved) ----------
    def open_summary(self):
        self.screen, self.srow, self.editing = "summary", -1, None

    def on_summary(self, k):
        if self.editing is not None:
            if k == "\r":
                self.message, self.editing = self.editing or self.message, None
            elif k == "\x1b":
                self.editing = None
            elif k in ("\x7f", "\x08"):
                self.editing = self.editing[:-1]
            elif len(k) == 1 and k.isprintable():
                self.editing += k
            else:
                return False
            return True
        if k == "\x1b":
            self.screen = "resolver"
        elif k == "e":
            self.editing, self.srow = self.message, -1
        elif k in ("j", "k") and self.auto:
            self.srow = max(-1, min(len(self.auto) - 1, self.srow + (1 if k == "j" else -1)))
        elif k == "\r" and self.srow >= 0:
            self.screen, self.diff = "autodiff", self.srow
        elif k == "\r":
            self.screen, self.done, self.outcome = "merged", True, "merged in the app"
        else:
            return False
        return True

    def on_autodiff(self, k):
        if k == "\x1b":
            self.screen = "summary"
            return True
        return False

    def summary_rows(self, w):
        g = {"ours": f"kept {YOURS}", "theirs": f"took {THEIRS}"}
        n = self.blocks
        notes = g[n[0]] if n[0] == n[1] and n[0] in g else " · ".join(
            f"conflict {i+1} {'both' if b == 'both' else g[b].split(' ', 1)[0] + ' ' + self.took(b)}"
            for i, b in enumerate(n))
        rows = ["Finish merge: feature → main", "",
                "resolved by you:",
                f"   gone.txt    {g[self.gone] if self.gone == 'ours' else 'took ' + THEIRS + ' (deleted)'}",
                f"   notes.txt   {notes}", "",
                "combined by git on its own:"]
        if not self.auto:
            rows.append("   none")
        for i, (name, a, d, _) in enumerate(self.auto):
            m = "▏" if self.srow == i else " "
            rows.append(f" {m} {name:<11} +{a} -{d}" + ("   ⏎ open" if self.srow == i else ""))
        m = "▏" if self.srow == -1 else " "
        msg = (self.editing + "▌") if self.editing is not None else self.message
        rows += ["", "commit message:", f" {m} {msg}"]
        return rows

    def autodiff_rows(self, w):
        name, a, d, lines = self.auto[self.diff]
        return [f"{name} · combined by git on its own · +{a} -{d}", ""] + [f"  {ln}" for ln in lines]

    def bottom(self):
        if self.screen == "summary":
            if self.editing is not None:
                return " editing message · ⏎ keep  esc cancel  ⌫ delete"
            if self.srow >= 0:
                return " ⏎ open diff  j/k move  e edit message  esc back to resolver"
            j = "  j/k files" if self.auto else ""
            return " ⏎ commit  e edit message" + j + "  esc back to resolver"
        if self.screen == "autodiff":
            return " esc back to summary"
        return super().bottom()

    def frame(self, w, h):
        if self.screen not in ("summary", "autodiff", "merged"):
            return super().frame(w, h)
        body = {"summary": self.summary_rows, "autodiff": self.autodiff_rows,
                "merged": lambda w: [r.replace("Merge branch 'feature' into main", fit(self.message, 32))
                                     for r in self.graph_rows(None, merged=True)]}[self.screen](w)
        tail = [(" " + self.msg) if self.msg else "", self.bottom()]
        out = [self.title(w)] + body[: max(0, h - 1 - len(tail))]
        out += [""] * (h - len(out) - len(tail)) + tail
        return [fit(ln, w) for ln in out[:h]]

    def title(self, w):
        tag = "" if self.screen in ("graph", "preview", "merged") else "MERGING"
        left = f" graft {REPO}   PROTOTYPE  --layout jb{'  --auto' if self.auto else ''}"
        return left + " " * max(1, w - len(left) - len(tag) - 1) + tag

    def on_banner(self, k):
        if k == "c" and self.banner == "resolved":
            self.open_summary()
            return True
        return super().on_banner(k)

    # ---------- resolver ----------
    def file_open(self, f):
        if f == 0:
            return 0 if self.gone else 1
        return sum(1 for x in self.blocks if x is None)

    def stage(self):
        prev = self.file
        self.staged[prev] = True
        nxt = [f for f in range(2) if not self.staged[f]]
        if nxt:
            self.file, self.cur = nxt[0], 0
            self.msg = f"{FILES[prev]} done ✓ · ▸ moved to {FILES[self.file]}"
        else:
            self.msg = "every file done ✓ · c finishes the merge"

    def on_resolver(self, k):
        f = self.file
        if k == "\x1b":
            self.screen, self.banner = "banner", "stopped"
            return True
        if k in ("j", "k"):
            nf = min(1, f + 1) if k == "j" else max(0, f - 1)
            if nf == f:
                return False
            self.file, self.cur = nf, 0
            return True
        if k == "c":
            if all(self.staged):
                self.open_summary()
                return True
            return False
        if k in ("O", "T"):  # whole file from the list
            side = "ours" if k == "O" else "theirs"
            if f == 0:
                self.gone = side
            else:
                self.blocks = [side, side]
            self.stage()
            return True
        if f == 0:
            if k in ("o", "t"):
                self.gone = "ours" if k == "o" else "theirs"
                self.staged[0] = False
            elif k == "\r" and self.gone:
                self.stage()
            else:
                return False
            return True
        b = self.blocks
        if k in ("o", "t", "b"):
            b[self.cur] = {"o": "ours", "t": "theirs", "b": "both"}[k]
            self.staged[1] = False
            nxt = [i for i in range(len(b)) if b[i] is None]
            if nxt:
                self.cur = nxt[0]
        elif k in ("\x0f", "\x14"):
            for i in range(len(b)):
                if b[i] is None:
                    b[i] = "ours" if k == "\x0f" else "theirs"
        elif k in ("n", "N"):
            self.cur = 1 - self.cur
        elif k == "\r" and all(b):
            self.stage()
        else:
            return False
        return True

    def state_word(self, f):
        if self.staged[f]:
            return "done ✓"
        n = self.file_open(f)
        total = 1 if f == 0 else 2
        return f"{n} of {total} open" if n else "⏎ to mark done"

    def list_column(self):
        rows = [" FILES · 2 conflicted", ""]
        for f, name in enumerate(FILES):
            m = "▸" if f == self.file else " "
            rows.append(f" {m} {name:<10} {self.state_word(f)}")
        rows += ["", "  j/k file", "  O/T whole file"]
        return rows

    def list_strip(self):
        parts = [f"{'▸' if f == self.file else ' '} {n} · {self.state_word(f)}" for f, n in enumerate(FILES)]
        return " files:  " + "   │   ".join(parts)

    def took(self, side):
        return {"ours": YOURS, "theirs": THEIRS, "both": "both"}[side]

    def kind_row(self):
        if self.file == 0:
            return f"▸ gone.txt · modified on {YOURS}, deleted on {THEIRS}"
        return f"▸ notes.txt · 2 conflicts · {self.file_open(1)} open"

    # 3 panes: main (yours) │ result │ feature (theirs)
    def panes(self, w):
        pw = (w - 1 - 2) // 3
        L, M, R = [f"  {YOURS}"], ["  result"], [f"  {THEIRS}"]
        cur = []
        num = lambda n, t: f"{n:>4} {t}"
        if self.file == 0:
            g = self.gone
            L += ["── taken ✓" if g == "ours" else ("── not taken" if g else "── o keep"),
                  num(1, "keep me, edited on main")]
            M += [f"── {'took ' + self.took(g) if g else 'open · o keep  t delete'}",
                  num(1, "keep me, edited on main") if g == "ours" else ("     (file deleted)" if g else "     ?")]
            R += ["── taken ✓" if g == "theirs" else ("── not taken" if g else "── t delete"),
                  "     (deleted on feature)"]
            cur = [False, True, True]
        else:
            n, rn = 1, 1
            for item in NOTES_LAYOUT:
                if isinstance(item, str):
                    L.append(num(n, item)); R.append(num(n, item)); M.append(num(rn, item))
                    cur.append(False)
                    n += 1; rn += 1
                    continue
                i, blk = item, self.blocks[item]
                on = i == self.cur
                tag = lambda s: "── taken ✓" if blk in (s, "both") else ("── not taken" if blk else f"── {'o' if s == 'ours' else 't'} take")
                L.append(tag("ours")); R.append(tag("theirs"))
                M.append(f"── conflict {i+1} of 2 · " + (f"took {self.took(blk)}" if blk else "open"))
                o, t = NOTES[i]["ours"][0], NOTES[i]["theirs"][0]
                res = {"ours": [o], "theirs": [t], "both": [o, t], None: []}[blk]
                L.append(num(n, o)); R.append(num(n, t))
                if res:
                    M.append(num(rn, res[0]))
                else:
                    M.append("     ? open")
                cur += [on, on]
                for extra in res[1:]:
                    rn += 1
                    L.append(""); R.append(""); M.append(num(rn, extra)); cur.append(on)
                n += 1; rn += 1
            cur = [False] + cur
        rows = []
        for j in range(max(len(L), len(M), len(R))):
            g = lambda col: col[j] if j < len(col) else ""
            c = "▏" if j < len(cur) and cur[j] else " "
            rows.append(c + fit(g(L), pw) + "│" + fit(g(M), pw) + "│" + fit(g(R), pw))
        return rows, 1 + 3 * pw + 2

    # card below 180 cols, with audit C's fixes: one name per side, contiguous numbers
    def card(self, w):
        half = (w - 9) // 2
        rows = []
        if self.file == 0:
            g = self.gone
            state = f"took {self.took(g)}" if g else "open"
            head = f"▏ ── {state} ── o keep, as {YOURS}  t delete, as {THEIRS} "
            rows.append((head + "─" * w)[: w - 1])
            rows.append("▏      " + fit(YOURS, half) + "│ " + THEIRS)
            rows.append("▏    1 " + fit("keep me, edited on main", half) + "│      (deleted)")
            return rows
        n = 1
        for item in NOTES_LAYOUT:
            if isinstance(item, str):
                rows.append(f"  {n:>4} {item}")
                n += 1
                continue
            i, blk = item, self.blocks[item]
            c = "▏" if i == self.cur else " "
            if blk is None:
                head = f"{c} ── conflict {i+1} of 2 · open ── o {YOURS}  t {THEIRS}  b both "
                rows.append((head + "─" * w)[: w - 1])
                rows.append(f"{c}      " + fit(YOURS, half) + "│      " + THEIRS)
                rows.append(f"{c} {n:>4} " + fit(NOTES[i]["ours"][0], half) + f"│ {n:>4} " + NOTES[i]["theirs"][0])
                n += 1
            else:
                head = f"{c} ── conflict {i+1} of 2 · took {self.took(blk)} "
                rows.append((head + "─" * w)[: w - 1])
                lines = {"ours": NOTES[i]["ours"], "theirs": NOTES[i]["theirs"],
                         "both": NOTES[i]["ours"] + NOTES[i]["theirs"]}[blk]
                for ln in lines:
                    rows.append(f"{c} {n:>4} {ln}")
                    n += 1
        return rows

    def resolver_rows(self, w):
        col = w >= 160
        colw = 30 if col else 0
        rw = w - colw - (1 if col else 0)
        body = [self.kind_row(), ""]
        if w >= 180:
            prow, _ = self.panes(rw)
            body += prow
        else:
            body += self.card(rw)
        if all(self.staged):
            body += ["", "  every file done ✓ · c finishes the merge"]
        elif self.file_open(self.file) == 0 and not self.staged[self.file]:
            body += ["", f"  all taken · ⏎ marks {FILES[self.file]} done"]
        if not col:
            return [self.list_strip(), "─" * w] + body
        lst = self.list_column()
        return [fit(lst[j] if j < len(lst) else "", colw) + "│" + (body[j] if j < len(body) else "")
                for j in range(max(len(lst), len(body)))]

    def resolver_bar(self, w=0):
        if self.file == 0:
            core = f" o keep, as {YOURS}  t delete, as {THEIRS}  ⏎ done"
        else:
            core = f" o {YOURS}  t {THEIRS}  b both  ⏎ done"
        wide = "  ^O/^T rest of file  O/T whole file  j/k file  n next  c finish  esc banner"
        narrow = "  O/T file  j/k  c finish"
        return core + (wide if len(core + wide) <= w else narrow)

    def caption(self):
        s = self.screen
        if s == "summary":
            if self.editing is not None:
                return "editing the commit message in place · next: ⏎ keeps it"
            if self.srow >= 0:
                return f"cursor on {self.auto[self.srow][0]} (combined by git) · next: ⏎ opens its diff"
            what = "2 files git combined, j/k + ⏎ opens one" if self.auto else "git combined none — shown honestly"
            return f"finish summary: your choices per file, {what} · next: ⏎ commits"
        if s == "autodiff":
            return "diff of a file git combined on its own · next: Esc back to the summary"
        if s == "merged":
            return "merged: cursor on the new merge commit, u undo in the bar · done"
        if s != "resolver":
            if s == "preview" and self.sel == 0:
                return "preview, cursor on Cancel · next: m (P1)"
            return super().caption()
        f = FILES[self.file]
        if all(self.staged):
            return "list shows both files done ✓ · next: c"
        if self.file == 0 and self.gone:
            return f"took main (yours) in {f}; both sides marked · next: ⏎ marks it done"
        if self.file == 0:
            return f"stop lands in the resolver (P5); list ▸ on {f} · next: O (keep whole file as {YOURS}) or o ⏎"
        if self.blocks == [None, None]:
            return f"list ▸ moved to {f}, gone.txt shows done ✓ · next: T (whole file) or t t ⏎"
        if all(self.blocks):
            return "both blocks taken, result pane filled · next: ⏎"
        return "took block 1, cursor advanced to block 2 (P2) · next: t"

    def note_count(self):
        if self.start == "shell":
            return "launch ⏎ 1 + gone O 1 + notes T 1 + finish c ⏎ 2 = 5 (v1 in-spec 7 + shell, v1 proposed 6)"
        return "merge m m 2 + gone O 1 + notes T 1 + finish c ⏎ 2 = 6 (v1 in-spec 11, v1 proposed 7)"


def pick_size():
    try:
        c, r = os.get_terminal_size()
    except OSError:
        return 120, 40
    # Draw at the real size: a frame taller than the window scrolls its top rows away.
    return c, r


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
    walk = make(args.layout, args.proposed, args.start, args.auto)
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    sys.stdout.write("\x1b[?1049h\x1b[?25l")
    try:
        tty.setraw(fd)
        while True:
            w, h = pick_size()  # follow resizes
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


JB_SCRIPTS = [
    ("jb A, whole-file route", "app", ["m", "m", "O", "T", "c"]),
    ("jb A, per-conflict route", "app", ["m", "m", "o", "\r", "t", "t", "\r", "c"]),
    ("jb A, out of order", "app", ["m", "m", "j", "\x14", "\r", "O", "c"]),
    ("jb B, shell-begun", "shell", ["\r", "O", "T", "c"]),
]


def make(layout, proposed, start, auto=False):
    return JbWalk(start, auto) if layout == "jb" else Walk(proposed, start)


def run_script(out, walk, name, keys, w, h):
    out.append("#" * min(w, 120))
    out.append(f"# RUN: {name} at {w}x{h}   (keys: {' '.join(kname(k) for k in keys)})")
    out.append("#" * min(w, 120))
    step = 0
    while True:
        out.append(f"=== PROTOTYPE {name} · {w}x{h} · frame {step} · {walk.caption()}")
        out += [ln.rstrip() for ln in walk.frame(w, h)]
        if step >= len(keys):
            break
        walk.press(keys[step])
        step += 1
    out += ["", *("# " + s for s in walk.summary()), ""]


def capture(path):
    out = [
        "# M6 conflict journey — PROTOTYPE captures (walk.py --capture), 120x40",
        "# Wizard-of-Oz: canned frames, no git. Task: \"merge `feature` into `main` and get it finished.\"",
        "# Colour: none. These are plain text; reverse video / colour is not prototyped and not verifiable here.",
        "# Each frame sits under one line: what you should see / what key next.",
        "",
    ]
    out.append("# v2 --layout jb (default): file list always on screen; 3 panes at >=180 cols, card below.")
    out.append("# Sizes: 200x50 (list column + 3 panes) and 120x40 (list strip + card). First row title, last row keys.")
    out.append("")
    for w, h in ((200, 50), (120, 40)):
        for name, start, keys in JB_SCRIPTS:
            if w == 120 and "out of order" in name:
                continue
            run_script(out, JbWalk(start), name, keys, w, h)
    out.append("# v1 --layout card (unchanged behaviour), 120x40")
    out.append("")
    for name, prop, start, keys in SCRIPTS:
        run_script(out, Walk(prop, start), "v1 " + name, keys, 120, 40)
    with open(path, "w") as f:
        f.write("\n".join(out) + "\n")


SUMMARY_SCRIPTS = [
    ("jb finish summary", False, ["m", "m", "O", "T", "c", "e", "\x7f", "\x7f", "\x7f", "\x7f", "m", "a", "i", "n", "\r", "\r"]),
    ("jb finish summary --auto", True, ["m", "m", "O", "T", "c", "j", "\r", "\x1b", "\x1b", "c", "\r"]),
]


def capture_summary(path):
    out = ["", "# v2 --layout jb: FINISH SUMMARY (field note 2026-09-23, approved). PROTOTYPE.",
           "# c opens one screen, not a yes/no box. ⏎ commits, e edits the message, Esc back to the resolver (state kept).",
           "# Without --auto the canned repo has nothing git combined on its own: `none`. --auto fakes 2 files.", ""]
    for w, h in ((200, 50), (120, 40)):
        for name, auto, keys in SUMMARY_SCRIPTS:
            walk = JbWalk("app", auto)
            for k in keys[:4]:  # skip to the resolver's last frame; earlier frames are above
                walk.press(k)
            run_script(out, walk, name, keys[4:], w, h)
    with open(path, "a") as f:
        f.write("\n".join(out) + "\n")


def main():
    ap = argparse.ArgumentParser(description="PROTOTYPE: M6 conflict journey walk")
    ap.add_argument("--layout", choices=["jb", "card"], default="jb", help="jb: file list + 3 panes (v2); card: v1")
    ap.add_argument("--proposed", action="store_true", help="card layout: turn on P1-P5 (jb always has them)")
    ap.add_argument("--start", choices=["app", "shell"], default="app")
    ap.add_argument("--auto", action="store_true", help="jb: fake 2 files git combined on its own (summary)")
    ap.add_argument("--capture", metavar="OUT")
    ap.add_argument("--capture-summary", metavar="OUT", help="append the finish-summary runs to OUT")
    args = ap.parse_args()
    if args.capture:
        capture(args.capture)
    elif args.capture_summary:
        capture_summary(args.capture_summary)
    else:
        interactive(args)


if __name__ == "__main__":
    main()
