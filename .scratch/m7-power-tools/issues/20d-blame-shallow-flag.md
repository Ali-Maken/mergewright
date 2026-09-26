# 20d blame-shallow-flag — Part A of 20's shallow-boundary ruling

Status: ready
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
- [ ] Every test Part A names, red first, each named at tick time.
- [ ] `parse_porcelain` unchanged (state the diff) and its tests green.
- [ ] Gates workspace-wide.

## Touches
Every `BlameCommit {` literal and `blame(` caller (grep; graft-ui's blame draw from 20c reads `BlameCommit`). Name how
each was checked.
