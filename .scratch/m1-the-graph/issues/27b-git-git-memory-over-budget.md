# m1-the-graph/27b — git/git without a commit-graph misses the 300 MB budget

Status: ready
Build with: **Opus** (high) — split from 27's remainder; same scope and model
Blocked by: 27
Leaves runnable: yes — git/git stays under 300 MB peak RSS with and without a commit-graph.

Why: 27 measured git/git (85,649 commits, release) at **300.5–305.9 MiB without a commit-graph** (over PROMPT §1.2's 300 MB) and 254–256 MiB with one (too close). Nearly all of it arrives on `G` (+278 MiB no cg, +175 MiB cg). The 100k synthetic fixture passes (158/179 MiB) because its commits are small.

## Read first
- Ticket 27: "Results", "Decisions", "Handoff (what remains)" and "Suspects" — **the suspects are unmeasured**
- PROMPT.md §1.2 (the two budget bullets), §6.3 "Performance budgets"
- DESIGN.md §3.6 (budget and memory paragraphs only); ADR-0008, ADR-0015, ADR-0018
- `MAP.md` of `graft-git`, `graft-graph`, `graft`
- Scouts (Explore, haiku/sonnet) for source beyond a few lines.

## Scope
In:
1. **Measure first:** heap profile (or allocation counting) of `examples/budgets` on a no-cg copy of git/git (`cp -R` the clone from `scripts/clone-git-git.sh --where` into `$CLAUDE_JOB_DIR/tmp` or `target/`, delete `objects/info/commit-graph`). Name what holds the memory with numbers before changing anything. Profile output goes to files; summarise, don't dump into context.
2. **Fix the cause** inside M1 so git/git peaks under 300 MB **with a real margin** both ways; record before/after per suspect confirmed.
3. If possible, make the heavy tier catch this class of regression (e.g. a synthetic fixture option with realistic message/identity sizes), without generating large fixtures outside the heavy tier. If that is its own ticket's worth of work, say so and stop at that seam.
4. Update 27's results table rows for git/git and the KNOWN_GAPS rows 27 touched.

Out:
- Compact `EmittedOrder` index, checkpoint thinning, background advance, 1M claim (→ 30).
- Changing behaviour to meet the budget (e.g. no full `G`, dropping promotion): **stop and ask** through the conductor.

## Signatures
Expose: none public. Keep any seam `pub(crate)`. If a public item changes, update that crate's MAP.md.

## Acceptance
- [ ] Profile finding written in this ticket: top holders with sizes, which of 27's suspects were real.
- [ ] git/git release, 3 runs each, cg and no cg: peak RSS and first paint, before/after table. Both under 300 MB.
- [ ] 100k heavy-tier budgets still green (run once, alone, at the end).
- [ ] No behaviour change without the user's OK.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end. Gotcha from 27: zsh doesn't word-split `$VAR` commands; spell cargo commands out.

## Size
One context, under ~80k tokens. Seam if it grows: (a) profile + finding + first fix, (b) remaining fixes + heavy-tier guard. Stop at the seam with a handoff note rather than compacting.
