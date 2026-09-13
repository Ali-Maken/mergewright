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
- **Code not changed yet.** The four generator tests in
  `crates/graft-fixtures/tests/it/heavy.rs` still carry `#[ignore]`.
  m1-the-graph/27 removes them as its first step. Until then, append
  `--include-ignored` to the heavy command.
