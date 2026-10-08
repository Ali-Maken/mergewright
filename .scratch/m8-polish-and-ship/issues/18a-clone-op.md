# 18a — graft-ops::clone, and the remote runner narrowed to what it reads

Status: done (builder 2026-10-08; gates green: fmt, clippy, nextest 5912/5912, doc)
Build with: **Opus** — a remote write outside any repo; the folder-cleanup and refusal-before-spawn rules are only
provable by running git, and the runner change touches every remote operation's path.
Blocked by: 08 (`init` / `GitInvocation::in_folder` in place)
Needs: none
Crate set: **graft-ops** only

## Read first
- `.scratch/m8-polish-and-ship/issues/18-rulings.md` R1, R2, R3
- `.scratch/m8-polish-and-ship/scouts/18-first-run.md` Q1
- `crates/graft-ops/MAP.md` `## apply` (`Context`, `RemoteHooks`), `## init`, `## bin (test-only)`
- `crates/graft-git/MAP.md` `run` (`GitInvocation::in_folder`, `run`, `Stop`, `STOP_WITHIN`), `remote`
  (`RemoteFailure`, `classify_failure`, `RemoteProgress`), `run/askpass`
- Source after the MAPs: `apply/remote.rs` (`run` :76, `remote_invocation` :57), `apply.rs:130-160`, `init.rs`

## Scope
In: `RemoteRun` + `Context::remote_run`; `remote::run` takes `RemoteRun`; new `clone` module (`CloneParams`,
`Cloned`, `CloneError` + `explain`, `name_from_url`, `clone`); tests; MAP.md.
Out: anything in graft or graft-ui; clone options (depth, branch, recursive); deleting a leftover folder (R3).

## Signatures
```rust
// graft_ops::apply::remote — crate-private
pub(crate) struct RemoteRun<'a> {
    pub(crate) hooks: Option<&'a RemoteHooks>,
    pub(crate) stop: &'a dyn graft_git::run::Stop,
}
impl Context<'_> { pub(crate) fn remote_run(&self) -> RemoteRun<'_>; }   // lifetimes as `Context` is declared
// `run(ctx: &Context, …)` becomes `run(at: RemoteRun<'_>, …)`: every other parameter and the return type unchanged.

// graft_ops::clone — pub; not an Operation, not journaled (no repo before it), as `init`
pub struct CloneParams { pub url: String, pub into: PathBuf }      // `into` absolute; its parent exists
pub struct Cloned { pub path: PathBuf, pub askpass: Option<std::io::Error> }   // askpass socket not cleaned up
pub enum CloneError {
    BadUrl,                                               // empty, starts with `-`, or no name from it
    TargetNotEmpty(PathBuf),
    Remote { failure: graft_git::remote::RemoteFailure, stderr_tail: Vec<String>, left: Option<PathBuf> },
    Stopped { left: Option<PathBuf> },
    Run { error: graft_git::run::RunError, left: Option<PathBuf> },
}
impl CloneError { pub fn explain(&self) -> String; }
pub fn name_from_url(url: &str) -> Option<String>;
pub fn clone(params: &CloneParams, hooks: Option<&RemoteHooks>, stop: &dyn graft_git::run::Stop)
    -> Result<Cloned, CloneError>;
```
- Order inside `clone`: `BadUrl` → `TargetNotEmpty` → **then** anything else (the stop is not polled before these).
- Plan: `git clone --progress -- <url> <into>` via `GitInvocation::in_folder(into.parent())`, run through
  `remote::run` with a `RemoteRun { hooks, stop }`. Not `stop_hooks_first` (remote lines are not).
- `left`: `Some(into)` when, after the failure or stop, `into` exists and either did not exist before or is now
  non-empty; else `None`.
- `explain`: `can't name a folder from this URL`; `<name> already exists here`; `clone failed: <the same
  per-RemoteFailure words the remote operations use>`; `clone stopped · nothing kept`; `clone: <RunError>`; each
  with ` · <path> was left behind` when `left` is `Some`. Not sanitised (the app sanitises, as for `ApplyError`).
- `name_from_url`: trim trailing whitespace and `/`; strip a trailing `/.git`, then `.git`; take the text after the
  last `/` or (scp form, no `://`) `:`; `None` when that is empty, `.`, `..`, or holds a control byte.

## Acceptance (each tick names its test, at tick time)
- [x] `name_from_url` table: `https://github.com/org/x.git`, `https://h/org/x/`, `https://h/org/x/.git`,
      `git@github.com:org/x.git`, `ssh://git@h:22/org/x.git`, `file:///tmp/r/x.git`, `/tmp/x`, `h:x.git` → `x`;
      `https://h/`, `` , `h:` → `None`. (`clone::tests::name_from_url_table`)
- [x] `file://` bare → `Ok`, `into` is a repo whose HEAD equals the bare's. (`tests/it/clone.rs::clones_a_file_bare`)
- [x] HTTP Basic auth (`HttpRemote::serve`) with `askpass_test_helper` → asked, `Ok`, `askpass: None`.
      (`tests/it/clone.rs::asks_and_clones_over_http`)
- [x] Unreachable / missing bare → `Remote { left: None }`, no `into` on disk. (`…::a_failed_clone_leaves_no_folder`)
- [x] `HungRemote`: stop raised once `into` exists (bounded wait on that condition, deadline stated) → `Stopped
      { left: None }`, `into` absent. Folder present → **stop, report** (R3). (`…::a_stopped_clone_leaves_no_folder`)
- [x] `into` non-empty → `TargetNotEmpty`, a counting `Stop` polled 0 times, `into`'s listing unchanged.
      (`…::a_non_empty_target_is_refused_before_git_runs`)
- [x] URL `-uhttps://x` → `BadUrl`, counting stop 0. (`…::a_url_like_an_option_is_refused`)
- [x] Every existing remote op unchanged: `tests/it/remote_ops.rs` green **unedited**. (that file, in NOTES)

## NOTES (builder, 2026-10-08)
- Scout used: `scouts/18-first-run.md` Q1 (no new scout; Q1 was exactly this seam). No compaction.
- Red first: all 10 new tests ran against `todo!()` stubs and failed, then went green.
- `remote::run`'s one caller: `apply/run.rs:149` (`run_plan`) → `ctx.remote_run()`. `RemoteRun` is `Copy`
  (clippy `needless_pass_by_value`). `mod remote` in `apply.rs` became `pub(crate)` so `clone` can reach it.
- `tests/it/remote_ops.rs` unedited (`git diff --quiet HEAD -- …` clean) and green in the full run.
- HTTP test runs in a child of the test binary with `GIT_CONFIG_NOSYSTEM=1` + an empty `GIT_CONFIG_GLOBAL`:
  this Mac's system gitconfig has `credential.helper=osxkeychain`, and a successful clone would otherwise store
  the test password in the user's keychain (a clone has no repo config to blank first). The parent asserts the
  child's `... ok` line, so a filter matching nothing cannot pass vacuously (checked by hand: `1 passed`).
- Stop test: the `Stop` itself flips once `into` exists (deadline 30 s, `MADE_WITHIN`); asserted that it did.
  git's own SIGTERM cleanup removed the folder — R3's "folder present" branch did not occur, no deletion added.
- `explain`'s remote words are `remote::short` (the per-`RemoteFailure` words without an entry/branch, which
  `remote::explain` needs and a clone has none of).
- **Question (rule 3):** an askpass socket close failure on a *failed or stopped* clone is dropped — the
  ticket's `CloneError` has no slot (commented at the drop in `clone.rs`). Add `askpass` to the error variants?
- Extra tests beyond the list: `clones_into_an_existing_empty_folder`,
  `a_failed_clone_into_an_empty_folder_leaves_it_empty`, `clone::tests::explain_words`; the BadUrl test also
  covers `""` and `https://h/`; the failed-clone test covers a missing bare and a closed loopback port.

## Touches
- `remote::run`'s callers (`rg -n 'remote::run\(' crates/graft-ops`) — each now passes `ctx.remote_run()`; list them.
- `graft::jobs::operate` builds `Context` only — unaffected (`rg -n remote_run crates/graft` = 0 hits).
- MAP.md: `## apply` (`RemoteRun`), new `## clone`.
- graft-ops `lib.rs` module list. Journal / undo: none (clone writes no entry). Keymap: none.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide.

## Size
One context, ~40k tokens.
