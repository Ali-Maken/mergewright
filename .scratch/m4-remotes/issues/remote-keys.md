# remote-keys: `f` / `p` / `P`, the push box, `Esc` cancels a running write, live progress, plain-English failures, remote facts in `AppState`

Status: draft
Build with: <!-- recommendation: Opus — brownfield in the app crate's key routing and job plumbing, the first interaction that moves a remote, cancel wiring through the pool. Conductor sets after the scout. -->
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

- [ ] `f` after `commit_on_remote`: tracking ref moves, badge reads `main ↓1` (via the refs product), status ends `fetched`, journal has a `Fetch` entry — `remote_by_key::fetch_key_moves_tracking_ref`
- [ ] `p`: `main` == tracking tip, file bytes updated; `u` → `main` back, bytes back, tracking ref unchanged; `U` → forward — `remote_by_key::pull_then_undo_redo`
- [ ] `p` with no upstream / detached HEAD: no command, status text per the note — `update::remote::tests::pull_refusals_are_words_not_commands`
- [ ] `P` opens the box with the note's exact title/consequence/command for: upstream present; no upstream (sets upstream); lease after rejection — `update::remote::tests::push_box_text_table` (3 cases)
- [ ] `P` → `Enter`: bare's `main` == local; `u` undoes the **commit** locally, status ends `origin/main still has it`, bare unchanged; `U` restores — `remote_by_key::push_then_undo_is_local_only`
- [ ] `P` after `commit_on_remote` → `Sticky` "push rejected: origin/main has commits you don't · p to pull first, or P to force with lease"; `P` → lease box → `Enter` → (after a local amend) bare's `main` == local; without the amend (stale) → `Sticky` names `stale lease` and bare unchanged — `remote_by_key::rejected_then_lease_push`, `remote_by_key::stale_lease_refused`
- [ ] `Esc` during `f` against `HungRemote`: `operating` clears within 5 s, journal entry `Aborted` with `cancelled`, status `fetch all remotes cancelled`, no `.git/**/*.lock`, `HungRemote::connections() == 1` — `remote_by_key::esc_cancels_hung_fetch`
- [ ] `Esc` during a local `commit` (a pre-commit hook `sleep 5`) cancels it the same way — `remote_by_key::esc_cancels_local_write`
- [ ] `Status::Busy` text during a fetch contained a `RemoteProgress.brief` at least once (record every status the harness saw) — `remote_by_key::progress_reaches_the_status_line`
- [ ] `u`, `U`, `Ctrl+Z`, `f`, `p`, `P` typed while the push box is open change nothing but the box (M3 06 C1's shape) — `update::remote::tests::box_is_modal`
- [ ] Journal browser rows for Push/Fetch/Pull read exactly as the note says, no `← u` on Push/Fetch — `view::journal::tests::remote_rows`
- [ ] Radar `RepoFacts.remotes` is the real list — `view::radar::tests::remotes_come_from_facts`
- [ ] A `Product::Remotes` under a stale id is dropped — `update::tests::stale_remotes_dropped`
- [ ] MAP.md, ARCHITECTURE.md updated

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` (M3 06 C2: the heavy tier must still compile) · `cargo nextest run -p graft` · `cargo test --doc -p graft`.

## Size

One context; brownfield — **scout first** (sonnet, ≤ 1 page each): `update/keys.rs` routing order and free keys, `jobs/pool.rs` cancel semantics, `update/operate.rs::launch`/`operated`, `tests/it/undo_by_key.rs` harness helpers. Report whether you did. Stop seam: after piece 3 (keys + cancel, no progress/failure wording) with 4–7 in a handoff note. Past 150 turns: stop, `partial`, handoff.
