# 20d blame-shallow-flag — Part A of 20's shallow-boundary ruling

Status: done
Blocked by: 18c (merged aeee9040: `shallow_boundaries`)
Crates: graft-git only (`blame.rs`, its tests, MAP.md).
Build with: Sonnet (the planner sized it at ~10k: one field set after git exits, one error variant).
Scout: none.
Split by: conductor, 2026-09-26. The spec is `20-blame-place.md` "Ruling (planner): shallow boundary", **Part A**, which
the planner left for the conductor to place. No new decision.

## Scope
Exactly Part A of that ruling: `BlameCommit::shallow`, set by `blame` from 18c's `shallow_boundaries(repo)` once after
git exits; `parse_porcelain` unchanged; the error variant; the tests it names (a `--depth 2 file://` clone), red first;
MAP.md.

## Acceptance
- [x] Every test Part A names, red first (compile-red: no field/variant): `blame::blame_marks_lines_from_the_shallow_boundary`, `blame::a_full_clones_root_commit_is_not_shallow`, `blame::an_unreadable_shallow_file_fails_blame`.
- [x] `parse_porcelain` logic unchanged; diff is one line in `blame/porcelain.rs`, the literal `shallow: false,` the new field needs. Its tests green (`blame::porcelain` unit tests in nextest run).
- [x] Gates workspace-wide: fmt, clippy -D warnings, nextest, test --doc.

## NOTES (builder, 2026-09-26)
- **`boundary` on a full clone's root: `true`** (observed, `a_full_clones_root_commit_is_not_shallow` with --no-capture, no `blame.showRoot` set). Confirms the ruling's git-source claim: `boundary` can't tell root from shallow cut.
- **The ruling's "17–25, 27–30" is not the whole-file set.** Whole-file blame of HEAD in a depth-2 clone credits every line but 3 and 26 to the boundary (observed failing assertion). The probe's numbers are likely a `-L` range. Test uses git's own blame as oracle and checks 17–25, 27–30 are included.
- `shallow.rs`: split `read_shallow` (returns `(PathBuf, io::Error)`) out of `shallow_boundaries` so blame and trace share one reader; `trace::shallow` now `pub(crate)`.
- Test helper `shallow_clone` moved from `tests/it/trace.rs` to `tests/it/session.rs` (shared).
- No scout: ticket said none; read `blame.rs`/`shallow.rs` directly.

## Touches
Every `BlameCommit {` literal and `blame(` caller (grep; graft-ui's blame draw from 20c reads `BlameCommit`). Name how
each was checked.
