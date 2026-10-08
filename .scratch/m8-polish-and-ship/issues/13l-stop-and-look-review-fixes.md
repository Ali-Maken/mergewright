# 13l — stop/look review fixes: no `unsafe` in graft's tests, the two big test-side files split

Status: ready (conductor 2026-10-08; Part A now, Part B after 11e) — was draft (planner spec-only pass 2026-10-08, from `docs/reviews/m8-stop-and-look.md` Majors 1–3)
Build with: **Part A Sonnet**: a move-only split plus naming `Drop`'s steps, with the review naming the cuts.
**Part B Opus (high)**: whether the hang-up tests still deliver a real kernel `SIGHUP` to a session leader can't
be judged from the diff, only by seeing a test go red without it. Not Fable: the code is test-only, nothing is
structural, and it fails Fable test 1.
Scout: none needed. The review names every line and cut (`hangup.rs:1-300`, `:143-163`, `http.rs:401-468`). Say
this in the report.
Needs: none (no new public signatures).
Blocked by:
- **Part A** (graft-fixtures) can run **now**, beside 11e (11e doesn't touch graft-fixtures). The cap is two cargo
  helpers.
- **Part B** (graft `tests/it/quit_waits/*`) waits for **11e merged**. 11e owns the captures and re-shoots
  `tests/it/*` capture tests. The hang-up capture (`hangup.rs:593` banner, `check_shot`) must be read *after* 11e's
  footers land, then rebased. Part B may run beside 18d, 18b or 15: their files (`places.rs`, `empty_state.rs`,
  `first_launch`, `jobs`, `update::credential`, `hints*`) are disjoint from Part B's. Only `crates/graft/MAP.md`
  is shared, on different rows, and the conductor merges it.
- **Part C** (box M1) is the **planner's**, after `docs/research/m8-look-polish.md` lands. The conductor routes it.
  It isn't builder work.

Crate set:
- Part A: `crates/graft-fixtures/src/remote/http.rs` → `remote/http/*`, `crates/graft-fixtures/MAP.md`.
- Part B: `crates/graft/tests/it/quit_waits/*`, the `it` crate root (`#![forbid(unsafe_code)]`),
  `crates/graft/Cargo.toml` `[dev-dependencies]` (features only), `crates/graft/MAP.md` test rows.
- **Not:** product code in any crate, `docs/shots/**`, ADRs, the workspace `Cargo.toml` comment, graft-ui.

## Rulings (planner)
- **M2: replace with a safe route; no new ADR.** The `unsafe` exists only because `pre_exec` runs code between
  fork and exec. Remove that need, and the rule violation goes with it. ADR-0015's invariant ("two files to review
  for `unsafe`") then stays true. Route:
  - The `it` test binary **re-execs itself** into a no-op-unless-flagged entry test. The precedent is 21's re-exec'd
    child (graft-git `snapshot_reads::autocrlf_from_the_global_config_is_read_isolated`).
  - That child is a fresh process, not a fork. In it:
    - safe `nix::unistd::setsid()`;
    - fd 0 (the pty slave) taken as the controlling terminal through a **safe** wrapper (`rustix::process::ioctl_tiocsctty`,
      rustix 1.1.4 is already in `Cargo.lock`), or by opening the slave by path after `setsid` where the platform
      grants a controlling terminal on open;
    - `dup2` onto 1 and 2;
    - `CommandExt::exec(graft)`. The pid is kept, so the session leader is graft itself, as now.
  - libtest's own stdout goes to `/dev/null` until the `dup2`, so the "running 1 test" text never reaches the pty.
  - The master fd is set `FD_CLOEXEC` in the parent. This replaces the `close` inside `pre_exec`.
  - The argument vector travels in env vars, never in libtest's argv.
  - **Stop rule:** if no safe call gives graft a controlling terminal on macOS, or if the route needs a package
    `Cargo.lock` doesn't already hold, stop and report `blocked: safe route` with the evidence. The fallback is
    already agreed, and the conductor dispatches it: an ADR amending 0015 for exactly this `pre_exec`, the helper
    moved to `graft-fixtures`, and a line in the `Cargo.toml` comment. Don't keep the `allow`.
- **M3: decided per file.**
  - `remote/http.rs` (942): **split now (A)**. It's new in 13j, nobody else holds it, and `Drop` is 67 lines.
  - `quit_waits/hangup.rs` (674): **split now (B)**. B rewrites its harness anyway.
  - `graft-ui/blame.rs` (601): **→ 21**. Minors m6 and m7 reshape it (the move of `whitespace_tail` alone shrinks
    it), and the M1 study may file a follow-up on its title tail. One split, after those.
  - `graft-ui/graph_column/mod.rs` (443): **→ 21**. The M1 study covers the lane fade and the `+N` there. Split once,
    after any follow-up.
  - `graft/event_loop/startup.rs` (801): **→ 21, after 18b**. The 18 group is in `event_loop`, and minor m3 lands in
    this file.
  - The rows for these three are in NOTES.

## Scope
**In:**
- **A1.** Move-only split of `remote/http.rs` into `remote/http/{listener,request,cgi}.rs`, in **one commit**:
  - listener: the accept loop and `Drop`;
  - request: framing (`read_request`, `read_chunked`, `write_response`);
  - cgi: the `http-backend` bridge.

  The public path `remote::HttpRemote` is unchanged. `git diff -M --stat` shows moves only.
- **A2.** A **separate commit**: `Drop` becomes ≤ 60 lines by naming its steps as functions, with the order kept
  exactly as the MAP row states it.
- **B1.** Move-only: the `Session`/pty harness (`hangup.rs:1-300`) moves to `quit_waits/pty_session.rs`. Test paths
  `it::quit_waits::hangup::*` are unchanged.
- **B2.** The safe route above replaces `session_leader` (`:143-163`). `#![forbid(unsafe_code)]` goes on the `it`
  crate root, so a fourth site fails to compile.
- **C.** Planner: Prior art and Journeys rows (box below).

**Out** (proposed rows in NOTES, for the conductor):
- Minors m1–m11 and Questions Q1–Q3.
- The blame, graph_column and startup splits.
- Any capture re-shoot.

## Signatures
No public item changes. New private test items only:
```rust
// crates/graft/tests/it/quit_waits/pty_session.rs
pub(crate) const SESSION_EXEC_VAR: &str = "GRAFT_IT_SESSION_EXEC"; // program; args in `_ARG_<n>`, count in `_ARGC`
/// The re-exec target: returns at once unless SESSION_EXEC_VAR is set; then setsid, fd 0 → controlling tty, exec.
#[test] fn session_exec_entry();
/// Replaces `session_leader`: builds the re-exec command around `program`, stdin = slave, stdout/stderr = null.
pub(crate) fn session_command(program: &Path, args: &[OsString], slave: OwnedFd) -> Command;
```

## Read first
- `docs/reviews/m8-stop-and-look.md`: Majors M2 and M3, minor m5, Q2.
- `docs/adr/0015-one-unsafe-getrusage-call-in-graft-fixtures.md` "Amendment" and "Consequences".
- `crates/graft-fixtures/MAP.md` `remote::HttpRemote` (:44). `crates/graft/MAP.md`: the `tests/it` rows.
- `.scratch/m8-polish-and-ship/issues/13g-hangup-leaves-it-paused.md` NOTES (why a session leader).
- `.scratch/m8-polish-and-ship/issues/21-hygiene-code.md` item 4 (the re-exec precedent).
- Then source: `remote/http.rs`, `remote/tests.rs`, `quit_waits/hangup.rs`.

## Touches (check each; say how)
- `HttpRemote` callers (graft-ops and graft M4 remote tests, `m4_walk` credentials): the public API is unchanged,
  and workspace nextest covers them. Grep `HttpRemote` and list the callers in NOTES.
- The 13j heavy load loop (open box on 13j): it uses `HttpRemote`. Heavy clippy compiles it. Don't run it.
- Other users of the moved harness: grep `session_leader` and `Session` under `tests/it`, and list them.
- Captures: the hang-up capture must pass **unchanged**, with no `GRAFT_UPDATE_SHOTS`. A diff is a defect: stop.
- `Cargo.lock`: no new `[[package]]`. Show `git diff Cargo.lock`.
- ADR-0015 and the workspace `Cargo.toml` `forbid` comment are unchanged. That is the point.
- MAPs: graft-fixtures gets a `remote/http/` layout line. graft gets the `quit_waits` harness row.
- No keymap, journal, undo or product code. `git diff --stat` names only the crate-set paths.

## Acceptance (each tick names its test or check inline)
- [ ] A1: `git diff -M --stat` of the move commit shows moves only. Nextest total N before = N after (both in
      NOTES). Each `remote/http/*.rs` is ≤ 400 production lines (`wc -l` up to `#[cfg(test)]`, in NOTES).
- [ ] A2: `Drop` ≤ 60 lines (count in NOTES). These pass unchanged: `remote::tests` drop-waits (:419), chunked
      (:457), 100-continue (:484), idle client (:520).
- [ ] B1: `hangup.rs` ≤ 400 lines, `pty_session.rs` ≤ 400. Every `quit_waits::hangup::*` test name is unchanged
      (`cargo nextest list` before and after, in NOTES).
- [ ] B2, no unsafe: `grep -rn "unsafe" crates/graft` finds nothing outside comments (output in NOTES). The `it`
      root has `#![forbid(unsafe_code)]`.
- [ ] B2, real hang-up: `quit_waits::hangup` tests pass, run alone as 13g did (heavy features if they're gated; time
      in NOTES). They include the second hang-up (:430-556) and the capture banner (:593).
- [ ] B2, seen red: with the controlling-terminal step removed, the first hang-up test fails because graft never
      gets `SIGHUP`. Record the failing line in NOTES, then restore the step. Without this, the route is unverified:
      say so and leave the box open.
- [ ] B2: `ps -A | grep -c "[g]raft"` reads 0 after the hang-up run, so no orphaned graft (NOTES).
- [ ] C (planner, after `docs/research/m8-look-polish.md` lands): the design note `m8-polish-and-ship.md` gains
      **Prior art** rows (take/reject/invent, each citing that file) and **Journeys** rows for 17's aspects:
      - the folded-merge `+N` (`Space` → `+N`, step-counted);
      - the off-thread fade by hue;
      - the whitespace-mode title tail;
      - `ui.overview`;
      - Pixel ghosts at Unicode (ADR-0060).

      Each "reject" becomes a follow-up ticket spec.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
One context for each part:
- Part A: about 35k tokens (reading `http.rs` is most of it).
- Part B: about 45k tokens.

The conductor may dispatch A and B as two builders at different times (see Blocked by). If one builder takes both,
do A first and commit at the seam.

## NOTES — proposed KNOWN_GAPS rows (planner, for the conductor)
| Id | Row | Status | Route |
|---|---|---|---|
| M8/stop-read-cadence | m1: `next_read` is taken from the `now` before the read (graft-git `run/stop.rs:226,234`; `:262,266` in `after_git_ended`), so a slow `ps` makes the reads back-to-back. Test `a_slow_read_of_the_group_does_not_spend_gits_grace` (:507-532): about 6.3 s by reading, NOTES claim 2.3 s | broken (cadence); time unverified | 21 |
| M8/13k-sleep-unguarded | m2: `stop.rs:510` `sleep 600` holds no `Reap` guard, which breaks 13e's Minor 8 rule | broken | 21 |
| M8/hangup-exemption-too-wide | m3: `startup.rs:531` and `:482` turn any error into `Ok` on `Signal(SIGHUP)`. They should exempt only `EIO` / `input::hung_up` (rule 3) | broken | 21, with the startup split, before 28 |
| M8/waiting-line-again | m4: `exit_wait.rs:186` "send the signal again" is printed on `Ending::Exited` too, where no signal was sent | broken (textual) | 20 |
| M8/idle-client-bound | m5: `remote/tests.rs:534-537` allows 2 s, but the 1 s `READ_TIMEOUT` (`http.rs:42,383`) passes it alone, so the test doesn't pin `Drop`'s shutdown. Line numbers move with 13l A: the builder updates them | broken (test weakness) | 21 |
| M8/blame-ws-default-twice | m6: graft-ui `blame.rs:232` literal `true` vs graft-git `blame.rs:77` `BlameOptions::default` | broken | 21, before the blame split |
| M8/whitespace-tail-home | m7: `whitespace_tail` and `put_whitespace_tail` live in `blame.rs` but are used by `detail_pane/header.rs:350-351` and `history.rs:299-300`. Move them to `graft-ui/src/whitespace_tail.rs` | broken (placement) | 21, before the blame split |
| M8/captures-two-places | m8 **ruling**, below | decided | 28 |
| M8/fold-refresh-test-blind | m10: `update/fold_count/tests.rs:56-60` asserts only the absence of `CountFolded`, and also passes if `Ctrl+R` did nothing | broken (test weakness) | 21 |
| M8/adr-0060-wording | m11: ADR-0060 decision 1 and §1.7 say "with ghost rows", but the built condition is any landed preview (`view/rows.rs:177-182`). Decision 2 says `tier()`, but the code reads `tier_in_force` (`view/frame.rs:110,224`) | broken (textual) | 23 docs pass |
| M8/look-sizes-2 | graft-ui `blame.rs` 601, `graph_column/mod.rs` 443 (PROMPT 6.2). Split after the M1 follow-ups and m6/m7 | broken | 21 |
| M8/look-file-sizes (refresh) | `startup.rs` is now 801, not 704. Split after 18b | broken | 21 |
| M8/13g-double-sighup (update) | Q1 now has its lines: `signals.rs:98-99` (`at = max(seen,1)`) and `:106`; `exit_wait.rs:92`. Whether his shells forward `SIGHUP` on window close is not verified | unverified (question) | keep |
| M8/http-read-timeout | Q2: does `shutdown` wake a blocked read on macOS? If it does, the 1 s per-read timeout (`http.rs:383`, `handle` :732-737) buys nothing for teardown and could fail a slow client under load. A spec change goes through the conductor | unverified (question) | 21 |
| M8/blame-place-ends-with | Q3: should `tests/it/blame_place.rs:160` go back from `contains` to `ends_with` now that the tail is off by default? | unverified (question) | 21, with m6/m7 |

**m9** (the conductor's job, not a row): close these, naming the ticket that fixed each:

| Row | Fixed by |
|---|---|
| `KNOWN_GAPS.md:156` | 17b |
| `:983` `M7/21c-ws` | 17/17b |
| `:1149` `M8/hangup-exit-101-orphans-git` | 13g |
| `:1219` `M8/stop-files-over-400` | 13e/13f, with the review's line counts |

**m8 ruling (captures in two places).** No files move. 11e owns the captures and is re-shooting `docs/shots/m8/doorway-*`.
- 28 builds the sheet with `scripts/contact-sheet.sh --captions-order m8 docs/shots`, with
  `docs/shots/m8-contact-sheet-captions.tsv` naming the subdirectory captures as `m8/<file>`.
- The script already takes those names: it checks `"$shots_dir/$name"` (`contact-sheet.sh:154`) and orders by the
  captions file (:102-106). So one run gathers both sets.
- The sheet lands at `docs/shots/m8-contact-sheet.txt`, the script's output name. 28's box says
  `docs/shots/m8/contact-sheet.txt`: **the conductor amends 28**.
- 28's coverage test must read both `docs/shots/m8-*.txt` and `docs/shots/m8/*.txt`.
- New M8 captures from here on go flat, as `docs/shots/m8-*.txt`.
- 17's tick naming the `m8/` path stands.
