# 33c radar-failed-context — a failed radar read hides nothing it did find, and a crashed read says so

Status: ready
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
- [ ] **m1 view, red first:** `view::radar::tests::an_evil_merge_shows_while_the_context_failed` — context
      `Failed("…")`, selected merge's evil verdict `Evil`: the radar lines are exactly `[NotChecked(..), <evil line>]`
      in that order, and `findings_for` includes the `Evil` finding. Red today (evil absent).
- [ ] **m1 view, the other states:** same, context `Idle` and `Pending`: evil line present, no NotChecked. `Ready`
      unchanged (existing tests green).
- [ ] **m1 marker:** the test that asserts `marker_for` is None on `Failed` (33's) is updated to: None for a plain
      commit, the evil marker for an evil merge. Name both assertions. Red first for the evil half.
- [ ] **m1, existing test:** `view::radar::tests::a_failed_context_says_radar_not_checked` — if it asserts no evil line
      on `Failed`, it is changed to the ruling above; say which assertion changed and why.
- [ ] **m2 context, red first, through `update`:** `update::tests::a_crashed_radar_context_job_reads_as_failed` —
      context `Pending(id)`, feed `Event::JobFailed(id, Panicked)`: context is `Failed(_)` carrying the failure words;
      then 5 events that don't move the selection (name them): no `Command::RadarContext`. Then `Ctrl+R`: exactly one.
- [ ] **m2 evil, red first, through `update`:** `update::tests::a_crashed_evil_probe_reads_as_not_checkable` —
      `detail.evil` `Pending(id)`, `JobFailed(id, Panicked)`: evil is `Failed(_)`; the view draws
      `evil merge: not checkable — <why>`, not "checking".
- [ ] **m2 no loop, red first:** `update::tests::a_failed_evil_probe_is_not_asked_again_for_the_same_commit` — after
      the above, 5 events that don't move the selection: no `Command::EvilProbe`. Moving to another merge and back:
      one probe each way. Red is expected only once `job_failed` sets `Failed` (the loop is live then); show it red
      with the `job_failed` arm in and the `probe_evil` guard out.
- [ ] **m2 audit:** NOTES, one line per `Pending` holder with lines and verdict; each fix it makes has its own
      red-first test named here.
- [ ] **Capture** `docs/shots/m7/radar-not-checked-evil-120x40.txt`, drawn by `view::view` via `frame_at` (as 33's),
      context `Failed("commit b32274b is not in the repository")`, an evil merge selected, verdict `Evil`, asserted by
      a named test. **Right:** the detail pane's radar block's first line is
      `radar: not checked — commit b32274b is not in the repository · ctrl+r to try again`; the next line is the
      evil-merge line (quote its exact words in NOTES); the graph's marker column shows the evil marker on that merge's
      row and on no other row. Colour unverified (rule 5). By eye: user.
- [ ] `docs/shots/m7/radar-not-checked-120x40.txt` (33's, plain commit) is byte-identical; show by `git diff --stat`.
- [ ] graft MAP.md: `job_failed` (what a crashed job leaves: `Failed`, never `Pending`), `probe_evil` (asks from `Idle`
      or another commit only), `findings_for` (evil ignores the context). Checked by reading; say so.
- [ ] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
      `cargo test --doc`.

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
