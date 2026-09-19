# remote-keys: `f` / `p` / `P`, the push box, `Esc` cancels a running write, live progress, plain-English failures, remote facts in `AppState`

Status: done
Blocked by: 04
Build with: Opus — taking the planner's recommendation: key routing, job plumbing and cancel wiring through the pool.
Crates: graft
Needs: `ops-remote-operations` — `Operation::{Fetch, Pull, Push}`, `FetchParams`, `PullParams`, `PushParams`, `Lease`, `Operation::preview`, `ApplyError::Remote`, `RemoteHooks`, `Context.remote`; `git-remote-and-askpass` — `graft_git::remote::{Remote, TrackedUpstream, list, upstream_of, RemoteFailure, RemoteProgress}`. Tests: `graft_fixtures::remote::{LocalRemote, HungRemote}` from `remote-fixtures`. Footer hints come from `ui-credential-prompt-and-hints` (graft-ui) — if that group has not landed, the hints appear when it does; nothing here blocks on it.

## Read first

- `docs/design-notes/m4-remotes.md` — Promise; Interfaces (the `graft (app)` block); Hard choices 1, 4, 5; Proposed changes 3, 4; New UI: Push box, Footer while a remote write runs, Failure line, Journal browser rows, After `u` skips a push
- `.scratch/m4-remotes/scout.md` §Job runner (lines 6–19), §Status and panes (63–71), §Key routing (73–79), §Remote data (85–91)
- `crates/graft/MAP.md` §update (lines 106–175: `Progress`, `Task`, `Product`, `Command`, `AppState::operating`, `Status`, `text_field_open`), §update::operate (188–196: `launch`, `operated`, `Refresh`), §update::confirm (218–222), §update::undo (224–228), §update::journal (230–236), §update::radar (`reset_for_refresh`, 267), §view (`journal_view` 418, `radar` 420), §jobs (471–503: `operate`, `Operated`, `JobRunner::cancel`), §refs (72–83: `Upstream`)
- `crates/graft-ops/MAP.md` §operation, §apply (after G3 lands); `crates/graft-git/MAP.md` §remote (after G2)
- `docs/design/02-keymap.md` §2.3 rows `f`/`p`/`P` (lines 85–87); §2.2 `Esc`, `q` rows (53, 57)
- M3 review 06 C1 and M4 (`docs/reviews/m3-06-undo-and-stash-ui.md` lines 95–124, 202–217) — the two traps this group must not walk into
- `tests/it/undo_by_key.rs` **by description only** (M3 review 06 lines 37–47): the key-to-repo test shape to copy; ask a scout for its harness helpers' names rather than reading it

## Scope

In — in this order (each piece a commit; this is one interaction — "talk to the remote" — sized per ADR-0035):

1. **Remote facts job.** `jobs/remote.rs::read_remotes(repos, cancel) -> Result<Arc<RemoteFacts>, String>` (`remote::list` + `RepoHandle::head` + `upstream_of`); `Command::ReadRemotes(JobId)`, `Product::Remotes(..)`, `Task::ReadRemotes`; `AppState::remotes: Loading<Arc<RemoteFacts>>`; requested where `ReadRefs` is requested (startup, `refresh_key`, after every write that refreshes `Everything`), stale ids dropped. `view/radar.rs::repo_facts` gets `remotes` from it (the `&[]` comment goes) — one line + one test.
2. **`update/remote.rs`.** `RemoteState { last_rejected_push: Option<PushParams> }` on `AppState`. `fetch_key`: `Operation::Fetch { remote: None, prune: true }` → `update::operate::launch`-style dispatch (`operating` set, `Status::Busy("fetch all remotes · esc cancels")`). `pull_key`: from `remotes` — `head_branch` + `head_upstream`; no upstream → `Status::Failed("main has no upstream — P pushes and sets one")` and no command; detached → `Status::Failed("HEAD is detached — check out a branch first")`; else `Operation::Pull`. `push_key`: no remotes → `Status::Failed("no remote — add one in the shell: git remote add origin <url>")`; else build `PushParams` (upstream's remote/branch, or `origin`/same name with `set_upstream` when none; `origin` absent → the only remote; several and none named origin → `Status::Failed("several remotes and no upstream — set one: git push -u <remote> main")`); when `last_rejected_push` matches this branch, build the **lease** variant with `Lease { expected: upstream.tracking_tip }` (no tip → `Status::Failed("f to fetch first — nothing to lease against")`); open `AppState::confirm` with `ConfirmView::new(title, consequence, preview.join("\n"))`, labels `Cancel`/`Push` or `Force push`, `ConfirmThen::Operate(op)`; text exactly per the note's Push box (counts from `RefBadges`' `Upstream::Apart { ahead, .. }` when known, else omitted). Keys wired in `update/keys.rs` at the graph-screen level (`f`, `p`, `P` are free — scout), no-ops while `operating` or while any overlay/text field is open (`text_field_open()` guard, and the confirm box is modal already).
3. **Cancel.** `cancel_operating(state, commands) -> bool`: `Esc` while `operating.is_some()` and no confirm/menu/editor/prompt is open → `Command::CancelJob(id)`, `Status::Busy("<desc> · cancelling…")`. **Verify first (scout `jobs/pool.rs`):** what `JobRunner::cancel` does to a job's later `Produced` — delivered or dropped. If delivered: `operated()` handles the `Aborted` product as today. If dropped: handle `Event::JobCancelled(id)` when `id == operating` → clear `operating`, `Status::Done("<desc> cancelled")`, refresh `Everything`. Either way the journal has the `Aborted("cancelled")` entry (ops wrote it). Applies to every write, not only remote ones (the M3 gap the scout found).
4. **Progress.** `Progress::RemoteProgress { brief: String }`; `jobs/operate.rs::run_operation` builds `Context.remote = Some(RemoteHooks { progress: |p| send Progress::RemoteProgress { brief: p.brief() }, askpass: NoAskPass /* answers None; G5b replaces it */, helper: current_exe()? })` — `RemoteHooks.helper` is required by the type; use `std::env::current_exe()` and, on error, `Context.remote = None` with the reason in the operation's status line (`no askpass helper: <err> — credentials cannot be asked`). `update::remote::remote_progress`: `Status::Busy(format!("{desc} · {brief} · esc cancels"))` for `id == operating`; under 100 columns `brief` is shortened to `<phase-first-word> <percent>%`.
5. **Failures.** `Operated::Failed` gains the classified failure: `Operated::Failed { error, refresh, remote: Option<RemoteFailure> }` (or read it from `ApplyError::Remote` inside `run_operation` — keep `Operated` the boundary). `update::remote::remote_failed(state, op, failure)`: `Status::Sticky(text)` per the note's Failure line list; `NonFastForward` on a Push → `last_rejected_push = Some(params)`; any successful push or a branch change clears it. `Cancelled` → `Status::Done("<desc> cancelled")` (not sticky). Every remote op's `Done` → `Status::Done("fetched" / "pulled origin/main" / "pushed main → origin/main")` and `Refresh::Everything` (radar reset rides on `refresh_key`).
6. **Undo wording + journal rows.** `update::undo::undone` on `Done`: if any in-force entry newer than the undone one is a `Push`, append ` · <tracking short> still has it` (from `AppState::journal.entries` — no ops change). `view/journal.rs`: Push row summary `origin/main a1b2c3d → 9f8e7d6 · not undoable — origin has it` (from `refs_before/after`; `not undoable — origin has it` when the ref was not recorded); Fetch row `remote-tracking refs · not undoable`; neither ever carries `← u`/`← U` (they come from ops' `next_undo`, which already skips them — assert, don't restate).
7. **Tests** (`tests/it/remote_by_key.rs`, the `undo_by_key.rs` shape: real `JobRunner`, `LocalRemote::work` as the repo, keys through `update()`, truth read by running git): see Acceptance. Plus unit tests for `push_key`'s params/text table.
8. `crates/graft/MAP.md` lines; ARCHITECTURE.md "Where to start reading" gets one bullet: "Changing fetch/pull/push → `app::update::remote` for the keys and the box, `app::jobs::remote` for the facts read, `ops::operation` for what runs".

Out: the credential prompt (G5b — this group's `askpass` answers `None`, so an HTTP remote with auth fails with `AuthFailed`, which is the right interim); `q` confirming during an operation (KNOWN_GAPS row, expiry M8); per-remote fetch; a stall notice.

## Signatures

```rust
// crates/graft/src/jobs/remote.rs
pub struct RemoteFacts { pub remotes: Vec<Remote>, pub head_branch: Option<String>, pub head_upstream: Option<TrackedUpstream> }
pub fn read_remotes(repos: &RepoStore, cancel: &Cancel) -> Result<Arc<RemoteFacts>, String>;
// crates/graft/src/update/mod.rs (additive)
pub enum Command { …, ReadRemotes(JobId) }
pub enum Product { …, Remotes(Result<Arc<RemoteFacts>, String>) }
pub enum Progress { …, RemoteProgress { brief: String } }
pub struct AppState { …, pub remotes: Loading<Arc<RemoteFacts>>, pub remote: update::remote::RemoteState }
// crates/graft/src/update/remote.rs
pub struct RemoteState { pub last_rejected_push: Option<PushParams> }
pub fn fetch_key(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn pull_key(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn push_key(state: &mut AppState) -> bool;                    // opens the box; the box's Enter dispatches
pub fn push_params(state: &AppState) -> Result<PushParams, &'static str>;   // pure; the table the tests drive
pub fn push_box(params: &PushParams, ahead: Option<usize>) -> graft_ui::confirm::ConfirmView;   // pure; title/consequence/command per the note
pub fn cancel_operating(state: &mut AppState, commands: &mut Vec<Command>) -> bool;
pub fn remote_progress(state: &mut AppState, id: JobId, brief: &str);
pub fn remote_failed(state: &mut AppState, op: &Operation, failure: &RemoteFailure) -> Status;
// crates/graft/src/jobs/operate.rs
pub enum Operated { Done(Applied), Incomplete { .. }, Failed { error: String, refresh: Option<Refresh>, remote: Option<RemoteFailure> } }
```

## Acceptance

Every tick names its verifying test inline, at tick time (ADR-0035). Key-to-repo tests read git, never `AppState`, for repository facts.

- [x] `f` after `commit_on_remote`: tracking ref moves, badge reads `main ↓1` (via the refs product), status ends `fetched`, journal has a `Fetch` entry — `remote_by_key::fetch_key_moves_tracking_ref`
- [x] `p`: `main` == tracking tip, file bytes updated; `u` → `main` back, bytes back, tracking ref unchanged; `U` → forward — `remote_by_key::pull_then_undo_redo`
- [x] `p` with no upstream / detached HEAD: no command, status text per the note — `update::remote::tests::pull_refusals_are_words_not_commands`
- [x] `P` opens the box with the note's exact title/consequence/command for: upstream present; no upstream (sets upstream); lease after rejection — `update::remote::tests::push_box_text_table` (3 cases)
- [x] `P` → `Enter`: bare's `main` == local; `u` undoes the **commit** locally, status ends `origin/main still has it`, bare unchanged; `U` restores — `remote_by_key::push_then_undo_is_local_only`
- [x] `P` after `commit_on_remote` → `Sticky` "push rejected: origin/main has commits you don't · p to pull first, or P to force with lease"; `P` → lease box → `Enter` → (after a local amend) bare's `main` == local; without the amend (stale) → `Sticky` names `stale lease` and bare unchanged — `remote_by_key::rejected_then_lease_push`, `remote_by_key::stale_lease_refused`
- [x] `Esc` during `f` against `HungRemote`: `operating` clears within 5 s, journal entry `Aborted` with `cancelled`, status `fetch all remotes cancelled`, no `.git/**/*.lock`, `HungRemote::connections() == 1` — `remote_by_key::esc_cancels_hung_fetch`
- [x] `Esc` during a local `commit` (a pre-commit hook `sleep 5`) cancels it the same way — `remote_by_key::esc_cancels_local_write`
- [x] `Status::Busy` text during a fetch contained a `RemoteProgress.brief` at least once (record every status the harness saw) — `remote_by_key::progress_reaches_the_status_line`
- [x] `u`, `U`, `Ctrl+Z`, `f`, `p`, `P` typed while the push box is open change nothing but the box (M3 06 C1's shape) — `update::remote::tests::box_is_modal`
- [x] Journal browser rows for Push/Fetch/Pull read exactly as the note says, no `← u` on Push/Fetch — `view::journal::tests::remote_rows`
- [x] Radar `RepoFacts.remotes` is the real list — `view::radar::tests::remotes_come_from_facts`
- [x] A `Product::Remotes` under a stale id is dropped — `update::tests::stale_remotes_dropped`
- [x] MAP.md, ARCHITECTURE.md updated — docs, no test; checked by reading the diff

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` (M3 06 C2: the heavy tier must still compile) · `cargo nextest run -p graft` · `cargo test --doc -p graft`.

## Size

One context; brownfield — **scout first** (sonnet, ≤ 1 page each): `update/keys.rs` routing order and free keys, `jobs/pool.rs` cancel semantics, `update/operate.rs::launch`/`operated`, `tests/it/undo_by_key.rs` harness helpers. Report whether you did. Stop seam: after piece 3 (keys + cancel, no progress/failure wording) with 4–7 in a handoff note. Past 150 turns: stop, `partial`, handoff.

## NOTES

**Compacted once mid-ticket.** Remaining after the compaction was: `tests/it/remote_by_key.rs`, MAP.md, ARCHITECTURE.md, gates, capture, this ticket. All done in the second context; nothing left over.

Scouts: two Sonnet scouts (key routing + pool cancel semantics; operate/undo/harness).

Verified by running (not only gates): the 8 `remote_by_key` tests pass on real `git` against `LocalRemote`/`HungRemote`, three consecutive runs, 8/8 each, no flake of `esc_cancels_hung_fetch` seen. `esc_cancels_local_write` first passed vacuously-then-failed: `Esc` landed before the pool started the job, so nothing ran and nothing was journaled. The test now waits for the hook to write a marker file (it is inside `git commit`) before `Esc`.

Deviations from the Signatures block, each deliberate:
- `push_params -> Result<PushParams, String>` (not `&'static str`): the several-remotes refusal names the branch.
- `push_box(params, ahead, local_tip)`: the lease box names the tip it sends (`9f8e7d6 → a1b2c3d`).
- `remote_failed(state, op, failure, explained)`: reuses ops' `ApplyError::explain()` wording rather than restating it here.
- `repo_facts(state, ctx, remotes)` + `view::radar::remote_names`.
- `RemoteState` also holds `reading`, `running`, `cancelling`.
- The launch read runs from `update::settle` once the repository has opened, not from the event loop's startup list.
- Only remote operations get ` · esc cancels`; local writes stay as M3 worded them, though `Esc` stops them too.
- `Busy("<desc> · cancelling")` without `…`: the footer adds it. And the footer now draws a line ending in `ESC_HINT` with no trailing `…` (design note wording) — `view::tests::a_remote_write_ends_in_its_hint`.
- `repository_opened` no longer resets a `Sticky` status to Ready: the refresh after a failed push was wiping the rejection line the next `P` depends on. It clears `Failed` only.
- The commit editor's own `Esc` goes through `update::remote::stop_write`, so a slow commit stops the same way.
- `main.rs` gained askpass mode (`GRAFT_ASKPASS_SOCKET` set → `run::askpass::respond`, exit). Without it git would launch the TUI as its askpass helper. G5b owns the prompt itself.
- Lease acceptance setup: the *succeeding* lease uses an amended, already-pushed commit (rejected as non-fast-forward, then the lease holds because the tracking tip is current). After `commit_on_remote` with no fetch the lease is always stale — that is `stale_lease_refused`.

Unverified / for the user's eye:
- `docs/shots/m4-remote-keys.txt` (TestBackend, not pty; no colour): the lease box and the fetch footer. The box truncates the long `--force-with-lease=…` command at its width (`…9f8e7d6c…`) — graft-ui's box, not changed here.
- Real network remotes (ssh/https with credentials) not exercised; askpass answers `None` until G5b.

Known failing, not mine: `view::tests::every_graph_renderer_config_value_draws_as_it_says` (another builder's snapshot; not accepted, `.snap.new` deleted).

## Review fixes (round 1, `docs/reviews/m4-05-remote-keys.md`)

Scout: one Sonnet `Explore` over `update/undo.rs`, `jobs/undo.rs`, `jobs/pool.rs`,
`update/remote.rs`, `graft-fixtures/remote/hung.rs` and the two poll loops.

**M3 — `Esc` reaching `u`/`U`: the cancel is withdrawn, not proved safe.**
I could not prove wholeness, and the code says why. `graft_ops::restore::execute`
checks its stop token **once, before the first step** (`restore.rs:117`); the
`for step in &plan.steps` loop has no check between steps. The only thing a cancel
reaches mid-plan is the git child of the step in flight (`graft_git::run`, SIGTERM
then SIGKILL): that step fails, the loop breaks, and `finish` journals `Aborted`
with the earlier steps still applied. `jobs/undo.rs`' own doc comment on
`refresh_after_failure` already names the case — two `UpdateRef` steps, the second
refused, the first ref already moved. Nothing rolls that back.

So `u`/`U` are now `Stoppable::No(UNDO_CANNOT_STOP)`: `Esc` sends no `CancelJob`,
the undo runs to its end, and the line reads `undo cannot be stopped part-way —
finishing…`. Every other write is unchanged and still stoppable.

- `update::remote::tests::esc_does_not_stop_an_undo_or_a_redo` — for both `u` and
  `U`: no command from `Esc`, `operating` unchanged, `cancelling` still `None`,
  the line, a second `Esc` the same, and the commit editor's `stop_write` path
  refusing too.
- `update::remote::tests::a_remote_write_is_still_stoppable` — `f` is
  `Stoppable::Yes` and `Esc` still emits `CancelJob`; the refusal is on the write,
  not on `Esc`.
- `it::remote_by_key::esc_cancels_hung_fetch`, `it::remote_by_key::esc_cancels_local_write`
  — unchanged, still green.

**For the conductor, not done here (it is a `graft-ops` change):** restoring the
cancel means a between-step stop check in `restore::execute` *and* a rollback of
the steps that ran. Until then `u`/`U` cannot be interrupted. A withdrawn
capability, not a broken one.

**M1 — the two poll-and-sleep waits are gone.** Both are now blocking waits on a
real event, with a 30 s deadline for the event that never comes (a deadline, not a
poll interval: nothing wakes in between).

- `HungRemote::wait_for_connection(within) -> bool` in `graft-fixtures`: the
  listener raises a `Condvar` as it accepts. `it::remote_by_key::esc_cancels_hung_fetch`.
- `graft_fixtures::fifo::Fifo` (new, unix): a named pipe with a thread already
  blocked on reading it; the pre-commit hook does `echo started > "$FIFO"`. Held
  open read-write, so the hook's write never blocks and a write that arrives
  before the wait is not missed. `fifo::tests::a_write_ends_the_wait`,
  `fifo::tests::a_write_before_the_wait_is_not_missed`,
  `fifo::tests::a_wait_with_no_writer_times_out`; used by
  `it::remote_by_key::esc_cancels_local_write`.
- `mkfifo` goes through the POSIX binary, not `libc::mkfifo`: `graft-fixtures` is
  `#![deny(unsafe_code)]` and admits only the sites ADR-0015 names. Widening that
  is a decision, not a fixture's to take.

**M4 — the catch-all is gone.** `remote_event` deleted; `update::answer` calls
`remote::remotes_arrived` and `remote::remote_progress` from its own two arms.
Every event is matched exactly once. `answer` carries an
`#[expect(clippy::too_many_lines)]` whose reason is the one
`graft_ops::operation::explain` already carries (m4/04 review, Minor 7): the only
way to shorten it is the re-entering catch-all this fix removed.
Covered by `update::remote::tests::progress_reaches_the_line_and_narrows`,
`update::tests::stale_remotes_dropped`,
`it::remote_by_key::progress_reaches_the_status_line`.

**M2 — `Sticky` surviving a refresh is now tested.**
`update::tests::only_a_read_failure_is_cleared_when_the_repository_opens`: `Sticky`
stays, `Failed` becomes `Ready`, `Busy` and `Done` untouched.

**Minor 1 (non-UTF-8 argv) — already fixed by group 06, nothing to do.**
`cli::from_process` is the only argv reader in the crate: `env::args_os()`,
`env::var_os` for the socket, `to_string_lossy` for the prompt. No `env::args()`
anywhere under `crates/graft/src`. Covered by
`cli::tests::askpass_mode_needs_the_socket_and_exactly_one_argument`.

**Minor 4** — `remote_failed`'s `Cancelled` arm carries a comment saying why it is
unreachable through the pool and why it is kept.

**Minor 10** — the `chmod` spawn is replaced by `PermissionsExt` (`executable()`
in `tests/it/remote_by_key.rs`).

Not done, still open: Minors 2, 3, 5, 6, 7, 8, 9, 11. None are in the files this
round touched, and 2 and 3 are `graft-git`/`jobs::pool` behaviour changes rather
than app fixes.

### From the group 06 review (`docs/reviews/m4-06-credential-prompt-wiring.md`)

- **Minor m1, `Esc` order.** `update/credential.rs`: the job is cancelled *before*
  the ask is refused. The other order let git die of its own accord and be
  classified `AuthFailed`, so the line blamed the password for what the user
  chose. `update::credential::tests::esc_cancels_the_operation` pins the command
  order and the resulting status.
- **Doc overclaim in `jobs/askpass.rs`.** Corrected: the `Secret` zeroing starts
  at the `Enter` that builds the `Answer`. The characters before it live in the
  prompt's plain `TextField` `String`, which `update` clones on every event and
  drops without zeroing. The comment now says exactly that, and names the fix (a
  secret-backed field in `graft-ui`) as not-here.
- **`KNOWN_GAPS.md` row 467** can be closed — see Minor 1 above. Not edited here.

### Gates

Workspace-wide, in `target-g05fix`: `cargo fmt --check` clean;
`cargo clippy --all-targets --message-format=short -- -D warnings` clean, and the
same with `--features heavy-tests`; `cargo nextest run` **1799 passed, 22 skipped,
0 failed** (29.95 s); `cargo test --doc` clean. `remote_by_key`'s 8 tests now run
in 2.15 s together — the two waits were the slow part.

### Unverified

- The undo refusal is proved at the app layer: `Command::CancelJob` is the only
  route to a cancel, and no key path now emits one for `u`/`U`. I did **not**
  build a repository where an undo runs long enough to press `Esc` against —
  there is no hook git runs during a restore. The proof is that the cancel is
  never sent, not that sending it would be survived.
- The refusal line's wording is for the user's eye. No capture: it is one status
  line and no pane changed.

