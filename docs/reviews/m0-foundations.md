# M0 Foundations — milestone review

Reviewed: 2026-09-12. Range: `662f108..ceb0712` (12 tickets, `.scratch/m0-foundations/issues/`).
Reviewer read every non-test source file in the range, both review axes (spec + standards) in one pass.

**Verdict: passes M0's bar with 0 Critical, 3 Major, 3 Minor findings.** The Majors are
fixable in under a day and none blocks the checkpoint test ("opens a repo; quitting leaves
the terminal clean") — but Major 2 should be fixed *before* the user runs the checkpoint,
because the checkpoint has them reading the exact line that lies.

## Gates and measurements

| Check | Result |
|---|---|
| `cargo fmt --check` | clean, 0.08 s |
| `cargo clippy --all-targets -- -D warnings` | clean, 0.14 s warm (~27 s cold per m0/12) |
| `cargo test` | **268 passed, 0 failed**, 0.58 s warm wall |
| Heavy tier | **does not exist** — `--features heavy-tests` errors: no such feature in any crate. Documented (KNOWN_GAPS "Only the fast test tier exists", expiry M1), so honest, but this review could run nothing beyond the fast tier |
| Fuzzing / benchmarks | none exist (same KNOWN_GAPS row) |
| Peak memory (test run) | 93,945,856 bytes ≈ 90 MB RSS — far under §7.1's 2 GB |
| `target/` | 771 MB — far under the 20 GB cap |
| Slowest suites | graft-git 0.21 s (spawns real `git`), graft app lib 0.04 s, graft-term 0.03 s, graft-ui 0.01 s, `tests/it` 0.01 s. Per-test times unavailable: `--report-time` needs nightly, and installing a toolchain to time a 0.58 s suite isn't justified |

Laptop-safety: fast tier is exemplary (268 tests, sub-second, no sleeps except one
2 ms-poll deadline loop in `version.rs` with a written rationale, no fixture generation).

## Critical

None found.

- No data-loss risk: M0 performs no repository writes anywhere (grepped; `graft-ops` is empty).
- No git on the UI thread: both git touches (`gix::discover`, `git --version`) run as jobs
  (`crates/graft/src/jobs/mod.rs:429-436`); `git --version` has a 10 s kill deadline.
- No swallowed errors: every `let _ =` / `unwrap_or(())` sits on a last-resort stderr write
  with a comment saying why it's safe (`terminal.rs:316`, `crash.rs:156`, `main.rs:75`);
  `unwrap_or_default()` at `detect/mod.rs:199,216` is an absent-env-var default, not an error.
- `unwrap()`/`expect()`: all hits inside `#[cfg(test)]`; denied by workspace lints otherwise.
- Sanitiser is strong: whole `Cf` category + C0/C1/DEL + U+2028/29, placeholder-not-delete,
  idempotent, single-line variants for rows and paths, `SanitizedText` constructible only in
  `graft-term::sanitize`. `view.rs` sanitises every repo-derived string, including "safe" hex.
- Terminal always restored: signal → `Wake::Shutdown` → normal unwind → `Drop`; re-raise in
  `main.rs` keeps `$?` honest; both-restores-failed path still reports. Verified by the
  builders via termios byte-comparison under a real `pty.fork` (m0/03, m0/11, m0/12) — the
  strongest evidence available without a human, and the checkpoint covers the rest.

## Major

### 1. PROMPT §6.2's "Enforced, not hoped for" is not enforced — and the gap is not in KNOWN_GAPS

- **Where:** `Cargo.toml:22-24` (workspace lints = only `unwrap_used`/`expect_used`),
  `clippy.toml` (only the two test allowances), no `rustfmt.toml` anywhere, no
  `#![forbid(unsafe_code)]` in any of the 7 crates.
- **What's wrong:** §6.2 requires, verbatim: clippy `pedantic` on at workspace level,
  `too_many_lines`/`cognitive_complexity` thresholds in `clippy.toml`, `rustfmt.toml`,
  `missing_docs` on, and `#![forbid(unsafe_code)]` in every crate with any exception ADR'd.
  None of that exists. Worse, `crates/graft-term/src/detect/tty.rs:83,85,117` uses `unsafe`
  three times (libc `ioctl(TIOCGWINSZ)`, `libc::poll`) with SAFETY comments but **no ADR**,
  and no `KNOWN_GAPS.md` row records any of this — the one honesty miss in an otherwise
  scrupulously honest gap ledger.
- **Why it matters:** the code today is well-documented and would probably survive these
  lints — but that's "hoped for", the exact failure mode §6.2 names. The 573-line
  `detect/mod.rs` (Major 3) is what un-enforced thresholds produce by M2. And unsanctioned
  `unsafe` in the crate that parses hostile terminal bytes is precisely where the ADR rule
  earns its keep.
- **Fix:** one small ticket: add `[workspace.lints.rust] missing_docs = "warn"` +
  `unsafe_code = "deny"`, clippy `pedantic = "warn"` (allow noisy lints individually, with
  reasons), thresholds in `clippy.toml`, a `rustfmt.toml` (defaults are fine — the point is
  the file pins them), `#![forbid(unsafe_code)]` in six crates, and either an ADR blessing
  `tty.rs`'s two libc calls (recommended: they're the standard TIOCGWINSZ/poll idioms and
  well-commented) or a safe replacement (`rustix`). Then fix what the lints flag.

### 2. The status screen says `detected` when the probe timed out — and the M0 checkpoint reads that line

- **Where:** `crates/graft-ui/src/status_screen/mod.rs:212-217` (`rendering()` prints
  `"detected"` whenever `!tier_forced`); root cause `crates/graft/src/session.rs:56-62`
  (`RenderingView` gets only `tier_forced: forced_tier.is_some()` — the probe's
  `ProbeOutcome` never reaches the view).
- **What's wrong:** in m0/12's own pty run the probe reported `TimedOut`, every query
  `Silent` — and the screen printed `Rendering  unicode, detected`. That is CLAUDE.md's
  status-field rule inverted: a field reading as a confident measurement when nothing was
  measured. Cell size and theme carry honest provenance (`assumed`, `terminal background
  unknown`); the tier is the one field that doesn't.
- **Why it matters:** KNOWN_GAPS logs it with expiry M1, but item 0 of "Open for the M0
  checkpoint" has the user running Graft in Ghostty, iTerm2, Zed and VS Code specifically to
  see what detection concluded. Every terminal where the probe fails will *claim* detection
  succeeded, poisoning the checkpoint's evidence — the first real-emulator evidence this
  project will have.
- **Fix:** small and contained: pass the probe outcome through `Session::settle` into
  `RenderingView` as a three-state source (`forced` / `detected` / `fallback (probe timed
  out)` etc., mirroring `CellSizeSource`), and print it in `rendering()`. Do it before the
  checkpoint; re-dispute the M1 expiry if not.

### 3. `detect/mod.rs` is 573 non-test lines against the ~400-line rule

- **Where:** `crates/graft-term/src/detect/mod.rs` (575 lines to the `#[cfg(test)]` marker
  at 574; tests live in `tests.rs`).
- **What's wrong:** §6.2 sets files at ~400 lines and names oversized files as Major review
  findings. Every other file in the workspace is under the line once inline tests are
  discounted (checked: next-largest non-test bodies are `reply.rs` 325, `crash.rs` 312,
  `event_loop/mod.rs` 303).
- **Why it matters:** this is the file that decides every capability, and it already carries
  four of the six spec divergences. M1 adds tier re-query on focus; it only grows.
- **Fix:** it splits along seams that already exist: `environment.rs` (the env-hint struct,
  ~lines 130-230) and `decision.rs` (tier/cell-size/theme/colour-depth tables, ~430-575),
  leaving orchestration in `mod.rs`. Mechanical; no behaviour change; do it while the file
  is young.

## Minor

### 1. `tmux -V` runs with no deadline on the startup path

- **Where:** `crates/graft-term/src/detect/tty.rs:99-103` (`tmux_version()` uses
  `Command::output()` — unbounded), called from `detect/mod.rs:401` during detection,
  before the first frame.
- **What's wrong:** the same startup-hang argument that earned `git --version` a 10 s kill
  deadline (`graft-git/src/version.rs:23`, "a `git` wedged on a stale network mount must
  [not leave] a window that never appears") applies to `tmux` on a shadowed or networked
  `PATH`. The 200 ms probe budget does not cover this call.
- **Why it matters:** low likelihood (you're inside a live tmux), unbounded cost — a hang
  before any UI exists, with nothing to kill it.
- **Fix:** reuse `version.rs`'s bounded-wait shape (a shared helper earns its second use
  here), or a short deadline of its own; 2 s is generous for `tmux -V`.

### 2. A tmux second-round failure downgrades fenced first-round answers

- **Where:** `crates/graft-term/src/detect/mod.rs:272-276` (a `Failed` second round
  replaces the whole outcome) + `:529` (`answered()` maps non-`Answered` + no reply to
  `Silent`).
- **What's wrong:** if round one is fenced (so silence on `CSI ? u` / DECRQM legitimately
  means **No**) and only the tmux passthrough re-probe fails, `kitty_keyboard` and
  `synchronized_output` flip from `No` to `Silent` — the second round never re-asked those
  queries, so its failure says nothing about them.
- **Why it matters:** diagnostics-only today (nothing branches on `No` vs `Silent`), but the
  diagnostics view is the tool for exactly the terminal where this happens.
- **Fix:** track the two rounds' outcomes separately, or compute `Supported` for the
  round-one queries before the replacement at `:275`.

### 3. No `thiserror` — every library error enum is hand-rolled

- **Where:** workspace-wide; no crate depends on `thiserror` (checked all `Cargo.toml`s).
  E.g. `graft-git/src/version.rs:206-263`, `repo.rs`, `cli.rs`, `config/mod.rs`.
- **What's wrong:** §6.2: "Each library crate has its own error enum (`thiserror`)". The
  *intent* — typed, per-crate, user-readable errors with correct `source()` chains — is
  fully met; the named mechanism isn't, and nothing records the choice.
- **Why it matters:** trivial today; unpinned, later crates will go either way and the
  error style will drift.
- **Fix:** either adopt `thiserror` (mechanical) or write a one-paragraph ADR "hand-rolled
  error impls, fewer proc-macro deps" and edit §6.2. No preference; pick one.

## The six DESIGN.md divergences (m0/04, m0/05, m0/12; KNOWN_GAPS row "diverges in six named places")

All six confirmed in code exactly as the tickets declared them. Rulings — **in every case
the code is right and the spec should be amended** (one ADR can cover all six):

1. **Sanitiser takes all of `Cf` + U+2028/29** where §8.2 lists five ranges
   (`sanitize/mod.rs:214-249`). Spec's list is a subset; a category survives Unicode
   revisions, an enumeration quietly stops being complete. **Keep code, edit §8.2.**
2. **`sanitize_line` / `sanitize_path*` added** where §8.2 assumed widgets handle `\n`/`\t`
   (`sanitize/mod.rs:141-172`). A file name legally contains a newline; drawn raw in a
   status row it scrolls the screen. Strictly safer. **Keep code, edit §8.2.**
3. **`graft_term::detect::detect()` free function** where DESIGN sketched
   `Capabilities::detect()`. Forced by ADR-0007 (no behaviour in `core`) — the spec sketch
   predates that ADR. **Keep code, edit the DESIGN sketch.**
4. **Inside tmux, `iterm2_images` is false unless the passthrough re-probe answered**
   (`detect/mod.rs`, m0/04). §4.1's iTerm2 env-hint exception describes the *outer*
   terminal; under tmux the image bytes must transit tmux, which the hint says nothing
   about. Failing closed is the only honest reading. **Keep code, add a sentence to §4.3.**
5. **Protocol-less tmux gets Unicode** where §4.2's table row ("neither of the above →
   ASCII") would say ASCII (`detect/mod.rs:457-459`). §4.3's prose *already says* "falls
   back to the Unicode tier" — the spec disagrees with itself, and the code sides with the
   prose and with the user's daily view (Zed/VS Code terminals, Unicode primary). Note
   `TERM=dumb`/not-a-TTY still wins ASCII first (`:210`), so the floor is safe.
   **Keep code, add a tmux row to §4.2's table.**
6. **The tmux re-probe gets a second full budget** (`detect/mod.rs:247,373`) where §4.4
   sizes one. Worst case 400 ms local / 1 s SSH startup, only inside tmux, only when the
   fence never returns. A shared budget would let a slow first round starve the re-probe
   and misreport passthrough. Acceptable; the worst case belongs in the ADR. **Keep code,
   edit §4.4 to "per round".**

## Claims vs evidence

Spot-checked the builders' claims; found them honest, and unusually well-hedged:

- Every "verified" claim names its method (pty harness, termios byte-comparison, byte-stream
  grep for SGR under `NO_COLOR`). `.scratch/m0-foundations/pty-signal-check.py` exists as
  claimed; both examples (`capability_probe.rs`, `terminal_probe.rs`) exist and are wired to
  the library, not stubs.
- m0/12, KNOWN_GAPS and the README all state plainly that no real emulator ever answered a
  probe and no human has seen the screen. Nothing claims otherwise. m0/12 *found and
  reported* the `detected` bug rather than shipping it silently — the right behaviour.
- KNOWN_GAPS re-checked every row against code, closed nothing on faith, and marks four rows
  as the checkpoint's job, not a builder's. One omission: Major 1 (see above).
- Nothing marked done is stubbed: `graft-graph`/`graft-ops` are declared empty, the runner's
  unused cancellation path is declared unused, the panic hook's untested real-hook path is
  declared unproven.

## Architecture and readability

- Crate graph matches ARCHITECTURE.md and PROMPT §6.1 exactly; the dependency-edge test
  (ADR-0006) pins it. `ui` imports neither `git` nor `ops`. `graft-core` holds plain data
  only (ADR-0007 — `capabilities.rs` has zero functions).
- `update` is pure, jobs return as events, the loop never waits on git. The `Dispatch` seam
  and `JobEvents` trait keep the loop testable without threads.
- Readability is the best I've reviewed at this scale: module docs explain *why*, names are
  full words, tests read as documentation (`a_branch_name_with_an_escape_in_it_cannot_reach_the_terminal`).
  Fowler-smell pass over the whole range found nothing beyond Major 3's file size.
- One accepted-and-documented design debt to watch: `AppState` keeps no job ids, so a stale
  job result still writes the status line (KNOWN_GAPS, M1). Fine for M0's two jobs.

## What this review could not check

- **Anything on a real terminal.** Every run to date — builders' and this review's evidence
  base — was a bare pty. No emulator has answered a probe; no human has seen the screen;
  tier decisions, theme matching, DEC 2026 wrapping and the restore sequence are verified
  against byte expectations only. That is exactly what the M0 checkpoint exists to close.
- **The real panic hook** (KNOWN_GAPS: reviewed, never executed — process-wide hook).
- **Non-UTF-8 paths end-to-end**: the plumbing is right by reading (`OsString`/bytes all the
  way, lossy only at the sanitiser), unit-tested at the sanitiser, but never driven through
  a real repo with a hostile path.
- **Any performance or memory budget in the spec**: no heavy tier, benchmarks or fixtures
  exist yet (KNOWN_GAPS, M1). "Gates green" currently means the fast tier only.
- **tmux behaviour**: the passthrough path is tested against synthetic replies only.

## Asks for the checkpoint

1. Fix Major 2 before the user runs the real-terminal checkpoint (or accept measuring with
   a lying line and re-run after).
2. Approve one ticket for Major 1 + Minor 3 (lint enforcement, unsafe ADR-or-removal).
3. Approve one ADR covering all six DESIGN divergences + the matching DESIGN.md edits.
4. Major 3's split can ride along with M1's first detection change if preferred; before M2
   at the latest.
