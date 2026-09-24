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
    `$EDITOR` (no inline editor is built, ADR-0042), `n`/`N` to
    next/previous block, `j`/`k` between files, `c` to finish (full list and
    the finish summary: §2.6, §2.6b). None of the graph pane's global bindings
    (`p` pull, `r` rebase, `f` fetch, `M` Map, `n`/`N` search) are live
    while it has focus — same letters, different, non-conflicting verbs.
  - The **operation banner**, when focused (a merge, rebase, cherry-pick or
    revert in progress — conflicted or not), binds `c s a` to
    Continue/Skip/Abort — "commit," "stage" and "amend" aren't valid while
    an operation is in progress, so the letters are free and match the
    buttons' initials exactly.
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
  - The **first-run / empty state** (§2.13) binds `o`/`i`/`c` to
    Open/Init/Clone — no repo is open yet, so the whole repo vocabulary
    (`c` commit included) is off duty.
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
  dedicated key.** Rename, delete branch/tag, cherry-pick, reset to here,
  explain/fix a Mistake Radar marker, create branch/tag from an arbitrary
  commit, copy SHA and open-in-browser are all reachable in at most two
  steps without owning a letter.
- **Every binding is remappable** in `[keys]` in `config.toml`; Graft detects
  and warns about remap collisions at startup rather than picking silently.

### 2.2 Global (every pane)

| Key | Action |
|---|---|
| `Ctrl+P`, `:` | Command palette |
| `?` | Context help for the focused pane |
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Esc` | Back out / close overlay / cancel |
| `Ctrl+Z`, `u` | Undo (ADR-0002) |
| `Ctrl+Shift+Z`, `U` | Redo |
| `Ctrl+R` | Manual refresh |
| `q`, `Ctrl+C` | Quit (confirms if an operation is in progress) |
| `↑ ↓ ← →` | Move selection / scroll |
| `j k h l` | Move down/up/left/right (vim, movement only) |
| `g` / `G` | Jump to top / bottom (vim) |
| `/` | Find in the focused list |
| `Enter` | Open / drill in / activate |
| `.` | Context actions menu for the selected item; `.` again or `Esc` closes it |
| `Space` | Toggle stage where staging applies, else expand/collapse |

`Ctrl+Shift+Z` is not a reliable binding: many terminals collapse it to the
same byte sequence as `Ctrl+Z`, so Graft cannot tell the two apart on every
terminal it targets. `U` is therefore not just a convenience alias — it's
the binding that actually works everywhere, and the one the help overlay
and any redo prompt should lead with.

### 2.3 Graph / history pane

| Key | Action |
|---|---|
| `c` | Commit |
| `a` | Amend |
| `m` | Merge selected branch into current |
| `r` | Rebase current onto selection |
| `b` | New branch from selected commit |
| `t` | New tag at selected commit |
| `d` | Discard — only reachable once the detail pane is open (`Enter`); see §2.4. Not yet bound directly on the collapsed Uncommitted-changes row (open ticket) |
| `s` | Stage — same: only reachable once the detail pane is open; see §2.4. Not yet bound directly on the collapsed row |
| `x` | Mark the selected commit as the compare base, or complete/clear a compare against it (PROMPT.md §4 "Diff any two commits") |
| `+` | Expand/collapse lanes folded behind `+N` (§3.5) |
| `S` | Stash… |
| `f` | Fetch |
| `p` | Pull |
| `P` | Push |
| `i` | Add to `.gitignore` |
| `e` | Open in `$EDITOR` |
| `Enter` (on a sidebar branch/tag/stash) | Checkout / open |
| `M` | Toggle Map view (§5.1, §2.3a) |
| `F` | Toggle Focus mode on the selected commit (§5.2) |
| `O` | Toggle the Overview strip (§5.3) |
| `Space` (on a merge) | Fold its merged branch into it / unfold (PROMPT.md §1.3; m1/15) |
| `1` | Toggle first-parent view (m1/15) |
| `H` | Hide / show the stash in the graph (m1/15) |
| `Enter` (on a graph row) | Open the detail pane on that commit (peek (M1) → open details (M2), ADR-0023, m2-details-and-diffs/10) |
| `y` | Copy the selected commit's SHA to the clipboard (OSC 52) (ADR-0023, m1/45) |

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
| `y` | Confirm, in any merge/rebase/operation preview (opened by `m`, `r`, `.` or the name prompt) once its plan has landed. The cursor still rests on Cancel (ADR-0038 §1). Not in a journal revert preview (user ruling 2026-09-23, `docs/field-notes.md`) |
| `Enter` | The choice under the cursor (ADR-0038 §1) |
| `Esc` | Cancel |

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
| `c` / `a` | Commit / amend (as above — same verbs, same keys) |

### 2.5 Diff pane (view options; active wherever a diff is shown)

| Key | Action |
|---|---|
| `w` | Toggle ignore-whitespace |
| `W` | Toggle word-diff granularity |
| `v` | Toggle unified / side-by-side |
| `[` / `]` | Decrease / increase context lines |
| `<` / `>` | Shrink / grow the detail pane on the focused divider, 4 cells a step; no-op with no divider shown (ADR-0028, m2-details-and-diffs/10b) |
| `L` | Load the full file, past the truncation point (PROMPT.md §6.3) |

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
| `d` | Start this file over (`git checkout --merge`), behind a confirm box — not undoable while the operation is open (the box says so) |
| `n` / `N` | Next / previous block, taken ones included; at the file's last block, cross to the next file; past the end, wrap to the first open block (`wrapped · 2 still open`), else `no more conflicts` (P2) |
| `j` / `k`, `↓` / `↑` | Next / previous file in the file list (P7, §2.2's "move selection"); the document scrolls with `n`/`N`, `PgUp`/`PgDn`, `g`/`G`, the wheel |
| `c` | Finish: a merge opens the finish summary (§2.6b); anything else continues. Refused while any file is open; the resolver stays open after the last file is done (P8) |
| `Enter` | Mark this file done and move to the next conflicted file, or close if none left |
| `e` | Open `$EDITOR` at the cursor block |
| `M` | Launch the configured external mergetool on this file |
| `Esc` | Close the resolver — never aborts the operation (the banner's own `Esc` is unchanged) |

Rows marked P2, P3, P6–P8 were approved 2026-09-23 (`docs/field-notes.md`;
`docs/design-notes/m6-conflicts.md` Amendments 1–2; layout ADR-0043).

### 2.6a Operation banner (M5 interim, until the resolver of §2.6 lands)

M5 stops at a conflict without a resolver. The banner above the panes says
what git is standing in the middle of and takes the keys; §2.6's pane
replaces it in M6, keeping the same letters for the same verbs.

**Modal while focused** (§2.1): the banner answers every key itself,
`Ctrl+C` excepted. A stop with anything unmerged opens the resolver (§2.6)
on the first conflicted file, and `Esc` there lands on the banner. A stop
with nothing unmerged focuses the banner. Either way the first key after a
stop goes to the thing that stopped, not to the graph underneath (P5,
approved 2026-09-23). A launch mid-operation keeps the banner.

| Key | Action |
|---|---|
| `Enter` | Opens the resolver (§2.6) |
| `c` | Continue — refused, in words, while any path is still unmerged. For a merge, opens the finish summary (§2.6b) — including a merge begun in the shell (P4, approved 2026-09-23); shell-begun rebase and cherry-pick stay M7 |
| `s` | Skip this commit (rebase and cherry-pick only; a merge says there is nothing to skip) |
| `a` | Abort, back to where the operation started |
| `M` | Launch the configured mergetool on the first conflicted file (§2.6) |
| `e` | Open the first conflicted file in `$EDITOR` at its first marker (§2.4) |
| `Tab` / `Esc` | Give the keyboard back to the panes; the banner stays drawn |

With focus given back, `M` and `e` keep working on the file under the
cursor in the detail pane (§2.4), and `Space` marks it resolved by staging
it. Nothing else here is bound outside the banner.

**`u` and `U` are refused while an operation is live**, focused or not:
`finish or abort the <kind> first · ⇥ banner`. Undo is otherwise global
(§2.2), but an undo on top of an unfinished operation is a second history
write over the first, and the banner is where the first one ends either way.
Commit and amend refuse for the same reason and name the same keys (M5/15).
A bisect draws no banner until M7, so it is not covered by this rule.

### 2.6b Finish summary

Opened by `c` for a merge (§2.6, §2.6a). **Modal**, full frame under the
header like the resolver. Nothing is written until `Enter`. Reuses existing
meanings (§2.2, §2.10), so it adds no letter (P9, approved 2026-09-23;
`docs/design-notes/m6-conflicts.md` Amendment 2 "New UI").

| Key | Action |
|---|---|
| `Enter` | Commit — in the file list and the subject; in the body it is a newline |
| `Ctrl+Enter` | Commit, from any focus (§2.10) |
| `Esc` | Back to the resolver, from any focus; the draft message is kept |
| `Space` | Show / hide the cursor file's diff |
| `j` / `k` | Move between files (§2.2) |
| `Tab` | Move focus files → subject → body → files |
| `Ctrl+E` | Open the message in `$EDITOR` (§2.10) |

### 2.7 Interactive rebase editor

**Modal** (§2.1): this pane answers every key itself, `Ctrl+C` excepted; the
graph pane's global bindings above are not live while it has focus.

| Key | Action |
|---|---|
| `p` `r` `e` `s` `f` `d` | pick / reword / edit / squash / fixup / drop |
| `Alt+↑` / `Alt+↓` | Reorder (also: drag with the mouse) |
| `Enter` | Start the rebase |
| `Esc` | Cancel |

### 2.8 Blame

| Key | Action |
|---|---|
| `Enter` | Jump to the commit in the graph |
| `n` / `N` | Older / newer change to this line |
| `^` | Blame the parent revision (walk up, like `git blame` at a prior commit) |
| `Space` | Toggle the age heatmap |

### 2.9 Stash list

| Key | Action |
|---|---|
| `s` | Save a new stash (message prompt; **modal**, §2.1 — `Tab` switches staged-only vs everything, `Enter` saves, `Esc` cancels) |
| `Enter` | Apply |
| `p` | Pop (apply, then drop) |
| `d` | Drop (undoable) |
| `b` | Create a branch from this stash — **planned, not yet built** (no milestone has claimed it yet; candidate for M5, which owns branch creation) |
| `Space` | Preview the diff |

### 2.10 Commit message editor

| Key | Action |
|---|---|
| `Ctrl+Enter` | Commit |
| `Tab` | Move between subject / body / options |
| `Ctrl+E` | Open `$EDITOR` instead |
| `Ctrl+S` | Insert a sign-off trailer |
| `Ctrl+Shift+C` | Insert a co-author trailer |
| `Esc` | Cancel |

### 2.11 Undo journal browser

| Key | Action |
|---|---|
| `↑ ↓`, `j k` | Move |
| `Enter` | Revert to just before this entry (Ghost Preview first) |
| `Esc` | Close |

### 2.12 Command palette

| Key | Action |
|---|---|
| type | Fuzzy-filter actions, branches, tags, files and commits |
| `↑ ↓` | Move |
| `Enter` | Run |
| `Esc` | Close |

### 2.13 First-run / empty state

A separate screen (no repo is open yet), with its own small vocabulary:

| Key | Action |
|---|---|
| `o` | Open… |
| `i` | Init |
| `c` | Clone… |
| `Enter` (on a recent-repo row) | Open that repo |
| `?` | Replay/skip the first-run tour |

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

---
