# M4 group 01 review — remote fixtures

Range: `a3010f5..5f6e6fd` (group commits `70c75c3`, `981abc2`; merge `5f6e6fd`).
Crate: `graft-fixtures` (`src/remote.rs`, `remote/{local,http,hung,tests}.rs`).

Gates: not re-run in full (conductor already ran fmt/clippy/nextest 1648/1648,
doctests 2/2 on merged main, per task brief). Verified narrowly: `cargo test
-p graft-fixtures --lib remote::` — 11/11 pass in 1.68 s; ran the compiled
test binary directly and checked `lsof -iTCP -sTCP:LISTEN` and `ps` afterward
— clean, no stray listener or process on the happy path.

## Tick spot-check (ADR-0035)

Checked all 6 ticked criteria against their named tests (ran each, read the
assertions): all 6 held.

- `local_remote_clone_has_origin_and_upstream` — ran, passes; asserts
  `origin` URL and `<branch>@{upstream}` = `origin/<branch>`.
- `commit_on_remote_leaves_work_behind`, `commit_locally_leaves_work_ahead` —
  ran, pass; check bare vs. work tips move as claimed, including a second
  `commit_on_remote` call after `work` has pushed (the "builds on the first"
  case in NOTES).
- `http_wrong_password_is_401_and_git_says_authentication_failed` — ran,
  passes. One real gap, already disclosed in NOTES: the tick's own wording
  ("401") is proved through git's behaviour and the `rejected` counter, never
  by reading the status line off the wire. Acceptable — git's real client
  behaviour is what group 02/06 actually depend on — but say so plainly in
  the tick text next time rather than leaving it for NOTES to explain.
- `http_right_password_clones_and_pushes` — ran, passes; the 2 MiB
  incompressible push does arrive chunked (`chunked_requests() >= 1`
  asserted).
- `hung_remote_blocks_then_dies_on_sigterm` — ran, passes; fetch is
  genuinely still running 300 ms after connecting, `kill()` ends it, port
  closes after drop.
- `hermetic_env_blocks_user_config` — ran, passes; asserts no config line
  names the real `$HOME`.

The two left unticked (thread-exit proof; MAP.md/docs — doc-only, no test
can check it) are correctly left open, not silently marked done. No false
ticks found.

## Major

1. **Listener-thread exit is genuinely unproven, and the failure mode is
   worse than "one test leaks a thread."** `HttpRemote::drop`
   (`remote/http.rs:190-216`) and `HungRemote::drop` (`remote/hung.rs:85-108`)
   both give the listener thread a bounded 2 s to stop, print a warning, and
   **return without joining** if it doesn't. If the thread is truly stuck —
   e.g. blocked inside `run_backend`'s `child.wait_with_output()`
   (`remote/http.rs:314-335`) because a spawned `git http-backend` hangs, or
   `hung.rs`'s wake-up race below — the thread, its bound port, and any child
   process it's waiting on are **not cleaned up for the rest of the test
   binary's life**, not just this one test's. `cargo nextest` runs many
   tests per process; a single stuck teardown here silently poisons every
   later test in that process (ports still nominally free per-process, but a
   dangling thread/child holds real file descriptors and CPU). This is
   exactly the condition the review brief warned about — a fixture that only
   sometimes cleans up is worse than none — and it's currently reachable
   only through a panic or a genuinely hung child, so ordinary green runs
   won't show it. No test manufactures this condition to prove the bound
   actually fires and recovers. Recommend: a test that stubs a
   never-returning "git" (a shell script) to force the 2 s path and asserts
   the warning fires and the test binary still exits promptly; short of that,
   file it in `KNOWN_GAPS.md` now, not after group 02+ start relying on this
   fixture under load (cancel-mid-fetch, `Esc` scenarios in 05/07).

## Minor

2. **`HungRemote`'s wake-up has a narrow race that can silently drop a real
   connection instead of holding it** (`remote/hung.rs:62-77` vs. `85-108`).
   `Drop` sets `stopping` then self-connects to unblock the blocking
   `accept()`. If a genuine client connection is already queued in the OS
   backlog ahead of that self-connect when `stopping` flips, `accept()` may
   return the *real* connection first; the loop sees `stopping == true` and
   returns before pushing it to `held` — the real connection is dropped
   (closed) rather than counted and held. Current tests never hit this: they
   confirm the fetch is killed and its process exit observed before dropping
   `HungRemote`, so there's no connection racing the teardown. Flag for
   whoever writes 05's `esc_cancels_hung_fetch` (needs `connections() == 1`
   after an `Esc`-driven cancel) — that test's ordering (cancel confirmed,
   then check the fixture) avoids the race the same way, but a future test
   that drops `HungRemote` while a fetch is still connecting could see an
   intermittent failure that looks like the app's fault and isn't.
3. **Signature deviations checked against downstream specs, low current
   risk.** `LocalRemote::new -> Result<_, CorpusError>` (design note said
   `Unavailable`) and `Oid` as a 40-hex `String` (not `graft_core::Oid`):
   grepped `.scratch/m4-remotes/issues/02,04,05,06,07*.md` — none bind to the
   concrete error type or to the `Oid` type of `commit_on_remote`/
   `commit_locally`'s return value (04/05 read truth back through git, not
   through the return value). Not a current defect; worth the conductor
   double-checking when 04/05 land, since `CorpusError` has no obvious
   `From<CorpusError> for io::Error`, so a `?` inside an `io::Result` test fn
   would not compile as written.
4. `HttpRemote::drop`'s bounded wait is a hand-rolled busy-wait
   (`thread::sleep(5ms)` loop) rather than a single bounded join — works,
   but slightly more code and CPU than `JoinHandle` warrants; a cosmetic
   nit, not a correctness issue.

## What's solid

- Hermeticity is thorough and goes beyond `HOME`: `GIT_CONFIG_NOSYSTEM=1`,
  `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_SYSTEM=/dev/null`,
  `GIT_ATTR_NOSYSTEM=1`, `LC_ALL=C`, plus a fixed identity/clock
  (`crates/graft-fixtures/src/git.rs:265-284`) — the M3 "wrote `~/.gnupg/`"
  class of leak looks closed, and `hermetic_env_blocks_user_config` proves it
  for `LocalRemote`. Confirmed no stray writes: `lsof`/`ps` clean after a
  direct run of the compiled test binary.
- Auth is a genuine round trip, not a stub: the wrong-password test drives a
  real anonymous request (server 401s it, git gives up locally) and a real
  Basic-auth attempt with a wrong password (server 401s it again, git prints
  "Authentication failed"); the counters move only on real HTTP responses.
  This is what group 02's classifier and group 06's prompt need.
- `tiny_http` is correctly a normal (not dev-)dependency with a comment
  explaining why (other crates' tests call it, and the whole crate is
  dev-only per `ARCHITECTURE.md`'s "fixtures: test-only, outside the table"
  section); `Cargo.lock` growth (`ascii`, `chunked_transfer`, `httpdate`,
  `tiny_http`) is confined to that edge.
- No `unwrap()`/`expect()` outside test code in `remote.rs`/`local.rs`/
  `http.rs`/`hung.rs`; no `unsafe`; errors are propagated with context
  (`CorpusError::Io { context, source }`), not swallowed.
