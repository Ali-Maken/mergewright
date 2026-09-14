# m1-the-graph/66 — Review M4c: piped stdout prints before the repository opens

Status: ready
Build with: **Sonnet** (medium) — one predicate and its test.
Blocked by: none

From the M1 review round 1, "M4" piped-stdout bullet. Closes KNOWN_GAPS.md row ~181.

## Read first
- docs/reviews/m1-the-graph.md "M4" only.
- `crates/graft/MAP.md`; `crates/graft/src/event_loop/mod.rs` `headless_settled` (~420–450); `crates/graft/tests/it/piped_stdout.rs`.

## Scope
In: a failing test showing `graft <repo> | cat` can emit a frame while `history` is `None`, made deterministic, with no sleeps. Then the fix: headless counts as settled only once `history.is_some()`, or once the open failed with an error that is printed. A repo that fails to open must still exit with its error, never hang. Close the row.
Out: other headless behaviour; file splits.

## Acceptance
- [ ] Test red before, green after; an open-failure case also tested (exits with its error, never hangs).
- [ ] KNOWN_GAPS row closed.
- [ ] Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
