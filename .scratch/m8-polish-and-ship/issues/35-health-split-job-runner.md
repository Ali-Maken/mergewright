# health — split graft `jobs/runner.rs` (1457 lines) by seam, move-only

Status: done (builder, 2026-10-09)

Build with: **Sonnet** (conductor, after scout `scouts/35-health-split-job-runner.md`). The planner expects **Sonnet**. The work is move-only, and `nextest list` plus the line-set check judge it.

Needs:
Blocked by: none (conductor, from Needs)
- 27 merged.
- `health-file-size-ratchet` merged, if it has landed.

Don't run it alongside `health-split-update-hub`. Both edit `crates/graft/MAP.md`, so run them one after the other (the two-helper limit applies anyway).

## Read first
- This file.
- `crates/graft/MAP.md`: grep `## jobs` and `runner`, and read those ranges only (about :160–171 and :1069–1100).
- `21b-hygiene-round-2.md` § "Move-only check", steps 1–6.
- 21b-7 ruling 3 (`21b-7-moves-remainder.md`:26–30).
- Then the scout file.

## Scout (≤1 page)
- `runner.rs`'s top-level items with line numbers.
- The inline test span.
- 2–5 seams. Candidates: `JobRunner` and its queue; per-family job fns (history and detail reads, remote, operations, map); `Product` assembly.
- Each group's line count.

## Scope
In:
- `jobs/runner/` children, one per seam.
- Inline tests move out first.
- Public (`pub(crate)`) paths stay the same, through re-exports.
- Update `over_400.txt`.
- MAP: one line per child.

Out: renames, logic changes, and changes to `Product`'s shape. Minor 7 in the review, typed errors, is not this ticket.

**Target.** Each file is at or under 400 lines. A group still over after one seam is a stop for that group: give the count and the seam needed.

## Signatures
None change.

## Acceptance
- [x] The 21b move-only check, steps 1–6, passes. NOTES lists the `mod`/`use` lines. **Check:** line-set script (NOTES) against the pre-split file: only `use`/`mod`/`//!` lines, one `//` comment, `pub(super)` widenings and their fmt re-wraps are new; `git diff --stat` on `docs/shots` empty; one commit.
- [x] `cargo nextest list --workspace | wc -l` is the same before and after (in NOTES). **Check:** 6222 = 6222, sorted lists byte-identical (`cmp`).
- [x] Each file's `wc -l` is in NOTES, each ≤ 400 or a stop. **Check:** `wc -l crates/graft/src/jobs/runner/*.rs`, max 324.
- [x] `file_size::every_file_over_400_lines_is_listed_with_its_count` passes. **Check:** that test in the workspace nextest run; `runner.rs` row removed from `over_400.txt`, no new file over 400.

## Touches
- graft `jobs/`
- `event_loop`'s calls into the runner: paths unchanged, and the gates prove it
- graft MAP `## jobs`
- `over_400.txt`

No behaviour or capture changes.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 50k tokens.

## NOTES — 35 (builder, 2026-10-09)
Scout used (`scouts/35-health-split-job-runner.md`). No inline tests existed, so nothing moved to `tests.rs`.
- Split `jobs/runner.rs` (1481) → `jobs/runner/`: mod.rs 324, queue.rs 277, reads.rs 315, operations.rs 316, lists.rs 219, dispatch.rs 118.
  Seams per scout: lifecycle (mod), submit/announce/queue + `dispatch_detail`/`dispatch_history` + radar probes (queue), impl #2 in three families (reads / operations / lists), `impl Dispatch` + `detail_read!` + `not_a_job` (dispatch).
- `mod`/`use` lines: mod.rs gains `mod dispatch; mod lists; mod operations; mod queue; mod reads;` and a private `use super::{clone, commit_message, conflict, detail, finish, fold_count, journal_list, map, operate, palette, preview, radar, remote, rewrite, rewrite_open, stash, stash_only, switch_list, undo};` so the moved bodies' `super::<module>` paths resolve unchanged (one `//` comment says so). Each child: the old `use` block trimmed to what it uses, `use super::X` → `use crate::jobs::X`, plus `use super::JobRunner;`. mod.rs drops `Dispatch` from its import.
- Visibility: 47 job methods `fn` → `pub(super) fn` so `dispatch.rs` (and mod/queue) reach them across siblings. No other line changed.
- nextest list: 6222 before, 6222 after, sorted lists identical.
- Public path `jobs::JobRunner` unchanged (`jobs/mod.rs` `pub use runner::JobRunner` untouched).
- Stale doc path, outside crate set: `ARCHITECTURE.md` names `jobs/runner.rs` (the `not_a_job` arms now live in `jobs/runner/dispatch.rs`). Conductor to fix.
