# Graft: Design — View mockups

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Next: [Keymap](02-keymap.md)

---

## 1. View mockups

Sixteen views. The first three show the same graph view at the three
responsive breakpoints from PROMPT.md §2; the rest are shown once, at whatever
width best fits. Colour, weight and italics don't render in ASCII — where they
matter, a legend line says what would be a colour or style in the real UI.
Lane colour is noted with a letter suffix on the node (`●a`, `●b`, …) standing
in for a distinct hue; ref badges are `[name]`; Mistake Radar markers are `⚠`
(warning) or `ⓘ` (info). A merge node uses the same letter on both sides of
its curve (`◉a──╯a`) when the recovered branch name on both parents resolves
to the same identity (§3.4) — the shape of an accidental pull merge, where
the "other side" is just the remote-tracking copy of the branch already
being drawn.

### 1.1 Main graph view — wide (160+ columns, three-column layout)

```
╭─ ◆ graft ─ ~/code/api ─ ⎇ main ↑2 ↓0 ● 3 changes ─────────────────────────────────────── ⌃P Commands ─╮
│ BRANCHES         ⌕ │ GRAPH   REF            SUBJECT                          AUTHOR  WHEN   SHA      │ COMMIT a1b2c3d                  │
│ ● main       ↑2    │ ◌                      Uncommitted changes (3 staged)                           │ Rui M · 2 min ago              │
│ ▾ feature/         │ ●a      [main]         Fix token refresh race            RM      2m     a1b2c3d │ parents 9f8e7d6                 │
│    auth            │ │ ●b    [feature/auth] Add OAuth provider                AK      1h     7f6e5d4 │ Contained in: main              │
│    billing         │ ◉a──╯b  ⚠ [main]       Merge 'feature/login'             RM      3h     3c2b1a0 │                                 │
│ REMOTES            │ ●a                     Bump deps                        RM      1d     0a9b8c7 │ Fix token refresh race          │
│ ▸ origin           │ ◉a──╯a  ⓘ              Merge branch 'main' of origin     RM      2d     9f8e7d6 │                                 │
│ TAGS               │ ●a      [v2.3.0]       Release 2.3.0                     AK      4d     88ee11c │ ▾ src/auth/token.rs   +12 −4    │
│ STASHES (2)        │ ⋮                                                                                │   @@ -40,7 +40,15 @@ fn refresh()│
│ WORKTREES          │                                                                                  │  40   let token = self.cache;   │
│ SUBMODULES         │                                                                                  │  41 - if token.is_expired() {   │
│                    │                                                                                  │  41 + if token.is_expired()     │
│                    │                                                                                  │  42 +     && !refreshing {      │
╰─ ↑↓ move  ⏎ open  space stage  c commit  m merge  r rebase  ⚠ foxtrot merge — . Explain  ? help ───────╯
```

- Left: sidebar tree, filter box at the top (`⌕`).
- Centre: the graph — one row per commit, ref badges inline, Mistake Radar
  marker column between the graph and the ref badge.
- Right: detail pane — commit metadata, containing branches, file tree and
  diff with a pinned hunk header.
- Top bar: repo name, path, current branch with ahead/behind, dirty count,
  and the command-palette hint. A background-task spinner (not shown) lives
  just left of `⌃P Commands` while a job is running.

### 1.2 Main graph view — medium (100–159 columns)

Sidebar collapses to icons; detail pane moves under the graph instead of
beside it.

```
╭─ ◆ graft ─ ~/code/api ─ ⎇ main ↑2 ↓0 ● 3 changes ──────────────────────── ⌃P ─╮
│ ⌕│ GRAPH  REF            SUBJECT                         AUTHOR  WHEN   SHA  │
│ ●│ ◌                     Uncommitted changes (3 staged)                     │
│ ▾│ ●a     [main]         Fix token refresh race           RM      2m    a1b2c│
│ ▸│ │ ●b   [feature/auth] Add OAuth provider                AK      1h    7f6e5│
│ ▸│ ◉a──╯b ⚠ [main]       Merge 'feature/login'             RM      3h    3c2b1│
│ ▸│ ●a                    Bump deps                         RM      1d    0a9b8│
├──┴────────────────────────────────────────────────────────────────────────────┤
│ COMMIT a1b2c3d · Rui M · 2 min ago · parents 9f8e7d6 · Contained in: main    │
│ Fix token refresh race                                                       │
│ ▾ src/auth/token.rs +12 −4    @@ -40,7 +40,15 @@ fn refresh()                │
│  40   let token = self.cache;      41 - if token.is_expired() {              │
╰─ ↑↓ move  ⏎ open  space stage  c commit  m merge  r rebase  ? help ──────────╯
```

### 1.3 Main graph view — narrow (under 100 columns, exactly 80×24)

A single pane with a tab strip. `Tab`/`Shift-Tab` still switch panes; here
that means switching tabs.

```
╭◆ graft ~/code/api ⎇ main↑2↓0 ●3 ─────────────────────────────────── ⌃P ─╮
│[Graph]  Changes  Detail                                                │
│ GRAPH   SUBJECT                              WHEN   SHA                │
│ ◌       Uncommitted changes (3 staged)                                 │
│ ●a      Fix token refresh race               2m     a1b2c3d            │
│ │●b     Add OAuth provider                   1h     7f6e5d4            │
│ ◉a─╯b ⚠ Merge 'feature/login'                3h     3c2b1a0            │
│ ●a      Bump deps                            1d     0a9b8c7            │
│ ◉a─╯a ⓘ Merge branch 'main' of origin        2d     9f8e7d6            │
│ ●a      Release 2.3.0                        4d     88ee11c            │
│ ⋮                                                                       │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
╰ ↑↓ move ⏎ open ⇥ next tab space stage c commit ? help ──────────────────╯
```

### 1.4 Working tree / staging view (detail pane, "Changes" tab)

```
╭ CHANGES ── ⎇ main ── 2 staged, 1 unstaged, 1 untracked ────────────────────╮
│ STAGED (2)                          │ src/auth/token.rs            +12 −4  │
│  [x] src/auth/token.rs      +12 −4  │ ───────────────────────────────────  │
│  [x] src/auth/session.rs     +3 −0  │  38   fn refresh(&mut self) {        │
│ UNSTAGED (1)                        │  39       let token = self.cache;   │
│  [ ] src/auth/mod.rs          +1 −1 │  40 - if token.is_expired() {       │
│ UNTRACKED (1)                       │  40 + if token.is_expired()         │
│  [ ] src/auth/refresh_test.rs (new) │  41 +     && !refreshing {          │
│                                      │  42           self.do_refresh();   │
│                                      │  43       }                        │
│                                      │                                    │
╰ space stage/unstage  s stage  ⌃A stage all  d discard  e edit  c commit ──╯
```

Hunks and lines expand under a file with the same `[x]`/`[ ]` checkboxes,
staged at hunk or line granularity; `Space` toggles whatever is under the
cursor (file, hunk or single line).

### 1.5 Conflict resolver (file list + three panes when wide, card below)

Per ADR-0043 (supersedes ADR-0042, which replaced this section's original
OURS/RESULT/THEIRS columns and `CONFLICTED FILES` strip). **A file list is
always on screen**: every file this operation conflicted on, with its state
(`2 of 3 open`, `⏎ marks done`, `done ✓`, `keep or delete`), cursor file
marked `▸`, files kept as `done ✓` until the operation ends. **At 180 columns
and wider** (`THREE_PANE_MIN`) the list is a column on the left and the file
is **three panes, yours | result | theirs**, rows lined up by git's own
regions (no 3-way diff): plain regions on the same rows in all three, an open
block `max(yours, theirs)` rows, an open block in the result pane a
placeholder (`? open · o t b`) that a take fills. **Below 180** the list is a
strip above the document (one row, else a grid of up to 3 rows ending
`… N more`) and the file is ADR-0042's **card**: OURS and THEIRS side by side
at ≥100 columns, stacked below that. `o`/`t`/`b` rewrite the block **on
disk**; a taken block keeps its header until the resolver closes. Full
reasoning: ADR-0043, `docs/design-notes/m6-conflicts.md` Amendment 2 "New UI".

Full frame below the header (graph and pane both covered, header kept —
`MERGING` / `REBASING step 3 of 7` stays visible): file list, document or
panes, footer (its own complete key list — modal, no `. more`, ADR-0039;
keys: `docs/design/02-keymap.md` §2.6).

**Both frames below are drawn by hand and compressed to fit this page.** They
are the authority on *what is on each row and in what order* — that is the
acceptance — and they are **not** literal terminal output at the size their
label names (the design note says so: "hand-drawn and not to scale"). Which
frame applies is set by width alone (ADR-0043): the file list is always on
screen; at 180 columns and wider (`THREE_PANE_MIN`) it is a column beside three
panes — the user's own terminal is 187, so he sees the `200x50` shape; below
180 it is a strip above the card. Don't build a cell-exact test
against them; it cannot pass, and one build round was spent discovering that.
The layout is pinned instead by golden files generated from the widget's own
render (`crates/graft-ui/tests/shots/resolver-*.txt`), which catch drift but
claim nothing about whether the screen looks right — that is judged by eye, by
the user, on the M6 acceptance capture (CLAUDE.md rule 6).

```
200x50 (≥180 columns: list column + three panes; list = clamp(longest row, 22, 32), panes a third of the rest)
 graft ~/code/api                                                                                       MERGING
 FILES · 2 conflicted        │ ▸ notes.txt · 2 conflicts · 1 open
                             │   yours · main                 │   result                      │   theirs · feature
   gone.txt    done ✓        │    1 one                       │    1 one                      │    1 one
 ▸ notes.txt   1 of 2 open   │  ── not taken                  │ ── 1 of 2 · took theirs       │ ── taken ✓
                             │    2 two from main             │    2 TWO from feature         │    2 TWO from feature
                             │    3 three                     │    3 three                    │    3 three
                             │▏ ── o take                     │ ── 2 of 2 · open              │ ── t take
                             │▏   4 five from main            │      ? open · o t b           │    4 FIVE from feature
                             │    5 six                       │    5 six                      │    5 six
 o yours  t theirs  b both  ^O/^T all  O/T file  ⏎ done  j/k file  n next  B base  e edit  c finish  esc banner
```

```
120x40 (<180 columns: list strip + card; sides paired at ≥100, stacked below)
 graft ~/code/api                                                              MERGING
 files   gone.txt · done ✓    ▸ notes.txt · 1 of 2 open
 ────────────────────────────────────────────────────────────────────────────────────
 ▸ notes.txt · 2 conflicts · 1 open
     1 one
   ── conflict 1 of 2 · took theirs · feature ─────────────────────────────────
     2 TWO from feature
     3 three
▏  ── conflict 2 of 2 · open ── o yours  t theirs  b both ─────────────────────
▏       yours · main                     │       theirs · feature
▏     4 five from main                   │     4 FIVE from feature
▏  ──────────────────────────────────────┴──────────────────────────────────────
     5 six
 o yours  t theirs  b both  ^T all theirs  O/T file  ⏎ done  j/k file  n next  e edit  esc banner
```

- **One name per side, everywhere** (ADR-0043 §4): `<role> · <name>` —
  `yours · main` / `theirs · feature` on a merge, `upstream · main` /
  `yours · 3f2a1b Add OAuth provider` on a rebase (cherry-pick `picked`,
  revert `reverting`, stash pop `stashed`). Git's raw marker labels and the
  rebase explainer line are gone; the names say who owns which side.
- **BASE** (`B`, on demand): below 180 columns a third, stacked
  `BASE · merged common ancestor` section inside every open block; at 180+
  the block's base lines fill the result placeholder, unnumbered, headed
  `base`. `base unavailable after
  edits` / `no base (added on both sides)` shows in its place when it can't
  be attached (base is matched by byte-equality against git's own diff3
  blocks, never guessed).
- **Whole-file shapes** (deleted/added/binary — UD/DU/AU/UA/DD): the document
  is the file as it is (or empty), one card at the top: `deleted by them · o
  keeps the file  t deletes it` — `o`/`t` resolve and mark it in the same
  press.
- **Line numbers** (ADR-0043 §5): every pane numbers its own lines, no gaps —
  each side as the file would read with that side taken everywhere; the
  result counts an open block as its yours lines; placeholders and marker
  rows draw none. In the card, plain and taken lines carry the result's
  numbers, the two sides their own. A taken block keeps its
  header (`took ours`, keys reduced to the other takes) until the resolver
  closes; reopening shows plain lines.
- Mouse: a click on a list entry selects that file; a click on a block's
  row (any pane) makes it the cursor block; the wheel scrolls all three
  panes together.
- Text too long for a pane is cut with `…` (no horizontal scroll yet).
- No text editor is built here: `e` hands the file to `$EDITOR` at the
  cursor block, `M` to `git mergetool`; the resolver re-reads the file on
  return, so what's on screen is always what's on disk.

### 1.6 Command palette

```
        ╭ ⌃P  rebase                                                ╮
        │ ACTIONS                                                   │
        │  Rebase current branch onto…                    r         │
        │  Rebase (interactive) current branch onto…                │
        │ BRANCHES                                                   │
        │  feature/rebase-cleanup                                    │
        │ COMMANDS RUN RECENTLY                                      │
        │  git rebase main                                           │
        ╰ ↑↓ move  ⏎ run  esc close ─────────────────────────────────╯
```

Fuzzy search over actions, branches, tags, files and commits at once, grouped
by kind, ranked by match quality then recency.

### 1.7 Ghost Preview (overlaid on the graph, before confirming)

```
│ ●a      [main]          Fix token refresh race            RM   2m   a1b2c│
│ ┊●a┈    [main] (ghost)  Rebase feature/auth onto main      —    —    —   │
│ ┊│┊●b┈  (ghost)         Add OAuth provider                 AK   1h   —   │
│ │ ●b    [feature/auth]  Add OAuth provider                 AK   1h  7f6e5│
│ ◉a──╯b                  Merge 'feature/login'               RM   3h  3c2b1│
╰ Preview: rebase feature/auth onto main (2 commits replayed) ──────────────╮
│ git rebase main                                                          │
│                                              [ Confirm ]   [ Cancel ]    │
╰────────────────────────────────────────────────────────────────────────╯
```

Ghost nodes/edges are drawn dashed and dimmed (`┊`/`┈` stand in for a lower
alpha, dashed stroke in the real renderer); real nodes stay solid so the
before/after reads as one continuous picture, not two separate graphs.

### 1.8 Mistake Radar marker + Explain

```
│ ◉a──╯b  ⚠ [main]  Merge 'feature/login'                    RM   3h  3c2b1│
├─────────────────────────────────────────────────────────────────────────┤
│ ⚠ FOXTROT MERGE                                                          │
│ This merge's first parent (9f8e7d6) is not main's previous tip           │
│ (0a9b8c7). main's first-parent line was rewritten — usually by pulling   │
│ on main and pushing the result, rather than merging in the normal way.   │
│                                                              [ Explain ] │
│ [main]: 0a9b8c7 ──╮                                                     │
│                    ◉ 3c2b1a0  (this merge)                              │
│         9f8e7d6 ──╯                                                     │
│                                                                [ Fix… ] │
╰───────────────────────────────────────────────────────────────────────╯
```

`[ Explain ]` and `[ Fix… ]` are buttons (mouse-clickable) reached from the
keyboard through `.` (context actions) on the marked row — Explain owns no
letter, because `e` means "open in `$EDITOR`" everywhere the graph vocabulary
is live (§2.3). `Fix…` opens the options from PROMPT.md §1.4 (revert with
`-m 1`, reset with a backup ref, or rebase instead), each behind its own
Ghost Preview.

### 1.9 Interactive rebase editor

Amended 2026-09-26 (M7, user "all yes"): rows **newest first**, like the graph;
opened by `R` on the cursor's row. `fixup!` commits arrive **pre-marked and
visible** beside their target (no silent autosquash); `p` un-marks one. `y`
starts, once the AFTER block has landed; `Enter` shows a row's changes
(`docs/design-notes/m7-power-tools.md` "Where I'd differ" 1; keys §2.7).

```
╭ INTERACTIVE REBASE onto main (4 commits, newest first) ── Alt+↑/↓ reorder ╮
│  squash  0a9b8c7  Add OAuth provider tests                               │
│  reword  3c2b1a0  Add OAuth provider                                     │
│  fixup   7f6e5d4  fixup! Fix token refresh race     (pre-marked · p un-marks) │
│  pick    a1b2c3d  Fix token refresh race                                 │
│                                                                            │
╰ p pick  r reword  e edit  s squash  f fixup  d drop  ⏎ changes  y start  esc cancel ╯
```

### 1.10 Undo journal browser

```
╭ UNDO JOURNAL ── ~/code/api ──────────────────────────────────────────────╮
│ ▸ 2m ago    Rebase feature/auth onto main         3 refs · backup saved  │
│   14m ago   Discard src/auth/refresh_test.rs       1 file · backup saved │
│   1h ago    Commit "Add OAuth provider"            HEAD moved            │
│   3h ago    Merge 'feature/login' into main        1 ref · backup saved  │
│   1d ago    Push main → origin/main                not locally undoable │
╰ ⏎ revert to before this  (Ghost Preview shown first)  esc close ─────────╯
```

M7 (ADR-0048; keymap §2.11) adds shell (`$`) rows from the reflog and lost
(`✗`) rows under the event that lost them; `⏎` acts per row kind, `b` makes a
branch at the row's commit, `/` finds, `n`/`N` step matches (or `✗` rows):

```
 ┌─ JOURNAL · what happened in this repo, newest first ─────────────────────────────────────────────────────────┐
 │   ●  Graft          10:44   fetch · origin/main moved 2 commits                                              │
 │   $  outside Graft  10:41   reset main 1 commit back               git: "reset: moving to HEAD~1"            │
 │ ▸    └ ✗ lost       9a8b7c6 "Add rate limiter" · was main's tip · no branch holds it                          │
 │   $  outside Graft  10:37   switch from spike to main              git: "checkout: moving from spike to main"│
 │      └ ✗ lost       4d5e6f7 "Try redis cache" · was spike's tip · spike is gone                               │
 │   ●  Graft          10:12   switch from main to spike                                                        │
 │  ● done in Graft: u can undo it · $ done outside Graft: u never touches it · ✗ a commit no branch holds      │
 └──────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ⏎ put main back to it  b new branch here  / find  esc close          (bar is per row)
```

An expanded `●` row lists `commands Graft ran`, each drawn `┆ git …`; `$`
means only "outside Graft".

### 1.11 Stash list + diff

```
╭ STASHES (2) ──────────────────────────────────────────────────────────────╮
│  stash@{0}  "WIP on main: token refresh"        2h ago   3 files          │
│  stash@{1}  "quick fix before pull"              1d ago   1 file           │
├──────────────────────────────────────────────────────────────────────────┤
│ stash@{0}  src/auth/token.rs  +8 −2                                       │
│  @@ -12,6 +12,12 @@ struct TokenCache                                    │
│  +   retry_count: u8,                                                    │
╰ ⏎ apply  p pop  d drop (undoable)  b branch from stash  space preview ────╯
```

### 1.12 Blame view

```
╭ BLAME src/auth/token.rs ── age heatmap ─────────────────────────────────╮
│ a1b2c3d  RM  2m    40   let token = self.cache;                        │
│ a1b2c3d  RM  2m    41   if token.is_expired() && !refreshing {         │
│ 7f6e5d4  AK  1h    42       self.do_refresh();                         │
│ 0a9b8c7  RM  1d    43   }                                               │
│ 88ee11c  AK  6mo   44   self.notify_listeners();                       │
╰ ⏎ open commit  / find  n/N next match  ^ blame parent  space heatmap ───╯
```

M7 (ADR-0048; keymap §2.8): full width, a place (`Esc` back); the cursor
line trails `◂ Name · age · subject`; `L` opens §1.17 on those lines.

Colour (not shown) ages from a bright "recent" hue to a dim "old" hue, redundant
with the printed relative age so it never depends on colour alone.

### 1.13 Operation banner (mid-rebase, focused)

```
╭ ◆ graft ─ ~/code/api ─ REBASING step 3 of 7 ───────────────────────────╮
│ ●     cut release 1.0                                      3d  a82960b │
╰ c continue  s skip  a abort  M mergetool  e edit  ⇥ graph ─────────────╯
```

Shown in place of the ahead/behind indicator in the top bar whenever
`rebase-merge/`, `MERGE_HEAD`, `CHERRY_PICK_HEAD` or `BISECT_LOG` exist; also
the first thing offered on relaunch if Graft (or the machine) died mid-op.

**The banner says the state and the step, and nothing else** (m5/20, the
user's ruling). It does not name a conflicted file: the changes pane lists
every one of them under `CONFLICTS (n)`, and a 24-row screen cannot afford
to say it twice. It carries no `⚠` either — a stopped rebase is a state to
finish, not a failure, and the glyph keeps its force for the Mistake Radar's
real findings. One wording at every width: the earlier `step 3 of 7` / `3/7`
pair came off a `width >= 100` tier that was fed the banner's **own** region
rather than the screen's, so the wider terminal drew the terser spelling.
The `[ Continue ] [ Skip ] [ Abort ]` row is gone (m5/16) — nothing answered
a click on it, and the keys are taught on the footer, where every other key
in Graft is taught.

### 1.14 Credential prompt (in-app askpass)

```
        ╭ Credentials for https://github.com ───────────────────╮
        │ Username  git                                          │
        │ Password  ************************                     │
        │                                                         │
        │                          [ Submit ]      [ Cancel ]     │
        ╰ ⇥ next field  ⏎ submit  esc cancel ───────────────────╯
```

`GIT_TERMINAL_PROMPT=0` is set on every git invocation, and `GIT_ASKPASS`
points at a small helper that talks back to Graft over a local socket/pipe so
the prompt renders in-app instead of falling through to a blocked terminal
prompt or failing silently.

### 1.15 First-run / empty state

```
╭ ◆ graft ──────────────────────────────────────────────────────────────────╮
│                                                                            │
│                 This folder isn't a Git repository yet.                  │
│                                                                            │
│                       [ o  Open… ]   [ i  Init ]   [ c  Clone… ]         │
│                                                                            │
│                 Recent repos                                              │
│                   ~/code/api                                              │
│                   ~/code/graft                                            │
│                                                                            │
╰────────────────────────────────────────────────────────────────────────╯
```

A 30-second, skippable first-run tour runs the first time a real repo opens
(not shown here — it's a sequence of four callouts pointing at the hint bar,
the graph, the sidebar and `Ctrl+P`, dismissible with `Esc` or `?` at any
point and never shown again unless replayed from the palette).

### 1.16 Context help overlay (`?`)

```
╭ HELP — Graph view ────────────────────────────────────────────────────────╮
│ MOVE      ↑↓/j k  Enter open   Esc back   Tab next pane                  │
│ ACT       c commit   m merge   r rebase   b branch   t tag   S stash     │
│ REMOTE    f fetch   p pull   P push                                       │
│ UNDO      ⌃Z / u undo   ⌃⇧Z / U redo                                     │
│ FIND      / search   ⌃P command palette                                  │
│ ITEM      . context actions for the selected row                          │
│                                                        remap in config → │
╰ esc close ─────────────────────────────────────────────────────────────╯
```

`?` always shows the keymap for whichever pane has focus — it's the same
table as §2 below, filtered to the current context, so the hint bar and this
overlay are never out of sync with each other or with reality.

### 1.17 File history (`L` from blame, `F` on a file) — M7

Added 2026-09-26 (ADR-0048; keymap §2.17). Left: the commits that changed
the file or the lines. Right: the whole file at the selected commit, its adds
and removes marked in place; `┃` brackets the traced lines.

```
 HISTORY · src/auth/session.rs · fn refresh_token, lines 18–29 · 4 changes
   7f6e5d4  Handle expired tokens       you    0d  +1 −1 │ a1b2c3d · Lina · 7 months ago · Retry refresh once on 401 · this file +3 −1
   c4d5e6f  Move session TTL to config  Priya  4mo +1 −1 │      17      /// Refresh the token when the server says it expired.
 ▸ a1b2c3d  Retry refresh once on 401   Lina   7mo +3 −1 │ ┃    18      pub fn refresh_token(&mut self, client: &Client) -> Result<()> {
   5e6f7a8  Add token refresh           Tom   10mo +12   │ ┃    19  +       let mut attempt = 0;
                                                         │ ┃    21              let resp = client.refresh(&self.token)?;
                                                         │ ┃        −           if resp.status() == 401 {
                                                         │ ┃    22  +           if resp.status() == 401 && attempt == 0 {
 j/k older·newer  tab file  n/N next change  B blame here  ⏎ that commit  esc back
```

Widths: ≥180 list 56 cells; 120–179 list 40 (initials); <120 one pane,
`Tab` swaps. Colour (green/red, unverified) doubles the `+`/`−` column and the
removed rows' missing line number.

---
