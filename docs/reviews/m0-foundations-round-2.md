# M0 Foundations — review round 2

Scope: verify the six round-1 findings (`docs/reviews/m0-foundations.md`) as
fixed in 1691ea7, 33cb445, cc60d4f, and anything those fixes broke or
introduced. Not a fresh full review of M0. Reviewer ran all gates; builder
this round verified by tests and code reading only — see "Unverified".

## Verdict

All six round-1 findings genuinely resolved. Nothing broken by the fixes.
New findings: **0 Critical, 0 Major, 3 Minor** — all documentation/diagnostic
accuracy, none blocks M0.

## Round-1 findings, one by one

### Major 1 — lints unenforced, unsafe un-ADR'd: **resolved**

- `Cargo.toml` `[workspace.lints]`: `unsafe_code = "deny"`,
  `unwrap_used`/`expect_used = "deny"`, `missing_docs = "warn"`, pedantic at
  priority -1, `cognitive_complexity` + `too_many_lines` on. All seven crate
  manifests carry `[lints] workspace = true`. `clippy.toml` pins
  `too-many-lines-threshold = 60`, `cognitive-complexity-threshold = 15`,
  `allow-unwrap-in-tests`. `rustfmt.toml` pins edition/width/tabs. Verified
  enforced: clippy is clean with `-D warnings` against all targets.
- `#![forbid(unsafe_code)]` in six crates; `graft-term/src/lib.rs:16` has
  `#![deny(unsafe_code)]` with a reasoning comment. ADR-0013 written.
- The deny-vs-forbid reasoning **holds**: `forbid` cannot be overridden by an
  inner `#[allow]` (compile error), so site-scoped allowances require `deny`.
  Allowances are on two small functions in `detect/tty.rs`
  (`window_pixels`, `wait_for_input`), each with `reason = "…"` and a
  `SAFETY:` argument. Scoping is narrow; a third `unsafe` anywhere in the
  crate still fails the build.
- Round 1 counted **three** unsafe sites (old tty.rs:83, 85, 117); ADR-0013
  covers **two**. Nothing was quietly dropped: the third was `mem::zeroed()`
  initialising the `winsize`, and the fix replaced it with field-by-field
  initialisation (verified against `git show 3592226:…/tty.rs`). One unsafe
  site was eliminated, not hidden.

### Major 2 — "detected" shown when nothing was detected: **resolved**

- `TierSource` (graft-ui `status_screen/mod.rs`) now has five states:
  Forced / Detected / Silent / ProbeFailed / NotProbed, fed from
  `ProbeOutcome` by `tier_source()` in `crates/graft/src/session.rs`.
  Forced wins over everything; Answered→"detected";
  TimedOut→"assumed (terminal silent)"; Failed→"assumed (probe failed)"
  plus the sanitised error pushed as a startup note; Skipped→"assumed
  (terminal not asked)".
- Not just renamed: the wording is derived from what actually happened, and
  `a_tier_nothing_measured_never_reads_as_detected` asserts the word
  "detected" cannot appear for any of the three nothing-measured states.
  Snapshot `a_terminal_that_said_nothing_about_itself.snap` pins
  `unicode, assumed (terminal silent), cell 8x16 (assumed)`. Seven unit
  tests in `session.rs` cover all states end-to-end including
  forced-over-timeout and the failed-probe note.

### Major 3 — `detect/mod.rs` at 573 lines: **resolved, seam is real**

- Split: `environment.rs` (160), `probe.rs` (180), `reply.rs` (326 + tests),
  `tmux.rs` (109), `tty.rs` (199), `mod.rs` (365 incl. tests). Not cosmetic:
  each file owns one concern (env reading / query bytes / reply parsing /
  passthrough round / real I/O / the decision), `tty.rs` is the only module
  touching the real terminal, and everything above it tests against the
  `TerminalHost` fake. `ARCHITECTURE.md:39-44` documents the map and matches
  the code.

### Minor 1 — `tmux -V` unbounded wait: **resolved**

`tty.rs:29` `TMUX_VERSION_DEADLINE = 2s`, 2 ms poll, kill+reap on deadline,
`let _ =` on the kill carries a why-it's-safe comment (tty.rs:158-162),
mirroring `graft-git/src/version.rs`'s shape. See new Minor 3 for a
follow-on documentation gap this created.

### Minor 2 — tmux second-round failure downgrades fenced answers: **resolved**

`detect/mod.rs:174-180`: `kitty_keyboard` and `synchronized_output` are now
judged against `first_round`, not the merged outcome, with a comment saying
why. New test `a_failed_tmux_reprobe_does_not_unanswer_the_first_round`
asserts `Supported::No` survives, probe reads Failed, graphics stay Silent,
tier stays Unicode.

### Minor 3 — hand-written Error impls: **resolved**

33cb445 converts `Error`, `OpenError`, `HeadError`, `VersionError` to
`thiserror` 2.0. Messages verified byte-for-byte against the old `Display`
impls (including `VersionError::Timeout` via the `unstoppable()` helper);
`#[source]` attributes match the old `source()` chains. `RefNameRule`,
`ConfigWarning`, `JobFailure` stay hand-written with stated reasons — fine.

### The six DESIGN.md divergences: **spec now matches code**

ADR-0014 records all six rulings. Checked each edit against the code:
§4.2's table row now sends protocol-less tmux to Unicode (matches
`tier()`), §4.3 pixel-fails-closed-in-tmux matches `reaches_the_terminal`,
§4.4 states the per-round budget, §8.2 describes the free function, the
whole-`Cf` rule and the `sanitize_line`/path variants. Each edit carries the
ADR-0014 marker. One residual inaccuracy in §4.4's worst-case numbers — new
Minor 3 below.

## New findings

### Critical

None.

### Major

None.

### Minor

1. **ADR-0013 justifies the right decision with a false claim** —
   `docs/adr/0013-two-unsafe-libc-calls-in-graft-term.md:101-103`. "an
   `unsafe` block cannot itself carry the allowance … so a function is the
   smallest unit the language offers here" is wrong: a statement-level
   `#[allow(unsafe_code)]` on the `let … = unsafe { … };` statement compiles
   under `#![deny(unsafe_code)]` (verified empirically with rustc 2024
   edition). Why it matters: an ADR is the document a future reader trusts;
   a false language claim in it invites either cargo-culting or distrust of
   the rest. The fn-level choice itself is fine — both functions are tiny —
   so the fix is one sentence: justify fn-level scoping as the chosen
   granularity (whole-function SAFETY argument), not as the only one the
   language offers.
2. **Contradictory diagnostics in one tmux timing edge** —
   `crates/graft-term/src/detect/mod.rs:160-163` with `:179-180`. If round 1
   times out but the tmux re-probe's fenced round answers, `outcome` stays
   `TimedOut` (it only ever replaces `first_round` on *failure*), while
   `tier()` can still select a pixel tier from the round-2 graphics answers.
   The status screen would then read "pixel, **assumed (terminal silent)**"
   next to `TmuxStatus::Passthrough` — the tier was measured, the label says
   it wasn't. Direction is safe (never claims "detected" for silence) and
   the timing is unlikely (outer terminal silent unfenced, then answering
   through tmux), but the two fields disagree about the same event. Fix:
   fold the second round into `outcome` — e.g. Answered if either round
   answered — or derive the tier-source from the round that decided the
   tier.
3. **§4.4's worst-case startup omits the new `tmux -V` deadline** —
   `DESIGN.md` §4.4 / `docs/adr/0014-m0-design-amendments.md:86` say worst
   case "400 ms locally, 1 s over SSH". The Minor-1 fix added a 2 s deadline
   to `tmux -V` (`tty.rs:29`), which runs when `$TMUX` is set and
   `TERM_PROGRAM` ≠ "tmux". True worst case inside tmux with a wedged tmux
   binary is ~2.4 s locally / ~3 s over SSH. Not a code bug — the bound is
   the point — but the fix round re-edited §4.4 and left the number stale.
   Fix: one sentence in §4.4 (and ADR-0014 §6) adding the `tmux -V` bound,
   or shrink `TMUX_VERSION_DEADLINE` to probe-budget scale, since tmux is a
   local binary and 2 s is generous.

   Related, noted not graded: `tty.rs:151-154`'s `try_wait()` `Err` path
   returns `None` without attempting a kill, leaving the child unreaped.
   `try_wait` on a just-spawned child essentially never errors, and the
   comment says why `None` is right; a `kill` before the `return` would cost
   one line.

## Gates and measurements (reviewer-run, one job at a time)

| Check | Result |
|---|---|
| `cargo fmt --check` | clean, ~0.08 s |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0 confirmed via `PIPESTATUS`) |
| `cargo test` | **276 passed, 0 failed, 0 ignored**; 0.59 s wall |
| Peak memory (test run) | ~92 MB max RSS (96,239,616 bytes) |
| Heavy tier | does not exist — `--features heavy-tests` matches no package; documented in `KNOWN_GAPS.md`, expiry M1 |
| Fuzzing / benchmarks | none exist yet (per plan; first due M1) |
| `target/` size | 1.3 GB |

Slowest suites (per-test timing needs nightly `--report-time`): graft-git
0.21 s (22 tests, spawns the real `git` binary), graft-term unit 0.03 s
(128), graft app 0.03 s (69), graft-ui 0.01 s (37), everything else 0.00 s.
Laptop-safe: no suite over a quarter second, no fixtures generated, no
hangs, no ignored tests silently skipped.

## Unverified (builder used tests + code reading only)

- **"detected" has still never been rendered anywhere real.** The pty test
  harness always yields the silent fallback, so the one wording that claims
  a measurement is the one wording no eye has seen. Needs one run in
  iTerm2/Ghostty at the checkpoint: status screen should read "detected"
  and show a measured cell size.
- The "assumed (probe failed)" wording and the probe-failure startup note
  have never appeared on a real screen (only in unit tests).
- The tmux passthrough round has never run against a real tmux; every
  answer it has ever parsed came from `FakeHost`.
- The `KNOWN_GAPS.md` rows marked for M0 review (real-emulator probe
  answers, human-seen screens, panic-hook behaviour under a real panic)
  remain open for the user's checkpoint, and `KNOWN_GAPS.md` says so
  honestly.
