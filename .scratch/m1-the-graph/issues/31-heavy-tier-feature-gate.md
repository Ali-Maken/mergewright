# m1-the-graph/31 — Heavy tests gated by the feature alone (ADR-0018)

Status: done
Build with: **Sonnet** (medium) — mechanical: remove attributes, fix docs, run one check
Blocked by: none. **Runs before 07** (07 is blocked by it).
Leaves runnable: yes — one heavy-tier command runs the whole tier.

Why: ADR-0018 decided heavy tests are `#[cfg(feature = "heavy-tests")]` only, no `#[ignore]`. The code still has `#[ignore]`, so TESTING.md carries an `--include-ignored` footnote. Make the code match the ADR now, not at the end of M1.

## Read first
- `docs/adr/0018-heavy-tests-gated-by-feature-alone.md` (whole file, short)
- TESTING.md "The fast tier" and "The heavy tier" sections only, plus the two `synthetic` commands under "Building them by hand"
- `crates/graft-fixtures/tests/it/heavy.rs` lines 1–12 and each `#[ignore = …]` line
- `crates/graft-fixtures/tests/it/main.rs`

## Scope
In:
- Remove every `#[ignore]` from heavy tests (`grep -rn '#\[ignore' crates` — today 4, all in `tests/it/heavy.rs`). Keep `#[cfg(feature = "heavy-tests")]`.
- Rewrite the module doc in `heavy.rs` (lines 4–8) to say the feature alone gates them, per ADR-0018.
- TESTING.md: delete the "Until m1-the-graph/31 lands" footnote. No other TESTING.md change unless a command there is now wrong.
- ADR-0018 "Consequences": change "Code not changed yet" to past tense, naming this ticket.
- KNOWN_GAPS.md row "The heavy tier takes two commands": mark closed by this ticket.
- Run the fast-tier check and record its output in this ticket.

Out:
- Running the heavy tier (review only, and it takes ~10 minutes). Any measurement (27). Any other test change.

## Signatures
Expose: none. Call: none. Test attributes and docs only.

## Acceptance
- [x] `grep -rn '#\[ignore' crates` finds nothing — output pasted.

```
$ grep -rn '#\[ignore' crates
(no output)
```

- [x] Plain `cargo test` compiles no heavy test: `cargo test -p graft-fixtures --test it -- --list | grep -c '^heavy::'` prints `0` — output pasted. Also `cargo test -p graft-fixtures --features heavy-tests --test it -- --list | grep -c '^heavy::'` prints `4`, proving the filter would catch them. Run these one at a time.

```
$ cargo test -p graft-fixtures --test it -- --list | grep -c '^heavy::'
0

$ cargo test -p graft-fixtures --features heavy-tests --test it -- --list | grep -c '^heavy::'
4
```

- [x] `cargo clippy --all-targets --features heavy-tests -- -D warnings` clean (the heavy code is otherwise unlinted).

```
$ cargo clippy --all-targets --features heavy-tests -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.08s
```

- [x] TESTING.md footnote gone; ADR-0018 and KNOWN_GAPS row updated.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

```
$ cargo fmt --check
(no output, clean)

$ cargo clippy --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s

$ cargo test
... all suites: ok, 0 failed
```

## Size
One context, well under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report.
