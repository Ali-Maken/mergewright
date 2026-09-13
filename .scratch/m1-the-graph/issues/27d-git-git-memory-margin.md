# m1-the-graph/27d — git/git memory: land C and D, get real margin, guard it

Status: ready
Build with: **Opus** (high) — remainder of 27b, same scope and model
Blocked by: 27c
Leaves runnable: yes — git/git peak RSS under 300 MB with real margin, with and without commit-graph; heavy tier catches the regression class.

Why: 27b landed fix B (no cg RSS 305 → 290–294, footprint 204 → 173; cg 255 → 223–226). 6–10 MiB margin isn't enough. C hung on a race that 27c fixes.

## Read first
- Ticket 27b: "Handoff (what remains)" item 2, "Continuation 1" (measurements table, fixes, Next steps 2–5). Measurement script `target/27b/meas.sh`.
- Ticket 27 "Results" table (to update)
- `crates/graft-git/MAP.md`, `crates/graft/MAP.md`

## Scope
**Update from 27c (conductor, 2026-09-13):** C's call site landed in 27c (commits f791d7b, 983184c): no cg peak RSS 250 MiB, footprint 162–164, 3 runs, no hang. Item 1 is now: measure C with a commit-graph (3×) and fill the before/after table from 27c's numbers.

In (27b "Next steps" 2–5):
1. Re-apply C's call site (`walk.next_ids` in `graft::history::topological_order`); measure 3× RSS and footprint, cg and no cg.
2. Wire D (`handle_with_object_cache` for the promotion handle); measure 3×, watch `G` time.
3. A′ only if still short of margin; judge on RSS.
4. Heavy-tier regression guard (27b scope item 3): a synthetic fixture option with realistic message/identity sizes, so the 100k heavy test sees what git/git does. If this is a ticket's worth alone, stop at that seam and say so.
5. Update 27's results table (git/git rows) and the KNOWN_GAPS rows 27/27b touched. Decide `prototypes/heap-profile-27b/Cargo.lock` (untracked): commit or ignore, per how other prototypes do it.

Out: changing the gate measure (RSS vs footprint is with the user; keep `getrusage` RSS). Behaviour changes to meet the budget: stop and ask.

## Acceptance
- [ ] Before/after table per fix, 3 runs, RSS + footprint + `G` time, cg and no cg.
- [ ] git/git no cg peak RSS under 300 MB with a margin you state and justify.
- [ ] Heavy-tier guard asserts, proven to fail by tightening; or a stated seam.
- [ ] Heavy tier once, alone, at the end; gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam: (a) C + D + A′ measured, (b) heavy-tier guard + tables.
