# m1-the-graph/30b — A one-row scroll in the pixel tier writes ~336 KB

Status: ready
Build with: **Opus** (high) — Kitty protocol, split from 30
Blocked by: 30
Leaves runnable: yes — a one-row scroll frame places existing images and transmits at most one new strip.

Why: 30's heavy `frames::` tests measured pixel frames writing 0.3–0.9 MB and ~84 Kitty commands for a one-row scroll (100k fixture). Expected by DESIGN §3.8 / tickets 24c, 25: place existing ids, ≤1 new strip. Pixel p99 input-to-frame is 0.5–1 s debug. Cause unknown.

## Read first
- Ticket 30 "Handoff (what remains)" item 1 and Gotchas; its results table
- DESIGN.md §3.8 (strip cache and transmit paragraphs only); ADR for transmit-once strips if one exists (grep docs/adr for "strip")
- `crates/graft-term/MAP.md`, `crates/graft-graph/MAP.md`, `crates/graft/MAP.md`; scouts for source

## Scope
In:
- Count per frame: transmits (`a=t`) vs placements (`a=p`) vs deletes, bytes each, from the `frames.rs` `Sink`. Write the finding (numbers) in this ticket before changing code.
- Fix the cause. Add a fast-tier test that pins it (e.g. a one-row scroll over cached strips emits 0 transmits and N placements).
- Re-run the pixel `frames::` heavy tests alone; record bytes/frame and input-to-frame (debug and release) in 30's table.

Out: Unicode chunk spikes, release-vs-debug assertion policy, tightening proofs (→ 30c). 50k refs (→ 30d).

## Acceptance
- [ ] Finding with numbers; cause named.
- [ ] New fast test red before fix, green after.
- [ ] Bytes per one-row scroll frame and p99 recorded, before/after.
- [ ] Capture under `docs/shots/30b-*` if what's drawn changes; user judges by eye.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; `frames::` heavy tests alone, output to a file.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
