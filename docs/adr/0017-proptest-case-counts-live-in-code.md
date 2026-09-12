# Proptest case counts live in code, not the environment

Decision: the proptest case count is **64 in the fast tier and 1,000 in the
heavy tier, set in `graft_fixtures::proptest_cases` and nowhere else**.
`PROPTEST_CASES` is no longer consulted, and it has been removed from
`.claude/settings.json`. `cargo test` runs the same test whether the user
runs it, a helper agent runs it, or CI does.

This overrides PROMPT.md §7.1's "(the `PROPTEST_CASES` variable can change
this)", which is amended to match.

## Why

The old design let the environment win, so a person could hammer one
property while chasing a failure. That shortcut cost more than it paid:

- **The heavy tier quietly shrank.** `PROPTEST_CASES=64` was set in this
  repository's own `.claude/settings.json`, so every agent-run `cargo test
  --features heavy-tests` ran 64 cases where the tier promised 1,000 — the
  weaker run announced nothing. `KNOWN_GAPS.md` and `TESTING.md` both
  described this as a hazard to remember. A gate you have to remember to
  arm is not a gate.
- **It was misdiagnosed for a day.** M1/02 reported the variable as coming
  from the user's shell, "from outside the repository". It was in the repo
  all along. An environment-dependent test is hard to reason about even for
  the people who wrote it.
- **Reproducibility is the point of a property test.** "1,000 cases" has to
  mean 1,000 cases, or a milestone review's evidence is worth nothing.

## What you give up, and what replaces it

You can no longer widen a run from the shell. To hammer one property, edit
its `ProptestConfig` for that run:

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
```

That is a one-line, visible, revertible change, and it cannot leak into
anyone else's run or into a review.

`PROPTEST_CASES` is proptest's own variable, so proptest still reads it for
any `ProptestConfig::default()` this workspace does not construct. Every
proptest here goes through `proptest_cases`, so today that is none of them.

## Follow-up

`proptest_cases` still returns `Option<u32>`, always `Some`, so that the
three call sites keep compiling unchanged while M1/06 is in flight. The
signature becomes a plain `u32` in the sweep that follows; tracked in
`KNOWN_GAPS.md`.
