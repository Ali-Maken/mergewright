# m0-foundations/12 — M0 wrap: README, CHANGELOG, gaps sweep

Status: todo
Build with: Sonnet
Blocked by: 02, 11

Why: PROMPT.md §8 — every milestone ends with gates green, README + CHANGELOG updated, short demo steps, KNOWN_GAPS.md current. The reviewer checks this next.

## Read first
- PROMPT.md §8 preamble (the end-of-milestone list)
- KNOWN_GAPS.md as it stands
- `git log --oneline` for what M0 actually shipped

## Goal
Close out M0 so the reviewer and the user can verify it in minutes.

## Scope
In:
- README.md: what exists after M0 (open a repo, status screen, tier detection, clean quit), how to build/run, and the demo steps: `cargo run -- <repo>`, what you should see, `q` to quit, the not-a-repo case. Codename note per CLAUDE.md (no public naming).
- CHANGELOG.md: an M0 entry (Keep-a-Changelog style, no version number — the name and versioning land in M8).
- KNOWN_GAPS.md sweep: confirm every M0 stub/deferral has a row with an expiry (probe typeahead + resize re-probe should already be there as M1 gaps; add anything tickets 01–11 left in Comments). Remove nothing that isn't actually closed.
- Run all three gates across the workspace one final time and record the timings in this ticket's Comments (fast tier must be under 3 min — PROMPT.md §7.1).

Out:
- TESTING.md (its manual tier starts at M1). Ticking PROGRESS.md milestone boxes (conductor's job). Any code changes beyond doc fixes — if a gate fails here, that's a reopened ticket, not a wrap fix.

## Acceptance
- [ ] README demo steps work verbatim on a fresh clone (walk them yourself).
- [ ] CHANGELOG lists M0's user-visible behaviour, not internals.
- [ ] KNOWN_GAPS.md has no stale M0 rows and no missing ones.
- [ ] All gates green, timings recorded.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
