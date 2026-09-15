# m1-the-graph/79 — Review round 2 m1: split examples/kitty_check.rs under the 400-line rule

Status: done
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
- [x] `cargo run --example kitty_check -- --help` (or its no-terminal path) still builds and runs; gates green.
- [x] Report: before/after line counts.

## Result
Moved `crates/graft/examples/kitty_check.rs` (469 lines) to
`crates/graft/examples/kitty_check/{main,driver,terminal}.rs` (cargo
multi-file example form; `cargo run --example kitty_check` unchanged).

Before: 469 lines, 1 file.
After: 493 lines total (doc comments/imports duplicated across files),
all ≤400: main.rs 61, driver.rs 275, terminal.rs 157.

- `main.rs`: doc comment, `main()`, `mod driver; mod terminal;`.
- `driver.rs`: encoder driver — `check`, `f3_round_trip`, `controls`,
  `erase_rows`, `frame`, `stripes`, `report`.
- `terminal.rs`: terminal/reply handling — `Step`, `run_step`,
  `da1_answer`, `spawn_stdin`, `decode`, `find`, `sequence_end`.

Moves only, no behaviour change. Verified: `cargo build --example
kitty_check` and `cargo run --example kitty_check` (no args) both
succeed, run with the same usage error as before the split. All four
gates pass (fmt, clippy, clippy --features heavy-tests, test).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens.
