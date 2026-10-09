# health — split graft `update/mod.rs` (1406 lines) by seam, move-only

Status: draft (planner, from `docs/reviews/m8-architecture.md` § KNOWN_GAPS disposition)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. The work is move-only, and `nextest list`, the line-set check and the purity guard judge it.

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
- [ ] The 21b move-only check, steps 1–6, passes. NOTES lists the `mod`/`use` lines.
- [ ] `cargo nextest list --workspace | wc -l` is the same before and after (in NOTES).
- [ ] `update::tests::purity::update_cannot_reach_the_world` passes and scans the new children. NOTES gives its file count before and after.
- [ ] Each file's `wc -l` is in NOTES, each ≤ 400 or a stop.
- [ ] `file_size::every_file_over_400_lines_is_listed_with_its_count` passes.

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
