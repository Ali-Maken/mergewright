# M8 polish — throwaway prototype

Pipeline step 5 for `docs/design-notes/m8-polish-and-ship.md` § "Prototype brief (step 5)". Fake data, real
keys. Nothing reads or writes git. Delete when the M8 specs land.

```
python3 prototypes/m8-polish-and-ship/polish.py 1               # scroll feel + terminal probe — do this first
python3 prototypes/m8-polish-and-ship/polish.py 2               # finding keys (? and Ctrl-P)
python3 prototypes/m8-polish-and-ship/polish.py 3               # first run and the practice tour
python3 prototypes/m8-polish-and-ship/polish.py 4               # themes, by eye
python3 prototypes/m8-polish-and-ship/polish.py 4 --no-colour   # the same, colour off
```

Run scene 1 in **each** of Ghostty, Zed and VS Code, full screen: it measures the terminal, so each one
needs its own run. The rest: wherever you like; scene 4 in VS Code too (see below).

Your task is at the top, in words; it never names the app's keys. The bar at the bottom names only the
prototype's own keys. When a **QUESTION** shows, answer with its number. `n` goes to the next scene, `q` or
Ctrl-C quits, and the terminal is left as it was.

Everything goes to `prototypes/m8-polish-and-ship/picks.txt` (gitignored) as it happens: your terminal
(`$TERM_PROGRAM`, `$COLORTERM`, its background colour reply), every key, every pause over 3 s (marked
STUCK over 30 s), the measurements below, and your answers. Don't open it until you're done: it names the
blind builds.

## What each scene asks

**1 — Go back about two weeks fast, stop on "bump deps".** Six tries: keyboard, trackpad, wheel, each in
build `1` and build `2` (`1`/`2` switch any time). One build is today's drawing; the other waits for the
terminal to finish each frame before drawing the next, and draws it in one piece. Which is which is
blind. `s` switches the wheel step between 1 and 3 rows (named on screen).
Question: which felt smoother?
Measured per terminal: how long the terminal takes to answer (p50/p99), whether it supports drawing a frame
in one piece (DEC 2026), events per second in each flick, and how long the screen took to catch up after
your last key or flick.

**2 — Four small tasks:** put your changes aside; what does `x` do; a rebase stopped, how do you go on;
what would you write in the config to move merge to another key (`=` opens the answer box).
Question: when the list opened, where did your eye land first?
Logged per task: keys and typed characters against the note's journey count, and the path you took.

**3 — First run.** Open yesterday's project; clone a URL (paste works); then, in a repo you've never opened,
find somewhere safe to practise. Follow the card for three tasks and a drag, then go back to your own work.
Question: whose repository is on screen now?

**4 — Themes.** Two dark candidates side by side (A: ground darker, B: text brighter), then light and high
contrast, then today's dark. Each panel prints its own contrast numbers. Question: which dark reads better?
Then one line, in your words: **where did two different things still look the same?**
Run it once in VS Code too: its "minimum contrast ratio" setting (4.5 by default) can repaint text colours;
say whether the coloured lines change.

## Can't run it? The frames

`docs/shots/m8-proto-*.txt`, one per step (1a–1e, 2a–2g, 3a–3l, 4a–4e). They are plain text: **no colour,
no timing, no feel.** Scene 1's whole point is timing, so its frames show only the layout and the status
line; the smoothness question can only be answered live.

## How it was checked (and what that can't show)

`python3 prototypes/m8-polish-and-ship/selfcheck.py` runs every scene in a pty that plays the terminal:
it answers the frame fence, DEC 2026 and the background query instantly (and in one run answers nothing,
so the 16 ms fallback path runs), sends keys, a bracketed paste and mouse drags, saves the frames above, and
checks the terminal is left clean (alternate screen left, mouse and paste reporting off, cursor shown,
termios restored) after `q`, Ctrl-C and SIGINT. Its timing numbers are meaningless — the fake terminal
answers at once and draws nothing. Colour, smoothness and feel are unverified until you run it.

## Findings while building (for the planner)

- **Search words matter for "put your changes aside".** `stash…` alone doesn't match "aside"; the prototype
  gives rows hidden search words (`aside shelve save park`). Whatever he types in scene 2 says whether the
  real palette needs them.
- **`c` and `a` were listed twice during a rebase** (operation and graph) until filtered: the keymap gives
  them to the operation while one is live (§2.3, §2.4). The real `?` must filter the same way.
- **The blue lane on the cursor row measures 1.2–1.3:1** against the selection in all three darks (Tol
  `#0077BB` on `#5A6478`-ish). No dark candidate fixes it; it needs its own rule (542).
- **Neither dark candidate keeps today's ground and text both:** A darkens the ground, B brightens the text.
  Both clear the 4.5:1 text-on-cursor floor today's dark misses (4.28:1).
- Scene 3's "first time?" line sits in the status row; a success message (e.g. after clone) covers it for 2 s.
  The note says a problem line wins; it says nothing about a success line.
- The wheel scrolls the view and the cursor stays inside it; stopping *on* a commit with the trackpad
  therefore takes a click. Graft does the same (keymap §2.15); the tries will show whether that's a step too
  many.
