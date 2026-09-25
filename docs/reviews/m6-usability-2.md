# M6 stop — whole-app usability walk, after picks 15–17

Third walk under CLAUDE.md "Every milestone ends with a whole-app usability walk". Reviewer, read-only, against
`29290cb8`. The previous walk (`docs/reviews/m6-usability-final.md`, against `1b1995cc`) led to picks 15–17 and tickets
67–75 and 82–84. This walk checks them, then walks the daily tasks again on every screen. Scored against
`docs/research/ux-principles.md` (**UX §n**; heuristics **H1–H10**).

**User context used:** 187 columns, Unicode tier, dark theme, Zed's terminal (field notes :49, :57). Daily tasks from
memory `graft-daily-workflows` and the field notes: history, stage and commit, stash and pop, push/pull including a
rejected push and a dirty tree, rebase, merge, resolve a conflict and finish, switch branch, cherry-pick, undo.

## How this was checked, and what was not

- **Driven:** the release `graft` binary, built in a throwaway detached worktree at `29290cb8`, run under a pty (pyte,
  187×24 to 187×40, plus 150 columns for the card layout; Unicode renderer forced by config). The driver answers the
  terminal's cursor-position query this time, so the pty exit seen in the last walk did not recur. Scratch repositories:
  a bare origin, a `work` clone with `main`, `feature` and `fix` (conflicting line 2 of `notes.md`) and a local `main`
  commit, and a `mate` clone that pushed a teammate's commit. One 5000-commit repo for scrolling. Commit dates fixed so
  row order is stable. The screen was dumped after each key group. "Observed" below means seen in these dumps.
- **Behaviour:** each behavioural finding names its file:line, read at `29290cb8` (rule 7). Where no line was found, it
  is under Questions.
- **Not run:** gates, heavy tier, perf. This is the usability walk; the milestone review runs those.
- **Colour and look: not judged (rule 5).** The selection fill was detected by its background value (`5a6478`) only to
  know which row the cursor was on. No finding rests on colour.
- **Not reached:** mouse and drag; `⌃⏎` in Zed; 80- and 120-column layouts; a real network remote; `$EDITOR` round trip.

## Daily tasks, walked

Counts per UX §1.3: keys plus decisions, a typed string 1, a repeated motion 1. "Note" is the design note's count
(`docs/design-notes/m6-conflicts.md:490-525` fence; `docs/design-notes/m6-stash-put-back.md:37-49`). ✓ holds, ✗ fails
(finding id), – not applicable.

| Task | Keys observed | Note | H1 | H4 | H6 | H9 | 3.1 |
|---|---|---|---|---|---|---|---|
| Scroll history, then back out of nothing | `j`… `esc` → header `stopped at 6 rows` | – | ✗ A | ✗ A | – | ✗ A | – |
| Read a commit's diff | `j ⏎` **2** | – | ✓ | ✓ | ✓ | – | ✓ |
| Stage one file, commit, from launch | `k ⏎ j j space c <msg> ⏎` **8** | 6 from the changes row | ✓ | ✓ | ✓ | ✓ | ✗ m-h, Q3 |
| Push | `P y` **2** | 2 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Push rejected → recover, dirty tree | `p r r P y` **5** | 5 | ✓ | ✓ | ✓ | ✓ | ✗ prev m5 |
| Pull with rebase, dirty tree | `p r r` **3** | 3 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Pull fast-forward over overlapping edits | `p` → refused | none | ✗ D | – | ✗ D | ✗ D | – |
| Stash pop that clashes → done | `S p o ⏎` **4** | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Stash apply that clashes → done (`stash@{1}`) | `S j ⏎ o ⏎` **5** (4 + the `j`) | 4 | ✓ | ✗ C | ✓ | ✓ | ✓ |
| Stash clash → put it all back | `esc u y` **3** | 2, +`y` after a take | ✓ | ✓ | ✓ | ✓ | ✓ |
| After a stash clash, read the file in the changes pane | `S ⏎ o esc` → pane shows `<<<<<<<` | – | ✗ B | – | – | – | ✗ B |
| Switch branch with clashing edits | `. ⏎ y o ⏎` **5** | 5 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Check out a branch (menu) | **3** | 3 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Check out the branch you are on | `.` → cursor on dimmed `Checkout — already on main`, `⏎` refuses | – | ✓ | – | ✗ m-g | – | – |
| Cherry-pick (menu) | `. j ⏎ y` **4** | 4 | ✓ | ✓ | ✓ | ✓ | ✗ prev m5 |
| Merge, one conflict, whole file | `m m O y` **4** | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Merge, one conflict, with `o` (bar's lead key) | `m m o ⏎ y` **5** | 4 | ✓ | ✓ | ✓ | ✓ | ✓ (prev n4) |
| Abort a stop | `a y` **2** → `aborted: rebase fix onto feature ✓` | 2 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Undo with nothing to undo | `u` → `nothing to undo ✓` | – | ✗ m-j | – | – | – | – |

## Critical

None. No path found that loses work without a way back. Every stash drop observed (pop, set-aside) said
`u brings it back`, and `u` did.

## Major

**A — `Esc` on a plain graph cancels the history walk, says `stopped at N rows`, and leaves a long history
unscrollable.**
- Task: any. `Esc` is the key he presses to "get out" when unsure (keymap §2.2 `:165`, "Back out / close overlay /
  cancel").
- Screen: graph header; graph body.
- Principles: H1 (says the history stopped when it did not), H3, H9 (no named way back), UX §3.2 (a mode he cannot
  see: the walk is invisible once loaded).
- Evidence:
  - With nothing open, `escape_key` falls through to cancelling the walk: `if let Some(id) = next.awaiting.take() {
    commands.push(Command::CancelJob(id)); … Phase::Cancelled }` (`crates/graft/src/update/keys.rs:518-523`). A walk
    stays `awaiting` for as long as it is on screen (KNOWN_GAPS.md :148), so this fires on a fully loaded history.
  - The header then draws `stopped at {laid_out} rows` (`crates/graft-ui/src/graph_screen/layout.rs:430-432`).
    Observed on a 6-commit repo that had read `6 commits` a moment before: `stopped at 6 rows`. Also `stopped at 10
    rows` on a 10-commit repo.
  - After that, `move_cursor` sends `ShowRows` only while `next.awaiting` is `Some`
    (`crates/graft/src/update/keys.rs:470-476`). Observed on the 5000-commit repo: `esc` then `G` drew a blank graph
    pane with the row keys still on the bar (`stopped at 500 rows`).
  - Only `⌃R` recovers. Nothing on screen names it.
- Direction: `Esc` with nothing to back out of does nothing. Cancelling a walk only while it is visibly loading
  (`reading…`), and the header says `stopped · ⌃R reloads` when it does.

**B — After a take in the resolver, the changes pane shows the file's old conflict markers.**
- Task: resolve a stash clash (the put-back landing) or any conflict, then look at the file before committing.
- Screen: resolver → changes pane.
- Principles: H1 (the pane contradicts the file on disk), UX §3.1 (the landing shows a stale state).
- Evidence:
  - Walked `S ⏎ o esc`: the resolver closed onto the changes pane, whose diff still showed `<<<<<<<` / `=======` /
    `>>>>>>>`. `notes.md` on disk had no markers. `⌃R` redrew it correctly.
  - `close()` does not refresh the detail pane (`crates/graft/src/update/resolver/leave.rs:50-63`).
  - `show_cursor` serves a cached diff keyed on the entry and section only (`crates/graft/src/update/detail/ops.rs:293`),
    so the pre-take diff is a cache hit. `refresh` clears the cache (`:68-80`); it runs on `⌃R`
    (`crates/graft/src/update/keys.rs:433`), after an editor handover (`update/mod.rs:390`) and after an operation
    (`update/operate.rs:57`), not after a take.
- Direction: a take (or resolver close) invalidates the detail pane's diff cache for that path. Test: take, close,
  assert the pane's diff has no marker lines.

**C — `⏎` in the stash overlay applies the stash: a write on the key that opens everywhere else.**
- Task: look at a stash, then decide.
- Screen: stash overlay (`S`).
- Principles: H4 (`⏎` opens on the graph, the sidebar, the changes pane; keymap `:204` "Checkout / open"), H5, and
  the keymap's own rule that `y` confirms every preview. Escalates the previous walk's m4 from bar-read to observed.
- Evidence:
  - `KeyCode::Enter => operate_on_selected(next, commands, next_id, Apply::Keep)`
    (`crates/graft/src/update/stash.rs:213`). Keymap §2.9 `:381` specifies it.
  - Observed: `space files` showed `M notes.md +1 −1`. The next `⏎`, read as "open that file", applied the stash with
    no preview and landed in a clash on the resolver.
  - `esc u y` put it back, so nothing was lost; hence Major, not Critical.
- Direction: this is a keymap change (§2.9), through the conductor. Either `⏎` opens the selected stash's diff
  (lazygit shows it beside the list, `docs/research/commit-stash.md` §1) and apply moves to `a` with a `y` preview, or
  apply keeps `⏎` but opens the same forecast preview `p` would, confirmed by `y`.

**D — A fast-forward pull over overlapping unsaved edits refuses, in git's terms. Known row, re-observed.**
- Task: pull while working (daily; memory `graft-daily-workflows`).
- Screen: status line.
- Principles: H9 (the next step names no key), H4 (pull with rebase and switch set the edits aside; this pull does
  not).
- Evidence:
  - Observed: `pull refused: would overwrite notes.md (uncommitted) · commit or stash first`
    (`crates/graft-ops/src/apply/remote.rs:252`).
  - `sets_aside` covers `Rebase` and `Checkout` only (`crates/graft-ops/src/operation.rs:833-836`).
  - Already `KNOWN_GAPS.md` :806 (M6/0045, **broken**, M7). Counted here because it is the one daily pull path that
    still sends him to the shell's vocabulary. The words also carry the previous walk's m7.
- Direction: as the row says, M7. Until then, name the keys: `S s stash first · p r r pulls with rebase`.

## Minor

**m-a — `o`/`t` swap roles between screens.** Merge and pop: `o yours  t theirs|stashed`. Rebase: `o upstream
t yours`. Set-aside: `o branch  t yours`. Documented (ADR-0043 `:59-64`: keys follow pane position, words follow the
role), so not a defect against the spec. Filed for H4: in a rebase and a set-aside, "take mine" is `t`, in a merge
and a pop it is `o`. The words on the bar are right each time; motor memory is not.

**m-b — The uncommitted header names the wrong commit as HEAD.** `working_tree_header` labels the graph's first row
`HEAD` (`crates/graft/src/view/detail.rs:169-180`; its test at `:284` assumes "HEAD is the topmost row"). Observed:
`diff vs HEAD 7ddba9d` (`fix`'s tip) while HEAD was `main` at `c5e7748`; and `diff vs HEAD 44df609`, a stash commit.
H1. Direction: read HEAD's id from the repo facts, not the row order.

**m-c — The detail header reports checks that did not run.** `evil merge: unchecked`
(`crates/graft-ui/src/detail_pane/header.rs:50`) on stash commits: the radar adds an `EvilUnchecked` line for any
commit with 2+ parents (`crates/graft/src/view/radar.rs:166-194`), and a stash commit always has them. Previous N1
excluded stashes from the probe, not from this line. CLAUDE.md "a status field must never read as an error when
nothing is wrong".

**m-d — The resolver's file column changes width with its state words, shifting all three panes.**
`crates/graft-ui/src/conflict_resolver/files/column.rs:18-31` sizes the column to the widest current label. Observed:
`notes.md  1 of 1 open│` → `notes.md  ⏎ marks done│`, and every pane moved one column after the take (compare
`yours · main` header positions). H4 (things that don't move read as stable). Direction: size to the widest label the
state can take.

**m-e — Reopening a taken file shows the result in both side panes.** After a take, `esc`, and reopen, the left and
right panes both drew the taken text; the right pane's header still read `stashed · stash@{0}` over `main`'s line.
ADR-0043 `:107-112` says the side panes are projections of the file, so this may be the design; filed because the
header names a source the pane no longer shows (H1). Direction: on reopen, re-read the sides from the index stages
while they exist, or say `taken` in the side headers.

**m-f — The card layout's taken card keeps a `┴` with nothing above it.** 150 columns, one conflict, after `t`: the
two-column split is gone, the closing rule still reads `──────┴──────` (textual, from the capture). Direction: draw a
plain rule once the card is single-column.

**m-g — Checkout on the branch you are on.**
- The `.` menu on HEAD's row opens with its cursor on the dimmed `Checkout — already on main`; `⏎` refuses. H6: the
  cursor should start on the first live item.
- The refusal `already on main · esc dismiss` stays on the bar after the menu closes.
- `m` on HEAD says `already on it`, with no next step. H9.
(Observed; no line traced for the menu's start index, so the first bullet is textual.)

**m-h — Section headers are cursor stops in the changes pane.** `UNTRACKED (1)` and the other headers take the
cursor and draw `select a file` as the body. From launch the commit journey is `k ⏎ j j space c <msg> ⏎`, 8, against
the note's 6 from the changes row. H1/H6. (Observed; the list's selection rule was not traced — filed textual.)

**m-i — `u` inside the resolver is swallowed silently.** `_ => {}` after `_ if plain => scroll_key`
(`crates/graft/src/update/resolver.rs:362-382`), by keymap §2.1's modal rule. `u` is undo on every other screen, and
the resolver's bar does not say it is off here. H4, H9. Direction: `u` in the resolver answers `esc first — u undoes
from the graph`, the way `c` answers a refusal.

**m-j — `nothing to undo ✓`.** A check mark on a non-event (H1). Direction: no mark.

**m-k — Untracked-only stash: the counts disagree.** The overlay row reads `1 file`, `space files` shows nothing, and
the graph's detail for that stash says `no changes`. H1. (Observed; not traced.)

**m-l — Two names for the stash drop.** The done line says `set-aside copy dropped`; the journal's undo row says
`undo: drop stash@{0}`. H4 (CONTEXT.md picks one term).

**m-m — Journal row text runs together.** `failed: aborted · 1 ref · the index · the woundone`. Known row M6/68
plus a truncation that eats the space between `working tree` and `undone`. Textual.

**m-n — The tab word goes stale.** A sticky refusal `⇥ ⇥ banner` stays on the bar after focus has reached the banner.
The words are right now (previous N6); the line outlives them. Previous walk's M6/70 question, now observed.

**m-o — `?`, `:` and `⌃P` do nothing.** No help and no palette (keymap §2.2 lists the palette). Previous Q2, observed.

## Questions (no line proves them)

- **Q-a.** The detail header reads `force-pushed: not checkable — no remote-tracking reflog` on `main`, whose
  `origin/main` does have a reflog in these repos. Is the check reading the right ref? Not traced.
- **Q3** (stands). The app opens with the cursor on row 0 (the newest branch tip, `fix` here), not on HEAD and not on
  the uncommitted row. Which is the intended landing?
- **Q-b.** The stash overlay lists only the file names in its preview. lazygit shows the selected stash's diff beside
  the list; GitUp opens it with Return (`docs/research/commit-stash.md` §1–2). Is the list-only view the intended
  design? It is what makes C's `⏎` read as "open".

## Look, compared

- **Resolver** against `docs/research/conflict-visuals-and-finish.md` §1–4: three aligned panes with a file list;
  `?` on open files, `▶`/`┃` on the taken side and `┊` on the untaken one (KDiff3's per-file state glyph, JetBrains'
  band going dotted once applied). The shapes match what the study took. Colour not judged (rule 5).
- **Stash overlay** against `docs/research/commit-stash.md`: a full-screen list with no diff. lazygit shows the stash's
  content next to the list (§1); see Q-b.

## The previous walk's findings

| Id | Then | Now | Evidence |
|---|---|---|---|
| N1 radar flags bookkeeping commits | Major | **partly** | No `ⓘ`/`⚠` on rebased or stash rows observed. The detail header still says `evil merge: unchecked` on stash commits (m-c) |
| N2 abort reads as done | Major | **fixed** | Observed: `aborted: rebase fix onto feature ✓` |
| N3 stash pop clash → shell | Major | **fixed** | Observed: `S p` lands on the resolver; `o ⏎` → `every file done · stash@{0} dropped ✓ · u brings it back` |
| N4 rebase preview then refuses | Major | **fixed** | Observed: preview reads `your edits: 1 file set aside…`; `p r r P y` pushed, 5 |
| N5 box shows the old status | Major | **fixed** | Observed: force and drift boxes draw their own keys |
| N6 `⇥` words | Major | **partly** | Words now name the target; the sticky line outlives them (m-n) |
| N7 / M5 checkout, cherry-pick | Major | **partly** | Fixed menu slots: checkout 3, cherry-pick 4 keys, as the note counts. Own keys are M7 |
| M6 journeys outside conflicts | Major | **partly** | Stash, pull-dirty, switch-dirty counted and walked (table). Push not walk-gated |
| m1 unknown-key rules | Minor | **not re-checked** | – |
| m2 `S`/`J` silent from the pane | Minor | **stands** | Observed: `S` from the changes pane did nothing |
| m3 three confirm rules | Minor | **stands** | Observed: `b topic ⇥ ⏎` checked out with no preview |
| m4 stash `⏎`/`p` | Minor | **escalated** | Now Major C |
| m5 only `--continue` lands | Minor | **stands** | Observed after commit, rebase and cherry-pick; merge finish lands |
| m6 `p pull first` | Minor | **partly** | `p` now offers `m`/`r` on divergence |
| m7 "commit or stash first" | Minor | **stands** | Observed (D), `remote.rs:252` |
| m8 `subject empty` on open | Minor | **stands** | Observed |
| m9 every branch is risky | Minor | **stands** | Observed: `ⓘ UNPUSHED COMMITS` on `main` after every local commit, `ⓘ` on every branch |
| m10 commit-graph offer | Minor | **stands** | Observed |
| m11 `(M5)` on screen | Minor | **stands** | In code, `remote.rs:248`; not reached |
| n1 untracked `bin` | Minor | **fixed** | Observed: `new`, content shown once |
| n2 `c continue` vs `c finish` | Minor | **fixed** | Observed: `c finish` on the merge banner |
| n3 keys in three places | Minor | **partly** | Preview and boxes use the bar; the name prompt still draws its keys twice |
| n4 `o` costs a key | Minor | **stands** | `m m o ⏎ y` is 5 against 4 |
| n5 stash internals in history | Minor | **stands** | Observed: `index on` / `untracked files on` rows drawn |
| n6 force box names SHAs | Minor | **stands** | Observed |
| Q1 stash entry after resolve | Question | **closed** | Dropped by commit id with `u` back (N3) |
| Q2 palette | Question | **stands** | m-o |
| Q3 launch cursor | Question | **stands** | Observed on row 0 |
| Q4 `u y` drift and the edit | Question | **answered** | Observed: `U brings back your edit to app.txt` |

**Closed: 8 of 29** (N2, N3, N4, N5, n1, n2, Q1, Q4). Partly: 6 (N1, N6, N7/M5, M6, m6, n3). Stands: 13. Escalated:
1 (m4 → C). Not re-checked: 1 (m1).
