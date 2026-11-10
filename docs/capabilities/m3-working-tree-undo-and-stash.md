# Capabilities — M3: working tree, undo and stash

This milestone made Graft safe to use for real, daily git work, not just to
look at history. Every write the app makes to the working tree, the index or
a ref — staging, discarding, committing, amending, stashing — goes through one
seam that snapshots what it's about to touch and records what it did in a
journal, so `u` reliably takes the last change back and `U` brings it forward
again, file bytes included. A changes pane lets you stage or discard by file,
by hunk, or by line; a commit editor with a real `$EDITOR` escape hatch lets
you write the message; a stash overlay lets you save, list, apply, pop and
drop stashes; a journal browser lets you see and act on the history of what
Graft has done to your repo. The milestone's promise, proven end to end on a
real repository: **you cannot lose work**.

## Capabilities

### M3-01. Every working-tree write goes through one seam that snapshots first

```meta
id: M3-01
area: undo
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** Internal — every stage, unstage, discard, commit, amend, stash push/apply/pop/drop dispatches as an `Operation` through `mergewright_ops::apply`.
- **Behaviour:** Before running, the seam reads current repo facts (`RepoFacts`) and takes a snapshot; the snapshot backs up whatever the operation is about to touch as a git commit under `refs/graft/backup/<worktree>/<ulid>` whose tree has exactly two sub-trees, `index` and `worktree` (`git ls-tree <commit>` shows only those two names). A worktree-touching operation's backup ref points at an after-commit whose parent is the before-commit. The operation only actually runs after the snapshot succeeds. Every apply is proven **parity**: applying through Graft's own code on one fixture copy and running the exact same `git` command line (from `Operation::plan`) on a second copy produce byte-identical `for-each-ref`, `ls-files -s` and `status --porcelain=v2` (excluding the backup refs themselves).
- **Proven by:**
  ops-execution acceptance suite (ticket 03, all on `copy_into_temp` fixtures asserting repo state by reading git, never by trusting the return value) — parity-every-kind test, backup-commit-shape test
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §Interfaces and data shapes, §Hard-to-reverse choices; ADR-0010 (journal per worktree, backup refs namespaced); ADR-0031
- **Status:** works

### M3-02. `u` undoes the last operation; `U` redoes it — byte-for-byte

```meta
id: M3-02
area: undo
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** `u` / `U`, anywhere in the app, not just inside a pane.
- **Behaviour:** `u` emits `Command::Undo(_, force: false)`. On success, sets a status line (the entry's own description, e.g. `undo: stage src/a.rs` — not `undid: …`, since the description already carries the `undo:`/`redo:` prefix) and refreshes exactly the scope the undo touched (`Applied::refresh`: `WorkingTree` or `Everything`). `U` mirrors it with redo. `u`/`U` while a write is already in flight emit nothing (status stays `Busy`). For every operation kind, taking a full snapshot before the op, applying, then undoing reproduces byte-for-byte: `for-each-ref` (excluding `refs/graft`), `ls-files -s`, and every worktree file's bytes including untracked files. Redo then reproduces the post-op snapshot. Undo again reproduces the pre-op snapshot. Proven concretely on: an untracked binary file (bytes `[0, 255, 10, 13, ...]`) discarded then undone comes back byte-identical; an unstaged edit to a tracked file, same; a file both staged and unstaged, same, both index and worktree bytes. Discarding one file and then creating an unrelated new file: undo restores only the discarded file, leaves the new one alone (path scoping).
- **Proven by:**
  `mergewright-ops` ops-execution undo/redo parity suite (ticket 03): "Undo restores exactly, every kind"; "The PROMPT §3 test verbatim"; "Path scoping"
  `mergewright::update::undo::tests::u_asks_for_an_undo_and_says_so`
  `mergewright::update::undo::tests::capital_u_asks_for_a_redo`
  `mergewright::tests::it::undo_by_key::u_brings_a_discarded_untracked_file_back_byte_for_byte`
  `mergewright::tests::it::undo_by_key::u_takes_a_commit_back_and_capital_u_puts_it_again`
- **Looks like:** `docs/shots/m3-09-undo-drift-box-80x24.txt`, `docs/shots/m3-08-journal-after-unstage-80x24.txt`
- **Decided in:** PROMPT.md §3 ("you cannot lose work"); `docs/design-notes/m3-working-tree-undo-and-stash.md` §Interfaces, §Genuinely risky groups; ADR-0002 (`Ctrl+Z` is also undo when configured)
- **Status:** works

### M3-03. Undo refuses safely when the repo moved under it (the drift guard)

```meta
id: M3-03
area: undo
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** Automatic — `u`/`U` when something changed since the operation ran.
- **Behaviour:** If a ref the entry touched has moved (e.g. someone ran `git update-ref` by hand), undo returns `UndoError::Drift` naming the ref; the app opens a confirm box quoting `Drift::explain()` (e.g. "the index changed"). `→ Enter` forces it (`Undo(_, force: true)`); plain `Enter` on the "Stop" choice does nothing. With `force`, a moved ref specifically returns `RestoreError::RefMoved` and touches nothing else — a CAS (compare-and-swap) guard, not a blind overwrite. If the worktree itself changed (a file edited after the operation), undo reports `Drift { worktree: true }`; forcing it proceeds, and the drift's own backup preserves the edit that was overwritten (nothing is silently lost, it's recoverable from the entry's own backup). An undo failure is shown as a sticky status line, not a normal one, so the refresh the failure itself triggers doesn't wipe the reason a moment later.
- **Proven by:**
  ops-execution acceptance: "CAS guard", "Drift guard" (ticket 03)
  `mergewright::update::undo::tests::drift_opens_a_box_that_quotes_what_moved_and_answers_nothing_yet`
  `mergewright::update::undo::tests::enter_on_stop_changes_nothing_and_enter_on_undo_forces` (per ticket 06 acceptance)
- **Looks like:** `docs/shots/m3-09-undo-drift-box-80x24.txt`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §Hard-to-reverse choices
- **Status:** works. Known gap: a forced undo isn't pinned to the exact entry the drift box named — between the box being drawn and `U`/force being pressed, the journal tip could move and the force could apply to a different entry (needs `undo(ctx, force, aim)` with an `Aimed` error; signature change, not built this milestone).

### M3-04. A crashed or interrupted operation is recoverable, not silently lost

```meta
id: M3-04
area: undo
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** Automatic — checked at every launch and on refresh.
- **Behaviour:** If Graft (or the machine) dies between writing a journal's "pending" line and its outcome line, the next launch finds it in `Journal::unresolved()`. `recover_interrupted` (not a bare `Journal::mark_interrupted` — this is the actual recovery path, sealing the entry's before/after trees behind its backup ref) appends an `Aborted` outcome line and shows an "interrupted" notice in the UI until acknowledged with `J` (opening the journal). The notice survives every re-read in the meantime. An `Aborted` entry that has a backup can still be undone (`u` on it restores the backup); it's marked `← u` in the journal browser exactly when it's the next thing undo would act on. A `StashPop` that fails mid-way (e.g. because applying conflicts) similarly leaves an `Aborted` entry carrying `worktree_before`, and `undo` on it restores the pre-pop tree.
- **Proven by:**
  ops-execution acceptance: "Crash between Pending and outcome", "Failed midway is recoverable" (ticket 03)
  `mergewright::view::journal::tests::an_entry_recovery_closed_is_marked_interrupted_not_failed`
  `mergewright::tests::it::undo_by_key::an_interrupted_operation_is_announced_until_j_and_its_row_says_interrupted`
- **Looks like:** `docs/shots/m3-08-journal-after-unstage-80x24.txt`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §Journal entry; ADR-0031
- **Status:** works

### M3-05. The journal browser: every operation Graft has done, and what to do with it

```meta
id: M3-05
area: undo
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** `J` opens it; `j`/`k` move; `Space` expands a row to show its exact git command(s); `Enter` on the next-undoable row runs `u`; `Enter` on the next-redoable row runs `U`; `Enter` on any other row sets a notice and does nothing; `Esc`/`q` closes.
- **Behaviour:** Rows are journal entries, newest first. The row that `u` would act on next is marked `← u`; the one `U` would act on is marked `← U`. `Enter` acts only where a key already would (same rule as undo/redo, not a general jump-to-any-point) — pressing it on an older row shows a notice instead of doing anything ("multi-step revert comes with a later milestone's Ghost Preview," not built here). An `Aborted` entry with a backup reads `failed: <git's first error line> · backup saved`; an interrupted one reads `interrupted`. `j`/`k` clamp at the ends (no wraparound). A background focus event or `Ctrl+R` re-reads the journal (another Graft process on the same worktree could have appended to it); the browser opens with what it already has and swaps in the fresh read when it lands.
- **Proven by:**
  `mergewright::view::journal` tests: newest-first order and the `← u` mark, an undo entry as the `← U` target, an interrupted entry, an aborted entry leading with git's error and still marked `← u` (5 tests per ticket 06 notes)
  `mergewright::update::journal` tests: open+re-read, notice taken up, `Esc` cancels an in-flight read, `j`/`k` clamp, `Space` expands one row at a time, `Enter` on `← u` runs undo and closes, `Enter` on an older row sets the notice (8 tests per ticket 06 notes)
  `mergewright-ui::journal_screen` widget tests over `TestBackend` (6 tests)
- **Looks like:** `docs/shots/m3-journal-160x48.txt`, `docs/shots/m3-journal-80x24.txt`, `docs/shots/m3-journal-expanded.txt`, `docs/shots/m3-08-journal-after-unstage-80x24.txt`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §New UI "Journal browser"
- **Status:** works. Known gap: the browser has no status line of its own while open, so a write started from it (or an older-row notice) can be invisible while `J` is up — same defect `StashView::notice` was given a fix for, not applied here.

### M3-06. The changes pane: staged / unstaged / untracked, at a glance

```meta
id: M3-06
area: working-tree
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** Automatic pane showing working-tree status; sections `STAGED (n)`, `UNSTAGED (n)`, `UNTRACKED (n)`.
- **Behaviour:** A file that is both staged and unstaged (partially staged) appears as a row under both sections with identical flags — which section a row is under, not the file's own status flags, decides whether `Space` on it stages or unstages (two rows for the same file can mean opposite writes). `Space` on a section heading acts on every file under that heading up to the next heading. `Ctrl+A` stages every unstaged and untracked file, but skips a conflicted (unmerged) path deliberately — `git add` on an unmerged path marks it resolved with its conflict markers still in the file, and resolving conflicts is a later milestone's job. A second `Space` while a write is already running does nothing and the status line reads busy. An empty working tree shows an empty list plus a body message (not a literal "nothing to commit" list row — a known wording deviation from the original spec). The pane's own footer hint (`space stage  d discard`) is drawn ahead of movement-key hints so a narrow terminal drops other hints first.
- **Proven by:**
  `mergewright::update::operate::tests::stage_selected_is_a_noop_while_a_write_is_already_running`
  `mergewright::update::operate` tests: `stage_all_skips_the_paths_git_add_would_refuse`, `operated_done_refreshes_the_working_tree_without_restarting_the_walk`, `operated_done_on_everything_emits_the_same_commands_as_ctrl_r`, `operated_failed_part_way_reports_the_error_and_still_refreshes`
  `mergewright::view::detail::assemble_working_tree` unit tests (ticket 04 acceptance)
  `mergewright::tests::it::changes_pane_writes` — real-runner: staging a hunk, discarding an untracked file
- **Looks like:** `docs/shots/m3-changes-pane-160x48.txt`, `docs/shots/m3-changes-pane-120x40.txt`, `docs/shots/m3-changes-pane-80x24.txt`, `docs/shots/m3-01-stage-file-80x24.txt`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §New UI "Changes pane"; ADR-0034
- **Status:** works. Known gap: `Esc` doesn't clear a `Failed` status line — it just sits until the next status replaces it.

### M3-07. Staging and discarding at hunk and line granularity

```meta
id: M3-07
area: working-tree
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** With the diff body focused (unified mode only — see below): `Space` on a hunk heading or a context line stages/unstages the whole hunk; `Space` on a `+`/`-` line stages/unstages just that line; `d` opens a discard confirmation at any of those granularities.
- **Behaviour:** The patch cut for a hunk or line write comes from `hunk_patch`, applied to git's own raw diff bytes (`DetailState::diff_raw`) — never derived from the rendered/drawn view, so what's staged is exactly what git would compute. Direction and side follow the section: staging goes forward from worktree to index, unstaging goes reverse from index, discard is always reverse (destructive). Untracked files always stage whole-file (`git add -N` isn't used — a narrower "stage this line of an untracked file" isn't supported, see Deliberately not built). A line whose hunk has no trailing newline at end of file refuses with a named error (`PatchError::NoNewlineBoundary`) pointing at the `@@` heading — whole-hunk staging still works for that same hunk; only the single-line path is refused, deliberately, rather than silently corrupting the file via `git apply --cached`'s own newline-fusing behaviour. Side-by-side diff view has no line cursor, so hunk/line staging there refuses with a message telling you to press `v` for the unified view first; whole-file staging and untracked-file `Space` are unaffected in side-by-side.
- **Proven by:**
  `mergewright::update::detail` diff-focus tests: `Target::Patch` on a heading equals `hunk_patch(raw, h, WholeHunk)`; a `+` line gives `Lines([i])`; a context line gives the enclosing hunk (ticket 04 acceptance)
  `mergewright::tests::it::changes_pane_writes` real-runner: `Space` on a hunk heading puts only that hunk in the index (asserted via `git diff --cached`)
  ops-execution `hunk_patch` oracle test against `git diff-tree -p` and `git apply --cached` (ticket 01 acceptance)
- **Looks like:** `docs/shots/m3-02-stage-hunk-80x24.txt`
- **Decided in:** ADR-0034 (M3 design deltas)
- **Status:** works. Known gap: unstage-hunk and discard-hunk have no key-to-repository (real git, real keypress) test — proven only by pure/scripted tests; stage direction is the one proven end to end.

### M3-08. Discard with a confirmation box, per file, hunk or line

```meta
id: M3-08
area: working-tree
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** `d` on a file row, or in the diff body on a hunk/line, opens a confirm box; the box defaults to the safer choice under the cursor.
- **Behaviour:** The box states the consequence in plain words specific to the target: an untracked file says it will be deleted; a file both staged and unstaged says both changes return to `HEAD`; otherwise it names which side (index or worktree) the content is restored from, with the affected line/file count. `→ Enter` confirms and emits `Operate(Discard(...))`; `Esc` or plain `Enter` on the default (safe) choice cancels with no command. While the box is open, keys never reach the pane underneath (e.g. `j` does not move anything). A real-runner test drives this to completion on an untracked file: `d` → `→` → `Enter` deletes the file, leaves exactly one `refs/graft/backup` ref, and a two-line journal (the discard, later its undo).
- **Proven by:**
  `mergewright::update::detail` confirm-box tests: opens with `selected == Safe`, `Enter` closes with no command, `→ Enter` emits `Discard`, `Esc` closes, keys under the box are swallowed (ticket 04 acceptance)
  `mergewright::tests::it::changes_pane_writes` — real-runner discard-then-check test
- **Looks like:** `docs/shots/m3-03-discard-confirm-80x24.txt`, `docs/shots/m3-confirm-delete-untracked.txt`, `docs/shots/m3-confirm-discard.txt`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §New UI "Confirm box"
- **Status:** works. Known wording bug: the untracked-file box asks "Delete notes.md?" over a button labelled `Discard` — question and button disagree.

### M3-09. The commit editor: `c` to commit, `a` to amend

```meta
id: M3-09
area: working-tree
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** `c` opens the commit screen (ease-pass/08) with the cursor in the subject; `a` opens it prefilled with `HEAD`'s message in amend mode; `Enter` in the subject commits; `Tab` cycles subject → body → file list; `space` in the file list puts a file in or leaves it out; `Ctrl+Enter` commits from any focus.
- **Behaviour:** The header names the branch and its push state; the file list shows what goes in and what stays out, the cursor file's changes beside it. The commit message sent is `subject + "\n\n" + body + "\n"`. `Ctrl+S` toggles sign-off, the header says so, and it reaches the commit params. With every file left out, `Enter` does nothing and the bar says why. On success (`Product::Operated(Ok(Everything))`), the screen closes and the draft clears; on failure it stays open with a sticky failure line and keeps the typed text. `Esc` keeps the draft — the next `c` reopens with it intact. A real-runner test proves the whole path: change a file, type a subject, commit — `git log -1 --format=%s` matches, the journal has a `Commit` entry whose `refs_before[main] != refs_after[main]`; then `a`, edit the subject, commit again — the subject changes but `git rev-list --count HEAD` stays the same (a true amend, not a second commit).
- **Proven by:**
  `mergewright::update::commit_editor::tests::c_opens_the_editor_and_typing_lands_in_the_subject`
  `mergewright::update::commit_editor::tests::enter_from_the_files_commits_the_subject_and_the_body`
  `mergewright::update::commit_editor::tests::a_asks_for_heads_message_and_opens_on_the_answer`
  `mergewright::update::commit_editor::tests::esc_keeps_a_draft_and_the_next_c_picks_it_back_up`
  `mergewright::tests::it::commit_editor_writes::enter_commits_what_was_typed_and_then_amends_it`
  `mergewright::tests::it::commit_editor_writes::nothing_staged_makes_no_commit`
- **Looks like:** `docs/shots/m3-04-commit-editor-80x24.txt`, `docs/shots/m3-05-commit-refused-80x24.txt`, `docs/shots/m3-commit-editor-80x24.txt`, `docs/shots/m3-commit-editor-refused.txt`, `docs/shots/m3-amend.txt`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §New UI "Commit editor"; ADR-0034 (Tab order is 3 stops, not the original spec's 4)
- **Status:** works

### M3-10. `Ctrl+E` (or `e`) opens the commit message in your real `$EDITOR`, cleanly

```meta
id: M3-10
area: working-tree
milestone: m3-working-tree-undo-and-stash
status: partial
unverified_aspect: a real $EDITOR leaving a real terminal usable afterward — no test in this repo can prove it
```

- **How:** From the commit editor.
- **Behaviour:** Resolves the editor command per program name: `vim` → `vim +12 path` (jump to the current line), `code` → `code --goto path:12`, `zed` → `zed path:12`, anything unrecognised → `<editor> path` (no line jump); an editor string carrying its own arguments (e.g. `"emacs -nw"`) keeps them. Running it suspends Graft's own terminal handling cleanly: leaves the alternate screen, restores original terminal modes, runs the editor, restores Graft's modes, re-enters the alternate screen — and does this even if the editor process (or the code driving it) panics, via a guard. The same suspend/resume path also serves plain `Ctrl+Z` (ADR-0002), controlled by `[keys] suspend_on_ctrl_z` in config (reaches `AppState` the same way `[radar]` settings do; with it off, `Ctrl+Z` is left unclaimed here for undo/redo, not swallowed).
- **Proven by:**
  `mergewright::event_loop::outside::tests::the_vi_lineage_takes_a_plus_line`, `vs_code_wants_goto_and_zed_wants_a_suffix`, `an_editor_nobody_knows_gets_the_file_and_nothing_else`, `an_editor_with_arguments_keeps_them_in_front`
  `mergewright::event_loop::outside::tests::the_terminal_is_given_up_before_f_runs_and_taken_back_after`
  `mergewright::event_loop::outside::tests::a_panic_inside_f_still_re_enters_the_terminal`
  `mergewright-term::terminal::tests::suspend_undoes_every_mode_and_resume_puts_the_same_ones_back`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §Genuinely risky groups (external editor); PROMPT.md principle 8 (never break the user's terminal)
- **Status:** partial/unverified for the real-terminal case. The suspend/resume mechanics are proven by transcript order and a panic guard in tests; no test in this repo can prove a real `$EDITOR` leaves a real terminal usable afterward (needs a human at a real terminal). `Ctrl+Enter`-vs-`Enter` distinguishing (outside the kitty keyboard protocol) is also unverified outside a real terminal — `Tab` to the button is the proven path.

### M3-11. Stash: save, list, apply, pop, drop — all undoable

```meta
id: M3-11
area: stash
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** `S` opens the stash overlay; `s` inside it opens a save prompt (type a message, `Tab` toggles staged-only vs. everything, `Enter` saves); `Enter`/`p` on a row applies/pops it; `d` opens a drop confirmation; `Space` previews a row's file list.
- **Behaviour:** `S` opens and reads the stash list. `Enter` and `p` carry the row's actual commit object, never just a numeric index (so a concurrent change to the stash can't apply the wrong entry). Save with a clean tree refuses (`Enter` does nothing, shown as "nothing to stash"); an empty typed message sends `None`, not an empty string. `d` opens a box (`[ Keep ] / [ Drop ]`) before anything runs. Every write here goes through the same journal/undo seam as everything else: `s` (push) then `u` restores the working tree and empties the stash list, `U` re-stashes; apply then `u` cleans the tree again (apply doesn't remove the stash entry — only pop does); pop then `u` restores the entry at `stash@{0}`, proven by its message, not its index (a restored stash always re-lists at index 0 — original position isn't preserved, git has no insert-at-index for the stash reflog; content and commit are unchanged). Drop then `u` restores the correct one of two entries, verified after `git gc --prune=now` runs between the drop and the undo (the entry stayed reachable the whole time via its backup ref). Every real stash entry gets a `stash@{n}` badge in the graph; `H` hides all stash-derived rows and shows them again.
- **Proven by:**
  `mergewright::tests::it::stash_by_key::s_saves_the_working_tree_and_u_takes_the_stash_back`
  `mergewright::tests::it::stash_by_key::enter_applies_a_stash_and_u_takes_the_application_back`
  `mergewright::tests::it::stash_by_key::p_pops_a_stash_and_u_puts_the_entry_back`
  `mergewright::tests::it::stash_by_key::d_drops_a_stash_behind_the_box_and_u_puts_it_back_at_the_top`
  `mergewright::tests::it::stash_by_key::every_stash_entry_is_badged_on_a_real_repository`
  `mergewright::refs::tests::every_stash_entry_is_badged_by_its_own_index`
  `mergewright::update::tests::capital_h_hides_the_stash_and_shows_it_again`
- **Looks like:** `docs/shots/m3-06-stash-list-80x24.txt`, `docs/shots/m3-07-stash-prompt-80x24.txt`, `docs/shots/m3-stash-list.txt`, `docs/shots/m3-stash-save.txt`, `docs/shots/m3-stash-empty.txt`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §New UI "Stash overlay"; ADR-0034
- **Status:** works. Known gaps: can't stash only selected files (`S` always stashes everything, or staged-only via the Tab toggle — not an arbitrary subset); can't create a branch from a stash; redo of an undone conflicted `stash pop` restores content correctly but loses git's own "mid-conflict" flag (index reads as ordinary modified, not unmerged) — conflict-aware redo is M6's job; a multi-line stash message survives undo as its first line only (git's stash reflog holds one line — content unaffected).

### M3-12. Refresh on window focus, without losing your place

```meta
id: M3-12
area: foundations
milestone: m3-working-tree-undo-and-stash
status: works
```

- **How:** Automatic — coming back to the terminal window (focus-gain).
- **Behaviour:** A focus-gained event re-reads the working tree only — no history restart, no row-list rebuild. A second focus report within 500ms of the first is ignored (debounced); one more than 500ms later reads again. A focus report that arrives before the repository has finished opening asks for nothing. `Ctrl+R` (manual refresh) also re-reads the journal (in case another Graft process on the same worktree appended to it) and the stash list, keeping `Refresh::Everything` identical in scope to what `Ctrl+R` itself does.
- **Proven by:**
  `mergewright::update::tests::coming_back_to_the_window_re_reads_the_working_tree_and_nothing_else`
  `mergewright::update::tests::a_second_focus_report_within_half_a_second_is_ignored`
  `mergewright::update::tests::a_focus_report_before_the_repository_opened_asks_for_nothing`
- **Decided in:** `docs/design-notes/m3-working-tree-undo-and-stash.md` §New UI "Status line"
- **Status:** works

### M3-13. Graft's own bookkeeping refs never appear as history

```meta
id: M3-13
area: undo
milestone: m3-working-tree-undo-and-stash
status: works
unverified_aspect: whether stash-internals-in-graph was the original M2/M3 design intent
```

- **How:** Automatic.
- **Behaviour:** Any ref under `refs/graft/**` (the journal's backup refs) is excluded before it ever reaches the graph, the ref badges, or the "contained in" computation — filtered once, at the read seam (`RefName::is_graft_internal`), not re-implemented at each call site. `refs/stash` is different: it's git's own namespace, the user already knows about it, and Graft draws stash entries on the graph on purpose.
- **Proven by:**
  `mergewright-git::repo::tests::grafts_own_refs_are_not_drawn_but_are_still_readable`
  `no_bookkeeping_in_the_graph` (run after every scenario in the M3 acceptance walk)
- **Decided in:** `docs/reviews/m3-07-acceptance.md` (review finding C1), fixed in ticket 08
- **Status:** works. Known gap one level down: git's *own* stash internals (`index on main: …`, `untracked files on main: …` synthetic commits reachable one hop below `refs/stash`) still draw in the graph — unverified whether that was the original M2/M3 design intent.

### M3-14. The pinned uncommitted row reaches the working tree from history

```meta
id: M3-14
area: working-tree
milestone: m3-working-tree-undo-and-stash
status: works
unverified_aspect: HEAD scrolled below the top in a live key sequence
```

- **How:** `k` from history row `top` with a dirty working tree.
- **Behaviour:** Continues M2's pinned uncommitted row (see the M2 ledger, M2-02): with `HEAD` scrolled off the top of the visible window and the tree dirty, row 0 becomes the pinned `◌` row and `k` from row 1 reaches it — extending the M2 behaviour, which only worked while `HEAD` itself was the top row.
- **Proven by:** `mergewright::update::selection` tests extending the M2 06b suite (ticket 04 acceptance, "Pinned row" line)
- **Decided in:** M2 design note (owner handed to M3); `docs/design-notes/m3-working-tree-undo-and-stash.md`
- **Status:** works in the scenarios exercised (17 of 22 acceptance-walk scenarios reach it, all with `HEAD` topmost). Unverified with `HEAD` scrolled below the top in a live key sequence — the acceptance walk didn't happen to construct that case.

### M3-15. `TESTING.md` "you cannot lose work" acceptance walk

```meta
id: M3-15
area: working-tree
milestone: m3-working-tree-undo-and-stash
status: works
unverified_aspect: everything judged only by eye: the captures themselves, the 120x40 breakpoint, terminal-specific behaviour, colour
```

- **How:** `cargo run -p mergewright --example m3_walk` (prints `ok <name>` per scenario) or `cargo test` (one test per scenario).
- **Behaviour:** 22 scripted scenarios, each driving real keys through the real job pool against a real git repository and checking the result by reading git directly afterward (`for-each-ref`, `ls-files -s`, `stash list`, `status --porcelain`, `rev-parse`, `rev-list --objects`, `hash-object --stdin`, actual file bytes, `fsck --no-dangling`, backup-ref counts) — never by trusting `AppState`. Scenarios: stage file/hunk/line/all, discard (tracked-unstaged, untracked-binary, staged-and-unstaged), commit, commit-refused-nothing-staged, amend, commit-on-unborn-head, undo-drift-guard, stash save/pop/apply/drop, stash-apply-refused-dirty-touched-file, interrupted-entry, linked-worktree, refresh-on-focus, no-index-write-on-read. All 22 pass (22 ok, 0 failed, 12.1s wall for the example; 4.6s under `cargo nextest` for the tests). 18 terminal captures at two sizes (80×24, 160×48) plus a contact sheet concatenating them in order.
- **Proven by:** `mergewright::tests::it::m3_walk::every_scenario::*` (one test per scenario name above, plus `every_scenario_has_a_test` checking the walk's own list matches the test list)
- **Decided in:** PROMPT.md §3 ("you cannot lose work"); CLAUDE.md's milestone-acceptance rule
- **Status:** works. Explicitly **not** verified by this walk: anything judged only by eye (the captures themselves — user's call), the 120×40 breakpoint, Ghostty/Zed/VS Code terminal-specific behaviour, colour (captures carry none), `Ctrl+E`/external editor in a real terminal, commit signing, git hooks, a real terminal focus event, and the snapshot-cost perf number (recorded, not gated).

## Deliberately not built

- **Hunks inside an untracked file cannot be staged.** Needs `add -N` first; `Space` on an untracked row always stages the whole file. Scope cut, agreed with the user (ADR-0034). Owner: M7.
- **No `i` to add a path to `.gitignore`** from the changes pane. Scope cut (ADR-0034). Owner: M7.
- **No commit message history.** The editor always starts empty (except when amending or resuming a kept draft) — no recall of recent messages. Design-note deferral. Owner: M7.
- **Cannot stash only selected files.** `S`'s save prompt only offers everything or staged-only, not an arbitrary file subset. Design-note deferral. Owner: M7.
- **Cannot create a branch from a stash.** Design-note deferral. Owner: M5.
- **No file watcher.** The working tree refreshes only on focus-gain and on demand (`Ctrl+R`) — an external edit made while the window has focus is invisible until you come back to it or refresh manually (`notify` crate not wired). Design-note deferral. Owner: M7.
- **No action to clear undo backups.** The journal's `refs/graft/backup/**` refs accumulate forever with nothing in the UI to prune them. Design-note deferral. Owner: M8.
- **Commit/amend are not specially refused mid-merge/mid-rebase/mid-cherry-pick beyond what git itself does.** Full in-progress-state awareness (surfacing `--continue`/`--abort` guidance in the UI) belongs to conflict resolution, a later milestone. Owner: M5/M6.
- **Rebase with a dirty working tree.** Whether Graft offers `--autostash` or just surfaces git's refusal is undecided — deferred to M5's design pass. Owner: M5.
- **Redo of an undone conflicted `stash pop`** restores file content correctly but not git's own "mid-conflict" index flag — `git status` reads it as an ordinary modified file, not unmerged. Owner: M6 (conflict resolver owns unmerged-index state).
- **A restored stash re-lists at `stash@{0}`, not its original position**, and **a multi-line stash message survives undo as only its first line** — both are limits of git's own stash reflog (no insert-at-index; single-line reflog subject), not something Graft chose. Not planned to fix (first is a real gap, revisit only if real use finds it confusing, M7; second is not planned).
- **A forced undo isn't pinned to the entry the drift box named** — a race window exists between the box drawing and the force keypress during which the journal tip could move. Needs a signature change (`undo(ctx, force, aim)`). Owner: M4.
- **The journal browser draws no status line of its own** while it's open — a write or notice triggered from inside it has nowhere to show. Owner: M5.
- **`Esc` doesn't clear a `Status::Failed` line** in the changes pane; it sits until the next status replaces it. Owner: unassigned, small fix.
- **Two acceptance-walk tests run over PROMPT §7.1's 10-second fast-tier line** (13.2s and 12.1s) — both are real, deliberate coverage (every commit of every fixture shape; a 100,000-row scroll test); moving them to the heavy tier is a scope trade-off the user hasn't made yet. Owner: M4, user's call.
- **Force-push verdicts, evil-merge probing and duplicate-detection are M2 capabilities, not M3** — listed there, not repeated here.
- **`m3_walk.rs` is ~2,042 lines against PROMPT §6.2's ~400-line guidance.** A pure split was deliberately not done in the same pass as the C1/M1 review fixes (a mechanical move alongside a behaviour change hides the change). Owner: M5, its own ticket.
- **Capture headers embed the machine's own scratch temp-directory path** (e.g. `/var/folders/…`), which is machine-specific noise in a repo that may be published. The fix conflicts with TESTING.md's "a capture is exactly what pyte saw" contract, so it needs a decision, not a patch. Owner: M4.
- **UNVERIFIED, standing: no capture in this repo proves any colour anywhere in M3.** The pty capture pipeline records character grids only, never colour attributes (CLAUDE.md rule 5) — every colour decision in the changes pane, commit editor, stash overlay and journal browser ships judged by the user's own eye only, never by a test.
- **UNVERIFIED: two Graft processes open on the same worktree at once.** The journal is a plain file with no cross-process arbitration; untested, undesigned-against. Owner: M4 or the first field note that hits it.
- **UNVERIFIED: every M3 git-behaviour claim is proven on git 2.54 only**, while the documented minimum is 2.38 (`restore --source=HEAD` on a staged rename, `stash store -m`, `clean -f` with a pathspec, and `update-ref` CAS are the pieces most likely to have moved between those versions). Owner: an M3 review re-run against 2.38, or raising the documented minimum.
- **BROKEN: `strip_comments` hardcodes `#` as the comment character**, ignoring a user's own `core.commentChar` git config — such a user gets real commit-message lines stripped, or comment lines kept, incorrectly. Owner: M4, sooner if a field note reports it.
- **GPG/SSH commit signing while the editor holds the terminal** (a pinentry program wanting the tty mid-commit) is untried. Owner: M4 or the first field note.
