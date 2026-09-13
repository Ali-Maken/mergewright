# m1-the-graph/30d — 50k refs: time the real ref read

Status: ready
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
- [ ] Fixture in heavy tier only; numbers recorded; assertion proven.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier alone.

## Size
Under ~80k tokens.
