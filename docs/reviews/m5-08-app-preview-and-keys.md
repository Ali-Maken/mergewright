# Review: m5-history-and-ghost-preview / 08 app-preview-and-keys

Reviewed at `be719c6`, as the union of exactly four merge diffs — `59209be`
(pieces 1–5, the preview path), `8525555` (pieces 7–10, the time machine and
`Esc` stops an undo), `3f0d1a2` (piece 6, the `.` menu verbs) and `be719c6`
(piece 6 labels, the trailing key-hint rule). Other groups' commits sit
between them and were not judged. Read-only; every check and every reviewer
mutation ran in a detached worktree at `/tmp/graft-review-m508` with its own
target dir, deleted afterwards. Crates: `graft`, `graft-ui`.

**Counts: Critical 0 · Major 6 · Minor 11.** Gates green, heavy tier green,
release perf green. Ticks spot-checked: 14 of 33, 13 held, 1 held only by
inspection (no test through `key_event`). Reviewer mutations: 3 run — 2
caught, 1 (the preview's row alignment/highlight) caught by nothing, which
is Major 1.

**The one thing to know:** the preview draws its slice from the slice's
**top**, not from the viewport. The slice is `[top − graph_rows, top +
2·graph_rows)` and `graph_screen` draws `rows[..height]` from index 0, so the
moment the user is scrolled one screen or more into history, confirming `r`
shows them the screen *above* the one they were looking at — the selected
commit and (usually) the ghosts are off-screen. Every test and both captures
use a repository shorter than one screen, so `top == 0` and it never shows.
Nothing is lost (Confirm still runs the pinned op), but M5's promise, "you
see the result before you confirm", holds only on the first screen.

## Gates (workspace-wide, at `be719c6`)

| Check | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo test` (fast tier) | **2109 passed, 0 failed, 34 ignored** — 98.9 s wall incl. build; peak RSS 730 MB |
| `cargo test --doc` | pass (0 doctests; a gate, not evidence) |
| `cargo test --features heavy-tests -- --test-threads=2` | 2140 passed, 0 failed, 34 ignored — 230 s wall; peak RSS 731 MB |
| `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1` | 13 passed in 73.0 s (127 s wall incl. build); peak RSS 558 MB |

Release perf numbers (ADR-0021): `budgets_hold` first paint 14.0 ms (100k,
commit-graph), 38.1 ms (100k + 50k refs, commit-graph), 15.8 ms (100k, no
commit-graph), 14.0 ms / 16.5 ms (realistic, with / without). `frames::`
input-to-frame over 420 keys: p50 3.6–4.3 ms, p99 4.5–5.6 ms, max 8.95 ms
(Pixel, 50k refs) — all under 16 ms. No fuzz targets or benches exist for
this group.

Slowest five (fast tier, `cargo nextest run --workspace`: 2107 passed, 34
skipped, 57.8 s): `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape` 12.0 s;
`graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean` 8.8 s;
`graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 8.3 s;
`graft-fixtures::it corpus::two_builds_produce_the_same_object_ids` 5.6 s;
`graft-git diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 3.0 s.
None of the five is this group's; the slowest new test is
`journal_enter_on_an_older_row_previews_and_confirm_really_reverts` at under
1 s.

`target/`: 13 GB (`main`), 5.3 GB (review worktree, deleted after).

## What was checked, and how

**Acceptance ticks (14 of 33 checked, 13 held).** For each, I read the named
test at `be719c6` and asked whether it goes red when the behaviour breaks —
not whether it exists and passes.

- `confirm_runs_the_very_operation_the_picture_was_drawn_from` — walks
  `→`/`Enter` through `preview_key`, pulls the `Operation` out of
  `Command::Operate` and `assert_eq!`s it against the fixture's value. Held.
- `the_box_shows_the_plans_own_command_lines` — compares
  `PreviewBox.commands` element-for-element with `plan.commands`; a
  hand-written line would differ. Held. The `grep -n '"git '` inspection also
  finds nothing in `preview.rs`/`slice.rs`.
- `an_open_preview_claims_the_keys_behind_it` — calls `preview_key`
  directly for `u`/`j`/`m`/`/` and asserts `true`, no command, preview still
  open. It proves the function is modal; it does **not** prove `key_event`
  reaches `preview_key` before the global `u` handler. The ordering is
  right in `keys.rs` (`boxes_key` at the top of `key_event`, `is_undo_key`
  further down), so the tick holds by inspection only. Minor 1.
- `slice_refuses_a_missing_ghost_parent`,
  `a_preview_that_cannot_be_drawn_here_says_so_instead_of_drawing_it` — the
  second asserts `undrawable == Some(oid(200))`, `rows.is_empty()`, the exact
  line prefix and `["Close"]`. Held (the conductor's binding condition).
- `slice_clips_parents_outside_and_counts_them` (asserts `clipped == 3`
  and no row dropped), `slice_appends_anchors_last`,
  `lanes_that_leave_the_view_are_counted_on_the_box`. Held.
- `a_preview_refuses_while_a_write_is_running_and_says_so_as_a_problem`,
  `a_preview_refuses_while_git_has_an_operation_in_progress` — both assert
  `Status::Sticky` with the exact sentence and no command. Held.
- `the_preview_box_and_its_ghost_rows_are_actually_drawn` — reads the drawn
  100×30 buffer for the title, a `git rebase` line and ≥ 2 rows containing
  `—`. Held, though `text.contains("[ Cancel ]") || text.contains("Cancel")`
  is looser than it needs to be.
- `a_click_lands_on_the_button_the_widget_drew` — renders the real
  `PreviewBoxWidget`, finds `Cancel`/`Confirm` on the buffer row
  `choice_at` uses, clicks each and asserts `Operate` / close. Held; this is
  what keeps `BoxRect` honest.
- `a_refused_history_operation_says_which_key_answers_it` — launches via
  `launch_unstoppable(.., Some("r"))`, feeds `Operated::Failed { refused }`
  and asserts the whole `Status` for both `Moved` and `DirtyTree`. Held.
- `enter_on_an_older_row_opens_a_preview_of_reverting_to_it` /
  `the_browser_comes_back_on_the_same_row_when_the_preview_is_cancelled` —
  exact subject, `return_to_journal`, single `PlanPreview`, `operating ==
  None`; and `shown`/`selected` after `close_preview`. Held (reviewer
  mutation C below).
- `journal_enter_on_an_older_row_previews_and_confirm_really_reverts` — real
  repo, three real stages, `J j j Enter → Enter`, then `git diff --cached`
  empty, files still on disk, six entries with the last three `Undo`, and
  journal/index unchanged between `Enter` and `Confirm`. Held; the one write
  path this group exercises end to end.
- `diverged_pull_sets_m_r_source` — goes through `press` (real
  `key_event`), asserts the exact `Sticky`, the recorded `(tracking, tip)`,
  then the whole `Rebase` value `r` builds. Held.
- `history_ops_launch_unstoppable` — `Esc` through `press`, asserts no
  command, `cancelling == None`, and the exact busy line ending in
  `HISTORY_CANNOT_STOP`. Held.
- `esc_cancels_an_undo_in_flight` — asserts `[CancelJob(id)]`, `cancelling
  == Some(id)`, the `· cancelling` line, and that a second `Esc` sends
  nothing. Held on the app side; see "Esc stops an undo" below.
- `menu_verbs_build_expected_operations`,
  `a_foxtrot_merge_head_is_on_offers_all_three_fixes`,
  `no_fix_item_when_the_previous_tip_is_not_a_parent` — exact label list
  then one `assert_eq!` on the whole `Operation` per label; the foxtrot test
  asserts `-m 2` and `real_tip` in all three. Held.
- `the_context_menu_lists_its_history_verbs_and_draws_them`,
  `choosing_a_menu_verb_previews_that_operation_and_writes_nothing` — real
  git, labels off the drawn buffer, and `for-each-ref` byte-identical before
  and after planning. Held.

**Reviewer mutations (3 run, each applied, tested, restored from git).**

| # | Mutation | Caught by |
|---|---|---|
| A | `view/preview.rs` never highlights a row (`index == preview.cursor` → `false`) and `view/rows.rs` reveals nothing (`Some(preview.cursor)` → `None`) | **nothing** — 44 lib + 6 integration `preview` tests green |
| B | `slice.rs` anchors a ghost at the **lowest** transitive parent (`min` → `max`) | `a_ghost_merge_sits_above_the_higher_of_its_parents` |
| C | `journal.rs` leaves the browser shown after `Enter` opens the preview | `the_browser_comes_back_on_the_same_row_when_the_preview_is_cancelled` (+3 that share the sed) |

**`Esc` stops an undo, and what it costs.** The app side is right: `u`/`U`
now launch `Stoppable::Yes`, `Esc` sends one `CancelJob` and latches
`cancelling`, and `jobs::undo::undone_from` maps `RestoreError::Cancelled`
→ `Undone::Cancelled { clean: true }` (`Status::Done`) and
`CancelledPartway` → `clean: false` (`Status::Sticky`), both with
`Refresh::Everything`. The rollback itself is group 06's
`restore::rollback::roll_back_after_stop`, which I read: it reverses the
applied steps newest-first under `uncancellable`, each reversal a
compare-and-swap, and reports `Partway` honestly when a step's settled
state is unknown or a reversal refuses. So the claim is real, not
best-effort-with-a-log. What it costs the user: a part-way cancel leaves
the older steps in force by design (the comment in `rollback.rs` says why)
and the line says so. **The time machine is different and honest about it:**
`RevertTo` launches `Stoppable::No(HISTORY_CANNOT_STOP)`, and
`undo::revert_to` checks the stop only between steps, keeping what ran
(each step is its own journaled `Undo`). Unverified: no test drives a real
cancelled undo through the app — `graft-ops::it::undo_cancel::*` does it
below the app, and the builder says so (KNOWN_GAPS row below).

**Rule 3 (never swallow an error).** Swept every non-test source file the
four merges touched for `unwrap()`, `expect(`, `unreachable!`, `panic!`,
`let _`, `unwrap_or_default()`, `Err(_) =>`. Findings: `read_anchors`'s
`if let Ok(commit) = read_commit(..)` (Major 4 — the comment's safety
argument covers only half the cases), `preview_key`'s
`unreachable!("checked just above")` (Minor 3), and the `let _opened: bool
= open_preview(..)` in `context_menu.rs`, which carries a comment saying why
the bool is not needed and is fine. `history/seeds.rs:91`'s
`unwrap_or_default()` is an `Option<&[_]>` → empty slice when there are no
walk facts, not a failure stand-in.

**`plan_revert_to` over a mixed span.** Read `graft_ops::undo::plan_revert_to`
and `journal::{is_operation, next_redo}`. It walks newest → `target`, skipping
anything not in force, not an operation (`Undo`/`Redo`), or not undoable;
`in_force_all` is what decides whether a redone operation counts. For
`[A, B, undo(B), C]` → `[C, A]` — right, and pinned by
`the_time_machine_does_not_redo_what_was_undone`. Two gaps: (1) no test
puts a **`Redo`** in the span (`[A, B, undo(B), redo(B), C]` should yield
`[C, B, A]`; it rests on `in_force_flags`, which is M3's and tested there,
but nothing checks the composition) — unverified; (2) the **target itself
can be an `Undo` row**, and then the plan is wrong (Major 2).

**Trailing key-hint rule (`be719c6`).** The ruling says a trailing `(x)` on a
menu label means "that key is live, skip the menu". At `be719c6` five labels
violate it: `Checkout <sha7> (detached)`, `Revert <sha7> (-m 1)`, `Reset <cur>
to here (mixed)`, `Reset <cur> to here (hard)`, `Fix: revert the merge (-m N)`.
Ticket 12 (`faed525`, after this hash) rewrote the first four to `, detached`
/ `, -m 1` / `, mixed` / `, hard`; **`Fix: revert the merge (-m N)` still
violates it at `HEAD`**, and no test can catch it: `every_label_names_its_key`
builds the menu with `Vec::new()` for the verbs. Major 3.

**Git on the UI thread.** None. `run_preview` (facts, `plan`, anchor reads)
and `journal_entries` run in jobs; `open_preview`, `history_verbs`,
`build_slice`, `layout` are pure. `graft_graph::layout` over the slice runs
in `preview_produced` on the update thread, bounded by
`slice_of_three_screens_is_built_well_inside_the_frame_budget` (170 rows).

**Terminal escapes.** Every repo-derived string reaching a widget goes
through `sanitize_line`: box lines and commands in `plan_lines`/`ready_box`,
ghost subjects and initials in `view/preview.rs`, menu labels via
`MenuItem::label: SanitizedText` (the type now forbids an unsanitised one).
`an_invisible_character_in_a_ref_name_is_cleaned_out_of_the_label` and
`a_ref_name_carrying_an_escape_cannot_reach_the_buffer` pin it.

**Architecture.** `graft-ui` gained no `graft-ops` edge — `ContextAction::
History(usize)` is an index, the `Operation` mapping lives in
`update::context_menu::verbs` (ARCHITECTURE.md:191 honoured). `update`
names `graft_ui::preview_box::{PREVIEW_MIN, PreviewBox, preview_rows}` the
same way it already names `ConfirmView`; `BoxRect` keeps `Rect` out
(`update_cannot_reach_the_world` still green). `tests/it/seam_rule.rs`
green with the preview read in place. MAP.md entries for `update::preview`,
`update::preview::slice`, `update::context_menu::verbs`, `update::journal`,
`update::undo`, `jobs::preview` match the code.

## Critical

None.

## Major

1. **The preview draws the wrong window once the user is scrolled a screen
   or more.** `crates/graft/src/update/preview.rs:274-298`
   (`resident_slice`) snapshots `[top − height, top + 2·height)`;
   `preview_produced` (`:427`) lays the whole slice out into
   `preview.rows`; `crates/graft/src/view/rows.rs:44-51` hands *all* of
   `preview.rows` to the screen; `crates/graft-ui/src/graph_screen/layout.rs:64`
   draws `rows[..height]` from index 0. With `top ≥ height` the first drawn
   row is `top − height` — one screen above where the user was — and
   `slice_top` is stored but never read. The ticket's own piece 4 says
   "aligned so the cursor's commit keeps its screen row"; it is not. Why it
   matters: on any real repository the user is usually not on screen one,
   and the picture they are asked to confirm shows neither their row nor,
   often, the ghosts. Not caught because `on_screen()` (unit) and
   `forked()` (integration) both have `top == 0`; reviewer mutation A
   confirms no test looks. Fix: in `view/rows.rs`, skip
   `(history.top − preview.slice_top) + ghosts inserted above that point`
   rows (or record the cursor commit's index in `preview.rows` at
   `preview_produced` and window around it), and add a test with a history
   of 3·height rows scrolled to `top = 2·height` that asserts the selected
   commit's subject is on the same screen row before and after `r`.

2. **`Enter` on an `Undo` row that is neither next-undo nor next-redo
   previews a revert that does not go where its title says.**
   `crates/graft/src/update/journal.rs:238-252` sends any other row to
   `open_preview(RevertTo(id))`; `graft_ops::undo::plan_revert_to`
   (`crates/graft-ops/src/undo.rs:389`) takes the range `[target ..]` and
   skips `Undo`/`Redo` entries, so with journal `[A, B, undo(B), C]` and
   target `undo(B)` the plan is `[inverse(C)]` while the box's title reads
   `revert to before "undo: B" (1 steps)`. "Before undo(B)" is a state with
   `B` in force; the result leaves `B` undone. `journal::next_redo` returns
   `None` once an operation newer than the undo is in force, so this row is
   reachable in the browser (it is drawn as a plain row). Keymap §2.11
   promises "Revert to just before this entry". No data loss (every step is
   journaled), but the picture and the act disagree, which is the one thing
   hard choice 3 exists to prevent. Fix: either refuse in words on a
   non-operation row (`enter` checks `is_operation` and says `pick an
   operation row`), or make `plan_revert_to` resolve an `Undo` target to
   its root operation's position. Add a test with that journal shape.

3. **The trailing key-hint rule is broken by the labels shipped beside it,
   and its test cannot see them.** At `be719c6`,
   `crates/graft/src/update/context_menu/verbs.rs:132,195,221,316` end in
   `(detached)`, `(-m 1)`, `(mixed)`/`(hard)`, `(-m N)`; the ruling in the
   same merge says a trailing `(x)` means "press x to skip the menu".
   `crates/graft-ui/src/context_menu.rs:256` (`every_label_names_its_key`)
   passes `Vec::new()` for the verbs, so the invariant is only checked
   over the eight fixed items. Ticket 12 (`faed525`) later fixed four of
   the five; `Fix: revert the merge (-m N)` (`verbs.rs:321` at `HEAD`) still
   reads as a key hint for a key that does not exist. Fix: `Fix: revert
   the merge, -m N`; and give `every_label_names_its_key` (or a sibling in
   `verbs/tests.rs`) a rule over every label the app can produce: a
   trailing `(…)` must be exactly one of the live keys.

4. **`read_anchors` drops a failed commit read with a comment that only
   covers half the cases.** `crates/graft/src/jobs/preview.rs:118-126`:
   `if let Ok(commit) = read_commit(handle, oid) { anchors.push(commit) }`.
   The comment says the slice will refuse by name if the missing commit
   "was holding a ghost up". True — but `wanted()` also collects every
   `RefMove.from`/`to` and a detached `HeadMove` target, and for those a
   silent miss means the badge or `HEAD` mark simply does not appear in the
   picture (`badges_for_preview` adds it on an oid no row has), which is
   exactly the "truncated shape the user could misread" the conductor's
   condition forbids. It also conflates a pruned object with a failed git
   spawn (`RunError`). Fix: distinguish `NotFound` from other errors —
   propagate the latter as the plan's `Err`, and record the former in
   `Planned` (e.g. `missing: Vec<Oid>`) so the box can say `<sha7> is not
   loaded` for a ref target the same way it does for a ghost parent.

5. **The preview's highlighted row is the *choice* cursor, not the selected
   commit.** `crates/graft/src/view/preview.rs:47` passes `index ==
   preview.cursor` as `highlighted`, and `view/rows.rs:51` uses
   `Some(preview.cursor)` as the hover-reveal index; `PreviewState.cursor`
   is `0` for Cancel and `1` for Confirm. So slice row 0 is drawn
   `Highlighted` while the cursor rests on Cancel and row 1 when it moves
   to Confirm, and the commit the user pressed `r` on is never marked.
   Cosmetic on a colourless capture, visible in the terminal, and it is
   the same untested seam as Major 1 (mutation A). Fix: record the selected
   commit's index in `preview.rows` at `preview_produced` and highlight
   that; leave `reveal` `None`.

6. **`update/context_menu.rs` carries ~600 lines of tests inline and
   `context_menu.rs`'s public surface now needs three files to read.**
   PROMPT.md 6.2. `preview.rs` and `verbs.rs` split their tests into
   `tests.rs`; `context_menu.rs` did not, and its `mod tests` now holds the
   fixture builders (`refs_over_the_fixture`, `on_the_feature_row`,
   `choose`) that `verbs/tests.rs` duplicates in its own words. Fix: move
   `context_menu.rs`'s tests to `context_menu/tests.rs` and share the
   fixture with `verbs/tests.rs`.

## Minor

1. `an_open_preview_claims_the_keys_behind_it` tests `preview_key`, not
   `key_event`; the "`u` over a preview does nothing" claim is held by
   reading `keys.rs`. Add one `press(state, 'u')` through `key_event` with a
   preview open and assert no `Command::Undo`.

2. `update/preview.rs:499-503`: `confirm_preview`'s `RevertTo` fallback
   description `"revert to"` is unreachable (`enter` only confirms a
   `Ready` plan) but reads as a real status if it ever fires; and
   `update/undo.rs:150` (`revert_to_forced`) puts the **ULID** on the busy
   line — `revert to before “01J…”` — because the entry's description is
   not carried through `ConfirmThen::RevertTo`. Carry the description.

3. `update/preview.rs:342`: `unreachable!("checked just above")` in
   `preview_key`. The `is_none()` check and the `as_mut()` are separated
   by `choice_count(state)`; restructure as `let Some(preview) = … else {
   return false }` after computing `choices`, and the panic goes away.

4. `jobs/undo.rs:run_revert_to`: a plan that cannot be made (`Err(other)`)
   reports `revert stopped after 0 of 0 steps: there is nothing to revert`
   — "stopped after 0 of 0 steps" is not what happened. Word the
   `Nothing`/`Facts` arms as a refusal (`cannot revert: <why>`), keep
   "stopped after n of m" for `PartWay`.

5. `graft_ops::preview::plan_revert_to` (`crates/graft-ops/src/preview.rs:555`)
   prints `(1 steps)`. Outside this group's fence, but it is on the box
   this group draws. Pluralise.

6. The time-machine capture header (`tests/it/preview.rs`,
   `capture_time_machine_for_docs_shots`) says the box shows "the commands
   it would run"; the builder's own NOTES say `plan_revert_to` yields no
   `commands`, and the capture indeed has none. Fix the header so the user
   is not asked to look for a line that is not there.

7. `update/preview.rs:plan_lines`: a nothing-to-do plan still prints `undo:
   u takes this back` above `nothing to do`. Skip the undo line when
   `nothing_to_do`.

8. `AppState::graph_rows()` (`update/state.rs:251-260`) now calls
   `preview_rows_taken`, which builds a full `PreviewBox` — sanitising every
   line — on each call; `graph_rows()` has 22 call sites and is hit several
   times per frame. Unmeasured; the box is a dozen lines, so probably
   harmless, but cache the row count on `PreviewState` when the plan lands.

9. `RemoteState.diverged_upstream` outlives its words whenever the sticky
   line is replaced by anything other than `Esc` (a job's `Done` line, a
   later refusal): `m`/`r` then silently aim at the upstream from whatever
   row the cursor is on. The preview shows the target before anything
   runs, which is why this is Minor; clear the field whenever
   `state.status` stops being that `Sticky`.

10. `hint_context` stays the graph's while the plan is `Pending`
    (`view/rows.rs:39-43` requires non-empty `rows`), so the footer offers
    `m merge  r rebase` that the modal box swallows. Use `state.preview
    .is_some()` for the hint context and keep the rows condition for the
    slice.

11. `BadgeOverlay::on` (`slice.rs`) removes by **short name**, so a tag and
    a branch sharing a short name on the moved-from commit both vanish
    when only the branch moves. Rare; match on the label kind too.

## KNOWN_GAPS rows to file (builders do not edit that file)

| Item | Status | Expiry |
|---|---|---|
| Preview slice drawn from its top, not the viewport (Major 1); highlight follows the choice cursor (Major 5) | **broken** | M5 (fix round) |
| `Enter` on a non-next `Undo` journal row previews a revert whose title is not where it lands (Major 2) | **broken** | M5 (fix round) |
| `Fix: revert the merge (-m N)` still wears the key-hint shape; the invariant test never sees history verbs (Major 3) | **broken** (cosmetic) | M5 (fix round) |
| `read_anchors` drops a `RefMove`/`HeadMove` target it cannot read; badge silently absent from the picture (Major 4) | **broken** (edge) | M6 |
| `(ghost)` badge absent — `graft_ui::graph_screen::BadgeKind` has no `Ghost` variant; ghost rows carry only the hollow glyph, dashed edges and `—` | broken (wording) | M8 (or decide the glyph is enough) |
| `Esc` = abort a conflicted history operation; `HISTORY_CANNOT_STOP` says so in words instead | unverified (not built) | M7 |
| Pixel tier draws the preview at the text tier | unverified | M8 |
| Ghost lanes approximated to the lanes the node touches; a ghost lane passing *through* a row draws solid | unverified | M6 |
| No test drives a real cancelled undo through the app (`graft-ops` `undo_cancel::*` covers the rollback below it) | unverified | M6 |
| `stale_tip_is_refused_in_words` (a real `git update-ref` between `Ready` and `Enter`) not written; the `Moved` wording is unit-tested, the guard is `graft-ops`' | unverified | M5 (fix round) |
| `plan_revert_to` never tested over a span containing a `Redo` | unverified | M6 |
| Menu/`m`/`r` merge and rebase unverified past the preview (no app test confirms one and reads the repository back) | unverified | M5 acceptance |
| The revert box carries no command lines (`plan_revert_to` returns none) | unverified (`graft-ops`, small) | M6 |
| `graft_ops::apply::remote::explain`'s `NotFastForward` string still says "(M5); shell: git pull --rebase" — reachable only when the tracking tip is unknown | unverified | M6 |
| Row 480 (`ghost_lanes` had no production caller) — `view/preview.rs` is one now | closable | — |
| Rows 501/502 (`Esc` cannot cancel u/U) — `UNDO_CANNOT_STOP` gone, `esc_cancels_an_undo_in_flight` is the proof row 502 asked for | closable | — |
| Row 493 (`OperationKind::RevertTo` unused, expiry "M5 (group 08)") — still unused at `HEAD` (`graft-core/src/journal.rs:184`); the group neither deleted nor claimed it | unverified, expiry passed | re-date or delete |
| Row 494 (`Undone` cannot carry a part-way revert's `done`) — answered by `RevertedTo::Failed { done, total }` | closable | — |

## What I did not judge

Captures (`docs/shots/m5-ui-preview-rebase-*.txt`,
`m5-ui-preview-time-machine-*.txt`, `m5-ui-context-menu-verbs-*.txt`) are the
user's to judge by eye (rule 6); colour is unverified by construction (rule
5). All six are under 8 KB. Group 09's banner, ticket 12's overlay fix and
ticket 13's menu-key rule landed between these merges and are not in scope.
