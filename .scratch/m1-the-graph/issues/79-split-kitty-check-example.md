# m1-the-graph/79 — Review round 2 m1: split examples/kitty_check.rs under the 400-line rule

Status: ready
Build with: **Sonnet** (medium) — mechanical move, like 76b.
Blocked by: 78

From docs/reviews/m1-the-graph-round-2.md "m1".

## Read first
- docs/reviews/m1-the-graph-round-2.md "m1" only; PROMPT.md §6.2.
- `crates/graft/examples/kitty_check.rs`: find cut points with wc/grep, don't read it whole.

## Scope
In: split the encoder driver from terminal/reply handling. Use an `examples/kitty_check/main.rs` directory layout (cargo's multi-file example form), so the command `cargo run --example kitty_check` is unchanged. Every file must be ≤400 lines. Moves only.
Out: behaviour changes; the kitty module itself.

## Acceptance
- [ ] `cargo run --example kitty_check -- --help` (or its no-terminal path) still builds and runs; gates green.
- [ ] Report: before/after line counts.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens.
