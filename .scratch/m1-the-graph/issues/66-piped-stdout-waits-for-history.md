# m1-the-graph/66 — Review M4c: piped stdout prints before the repository opens

Status: done
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
- [x] Test red before, green after; an open-failure case also tested (exits with its error, never hangs).
- [x] KNOWN_GAPS row closed.
- [x] Report: cause or symptom; what it costs the user.

## Report

Cause, not symptom. `headless_settled` used `Option::is_none_or`, which
treats `history: None` as already settled — so the headless loop's `while`
condition was false on its very first check, before a single wake was
processed, and printed whatever frame `AppState::default()` draws. Fixed to
`is_some_and`, so `None` is never settled; only `history.is_some()` with a
non-`Streaming` phase settles. The open-failure path was already correct
(`Quit::Fatal` short-circuits the loop in the body, independent of
`headless_settled`) — confirmed by `not_a_repo.rs`'s existing pass and a new
piped-stdout case.

Cost to the user: `graft <repo> | cat` (or any redirect/pipe, e.g. scripting,
CI, `graft repo > out.txt`) could silently print a near-empty "opening…"
frame and exit 0 — a script piping Graft's output would see no commit rows
and no error, and have no way to tell the walk simply hadn't started yet
from "the repository has no commits."

Verified: unit tests in `event_loop::tests` (`headless_is_not_settled_before_history_exists`,
`..._while_history_is_streaming`, `..._settled_once_history_completes`,
`..._settled_when_the_walk_fails`) red before the fix, green after.
Integration: `piped_stdout_never_contains_an_escape_byte` now also asserts
the tip commit's subject (`Commit 200`) is present; new
`a_repository_that_fails_to_open_exits_with_its_error_never_hangs` confirms
exit 1, the error on stderr, and an empty stdout — via `Command::output`,
which blocks until the child exits, so a hang would hang the test itself,
not just Graft. Full `cargo test`, `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, and
`cargo clippy --all-targets --features heavy-tests -- -D warnings` all pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
