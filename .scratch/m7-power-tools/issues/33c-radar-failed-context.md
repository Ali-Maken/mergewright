# 33c radar-failed-context — a failed radar read hides nothing it did find, and a crashed read says so

Status: done
Milestone: M7 (m7-power-tools)
Source: review `docs/reviews/m7-18f-32b-06f-33.md` Minors 1–2 (gap rows M7/33r-m1, M7/33r-m2). Spec by planner, 2026-09-27.
Blocked by: none by content (33, 33b merged). Crate graft: the conductor sequences it against whoever holds it.
Crates: graft only (`update/mod.rs` `job_failed`, `update/detail/ops.rs` `probe_evil`, `view/radar.rs`, their tests,
`docs/shots/m7/`, graft MAP.md). graft-ui: **no change expected** (`RadarLine::NotChecked` and the
`evil merge: not checkable — <reason>` line exist). Needing graft-ui → `blocked: needs graft-ui`, stop.
Build with: **Sonnet**. Two cause fixes on decided patterns; every decision is below.
Scout: none. The review named every site with its line at a fixed hash; re-read each on `main` before editing, and
say in NOTES if any moved.
Needs: none.

## The defects (verified by the review, rule 7)
- **m1.** The evil probe needs no context: `probe_evil` (`update/detail/ops.rs:173-206`) sends
  `Command::EvilProbe(id, oid)`. But the view hides its answer unless the context is `Ready`: `findings_for` returns
  early (`view/radar.rs:77-79`), dropping a found `Evil` (`:110-114`); the evil line is gated `&& context_ready`
  (`:179`). On `Failed` the probe runs, an evil merge is found, and the pane says only "radar: not checked".
- **m2.** A job that panics or can't start becomes `Event::JobFailed` (`jobs/pool.rs:179`, `:195`; routed
  `update/mod.rs:891`). `job_failed` (`update/mod.rs:1013-1047`) never touches `radar.context` or `detail.evil`, so
  they stay `Pending(id)`: no radar line (NotChecked draws on `Failed` only, `view/radar.rs:169`), and the evil line
  says "checking" (`:187`) until the selection moves.

## Ruling
**m1: (a) show it.** Ticket 33 already ruled "the evil-merge line, which does not need the context, still shows"; the
builder kept the gate and asked. Why (a) over (b) "don't probe": the context fails in the shallow case (20f), where he
can least see history himself; a merge that changed code nobody reviewed is the finding he most needs there. (b) would
hide it by not looking. 33's ruling stands unamended.
- `findings_for`: the evil part moves **above** the `Ready` early return; everything else stays gated on `Ready`.
- `:179`: drop `&& context_ready`. The evil line shows on `Idle`/`Pending`/`Ready`/`Failed` alike.
- On `Failed` the radar block reads, in order: `radar: not checked — <why> · ctrl+r to try again`, then the evil line
  (existing words, unchanged). The graph's marker column shows the evil marker on that merge, and no other marker.
- No word changes. "not checked" is read with its reason line; `RadarLine::NotChecked`'s doc already says it covers
  findings that need the context. Correct that doc only if it claims otherwise.

**m2: a crashed job is an answer, like a failed one.** In `job_failed`, for the failed id:
- `radar.context == Pending(id)` → `Failed(failure.to_string())`. The pane then gets 33's NotChecked line; `Ctrl+R`
  retries (33's rule: `request_context` asks only from `Idle`).
- `detail.evil == Pending(id)` (whatever its shape holds the id as) → `Failed(failure.to_string())`, drawn as the
  existing `evil merge: not checkable — <why>`.
- **Knock-on that must be fixed with it (the loop):** 33's audit (its NOTES, `probe_evil` bullet) found `probe_evil`
  would **re-probe on `Failed` for the same commit** (`ops.rs:184-186`) and runs every update via
  `follow_selection` from `settle` (`selection.rs:117`). Today the arm is dead because nothing sets `Failed`; this
  ticket makes it live. So `probe_evil` probes a commit only when its evil state is `Idle` or belongs to another
  commit; `Failed` for the selected commit waits for the selection to move or `Ctrl+R`. Same cause and cure as 33
  (a failure is an answer, not "not asked yet"). No retry, backoff or timer (rule 2).
- **Audit every other `Pending(id)` holder** that a `JobFailed` could strand: radar `duplicates`, `force_push` probes
  (the review: "if they hold a `Pending(id)`"), journal `entries`, `remotes`, and any other `Loading::Pending` /
  pending-id field in `AppState` (grep `Pending(`). One NOTES line each: holds an id yes/no, stranded by
  `job_failed` yes/no, with lines. A yes in graft → the same arm here, plus a check that its requester does not
  re-ask on `Failed` every update (line). A yes needing more than one arm and its guard → question to the conductor,
  not a fix.
- The transient status `<id> failed: …` (`mod.rs:1041`) stays.

## Read first
- This ticket; ticket `33-radar-retry.md` Ruling and NOTES (the audit).
- graft MAP.md: `## update::radar`, `job_failed` / `update` entry, `AppState::radar`, `AppState::detail`, `Loading`.
- Source, only: `update/mod.rs:1013-1047` and `:166-190`; `update/detail/ops.rs:173-220`; `update/selection.rs`
  around `:117`; `view/radar.rs:60-120`, `:160-210`; `jobs/pool.rs:170-200`.

## Signatures
No public signature changes. `job_failed`, `probe_evil`, `findings_for` keep theirs; only their bodies change.

## Acceptance (name the verifying test at tick time; each new test shown red before the fix, with its failure quoted)
- [x] **m1 view, red first:** `view::radar::tests::an_evil_merge_shows_while_the_context_failed`. Red: panicked at
      `view/radar.rs:381` (findings empty on `Failed`).
- [x] **m1 view, the other states:** `view::radar::tests::the_evil_line_shows_on_an_idle_or_pending_context` (Idle and
      Pending: `[EvilClean]`, then `[Finding]` for Evil; no NotChecked). Red at `:406`. Ready: existing
      `the_evil_line_follows_the_probe_through_each_loading_state`, `an_evil_verdict_is_one_finding_and_no_separate_evil_line` green.
- [x] **m1 marker:** 33's test kept its plain-merge-no-verdict `marker_for == None`; the new
      `view::radar::tests::a_failed_context_marks_only_the_evil_merge` asserts `"plain commit"` None and `"evil merge"`
      = evil severity marker. Red on the evil half only (`:438`); the plain half passed.
- [x] **m1, existing test:** `view::radar::tests::a_failed_context_says_radar_not_checked` — the `Failed` assertion
      changed from `[NotChecked]` to `[NotChecked, EvilUnchecked]`: the merge is unprobed there, and the evil line now
      shows on every context (ruling m1). Red before the fix at `:357`.
- [x] **m2 context, red first:** `update::radar::tests::a_crashed_radar_context_job_reads_as_failed` (in
      `update::radar`, not `update::tests`: it reuses that module's `state_with_default_branch`/`press`/`context_asks`).
      `JobFailed(5, Panicked)` → `Failed(..lane 3 vanished..)`; 5× `j` on an empty history (no selection to move): 0
      `RadarContext`; `Ctrl+R`: 1. Red with the radar arm out: `Pending(JobId(5))` at `radar.rs:752`.
- [x] **m2 evil, red first:** `update::tests::a_crashed_evil_probe_reads_as_not_checkable` — `Failed(..)`, view draws
      `EvilNotCheckable("…tree walk broke")`, no `EvilChecking`; view half also
      `view::radar::tests::a_failed_evil_probe_reads_as_not_checkable` (red: `right: [EvilNotCheckable("job panicked")]`).
- [x] **m2 no loop, red first:** `update::tests::a_failed_evil_probe_is_not_asked_again_for_the_same_commit` — 5×
      `JobFinished(900..904)` (unrelated ids): 0 `EvilProbe`; `k` (to the plain row): 0; `j` back: 1. Red with the arm
      in, guard out: `assertion failed: matches!(state.detail.evil, EvilLoading::Failed(_))` at `tests.rs:5652`, and the
      sibling showed `Pending(JobId(9223372036854775810))` — the re-probe loop live.
- [x] **m2 audit:** NOTES below. Fixes: `update::radar::tests::a_crashed_duplicates_probe_reads_as_failed` (red:
      `matches!(state.radar.duplicates, Loading::Failed(_))` at `radar.rs:784`),
      `update::radar::tests::a_crashed_force_push_probe_reads_as_failed` (red: `{"main": Pending(JobId(7))}` at `:815`).
- [x] **Capture** `docs/shots/m7/radar-not-checked-evil-120x40.txt`, asserted by
      `radar_not_checked::capture_radar_not_checked_with_an_evil_merge` (real `git`, `Shape::EvilMerge`, real probe
      verdict, context then set `Failed`). The pane joins radar lines with ` · ` on one row, so "next line" reads as
      next item: row 26 is exactly
      `Merge branch 'alpha' · radar: not checked — commit b32274b is not in the repository · ctrl+r to try again · evil merge`.
      Evil words: `evil merge`. Marker: `⚠` on the `Merge branch 'alpha'` row only (asserted: exactly one row with ⚠).
      Before the `view/rows.rs` fix the frame had no ⚠ at all (observed in the failing run). Colour unverified (rule 5).
      By eye: user.
- [x] `docs/shots/m7/radar-not-checked-120x40.txt` byte-identical: `git diff --stat -- docs/shots/` empty (only the new
      untracked file); `capture_radar_not_checked` green.
- [x] graft MAP.md: `update::radar::job_failed` (new row), `DetailState::probe_evil` guard, `DetailState::evil_job_failed`
      (new row), `findings_for`. Checked by reading.
- [x] Gates workspace-wide: fmt ok, clippy ok, `cargo nextest run --workspace` 4207/4207 (3 skipped), `cargo test --doc` ok.

## Touches (check each; NOTES says how)
- **Consumers of `findings_for`** (the evil finding now appears on non-`Ready` contexts): the marker column
  (`marker_for`), the map's radar marks (`update/map.rs` / map view), the `.` context menu `marked`
  (`open_for_history_row`, graft MAP.md). Grep `findings_for`; for each, state what it shows on `Failed` with an evil
  merge, by test or reading with line.
- **`probe_evil`'s other callers** (grep): the guard change must not stop the first probe of a newly selected merge.
- **`job_failed`'s existing arms** (`awaiting`, `promoting`, status read, refs read): unchanged; existing tests green.
- **Captures:** grep `docs/shots/` and insta snapshots for radar blocks; only the new capture changes.
- **Keymap:** none (`Ctrl+R` exists). **Journal/undo:** none (read-only).
- **graft-ui:** none expected; grep `RadarLine::` to confirm no exhaustive match needs an arm.

## Size
~35k. One context.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Report says whether cause or
symptom, what it costs him, and the Touches line.

## NOTES (builder, 2026-09-27)
- Scout: none (ticket says so). Sites re-read on this branch: `findings_for` gate at `view/radar.rs:77`, evil gate
  `:179`, `probe_evil` guard `ops.rs:184-186`, `job_failed` `mod.rs:1013` — none moved.
- **Extra site the ticket did not name:** `view/rows.rs:105` gated the whole marker column on context `Ready`, so the
  evil `⚠` was computed but never drawn on `Failed`. Now `Ready || detail.evil == Ready(Evil)` (and seen). The
  no-history frame (`:189`) unchanged (no rows, no evil).
- Cause fixes, both. Cost to him: on a failed context he now sees an evil merge he found; a crashed probe says why
  instead of "checking" forever. Retrying a crashed evil probe needs moving off the merge and back (Ctrl+R does not
  reset `detail.evil`; `reset_for_refresh` touches radar only) — as ruled.
- Audit of `Pending(id)` holders vs `job_failed`:
  - `radar.context` (`radar.rs:342`): id yes, was stranded → fixed; requester asks from `Idle` only (`:335`).
  - `radar.duplicates` (`:527`): yes, stranded → fixed; asks from `Idle` only (`:524`).
  - `radar.force_push[branch]` (`:431`): yes, stranded → fixed; candidates skip branches in the map (`:456`).
  - `detail.evil` (`ops.rs`): yes → fixed + guard.
  - **Question to the conductor (not fixed, each its own arm+guard, outside radar):** also stranded by `job_failed`
    — `detail.files`/`detail.diff` (`prelude.rs:54-55` shows them gating "quiet"), `stash.entries` (`stash.rs:58`),
    `stash.preview` (`:288`), `journal.entries` (`journal.rs:56`), `remotes` (`remote.rs:250`), `map.data`
    (`map.rs:57`), `preview.plan` (`preview.rs:342`), finish `summary` (`finish.rs:158`; `finish::job_ended` at
    `mod.rs` may already cover it — not verified). None is touched by `job_failed` (read `mod.rs:1013-1047`).
    `refs_read` is handled (`answered(id,false)`).
- `findings_for` consumers on `Failed` + evil merge (by reading): marker column (`view/entries.rs:314`, capture
  shows ⚠), map strip (`view/strip.rs:55`, now marks the evil merge too), `.` menu `marked`
  (`update/context_menu.rs:61`, now true on that merge → explain offered), `open_explain` (`detail/control.rs:137`,
  finds the evil finding). All consistent with "the evil finding shows".
- `probe_evil` other callers: only `selection.rs:117`; first probe of a new merge covered by
  `an_evil_merge_on_a_branch_is_still_marked` and the away/back half of the no-loop test.
- graft-ui: no change; `RadarLine::` matches in graft-ui unchanged (no new variant).
