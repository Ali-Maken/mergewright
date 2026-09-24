# start-over-a-whole-file-choice: `d` walks back a keep-or-delete choice

Status: ready
From: user picks 2026-09-24 item 9 (batch 7, "yes"); `docs/design-notes/m6-conflicts.md` :830 (the proposal);
prototype scene 8 (:1290-1300).
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: (1) graft-ops: `restart_gate`, `restart_source`, `prepare_restart`, `run_restart_prep`, `stage_source`,
`restore_stages`, `operation::shape_of_stages` / `has_markers`, and where `NotMarkerShape` is decided for a `Backup`
stage set; what `RestartConflict`'s plan runs after the stages are back (`checkout --merge`); which shapes
`UnmergedHow` names. (2) graft: `jobs::conflict::StartOver` (what the app keeps of the gate's answer),
`update::resolver::restart` (`restart_box`, `start_over_refusal`), `restart_landed`, and the box's words today. What
a shell-begun merge's gate answers today for a done whole-file file (the refusal and its words).
Needs: `undo-and-abort-a-shell-begun-merge` merged (same crates, lane O, second).
Crates: graft-ops, graft.
Lane: wave 1, lane O, second.

## Read first
- `docs/design/02-keymap.md` §2.6 the `d` row (edited 2026-09-24).
- `docs/design-notes/m6-conflicts.md` :820-835 (the proposal) and prototype scene 8 (:1290-1300);
  `.scratch/m6-conflicts/spec-pass-edits-2026-09-24.md` E6, J17.
- `crates/graft-ops/MAP.md` §apply `StageSource` … `restart_gate` (:172-174), `Refusal::NotMarkerShape` (:48).
- `crates/graft/MAP.md` §update::resolver `DoneOpen` / `start_over_refusal` (:330), `restart_landed` (:333),
  `WholeFileShape` (:318).

## Scope
**In:**
- The gate: a **done** file whose recovered stage set (`StageSource::Backup`) is one of `UD`, `DU`, `AU`, `UA` is
  restartable as a **whole-file** start-over, in an operation begun in Graft. `DD` (both deleted), a binary shape, and a
  file still unmerged (`StageSource::Index` with a one-sided shape: nothing chosen yet) keep today's refusals.
- The write: the stages go back into the index (`restore_stages`, as for markers), then the surviving side's blob goes
  back to the worktree — stage 2 for `UD`/`AU`, stage 3 for `DU`/`UA` — with `git checkout --ours|--theirs -- <path>`,
  the command in the Command Log; no `checkout --merge`. Journaling exactly as a marker start-over today (folded into the
  live entry in a Graft operation).
- The box: the existing start-over box, plus one line `the choice comes back: keep or delete`. Cursor on Cancel. Still
  `not undoable while the merge is open` (the box says so, as for markers).
- After it lands (`restart_landed`): the same file reopens as its whole-file card
  (`deleted by them · o keeps the file  t deletes it`), the list row `keep or delete`, `Choices` forgotten for it.
- The bar and identity row offer `d start over` on such a done file (they read ops' gate answer already —
  `start_over_refusal` is the one predicate).

**Out:**
- Shell-begun merges: unchanged (refused as today; KNOWN_GAPS :613 is a separate row).
- `DD`, binary.
- The keymap text (done in the spec pass).

## The failing test first
graft-ops `tests/it/restart.rs` (or the file the scout names): `start_over_brings_back_a_modify_delete_choice`.
- Setup: a merge begun through `apply(Merge)` stops with `gone.txt` `UD`; `ResolveWholeFile` keeps yours (the file is
  staged, not unmerged). `apply(RestartConflict { path: gone.txt })`.
- Assert: `git ls-files -u -- gone.txt` lists stages 1 and 2; the worktree file equals stage 2's blob; the merge is
  still live.
- Red today: `NotMarkerShape`.

## Signatures
```rust
// graft-ops/src/apply/restart.rs (or where restart_gate lives)
pub enum RestartKind { Markers, WholeFile { how: UnmergedHow } }
pub struct RestartPlan { pub source: StageSource, pub kind: RestartKind }
pub fn restart_gate(repo: &Repo, facts: &RepoFacts, path: &Path)
    -> Result<Result<RestartPlan, Refusal>, RestartReadError>;        // was Result<Result<StageSource, Refusal>, …>
// prepare_restart / run_restart_prep (crate-private) match on RestartKind; the one rule stays in restart_source.
// graft/src/jobs/conflict.rs: StartOver keeps the kind, so the box can word it:
//   StartOver::Can { kind: RestartKind } (or the scout's equivalent shape)
```
`Repo` and argument names follow what exists; only the `Ok` payload changes.

## Touches (each entry, and how the builder checks it)
- **Every caller of `restart_gate`** (the app's done-file read and the whole-file card's gate).
  - Check: `grep -rn "restart_gate" crates`; each adapts to `RestartPlan`.
- **`prepare_restart` and the gate agree** (one rule, two callers — the m6-conflicts/28 invariant).
  - Check: `gate_and_apply_agree_on_every_shape` — a table over `UU` markers, `UD`, `DU`, `AU`, `UA`, `DD`, binary, and
    still-unmerged one-sided: the gate's yes/no equals whether `apply(RestartConflict)` writes.
- **Each whole-file shape restores the right side.** Check: `start_over_restores_the_surviving_side` (four rows).
- **After a delete choice** (the file is gone from disk): start-over writes it back. Check: a row in the same table.
- **Journal and undo:** folded into the live entry like a marker start-over; `u` after the operation ends restores
  the pre-restart state as it does for markers. Check: `undo_after_a_whole_file_start_over_matches_markers`.
- **The app:** the box words, the landing card, the list row, `Choices::forget`.
  - Check: `d_on_a_done_modify_delete_file_reopens_its_card` (graft `it/resolver/`), and the done card's keys show
    `d start over` (`done_whole_file_offers_start_over`).
- **Shell-begun merge unchanged.** Check: `start_over_of_a_whole_file_is_still_refused_in_a_shell_merge`, with today's
  words.
- **Hints:** `restart: true` on a done whole-file file. Check: `every_key_the_bar_draws_is_answered`.
- **Captures:** `fix-done-*-200` and any done whole-file capture gain `d start over`. Check: regenerate, list in NOTES.
- **The walk:** scenario `walk-back-a-whole-file-choice`: start on `notes.txt` after `gone.txt` was kept; keys `k d → ⏎`
  (4). Check: `journey_counts_match_the_note` with the row below.
- **Docs:** `crates/graft-ops/MAP.md` (`restart_gate`, `RestartKind`, `RestartPlan`); `crates/graft/MAP.md`
  §update::resolver (`DoneOpen`, the box words).

## Acceptance (name the verifying test at tick time)
- [ ] `start_over_brings_back_a_modify_delete_choice`: red first, then green.
- [ ] `start_over_restores_the_surviving_side`
- [ ] `gate_and_apply_agree_on_every_shape`
- [ ] `undo_after_a_whole_file_start_over_matches_markers`
- [ ] `d_on_a_done_modify_delete_file_reopens_its_card`, `done_whole_file_offers_start_over`
- [ ] `start_over_of_a_whole_file_is_still_refused_in_a_shell_merge`
- [ ] `journey_counts_match_the_note` with `walk-back-a-whole-file-choice` = 4.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-start-over-whole-file-120x40.txt`, three
      frames: `gone.txt` done (bar with `d start over`); the box; the card after `→ ⏎`.
  - **What right looks like:** frame 2 contains `the choice comes back: keep or delete` and `[ Cancel ]` under the
    cursor; frame 3's card reads `deleted by them · o keeps the file  t deletes it` and the list row `keep or delete`.
- [ ] MAP.md updated (doc, no test).

## For the conductor (exact text, in this spec's merge commit)
Add to the `journey-counts` fence, after `walk-back-a-done-file`:
```
walk-back-a-whole-file-choice 4                     # batch 7 (approved 2026-09-24): k d → ⏎ · ticket 50 → 3 (k d y) · Tower (?)
```

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium. One context.

## NOTES
