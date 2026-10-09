# health — split graft `jobs/runner.rs` (1457 lines) by seam, move-only

Status: draft (planner, from `docs/reviews/m8-architecture.md` § KNOWN_GAPS disposition)

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
- [ ] The 21b move-only check, steps 1–6, passes. NOTES lists the `mod`/`use` lines.
- [ ] `cargo nextest list --workspace | wc -l` is the same before and after (in NOTES).
- [ ] Each file's `wc -l` is in NOTES, each ≤ 400 or a stop.
- [ ] `file_size::every_file_over_400_lines_is_listed_with_its_count` passes.

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
