#!/usr/bin/env python3
"""PROTOTYPE — throwaway. Deleted when the M7 specs land.

Pipeline step 5 for docs/design-notes/m7-power-tools.md. Wizard of Oz: every frame is canned, nothing
reads git.

Rev 2 (default): "Prototype brief rev 2" — A blame, B history, C journal, then one question.
Rev 1 (--rev1): "Prototype brief (step 5)" plus scene 4, the branch picker.

  python3 prototypes/m7-power-tools/power.py                    # rev 2, scene A first
  python3 prototypes/m7-power-tools/power.py --no-colour        # rev 2, colour off from the start
  python3 prototypes/m7-power-tools/power.py --rev1 --scene 4   # rev 1, scene 4

Piped (for checking it runs): keys on stdin, the last frame printed as plain text.
  printf 'Bjjj' | python3 prototypes/m7-power-tools/power.py --scene 1 --picks /tmp/p.txt

Prototype keys (not the app's): ] next scene · [ previous · Backspace restart the scene (outside a
text field) · digits answer a question when one is on screen · # (rev 2) colour on/off ·
~ (rev 1 scene 4) unsaved edits on/off · Ctrl-C quits anywhere; q quits outside a text field.
Every other key is the app's own.

Every key, every scene reached, every pause over 3 s and every answer is appended to
prototypes/m7-power-tools/picks.txt (gitignored) as it happens.

Rev 1 has no colour. Rev 2 draws colour in the live terminal, always on top of a glyph, rule, position or
weight that carries the same distinction; captures are plain text and never show colour (CLAUDE.md
rule 5). The cursor row is drawn reversed AND marked `▸`, so a plain-text capture still shows it.
"""
import argparse
import codecs
import os
import select
import signal
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
PICKS_DEFAULT = os.path.join(HERE, "picks.txt")
W, H = 187, 50

# ------------------------------------------------------------------ canvas

# Rev 2 draws in colour in the live terminal; captures (Canvas.lines) never carry it (CLAUDE.md rule 5).
# Every colour below sits on top of a glyph, rule or position that says the same thing without it.
COLOUR = [True]  # flipped by `#` or --no-colour: shows the non-colour carriers alone
WEIGHT = {"r": "7", "b": "1", "d": "2"}
HUE = {"mut": "38;5;244", "add": "32", "del": "31", "acc": "36", "graft": "36", "shell": "33", "lost": "35",
       "hash": "33", "ok": "32", "age0": "38;5;208", "age1": "38;5;178", "age2": "38;5;143", "age3": "38;5;109",
       "age4": "38;5;68", "age5": "38;5;61", "age6": "38;5;60"}


def sgr_of(at):
    parts = ["0"]
    for f in at.replace("rb", "r+b").split("+"):
        if f in WEIGHT:
            parts.append(WEIGHT[f])
        elif COLOUR[0] and f in HUE:
            parts.append(HUE[f])
    return ";".join(parts)



class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.ch = [[" "] * w for _ in range(h)]
        self.at = [[""] * w for _ in range(h)]

    def put(self, x, y, text, at=""):
        if 0 <= y < self.h:
            for i, c in enumerate(text):
                if 0 <= x + i < self.w:
                    self.ch[y][x + i] = c
                    self.at[y][x + i] = at
        return x + len(text)

    def attr(self, x, y, n, at):
        if 0 <= y < self.h:
            for i in range(max(0, x), min(self.w, x + n)):
                self.at[y][i] = at

    def box(self, x, y, w, h, title=""):
        self.put(x, y, "┌" + "─" * (w - 2) + "┐")
        for r in range(1, h - 1):
            self.put(x, y + r, "│" + " " * (w - 2) + "│")
        self.put(x, y + h - 1, "└" + "─" * (w - 2) + "┘")
        if title:
            self.put(x + 2, y, " " + title + " ", "b")

    def lines(self):
        return ["".join(r).rstrip() for r in self.ch]

    def add(self, x, y, n, flag):
        """Add one flag (e.g. reverse) to cells, keeping their colour."""
        if 0 <= y < self.h:
            for i in range(max(0, x), min(self.w, x + n)):
                a = self.at[y][i]
                self.at[y][i] = flag if not a else a + "+" + flag

    def tint(self, y, text, at, start=0):
        """Colour the first `text` found on row y (from column start)."""
        row = "".join(self.ch[y]) if 0 <= y < self.h else ""
        i = row.find(text, start)
        if i >= 0:
            self.add(i, y, len(text), at)

    def ansi(self):
        out = []
        sgr = sgr_of
        for y in range(self.h):
            out.append(f"\x1b[{y + 1};1H")
            last = None
            for x in range(self.w):
                a = self.at[y][x]
                if a != last:
                    out.append("\x1b[" + sgr(a) + "m")
                    last = a
                out.append(self.ch[y][x])
            out.append("\x1b[0m\x1b[K")
        return "".join(out)


def cut(s, n):
    return s if len(s) <= n else s[:max(0, n - 1)] + "…"


def draw_rows(cv, x, y, w, rows, cur):
    """rows: dicts with g (graph glyphs), sha, refs, subj, who, when; sel=False for connector rows.
    cur: index into the selectable rows. Cursor row: reversed and marked ▸."""
    si = 0
    for r in rows:
        if not r.get("sel", True):
            cv.put(x + 2, y, r["g"])
            y += 1
            continue
        on = si == cur
        line = (("▸ " if on else "  ") + f"{r['g']:<10}{r['sha']:<9}{cut(r['refs'], 44):<46}"
                f"{cut(r['subj'], 70):<72}{r['who']:<7}{r['when']}")
        cv.put(x, y, cut(line, w), "d" if r.get("dim") else "")
        if on:
            cv.attr(x, y, w, "r")
        si += 1
        y += 1
    return y


def selectable(rows):
    return [r for r in rows if r.get("sel", True)]


# ------------------------------------------------------------------ scenes


class Scene:
    name, title, task = "", "", ""

    def __init__(self, app):
        self.app = app
        self.reset()

    def reset(self):
        self.status = ""

    def text_mode(self):
        return False

    def question(self):
        return None  # (id, text, [opt1, opt2, opt3]) while one is on screen

    def answered(self, qid, n):
        pass

    def frame(self):
        return self.mode


# ---- scene 1: history editor ------------------------------------------------------------------

FA = [("7f6e5d4", "Handle expired tokens", "20m"), ("6e5d4c3", "wip", "1h"), ("5d4c3b2", "wip: tests", "2h"),
      ("4c3b2a1", "fixup! Add OAuth callback", "3h"), ("3b2a1f0", "Add OAuth callback", "4h")]
FB = [("e5f6a7b", "Show token expiry in settings", "1h"), ("d4e5f6a", "Settings page skeleton", "2h")]
NEW = ["9c8d7e6", "8b7c6d5", "7a6b5c4", "6f5e4d3", "5e4d3c2", "4d3c2b1", "3c2b1a0"]
ACT = {"p": "pick", "r": "reword ✎", "e": "edit ⏸", "s": "squash ↓", "f": "fixup ↓", "d": "drop ✗"}
DIFFS = {
    "7f6e5d4": ["src/auth/session.rs", "@@ -30,3 +30,7 @@", "+    if token.expired() {",
                "+        return self.refresh(token);", "+    }", "     Ok(token)"],
    "6e5d4c3": ["src/auth/session.rs", "@@ -12,1 +12,1 @@", "-    // TODO", "+    // TODO: expiry"],
    "5d4c3b2": ["tests/session.rs", "@@ +1,6 @@", "+#[test]", "+fn expired_token_refreshes() {", "+    todo!()", "+}"],
    "4c3b2a1": ["src/auth/callback.rs", "@@ -8,1 +8,1 @@", "-    let code = q.code;", "+    let code = q.code.trim();"],
    "3b2a1f0": ["src/auth/callback.rs", "@@ +1,12 @@", "+pub fn callback(q: Query) -> Result<Token> {", "+    …"],
}


class Tidy(Scene):
    name, title = "tidy", "Tidy your branch"
    task = ("Tidy your branch feature-a before review: squash the last three commits into one. "
            "Sara's branch feature-b is built on yours. Then do it again, but stop at 'wip: tests' to change its files.")

    def reset(self):
        self.status, self.mode, self.cur, self.ecur = "", "graph", 2, 0
        self.todo = [{"sha": s, "subj": t, "when": w, "act": "f" if t.startswith("fixup!") else "p"} for s, t, w in FA]
        self.diff = False
        self.asked = False

    def text_mode(self):
        return False

    def base_rows(self):
        rows = [dict(g="○", sha=s, refs="feature-b" if i == 0 else "", subj=t, who="Sara", when=w)
                for i, (s, t, w) in enumerate(FB)]
        rows += [dict(g="●" if i == 0 else "○", sha=s, refs="feature-a ← HEAD" if i == 0 else "", subj=t, who="you",
                      when=w) for i, (s, t, w) in enumerate(FA)]
        rows += [dict(g="│ ○", sha="2a1f0e9", refs="main", subj="Bump version to 2.4", who="Omar", when="1d"),
                 dict(g="├─╯", sel=False),
                 dict(g="○", sha="1f0e9d8", refs="", subj="Release 2.3", who="Omar", when="2d")]
        return rows

    def groups(self):
        """Oldest first. Each group is one commit after y. None if an s/f has nothing below it."""
        gs = []
        for row in reversed(self.todo):
            if row["act"] == "d":
                continue
            if row["act"] in "sf":
                if not gs:
                    return None
                gs[-1].append(row)
            else:
                gs.append([row])
        return gs

    def group_subj(self, g):
        base = g[0]["subj"]
        if len(g) == 1:
            return base + ("   (new message)" if g[0]["act"] == "r" else "")
        melded = [r for r in g[1:]]
        if all(r["act"] == "f" for r in melded):
            return base + "   (fixup melded in)"
        return f"{base} + " + " + ".join(r["subj"] for r in melded if r["act"] == "s") + f"   ({len(g)} in one)"

    def result_rows(self, gs):
        rows, n = [], 0
        for i, (s, t, w) in enumerate(FB):
            rows.append(dict(g="○", sha=NEW[n], refs="feature-b" if i == 0 else "", subj=t, who="Sara", when="now"))
            n += 1
        for gi, g in enumerate(reversed(gs)):
            rows.append(dict(g="●" if gi == 0 else "○", sha=NEW[n], refs="feature-a ← HEAD" if gi == 0 else "",
                             subj=self.group_subj(g), who="you", when="now"))
            n += 1
        rows += [dict(g="│ ○", sha="2a1f0e9", refs="main", subj="Bump version to 2.4", who="Omar", when="1d"),
                 dict(g="├─╯", sel=False),
                 dict(g="○", sha="1f0e9d8", refs="", subj="Release 2.3", who="Omar", when="2d")]
        return rows

    def question(self):
        if self.mode == "landed" and not self.asked:
            return ("s1-feature-b-follows", "feature-b is Sara's. It moved on top of your new commits. Is that what you want?",
                    ["yes, move branches built on mine", "ask me each time", "never move a teammate's branch"])
        return None

    def answered(self, qid, n):
        self.asked = True

    def key(self, k):
        a = self.app
        if self.mode in ("graph", "landed"):
            rows = self.base_rows() if self.mode == "graph" else self.result_rows(self.groups())
            n = len(selectable(rows))
            if k in ("j", "down"):
                self.cur = min(n - 1, self.cur + 1)
            elif k in ("k", "up"):
                self.cur = max(0, self.cur - 1)
            elif k == "R" and self.mode == "graph":
                self.mode, self.ecur, self.diff = "editor", 0, False
            elif k == "R":
                a.flash("already rewritten here · Backspace restarts the scene")
            elif k == "u" and self.mode == "landed":
                self.reset()
                self.status = "undone: feature-a and feature-b back where they were"
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "editor":
            if self.diff:
                if k in ("enter", "esc"):
                    self.diff = False
                    return
            if k in ("j", "down"):
                self.ecur = min(len(self.todo) - 1, self.ecur + 1)
            elif k in ("k", "up"):
                self.ecur = max(0, self.ecur - 1)
            elif k in ACT:
                self.todo[self.ecur]["act"] = k
                self.ecur = min(len(self.todo) - 1, self.ecur + 1)
            elif k in ("alt-up", "alt-down"):
                d = -1 if k == "alt-up" else 1
                j = self.ecur + d
                if 0 <= j < len(self.todo):
                    self.todo[self.ecur], self.todo[j] = self.todo[j], self.todo[self.ecur]
                    self.ecur = j
            elif k == "enter":
                self.diff = True
            elif k == "esc":
                self.reset()
                self.status = "history editor closed · nothing changed"
            elif k == "y":
                gs = self.groups()
                if gs is None:
                    a.flash("the oldest row has nothing below it to meld into · mark it pick")
                    return
                if any(r["act"] == "e" for r in self.todo):
                    self.mode = "stop"
                else:
                    self.land()
            else:
                a.flash(f"{k}: not a history-editor key")
        elif self.mode == "stop":
            if k == "c":
                self.land()
            elif k == "a":
                a.flash("amended the stopped commit (pretend) · c continues")
            else:
                a.flash(f"{k}: not in this prototype")

    def land(self):
        gs = self.groups()
        moved = sum(1 for r in self.todo if r["act"] != "d")
        self.mode, self.cur = "landed", len(FB)
        self.status = (f"rebased {moved} commits · feature-a is {len(gs)} commits now · "
                       f"feature-b moved with it (built on it) · u undoes")

    def frame(self):
        return self.mode + ("+diff" if self.diff else "")

    def draw(self, cv, top, bot):
        x = 1
        if self.mode in ("graph", "landed"):
            rows = self.base_rows() if self.mode == "graph" else self.result_rows(self.groups())
            cv.put(x, top, "Graph" + ("  ·  after the rewrite" if self.mode == "landed" else ""), "b")
            draw_rows(cv, x, top + 2, W - 2, rows, self.cur)
            if self.mode == "graph":
                return "⏎ open  c commit  a amend  R rewrite  m merge  r rebase  b branch  x compare  J journal  . more"
            return "⏎ open  u undo  c commit  a amend  R rewrite  m merge  r rebase  b branch  . more"
        if self.mode == "editor":
            lw = 92
            cv.put(x, top, "REWRITE feature-a  ·  5 commits since it left main  ·  newest first", "b")
            y = top + 2
            for i, r in enumerate(self.todo):
                on = i == self.ecur
                act = ACT[r["act"]]
                tail = ""
                if r["act"] in "sf":
                    tail = "melds into the row below"
                line = f"{'▸ ' if on else '  '}{act:<11}{r['sha']:<9}{cut(r['subj'], 38):<40}{tail}"
                cv.put(x, y, line, "d" if r["act"] == "d" else "")
                if on:
                    cv.attr(x, y, lw, "r")
                y += 1
            y += 1
            cv.put(x, y, "built on it:  feature-b (Sara, 2 commits) moves too")
            y += 2
            cv.put(x, y, "↓ = melds into the row below  ·  ⏸ = stops there so you can change files  ·  ✗ = left out", "d")
            rx = 96
            if self.diff:
                r = self.todo[self.ecur]
                cv.put(rx, top, f"CHANGES IN {r['sha']}  {cut(r['subj'], 50)}", "b")
                for i, ln in enumerate(DIFFS[r["sha"]]):
                    cv.put(rx, top + 2 + i, ln)
                return "⏎ close  esc close"
            cv.put(rx, top, "AFTER y  ·  what the branches will be  ·  not written yet", "b")
            gs = self.groups()
            y = top + 2
            if gs is None:
                cv.put(rx, y, "✗ the oldest row can't meld: nothing below it")
            else:
                n = 0
                for i, (s, t, w) in enumerate(FB):
                    lab = "feature-b" if i == 0 else ""
                    cv.put(rx, y, f"after  {lab:<11}○ new  {cut(t, 40):<42}moves: built on yours")
                    y += 1
                    n += 1
                for gi, g in enumerate(reversed(gs)):
                    lab = "feature-a" if gi == 0 else ""
                    stop = "  ⏸ stops here" if any(r["act"] == "e" for r in g) else ""
                    cv.put(rx, y, cut(f"after  {lab:<11}{'●' if gi == 0 else '○'} new  {self.group_subj(g)}{stop}", 90))
                    y += 1
                cv.put(rx, y, f"{'':7}{'main':<11}○ 2a1f0e9  Bump version to 2.4   (unchanged)")
                y += 1
                dropped = [r for r in self.todo if r["act"] == "d"]
                if dropped:
                    y += 1
                    cv.put(rx, y, "left out: " + ", ".join(r["subj"] for r in dropped), "d")
            return ("y rewrite  ⏎ changes  p pick  r reword  e edit  s squash  f fixup  d drop  "
                    "alt+↑↓ move  j/k row  esc cancel")
        if self.mode == "stop":
            gs = self.groups()
            order = [r for r in reversed(self.todo)]
            eidx = next(i for i, r in enumerate(order) if r["act"] == "e")
            total = len(order) + len(FB)
            er = order[eidx]
            banner = (f" REBASING  step {eidx + 1} of {total}  ·  edit {er['sha']} {er['subj']}  ·  "
                      "change files, stage them, a amends it  ·  c continues  ·  the banner's a y aborts ")
            cv.put(x, top, banner, "r")
            # applied groups: those up to and including the one holding the e row
            applied, pending = [], []
            seen = False
            for g in gs:
                (pending if seen else applied).append(g)
                if any(r is er for r in g):
                    seen = True
            y = top + 2
            bw = 100
            cv.box(x, y, bw, len(pending) + 3, "still to apply  ·  not commits yet")
            yy = y + 1
            for g in reversed(pending):
                cv.put(x + 2, yy, f"pick   {g[0]['sha']}  {cut(self.group_subj(g), 70)}")
                yy += 1
            cv.put(x + 2, yy, "feature-b (Sara): 2 commits follow, on top")
            y += len(pending) + 4
            rows = []
            for gi, g in enumerate(reversed(applied)):
                head = gi == 0
                rows.append(dict(g="●" if head else "○", sha=NEW[gi], refs="HEAD · stopped here" if head else "",
                                 subj=self.group_subj(g), who="you", when="now"))
            rows += [dict(g="│ ○", sha="2a1f0e9", refs="main", subj="Bump version to 2.4", who="Omar", when="1d"),
                     dict(g="├─╯", sel=False),
                     dict(g="○", sha="1f0e9d8", refs="", subj="Release 2.3", who="Omar", when="2d")]
            draw_rows(cv, x, y, W - 2, rows, 0)
            return "a amend it  c continue  space stage  ⏎ open  . more"


# ---- scene 2: blame -----------------------------------------------------------------------------

COMMITS = {  # sha: (author, date, subject), oldest first
    "0f1e2d3": ("Omar", "2025-06-04", "Session store"),
    "5e6f7a8": ("Tom", "2025-11-19", "Add token refresh"),
    "a1b2c3d": ("Lina", "2026-03-02", "Retry refresh once on 401"),
    "c4d5e6f": ("Priya", "2026-05-21", "Move session TTL to config"),
    "b8c9d0e": ("Sara", "2026-07-14", "Log session ids at debug level"),
    "7f6e5d4": ("you", "2026-09-26", "Handle expired tokens"),
}
ORDER = list(COMMITS)
O, T, L, P, S, Y = ORDER
SRC = [  # (commit, text); the function refresh_token is lines 18-27 (1-based)
    (O, "use std::time::Duration;"), (O, "use crate::store::Store;"), (S, "use tracing::debug;"), (O, ""),
    (O, "pub struct Session {"), (O, "    id: SessionId,"), (O, "    token: Token,"), (P, "    ttl: Duration,"),
    (O, "}"), (O, ""), (O, "impl Session {"), (O, "    pub fn new(id: SessionId, token: Token) -> Self {"),
    (P, "        let ttl = config::session_ttl();"), (O, "        Session { id, token, ttl }"), (O, "    }"), (O, ""),
    (T, "    /// Refresh the token when the server says it expired."),
    (T, "    pub fn refresh_token(&mut self, client: &Client) -> Result<()> {"),
    (T, "        let mut attempt = 0;"), (T, "        loop {"), (T, "            let resp = client.refresh(&self.token)?;"),
    (L, "            if resp.status() == 401 && attempt == 0 {"),
    (T, "                attempt += 1;"), (T, "                continue;"), (T, "            }"),
    (P, "            self.ttl = resp.ttl().unwrap_or(self.ttl);"),
    (Y, "            self.token = resp.token()?; return Ok(());"),
    (T, "        }"), (T, "    }"), (O, ""), (O, "    pub fn save(&self, store: &Store) -> Result<()> {"),
    (S, "        debug!(session = %self.id, \"saving session\");"), (O, "        store.put(&self.id, self)"),
    (O, "    }"), (O, ""), (Y, "    pub fn expired(&self) -> bool {"), (Y, "        self.token.age() > self.ttl"),
    (Y, "    }"), (O, "}"), (O, ""),
]
LINE22_OLD = "            if resp.status() == 401 {"


class Blame(Scene):
    name, title = "blame", "Why is this line like this?"
    task = ("Line 22 of session.rs looks odd to you. Why is it like this, who changed it and why — "
            "and what else has changed that whole function over time?")

    def reset(self):
        self.status, self.mode, self.line, self.level, self.lcur, self.asked = "", "diff", 22, 0, 0, False

    def view(self):
        """[(original line, shown number, commit, text)] for the blame at this level. Level 1 = before
        Lina's change. Lines added by later commits are left out at the older level."""
        cutoff = [len(ORDER), ORDER.index(L)][self.level]
        out = []
        for i, (c, t) in enumerate(SRC, 1):
            if i == 22 and self.level >= 1:
                c, t = T, LINE22_OLD
            if ORDER.index(c) < cutoff:
                out.append((i, c, t))
        return [(o, n, c, t) for n, (o, c, t) in enumerate(out, 1)]

    def cur_idx(self, v):
        return next((i for i, r in enumerate(v) if r[0] >= self.line), len(v) - 1)

    def question(self):
        if self.mode == "lines" and not self.asked:
            return ("s2-reach-today", "Today, when you want to know why a line is like this, what do you reach for?",
                    ["Zed's (or VS Code's) blame", "GitHub / GitLab in the browser", "git blame / git log in the shell"])
        return None

    def answered(self, qid, n):
        self.asked = True

    def key(self, k):
        a = self.app
        if self.mode == "diff":
            if k in ("j", "down"):
                self.line = min(len(SRC), self.line + 1)
            elif k in ("k", "up"):
                self.line = max(1, self.line - 1)
            elif k == "B":
                self.mode, self.level = "blame", 0
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "blame":
            v = self.view()
            i = self.cur_idx(v)
            if k in ("j", "down"):
                self.line = v[min(len(v) - 1, i + 1)][0]
            elif k in ("k", "up"):
                self.line = v[max(0, i - 1)][0]
            elif k == "enter":
                self.mode = "graph"
                self.gsha = v[i][2]
            elif k == "^":
                c = v[i][2]
                if self.level == 0 and c == L:
                    self.level = 1
                    self.status = "blame before a1b2c3d (Lina's change): the file as Tom left it"
                elif self.level == 1 and c == T and self.line == 22:
                    self.status = "this line did not exist before 5e6f7a8 — Tom added it with the whole function"
                else:
                    a.flash("^ is acted out for line 22 only in this prototype")
            elif k == "L":
                self.mode, self.lcur = "lines", 0
            elif k == "esc":
                if self.level:
                    self.level, self.status = 0, ""
                else:
                    self.mode = "diff"
            elif k in ("n", "N"):
                a.flash(f"{k}: older/newer change to this line — not in this prototype")
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "graph":
            if k == "esc":
                self.mode = "blame"
            else:
                a.flash(f"{k}: not in this prototype · esc goes back to the blame")
        elif self.mode == "lines":
            if k in ("j", "down"):
                self.lcur = min(3, self.lcur + 1)
            elif k in ("k", "up"):
                self.lcur = max(0, self.lcur - 1)
            elif k == "esc":
                self.mode = "blame"
            else:
                a.flash(f"{k}: not in this prototype")

    def draw(self, cv, top, bot):
        x = 1
        if self.mode == "diff":
            cv.put(x, top, "CHANGES  ·  src/auth/session.rs  ·  in 7f6e5d4 Handle expired tokens (you, today)", "b")
            room = bot - top - 2
            off = max(0, min(self.line - room // 2, len(SRC) - room))
            for i, (c, t) in enumerate(SRC[off:off + room], off + 1):
                mark = "+" if c == Y else " "
                on = i == self.line
                yy = top + 1 + i - off
                cv.put(x, yy, f"{'▸' if on else ' '} {i:>3} {mark} {t}")
                if on:
                    cv.attr(x, yy, 110, "r")
            return "⏎ open file  B blame  space stage  e editor  w whitespace  v side by side  esc back"
        if self.mode == "blame":
            v = self.view()
            ci = self.cur_idx(v)
            hdr = ["BLAME  ·  src/auth/session.rs  ·  as it is now",
                   "BLAME  ·  src/auth/session.rs  ·  before a1b2c3d (Lina's change)"][self.level]
            cv.put(x, top, hdr, "b")
            prev = None
            room = bot - top - 2
            off = max(0, min(ci - room // 2, len(v) - room))
            for i, (o, n, c, t) in enumerate(v):
                if not off <= i < off + room:
                    continue
                au, d, subj = COMMITS[c]
                on = i == ci
                who = f"{c} {au:<6} {d}  {cut(subj, 30):<31}" if c != prev or on else f"{'':>7} {'':<6} {'·':>10}  {'':<31}"
                prev = c
                y = top + 2 + i - off
                cv.put(x, y, f"{'▸' if on else ' '} {who}│{n:>4}  {t}")
                if on:
                    cv.attr(x, y, 150, "r")
                    prev = None
            return "⏎ that commit  ^ before it  L history of these lines  n older  N newer  space ages  esc back"
        if self.mode == "graph":
            c = self.gsha
            au, d, subj = COMMITS[c]
            cv.put(x, top, "Graph  ·  cursor on the commit that last changed line 22", "b")
            rows = [dict(g="○", sha=s, refs="", subj=COMMITS[s][2], who=COMMITS[s][0], when=COMMITS[s][1])
                    for s in reversed(ORDER)]
            draw_rows(cv, x, top + 2, W - 2, rows, list(reversed(ORDER)).index(c))
            y = top + 10
            cv.put(x, y, f"CHANGES IN {c}  {subj}  ·  {au}, {d}", "b")
            diff = ["src/auth/session.rs", "@@ -19,7 +19,7 @@", "         loop {",
                    "             let resp = client.refresh(&self.token)?;",
                    "-            if resp.status() == 401 {", "+            if resp.status() == 401 && attempt == 0 {",
                    "                 attempt += 1;", "", "Message:", "    Retry refresh once on 401",
                    "    The server sends 401 for a token that expired in flight. One retry, then give up,",
                    "    or we loop forever when the refresh token itself is revoked."]
            for i, ln in enumerate(diff):
                cv.put(x + 2, y + 2 + i, ln)
            return "esc back to the blame  ⏎ open  x compare  . more"
        if self.mode == "lines":
            cv.put(x, top, "HISTORY OF LINES 18–29  ·  fn refresh_token  ·  src/auth/session.rs", "b")
            cv.put(x, top + 1, "4 commits changed these lines · every other commit is hidden (none of them touch these lines)")
            hist = [(Y, "+1 −1"), (P, "+1 −0"), (L, "+1 −1"), (T, "+13 −0")]
            rows = [dict(g="○", sha=s, refs="", subj=f"{COMMITS[s][2]}", who=COMMITS[s][0],
                         when=f"{COMMITS[s][1]}   {st} in these lines" + ("  (added the function)" if s == T else "")) for s, st in hist]
            draw_rows(cv, x, top + 3, W - 2, rows, self.lcur)
            s = hist[self.lcur][0]
            y = top + 9
            cv.put(x, y, f"THOSE LINES IN {s}  {COMMITS[s][2]}", "b")
            body = {Y: ["-            self.token = resp.token()?;", "+            self.token = resp.token()?; return Ok(());"],
                    P: ["+            self.ttl = resp.ttl().unwrap_or(self.ttl);"],
                    L: ["-            if resp.status() == 401 {", "+            if resp.status() == 401 && attempt == 0 {"],
                    T: ["+    pub fn refresh_token(&mut self, client: &Client) -> Result<()> {", "+        let mut attempt = 0;",
                        "+        …"]}[s]
            for i, ln in enumerate(body):
                cv.put(x + 2, y + 2 + i, ln)
            return "j/k commit  ⏎ open  esc back to the blame"


# ---- scene 3: journal + reflog -------------------------------------------------------------------

JROWS = [  # (graft?, time, words, git's own subject or "", lost line or "", branchable (sha, name) or None)
    (True, "10:44", "fetch · origin/main moved 2 commits", "", "", None),
    (False, "10:41", "reset main 1 commit back", 'git: "reset: moving to HEAD~1"',
     "9a8b7c6 Add rate limiter — no branch holds it now", ("9a8b7c6", "rate-limiter")),
    (False, "10:37", "switch from spike to main", 'git: "checkout: moving from spike to main"', "", ("4e5f6a7", "spike")),
    (False, "10:35", 'commit on spike: "Try redis cache"', 'git: "commit: Try redis cache"', "", ("4e5f6a7", "spike")),
    (True, "10:12", "switch from main to spike", "", "", ("3d4e5f6", "spike")),
    (True, "10:05", 'commit on main: "Add rate limiter"', "", "", ("9a8b7c6", "rate-limiter")),
    (True, "09:58", "pull main · 3 commits", "", "", None),
]


class Journal(Scene):
    name, title = "journal", "Lost a commit in the shell"
    task = ("You ran git reset --hard in the shell and lost your 'Add rate limiter' commit. Get it back. "
            "Also: where did your spike branch go? You deleted it in the shell by mistake. Bring it back too.")

    def reset(self):
        self.status, self.mode, self.cur, self.lcur = "", "graph", 0, 0
        self.filt, self.typing, self.bname = "", False, ""
        self.restored, self.spike, self.asked = False, False, False

    def text_mode(self):
        return self.typing or self.mode == "branch"

    def rows(self):
        return [r for r in JROWS if self.filt.lower() in (r[2] + " " + r[3]).lower()]

    def graph_rows(self):
        rows = []
        if self.spike:
            rows.append(dict(g="│ ○", sha="4e5f6a7", refs="spike", subj="Try redis cache", who="you", when="10:35"))
        if self.restored:
            rows.append(dict(g="●" if not self.spike else "● │", sha="9a8b7c6", refs="main ← HEAD",
                             subj="Add rate limiter", who="you", when="10:05"))
        if self.spike:
            rows.append(dict(g="├─╯", sel=False))
        rows.append(dict(g="○" if self.restored else "●", sha="3d4e5f6",
                         refs="origin/main" + ("" if self.restored else "  main ← HEAD"),
                         subj="Add metrics endpoint", who="you", when="1d"))
        rows.append(dict(g="○", sha="2c3d4e5", refs="", subj="Bump deps", who="Omar", when="2d"))
        return rows

    def question(self):
        if self.mode == "graph" and (self.restored or self.spike) and not self.asked:
            return ("s3-u-expected", "Before you opened the list: did you expect u (undo) to undo the reset you ran in the shell?",
                    ["yes", "no", "I didn't think of undo"])
        return None

    def answered(self, qid, n):
        self.asked = True

    def u_says(self):
        self.app.flash("u undoes Graft's own last step: fetch (10:44). The reset ran in the shell — "
                       "u never undoes shell steps; ⏎ on its row puts it back")

    def key(self, k):
        a = self.app
        if self.mode == "graph":
            if k == "J":
                self.mode, self.lcur = "list", 0
            elif k in ("j", "down"):
                self.cur = min(len(selectable(self.graph_rows())) - 1, self.cur + 1)
            elif k in ("k", "up"):
                self.cur = max(0, self.cur - 1)
            elif k == "u":
                if self.restored or self.spike:
                    a.flash("u would undo your last step here (not acted out)")
                else:
                    self.u_says()
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "list":
            rs = self.rows()
            if self.typing:
                if k == "enter":
                    self.typing = False
                elif k == "esc":
                    self.typing, self.filt = False, ""
                elif k == "bs":
                    self.filt = self.filt[:-1]
                elif len(k) == 1 and k.isprintable():
                    self.filt += k
                self.lcur = 0
                return
            if k in ("j", "down"):
                self.lcur = min(len(rs) - 1, self.lcur + 1)
            elif k in ("k", "up"):
                self.lcur = max(0, self.lcur - 1)
            elif k == "/":
                self.typing, self.filt = True, ""
            elif k == "enter" and rs:
                self.mode = "preview"
            elif k == "b" and rs:
                r = rs[self.lcur]
                if r[5] is None:
                    a.flash("nothing to branch from on this row: no commit of yours moved")
                else:
                    self.mode, self.bname = "branch", r[5][1]
            elif k == "u":
                self.u_says()
            elif k in ("esc", "J"):
                if self.filt:
                    self.filt = ""
                else:
                    self.mode = "graph"
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "preview":
            r = self.rows()[self.lcur]
            if k == "y":
                if r[4]:
                    self.restored, self.mode, self.cur = True, "graph", 0
                    self.status = "main back at 9a8b7c6 Add rate limiter · u undoes this"
                else:
                    self.mode = "list"
                    a.flash("acted out only for the reset row in this prototype")
            elif k == "esc":
                self.mode = "list"
            else:
                a.flash(f"{k}: y puts it back, esc cancels")
        elif self.mode == "branch":
            if k == "enter":
                sha = self.rows()[self.lcur][5][0]
                self.mode, self.cur = "graph", 0
                if self.bname == "spike" and sha == "4e5f6a7":
                    self.spike = True
                self.status = f"branch {self.bname} made at {sha} · u undoes this"
                self.filt = ""
            elif k == "esc":
                self.mode = "list"
            elif k == "bs":
                self.bname = self.bname[:-1]
            elif len(k) == 1 and k.isprintable():
                self.bname += k

    def frame(self):
        return self.mode + ("+typing" if self.typing else "") + (f" filter={self.filt!r}" if self.filt else "")

    def draw(self, cv, top, bot):
        x = 1
        cv.put(x, top, "Graph", "b")
        draw_rows(cv, x, top + 2, W - 2, self.graph_rows(), self.cur if self.mode == "graph" else -1)
        if self.mode == "graph":
            return "J journal  u undo  ⏎ open  c commit  b branch  m merge  r rebase  . more"
        bx, by, bw = 3, top + 8, 181
        rs = self.rows()
        bh = len(JROWS) + 9
        cv.box(bx, by, bw, bh, "JOURNAL  ·  what happened in this repo, newest first  ·  Graft's steps and the shell's")
        y = by + 2
        if self.filt or self.typing:
            cv.put(bx + 3, y, f"find: {self.filt}{'▏' if self.typing else ''}")
            y += 2
        for i, r in enumerate(rs):
            g, t, words, sub, lost, _ = r
            tag = "●  Graft         " if g else "$  outside Graft "
            on = i == self.lcur
            line = f"{'▸ ' if on else '  '}{tag} {t}   {words:<44}{sub}"
            cv.put(bx + 2, y, line)
            if on:
                cv.attr(bx + 2, y, bw - 4, "r")
            y += 1
            if lost:
                cv.put(bx + 2, y, f"{'':<28}✗ lost  {lost}", "d")
                y += 1
        y = by + bh - 2
        cv.put(bx + 3, y, "●  a step done in Graft: u can undo it   ·   $  done outside Graft (the shell, an editor): "
                          "⏎ can put it back, u never touches it   ·   ✗  a commit no branch holds", "d")
        if self.mode == "preview":
            r = rs[self.lcur]
            px, py = bx, by + bh + 1
            lines = (["Put main back where it was before this reset:", "",
                      "    main   3d4e5f6 Add metrics endpoint   →   9a8b7c6 Add rate limiter",
                      "    brings back 1 commit: 9a8b7c6 Add rate limiter", "",
                      "Your files: nothing unsaved, nothing is touched.",
                      "Can't come back: edits that were never in a commit when the reset ran. git never had them."]
                     if r[4] else ["Put things back to just before this row.", "", f"    {r[2]}", "",
                                   "(acted out only for the reset row in this prototype)"])
            cv.box(px, py, 100, len(lines) + 5, "PUT BACK")
            for i, ln in enumerate(lines):
                cv.put(px + 3, py + 2 + i, ln)
            cv.put(px + 3, py + len(lines) + 3, "y put it back    esc cancel", "b")
            return "y put it back  esc cancel"
        if self.mode == "branch":
            sha, _ = rs[self.lcur][5]
            px, py = bx, by + bh + 1
            cv.box(px, py, 100, 6, "NEW BRANCH")
            cv.put(px + 3, py + 2, f"at {sha} {'Try redis cache' if sha == '4e5f6a7' else ''}   name: {self.bname}▏")
            cv.put(px + 3, py + 4, "⏎ make it    esc cancel", "b")
            return "type a name  ⏎ make it  esc cancel"
        if self.typing:
            return "type to find  ⏎ keep  esc clear"
        return "⏎ put back to here  b branch here  / find  j/k row  esc close"


# ---- scene 4: branch picker ----------------------------------------------------------------------

BRANCHES = [  # (name, remote?, who, when, last subject)
    ("feature-a", False, "you", "20m", "Handle expired tokens"),
    ("main", False, "Omar", "1d", "Merge branch 'omar/cache'"),
    ("feature-b", False, "Sara", "1h", "Show token expiry in settings"),
    ("spike", False, "you", "3d", "Try redis cache"),
    ("release/2.3", False, "Omar", "2w", "Release 2.3"),
    ("origin/sara/login-page", True, "Sara", "1h", "Login page: remember me box"),
    ("origin/sara/fix-logout", True, "Sara", "3h", "Fix null session on logout"),
    ("origin/main", True, "Omar", "1d", "Merge branch 'omar/cache'"),
    ("origin/feature-b", True, "Sara", "1h", "Show token expiry in settings"),
    ("origin/omar/rate-limits", True, "Omar", "2d", "Rate limit per API key"),
    ("origin/omar/cache", True, "Omar", "1d", "Cache session lookups"),
    ("origin/lina/retry-401", True, "Lina", "5d", "Retry refresh once on 401"),
    ("origin/tom/settings-v2", True, "Tom", "1w", "Settings v2: split pages"),
    ("origin/release/2.3", True, "Omar", "2w", "Release 2.3"),
    ("origin/dev", True, "Priya", "4d", "Nightly: bump deps"),
]


def fuzzy(q, name):
    """None if q is not a subsequence of name; else a sort key (substring first, then compactness)."""
    if not q:
        return (0, 0)
    q, n = q.lower(), name.lower()
    if q in n:
        return (0, n.index(q))
    i, first, last = 0, None, None
    for j, c in enumerate(n):
        if i < len(q) and c == q[i]:
            first = j if first is None else first
            last = j
            i += 1
    if i < len(q):
        return None
    return (1, last - first)


class Picker(Scene):
    name, title = "picker", "Switch branch, take one commit"
    task = ("Switch to Sara's login-page branch to try it. Then go back to feature-a and take Sara's "
            "'Fix null session on logout' commit onto it.")

    def reset(self):
        self.status, self.mode, self.cur = "", "graph", 5
        self.head, self.prev, self.q, self.sel = "feature-a", "main", "", 0
        self.local_login, self.taken, self.dirty = False, False, False
        self.pending, self.asked = None, set()
        self.qpending = None

    def text_mode(self):
        return self.mode == "picker"

    def listed(self):
        """Empty filter: the branch you were on last first, then local, then remote. Typed: fuzzy order."""
        bs = list(BRANCHES)
        if self.local_login:
            bs.insert(1, ("sara/login-page", False, "Sara", "1h", "Login page: remember me box"))
        if not self.q:
            prev = [b for b in bs if b[0] == self.prev]
            return prev + [b for b in bs if b[0] != self.prev]
        scored = [(fuzzy(self.q, b[0]), i, b) for i, b in enumerate(bs)]
        return [b for s, i, b in sorted((s, i, b) for s, i, b in scored if s is not None)]

    def graph_rows(self):
        def refs(*names):
            out = []
            for n in names:
                if n:
                    out.append(n + (" ← HEAD" if n == self.head else ""))
            return "  ".join(out)
        fa_head = self.head == "feature-a"
        rows = [dict(g="○" if self.head != "sara/fix-logout" else "●", sha="c0ffee1", refs=refs("origin/sara/fix-logout"),
                     subj="Fix null session on logout", who="Sara", when="3h"),
                dict(g="○", sha="b1ade02", refs="", subj="Log session ids at debug level", who="Sara", when="4h"),
                dict(g="│ " + ("●" if self.head == "sara/login-page" else "○"), sha="a11ce03",
                     refs=refs("sara/login-page" if self.local_login else "", "origin/sara/login-page"),
                     subj="Login page: remember me box", who="Sara", when="1h"),
                dict(g="│ ○", sha="a11ce02", refs="", subj="Login page layout", who="Sara", when="5h")]
        rows.append(dict(g="│ │ ◆", sha="2a1f0e9", refs=refs("main", "origin/main"),
                         subj="Merge branch 'omar/cache'   (merge)", who="Omar", when="1d"))
        if self.taken:
            rows.append(dict(g="│ │ │ ●" if fa_head else "│ │ │ ○", sha="d15ea5e", refs=refs("feature-a"),
                             subj="Fix null session on logout   (taken from origin/sara/fix-logout)", who="you", when="now"))
        rows += [dict(g="│ │ │ " + ("●" if fa_head and not self.taken else "○"), sha="7f6e5d4",
                      refs="" if self.taken else refs("feature-a"), subj="Handle expired tokens", who="you", when="20m"),
                 dict(g="│ │ │ ○", sha="3b2a1f0", refs="", subj="Add OAuth callback", who="you", when="4h"),
                 dict(g="├─┴─┴─╯", sel=False),
                 dict(g="○", sha="1f0e9d8", refs="", subj="Release 2.3", who="Omar", when="2d")]
        return rows

    def row_of(self, branch):
        local = branch.replace("origin/", "")
        for i, r in enumerate(selectable(self.graph_rows())):
            names = r["refs"].replace(" ← HEAD", "").split()
            if branch in names or local in names:
                return i
        return None

    def question(self):
        if self.qpending and self.qpending not in self.asked:
            if self.qpending == "s4-take-head":
                return ("s4-take-head", "This commit is already in your branch (the one you're on, or one before it). What should 'take it onto yours' do here?",
                        ["say no, in words: it's already on your branch", "hide the key here", "copy it on top anyway"])
            return ("s4-take-merge", "This is a merge: it brought omar/cache's 3 commits into main. Taking it onto yours should…",
                    ["say no, in words: take one of its commits instead", "take everything it brought, as one new commit",
                     "take its 3 commits, one by one"])
        return None

    def answered(self, qid, n):
        self.asked.add(qid)
        self.qpending = None

    def switch(self, b):
        name = b[0]
        local = name[len("origin/"):] if name.startswith("origin/") else name
        made = ""
        if name.startswith("origin/") and not any(x[0] == local and not x[1] for x in BRANCHES):
            if local == "sara/login-page":
                self.local_login = True
            made = f" (new, follows {name})"
        elif name.startswith("origin/"):
            made = f" (your local copy; it follows {name})"
        self.prev, self.head = self.head, local
        drawn = "" if self.row_of(local) is not None else " · not drawn in this prototype's graph"
        self.status = (f"on {local}{made} · u goes back to {self.prev}"
                       + (" · your 2 unsaved files came along" if self.dirty else "") + drawn)
        r = self.row_of(local)
        self.cur = r if r is not None else self.cur
        self.mode = "graph"

    def key(self, k):
        a = self.app
        if k == "~" and self.mode != "picker":
            self.dirty = not self.dirty
            a.flash("your tree now has " + ("2 unsaved files" if self.dirty else "no unsaved edits"))
            return
        if self.mode == "graph":
            rows = selectable(self.graph_rows())
            if k in ("j", "down"):
                self.cur = min(len(rows) - 1, self.cur + 1)
            elif k in ("k", "up"):
                self.cur = max(0, self.cur - 1)
            elif k == "C":
                self.mode, self.q, self.sel = "picker", "", 0
            elif k == "o":
                r = rows[self.cur]
                mine = {"feature-a": {"d15ea5e", "7f6e5d4", "3b2a1f0", "1f0e9d8"},
                        "sara/login-page": {"a11ce03", "a11ce02", "1f0e9d8"}}.get(self.head, set())
                if r["sha"] in mine:
                    self.qpending = "s4-take-head"
                    a.flash(f"already in {self.head}: nothing to take")
                elif r["sha"] == "2a1f0e9":
                    self.qpending = "s4-take-merge"
                    a.flash("a merge: not acted out · answer the question below")
                elif self.taken and r["sha"] == "c0ffee1":
                    a.flash("already taken: d15ea5e on feature-a has these changes")
                else:
                    self.pending, self.mode = r, "take"
            elif k == "u" and self.taken:
                self.taken = False
                self.status = "undone: feature-a back at 7f6e5d4"
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "picker":
            ls = self.listed()
            if k == "enter" and ls:
                b = ls[self.sel]
                local = b[0].replace("origin/", "")
                if local == self.head:
                    a.flash(f"you're on {self.head} already")
                elif self.dirty:
                    self.pending, self.mode = b, "switch"
                else:
                    self.switch(b)
            elif k == "tab" and ls:
                b = ls[self.sel]
                r = self.row_of(b[0])
                if r is None:
                    a.flash(f"{b[0]} is not drawn in this prototype's graph")
                else:
                    self.cur, self.mode = r, "graph"
                    self.status = f"showing {b[0]} on the graph · cursor on its last commit"
            elif k in ("down", "ctrl-n"):
                self.sel = min(len(ls) - 1, self.sel + 1)
            elif k in ("up", "ctrl-p"):
                self.sel = max(0, self.sel - 1)
            elif k == "esc":
                self.mode = "graph"
            elif k == "bs":
                self.q, self.sel = self.q[:-1], 0
            elif len(k) == 1 and k.isprintable():
                self.q, self.sel = self.q + k, 0
        elif self.mode == "switch":
            if k == "y":
                self.switch(self.pending)
            elif k == "esc":
                self.mode = "picker"
            else:
                a.flash(f"{k}: y switches, esc goes back")
        elif self.mode == "take":
            if k == "y":
                if self.head == "feature-a":
                    self.taken = True
                    self.cur = self.row_of("feature-a")
                self.mode = "graph"
                self.status = f"took 1 commit onto {self.head}: Fix null session on logout · u undoes"
            elif k == "esc":
                self.mode = "graph"
            else:
                a.flash(f"{k}: y takes it, esc cancels")

    def frame(self):
        return self.mode + (f" q={self.q!r}" if self.mode == "picker" else "") + f" head={self.head}"

    def draw(self, cv, top, bot):
        x = 1
        cv.put(x, top, f"Graph  ·  you are on {self.head}" + ("  ·  2 unsaved files" if self.dirty else ""), "b")
        draw_rows(cv, x, top + 2, W - 2, self.graph_rows(), self.cur if self.mode in ("graph", "take") else -1)
        bar = "⏎ open  C switch branch  o take onto yours  c commit  m merge  r rebase  b branch  . more"
        if self.mode in ("picker", "switch"):
            ls = self.listed()
            bx, by, bw = 1, top + 14, 150
            n = min(len(ls), 16)
            cv.box(bx, by, bw, n + 7, "SWITCH BRANCH  ·  yours and the remote's  ·  type to find")
            cv.put(bx + 3, by + 2, f"> {self.q}▏")
            for i, b in enumerate(ls[:n]):
                name, rem, who, when, subj = b
                tag = "remote" if rem else "local "
                local = name.replace("origin/", "")
                note = ""
                if local == self.head and not rem:
                    note = "● you're on it"
                elif name == self.prev:
                    note = "↩ the one you were on before"
                elif rem and not any(x[0] == local and not x[1] for x in BRANCHES) and not (
                        local == "sara/login-page" and self.local_login):
                    note = "no copy of yours yet"
                on = i == self.sel
                line = f"{'▸ ' if on else '  '}{tag}  {name:<26}{who:<7}{when:<5}{cut(subj, 36):<38}{note}"
                cv.put(bx + 2, by + 4 + i, cut(line, bw - 4))
                if on:
                    cv.attr(bx + 2, by + 4 + i, bw - 4, "r")
            if not ls:
                cv.put(bx + 4, by + 4, "no branch matches")
            cv.put(bx + 3, by + n + 5, f"{len(ls)} of {len(BRANCHES) + (1 if self.local_login else 0)} branches", "d")
            bar = "type to find  ⏎ switch to it  tab show it on the graph  ↑↓ row  esc close"
            if self.mode == "switch":
                b = self.pending
                px, py = 60, by + 4
                lines = [f"Switch to {b[0].replace('origin/', '')}.", "",
                         "Your 2 unsaved files are set aside and put back on it:",
                         "    src/auth/session.rs   README.md",
                         "If they clash with it, you land on the conflict screen; nothing is lost."]
                cv.box(px, py, 90, len(lines) + 5, "SWITCH")
                for i, ln in enumerate(lines):
                    cv.put(px + 3, py + 2 + i, ln)
                cv.put(px + 3, py + len(lines) + 3, "y switch    esc back", "b")
                bar = "y switch  esc back"
        if self.mode == "take":
            r = self.pending
            px, py = 1, top + 14
            lines = [f"{r['sha']}  {r['subj']}  ·  {r['who']}, {r['when']} ago", "",
                     f"{self.head} gets 1 new commit on top, with these changes:",
                     "    src/auth/session.rs   +4 −1", "",
                     "Clash check: none expected. The original stays where it is."]
            cv.box(px, py, 100, len(lines) + 5, f"TAKE ONTO {self.head.upper()}")
            for i, ln in enumerate(lines):
                cv.put(px + 3, py + 2 + i, ln)
            cv.put(px + 3, py + len(lines) + 3, "y take it    esc cancel", "b")
            bar = "y take it  esc cancel"
        return bar


# ---- the end questions ---------------------------------------------------------------------------

class End(Scene):
    name, title = "end", "Two questions"
    task = "Two last questions about your own repos. Answer each with a number."
    QS = [("end-submodules", "Do the repos you work in have submodules?",
           ["yes, and I touch them", "yes, but I never touch them", "no"]),
          ("end-pr-contents", "Before you push, do you check what your pull request will contain?",
           ["yes, and I'd want it in Graft", "yes, on GitHub after pushing", "not really"])]

    def reset(self):
        self.status, self.mode, self.n = "", "questions", 0

    def question(self):
        return self.QS[self.n] if self.n < len(self.QS) else None

    def answered(self, qid, n):
        self.n += 1

    def key(self, k):
        self.app.flash("answer with 1, 2 or 3")

    def frame(self):
        return f"question {self.n + 1}" if self.n < len(self.QS) else "done"

    def draw(self, cv, top, bot):
        y = top + 2
        for i, (qid, text, opts) in enumerate(self.QS):
            ans = self.app.answers.get(qid)
            cv.put(4, y, f"{i + 1}. {text}", "b")
            cv.put(8, y + 1, f"your answer: {opts[ans - 1]}" if ans else "not answered yet")
            y += 3
        if self.n >= len(self.QS):
            cv.put(4, y + 1, "Thank you. That's the whole prototype: q quits, and your keys and answers are in picks.txt.")
        return ""


# ================================================================== REV 2 (default)
# docs/design-notes/m7-power-tools.md, "Redesigned screens (after rev 1)" and "Prototype brief rev 2".
# Three scenes: A blame (the name at the line), B history (the whole file, each change marked),
# C journal (the lost commit is a row; the landing shows it's back).

# Commits that touched src/auth/session.rs, oldest first: sha, who, short age, long age, subject.
R2 = [("0f1e2d3", "Omar", "15mo", "15 months ago", "Session store"),
      ("1a2b3c4", "Omar", "1y", "a year ago", "Move session into auth/"),
      ("5e6f7a8", "Tom", "10mo", "10 months ago", "Add token refresh"),
      ("a1b2c3d", "Lina", "7mo", "7 months ago", "Retry refresh once on 401"),
      ("c4d5e6f", "Priya", "4mo", "4 months ago", "Move session TTL to config"),
      ("b8c9d0e", "Sara", "2mo", "2 months ago", "Log session ids at debug level"),
      ("7f6e5d4", "you", "0d", "today", "Handle expired tokens")]
C2 = {c[0]: c for c in R2}
ORD2 = [c[0] for c in R2]
O2, RN, T2, L2, P2, S2, Y2 = ORD2
PATH = "src/auth/session.rs"

# Every line the file ever had, in file order: (added by, removed by or None, text).
# The file at commit c = lines added at or before c and not yet removed. A removed line sits just
# above what replaced it, so a commit's removes and adds show in place.
REC = [
    (O2, None, "use std::time::Duration;"), (O2, None, "use crate::store::Store;"), (S2, None, "use tracing::debug;"),
    (O2, None, ""), (O2, None, "pub struct Session {"), (O2, None, "    id: SessionId,"), (O2, None, "    token: Token,"),
    (P2, None, "    ttl: Duration,"), (O2, None, "}"), (O2, None, ""), (O2, None, "impl Session {"),
    (O2, None, "    pub fn new(id: SessionId, token: Token) -> Self {"),
    (O2, P2, "        Session { id, token }"),
    (P2, None, "        let ttl = config::session_ttl();"), (P2, None, "        Session { id, token, ttl }"),
    (O2, None, "    }"), (O2, None, ""),
    (T2, None, "    /// Refresh the token when the server says it expired."),                       # fn starts
    (T2, None, "    pub fn refresh_token(&mut self, client: &Client) -> Result<()> {"),
    (L2, None, "        let mut attempt = 0;"),
    (T2, None, "        loop {"),
    (T2, None, "            let resp = client.refresh(&self.token)?;"),
    (T2, L2, "            if resp.status() == 401 {"),
    (L2, None, "            if resp.status() == 401 && attempt == 0 {"),
    (L2, None, "                attempt += 1;"),
    (T2, None, "                continue;"),
    (T2, None, "            }"),
    (T2, P2, "            self.ttl = resp.ttl();"),
    (P2, None, "            self.ttl = resp.ttl().unwrap_or(self.ttl); // the server may shorten it; keep ours"),
    (S2, None, '            debug!(session = %self.id, user = %self.user_id, ttl = ?self.ttl, attempt, '
               '"token refreshed; one retry left at most");'),
    (Y2, None, "            if resp.token_expired() {"),
    (Y2, None, "                self.token = Token::empty();"),
    (Y2, None, "                return Err(Error::TokenExpired);"),
    (Y2, None, "            }"),
    (T2, None, "            self.token = resp.token()?;"),
    (T2, None, "            return Ok(());"),
    (T2, None, "        }"),
    (T2, None, "    }"),                                                                            # fn ends
    (O2, None, ""), (O2, None, "    pub fn save(&self, store: &Store) -> Result<()> {"),
    (S2, None, '        debug!(session = %self.id, "saving session");'), (O2, None, "        store.put(&self.id, self)"),
    (O2, None, "    }"), (O2, None, ""),
    (Y2, None, "    pub fn expired(&self) -> bool {"), (Y2, None, "        self.token.age() > self.ttl"),
    (Y2, None, "    }"),
    (O2, None, "}"),
]
FN_FIRST = next(i for i, r in enumerate(REC) if "/// Refresh" in r[2])
FN_LAST = FN_FIRST + next(i for i, r in enumerate(REC[FN_FIRST:]) if r[2] == "    }")


def later(a, b):
    return ORD2.index(a) > ORD2.index(b)


def alive(rec, c):
    return not later(rec[0], c) and (rec[1] is None or later(rec[1], c))


def file_at(c):
    """[(rec index, line number or None, mark)] for the file at c with c's own changes marked:
    '+' added by c, '−' removed by c (no number: it isn't in the file any more), ' ' unchanged."""
    out, n = [], 0
    for i, r in enumerate(REC):
        if alive(r, c):
            n += 1
            out.append((i, n, "+" if r[0] == c else " "))
        elif r[1] == c:
            out.append((i, None, "−"))
    return out


def stat(c, in_fn):
    rng = range(FN_FIRST, FN_LAST + 1) if in_fn else range(len(REC))
    a = sum(1 for i in rng if REC[i][0] == c)
    d = sum(1 for i in rng if REC[i][1] == c)
    return (f"+{a}" if a else "") + (f" −{d}" if d else "") or "±0"


HEAD_FILE = [(n, REC[i][0], REC[i][2]) for i, n, _ in file_at(Y2)]  # (number, commit, text) as it is now
FN_LINES = (HEAD_FILE[[i for i, _, _ in file_at(Y2)].index(FN_FIRST)][0],
            HEAD_FILE[[i for i, _, _ in file_at(Y2)].index(FN_LAST)][0])
AGE_HUE = {c: f"age{len(ORD2) - 1 - k}" for k, c in enumerate(ORD2)}  # newest warmest (DESIGN §1.12)

OTHER_FILES = [("tests/session.rs", "+6", [("+", "#[test]"), ("+", "fn expired_token_is_an_error() {"),
                                           ("+", "    let s = Session::with_expired_token();"),
                                           ("+", "    assert!(s.refresh_token(&fake()).is_err());"), ("+", "}"),
                                           ("+", "")]),
               ("README.md", "+1 −1", [(" ", "## Sessions"), ("−", "Tokens refresh forever."),
                                      ("+", "Tokens refresh once; an expired token is an error.")])]


class Code(Scene):
    """Scenes A and B: the diff pane, blame, and history on one fake file."""
    start = "diff"

    def reset(self):
        self.status, self.mode, self.line = "", self.start, 22
        self.drow = self.diff_rows().index(next(r for r in self.diff_rows() if r.get("n") == 22))
        self.hrange, self.hcur, self.focus, self.hoff, self.back = True, 0, "list", 0, "blame"
        self.qs, self.asked, self.seen = [], set(), set()

    # -- questions: queued as they trigger, shown one at a time
    def ask(self, qid):
        if qid not in self.asked and qid not in self.qs:
            self.qs.append(qid)

    def question(self):
        return (self.qs[0], *self.QTEXT[self.qs[0]]) if self.qs else None

    def answered(self, qid, n):
        self.asked.add(qid)
        self.qs.remove(qid)

    # -- the diff pane: every file of 7f6e5d4, the name row (▾) first, then its lines
    def diff_rows(self):
        rows = [dict(kind="file", path=PATH, stat=stat(Y2, False))]
        rows += [dict(kind="line", path=PATH, n=n, mark=m, text=REC[i][2]) for i, n, m in file_at(Y2)]
        for path, st, lines in OTHER_FILES:
            rows.append(dict(kind="file", path=path, stat=st))
            num = 0
            for m, t in lines:
                num += m != "−"
                rows.append(dict(kind="line", path=path, n=num if m != "−" else None, mark=m, text=t))
        return rows

    def hlist(self):
        return [Y2, S2, P2, L2, T2] if self.hrange else list(reversed(ORD2))

    def open_history(self, rng, back):
        self.mode, self.hrange, self.back, self.focus = "history", rng, back, "list"
        lst = self.hlist()
        blamed = next(c for n, c, _ in HEAD_FILE if n == self.line)
        self.hcur = lst.index(blamed) if rng and blamed in lst else 0
        self.scroll_home()
        self.seen.add("history-range" if rng else "history-file")

    def scroll_home(self):
        rows = file_at(self.hlist()[self.hcur])
        idx = [i for i, _, _ in rows]
        if self.hrange:
            first = min(k for k, i in enumerate(idx) if i >= FN_FIRST)
        else:
            first = next((k for k, (_, _, m) in enumerate(rows) if m != " "), 0)
        self.hoff = max(0, min(first - 3, len(rows) - self.room))

    room = 35  # file-pane rows at 50 lines; draw_history sets it

    def change_starts(self):
        rows = file_at(self.hlist()[self.hcur])
        return [k for k, (_, _, m) in enumerate(rows) if m != " " and (k == 0 or rows[k - 1][2] == " ")]

    def key(self, k):
        a = self.app
        if self.mode == "diff":
            rows = self.diff_rows()
            r = rows[self.drow]
            if k in ("j", "down"):
                self.drow = min(len(rows) - 1, self.drow + 1)
            elif k in ("k", "up"):
                self.drow = max(0, self.drow - 1)
            elif k == "B":
                if r["path"] != PATH or r["kind"] != "line":
                    a.flash("blame is acted out for the lines of src/auth/session.rs only")
                else:
                    self.mode, self.line = "blame", r["n"]
                    self.on_blame_move()
            elif k == "F":  # the note says "on a file row"; here any row of the file (README findings)
                if r["path"] != PATH:
                    a.flash("history is acted out for src/auth/session.rs only")
                else:
                    self.open_history(False, "diff")
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "blame":
            if k in ("j", "down"):
                self.line = min(len(HEAD_FILE), self.line + 1)
                self.on_blame_move()
            elif k in ("k", "up"):
                self.line = max(1, self.line - 1)
                self.on_blame_move()
            elif k == "L":
                self.open_history(True, "blame")
            elif k == "esc":
                self.mode = "diff"
                self.drow = next(i for i, r in enumerate(self.diff_rows()) if r.get("n") == self.line
                                 and r["path"] == PATH)
            elif k == "enter":
                a.flash("⏎: that commit in the graph (acted out in rev 1, not here)")
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "history":
            lst = self.hlist()
            if self.focus == "list" and k in ("j", "down", "k", "up"):
                self.hcur = max(0, min(len(lst) - 1, self.hcur + (1 if k in ("j", "down") else -1)))
                self.scroll_home()
                if k in ("j", "down"):
                    self.seen.add("stepped")
                    self.after_history_step()
            elif self.focus == "file" and k in ("j", "down", "k", "up"):
                n = len(file_at(lst[self.hcur]))
                self.hoff = max(0, min(n - self.room, self.hoff + (1 if k in ("j", "down") else -1)))
            elif k in ("n", "N"):
                starts = self.change_starts()
                if not starts:
                    a.flash("no change in this file at this commit")
                    return
                at = self.hoff + 3
                nxt = [s for s in starts if s > at] if k == "n" else [s for s in starts if s < at]
                tgt = (nxt[0] if k == "n" else nxt[-1]) if nxt else (starts[0] if k == "n" else starts[-1])
                self.hoff = max(0, min(tgt - 3, len(file_at(lst[self.hcur])) - self.room))
            elif k == "tab":
                self.focus = "file" if self.focus == "list" else "list"
            elif k == "esc":
                self.mode = self.back
                self.after_history_exit()
            elif k == "B":
                self.mode = "blame"
                a.flash("B: blame as of that commit, at that line (shown as it is now in this prototype)")
            elif k == "enter":
                a.flash("⏎: that commit in the graph (acted out in rev 1, not here)")
            else:
                a.flash(f"{k}: not in this prototype")

    def on_blame_move(self):
        pass

    def after_history_step(self):
        pass

    def after_history_exit(self):
        pass

    def frame(self):
        if self.mode == "diff":
            r = self.diff_rows()[self.drow]
            return f"diff {r['path']}:{r.get('n') or r['kind']}"
        if self.mode == "blame":
            return f"blame line {self.line}"
        return (f"history {'range' if self.hrange else 'file'} {self.hlist()[self.hcur]} focus={self.focus}")

    # -- drawing
    def draw(self, cv, top, bot):
        return {"diff": self.draw_diff, "blame": self.draw_blame, "history": self.draw_history}[self.mode](cv, top, bot)

    def draw_diff(self, cv, top, bot):
        x = 1
        cv.put(x, top, f"CHANGES  ·  in 7f6e5d4 Handle expired tokens  ·  you, today  ·  3 files", "b")
        rows = self.diff_rows()
        room = bot - top - 2
        off = max(0, min(self.drow - room // 2, len(rows) - room))
        for k, r in enumerate(rows[off:off + room], off):
            y = top + 2 + k - off
            on = k == self.drow
            if r["kind"] == "file":
                cv.put(x, y, f"{'▸' if on else ' '} ▾ {r['path']}   ", "b")
                end = x + 4 + len(r["path"]) + 3
                cv.put(end, y, r["stat"])
                if r["stat"].startswith("+"):
                    cv.tint(y, r["stat"].split()[0], "add+b", end)
                if "−" in r["stat"]:
                    cv.tint(y, r["stat"].split()[-1], "del+b", end)
            else:
                num = f"{r['n']:>4}" if r["n"] else "    "
                at = {"+": "add", "−": "del", " ": ""}[r["mark"]]
                cv.put(x, y, f"{'▸' if on else ' '} {num}  ")
                cv.put(x + 8, y, f"{r['mark']} {r['text']}", at)
            if on:
                cv.add(x, y, W - 2, "r")
        return "⏎ open file  B blame  F file history  space stage  e editor  w whitespace  v side by side  esc back"

    def trailer(self, c, room):
        _, who, _, agel, subj = C2[c]
        head = f"◂ {who} · {agel}"
        for t in (f"{head} · {subj} · {c}", f"{head} · {subj}"):
            if len(t) <= room:
                return t
        keep = room - len(head) - 4  # " · " and "…"
        if keep >= 6:
            return f"{head} · {subj[:keep]}…"
        return head if len(head) <= room else None

    def draw_blame(self, cv, top, bot):
        x = 1
        cv.put(x, top, f"BLAME  ·  {PATH}  ·  as it is now", "b")
        room = bot - top - 2
        ci = self.line - 1
        off = max(0, min(ci - room // 2, len(HEAD_FILE) - room))
        gw, cx = 46, x + 46 + 1 + 6  # gutter 46 cells, the │ rule, then "NNN  " and the code
        for k in range(off, min(len(HEAD_FILE), off + room)):
            n, c, text = HEAD_FILE[k]
            prev = HEAD_FILE[k - 1][1] if k else None
            nxt = HEAD_FILE[k + 1][1] if k + 1 < len(HEAD_FILE) else None
            first, last = c != prev, c != nxt
            br = "─" if first and last else "┌" if first else "└" if last else "│"
            on = k == ci
            y = top + 2 + k - off
            sha, who, age, _, subj = C2[c]
            info = f" {sha} {who:<6}{age:>4}  {cut(subj, 24)}" if first or on else ""
            cv.put(x, y, "▸" if on else " ", "b")
            cv.put(x + 1, y, br, AGE_HUE[c])
            cv.put(x + 2, y, cut(info, gw - 2), "mut")
            if info:
                cv.tint(y, f"{age:>4}", AGE_HUE[c], x + 2 + 8 + 6)
            cv.put(x + gw, y, "│", "mut")
            cv.put(x + gw + 1, y, f"{n:>4}  ", "mut")
            cv.put(cx, y, text)
            if on:
                start = cx + len(text) + 4
                t = self.trailer(c, W - start)
                if t:
                    cv.put(start, y, t, "mut")
                cv.add(x, y, W - 2, "r")
        return "⏎ that commit  ^ before it  L history of these lines  n older  N newer  space ages  esc back"

    def draw_history(self, cv, top, bot):
        x, lw = 1, 56
        lst = self.hlist()
        sel = lst[self.hcur]
        what = f"fn refresh_token, lines {FN_LINES[0]}–{FN_LINES[1]}" if self.hrange else "the whole file"
        cv.put(x, top, f"HISTORY  ·  {PATH}  ·  {what}  ·  {len(lst)} changes", "b")
        y = top + 2
        cv.put(x, y, "commits that changed " + ("these lines" if self.hrange else "this file") + ", newest first",
               "b" if self.focus == "list" else "mut")
        y += 1
        for k, c in enumerate(lst):
            sha, who, age, _, subj = C2[c]
            on = k == self.hcur
            st = stat(c, self.hrange)
            cv.put(x, y, f"{'▸' if on else ' '} {sha}  {cut(subj, 26):<27}{who:<6}{age:>4} ")
            cv.put(x + 50, y, st)
            cv.tint(y, sha, "hash", x)
            if st.startswith("+"):
                cv.tint(y, st.split()[0], "add", x + 50)
            if "−" in st:
                cv.tint(y, st.split()[-1], "del", x + 50)
            if on:
                cv.add(x, y, lw - 1, "r")
            y += 1
            if c == RN:
                cv.put(x + 4, y, "↳ was src/session.rs", "mut")
                y += 1
        if self.hrange:
            cv.put(x + 2, y + 1, f"5e6f7a8 added these lines: nothing older", "mut")
        for yy in range(top + 2, bot):
            cv.put(x + lw, yy, "│", "mut")
        # the file pane: the whole file at the selected commit, its changes marked in place
        fx = x + lw + 2
        _, who, _, agel, subj = C2[sel]
        head = (f"{sel} · {who} · {agel} · {subj} · " +
                ("renamed from src/session.rs · no line changed" if sel == RN else f"this file {stat(sel, False)}"))
        cv.put(fx, top + 2, cut(head, W - fx - 1), "b" if self.focus == "file" else "")
        cv.tint(top + 2, sel, "hash", fx)
        rows = file_at(sel)
        room = self.room = bot - top - 4
        for k, (i, n, m) in enumerate(rows[self.hoff:self.hoff + room]):
            yy = top + 3 + k
            if self.hrange and FN_FIRST <= i <= FN_LAST:
                cv.put(fx, yy, "┃", "acc+b")
            cv.put(fx + 2, yy, f"{n:>4}" if n else "    ", "mut")
            at = {"+": "add", "−": "del", " ": ""}[m]
            cv.put(fx + 8, yy, f"{m}  {REC[i][2]}", at)
        if len(rows) > self.hoff + room:
            cv.put(fx + 8, bot - 1, f"… {len(rows) - self.hoff - room} more lines", "mut")
        if self.focus == "file":
            return "j/k scroll  n/N next·previous change  tab the list  B blame here  esc back"
        return "j/k older·newer  tab file  n/N next change  B blame here  ⏎ that commit  esc back"


class BlameA(Code):
    name, title = "blame", "A · who changed each line"
    task = ("Line 22 of session.rs looks odd to you. Walk down from it through the next ten lines, "
            "and say out loud who changed each one.")
    QTEXT = {
        "a-eyes": ("Three lines down: where did your eyes go to find who changed the line you were on?",
                   ["the end of that line", "the left edge", "back and forth"]),
        "a-in-way": ("Did the name at the end of the line get in the way of reading the code?",
                     ["no", "a little", "yes"]),
        "a-blocks": ("The brackets on the left: did each one read as one commit's lines?",
                     ["yes", "not sure", "no"]),
    }

    def reset(self):
        super().reset()
        self.walked = set()

    def on_blame_move(self):
        self.walked.add(self.line)
        if self.line >= 25:
            self.ask("a-eyes")
        if self.line >= 32:
            self.ask("a-in-way")
            self.ask("a-blocks")


class HistoryB(Code):
    name, title = "history", "B · how this got this way"
    start = "blame"
    task = ("You're reading the blame, on line 22. How did this function get the way it is? "
            "Then go back to the changes and ask the same of the whole file: what has happened to it?")
    QTEXT = {
        "b-file": ("Is this \"the file, with what got added and removed\"?", ["yes", "partly", "no"]),
        "b-time": ("Moving down that list: did it feel like stepping back in time?",
                   ["yes", "no", "didn't notice"]),
        "b-same": ("The function's history and the file's history: did they feel like the same place?",
                   ["yes, the same place", "similar", "different places"]),
        "b-mono": ("Turn colour off (# in the top bar) and look at a commit's changes: does a removed line still "
                   "read as removed?", ["yes", "barely", "no"]),
    }

    def after_history_step(self):
        self.ask("b-file")
        self.ask("b-time")
        if {"history-range", "history-file"} <= self.seen:
            self.ask("b-same")
            self.ask("b-mono")

    def after_history_exit(self):
        if "stepped" in self.seen:
            self.ask("b-file")
        if {"history-range", "history-file"} <= self.seen:
            self.ask("b-same")
            self.ask("b-mono")


# ---- scene C: journal, lost commits as rows ------------------------------------------------------

LOST = {"main": ("9a8b7c6", "Add rate limiter", "was main's tip · no branch holds it"),
        "spike": ("4d5e6f7", "Try redis cache", "was spike's tip · spike is gone")}


class JournalC(Scene):
    name, title = "journal", "C · lost in the shell"
    task = ("You ran git reset --hard in the shell and lost your 'Add rate limiter' commit. Get it back. "
            "Also: where did your spike branch go? You deleted it in the shell by mistake. Bring it back too.")
    QTEXT = {
        "c-worked": ("Did it work? Answer from what the screen shows you.",
                     ["yes, it's back", "no", "can't tell"]),
        "c-mono": ("Turn colour off (# in the top bar) and open the journal: can you tell Graft's rows, the "
                   "shell's rows and lost commits apart?", ["yes", "only some", "no"]),
    }

    def reset(self):
        self.status, self.mode, self.cur, self.lcur = "", "graph", 0, 0
        self.back = {"main": False, "spike": False}
        self.done_log, self.tags = [], {}
        self.qs, self.asked = [], set()
        self.pending, self.bname, self.motions = None, "", 0

    ask = Code.ask
    answered = Code.answered

    def question(self):
        return (self.qs[0], *self.QTEXT[self.qs[0]]) if self.qs else None

    def text_mode(self):
        return self.mode == "branch"

    def lost(self, who):
        if who == "main":
            return not self.back["main"] and not self.back["spike"]  # spike's tip sits on top of it
        return not self.back["spike"]

    def rows(self):
        out = [dict(kind="g", time=t, words=w) for t, w in reversed(self.done_log)]
        out.append(dict(kind="g", time="10:44", words="fetch · origin/main moved 2 commits"))
        out.append(dict(kind="s", time="10:41", words="reset main 1 commit back", git='git: "reset: moving to HEAD~1"',
                        ref="main"))
        if self.lost("main"):
            out.append(dict(kind="x", who="main"))
        out.append(dict(kind="s", time="10:37", words="switch from spike to main",
                        git='git: "checkout: moving from spike to main"'))
        if self.lost("spike"):
            out.append(dict(kind="x", who="spike"))
        out += [dict(kind="g", time="10:12", words="switch from main to spike"),
                dict(kind="g", time="10:05", words='commit on main · "Add rate limiter"'),
                dict(kind="g", time="09:58", words="pull main · 3 commits")]
        return out

    def graph_rows(self):
        t = self.tags
        side = self.back["main"] or self.back["spike"]
        rows = [dict(g="○", sha="8b9c0d1", refs="origin/main", subj="Fix typo in login copy", who="Omar", when="20m"),
                dict(g="○", sha="7a8b9c0", refs="", subj="Bump deps", who="Omar", when="1h")]
        if self.back["spike"]:
            rows.append(dict(g="│ ○", sha="4d5e6f7", refs="  ".join(x for x in ("spike", t.get("4d5e6f7", "")) if x),
                             subj="Try redis cache", who="you", when="10:35"))
        if side:
            m = self.back["main"]
            rows.append(dict(g="│ ●" if m else "│ ○", sha="9a8b7c6",
                             refs="  ".join(x for x in ("main ← HEAD" if m else "", t.get("9a8b7c6", "")) if x),
                             subj="Add rate limiter", who="you", when="10:05"))
            rows.append(dict(g="├─╯", sel=False))
        m = self.back["main"]
        rows.append(dict(g="○" if m else "●", sha="3d4e5f6",
                         refs="  ".join(x for x in ("" if m else "main ← HEAD", t.get("3d4e5f6", "")) if x),
                         subj="Add metrics endpoint", who="you", when="1d"))
        rows.append(dict(g="○", sha="2c3d4e5", refs="", subj="Session store cleanup", who="Omar", when="2d"))
        return rows

    def put_back(self, who):
        sha, subj, _ = LOST[who]
        self.back[who] = True
        self.tags = {sha: "↺ put back"}
        if who == "main":
            self.tags["3d4e5f6"] = "○ main was here"
            self.done_log.append((time.strftime("%H:%M"), "put main back (the shell's reset)"))
            self.status = f'✓ put back · main has "{subj}" again · u undoes'
        else:
            self.done_log.append((time.strftime("%H:%M"), "bring spike back (deleted in the shell)"))
            self.status = f'✓ put back · spike is back, at "{subj}" · u undoes'
        self.mode = "graph"
        self.cur = next(i for i, r in enumerate(selectable(self.graph_rows())) if r["sha"] == sha)
        self.landed = who
        self.ask("c-worked")  # once: on the first landing, whichever it was
        if all(self.back.values()):
            self.ask("c-mono")

    def key(self, k):
        a = self.app
        if self.mode == "graph":
            if k == "J":
                self.mode, self.lcur, self.motions = "list", 0, 0
                self.tags = {}
            elif k in ("j", "down"):
                self.cur = min(len(selectable(self.graph_rows())) - 1, self.cur + 1)
            elif k in ("k", "up"):
                self.cur = max(0, self.cur - 1)
            elif k == "u":
                if self.done_log:
                    _, words = self.done_log.pop()
                    who = "main" if "main" in words else "spike"
                    self.back[who], self.tags = False, {}
                    self.cur = 0
                    self.status = f"undone: {words}"
                else:
                    a.flash("u undoes Graft's own last step: fetch (10:44). The reset ran in the shell — "
                            "u never undoes shell steps; the journal can put it back")
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "list":
            rs = self.rows()
            r = rs[self.lcur]
            if k in ("j", "down", "k", "up"):
                self.motions += 1
                self.lcur = max(0, min(len(rs) - 1, self.lcur + (1 if k in ("j", "down") else -1)))
            elif k == "enter":
                a.log(f"{time.strftime('%H:%M:%S')}   motions from J to ⏎: {self.motions} (row: {r['kind']} "
                      f"{r.get('who') or r['words']})")
                if r["kind"] == "x":
                    self.pending, self.mode = r["who"], "preview"
                elif r["kind"] == "s" and r.get("ref") == "main":
                    if self.back["main"]:
                        a.flash("main is already back where it was before this reset")
                    else:
                        self.pending, self.mode = "main", "preview"
                else:
                    a.flash("acted out only for the reset and the ✗ rows in this prototype")
            elif k == "b":
                if r["kind"] == "x":
                    self.pending, self.mode = r["who"], "branch"
                    self.bname = "spike" if r["who"] == "spike" else "rate-limiter"
                else:
                    a.flash("b is acted out on the ✗ rows only in this prototype")
            elif k == "/":
                a.flash("/ find: not acted out in rev 2")
            elif k == "u":
                a.flash("u undoes Graft's own steps (●); it never touches the shell's ($)")
            elif k in ("esc", "J"):
                self.mode = "graph"
            else:
                a.flash(f"{k}: not in this prototype")
        elif self.mode == "preview":
            if k == "y":
                self.put_back(self.pending)
            elif k == "esc":
                self.mode = "list"
            else:
                a.flash(f"{k}: y puts it back, esc cancels")
        elif self.mode == "branch":
            if k == "enter":
                who = self.pending
                if who == "spike" and self.bname == "spike":
                    self.put_back("spike")
                else:
                    a.flash("acted out only for making spike again in this prototype")
                    self.mode = "list"
            elif k == "esc":
                self.mode = "list"
            elif k == "bs":
                self.bname = self.bname[:-1]
            elif len(k) == 1 and k.isprintable():
                self.bname += k

    def frame(self):
        return self.mode + f" back={','.join(w for w, v in self.back.items() if v) or '-'}"

    def draw(self, cv, top, bot):
        x = 1
        cv.put(x, top, "Graph  ·  you are on main", "b")
        rows = self.graph_rows()
        draw_rows(cv, x, top + 2, W - 2, rows, self.cur if self.mode == "graph" else -1)
        for i, r in enumerate(rows):
            y = top + 2 + i
            cv.tint(y, "↺ put back", "ok+b")
            cv.tint(y, "○ main was here", "mut")
            if r.get("sha"):
                cv.tint(y, r["sha"], "hash")
        if self.mode == "graph":
            self.draw_detail(cv, top + 3 + len(rows), bot)
            return "J journal  u undo  ⏎ open  c commit  b branch  m merge  r rebase  . more"
        bx, by, bw = 3, top + 10, 181
        rs = self.rows()
        bh = len(rs) + 5
        cv.box(bx, by, bw, bh, "JOURNAL  ·  what happened in this repo, newest first")
        y = by + 2
        for i, r in enumerate(rs):
            on = i == self.lcur
            cv.put(bx + 2, y, "▸" if on else " ", "b")
            if r["kind"] == "x":
                sha, subj, note = LOST[r["who"]]
                cv.put(bx + 7, y, "└ ✗ lost", "lost+b")
                cv.put(bx + 24, y, sha, "hash")
                cv.put(bx + 32, y, f'"{subj}"', "lost")
                cv.put(bx + 34 + len(subj), y, f" · {note}")
            else:
                g = r["kind"] == "g"
                cv.put(bx + 4, y, "●  Graft" if g else "$  outside Graft", "graft+b" if g else "shell+b")
                cv.put(bx + 22, y, r["time"], "mut")
                cv.put(bx + 30, y, r["words"])
                if r.get("git"):
                    cv.put(bx + 110, y, r["git"], "mut")
            if on:
                cv.add(bx + 1, y, bw - 2, "r")
            y += 1
        ly = by + bh - 2
        cv.put(bx + 3, ly, "●", "graft+b")
        cv.put(bx + 5, ly, "done in Graft: u can undo it   ·   ", "mut")
        cv.put(bx + 40, ly, "$", "shell+b")
        cv.put(bx + 42, ly, "done outside Graft: u never touches it   ·   ", "mut")
        cv.put(bx + 87, ly, "✗", "lost+b")
        cv.put(bx + 89, ly, "a commit no branch holds", "mut")
        if self.mode == "preview":
            return self.draw_preview(cv, bx, by + bh + 1)
        if self.mode == "branch":
            sha, subj, _ = LOST[self.pending]
            px, py = bx, by + bh + 1
            cv.box(px, py, 100, 6, "NEW BRANCH")
            cv.put(px + 3, py + 2, f"at {sha} {subj}   name: {self.bname}▏")
            cv.put(px + 3, py + 4, "⏎ make it    esc cancel", "b")
            return "type a name  ⏎ make it  esc cancel"
        r = rs[self.lcur]
        per = {"x": ("⏎ put main back to it" if r.get("who") == "main" else "⏎ bring spike back at it"),
               "s": "⏎ put main back to before this" if r.get("ref") == "main" else "⏎ put back to before this",
               "g": "⏎ revert to before this"}[r["kind"]]
        return f"{per}  b new branch here  / find  j/k row  esc close"

    def draw_preview(self, cv, px, py):
        who = self.pending
        sha, subj, _ = LOST[who]
        if who == "main":
            what = "Put main back where it was before the shell's reset."
            now, after = "main → 3d4e5f6 Add metrics endpoint", f"main → {sha} {subj}   (+1 commit back)"
            files = "Your files: src/api/limit.rs comes back (it's in that commit). Nothing unsaved; nothing lost."
        else:
            what = "Bring back spike, the branch deleted in the shell."
            now, after = "spike — gone (deleted in the shell)", f"spike → {sha} {subj}   (made again)"
            files = "Your files: untouched. You stay on main."
        cv.box(px, py, 110, 10, "PUT BACK")
        cv.put(px + 3, py + 2, what)
        cv.put(px + 3, py + 4, "now      ", "mut+b")
        cv.put(px + 12, py + 4, now, "mut")
        cv.put(px + 3, py + 5, "after y  ", "ok+b")
        cv.put(px + 12, py + 5, after, "ok")
        cv.put(px + 3, py + 7, files)
        cv.put(px + 3, py + 8, "y put it back    esc cancel", "b")
        return "y put it back  esc cancel"

    def draw_detail(self, cv, y, bot):
        r = selectable(self.graph_rows())[self.cur]
        info = {"9a8b7c6": ("you, today 10:05", ["src/api/limit.rs  +48  (new file)", "src/api/mod.rs     +2"],
                            "Rate limit per API key: 100 requests a minute, 429 with Retry-After."),
                "4d5e6f7": ("you, today 10:35", ["src/cache/redis.rs  +61  (new file)", "Cargo.toml          +1"],
                            "Try a redis cache for session lookups. Spike, not for review.")}.get(r["sha"])
        cv.put(1, y, "─" * (W - 2), "mut")
        cv.put(1, y + 1, f"COMMIT {r['sha']}  {r['subj']}", "b")
        if not info:
            cv.put(1, y + 2, f"{r['who']}, {r['when']} ago", "mut")
            return
        cv.put(1, y + 2, info[0], "mut")
        for i, f in enumerate(info[1]):
            cv.put(3, y + 4 + i, f)
            cv.tint(y + 4 + i, "+", "add")
        cv.put(3, y + 5 + len(info[1]), info[2])


class EndRev2(Scene):
    name, title = "end", "One question"
    task = "One last question about the three screens you just used. Answer with a number."
    QS = [("end-still-same", "In which of the three did two different things still look the same?",
           ["blame", "history", "journal", "none of them"])]
    reset = lambda self: setattr(self, "n", 0) or setattr(self, "status", "")
    question = lambda self: self.QS[self.n] if self.n < len(self.QS) else None

    def answered(self, qid, n):
        self.n += 1

    def key(self, k):
        self.app.flash("answer with a number")

    def frame(self):
        return "question" if self.n < len(self.QS) else "done"

    def draw(self, cv, top, bot):
        qid, text, opts = self.QS[0]
        ans = self.app.answers.get(qid)
        cv.put(4, top + 2, text, "b")
        cv.put(8, top + 3, f"your answer: {opts[ans - 1]}" if ans else "not answered yet")
        if ans:
            cv.put(4, top + 5, "Thank you. q quits; your keys and answers are in picks.txt. "
                               "If you can, say which two things looked the same.")
        return ""


# ------------------------------------------------------------------ the app


class App:
    def __init__(self, args):
        self.args = args
        self.rev = 1 if args.rev1 else 2
        if self.rev == 1:
            self.scenes = [Tidy(self), Blame(self), Journal(self), Picker(self), End(self)]
        else:
            self.scenes = [BlameA(self), HistoryB(self), JournalC(self), EndRev2(self)]
        COLOUR[0] = not args.no_colour
        self.idx = min(args.scene, len(self.scenes)) - 1
        self.flash_msg, self.done = "", False
        self.answers = {}
        self.last_key_at = None
        self.log_fh = open(args.picks, "a", encoding="utf-8")
        self.log(f"=== m7 prototype rev {self.rev} session {time.strftime('%Y-%m-%d %H:%M:%S')} ==="
                 + ("" if COLOUR[0] else " (colour off)"))
        self.log(f"reached scene {self.idx + 1} ({self.scene().name})")

    def log(self, line):
        self.log_fh.write(line + "\n")
        self.log_fh.flush()

    def scene(self):
        return self.scenes[self.idx]

    def flash(self, msg):
        self.flash_msg = msg

    def goto(self, i):
        self.idx = max(0, min(len(self.scenes) - 1, i))
        self.scene().reset()
        self.log(f"{time.strftime('%H:%M:%S')} reached scene {self.idx + 1} ({self.scene().name})")

    def handle(self, k):
        now = time.monotonic()
        if self.last_key_at is not None and now - self.last_key_at > 3:
            self.log(f"{time.strftime('%H:%M:%S')}   pause {now - self.last_key_at:.1f}s")
        self.last_key_at = now
        sc = self.scene()
        before = sc.frame()
        self.flash_msg = ""
        q = sc.question()
        typing = sc.text_mode()
        if not typing and k == "q":
            self.done = True
        elif not typing and k == "]":
            self.goto(self.idx + 1)
        elif not typing and k == "[":
            self.goto(self.idx - 1)
        elif not typing and k == "bs":
            sc.reset()
            self.flash("scene restarted")
        elif not typing and k == "#" and self.rev == 2:
            COLOUR[0] = not COLOUR[0]
            self.flash("colour " + ("on" if COLOUR[0] else "off: glyphs, rules and positions only"))
        elif q and k.isdigit() and 1 <= int(k) <= len(q[2]):
            n = int(k)
            self.answers[q[0]] = n
            self.log(f"{time.strftime('%H:%M:%S')}   ANSWER {q[0]}: {n} = {q[2][n - 1]}")
            sc.answered(q[0], n)
        else:
            sc.key(k)
        sc = self.scene()
        self.log(f"{time.strftime('%H:%M:%S')} s{self.idx + 1} key {k!r:<10} {before} → {sc.frame()}"
                 + (f"   [{self.flash_msg}]" if self.flash_msg else ""))

    def canvas(self):
        cv = Canvas(W, H)
        sc = self.scene()
        if self.rev == 1:
            cv.put(0, 0, f" PROTOTYPE m7 · scene {self.idx + 1} of {len(self.scenes)}: {sc.title} · "
                         "] next scene  [ back  Backspace restart  q quit" + ("  ~ unsaved edits on/off" if sc.name == "picker" else ""), "d")
        else:
            cv.put(0, 0, f" PROTOTYPE m7 rev 2 · scene {self.idx + 1} of {len(self.scenes)}: {sc.title} · "
                         f"] next scene  [ back  Backspace restart  # colour {'on' if COLOUR[0] else 'off'}  q quit",
                   "d")
        words, lines, cur = ("Your task: " + sc.task).split(), [], ""
        for wd in words:
            if len(cur) + len(wd) + 1 > W - 14:
                lines.append(cur)
                cur = wd
            else:
                cur = (cur + " " + wd).strip()
        lines.append(cur)
        for i, ln in enumerate(lines[:2]):
            cv.put(1 if i == 0 else 12, 1 + i, ln, "b")
        cv.put(0, 3, "─" * W)
        hints = sc.draw(cv, 5, H - 6)
        q = sc.question()
        if q:
            qid, text, opts = q
            cv.put(0, H - 5, "─" * W)
            cv.put(1, H - 4, "QUESTION  " + text, "b")
            cv.put(1, H - 3, "    " + "     ".join(f"{i + 1}  {o}" for i, o in enumerate(opts)))
        msg = self.flash_msg or sc.status
        cv.put(1, H - 2, msg, "ok+b" if msg.startswith("✓") else "")
        cv.put(1, H - 1, hints or "")
        return cv


# ------------------------------------------------------------------ terminal

NAMES = {"\r": "enter", "\n": "enter", "\x1b": "esc", "\x7f": "bs", "\x08": "bs", "\t": "tab", "\x0e": "ctrl-n",
         "\x10": "ctrl-p", "\x1b[A": "up", "\x1b[B": "down", "\x1b[C": "right", "\x1b[D": "left",
         "\x1b[1;3A": "alt-up", "\x1b[1;3B": "alt-down", "\x1b[Z": "shift-tab"}


def split_keys(text):
    keys, i = [], 0
    while i < len(text):
        ch = text[i]
        if ch == "\x1b" and i + 1 < len(text) and text[i + 1] in "[O":
            j = i + 2
            while j < len(text) and not ("@" <= text[j] <= "~"):
                j += 1
            seq = "\x1b[" + text[i + 2:j + 1]
            keys.append(seq)
            i = j + 1
            continue
        keys.append(ch)
        i += 1
    out = []
    for k in keys:  # Esc then an arrow = Alt+arrow on terminals that send it that way
        if out and out[-1] == "\x1b" and k in ("\x1b[A", "\x1b[B"):
            out[-1] = "\x1b[1;3" + k[-1]
        else:
            out.append(k)
    return [NAMES.get(k, k) for k in out]


def piped(args):
    app = App(args)
    for k in split_keys(sys.stdin.read()):
        if k == "\x03":
            break
        app.handle(k)
        if app.done:
            break
    text = "\n".join(app.canvas().lines()).rstrip() + "\n"
    if args.capture:
        with open(args.capture, "w", encoding="utf-8") as fh:
            fh.write(text)
    else:
        sys.stdout.write(text)


def interactive(args):
    import termios
    import tty
    app = App(args)
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)

    def on_term(signum, frame):
        raise SystemExit(128 + signum)
    signal.signal(signal.SIGTERM, on_term)
    signal.signal(signal.SIGHUP, on_term)
    sys.stdout.write("\x1b[?1049h\x1b[?25l\x1b[?7l")
    decoder = codecs.getincrementaldecoder("utf-8")()
    try:
        tty.setraw(fd)
        cols = os.get_terminal_size(sys.stdout.fileno()).columns
        if cols < W:
            app.log(f"terminal is {cols} columns; drawn at {W}, the right edge is cut")
        while not app.done:
            sys.stdout.write(app.canvas().ansi())
            sys.stdout.flush()
            if not select.select([fd], [], [], None)[0]:
                continue
            for k in split_keys(decoder.decode(os.read(fd, 64))):
                if k == "\x03":
                    app.done = True
                    break
                app.handle(k)
                if app.done:
                    break
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
        sys.stdout.write("\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l")
        sys.stdout.flush()
    app.log("answers: " + (", ".join(f"{k}={v}" for k, v in app.answers.items()) or "none"))
    print("answers:", app.answers or "none")
    print("keys and answers written to", os.path.relpath(args.picks))


def main():
    ap = argparse.ArgumentParser(description="PROTOTYPE: M7 power tools (throwaway).")
    ap.add_argument("--rev1", action="store_true", help="rev 1's five scenes instead of rev 2's three")
    ap.add_argument("--rev2", action="store_true", help="rev 2 (the default): blame, history, journal")
    ap.add_argument("--no-colour", action="store_true", help="start with colour off (# toggles it)")
    ap.add_argument("--scene", type=int, default=1, choices=[1, 2, 3, 4, 5])
    ap.add_argument("--picks", default=PICKS_DEFAULT, help="where keys and answers are appended")
    ap.add_argument("--capture", help="piped mode: write the last frame here instead of stdout")
    args = ap.parse_args()
    if sys.stdin.isatty():
        interactive(args)
    else:
        piped(args)


if __name__ == "__main__":
    main()
