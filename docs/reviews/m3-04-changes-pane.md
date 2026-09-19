# Review: M3 group 04 — changes pane (`7b364ba..a0a58e7`)

Reviewer: Fable (`reviewer`). Read: ticket 04 + NOTES, `as-built.md`, design note
"Changes pane" / "Confirm box" / "Status line" / Proposed changes 6 & 8, both MAP.md
files, then the diff crate by crate. Source opened only to confirm findings
(`unified.rs`, `update/mod.rs`, `keys.rs`, ops `stage.rs`, `frame.rs`). One git
behaviour checked in a throwaway repo (`/tmp/graft-rv`).

Not re-filed (already in KNOWN_GAPS): `Esc` does not clear `Status::Failed`; hunk
unstage / hunk discard have no key-to-repo test; confirm-box command string is
hand-written. ADR-0034's cuts and the newline-less single-line refusal are settled.

## Gates

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo test` (fast) | 1416 passed, 0 failed, 11 ignored; **30.3 s wall**, peak RSS 343 MB |
| `cargo test --features heavy-tests -- --test-threads=2` | 1449 passed, 0 failed; 187 s, peak RSS 621 MB |
| release perf (`frames:: heavy::budgets_hold`) | 13 passed, 73 s — numbers below |
| seam rule (`tests/it/seam_rule.rs`), `GitInvocation` guard, purity guard | green (in the fast suite) |
| fuzzing | no fuzz targets in the repo |
| benches | `graft-ui` `GraphColumn::render/one screen` 14.7 µs (unchanged crate area; `graft-graph` not run, untouched) |
| `target/` | 20 GB |

Release perf: Unicode 100k scroll p50 4.0 ms / p99 5.4 ms / max 5.6 ms; Pixel p99 5.6 ms /
max 7.7 ms; 50k refs + pane open p99 5.1 ms; first paint 13.1–15.3 ms (38.3 ms with
50k refs); peak RSS 29–101 MB across the five budget fixtures. All under budget.

Five slowest fast tests: `diff_fixture::parse_unified_matches_numstat_for_every_shape`
6.2 s, `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`
5.9 s, `corpus::two_builds_produce_the_same_object_ids` 2.8 s,
`diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 1.7 s,
`corpus::a_second_build_in_the_same_place_is_a_no_op` 1.1 s. None from this group.

## What holds

- **Direction by section** is right and pinned by tests: `stage_selected` /
  `stage_at_diff_cursor` read `FileListView::section_at`, never the status flags; the
  "both" file has two rows and the test `section_not_status_decides_direction_for_a_file_in_both`
  proves each row goes its own way. A section heading takes its files and stops at the
  next heading (`section_entries`).
- **Cursor → unit** mapping (`diff_unit`) is right for heading / `+`/`-` / context rows,
  and the raw-line index (`line - 1`) matches `assemble_hunk`'s one-heading-then-1:1
  copy (`jobs/detail.rs:359-382`). Patches are cut from `diff_raw` (git's bytes), cached
  together with the view under the same `DiffKey` — they cannot drift.
- **Seam rule** holds: `app` builds `Operation`s and `Command::Operate` only;
  `jobs/operate.rs` is the one place that opens the journal and calls `apply`.
  `ApplyError::AfterState` is kept as `Operated::Incomplete` with its `Applied`, and
  `operated` refreshes on it.
- The confirm box is drawn (`view::frame::draw_confirm`, last, over the menu), opens on
  `Cancel`, and a non-ignored test asserts question, exact command and `[ Cancel ]` on
  screen. The real-runner tests prove hunk stage and untracked discard against git,
  with one backup ref and a two-line journal.
- Rename handling is ops' job and ops does it (`stage.rs::pathspec`, `Classes::split`
  extend `original_path`), so `app` passing only the new path is fine.
- No `unwrap`/`expect`/`let _ =` outside tests in the group's diff. Captures exist,
  3.0–8.9 KB, and show sections, the `▎` cursor and the footer hints at all three sizes.

## Critical

### C1. Scrolling up hides the line cursor under the pinned hunk heading; `Space`/`d` then act on a row the user cannot see

- `crates/graft-ui/src/detail_pane/unified.rs:112-127` — when `pin_heading` returns
  `Some`, `content_top = top + 1` and the row at index `top` is **never drawn**; the
  pinned heading takes its slot.
- `crates/graft/src/update/detail/ops.rs` `move_body_cursor` — moving up sets
  `body_top = body_cursor`. So from the first `k` that scrolls, until the cursor reaches
  a `@@` row, `body_cursor == body_top` and the cursor row is the one row not on screen.
  (Downward is fine: `body_top = cursor - rows + 1` puts the cursor on the last drawn row.)
- Reproduction by reading: body of 40 rows, region 15 rows. `j`×20 → cursor 20, top 6,
  heading of hunk 1 pinned. `k` until cursor 6: idx 6 < `content_top` 7 → not drawn.
  Every further `k` scrolls and stays hidden.
- Why Critical: this is the everyday path ("move up a bit, stage that line"), and the
  group's own rule is "never acting unseen". `d` mitigates with a box that counts lines
  but does not show which line; `Space` stages silently (and see M3: the status line
  shows nothing). Not exercised by any test or capture — every cursor test sits at
  `body_top == 0`.
- Fix (either): in `move_body_cursor`, when the cursor moves above `body_top` and
  `DiffView::body_row(cursor)` is not a `Heading`, set `body_top = cursor - 1` (the
  pinned heading then sits above a visible cursor row); or have `diff_body` skip the pin
  when `focus.cursor == Some(top)`. Add a test that scrolls up past a hunk boundary and
  asserts the `▎` row is in the drawn buffer.

## Major

### M1. `Operated::Failed` does not refresh — the acceptance tick says it does, and the test asserts the opposite

- `crates/graft/src/update/operate.rs:474-477` — `Failed(error) => { status = Failed; Vec::new() }`.
  Test `operated_failed_reports_the_error_and_asks_for_no_refresh` pins the *absence*
  of a refresh. Ticket acceptance line 2 is ticked: "`Err` sets `Status::Failed` and
  still refreshes". MAP.md documents "`Failed` only sets the status line" as if intended.
- Why it matters: `ApplyError::Git` is "git failed **part-way**; the entry is journaled
  `Aborted`" (`graft-ops/src/apply.rs:81-91`) — a whole-file discard plan is up to three
  invocations (`restore --staged --worktree`, `restore --worktree`, `clean -f`), so the
  first can have run when the second fails. `ApplyError::Journal { after: Some(_) }` is
  "ran, outcome line missing". After either, the file list, `diff_raw`, the header counts
  and `working_tree` are stale; the journal has a new Aborted entry with a backup. The
  next `Space`/`d` cuts a patch from stale bytes and the confirm box previews a command
  computed from a stale `StatusEntry`.
- Fix: `Failed` → set the status **and** `refresh(state, Refresh::WorkingTree, ..)`.
  Flip the test. Un-tick or reword the acceptance line until then.

### M2. `Ctrl+A` stages every status entry, and `git add` on a staged deletion is fatal

- `crates/graft/src/update/operate.rs` `stage_all` — collects `status.entries` wholesale.
  Spec: `Stage(Paths(all unstaged + untracked))`; acceptance ticked "Ctrl+A all
  unstaged+untracked" with a one-file test.
- Verified in a throwaway repo (git 2.x): `D  a.txt` + `?? b.txt`, then
  `git --literal-pathspecs add -- a.txt b.txt` → `fatal: pathspec 'a.txt' did not match
  any files`, exit 128, nothing staged. So `Ctrl+A` fails outright whenever the index
  holds a deletion or a rename (ops' `pathspec` adds the rename's `original_path`, which
  is gone from disk and index too). Also sweeps in `unmerged` entries — `git add` on a
  conflicted path marks it resolved with markers inside.
- Fix: filter `entry.unstaged.is_some() || entry.untracked`, skip `unmerged`; test with a
  staged-deletion entry present.

### M3. `Status::Busy` and `Status::Done` are never drawn, and `Done` is never cleared

- `crates/graft/src/view/rows.rs:32`, `view/status.rs:29` map both to `problem: None`;
  `view/frame.rs::footer_notice` (l. 368-380) handles only `Copied`; `update/keys.rs:21`
  clears only `Copied` on the next key. The `Done` doc comment ("Cleared by the next key,
  like `Copied`") is false.
- Spec piece 1/4 and design note "Status line": the line says `<description>…` while a
  write runs and `Status::Done("stage src/a.rs")` after. Today a `Space` gives no visible
  acknowledgement at all beyond the list redrawing; a second `Space` during the write is
  a silent no-op (the acceptance tick "…emits nothing and `Status::Busy`" is not tested
  for `Busy`, and `Busy` would not show if it were).
- Fix: `footer_notice` returns the `Busy`/`Done` text (sanitised); `keys.rs:21` clears
  `Done` too; a graph_screen test that the footer carries the description.

### M4. `AppState::operating` is never cleared when the job fails or is cancelled — the write keys go dead for the session

- `crates/graft/src/update/mod.rs:314-357` — `Event::JobFailed` / `JobCancelled` clear
  `awaiting`/`promoting` only. `operating = None` happens in exactly one place
  (`operate.rs:467`, on `Product::Operated`).
- A panic inside `run_operation`/`graft_ops::apply` (reported by the pool as
  `JobFailed`, `pool.rs:179`) or a stopped runner (`runner.rs:317`) leaves
  `operating = Some(id)` forever: every `Space`/`s`/`d`/`Ctrl+A` returns `false` and
  falls through, nothing says why (M3 makes it worse: no `Busy` line). The only way out
  is quitting — a state the user can reach and not escape.
- Fix: in both arms, `if next.operating == Some(id) { next.operating = None; }` and
  refresh the working tree (a panic mid-plan may have written).

### M5. Side-by-side (`v`) draws no cursor but `Space`/`d` still act on it

- `crates/graft-ui/src/detail_pane/side_by_side.rs:269-275` sets `cursor: false` with
  the comment "the changes pane's `Space`/`d` say so rather than acting unseen". Nothing
  says so: `update/operate.rs` never reads `detail.mode`; `DetailState::body_cursor()`
  ignores it. In `v` mode `Space` stages the hunk/line under an invisible cursor.
- Fix: in `stage_at_diff_cursor`/`request_discard_at_diff_cursor`, when
  `mode == SideBySide` set `Status::Failed("…press v for the unified view to stage a
  hunk")` and return `true`; or draw the cursor on the paired row. Test it.

### M6. Whole-file discard: the consequence line is boilerplate and wrong for two of the three classes

- `crates/graft/src/update/operate.rs` `request_discard` — one fixed message for every
  file: "This throws away the change in the working tree (and the index, if staged)."
  - **Untracked:** the command is `git clean -f`; the file is **deleted**. Spec text:
    "deletes the file; a backup is saved first, u undoes it". The box never says "delete".
  - **Staged + unstaged file from the UNSTAGED row:** the row shows the unstaged diff
    only; ops (`Classes::split`) classes the path as staged and runs
    `restore --staged --worktree --source=HEAD`, throwing away the staged change too.
    The box hedges "if staged" instead of saying "both changes go".
  - No line counts anywhere (spec: "N added / M removed lines"); the patch box has them.
- PROMPT §3 requires the consequence named. The command line is there, but the plain-words
  line is what the user reads.
- Fix: branch on the `StatusEntry`: untracked → "deletes `new.txt` (N lines); a backup is
  saved first"; staged+unstaged → "both the staged and the unstaged change to `p` go
  (+A −R)"; counts from `list.files[entry]`. Whether the UNSTAGED row's `d` should mean
  worktree-only (`Target::Paths` cannot say that today) is a spec question for
  `planner-fable`, not for this fix.

### M7. Every write resets both cursors — and can flip the next key's direction

- `DetailState::refresh` → `load` (`update/detail/ops.rs:146-149`) sets `files = Pending`,
  `body_cursor = 0`; `files_arrived` installs a fresh view whose cursor
  `assemble_working_tree` puts on the **first file**. So after each `Space` the user is
  back on the first file's first row. Staging three hunks of one file means re-navigating
  three times; the milestone promise is "stage files/hunks" daily.
- Worse: the first file is usually under **STAGED**. Stage the last unstaged hunk of
  file B, cursor jumps to file A (STAGED); the next `Space` — meant to keep staging —
  **unstages A**. Section-correct, but not what the user was doing, and (M3) nothing on
  screen said the cursor moved.
- Fix: carry the cursor across a reload by `(section, path)` and the body cursor by
  `(hunk, line)` clamped; fall back to the first file only when the path is gone.

### M8. `Refresh::Everything` does not emit the `Ctrl+R` commands the acceptance line ticks, and no test covers `Operated::Done`

- `operate.rs::refresh` pushes `ReadWorkingTree`, `detail.refresh`, `ReadHistory`,
  `ReadRefs`. `keys.rs::refresh_key` (l. 101-121) also cancels `awaiting` and
  `promoting`, sends `OpenRepository`, and `radar::reset_for_refresh`. Without the
  cancels a second history walk starts while the first still runs; without the radar
  reset the default-branch context is stale after a commit moves the tip.
- No unit test exercises `Operated::Done` at all (tests: stale drop, `Failed`). The
  ticked line "`Ok(WorkingTree)` emits `ReadWorkingTree` and no history restart;
  `Ok(Everything)` emits the Ctrl+R commands" is unproven and, for `Everything`, false.
  `Everything` is unused until commit/amend (group 05), which is why it did not bite.
- Fix: make `refresh_key` `pub(super)` and call it for `Everything`; add the two tests.

### M9. Acceptance ticks that are not honest (summary for the conductor)

| Ticked line | State |
|---|---|
| "`Err` sets `Status::Failed` and still refreshes" | not built; test asserts the opposite (M1) |
| "`Ctrl+A` all unstaged+untracked" | stages everything; fails on a staged deletion (M2) |
| "a second `Space` while pending emits nothing and `Status::Busy`" | `Busy` untested and undrawn (M3) |
| "`Ok(WorkingTree)` emits `ReadWorkingTree`…; `Ok(Everything)` emits the Ctrl+R commands" | no test; `Everything` differs from Ctrl+R (M8) |
| "empty status → one row `nothing to commit — working tree clean`" | built as an empty list + body message (`view/detail.rs:215-223`); acceptable, wording loose |

The other ticks (sections, hunk/line/context mapping, confirm keys, real-runner test,
pinned row, guards, captures) are backed by the tests and files named.

## Minor

1. **`ConfirmView` fields are `String`, spec said `SanitizedText`** (`graft-ui/src/confirm.rs:30-40`).
   Title and command embed `path.display()` unsanitised; only `fit()` dropping
   undefined-width chars keeps a hostile filename off the terminal. Use `SanitizedText`
   at the ui boundary like every other widget.
2. **Two sources of truth for the pane.** Rows come from `list_files(Staged)` +
   `list_files(Unstaged)` (`jobs/detail.rs::read_files`), while the header counts,
   `Ctrl+A` and `request_discard`'s `status_entry` read `status::read`. Read at
   different times they can disagree; then `request_discard` returns `false` — a `d`
   that silently does nothing, against the group's own rule. Set
   `Status::Failed("status is out of date — Ctrl+R")` instead of `false`. (Spec had
   `assemble_working_tree(&WorkingTreeStatus)`; the deviation is not written up in NOTES.)
3. **Any unbound key closes the confirm box** (`update/confirm.rs:52`, `_ => confirm = None`),
   `j` included. Safe direction, but a stray key dismisses a box the user was reading.
   Spec: `Esc` closes. Consider ignoring unbound keys.
4. **Box geometry and labels differ from the spec** — width `min(64)` not `min(72, w−4)`,
   `[ Cancel ]   Discard` not `[ Keep ]  [ Discard ]`; the unselected button has no
   brackets, so a capture (no colour) cannot show it is a button. The
   `m3-confirm-discard.txt` "judge by eye" note says "`Keep` … is the one under the
   cursor" — no button is named Keep. Align labels and the note.
5. **`run_operation` clock fallback** (`jobs/operate.rs:64-69`): `map_or(0, …)` on a
   pre-epoch clock silently mints a 1970 entry id. Either a comment saying why that is
   safe, or `Operated::Failed`. `system_git()` also spawns `git --version` per write.
6. **`Progress::Operating` arm** (`update/mod.rs:182`, `_ => {}`) swallows a stale
   progress — fine, but say so in a comment rather than an empty catch-all.
7. **`Status::Failed` from `Operated::Incomplete` on `Everything`** is wiped by the
   history walk's `JobFinished → Status::Ready` (`mod.rs:341`). Bites in group 05.
8. **MAP.md** `update::operate` describes the `Failed`-no-refresh behaviour as design;
   keep it in step with whichever way M1 goes. `ConfirmView`'s MAP line lists
   `new(title, message, command)` — matches code, not the spec's field names; fine.
9. **`footer` hint `space stage` shows on the changes pane even when the cursor is on a
   `Dir` row or the truncation footer**, where `Space` folds or does nothing. Cosmetic.

## For KNOWN_GAPS

- **Broken:** `Operated::Failed` leaves the screen stale (M1). `Ctrl+A` fails whenever
  the index holds a deletion or rename (M2). `Busy`/`Done` never drawn (M3).
  `operating` never cleared after a job failure (M4). Side-by-side `Space` acts unseen
  (M5). Cursor row hidden under the pinned heading when scrolling up (C1).
- **Unverified:** how the pane and box look (captures carry no colour). `Operated::Done`
  → refresh commands, both scopes (no test, M8). Whole-file discard wording per class
  (M6) — behaviour is right, words are not.

## Architecture / readability

- Crate rules intact: `graft` → `graft-ops`/`graft-git`/`graft-ui`; `graft-ui` gained
  `confirm` with no new deps; `getrandom` added to `graft` only (already in the lock).
  `update/` stays pure (guard green). `ARCHITECTURE.md` untouched and still accurate —
  `jobs/operate.rs` sits where the seam rule says.
- `update/operate.rs` is 1137 lines, ~480 of logic; readable, well-named, each function
  under 60 lines. `changes_pane_write_key` is a clear dispatcher. Doc comments explain
  *why* (the section-not-status rule, the marker beside the sign). No clever code.
- One naming wobble: `Operated` (jobs) vs `operated()` (update) vs `Operation` (ops) —
  three near-homonyms across the seam; acceptable, but `OperateOutcome` would read
  better.
