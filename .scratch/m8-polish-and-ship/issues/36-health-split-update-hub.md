# health — split graft `update/mod.rs` (1406 lines) by seam, move-only

Status: done (see NOTES for each tick's check); was draft (planner, from `docs/reviews/m8-architecture.md` § KNOWN_GAPS disposition)

Build with: **Sonnet** (conductor, after scout `scouts/36-health-split-update-hub.md`). The planner expects **Sonnet**. The work is move-only, and `nextest list`, the line-set check and the purity guard judge it.

Needs:
Blocked by: none (conductor, from Needs)
- 27 merged.
- `health-file-size-ratchet` merged, if it has landed.

Run it after `health-split-job-runner`, not alongside it, because both edit `crates/graft/MAP.md`. If `health-geometry-one-source` is in flight, wait for it; it edits `update/`.

## Read first
- This file.
- `crates/graft/MAP.md`: grep `## update` and read that heading's first ~40 lines only. Then grep `purity` (:955).
- `21b-hygiene-round-2.md` § "Move-only check", steps 1–6.
- 21b-7 ruling 1 (`21b-7-moves-remainder.md`:14–20). The purity scan reads every production file under `update/`, so new children are scanned too, and that is intended.
- Then the scout file.

## Scout (≤1 page)
- `update/mod.rs`'s top-level items with line numbers: the `update` fn, the dispatch arms, `AppState` impls if any, and helpers.
- 2–5 seams. Candidates: message dispatch; product handling; state accessors; helpers already called from one child only.
- Each group's line count.
- Any item the purity guard or `seam_rule.rs` names by path.

## Scope
In:
- Children under `update/`, one per seam.
- `mod.rs` keeps `update` itself, the module list and the re-exports.
- Paths stay the same.
- Update `over_400.txt`.
- MAP: one line per child.

Out: renames, logic changes, splitting the `update` fn's body, and moving items between existing children.

**Target.** Each file is at or under 400 lines. If the `update` fn alone keeps `mod.rs` over 400, that's a stop for `mod.rs`: give the count.

## Signatures
None change.

## Acceptance
- [x] The 21b move-only check, steps 1–6, passes. NOTES lists the `mod`/`use` lines.
- [x] `cargo nextest list --workspace | wc -l` is the same before and after (in NOTES).
- [x] `update::tests::purity::update_cannot_reach_the_world` passes and scans the new children. NOTES gives its file count before and after.
- [x] Each file's `wc -l` is in NOTES, each ≤ 400 or a stop.
- [x] `file_size::every_file_over_400_lines_is_listed_with_its_count` passes.

## Touches
- graft `update/`
- the purity guard
- `seam_rule.rs`'s path patterns: check with `rg`
- the keymap sweeps under `update/tests/`: they must still pass, and they are not edited
- graft MAP `## update`
- `over_400.txt`

No behaviour, keymap or capture changes.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 55k tokens.

## Conductor addition (as found, no new decision)

- [x] `ARCHITECTURE.md` still names `jobs/runner.rs`; since 35 it is `jobs/runner/` (`not_a_job` lives in `runner/dispatch.rs`). Fix that line with this ticket's own ARCHITECTURE edits. Check: `rg -n 'jobs/runner\.rs' ARCHITECTURE.md` → no hits. **Ticked:** both mentions (:161, :196) now `jobs/runner/dispatch.rs`; `rg` → no hits.

## NOTES — 36 (builder, 2026-10-09)
Scout used. No compaction. Status: **done**.
- Acceptance ticks (verifying check named):
  - [x] 21b move-only check 1–6: `comm` of sorted non-blank lines (HEAD `mod.rs` vs the 6 files): new-only lines are only `mod`/`use`/`//!` lines, `pub(super)` widenings, and fmt re-wraps of widened signatures; `docs/shots` untouched; one commit.
  - [x] `nextest list --workspace`: 6222 before / 6222 after, sorted `diff` empty.
  - [x] `update::tests::purity::update_cannot_reach_the_world` passes (full nextest run); scanned production files under `update/` (`find`, mirroring `is_test_module`) 159 → 164.
  - [x] `wc -l`: mod 271, answer 273, answers 281, tree_answers 232, facts 143, dispatch 311. All ≤ 400.
  - [x] `file_size::every_file_over_400_lines_is_listed_with_its_count` passes (full nextest run); `update/mod.rs` row (1420) removed from `over_400.txt`.
- Seams: `answer.rs` (`answer`); `answers.rs` (mergetool…operate answers); `tree_answers.rs` (repository…stash_only_answered); `facts.rs` (took_fact, refs_failed, repository_open_failed, name_prompt_claimed); `dispatch.rs` (dispatch_key…show_rows).
- `mod`/`use` lines in mod.rs: added `mod answer; mod answers; mod dispatch; mod facts; mod tree_answers;`, `use answer::answer;`, `pub(crate) use dispatch::dispatch_key;`, `use dispatch::{dispatch, show_rows};`, `#[cfg(test)] use graft_core::job::JobId;` (tests name `crate::update::JobId`). Dropped `KeyPress`/`History`/`Phase`/`RepositoryFacts`/`credential_needed` uses. Each child has a `//!` line, an explicit `use super::{…}` list (no glob, because pedantic `wildcard_imports` rejects it), and sibling `use super::<child>::…` lines.
- Docs: the last sentence of the mod.rs `//!` now names the children. Updated the first line of MAP `## update`.
- Gates: fmt ok, clippy -D warnings ok, nextest 6222/6222 (14 skipped), test --doc ok.
