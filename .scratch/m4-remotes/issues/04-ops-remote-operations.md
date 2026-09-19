# ops-remote-operations: Fetch, Pull and Push as operations — plan, check, journal meaning, undo transparency, streaming + askpass through `apply`

Status: done
Blocked by: 02 (done)
Build with: Opus — taking the planner's recommendation: git writes against a remote plus journal semantics M5 builds on.
Crates: graft-ops, graft-core
Needs: `git-remote-and-askpass` — `graft_git::remote::{Remote, TrackedUpstream, list, upstream_of, RemoteFailure, classify_failure, RemoteProgress, parse_progress}`, `GitInvocation::{on_stderr_line, askpass}`, `run::askpass::{AskPass, AskPassServer, Secret}`. Tests use `graft_fixtures::remote::{LocalRemote, HttpRemote}` from `remote-fixtures` (graft-fixtures is already a dev-dependency).

## Read first

- `docs/design-notes/m4-remotes.md` — Promise; Interfaces (graft-core/graft-ops block); Hard choices 1, 3, 4, 5; Proposed changes 1, 3, 4; "Journal browser rows"; Genuinely risky groups (G3)
- `.scratch/m4-remotes/scout.md` §Operations (lines 20–35), §Journal (46–54), §Remote data (85–91)
- `crates/graft-ops/MAP.md` — whole (it is the contract this group extends: `operation`, `journal`, `undo`, `apply`, `facts`, `restore`)
- `crates/graft-core/MAP.md` §journal
- `crates/graft-git/MAP.md` §remote, §run (after G2 lands — read the MAP, not the source)
- `docs/design/06-undo-journal-design.md` §6.4 Push row (lines 109–128) — the design this group departs from, per the note's choice 1
- `.scratch/m3-working-tree-undo-and-stash/as-built.md` (whole, if under 200 lines; else the `apply`/`Context` section) — signature corrections later groups were told to read
- `PROMPT.md` §4 Remotes (lines 343–349)

## Scope

In — in this order (each piece a commit):

1. **`graft-core/src/journal.rs`**: `OperationKind += Fetch, Pull, Push` (serde names `fetch`/`pull`/`push` or whatever convention the enum already uses — match it). Nothing else in core.
2. **`graft-ops/src/operation.rs` + `operation/remote.rs`**: `FetchParams`, `PullParams`, `PushParams`, `Lease`; `Operation::{Fetch, Pull, Push}`; `Invocation.remote: bool` (default false; every existing constructor sets it — grep them); `is_remote()`; `preview()`. `RepoFacts += remotes, upstream` (of `HEAD`'s branch; `None` when detached/unborn). `Refusal += DetachedHead, NoRemote, NoSuchRemote { name }, NoUpstream { branch }` with `explain` text: `HEAD is detached — check out a branch first`, `no remote — add one in the shell: git remote add origin <url>`, `no remote named 'x'`, `main has no upstream — P pushes and sets one`.
   - `kind`: the obvious three. `describe`: `fetch all remotes` / `fetch origin`, `pull origin/main` (short tracking name), `push main → origin/main`, `push main → origin/main (set upstream)`, `force-push main → origin/main (with lease)`.
   - `check` (after the shared bare/unmerged/in-progress rules — **Fetch is allowed in a bare repo and during an in-progress operation**; Pull and Push follow the M3 rules): Fetch `Some(name)` not in `facts.remotes` → `NoSuchRemote`; `facts.remotes` empty → `NoRemote`. Pull: `head` not `Branch` → `DetachedHead`; `facts.upstream` `None` → `NoUpstream`; `params.upstream != facts.upstream` → `Refusal::UpstreamChanged` (new; the app's copy is stale — "press p again"). Push: `head` not `Branch` → `DetachedHead`; `params.remote` not in `facts.remotes` → `NoSuchRemote`; `params.branch != head branch` → `Refusal::BranchChanged` (new). Nothing about ahead/behind — git decides, the failure explains.
   - `plan` (`remote: true` on every remote invocation; `--progress` on every one; exact args, verify in a throwaway repo): Fetch → `fetch --progress [--prune] (--all | <remote>)`. Pull → two invocations: `fetch --progress <remote>` then `merge --ff-only --no-edit <tracking_ref>` (the second `remote: false`). Push → `push --progress [-u] [--force-with-lease=<remote_branch>:<expected> --force-if-includes] <remote> refs/heads/<branch>:<remote_branch>` (`remote_branch` is a full `refs/heads/x`; `-u` iff `set_upstream`).
   - `touches`: Fetch `{ refs: [], index: false, worktree: false }` (remote-tracking refs are not recorded — 50k-ref repos; the browser row says `remote-tracking refs`); Pull `{ refs: [branch], index: true, worktree: true }` — **the tracking ref is not in it** (undo must never move a tracking ref); Push `{ refs: [upstream tracking ref if known, else none], index: false, worktree: false }` (recorded for the browser's `origin/main a1b2c3d → 9f8e7d6`, never for undo).
3. **`graft-ops/src/journal.rs`**: `undoable_kind(kind) -> bool` (false for `Fetch`, `Push`); `next_undo` skips non-undoable kinds; `next_redo`'s "no in-force operation newer than it" counts only undoable kinds (a fetch after an undo must not empty redo). `in_force` unchanged. **`graft-ops/src/undo.rs`**: `inverse` on an entry of kind `Fetch`/`Push` → `Err(NotInvertible::NotUndoable(kind))` before any generic step; `plan_undo`/`plan_redo` unchanged in signature.
4. **`graft-ops/src/apply.rs`**: `RemoteHooks`, `Context.remote: Option<RemoteHooks>`. For an invocation with `remote: true`: build the `GitInvocation` as today, then `.on_stderr_line(sink)` where the sink (a) collects every segment into an `Arc<Mutex<Vec<Vec<u8>>>>` and (b) when `hooks` is `Some`, calls `parse_progress` and, at most every 100 ms, `hooks.progress(p)`; when `hooks` is `Some`, spawn an `AskPassServer::spawn(hooks.helper.clone(), hooks.askpass.clone())` for the invocation's lifetime and `.askpass(&server)`; `close()` it after the child exits (its `io::Error` joins the after-state's `unread`-style reporting — never swallowed, never fatal). On a non-zero exit: `failure = if stopped { Cancelled } else { classify_failure(joined segments) }` → the entry is journaled `Aborted(failure short text)` exactly as `ApplyError::Git` does today, and `apply` returns `ApplyError::Remote { entry, failure, stderr_tail }` (last ≤ 8 segments). `Applied.refresh` is `Everything` for any `is_remote()` operation. `ApplyError::explain` words each `RemoteFailure` with the operation's names (text in the note's "Failure line" list; `Cancelled` → `<describe> cancelled`). Existing paths byte-for-byte unchanged when `remote` is false (guard test: a `commit` through `apply` with `Context.remote = Some(hooks)` never spawns an `AskPassServer` — count via a test `AskPass` handler that panics if constructed… simpler: assert no `GRAFT_ASKPASS_SOCKET` in the recorded env by testing the helper `remote_invocation(&Invocation, &Context) -> bool`).
5. **`graft-ops/src/facts.rs`**: `read_facts` gains `remotes` (`remote::list`) and `upstream` (`upstream_of` on the head branch, `None` when not a branch); `FactsError += Remote(RemoteError)`.
6. **Tests** (`tests/remote_ops.rs`, fast tier, real git, `LocalRemote`; HTTP only where auth matters): see Acceptance. Every claim about git behaviour is proved by reading the repository back (`for-each-ref`, `rev-parse`, file bytes, `git config`), never from return values alone (the M3 group-03 standard).
7. MAP.md lines (graft-ops, graft-core); `docs/design/06-undo-journal-design.md` §6.4 Push row edited to the as-built rule **only if** the conductor confirms the user accepted Proposed change 1; otherwise leave it and say so in NOTES.

Out: the app (keys, boxes, prompt); force-push *without* a lease (never); `pull --rebase`/merge (M5); fetching tags specially (`fetch` defaults suffice); `remote add/rename/remove`.

**Verify in a throwaway repo before coding, results in NOTES:** (a) `merge --ff-only <tracking_ref>` on a dirty tree whose files the ff changes → refuses, tree byte-identical; on diverged → `Not possible to fast-forward`, `HEAD` unchanged; (b) `push --force-with-lease=refs/heads/main:<old> --force-if-includes` after a local amend (old tip in the branch's reflog) succeeds, and after a remote-side move since fetch is refused with `stale info`; (c) `fetch --all --prune` removes a tracking ref whose remote branch was deleted; (d) `push -u origin refs/heads/x:refs/heads/x` writes `branch.x.remote` and `branch.x.merge`; (e) `fetch` on a bare repo works.

## Signatures

```rust
// crates/graft-core/src/journal.rs
pub enum OperationKind { …, Fetch, Pull, Push, Undo, Redo }
// crates/graft-ops/src/operation.rs
pub enum Operation { …, Fetch(FetchParams), Pull(PullParams), Push(PushParams) }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct FetchParams { pub remote: Option<String>, pub prune: bool }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct PullParams { pub branch: String, pub upstream: TrackedUpstream }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct PushParams { pub branch: String, pub remote: String, pub remote_branch: String, pub set_upstream: bool, pub lease: Option<Lease> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Lease { pub expected: Oid }
impl Operation { pub fn is_remote(&self) -> bool; pub fn preview(&self) -> Option<Vec<String>>; }
pub struct Invocation { pub args: Vec<GitArg>, pub stdin: Option<Vec<u8>>, pub remote: bool }
pub struct RepoFacts { …, pub remotes: Vec<Remote>, pub upstream: Option<TrackedUpstream> }
pub enum Refusal { …, DetachedHead, NoRemote, NoSuchRemote { name: String }, NoUpstream { branch: String }, UpstreamChanged, BranchChanged }
// crates/graft-ops/src/journal.rs
pub fn undoable_kind(kind: OperationKind) -> bool;
// crates/graft-ops/src/apply.rs
pub struct RemoteHooks { pub progress: Box<dyn Fn(RemoteProgress) + Send + Sync>, pub askpass: Arc<dyn AskPass>, pub helper: PathBuf }
pub struct Context { …, pub remote: Option<RemoteHooks> }
pub enum ApplyError { …, Remote { entry: Box<JournalEntry>, failure: RemoteFailure, stderr_tail: Vec<String> } }
// crates/graft-ops/src/facts.rs
pub enum FactsError { …, Remote(RemoteError) }
```

## Acceptance

Every tick names its verifying test inline, at tick time (ADR-0035). All through `apply` with `Context.remote = None` unless stated, against `LocalRemote`.

- [x] Fetch after `commit_on_remote` moves `refs/remotes/origin/main` to the bare's tip; journal entry kind `Fetch`, `Committed`, `refs_before`/`refs_after` empty, `commands` = the one fetch line; `Applied.refresh == Everything` — `fetch_moves_tracking_ref_and_journals`
- [x] Fetch with `prune` drops the tracking ref of a branch deleted on the remote — `fetch_prunes`
- [x] Fetch in a bare repo is allowed — `fetch_allowed_in_bare`
- [x] Pull fast-forwards `main`, updates the worktree file bytes, journals `{refs:[refs/heads/main], index, worktree}` with a backup; `u` (through `graft_ops::undo::undo`) moves `main` back and restores the file bytes **and leaves `refs/remotes/origin/main` at the fetched tip**; `U` forwards again — `pull_fast_forwards_then_undo_keeps_tracking_ref`
- [x] Pull on diverged branches: entry `Aborted`, `ApplyError::Remote { failure: NotFastForward }`, `main` unchanged, tracking ref advanced — `pull_diverged_is_refused_after_fetching`
- [x] Pull with an uncommitted change to a file the ff touches: `WouldOverwrite { paths: [that file] }`, file bytes unchanged — `pull_dirty_file_refused_bytes_unchanged`
- [x] Pull with no upstream → `Refused(NoUpstream)`, nothing journaled; detached → `Refused(DetachedHead)` — `pull_refusals`
- [x] Push after `commit_locally` moves the bare's `main`; entry records `refs/remotes/origin/main` before/after; `next_undo` over the journal returns the **commit** entry, not the push; `undo` then moves local `main` back while the bare's `main` and the tracking ref stay — `push_then_undo_touches_only_local`
- [x] Push with `set_upstream` on a new branch writes `branch.x.remote=origin` and `branch.x.merge=refs/heads/x` — `push_sets_upstream`
- [x] Push after `commit_on_remote` (no fetch) → `NonFastForward`, entry `Aborted`, bare unchanged — `push_non_fast_forward_classified`
- [x] Lease push after a local amend succeeds; lease push after a remote-side move since the last fetch → `StaleInfo`, bare unchanged — `lease_push_succeeds_after_amend`, `lease_push_refused_when_stale`
- [x] `next_redo` survives a fetch: commit → undo → fetch → `next_redo` is still the undo — `fetch_does_not_empty_redo`
- [x] `inverse` on a Fetch or Push entry → `NotUndoable(kind)` — `remote_entries_are_not_invertible` (unit, `undo::tests`)
- [x] Wrong password over `HttpRemote` with `Context.remote = None` → `AuthFailed { target }`, `Aborted` — `http_wrong_password_is_auth_failed`; with `Context.remote = Some(hooks)` whose `AskPass` answers the right password → push succeeds and `hooks.progress` was called ≥ 1 time — `http_askpass_answers_and_progress_flows`; the journal file's bytes do not contain the password — same test, `assert!(!journal_bytes.windows(n).any(..))`
- [x] Cancel mid-fetch (a `Stop` that flips after the first progress segment) → `ApplyError::Remote { failure: Cancelled }`, entry `Aborted("cancelled")`, no `*.lock` under `.git` — `cancel_mid_fetch_is_clean`
- [x] `preview()` on Push equals `plan(&facts)`'s command lines — `preview_matches_plan` (unit, `operation::remote::tests`)
- [x] `ApplyError::explain` for every `RemoteFailure` variant contains the note's words (rejected/authentication failed/diverged/would overwrite/unreachable/stale) — `explain_names_each_failure` (unit, `apply/remote/tests.rs`)
- [x] MAP.md (ops, core) updated — docs, no test; DESIGN §6.4 **left unchanged**: no confirmation that the user accepted Proposed change 1 reached this builder

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` · `cargo nextest run -p graft-ops -p graft-core` · `cargo test --doc -p graft-ops`. The whole workspace once at the end (`OperationKind` is matched exhaustively in the app — a new variant may break `graft`; if it does, add the minimal arm that keeps existing behaviour and **name it in NOTES**, because `graft` is not in this group's crate set — commit at the seam and report `blocked: needs graft` instead if the arm is more than a `match` line).

## Size

One context. Brownfield in `operation.rs`/`apply.rs`/`journal.rs`/`undo.rs` — send a scout for `apply.rs`'s invocation loop and error mapping before editing; report whether you did. Stop seam: after piece 3 (model + journal rule, no `apply` change) with pieces 4–6 in a handoff note.

## NOTES

- Two builders: the first stopped by a rate limit after pieces 1–3 and most of 4–6 (uncommitted); the second finished. No compaction.
- Throwaway-repo checks (a)–(e) are held by integration tests that read the repository back rather than a separate manual run: (a) `pull_dirty_file_refused_bytes_unchanged`, `pull_diverged_is_refused_after_fetching`; (b) `lease_push_succeeds_after_amend`, `lease_push_refused_when_stale`; (c) `fetch_prunes`; (d) `push_sets_upstream` (reads `git config`); (e) `fetch_allowed_in_bare`.
- **Outside crate set:** one match arm in `crates/graft/src/jobs/operate.rs` `refresh_after_failure` — `ApplyError::Remote { .. }` joins the `Everything` arm (a failed pull has already fetched). Needed for the workspace to compile; the ticket's Gates allow it.
- Deviation: `remote_invocation` takes `(&Invocation, Option<&RemoteHooks>)`, not `&Context`, so the guard is unit-tested without a repo (`only_a_remote_invocation_with_hooks_gets_them`); private, no public change.
- `src/bin/askpass_test_helper.rs`: a test-only binary in graft-ops, so `tests/it/remote_ops.rs` can drive git → helper → `AskPass` end to end before 05b gives Graft its askpass mode. It is built with the crate (not `cfg(test)`-gated — Cargo can't gate bins). 05b may want to delete it and point the tests at Graft's binary.
- Auth-failure line uses the operation's own retry key (`f`/`p`/`P`), the note shows `P` for the push example.
- `a_local_operation_with_hooks_runs_as_before` alone does not prove no askpass server spawns for a local invocation (a commit never prompts); the unit guard test above does.

## Review fixes (docs/reviews/m4-04-ops-remote-operations.md)

Crates: graft-ops, graft-git (graft-core unchanged). Scout: one Haiku `Explore` for `run::askpass::serve`'s wait loop; the rest read directly (files named by the review). No compaction.

- **M1 lease — cause.** `check_push` refuses `Refusal::LeaseStale { remote_side }` before git runs unless `lease.expected` equals `HEAD`'s upstream `tracking_tip` for the pushed remote/branch; `--force-if-includes` dropped from `plan_push`, `Lease` doc and MAP. Tests: `a_lease_must_match_the_fetched_tracking_tip` (unit: equal, fetched since, never fetched, no upstream, other branch), `lease_push_refused_before_git_when_fetched_since` (integration: their commit fetched after `expected` was read → `Refused(LeaseStale)`, journal empty, bare still at theirs). The unfetched remote-side move stays git's CAS: `lease_push_refused_when_stale` (unchanged, still `StaleInfo`). **Cost:** a lease push whose target is not `HEAD`'s upstream is refused (no local tip to compare) — e.g. `P -u` on a new branch rejected because the remote already has that name, then the lease box. Safe side; the app group should know. Design note lines 68/143 still say `--force-if-includes` — conductor's to edit.
- **M2 read_facts — cause.** `read_facts(repo, git, stop, with_remotes)`; `apply` passes `op.is_remote()`; local ops get empty `remotes`/`None` upstream. Test: `a_non_utf8_remote_url_does_not_block_a_commit` (stage+commit succeed with `url = /srv/\xff\xfe.git` in config; a fetch in the same repo still fails `ApplyError::Facts`, proving the config really is unreadable).
- **M3 up-to-date pull — cause.** Journaled, transparent: new `journal::undoable(entry)` = `undoable_kind` and not a `Committed` pull with every after == before; used by `next_undo`, `next_redo`, `undo::inverse` (`NotUndoable(Pull)`). Chosen over not journaling because the pull's fetch ran and may have moved tracking refs, which the browser should show. Tests: `a_pull_that_moved_nothing_is_transparent` (unit, also redo survives it), `an_up_to_date_pull_is_transparent_to_undo` (integration: commit → pull → `next_undo` is the commit, one `u` reverses the commit).
- **M4 Names — cause.** `RemoteNames` (pub, `operation`) built from params by `Operation::remote_names`; carried `run_plan` → `PlanFailure` → `ApplyError::Remote { names: Box<RemoteNames> }` (boxed: `result_large_err`); description parsing deleted. Test: `names_survive_arrows_and_slashes` (`a→b` branch, `team/origin` remote — the old parse named `team`); `explain_names_each_failure` now builds entries from real `Operation`s.
- **askpass poll — cause.** `serve` waits with one blocking `recv` on a per-exchange channel; shutdown sends `Wake::Stop` through a `Shared.waiting` slot registered under the same lock `stop` is checked under; a handler that unwinds sends `Wake::Panicked` from `Reply`'s `Drop`. `SHUTDOWN_POLL` gone. Tests: existing `drop_unblocks_waiting_client` (drop < 1 s with a blocked handler) and the rest of `run::askpass::tests`; new `a_panicking_handler_fails_its_helper_and_the_server_carries_on` (passes on the old code too — a guard for the new wake path, not a red-first test).
- **Minors taken** (files edited anyway): 1 askpass spawn failure now reads `could not run git: askpass: the credential prompt's socket could not open: …` (no test: needs a failing bind); 3 `forward` is a blocking `for` over the receiver, ended by dropping `git` (the sink's sender) inside the scope — `FORWARD_POLL` gone; covered by `http_askpass_answers_and_progress_flows` and `cancel_mid_fetch_is_clean` (progress still arrives, run returns); 6 the plan constructor renamed `remote_step`; 7 `Refusal::explain` matches every variant itself (`#[expect(clippy::too_many_lines)]` with reason), `operation::remote::explain` deleted. **Not taken:** 2 (throttle drops last segment), 4 (test bin, 05b), 5 (cancel test timing), 8 (segment retention doc).
- Red-first: M1, M2, M3, M4 tests fail against the old code by construction (old `check_push` ran git → `Remote(StaleInfo)`; old `read_facts` erred on the config; old `next_undo` returned the pull; old parse gave `team unreachable`). Not re-run red.
- Gates, workspace-wide: fmt ok, clippy `--all-targets -D warnings` ok (also with `--features heavy-tests`), nextest 1746/1746, `cargo test --doc` ok.
