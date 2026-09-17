# Heavy tests are gated by the `heavy-tests` feature alone

Decision: a heavy test is `#[cfg(feature = "heavy-tests")]` and **not**
`#[ignore]`. The whole heavy tier is one command:

```sh
cargo test --features heavy-tests -- --test-threads=2
```

This amends PROMPT.md §7.1, which named
`cargo test --features heavy-tests -- --ignored --test-threads=2` and told
heavy tests to be both ignored and feature-gated.

## Why

- **The documented command skipped half the tier.** `--ignored` runs *only*
  ignored tests, so §7.1's command never reached the property tests, and
  "proptest at 1,000 cases" silently did not happen. M1/02 found it;
  TESTING.md worked around it with two commands.
- **A gate you have to remember to arm is not a gate** (same reasoning as
  ADR-0017). Two commands means a review can run one and believe it ran the
  tier.
- **The second guard bought little.** `#[ignore]` only protected against
  turning the feature on by accident. Nothing turns it on: no crate lists it
  in `default`, and nothing in the repo runs `--all-features`.

## Consequences

- One command runs every heavy test and every proptest at 1,000 cases.
- A heavy test can be run by name with the feature and a filter; no
  `--ignored` / `--include-ignored` needed.
- **Lost:** a stray `--features heavy-tests` (or a future `--all-features`)
  now starts the ~10-minute tier. Keep `heavy-tests` out of every `default`
  list and never add `--all-features` to a gate.
- The fast-tier proof changes: heavy tests are not compiled without the
  feature, so `cargo test --test it -- --list` shows no `heavy::` test.
- **Code changed.** m1-the-graph/31 removed `#[ignore]` from the four
  generator tests in `crates/graft-fixtures/tests/it/heavy.rs`; the
  `heavy-tests` feature is now the only gate.

## Amendment (2026-09-17, m1-the-graph/80)

The two `frames::` tests on the ~300 MiB 1M-commit fixture
(`crates/graft/tests/it/frames.rs`, m1-the-graph/64) had been gated on an
environment variable instead of a feature: unset, they **failed**
rather than being skipped, so the documented release perf command
(`cargo test --release --features heavy-tests …`) reported "2 failed" on
every run (round-2 review Minor m4). A test gates on a build feature, never
on the environment — the same reasoning as the decision above, applied to
the one case it didn't yet cover.

- **`heavy-1m` is a second feature**, on the `graft` crate, that implies
  `heavy-tests` (`heavy-1m = ["heavy-tests"]`). It is never in any
  `default` list.
- The two 1M tests are `#[cfg(feature = "heavy-1m")]`. Under plain
  `heavy-tests` they do not exist to `cargo test`, so the standard heavy-tier
  and release-perf commands need no `--skip` to stay green. The old
  environment-variable gate and its read are removed.
- Commands:
  ```sh
  cargo test --features heavy-tests -- --test-threads=2                                    # heavy tier, no 1M
  cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture   # release perf, no 1M
  cargo test --release -p graft --features heavy-1m --test it -- scrolling_the_1m_commit_graph_fixture --test-threads=1 --nocapture   # adds the 1M frames tests
  ```
