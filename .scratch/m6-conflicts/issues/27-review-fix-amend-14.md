# review-fix-amend-14: three over-long files, clever over plain in the pane code, the `two_boxes_over_content` flake

Status: ready
Blocked by: 25-stable-file-order-and-done-file-open.md (building; same crates, ADR-0036). Runs before 15.
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-14.md` Major 1, 2, 3.
Build with: **Opus** (low) — splits, plain-code rewrites, one test wait.
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-14.md`: Major 1, 2, 3 — each names file and line.
- `crates/graft/MAP.md` (`view::resolver`), `crates/graft-ui/MAP.md` (`conflict_resolver::panes`).
- `ARCHITECTURE.md`; `codebase-design` for the splits. PROMPT.md 6.2 (readability).

## Scope
In:
1. **Major 1 — sizes.** Split, pure moves, to ~400 lines where a seam exists: graft `view/resolver.rs` (727); graft-ui `conflict_resolver/tests/three_panes.rs` (467); `conflict_resolver/draw.rs` (402). MAP.md updated. No seam → say so with the count.
2. **Major 2 — plain over clever.** `panes/draw.rs:88` tuple parameter → named struct or separate params; `panes.rs:264-289` magic pane indices 0/1/2 → named (enum or consts); `panes.rs:225-236` a `let-else` that can never fire → remove or make the impossibility a type. Behaviour unchanged; existing tests and goldens pass unchanged.
3. **Major 3 — flake.** `tests/.../two_boxes_over_content.rs:102-121` waits for the frame without waiting for the git version check; the commit-graph footer and row count depend on it (`update/state.rs:355`, `:378`). Fix the cause: the wait also requires `state.system_git.is_some()`. No sleep, no retry (rule 2). Search for other captures with the same wait shape and list them in NOTES (fix them if the fix is the same one line).

Out: review-14 Minors (→ KNOWN_GAPS); Minor 5 (`three_panes_capture` vs 15's advance) is ticket 15's.

## Acceptance (each tick names its test, at tick time)
- [ ] Major 1: splits done, line counts in NOTES, MAP.md updated — (gates green; list files)
- [ ] Major 2: three rewrites, goldens unchanged — (name the goldens/tests that pin it)
- [ ] Major 3: the wait includes `system_git`; other same-shape waits listed — test:
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
