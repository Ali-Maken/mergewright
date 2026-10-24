# Mergewright: Design — View mockups

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

### 1.0 Screen chrome (M8, ADR-0058) — supersedes the borders, header and footer of §1.1–§1.3

**Ease pass (ADR-0063; user approved 2026-10-17)** — today's frame kept; the top line's right end adds `on <branch>`, `lanes drawn: N of M` and `K merged branches folded`, the last two only while something is not drawn (Q11 a, Q14 a). Per the mocks in `prototypes/ease-mocks/`, rows in `docs/design-notes/ease-pass.md` "Acceptance".

The pane layouts of §1.1–§1.3 stand (sidebar, graph, detail beside / below / tab). Their outer box, top-border
header and bottom-border footer are replaced by the M8 mock-up's chrome (`docs/shots/m8-proto-3d-first-line.txt`,
judged against the real app in `.scratch/m8-polish-and-ship/real-vs-mockup.md`):

```
 ◆ billing-api  ~/code/billing-api · main                                    reading 4,210 rows
                                                                       ← blank row (≥ 30 rows tall)
▸ ●      [main] [origin/main] Add invoice totals            Ali        2 hours ago   9f2c1ab
  ●      Fix rounding in tax lines                          Lina Park  5 hours ago   41d0e7c
  ●─╮    Merge pull request #212 from acme/csv-export       Tom Ruiz   yesterday     c8a1f02
  ⋮
─────────────────────────────────────────────────────────────────────── ← rule (≥ 30 rows tall)
 ⏎ open · c commit · m merge · r rebase · S stash · . more
```

- **Header:** ` ◆ ` + repo name (bold, accent) + two spaces + `~`-abbreviated path (muted, cut from the **left**
  with `…`) + ` · ` + branch (or `detached <sha7>`). Right-aligned: the informational tail (walk state, filters,
  `gfx`, `+N lanes`); an operation banner replaces the tail as before (§1.13). No product name. In the practice repo
  the glyph is `◇` and the path reads `practice · <tmp> · a throwaway repo, not yours` (ADR-0056).
- **Gutter:** list rows start with two cells: the cursor glyph `▸` (accent, bold; ASCII `>`) and a gap. Header and
  bar are indented one cell. One cell of right margin.
- **Cursor row:** the `selection` fill runs from the marker slot to the right margin; the gutter and the graph
  column keep the ground, so no lane disappears into the fill (KNOWN_GAPS 542). Foregrounds on the fill under 4.5:1
  are lifted by `Theme::legible_on`. The marker slot keeps `▏` for the compare base only; a drop target draws `→`
  (ASCII `=`).
- **Rows ≥ 30:** blank row under the header, rule (`─`, border token) above the bar. Under 30 rows: neither.
- **Bar:** hints joined by ` · `; outside modal contexts every hint keeps its words or is not drawn; the doorway
  (`. more`, `? keys` once `?` is built) is always last among the words, and leads any bare keys a place still draws (ADR-0039). Problem, notice and stop lines take the bar's row as before
  (ADR-0037 §4). Modal contexts keep their own ladders (ADR-0044 §4).

### 1.1 Main graph view — wide (160+ columns, three-column layout)

**Ease pass (ADR-0063; user approved 2026-10-17)** — supersedes the columns below at every width. Columns: `graph | branch | message | who | age`, then `│` and the branch pane (≥120 cols: branch 20, who 6, age 4, pane 36; 100–119: pane 30, branch 14; <100: no pane, branch 12, the cursor row's second line says who/when/branch/hash). No SHA column, no ref badges. Branch column written once where it changes, in its role colour; own rows `you` bold. Right pane: pushed? merged? behind its base? who/last. `⏎` opens the detail pane, which replaces the branch pane while open. Merged branches open folded (`▸ N commits from <branch>`). Per the mocks in `prototypes/ease-mocks/`, rows in `docs/design-notes/ease-pass.md` "Acceptance".

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

**Ease pass (ADR-0063; user approved 2026-10-17)** — see §1.1's ease-pass note; it supersedes this layout.

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

**Ease pass (ADR-0063; user approved 2026-10-17)** — see §1.1's ease-pass note; under 100 columns no pane, branch column 12.

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

**Ease pass (ADR-0063; user approved 2026-10-17)** — inline at every width ≥100: only clash blocks split yours | theirs; the three-pane layout and kept headers are retired. List column 30 (`Clashing files`, `?`/`✓`, `N open`/`done`, `✓ Review the merge y`); below 100, sides stack (today's). A merge or rebase stop lands on the first clash (Q17 a). Per the mocks in `prototypes/ease-mocks/`, rows in `docs/design-notes/ease-pass.md` "Acceptance".

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
render (`crates/mergewright-ui/tests/shots/resolver-*.txt`), which catch drift but
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
        │  Rebase current branch onto…                    r         │
        ╰ ↑↓ move  ⏎ run  esc close ─────────────────────────────────╯
```

Fuzzy search over actions, branches, tags, files and commits at once, grouped
by kind, ranked by match quality then recency.

M8 (ADR-0056): the same widget is what `?` opens, filtered to the focused place (§1.16). Every action row draws its
key in the accent first (6 cells), then the label, then the action's config id muted and right-aligned (dropped
below 100 columns), so the palette teaches the shortcut and the config name at once. `Theme: dark / light / high
contrast` and `Practice tour` are action rows.

M8 (ADR-0061, approved 2026-10-07): **empty box** — `⌃P` with nothing typed lists `COMMANDS RUN RECENTLY` (when any),
then `ACTIONS`; branches, tags, files and commits appear once he types, and the empty field reads, muted, `type to
find actions, branches, tags, files, commits`. **Recent rows** are action rows as above (key, label, config id): the
last 8 distinct actions run from the palette this session, newest first. No argument is stored: `⏎` runs the row as
its key would and asks for the argument again. (The drawing's last row read `git rebase main`, a command with its
argument, until ADR-0061.)

### 1.7 Ghost Preview (overlaid on the graph, before confirming)

**Ease pass (ADR-0063; user approved 2026-10-17)** — the merge preview has no buttons: `y` merges, `Esc` cancels, `⏎` lists the incoming commits; the faint would-be rows stay (Q5 a, Q6 a). Other previews unchanged. Over unrelated histories the box says `Can't merge: <from> and <into> share no history.` and withholds `y` (`m m` and the list's `y` too); `Esc` and `Enter` still work. A fast-forward says `Git combines  nothing: no merge commit is made` and `After y <into> moves up to <from>.` (ease-pass/05s).

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

M8 (ADR-0060, approved 2026-10-07): at the Pixel tier the graph column draws at the Unicode tier while a preview is
open, so ghost and real rows share one renderer and the `┊`/`┈` above is what he sees; the strips return on close.
The header's `gfx` field still names the tier in force. A pixel dashed, low-alpha stroke is past v1.

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
branch at the row's commit, `/` finds, `n`/`N` step matches (or `✗` rows).
A commit an undoable `●` step left behind reads `└ ⇢ replaced` (or `⇢ deleted`),
muted, ending `· undo brings it back`; the legend's `⇢` clause shows only when
the whole legend fits: at this mock's 130 cells it no longer does since the product word became `Mergewright`
(m8/52); on a wider screen (187 columns) it shows. Amended 2026-10-11 (usability Q1, user: 'yes say replaced'):

```
 ┌─ JOURNAL · what happened in this repo, newest first ─────────────────────────────────────────────────────────────────────────────┐
 │   ●  Mergewright    10:44   fetch · origin/main moved                                                                            │
 │   $  outside        10:41   reset main 1 commit back               git: "reset: moving to HEAD~1"                                │
 │ ▸    └ ✗ lost       9a8b7c6 "Add rate limiter" · was main's tip · no branch holds it                                             │
 │   $  outside        10:37   switch from spike to main              git: "checkout: moving from spike to main"                    │
 │      └ ✗ lost       4d5e6f7 "Try redis cache" · was spike's tip · spike is gone                                                  │
 │   ●  Mergewright    10:12   switch from main to spike                                                                            │
 │   ●  Mergewright    10:05   rebase feature onto main               1 ref · backup saved                                          │
 │      └ ⇢ replaced   1c2d3e4 "Add OAuth" · was feature's tip · undo brings it back                                                │
 │ ● done in Mergewright: u can undo it · $ done outside Mergewright: u never touches it · ✗ a commit no branch holds               │
 └──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ⏎ put main back to it  b new branch here  / find  esc close  ? keys  (bar is per row; `? keys` last, m8/11e)
```

An expanded `●` row lists `commands Mergewright ran`, each drawn `┆ git …`; `$`
means only "outside Mergewright".
**Failure mark (ease-pass 26, ADR-0067).** Anything that failed (an action he asked for, or a read a whole line or pane stands for) reads `⊗ <why>`, ASCII `(x) <why>`, the mark in the error colour. One mark app-wide, drawn by the screen, never typed into the words. Not a failure, so not `⊗`: `✗` a commit no branch holds (J), `!` needs attention, `⚠` warning, `ⓘ` info, `?` a value not known (the why is on the bar), `gfx: <error>` the graphics status. Amended 2026-10-24 (user 'yes'; KNOWN_GAPS ease/25-failure-mark-consistency).


### 1.11 Stash list + diff

```
╭ STASHES (2) ──────────────────────────────────────────────────────────────╮
│  stash@{0}  "WIP on main: token refresh"        2h ago   3 files          │
│  stash@{1}  "quick fix before pull"              1d ago   1 file           │
├──────────────────────────────────────────────────────────────────────────┤
│ stash@{0}  src/auth/token.rs  +8 −2                                       │
│  @@ -12,6 +12,12 @@ struct TokenCache                                    │
│  +   retry_count: u8,                                                    │
╰ ⏎ apply  p pop  d drop (undoable)  b branch from stash  space preview  ? keys ╯
```

The footer ends with the keys doorway, `? keys` (m8/11e); short of room it
sheds whole clauses from the middle, never cutting a word.

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

**Ease pass (ADR-0063; user approved 2026-10-17)** — a merge's banner reads `merging <from> into <into>`.

```
╭ ◆ graft ─ ~/code/api ─ REBASING step 3 of 7 ───────────────────────────╮
│ ●     cut release 1.0                                      3d  a82960b │
╰ c continue  s skip  a abort  M mergetool  e edit  ⇥ graph ─────────────╯
```

Shown in place of the ahead/behind indicator in the top bar whenever
`rebase-merge/`, `MERGE_HEAD`, `CHERRY_PICK_HEAD` or `BISECT_LOG` exist; also
the first thing offered on relaunch if Mergewright (or the machine) died mid-op.

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
in Mergewright is taught.

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
points at a small helper that talks back to Mergewright over a local socket/pipe so
the prompt renders in-app instead of falling through to a blocked terminal
prompt or failing silently.

### 1.15 First-run / empty state (redrawn M8, ADR-0056, ADR-0058)

```
 Mergewright · no repository here
 ~/code

  o  Open…     i  Init     c  Clone…     t  practice tour

  RECENT
  ▸ payments-service    ~/code/payments-service     yesterday
    web-dashboard       ~/code/web-dashboard        3 days ago
    dotfiles            ~/dotfiles                  last week

───────────────────────────────────────────────────────────────────────
 o open… · i init · c clone… · t practice tour · ⏎ open the selected · ? keys · q quit
```

- Shown when Mergewright starts in, or is pointed at, a folder that is not inside a repository. `Mergewright` is the product
  name (one constant, `APP_NAME`, read from the names table). The second line is the folder, `~`-abbreviated; a path that does not exist
  reads `~/foo · no such folder` there.
- RECENT: newest first, the newest preselected, so "open what I had yesterday" is `⏎`. Name bold, path muted, age
  muted. Empty list: the RECENT block is not drawn.
- `o` opens a one-field path prompt (`open › ~/code/▏`, `~` expanded, `⇥` completes a directory); `i` asks
  `Make ~/code a Git repository?` with `y`; `c` opens `clone › <url>` with `→ <cwd>/<name>` under it, progress in
  place, errors on the prompt with the URL kept (`⏎ retry · esc cancel`).
- A repository that cannot be opened: the screen stays, the row stays, the bar's row reads the reason.
- Keys and options not built yet are not drawn (a key the screen does not answer is never offered).
- The practice tour replaces the callout tour this section used to describe (ADR-0056).

### 1.16 Help (`?`) — the palette opened on this place (M8, ADR-0056)

The separate key grid this section drew is gone. `?` opens §1.6's palette on the focused place:

```
╭─ keys here ──────────────────────────────────────────────────────────────── esc close ╮
│ › ▏                                                                                   │
│ IN PROGRESS                                                                           │
│▸c     continue the rebase                                         operation.continue  │
│ s     skip this commit                                                operation.skip  │
│ HERE                                                                                  │
│ m     merge the selected branch into yours                               graph.merge  │
│ r     rebase yours onto the selected commit                             graph.rebase  │
│ EVERYWHERE                                                                            │
│ ⌃P    find anything (palette)                                            app.palette  │
│ u     undo the last thing you did                                           app.undo  │
╰───────────────────────────────────────────────────────────────────────────────────────╯
```

- Groups: `IN PROGRESS` (only while an operation is live; a key the operation takes is not listed again under
  HERE), `HERE`, `EVERYWHERE`; `YOUR CONFIG` first when `[keys]`/`[theme]` had problems, one row each
  (`m given to graph.fetch — merge has no key`).
- Row: key (accent, 6 cells) · label · config id (muted, right-aligned; dropped below 100 columns). The eye lands on
  the key column. Unbound: `—`.
- Typing filters labels and hidden search words; a typed chord that is bound ranks its row first. `↑↓` move, `⏎`
  runs, `esc` closes. No match: `nothing matches · esc close`.
- Box over the focused pane, ≤ 96 columns, centred; full width under 100 columns; scrolls.
- It reads the live key table (ADR-0055), so the bar, this list and reality cannot disagree.

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
