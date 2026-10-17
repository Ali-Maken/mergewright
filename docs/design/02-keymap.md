# Graft: Design — Keymap

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [View mockups](01-view-mockups.md) · Next: [Graph layout algorithm](03-graph-layout-algorithm.md)

---

## 2. Keymap

### 2.1 Principles

- **Mnemonic first, one meaning per verb.** A letter is assigned to one verb
  (commit, merge, rebase, branch, tag, stash, fetch, pull, push, discard,
  edit, ignore) and that verb is never reassigned to something else anywhere
  the letter is live. Vim movement (`j k h l`, `g G`, `/`) is additional and
  never collides with a mnemonic, per PROMPT.md §3.
- **Lowercase, UPPERCASE and modal reuse follow one rule** (keymap
  reconciliation, 2026-09-20 — see "History" below for the reasoning):
  lowercase always means one verb for its mnemonic, applied to the cursor
  where the verb has one (a row, file, hunk, line, branch, conflicted hunk)
  and to ambient state (`HEAD`, current branch, all remotes) where it
  doesn't (`c` commit, `a` amend, `f` fetch, `p` pull, `i` ignore).
  UPPERCASE only does one of two jobs: opens/toggles a place or mode over
  the same or a separate screen (`S` `J` `M` `O` `F` `H`), closed by the
  same letter or `Esc`; or reverses one of exactly two direction pairs,
  `u`/`U` and `n`/`N` — no new letter joins that pair by resemblance alone.
  `B`/`X` on the commit-graph offer banner are a named, narrow exception:
  one-shot initials on a banner that appears once per session and claims
  the keyboard the same way a modal pane does. `O`/`T` in the conflict
  resolver (§2.6) are a second one (P6, approved 2026-09-23; ADR-0043):
  whole-file yours/theirs, the bulk form of `o`/`t` that §2.1's modifier rule
  would otherwise put on `Ctrl+O`/`Ctrl+T` — which the per-file-but-no-mark
  take already holds. A slipped `Shift` is recovered with `d`. `p`/`P` (pull/push) are the
  model for any future same-shift-key opposite pair: box the irreversible
  half behind a confirm, never let a slipped `Shift` run it. A hand-over to
  an external program (`M` mergetool) is a place you go to and come back
  from, and joins this set the same way (KNOWN_GAPS row 482).
- **The hint bar is a ranked selection, not an enumeration** (added by the
  conductor, 2026-09-22, from m5/33's exemption list). "The bar always shows
  what's active" — said twice below — means the bar reflects the **current
  mode**, never that every bound key appears on it. The bar has a width
  budget and a `more` affordance; it ranks by relevance at the cursor.
  Outside modal contexts a hint is drawn with its words or not at all; what
  does not fit is behind the doorway (M8, ADR-0058; `?` lists every key of
  the place, §2.12). The doorway ends every bar's words but on the modal
  ones ADR-0058 §4 names; where a place keeps bare keys it leads them
  (ADR-0039). On a marked row it ranks right after `u undo` and `/ search`
  and never displaces them. `u undo` is fitted second in any bar that
  holds it; what gives way is the lowest-ranked hint, not `u` (m8 15 A5).
  **Two classes never appear on it, and that is correct:**
  - **Navigation and meta globals** (§2.2): `g`/`G` top/bottom, `PgUp`/`PgDn`
    page, `Ctrl+R` refresh, `U` redo, `Esc` dismiss. They are live in every
    pane and identical in every context, so a bar slot spent on them teaches
    nothing and costs a slot that changes with the cursor.
  - **A key whose verb is not valid at the cursor**, which is the mode rule
    already stated below, not an exception to it.

  Everything else that is bound **is a candidate for a slot**, and a row or
  ambient verb that never appears anywhere is a finding, not a default.
  `crates/graft/src/update/tests.rs::every_bound_action_is_hinted_live_or_is_named_here`
  is the test that holds this; its `NEVER_HINTED` list is the first class
  above and nothing else. **Adding a row to that list is a spec change and
  belongs here first.**

- **A pane dense enough to need the alphabet for its own vocabulary is
  modal: it answers every key itself, `Ctrl+C` excepted, and nothing
  outside it is live while it has focus.** The hint bar always shows what's
  active, so this isn't a hidden mode. These do:
  - The **conflict resolver** (§2.6) binds `o`/`t`/`b` to ours/theirs/both
    per block, `B` to show/hide BASE, `d` to start the file over, `e` to
    `$EDITOR` (no inline editor is built, ADR-0042), `n` to
    the first open clash, `N` to the previous block, `←`/`→` between
    clashes in the file, `j`/`k` between files, `c` to finish (full list and
    the finish summary: §2.6, §2.6b; for a merge the summary also opens by
    itself when the last file is done, P11). None of the graph pane's global bindings
    (`p` pull, `r` rebase, `f` fetch, `M` Map, `n`/`N` search) are live
    while it has focus — same letters, different, non-conflicting verbs.
  - The **operation banner**, when focused (a merge, rebase, cherry-pick or
    revert in progress — conflicted or not), binds `c s a` to
    Continue/Skip/Abort — "commit," "stage" and "amend" aren't valid while
    an operation is in progress, so the letters are free and match the
    buttons' initials exactly.
  - The **branch picker** (§2.16, M7) is a text field: it claims every key
    but `Ctrl+C`; `↑`/`↓` move, `⏎` switches, `Tab` shows the branch on the
    graph, `Esc` closes. Amended 2026-09-26 (M7, user "all yes").
  - The **palette** (§2.12, M8) is a text field the same way, whether `?` or
    `Ctrl+P` opened it: it claims every key but `Ctrl+C`.
  - **Blame** (§2.8) and **History** (§2.17) are places (M7): full width,
    opened by `B`/`L`/`F`; each answers its own keys and the §2.2 globals,
    and none of the graph's verbs is live in them. `Esc` goes back one
    place; after `⏎` to the graph, `Esc` there returns to the place (once
    the graph's own menu, box, prompt, search, Focus or Explain is closed),
    with the graph or the detail pane focused; an open detail pane is not
    closed first, and `Esc` closes it again only once the trail has ended.
    Amended 2026-09-26 (M7 pass 2b; ADR-0048); the pane clause 2026-09-27
    (m7-power-tools/20i).
  - The **interactive rebase editor** (§2.7) binds `p r e s f d` to
    pick/reword/edit/squash/fixup/drop — git's own convention for
    `rebase -i` todo lines, which every Git user already knows. `r` here is
    "reword," not "rebase," because you're already inside a rebase; none of
    the graph pane's globals are live here either.
  - The **commit message editor** (§2.10) claims every key but `Ctrl+C`.
  - The **stash save prompt** (inside §2.9's `S` → `s`) and the **search
    prompt** (`/`) claim every key but `Ctrl+C` while typing.
  - The **Map, Journal and Stash overlays** (added by the conductor,
    2026-09-22, from m5/42). Each claims the keyboard through its own
    `shown_key` before the graph sees it, which is what a modal pane does —
    they were simply never written down here. They are **not** identical:
    the Map view eats `p`, `d` and `s`; the stash answers them. So the bar
    behind one is built per overlay, not once for all three.
  - The **first-run / empty state** (§2.13) binds `o`/`i`/`c`/`t` to
    Open/Init/Clone/practice tour — no repo is open yet, so the whole repo
    vocabulary (`c` commit, `t` tag included) is off duty. `?` is help there
    as everywhere (M8, ADR-0056).
  - The **Ghost Preview** (§2.3b; added 2026-09-23, m6-conflicts/11). It
    already answered every key and swallowed the rest; with `m`/`r`/`y`
    confirming it now has letters of its own, so it joins this list. The
    graph's `y` (copy SHA) is not live while it is open.
- **An overlay that needs only a cursor is not a modal pane** (decided
  2026-09-20, from the M5/12 captures; closes the gap the `.` menu fell
  through). The modal panes above claim the alphabet because they need
  it for their own vocabulary. The `.` context menu does not: it needs `j`/`k`
  to move, `Enter` to choose, and `Esc` or `.` again to close — movement,
  confirm, and its own opener, not verbs. So it claims **those five and
  nothing else**, and every verb letter stays live and keeps its meaning
  while it is open.

  Three things follow, and they are the reason this is written down rather
  than left to each pane:

  - **The hint bar does not change while the menu is open**, and that is
    correct rather than an oversight. §2.1 says the hint bar always shows
    what's active; while the menu is open the graph's verbs *are* still
    active, so showing them is the truth.
  - **A key the menu displays must actually work when pressed.** A label
    reading `Merge main into feature (m)` promises that `m` merges; it does,
    because the menu never took `m` away. The label teaches a shortcut that
    is live, which is the whole point of showing it.

    This is one case of a rule that isn't only the menu's (extended
    2026-09-21): **trailing parens carry a key, not a word — anywhere the
    user might press what's inside them.** `(forecast)` is a label, not a
    shortcut, and sat in trailing parens anyway; a user reading the preview
    box has every reason to expect `f` does something, because that's what
    parens mean everywhere else in Graft. Scoping the rule to the menu left
    the next surface free to rediscover the collision on its own, which is
    exactly how `(forecast)` got there. Fixing that label is a code change
    (the preview box, not this doc) and isn't done by this note.
  - **`.` closes what `.` opened** (decided 2026-09-21, M5/15). `.` is the
    doorway (ADR-0039), and a key that opens a place closes it again on the
    same letter or `Esc`, exactly as `S`, `J` and `M` do. It needs saying
    because the rule above — every other key closes the menu and falls
    through — sends a fallen-through `.` straight back to the graph pane's
    `.`, which reopens what was just closed. `.` is not a verb, so claiming
    it takes nothing away from the panes underneath.
  - **The menu's contents never depend on terminal width.** It lists
    everything valid in the current context — per ADR-0039, which made `.`
    the doorway — whether or not the hint bar had room for it. Filtering the
    menu against the live hint bar would make a verb discoverable at 160
    columns and invisible at 80, which is precisely the failure ADR-0039
    exists to prevent.

  A future overlay that genuinely needs letters of its own becomes a ninth
  modal-pane entry by amending the list above, not by quietly claiming them.

- **Bulk variants use a modifier, not a different letter.** `Ctrl+A` stages
  everything; plain `a` stays "amend" everywhere. This avoids growing the
  letter bank for "the same verb, but all of them."
- **Rare verbs live behind `.` (context actions) or the palette, not a
  dedicated key.** Rename, delete branch/tag, reset to here,
  explain/fix a Mistake Radar marker, create branch/tag from an arbitrary
  commit, copy SHA and open-in-browser are all reachable in at most two
  steps without owning a letter. On a commit row, Checkout holds the `.`
  menu's first place and Cherry-pick its second, fixed; Checkout dims in place
  on the branch you are on (`Checkout — already on <b>`). Checkout, like the picker's `⏎` and the name prompt's `⇥`, follows one switch rule: instant on a clean tree (`u` goes back); a preview (`y`) only with unsaved edits or a detached target (approved 2026-09-26, pass-2b decision 1; ADR-0048). Cherry-pick has its
  own key, `o` (§2.3), and keeps the menu's second place; it dims in place with
  `o`'s refusal words (already on <b>; a merge). Labels carry their keys
  (`(o)`). `C` opens the branch picker (§2.16). On the uncommitted row the menu gains `Put staged changes into their commits (absorb)` in its first place, dimmed in place (`nothing staged`) when nothing is staged, as Checkout's on a commit row (conductor, 2026-09-27, 09b: `k . ⏎ y` = 4, note's count): `⏎` opens a preview mapping each staged hunk to its commit, `y` writes; a hunk no commit on your branch changed stays staged, in words. Amended 2026-09-26 (M7, user "all yes"). Rows carrying a stash badge keep the older order (conductor, 2026-09-25) (`docs/design-notes/m6-stash-put-back.md` New UI, Prior art "Menu order"; ADR-0045 does not cover it).
- **Every binding is remappable** (M8, ADR-0055). Each binding has an
  action id, `<context>.<verb>` in snake_case from this document's words
  (`graph.merge`, `app.undo`, `operation.continue`), shown beside its key in
  `?`; it is the config's name for it and is never renamed after release.
  `config.toml`: `[keys.<context>] <verb> = "<chord>"` or a list of chords;
  `"none"` unbinds. Graft translates a pressed chord to the action's default
  chord before any pane sees it; a default whose action was moved is
  unbound. Collisions (both actions named, defaults included), unknown
  actions, unparsable chords and reserved chords (`Ctrl+C`) are warned
  about at startup, never fatal: the status row reads `config: N problems ·
  ? shows them` and `?` lists them first. Typed text in a field is never
  remapped. Every surface that prints a key (hint bar, `?`, palette, `.`
  labels, the tour card, the man page) spells it from the live table.
  A line acts where its section is live and never over a place's own keys:
  a place answers its own rows and the keys it handles itself (movement in
  panes and the `.` menu) before anything it falls through to. Each line
  with a problem is one problem, naming every place it changes.
  `confirm.yes` is the one confirm key (§2.3b). With `suspend_on_ctrl_z`,
  `⌃Z` is `app.suspend` and undo keeps `u`. Inside a text field no line
  acts: keys type themselves.

### 2.2 Global (every pane)

| Key | Action |
|---|---|
| `Ctrl+P`, `:` | Command palette, on everything (§2.12) |
| `?` | Help for the focused place: the palette opened on its keys, an operation's keys first while one is live (§2.12; M8, ADR-0056). Every screen, the empty state included |
| `Tab` / `Shift+Tab` | Next / previous pane. `Shift+Tab` runs the `Tab` cycle in reverse, the operation banner included: graph → banner → diff → file list → graph with the pane open (M8/20d) |
| `Esc` | Back out / close overlay / cancel. A failure held while a box was open (drawn with `esc dismiss` once the box closes) is dismissed first; that `Esc` cancels nothing, the next one cancels (ticket 78 Decided 1). After `⏎` from blame or history: back to it (M7, ADR-0048). While a write runs, `Esc` stops it before any overlay's own `Esc` (a text field, a confirm box or a menu keeps its own); so J's, the stash list's and the Map's bars then read `esc cancels` and close by their own letter alone (`J close`, `S close`, `M back`) — one key, one meaning on screen (m8/59, usability walk Major A) |
| `Ctrl+Z`, `u` | Undo (ADR-0002) |
| `Ctrl+Shift+Z`, `U` | Redo |
| `Ctrl+R` | Manual refresh |
| `q`, `Ctrl+C` | Quit. `Ctrl+C` quits at once with no box, except over a write that can't stop, where it boxes exactly like `q` and never signals that write (**Proposed**, M8/13b conductor decision 2026-10-02; not yet seen by the user). `q` confirms only while a write runs ("`q` while a write runs", below). Git stopped mid-operation with nothing running: `q` quits at once, no box; the next launch keeps the banner, which offers continue or abort (§2.6a `q`; approved 2026-10-02) |
| `↑ ↓ ← →` | Move selection / scroll |
| `j k h l` | Move down/up/left/right (vim, movement only) |
| `g` / `G` | Jump to top / bottom (vim) |
| `/` | Find in the focused list |
| `Enter` | Open / drill in / activate |
| `.` | Context actions menu for the selected item; `.` again or `Esc` closes it |
| `Space` | Toggle stage where staging applies, else expand/collapse. In a commit's file list: a directory folds / unfolds; a file has nothing to toggle, so `space` is unbound there and not offered. While a pane has the keyboard (its file list or its diff), the graph's merge fold (§2.3) is not live and not offered: `space` never acts on a screen other than the focused one (Amended 2026-09-27, M7/spacefold; M7/20h-diff-space) |

`Ctrl+Shift+Z` is not a reliable binding: many terminals collapse it to the
same byte sequence as `Ctrl+Z`, so Graft cannot tell the two apart on every
terminal it targets. `U` is therefore not just a convenience alias — it's
the binding that actually works everywhere, and the one the help overlay
and any redo prompt should lead with.

**`q` while a write runs** (M8/13 review Major 2; planner 2026-10-02; approved 2026-10-02, field notes :89). One box,
title `Quit?`, `[ Cancel ] [ Quit ]`, Cancel first, `y` quits (§2.3c). Its line depends on
whether the write can stop halfway:

- **A write that can stop** (a fetch, an undo): `<op> is still running · quit stops it`. `y` stops it, then quits
  (ticket 13, unchanged).
- **A write that can't** (launched `Stoppable::No`: merge, rebase, cherry-pick, revert, the journal's time machine
  and put-back): two lines, `<op> is still running · quit waits for it to finish` and `if it stops at a conflict or
  fails, you stay to see it`. `y` closes the box; nothing is cancelled; the status row reads
  `<op> · quitting when it finishes · esc stays`, and keeps it while the write's progress updates.
  - The write **ends done**: its answer is applied and journaled first (a write with a put-back after it, such as
    edits set aside around a rebase, ends after the put-back), then Graft quits. `u` on the next launch takes it back.
  - The write **stops** (a conflict) **or fails or ends incomplete**: the wait is dropped, and he lands where that
    event always lands (resolver, banner, error row), with its own words. Graft stays open.
  - `Esc` while waiting drops the wait; the write runs on. It comes before the write's own `Esc` line and after a held
    failure's dismiss (§2.2 `Esc`). `q` again while waiting does nothing: the status row already says it.
- Why not a Cancel-only box: a box with no work in it is a dismiss step (ux-principles §3.1), and he would have to
  watch for the end and press `q` again. Why not stop it: a rebase stopped halfway by a signal is the state the M3
  promise rules out. Two keys, `q y`, the same as the fetch box (heuristic 4: one box, one `y`).
- `Ctrl+C` over a write that can't stop does what `q` does: the same box, the same wait. Over the quit box itself it does not quit (**Proposed**, M8/13b, conductor 2026-10-02).
- **A stuck write** (**Proposed**, M8/13b review fix; a stop leaves the operation paused, nothing lost: the user's ruling 2026-10-04, field notes :91): after 10 s of waiting the status row reads `<op> · still running after 10 s · q stop it now · esc stays`. `q` or `Ctrl+C` then opens `Quit now?`, with the lines `<op> is still running · a git hook may be stuck` and `quit now stops it half-way · the next launch shows where it stopped`, and `[ Cancel ] [ Quit ]`; `y` quits. Before 10 s, `q` and `Ctrl+C` while waiting do nothing. Why 10 s: a merge or rebase with ordinary hooks ends inside it, and an earlier offer invites stopping a write that was about to land. The shell then reads one line saying where git was left: paused (the next launch offers continue or abort), landed, or stopped before git could pause it (`u` puts the files back). Graft ends a merge's, cherry-pick's or revert's stuck hook first, so git pauses it itself; a rebase is stopped where it stands and stays paused at its step (**Proposed**, M8/13c–13d).
- **Quit from outside** (SIGTERM, SIGHUP, or the terminal closing; **Proposed**, M8/13b, 13g): the terminal is given back first. A write that can't stop gets up to 60 s to land; then Graft stops it as `q` `y` does (its stuck hook first, so git pauses it), and the same signal again stops it at once. The shell gets the line saying where git was left. When the terminal has closed nothing is printed: the next launch shows where git was left (paused: the banner offers continue or abort; landed: the graph).

### 2.3 Graph / history pane

| Key | Action |
|---|---|
| `c` | Commit |
| `a` | Amend. On HEAD or the uncommitted row: as before; previewed (`y`) only when a branch is built on HEAD (the dependent branch follows). On an older commit of your branch that you haven't pushed: put the staged changes into it, previewed, `y`. On a pushed one, `a` says so in words and changes nothing. Amended 2026-09-26 (M7, user "all yes"); wording 2026-09-27 (M7/06e-keymap) |
| `R` | Open the rewrite editor (§2.7) on the cursor's row. Amended 2026-09-26 (M7, user "all yes") |
| `C` | Open the branch picker (§2.16). Amended 2026-09-26 (M7, user "all yes") |
| `o` | Take the cursor commit onto your branch (cherry-pick), previewed, `y`. Unsaved edits are set aside and put back (ADR-0045). Amended 2026-09-26 (M7, user "all yes") |
| `m` | Merge selected branch into current. Unsaved edits are set aside and put back (ADR-0045). Amended 2026-10-08 (M8/19d) |
| `r` | Rebase current onto selection |
| `b` | New branch from selected commit |
| `t` | New tag at selected commit |
| `d` | Discard — only reachable once the detail pane is open (`Enter`); see §2.4. Not yet bound directly on the collapsed Uncommitted-changes row (open ticket) |
| `s` | Stage — same: only reachable once the detail pane is open; see §2.4. Not yet bound directly on the collapsed row |
| `x` | Mark the selected commit as the compare base, or complete/clear a compare against it (PROMPT.md §4 "Diff any two commits"). Between two branch tips it diffs since they split: `changes on <feature> since it left <main>` (Amended 2026-09-26 (M7, user "all yes")) |
| `+` | Expand/collapse lanes folded behind `+N` (§3.5) |
| `S` | Stash… |
| `f` | Fetch |
| `p` | Pull |
| `P` | Push |
| `i` | Add to `.gitignore` |
| `e` | Open in `$EDITOR` |
| `Enter` (on a sidebar branch/tag/stash) | Checkout / open |
| `M` | Toggle Map view (§5.1, §2.3a) |
| `F` | Toggle Focus mode on the selected commit (§5.2). With the detail pane focused on a file (its row or a line of its diff, the changes pane included): that file's history (§2.17) (M7, ADR-0048) |
| `O` | Toggle the Overview strip (§5.3) |
| `Space` (on a merge) | Fold its merged branch into it / unfold (PROMPT.md §1.3; m1/15) |
| `1` | Toggle first-parent view (m1/15) |
| `H` | Hide / show the stash in the graph (m1/15) |
| `Enter` (on a graph row) | Open the detail pane on that commit (peek (M1) → open details (M2), ADR-0023, m2-details-and-diffs/10) |
| `y` | Copy the selected commit's SHA to the clipboard (OSC 52) (ADR-0023, m1/45) |
| `#` | **Decided (ADR-0063; user approved 2026-10-17)** — `graph.colours_off`: colours off / on for the whole screen until you quit; lines keep their shapes (ease pass Q3 a). Lands in the table with its handler (G): an unanswered row fails `graft`'s dead-key sweep |

**Decided (ADR-0063; user approved 2026-10-17)** — fold default: merged branches open folded (`▸ N commits from <branch>`); `Space` on a merge opens it (`▾`), `Space` on an opened row closes it and puts the cursor on the merge (PROMPT §1.3 default changes; same key, same meaning as the `Space` row above). `C` stays the branch picker (Q1 a) and `x` stays mark/compare (Q2 a); only the bar's words change (`C switch to <branch>`, `x compare`).

### 2.3a Map view

| Key | Action |
|---|---|
| `M` | Open (from the graph pane) or close the Map view |
| `↑`/`↓`, `j`/`k` | Move the selected branch row |
| `Enter` | Seek the graph to the selected branch's tip and close the view |
| `Esc` | Close |

### 2.3b Ghost Preview (merge / rebase)

| Key | Action |
|---|---|
| `m` / `r` | Confirm, when the preview was opened by that same key and its plan has landed (`m m`, `r r`). The cursor still rests on Cancel (ADR-0038 §1); a preview opened any other way gains only `y` — `.` still closes what `.` opened (P1, approved 2026-09-23) |
| `y` | Confirm, in any merge/rebase/operation preview (opened by `m`, `r`, `.` (Checkout only with unsaved edits or a detached target, §2.1), `a` on an older commit or with a branch built on HEAD, `o`, the picker's `⏎` with unsaved edits, the name prompt (its `⇥` switch only with unsaved edits), or the stash list's `Enter`/`p` — amended 2026-09-26, ticket 86; Amended 2026-09-26 (M7, user "all yes")); the rewrite editor's `y` starts it (§2.7) once its plan has landed. The cursor still rests on Cancel (ADR-0038 §1). Not in a journal revert preview (user ruling 2026-09-23, `docs/field-notes.md`). The same `y` confirms every confirm box with one acting choice (§2.3c) and the finish summary's file list (§2.6b) — one confirm key (P12, P15, approved 2026-09-24; ADR-0044 §3) |
| `R` | In a rebase preview showing `already in main: … · dropped`: open the rewrite editor (§2.7) on that rebase, those commits pre-marked `d`; `p` keeps one, `y` starts, `Esc` back to the graph. Same meaning as §2.3 `R` (opens the rewrite editor, §2.1 UPPERCASE opens a place). Not offered in any other preview (planner, 2026-09-27, scout 05) |
| `Enter` | The choice under the cursor (ADR-0038 §1) |
| `Esc` | Cancel |

**Decided (ADR-0063; user approved 2026-10-17)** — the merge preview (`m`) has no buttons: `y` merges, `Esc` cancels, `Enter` lists the incoming commits (`Esc` back to the preview, `y` merges from there). Scoped by preview kind (Q6 a): every other preview keeps its buttons and `Enter` takes the choice under the cursor. No table row: `Enter` stays hard-matched in the preview's handler (Mp).

### 2.3c Confirm box

`←`/`→` choose (clamped, Cancel first, ADR-0038 §1) · `Tab` cycles · `Enter` the choice under the cursor · `y` the box's one acting choice, whatever the cursor is on, and the hint bar leads with `y <verb>` — the box has no footer of its own (P15, approved 2026-09-24; ADR-0044 §3) — **except** the force-push (lease) box, the undo-after-push box (two acting choices: its answer is the cursor's) and the journal revert's box, where `y` is swallowed and not drawn · `Esc` cancels · `Ctrl+C` passes through to §2.2's quit (over a write that can't stop it boxes like `q`; over the quit box itself it does not quit — **Proposed**, M8/13b) · **any other key is swallowed: the box stays** (usability walk M1).

### 2.4 Working tree / staging (detail pane)

| Key | Action |
|---|---|
| `Space` | Stage/unstage the file, hunk or line under the cursor |
| `s` | Stage (alias) |
| `Ctrl+A` | Stage all |
| `d` | Discard, with confirmation and an undoable backup |
| `e` | Open in `$EDITOR` at the current line — on a `CONFLICTS` row, at the first conflict marker |
| `Enter` | On a `CONFLICTS` row: opens the resolver (§2.2's global open / drill in, applied — not a new letter; see §2.6) |
| `M` | On a `CONFLICTS` row only: launch the configured mergetool on that file (§2.6a) |
| `i` | Add to `.gitignore` |
| `c` / `a` | Commit / amend (as above — same verbs, same keys). While an operation is live, `c` is the banner's `c` (§2.6a: a merge opens the finish summary, a rebase begun in Graft continues) and `a` refuses in words (P14, approved 2026-09-24) |

### 2.5 Diff pane (view options; active wherever a diff is shown)

| Key | Action |
|---|---|
| `w` | Toggle ignore-whitespace |
| `W` | Toggle word-diff granularity |
| `v` | Toggle unified / side-by-side |
| `[` / `]` | Decrease / increase context lines |
| `<` / `>` | Shrink / grow the detail pane on the focused divider, 4 cells a step; no-op with no divider shown (ADR-0028, m2-details-and-diffs/10b) |
| `L` | Load the full file, past the truncation point (PROMPT.md §6.3) |
| `B` | Blame this file at this line, as of the commit shown (the file on disk in the changes pane) (§2.8) (M7, ADR-0048) |
| `F` | This file's history, following renames (§2.17) (M7, ADR-0048) |

### 2.6 Conflict resolver

**Modal** (§2.1): this pane answers every key itself, `Ctrl+C` excepted; the
graph pane's global bindings above (`p` pull, `r` rebase, `f` fetch, `M` Map,
`n`/`N` search) are not live while it has focus.

| Key | Action |
|---|---|
| `o` | Take ours (cursor block), then move the cursor to the next open block (P2) |
| `t` | Take theirs (cursor block), then move the cursor to the next open block (P2) |
| `b` | Take both, then move the cursor to the next open block (P2) |
| `Ctrl+O` / `Ctrl+T` | Yours / theirs in every open block of this file, one write; does not mark it done — `Enter` as usual (P3). On a whole-file card or a done file, refused in words. |
| `O` / `T` | Yours / theirs in every open block of this file, mark it done, move to the next file; on a whole-file card, same as `o`/`t` (P6; the named §2.1 exception) |
| `B` | Show / hide BASE (UPPERCASE toggles a mode, §2.1) |
| `d` on a file marked done | Start it over, as before `Enter`: box `Start <file> over`, line `your choices in <file> are dropped`, `y` confirms. Where the stages were not kept: `git can't bring back the conflict in <file>: <reason>` (Times I left :109) |
| `d` | Start this file over, behind a confirm box — not undoable while the operation is open (the box says so). A file with conflict markers: `git checkout --merge`. A file one side deleted or added whole (`UD`/`DU`/`AU`/`UA`), in an operation begun in Graft: its conflict stages and the surviving side's file come back, the box saying `the choice comes back: keep or delete` (approved 2026-09-24). Offered only where it can act (ops' gate); still refused in a merge begun in the shell |
| `n` / `N` | `n`: the first open clash from the top, across files (Decided below). `N`: previous block, taken ones included; at the file's first block, cross to the previous file; past the start, wrap to the first open block (`wrapped · 2 still open`), else `no open clashes left` (with `· ←/→ step through them` when the file holds a taken block) (P2; ticket 05r). On a done file, `N` crosses straight to the next file not done (ticket 17 D6). |
| `j` / `k`, `↓` / `↑` | Next / previous file in the file list (P7, §2.2's "move selection"); the document scrolls with `N`, `←`/`→`, `PgUp`/`PgDn`, `g`/`G` (`Home`/`End` as `g`/`G`; conductor 2026-09-24: they scroll today, kept), the wheel |
| `c` | Finish: a merge opens the finish summary (§2.6b); anything else continues. Refused while any file is open. When every file is done and the summary is not open (after `Esc` from it; a rebase, cherry-pick or revert), the resolver shows the finish row `✓ every file done — c …`, `c` leads the bar, and a click on the row does what `c` does (P8, P11) |
| `Enter` | Mark this file done and move to the next conflicted file. After the last: a merge → the finish summary opens by itself (P11, approved 2026-09-24); a rebase, cherry-pick or revert → stays on it with the finish row (P8); a stash clash (pop, apply, or edits set aside around a rebase, switch, cherry-pick or fast-forward pull — the last two %s) → closes to the changes pane; a pop or set-aside then drops the stash by commit id, no box, `u` brings it back; an apply keeps it (`docs/design-notes/m6-stash-put-back.md` Journeys; ADR-0045) |
| `e` | Open `$EDITOR` at the cursor block |
| `M` | Launch the configured external mergetool on this file |
| `Esc` | Close the resolver — never aborts the operation (the banner's own `Esc` is unchanged) |

**Decided (ADR-0063; user approved 2026-10-17)** — the clash screen (ease pass; Q7 a, Q12 a):

| Key | Action |
|---|---|
| `o` / `t` / `b` | Take yours / theirs / both, then advance to the next open clash **in this file**; a solved file stays on screen with `✓ Every clash in <file> is solved.` and the next step |
| `n` | The first **open** clash from the top, across files (the mock's `nextOpen`); was: next block, taken ones included |
| `←` / `→`, `h` / `l` | Previous / next clash **in this file**, open or taken (§2.2's movement keys; lazygit's `←/→`). Clamped: `first clash in <file>` / `last clash in <file>`; never crosses files. On the bar of a solved file only, as `←/→ clashes` at the end (Q19 a, user 2026-10-17); `?` lists them (planner 2026-10-17, review ease-pass-05 M1, under ADR-0063) |
| `u` | Undo the last pick: that clash opens again, the cursor on it (also from the review, §2.6b). Resolver-local; see §2.6a |
| `y` | In a merge, when every clash is solved: the review (§2.6b), `resolver.review`. Not drawn and not answered in a rebase, cherry-pick, revert or stash clash, where `c` goes on as today. Lands in the table with its handler (P) |
| `c` | Unchanged; in a merge an alias of `y`, not on the bar |
| `Esc` | Pause: back to the graph with the merge's banner selected; `Enter` there goes back to the first open clash (Q7 a). Same `app.back` row; the bar says `esc pause, come back later` |
| `Enter` | Not offered in a merge: picks advance, so there is no "mark done" step; P11's auto-open of the summary is dropped (the review opens on `y`) |

Rows marked P2, P3, P6–P8 were approved 2026-09-23 (`docs/field-notes.md`;
`docs/design-notes/m6-conflicts.md` Amendments 1–2; layout ADR-0043). P11,
the `n` wording and `d` on a whole-file choice were approved 2026-09-24
(`.scratch/m6-conflicts/user-picks-2026-09-24.md`; ADR-0044).

### 2.6a Operation banner (M5 interim, until the resolver of §2.6 lands)

M5 stops at a conflict without a resolver. The banner above the panes says
what git is standing in the middle of and takes the keys; §2.6's pane
replaces it in M6, keeping the same letters for the same verbs.

**Modal while focused** (§2.1): the banner answers every key itself,
`Ctrl+C` excepted. A stop with anything unmerged opens the resolver (§2.6)
on the first conflicted file, and `Esc` there lands on the banner. A stop
with nothing unmerged focuses the banner, except an `edit` stop, which gives
the keyboard to the graph (banner drawn; `⇥` focuses it with the pane closed, one `⇧⇥` with it open): `a` amends the
stopped commit, `space` stages — on the stopped commit or the Uncommitted-changes row, every change, as §2.4's Ctrl+A; on any other row §2.3's fold (conductor, 2026-09-28, 07d D1) — `c` continues (Amended 2026-09-26 (M7, user "all yes")). Either way the first key after a
stop goes to the thing that stopped, not to the graph underneath (P5,
approved 2026-09-23). A launch mid-operation keeps the banner. So does an undo that puts an operation back (ticket 41).

| Key | Action |
|---|---|
| `Enter` | Opens the resolver (§2.6) |
| `c` | Continue — refused, in words, while any path is still unmerged. For a merge, opens the finish summary (§2.6b) — including a merge begun in the shell (P4, approved 2026-09-23); a rebase or cherry-pick begun in the shell continues by handing the terminal to `git <kind> --continue`, so git's own editor can ask for a message (Amended 2026-09-26 (M7, user "all yes")) |
| `s` | Skip this commit (rebase and cherry-pick only; a merge says there is nothing to skip) |
| `a` | Abort, back to where the operation started, behind a confirm box. A merge begun in the shell aborts too (`git merge --abort`), journaled, so `u` puts the merge back as it was (approved 2026-09-24) |
| `M` | Launch the configured mergetool on the first conflicted file (§2.6) |
| `e` | Open the first conflicted file in `$EDITOR` at its first marker (§2.4) |
| `q` | Quit, the same `q` as §2.2, answered from the focused banner too (M8/13 Major 1). A write running (a `c` continue, say): §2.2's "`q` while a write runs" box — a write that can't stop is waited for, then Graft quits. Git stopped, nothing running: quits at once, no box; git's stopped state stays on disk and the next launch keeps the banner, which offers continue or abort (design note 2b). The banner's hint bar draws `q quit` **last**, and drops it first when the row is narrow, as the graph bar's tail does: `q` is bound and is not a navigation global, so it is a slot candidate (§2.1), and a stop is where he lands (P5), so the way out is shown there (heuristic 6). Planner 2026-10-02; approved 2026-10-02 (field notes :89) |
| `Tab` / `Shift+Tab` / `Esc` | Give the keyboard back to the panes; the banner stays drawn. `Tab` lands on the graph, `Shift+Tab` on the diff (the cycle reversed, m8/20d). `Esc` never aborts (KNOWN_GAPS :549; Amended 2026-09-26 (M7, user "all yes")) |

With focus given back, `M` and `e` keep working on the file under the
cursor in the detail pane (§2.4), and `Space` marks it resolved by staging
it. Nothing else here is bound outside the banner.

**`u` and `U` are refused while an operation is live**, focused or not:
`finish or abort the <kind> first · ⇥ banner`, where the clause names `⇥` per pane between him and the banner, or `⇧⇥` per pane the other way, whichever is fewer; ties forward (m8/20d). On the focused banner the reason stands alone: he is there, and `⇥` would take him off it (review m8 words-focus M2). Undo is otherwise global
(§2.2), but an undo on top of an unfinished operation is a second history
write over the first, and the banner is where the first one ends either way.
**One exception** (approved 2026-09-24): in a merge **begun in the shell**,
`u`/`U` undo or redo a step Graft journaled inside that same merge — a
whole-file choice or a start-over (`d`) — which is how a file it overwrote
comes back. Those steps are entries of their own there; a block take (`o`,
`t`, `b` on a marker block) is a file write inside the merge, not an entry.
Anything older than the merge still refuses. A merge begun in Graft folds its
steps into one entry, so `a` is its way back.
Amend refuses for the same reason and names the same keys (M5/15), except at
an `edit` stop, where `a` amends the stopped commit (Amended 2026-09-26 (M7, user "all yes")); `c` is
the banner's `c` (§2.4, P14).
A bisect draws no banner until M7, so it is not covered by this rule.

**Decided (ADR-0063; user approved 2026-10-17)** — `u` inside the resolver and the merge review
undoes the **last pick** of the operation (the clash opens again, cursor on
it), not a journal entry: the refusal above still holds on the banner and
the graph. A merge paused from the clash screen (`Esc`) leads the banner's
bar with `⏎ back to the clashes`.

### 2.6b Finish summary

Opened by `c` for a merge (§2.6, §2.6a, §2.4), and by itself when the last
file of a merge is done in the resolver (P11). **Modal**, full frame under the
header like the resolver, drawn as a preview: the Ghost Preview's rounded
frame, the title `Preview: finish merging <from> into <into>`, its keys in the
frame's bottom border. Nothing is written until `y` (or `Enter` in the
subject, `Ctrl+Enter` anywhere). Reuses existing meanings (§2.2, §2.3b,
§2.10), so it adds no letter (P9, approved 2026-09-23; P12 and the arrow and
page keys approved 2026-09-24; `docs/design-notes/m6-conflicts.md` Amendment
2026-09-24; ADR-0044 §3).

| Key | Action |
|---|---|
| `y` | Commit — in the file list. In the subject and the body `y` is typed text (P12) |
| `Enter` | File list: show / hide the cursor file's diff (P12). Subject: commit. Body: a newline |
| `Ctrl+Enter` | Commit, from any focus (§2.10) |
| `Esc` | Back to what opened it — the resolver (showing the finish row) or the banner — from any focus; the draft message is kept |
| `Space` | Show / hide the cursor file's diff |
| `j` / `k`, `↓` / `↑` | Move between files (§2.2) |
| `PgUp` / `PgDn` | Page the file list (§2.2) |
| `Tab` / `Shift+Tab` | Move focus files → subject → body → files, and back |
| `Ctrl+E` | Open the message in `$EDITOR` (§2.10) |

**Decided (ADR-0063; user approved 2026-10-17)** — for a merge the review replaces this summary: `y`
commits the merge, `j`/`k` move, `Enter`/`Space` open the read-only **look**
at the cursor file (no inline diff; same `finish.toggle_diff` row), `u`
undoes the last pick (§2.6a), `Esc` back to the clashes (`y` brings you
here again). In the look: `Esc` back to the review, `y` commits the merge.
Other operations keep the summary above.

### 2.7 Interactive rebase editor

**Modal** (§2.1): this pane answers every key itself, `Ctrl+C` excepted; the
graph pane's global bindings above are not live while it has focus.

Amended 2026-09-26 (M7, user "all yes"): rows newest first, like the graph; opened by `R` on the cursor's row (§2.3). `fixup!` rows arrive pre-marked and visible (no silent autosquash); `p` un-marks one.

| Key | Action |
|---|---|
| `p` `e` `s` `f` `d` | pick / edit / squash / fixup / drop the row, then move one row older |
| `r` | Reword: a one-line message field under the row (`⏎` keeps, `Esc` drops, `Ctrl+E` `$EDITOR`) |
| `Alt+↑` / `Alt+↓` | Reorder (also: drag with the mouse) |
| `y` | Start, once the AFTER block has landed (one confirm key, §2.3b) |
| `Enter` | Show / hide the row's changes (as §2.6b) |
| `Tab` | Below 120 columns: swap rows ↔ AFTER block |
| `Esc` | Cancel |

### 2.16 Branch picker (M7)

Amended 2026-09-26 (M7, user "all yes"). **Modal** (§2.1): a text field; claims every key but `Ctrl+C`.

| Key | Action |
|---|---|
| typing | Filter |
| `↑` / `↓` | Move the row |
| `⏎` | Switch: instant on a clean tree (`u` goes back); a preview (`y`) with unsaved edits or a detached target; on the branch you are on, closes the picker, cursor on HEAD, nothing written (amended 2026-10-11, usability Q2, user: 'close the picker') |
| `Tab` | Show it on the graph, cursor on its tip, nothing written |
| `Esc` | Close |

### 2.8 Blame

Opened by `B` from the diff pane. Full width. Who, when and why trail the
cursor line. `Esc` goes back (after `^`, to the newer blame). Amended
2026-09-26 (M7 pass 2b; ADR-0048).

| Key | Action |
|---|---|
| `Enter` | Jump to the commit in the graph, its diff scrolled to this line; `Esc` comes back (M7) |
| `/` | Find in the file (M7) |
| `n` / `N` | Next / previous match, as the graph's search (M7) |
| `^` | Blame the parent revision, cursor where this line came from; `Esc` comes back (M7) |
| `L` | History of these lines: the function around the cursor, else its block (§2.17) (M7) |
| `Space` | Toggle the age heatmap |
| `w` | Toggle ignore-whitespace, on by default (§2.5's meaning) (M7) |
| `i` | Toggle the repo's ignore-revisions file; bound only when the repo has one (M7) |

### 2.9 Stash list

| Key | Action |
|---|---|
| `s` | Save a new stash (message prompt; **modal**, §2.1 — `Tab` switches staged-only vs everything, `Enter` saves, `Esc` cancels) |
| `Enter` | Open the **apply preview** for the selected stash (Ghost Preview frame, §2.3b): the files it writes (untracked ones included), whether it may clash with the tree now, `stash@{n} stays in the list`. `y` applies; `Enter` is the choice under the cursor (Cancel first, ADR-0038 §1); `Esc` back to the list. Refused in the box, nothing written, when unsaved, untracked or ignored files are in the way (Graft refuses them before git runs, ADR-0049); an ignored one reads `<path> (ignored) is in the way · move it aside first`, since `S` can't take it. A clash opens the resolver on the first clashed file; the stash is kept (ADR-0045) |
| `p` | Open the **pop preview**: as `Enter`, with `stash@{n} is dropped · u brings it back`. `y` pops. A clash opens the resolver; the stash is kept until the last file is done, then dropped, `u` brings it back (ADR-0045) |
| `d` | Drop (undoable) |
| `b` | Create a branch from this stash — built (m3/m5 name prompt, `update/stash.rs`) |
| `Space` | Preview the diff |

Amended 2026-09-26 (user: "86 yes"; usability walk 2 C): `Enter` and `p` open a preview, `y` goes ahead. Replaces: `Enter` applied and `p` popped at the keypress (m3 ticket 06; kept by `m6-stash-put-back.md` Journeys). See `docs/design-notes/m6-stash-preview.md`.

### 2.10 Commit message editor

| Key | Action |
|---|---|
| `Ctrl+Enter` | Commit |
| `Enter` | In the subject: commit, as in §2.6b (P16, approved 2026-09-24; ADR-0044 §3). In the body: a newline |
| `Tab` | Move between subject / body / options (`Tab` from the subject reaches the body) |
| `Ctrl+E` | Open `$EDITOR` instead |
| `Ctrl+S` | Sign off: on / off |
| `Ctrl+Shift+C` | Insert a co-author trailer |
| `Esc` | Cancel |

**Decided (ADR-0063; user approved 2026-10-17)** — the commit screen: `Tab` moves subject → body →
files (the options row is gone); `Esc` in the body back to the subject, in
the files back to the subject, in the subject closes (message kept for the
next `c` this run); in the files `j`/`k` move, `Space` puts a file in or
out, `Enter` commits. The subject counter is 72 (was 50). The options
(amend, skip hooks, allow empty) have no key: `commit.toggle_amend`,
`commit.toggle_no_verify`, `commit.toggle_allow_empty`, listed from `Ctrl+P`,
or `?` off the text fields (`?` types in the subject and body) (Q8 a);
`Ctrl+S` still signs off.

### 2.11 Undo journal browser

| Key | Action |
|---|---|
| `↑ ↓`, `j k` | Move |
| `Enter` | Per row: `●` revert to just before it (Ghost Preview, `→ ⏎`, unchanged; none on a step Graft doesn't undo, such as a fetch); `$` put that ref back to before it, and on a switch row switch back to the branch it left; `✗` put back what held the commit, or a lost edit's file, replacing what is in that file now (Ghost Preview, `y`; `u` brings it back) (M7) |
| `b` | New branch at a `$` or `✗` row's commit (name prompt; prefilled `<branch>-restored` when the row names one); on a `●` row it says so (M7) |
| `/` | Find a row by the words that name it, as blame's find (§2.8): `Enter` keeps it, `Esc` drops it (M7) |
| `n` / `N` | Next / previous match; with no find, next / previous `✗` row (`⇢` rows passed over) (M7) |
| `u` / `U` | Undo / redo, global as everywhere (§2.2): the journal's next entry (the `← u` / `← U` row), whatever row the cursor is on; never a `$` or `✗` row. Lands as `Enter` on that row (M7) |
| `Esc`, `J` | Close (a find first: `Esc` drops it). While a write runs, `Esc` stops the write instead (§2.2 `Esc`) and `J` alone closes; the bar reads `esc cancels` … `J close` (m8/59, usability walk Major A) |

A `⇢ replaced` / `⇢ deleted` row is a commit an undoable step left behind; `Enter` and `b` act on it as on `✗`. Amended 2026-10-11 (usability Q1, user: 'yes say replaced').

The cursor opens on the newest `✗` row under a `$` row, when that `$` row is
newer than every `●` row except a fetch, until a key is pressed; a `✗` under a
`●` is never preselected, `n` reaches it (M7). Amended 2026-09-26 (M7 pass 2b;
ADR-0048; F1 amended after review M4); u/U in the browser ruled global 2026-09-29 (m7 24-a question). Amended 2026-09-30 (m7 30o: Enter on switch and fetch rows, a lost edit put back over the file as it is, b on ● rows, J closes, / as blame's).

### 2.17 History (M7)

Opened by `L` in blame (those lines) or `F` on a file (the whole file). Full
width. `Esc` goes back. Added 2026-09-26 (M7 pass 2b; ADR-0048).

| Key | Action |
|---|---|
| `j` / `k` | List: older / newer commit. File pane: scroll |
| `Tab` | List ↔ file pane (below 120 columns, swaps the one pane shown) |
| `n` / `N` | Next / previous change in the file pane, from either focus (as the resolver's blocks) |
| `B` | Blame the file as of the selected commit, at the file pane's line |
| `Enter` | The selected commit in the graph, its diff on this file; `Esc` comes back |
| `w` `v` `L` | §2.5, on the file pane (no `[` `]`: a whole file has no context to widen; m7-power-tools/21c) |

### 2.12 Command palette and help (M8, ADR-0056)

One widget, two ways in. **Modal** (§2.1): a text field; claims every key but `Ctrl+C`.

- `?` opens it on **this place**: `IN PROGRESS` (while an operation is live), `HERE` (the focused place's keys),
  `EVERYWHERE` (§2.2); `YOUR CONFIG` first when `[keys]` or `[theme]` had problems.
- `Ctrl+P` / `:` open it on **everything**. Typed: actions, then branches, tags, files, commits, then recently run
  commands. Empty: recently run commands first, then actions; the rest once he types (ADR-0061, approved 2026-10-07).
  A recently run command is an action, run as its key would; no argument is stored (ADR-0061).
  Actions include `Theme: dark`, `Theme: light`, `Theme: high contrast` (this run only) and `Practice tour`.

| Key | Action |
|---|---|
| type | Filter by label and hidden search words (`aside` finds stash). A typed chord that is bound in this place ranks its row first |
| `↑ ↓` | Move |
| `Enter` | Run the row's action (as its key would, in the place `?` was opened from) |
| `Esc` | Close |

### 2.13 First-run / empty state

A separate screen (no repo is open yet), with its own small vocabulary (redrawn M8: §1.15, ADR-0056, ADR-0058):

| Key | Action |
|---|---|
| `o` | Open…: a one-field path prompt (`~` expanded, `Tab` completes a directory) |
| `i` | Init: `git init` in this folder, behind a confirm (`y`) |
| `c` | Clone…: a one-field URL prompt; clones into `<this folder>/<name>` and opens it |
| `t` | Practice tour (§2.18) |
| `Enter` (on a recent-repo row) | Open that repo |
| `↑ ↓`, `j k` | Move in the recent list (the newest is preselected) |
| `?` | Help (§2.12), as on every screen |
| `q` | Quit |

### 2.18 Practice tour (M8, ADR-0056)

The tour claims **no key but `Esc`**, and only when nothing else on screen would take it (no box, prompt, menu,
overlay, search or focused pane): then `Esc` leaves the practice repo and returns to where he was. Every other key is
the app's own, spelled on the card from the live table (ADR-0055).

### 2.14 Credential prompt

**Modal** (§2.1): one field, answers every key itself except `Ctrl+C`, which
still quits (m4-remotes/06). git asks one question at a time (username, then
password), so there is never more than one field open to move between.

| Key | Action |
|---|---|
| type | Insert into the field (masked with `•` when git asked for a password or passphrase) |
| `Enter` | Submit the answer |
| `Esc` | Cancel — stops the job and fails the operation cleanly, nothing left half-authenticated |
| `Ctrl+U` | Clear the field |
| `Backspace` `Delete` `←` `→` `Home` `End` | Edit the typed text |
| `Ctrl+C` | Quit (passes through; the only chord this pane doesn't claim) |

### 2.15 Mouse (every pane, always first-class — PROMPT.md §3)

- Click selects; double-click opens; right-click opens the same context menu
  as `.`, and `.` or `Esc` closes it however it was opened.
- The scroll wheel works in every scrollable area.
- Divider drags resize panes; sizes are remembered per repo.
- Drag a branch badge onto a commit or another branch to open Merge
  into…/Rebase onto…/Reset to here/Cherry-pick, each behind a Ghost Preview.
- In the graph pane, hovering a row whose subject is truncated reveals the
  full subject in the footer, the same reveal `Enter` gives without a mouse
  (§2.3); moving off the row restores the hint bar. There is no tooltip
  (ADR-0023 — supersedes this section's "same tooltip content" and PROMPT.md
  §1.3's hover line): the full message, parents and containing branches live
  in the detail pane, M2.
- Mouse capture never sends `XTSHIFTESCAPE` with `n=1` — `Shift`+drag stays
  the terminal's native text selection, never captured by Graft (ADR-worthy
  rule already fixed by PROMPT.md §3; noted here because it constrains how
  the `term` module configures crossterm's mouse capture).

### History

This doc is the single source of truth for keys (ADR-0037, decision 1); code
follows it, not the other way round. `docs/reviews/keymap-reconciliation.md`
(2026-09-20) found and fixed seven drifts between this doc and the shipped
bindings — `x`, `L`, `+`, the Map view and the stash save prompt were live
and undocumented; `d`/`s` on the collapsed graph row and `b` on the stash
list were documented but unbuilt — and proposed the lowercase/UPPERCASE/
modal rule now in §2.1. Read that review for the reasoning; its tables are
now folded into this file.

2026-09-24 (M6 spec pass after the user's picks): §2.1, §2.3b, §2.3c, §2.4,
§2.6, §2.6a, §2.6b and §2.10 edited for P11, P12, P14–P16 and the approved
batch items (ADR-0044). The code follows as the M6 look-and-finish specs
land; until each lands, the build still does what the doc said before.

2026-10-02 (M8 pass 2a, user approvals of the same day): §2.1 (palette modal; empty state `t`, `?`; remapping by
action id, ADR-0055; bar words-only, ADR-0058), §2.2 (`?` is help on every screen, `Ctrl+P` on everything), §2.12
(palette and help as one widget, ADR-0056), §2.13 (`t` tour, `?` help, list keys, `q`), §2.18 (practice tour, new).
The code follows as the M8 groups land.

2026-10-02 (planner, M8/13 review Major 2 and the banner's `q`; approved by the user 2026-10-02, field notes :89): §2.2
`q` row and "`q` while a write runs" (a write that can't stop halfway: `q y` waits for it, then quits; stays if it
stops or fails), §2.6a `q` row (`q` on the focused banner; `q quit` last on its bar; git stopped with nothing running:
quits with no box, the next launch offers continue or abort). Spec:
`.scratch/m8-polish-and-ship/issues/quit-waits-for-a-write.md`.

2026-10-07 (planner, palette find P-find-1 and P-find-2; approved by the user 2026-10-07, field notes :92): §2.12
`Ctrl+P` line (an empty box opens on recently run commands, then actions; a recent row stores no argument; ADR-0061).
Specs: `.scratch/m8-polish-and-ship/issues/12-palette-find-anything.md`, `12b-palette-find-commits-and-files.md`.

---
