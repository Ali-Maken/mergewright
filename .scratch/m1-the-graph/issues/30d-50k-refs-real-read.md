# m1-the-graph/30d — 50k refs: time the real ref read

Status: partial — fixture, timings, two fixes, guards done; graph first paint with 50k refs misses 150 ms (cause not found), split out
Build with: **Opus** (high) — split from 30, same model
Blocked by: 30
Leaves runnable: yes — heavy tier times `refs::read` on a generated repo with 50k refs.

Why: 30 measured only the pure `badges()` (1.35 s debug / 0.46 s release) and lookup (22 ns). `refs::read` does one `RepoHandle::upstream` per branch and was never timed on a real 50k-ref repo.

## Read first
- Ticket 30 "Handoff" item 5; PROMPT.md §1.2 / §6.3 lines naming 50k refs
- `crates/graft-git/MAP.md`, `crates/graft-fixtures/MAP.md`; scouts for source

## Scope
In:
- Heavy-tier fixture: 50k refs via packed-refs + `[branch]` config sections (generated only in the heavy tier).
- Time open → refs read → first paint with those refs, debug and release; record in 30's table.
- If over budget: find the cause (e.g. per-branch `upstream` lookups) and fix without behaviour change; otherwise record. Assertion proven by tightening.

Out: frame budgets (30b, 30c).

## Acceptance
- [x] Fixture in heavy tier only; numbers recorded (ticket 30 table); assertion proven (guards at 500 ms fail: 0.91 s read, 1.04 s paint; reverted to 1.5 s).
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier alone.

## Size
Under ~80k tokens.

## Handoff (2026-09-13, builder-opus)

Done: `Scale::with_refs(n)` (graft-fixtures, heavy tier only): n/2 packed branches `refs/heads/many/NNNNNN`, `[branch]` sections, n/2 `refs/remotes/origin/many/*`, 1 in 100 one commit ahead; fixture test in `crates/graft-fixtures/tests/it/heavy.rs`. `crates/graft/tests/it/many_refs.rs`: `refs::read` and open → graph+badges, debug guards 1.5 s. `heavy::budgets_hold_on_the_100k_fixture_with_50k_refs` (150 ms first paint, 300 MB).
Fixes: `refs::badges` merged-upstream `Vec` → `HashSet` (quadratic); `ahead_behind` returns before opening the commit graph when tips are equal. `refs::read` 2.94 s → 0.91 s debug, 165 ms release.

Still failing (a measurement, like 30's frames tests): `heavy::budgets_hold_on_the_100k_fixture_with_50k_refs` — graph first paint 800 ms debug / 160 ms release, vs 63 / 31 ms without the refs. The graph does not wait for the refs job, so something on the open/history path scales with refs.

Next step: time the jobs dispatched by `budget_run::open` on this fixture (OpenRepository, ReadHistory's tips via `commit_tips`/`peeled_references`, `Branches::resolve`) with temporary prints, as done here for `refs::read` (peel 424 ms debug alone). Suspect every ref peeled with an object lookup; packed-refs already carries `^` peeled lines. Also check whether 4 pool threads contend (refs job + history job both peel).

Gotchas: fixture builds in ~25 s; run heavy tests alone with `--test-threads=1`, output to a file.
