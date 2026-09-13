# m1-the-graph/36 — Pinned tooltip shows the wrong commit after a re-sort

Status: ready
Build with: **Opus** (high) — state that must survive the ADR-0008 promotion swap
Blocked by: 30c
Leaves runnable: yes — a tooltip pinned with Enter keeps showing the commit it was pinned on, through promotion, refresh and scrolling.

Why: found in 27c. The pinned tooltip (34, ADR-0020) stores a row number. When the promotion swap (ADR-0008) re-sorts history, that row holds a different commit, so the pin silently shows the wrong commit's hash, author and message. **User decision (2026-09-13): a correctness bug, fixed in M1, not carried to the review.**

## Read first
- Ticket 34 "User decisions" and its notes; ADR-0020; ADR-0008 (decision and consequences)
- Ticket 27c result notes (what else the swap re-keys: jump, seek, selection)
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`; scouts for source (the pin state, `promoted()` in `graft/src/history/scroll.rs`)

## Scope
In:
- Test first: pin a tooltip, trigger the promotion swap (and Ctrl+R refresh), assert the pin still shows the same commit id; red before the fix.
- Fix the cause: the pin is keyed by commit id, not row. Decide what happens if the pinned commit is no longer in history (e.g. after refresh drops it): unpin visibly — never show a different commit.
- Check the same class across the swap: selection and hover by row. Fix any other row-keyed state that shows a wrong commit; list what you checked.
- Remove the KNOWN_GAPS row for this bug.

Out: new tooltip content or layout.

## Acceptance
- [ ] Test red before, green after (swap and refresh).
- [ ] Pinned commit gone → pin cleared, no wrong commit shown (test).
- [ ] Other row-keyed state checked and listed.
- [ ] pty capture under `docs/shots/36-*` of a pin surviving a swap if feasible; user judges by eye.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
