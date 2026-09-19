# Review: m4-remotes/05 remote-keys

Reviewed: merge `4be9a99` against its first parent (`git diff 4be9a99^1 4be9a99`).
Ticket: `.scratch/m4-remotes/issues/05-remote-keys.md`. Read-only review; no fixes made.

## Verdict

0 Critical · 4 Major · 11 Minor. The group does what the ticket says: `f`/`p`/`P` are
wired at the graph level with the keymap's keys, the push box carries the design note's
words, `Esc` reaches every write through one path (`stop_write`), and the eight
key-to-repo tests read the truth from git. The Majors are about how it was verified,
not what it does: two poll-and-sleep waits in the integration test, one claimed
behaviour change with no test, one newly reachable cancel path with no test.

## What was checked and how

- Gates, in the conductor's isolated target dir (`target-review05`, deleted after):
  - `cargo fmt --check` — clean.
  - `cargo nextest run -p graft` — **707 passed, 18 skipped, 0 failed**. 14.66 s test time
    on the first run (39.7 s wall with a cold build), 9.53 s on the second
    (`--status-level all`, for per-test times). Peak RSS of the largest process ≈ 695 MB
    (`/usr/bin/time -l`). `target-review05` reached 1.5 GB; removed.
  - 5 slowest: `history::tests::a_configured_default_branch_takes_lane_0` 1.196 s,
    `history::tests::a_search_hit_past_the_first_chunk_is_laid_out_to_and_selected` 1.188 s,
    `history::tests::a_request_in_another_view_interrupts_and_is_answered_next` 1.187 s,
    `history::tests::a_request_lays_out_only_the_rows_it_names` 1.080 s,
    `it::remote_by_key::pull_then_undo_redo` 1.051 s. The eight `remote_by_key` tests
    take 0.35–1.05 s each. Laptop-safe.
  - **Not run**, on the conductor's instruction (builders share the machine): clippy
    (plain and `--features heavy-tests`), `cargo test --doc`, the heavy tier, the release
    perf run, fuzzing and benchmarks. Their results are unknown here, not green.
- Ticks: 14 in the ticket. 13 name a test; I ran all 13 (8 in
  `tests/it/remote_by_key.rs`, 5 unit) and **13 held**. The 14th (MAP.md,
  ARCHITECTURE.md) is checked by reading the diff: both updated, held.
- `main.rs` askpass mode: read against `graft_git::run::askpass::respond` /
  `RespondError` (askpass.rs L388–434). The helper creates no directory (the
  `0700` dir is `AskPassServer`'s, created and removed on the running Graft's side,
  askpass.rs L150, L235–240). `RespondError`'s `Display` carries an `io::Error`, a
  static `&str` or `cancelled` — never the answer. Stdout carries the secret only to
  git, which is the channel. No exit path leaks the dir or prints a secret.
- Pool cancel semantics: `jobs/pool.rs` L175–193 turns any product returned after
  `cancel` into `Event::JobCancelled`, so `update::remote::cancelled` is the path that
  runs; confirmed by `esc_cancels_the_running_write_once` and the two `Esc` it tests.
- Key order (`update/keys.rs` L36–88): confirm → menu → editor → undo → `Esc` cancel →
  `Failed` dismissal → panes. `cancel_operating` also refuses while `text_field_open()`.
  Matches the ticket and M3 06 C1.

## Critical

None.

## Major

### M1. Two poll-and-sleep waits in the integration test (rule 2)

`crates/graft/tests/it/remote_by_key.rs`, `wait_for_connection` (≈L343–352) and the
marker-file loop in `esc_cancels_local_write` (≈L425–429): both are
`while !cond { sleep(20 ms) }` with a 30 s deadline. CLAUDE.md rule 2 bans sleep and
poll loops without the user's OK; the ticket did not grant one, and the conductor asked
about exactly this. The builder's comments say the hook and the fixture "have no event
to wait on" — that is a reason to add one, not to poll:

- Hook: `mkfifo` a path, have the hook `echo > fifo` before `sleep 5`; the test opens the
  FIFO for read, which blocks until the hook writes. One event, no timer.
- `HungRemote`: give the fixture a `wait_for_connection()` (a `Condvar` or channel
  raised in `accept_and_hold`). That is `graft-fixtures`, outside this group's crate
  set — the right move was `blocked: needs graft-fixtures` naming the signature.

Why it matters: the harness's own `pump` waits on `recv_timeout`, so these two loops are
the only timers in the file; they are what will flake first on a loaded laptop, and the
rule exists to keep such waits out of the suite.

### M2. `Sticky` surviving a refresh is claimed, changed, and untested

`crates/graft/src/update/mod.rs` `repository_opened` (≈L293–303) now clears only
`Status::Failed`; the NOTES say this is what makes the second `P` possible. No test
covers it. `rejected_then_lease_push` asserts `Sticky(...)` right after `finish()`,
which returns *before* the refresh's `Product::Repository` is processed (`pump`
dispatches the refresh commands and returns; `ready()` then swallows the answer without
asserting status). The second `P` depends on `last_rejected_push`, not on the status, so
the test would pass even if `Sticky` were still wiped.

Fix: one unit test in `update/tests.rs`: `Sticky("x")` + `Product::Repository(Ok(..))`
→ still `Sticky("x")`; `Failed("y")` + the same → `Ready`; `Busy`/`Done` untouched.

### M3. `Esc` now reaches an undo or redo in flight — unverified

`update/undo.rs` `launch` registers with `remote::launched`, so `Esc` sends
`CancelJob` for `u`/`U` too (ticket: "every write"). No test cancels an undo. An
inverse plan is several git invocations; a SIGTERM between steps leaves the undo
half-applied and relies on the M3 `Pending`/interrupted-notice mechanism
(`jobs/undo.rs` L264 mentions the compare-and-swap case). This is the "you cannot lose
work" promise, so it is either tested or written down as unverified.

Fix: an it test in the `esc_cancels_local_write` shape (a slow post-checkout hook or a
large restore), or a KNOWN_GAPS row marked **unverified** with M3's interrupted-notice
behaviour as the safety net.

### M4. `remote_event` re-matches with a silent catch-all (rule 3 shape, PROMPT 6.2)

`crates/graft/src/update/remote.rs` ≈L130–141: `answer` in `update/mod.rs` matches
`event @ (Produced(_, Remotes(_)) | Progress(_, RemoteProgress { .. }))`, hands the
whole event over, and `remote_event` matches it again with `_ => {}`. Two matches for
one dispatch, and a catch-all that would silently drop any future event routed here by
mistake. Plain version: `answer` calls `remotes_arrived(next, id, result)` and
`remote_progress(next, id, &brief)` directly from its own two arms; `remote_event`
goes.

## Minor

1. **`main.rs` `askpass`**: `std::env::args().nth(1)` panics if any argv is not UTF-8.
   git's prompt embeds the remote URL, which need not be. The helper then exits 101 with
   a panic message on git's stderr — no secret, but a crash where a refusal was meant.
   Use `std::env::args_os().nth(1).map(|a| a.to_string_lossy().into_owned())`.
2. **Cancel kills git's pid, not its process group** (`graft-git/src/run/exec.rs`
   L203–245, not this group's crate). The hook's `sleep 5` and git's `git-remote-http`
   child outlive `Esc`. The it test cannot see it: `HungRemote::drop` closes the socket,
   which ends the orphan. Against a real hung network the helper lingers until TCP gives
   up. Route to a G2 fix (`setsid`/`kill -- -pgid`) or a KNOWN_GAPS row.
3. **A write that finishes in the cancel window is reported cancelled**
   (`jobs/pool.rs` L190: any product after `cancel` → `JobCancelled`). The line then says
   `commit "x" cancelled` for a commit that landed; the `Everything` refresh corrects the
   screen and the journal is right. Pre-existing pool rule, now user-reachable from every
   write. KNOWN_GAPS row, or have `cancelled` read the journal's last entry on refresh.
4. **`remote_failed`'s `Cancelled` arm is unreachable through the pool** (see 3); it is
   exercised only by calling the function directly in
   `failures_are_worded_and_remembered`. Say so in a comment or drop the arm.
5. **Local writes get no `esc cancels` cue** (deviation "only remote operations get
   the hint"). `Esc` does stop a slow commit; nothing on screen says so. Fine to defer —
   the design note's footer text is for remote writes — but it is a gap the user will
   hit on a slow hook. Note it in the ticket or KNOWN_GAPS.
6. **`current_exe()` under `cargo test` is the test binary**: `GIT_ASKPASS` then names a
   binary with no askpass mode. Harmless today (no test reaches an auth prompt: `file://`
   and a hung `http://`) but a trap for 06's tests. Worth a line in 06's ticket.
7. **KNOWN_GAPS rows the ticket and design note point at do not exist**: "`q` confirming
   during an operation (KNOWN_GAPS row, expiry M8)" and choice 4's "no data for N s"
   stall notice. Neither is in `KNOWN_GAPS.md`. Conductor to add both.
8. **Title punctuation differs between the two boxes**: `Push main → origin/main` (no
   `?`) and `Force-push main → origin/main?` (`?`). Copied faithfully from the design
   note, which is itself inconsistent. Pick one in the note; the code follows.
9. **`remotes_arrived` replaces `Ready` facts with `Failed` on a transient read error**
   (`update/remote.rs` ≈L165): after one bad re-read, `p`/`P` refuse with
   `remotes could not be read` until the next refresh. Keeping the last good facts and
   surfacing the error only on the key would match how `request` already keeps `Ready`
   while pending.
10. **`chmod` via `Command::new("chmod")`** in `esc_cancels_local_write`:
    `std::os::unix::fs::PermissionsExt` is plainer and has no spawn.
11. **`Running` duplicates `operating`'s id** (`RemoteState::running`). Documented and
    read only under a matching id, so correct; but the id lives in two places. Not
    worth a change now; noted for the post-M5 architecture pass.

## Deviations listed in the NOTES, judged

| Deviation | Judgement |
|---|---|
| `push_params -> Result<_, String>` | Fine: the several-remotes line names the branch. |
| `push_box(params, ahead, local_tip)` | Right: the lease line needs both tips (`9f8e7d6 → a1b2c3d`), per the note. |
| `remote_failed(.., explained)` reusing `ApplyError::explain` | Good: one source for every failure line; status and journal cannot disagree. |
| `repo_facts(state, ctx, remotes)` + `remote_names` | Fine. |
| `RemoteState` holds `reading`, `running`, `cancelling` | Fine; see Minor 11. |
| Launch read from `settle`, not the startup list | Consistent with the journal's launch read; `an_opened_repository_becomes_what_the_screen_shows` covers it. |
| Only remote ops get ` · esc cancels` | Acceptable; see Minor 5. |
| `cancelling` without `…`; footer draws `ESC_HINT` lines bare | Fine; `a_remote_write_ends_in_its_hint` covers the footer. |
| `repository_opened` clears only `Failed` | Right fix; **untested** — Major M2. |
| Editor `Esc` via `stop_write` | Good: one cancel path. |
| `main.rs` askpass mode | Necessary — without it git would exec the TUI as its helper. Safe on every exit path (see above). Minor 1 on argv decoding. |
| Lease acceptance setup (amend, not `commit_on_remote`) | More honest than the ticket's scenario, which would always be a stale lease; the stale case is its own test. |

## Spec compliance

- Keys `f`/`p`/`P` match `docs/design/02-keymap.md` §2.3; `Esc` per §2.2. Plain keys
  only (`is_remote_key` refuses alt/ctrl). Not free-key squatting: the doc names them.
- Design note choice 1 (push transparent to undo, `· origin/main still has it`):
  `push_then_undo_is_local_only` reads the bare before and after `u`. Held.
- Choice 4 (no timeout, `Esc` cancels, every write): both `Esc` tests; see M3 for undo.
- Choice 5 (progress ships): `progress_reaches_the_status_line` records every status.
- New UI: push box text per the note (`push_box_text_table`); footer line per the note;
  failure line words come from ops (`rejected_line` matches the note); journal rows per
  the note (`remote_rows`). Hints `f fetch  p pull  P push` are visible in the capture.
- Proposed change 3 (`f` = `--all --prune`): `FetchParams { remote: None, prune: true }`.
- Architecture: `app` builds `Operation`s and words outcomes; no write `GitInvocation`
  in `app` (seam rule test still passes). `jobs/remote.rs` is config and refs only.
  ARCHITECTURE.md's new bullet points at the right three places.

## Readability

Good overall. `update/remote.rs` is 579 lines with heavy, useful doc comments; every
`pub` item is documented. Extracting `apply_view_change` out of `graph_key` was a real
improvement. Names are plain (`stop_write`, `cancelled`, `pushed_since`). The one
piece of indirection worth removing is M4.

## Data-loss review

- `Esc` on fetch/pull/push: refresh `Everything` after the cancel, journal `Aborted`,
  locks checked empty in both `Esc` tests. Held.
- `Esc` on commit mid-hook: `before == rev(main)` after; held.
- `Esc` on undo/redo: unverified — M3.
- Push box before every push, `Cancel` under cursor, modal (`box_is_modal`). Held.
- A rejected push never moves anything; a stale lease is refused and the bare keeps
  the teammate's commit (`stale_lease_refused`). Held.

## Unverified (the builder said so; still true)

- `docs/shots/m4-remote-keys.txt`: TestBackend, no colour; the lease box's long command
  is truncated by graft-ui's box at 120 columns (`…`). For the user's eye.
- Real ssh/https remotes with credentials: not exercised; askpass answers `None` until 06.
- Clippy (both feature sets), doc tests, heavy tier, release perf: not run in this
  review (conductor's instruction). The builder's gates line claims them; unconfirmed here.
