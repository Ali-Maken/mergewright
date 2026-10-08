# Scout 18 — clone from the empty state, first-launch line

## Q1. Can graft-ops' remote path run with no repository?
**No, not as written.** `apply::remote::run(ctx: &Context, …)` (`crates/graft-ops/src/apply/remote.rs:76`) takes a whole `Context`. `Context` (`crates/graft-ops/src/apply.rs:130-154`) has `repo: &RepoHandle`, `journal: &mut Journal`, `scratch` and `worktree` as required fields, and `Context::open` (`apply.rs:158`) gets `scratch` from the journal's parent folder. What `run` actually reads is just `ctx.remote` (`remote.rs:82`) and `ctx.stop` (`remote.rs:121,136`). `remote_invocation` already takes `Option<&RemoteHooks>`, not the Context, so it can be tested without a repo (`remote.rs:57-62`).
- Running in a folder that isn't a repo: `GitInvocation::in_folder(folder)` (`crates/graft-git/src/run.rs:306`) clears the repo env vars. Its one caller today is `graft_ops::init` (`crates/graft-ops/src/init.rs:41`).
- **Contradiction:** the ticket's `StopFlag` type does not exist. Cancellation is `&dyn Stop` (trait at `graft-git/src/run.rs:112`; field `Context.stop`, `apply.rs:147`). `RemoteHooks` is `graft-ops/src/apply/remote.rs:28` (fields `progress`, `askpass`, `helper`).
- `remote.rs:run` is `pub(crate)`. A new `graft-ops::clone` module in the same crate can call it, but only if it has a `Context`.

## Q2. Does askpass or the credential prompt depend on an open repo or AppState?
- Server side, no repo needed: `AskPassServer::spawn(helper, handler)` (`graft-git/src/run/askpass.rs:95`).
- App side, yes. `ChannelAskPass::new(id: JobId, ask, waiting: Waiting, cancel: Cancel)` (`crates/graft/src/jobs/askpass.rs:126`) belongs to the job pool. The prompt is `update::credential::credential_needed(state: &mut AppState, id, …)` (`crates/graft/src/update/credential.rs:283`), which requires `state.operating == Some(id)`. `close_if_orphaned` (`credential.rs:302`) checks the same field. The app's `RemoteHooks` are built in `jobs/operate.rs:181-202` from an `Operation`, and `is_remote()` gates them.
- The empty state has none of this. `outside_loop` (`crates/graft/src/event_loop/places.rs:296-363`) is a synchronous loop: no `AppState`, no job pool, and wakes come only from `ChannelWakes`. Init runs inline, blocking (`places.rs:352`). `OutsideStep` (`crates/graft/src/empty_state.rs:67-79`) has no Clone variant. `Prompt` has only `Open` and `ConfirmInit` (`empty_state.rs:258-259`, `430`).
- Could not confirm whether `ChannelWakes` can carry job progress into `outside_loop`. I did not read it.

## Merged since the spec
- **11/11b:** the palette opens on the empty state (`empty_state.rs:96-133`, `palette_rows` :143). The `bar_keys` doorway is at `graft-ui/src/empty_state.rs:106-145`. The comment at `empty_state.rs:86` already reserves "the clone prompt joins the 'prompt open' check". `palette::opener` (`empty_state.rs:124`) is the hook for "palette opened → `done`".
- **Keys:** `empty_state.clone` is bound to `c` in `graft-ui/src/keymap/table.rs:493`, which matches §2.13 (`docs/design/02-keymap.md:557`). Two sweep tables list it as pending: `graft/src/empty_state/sweep_tests.rs:74` and `graft/src/update/tests/keymap_edge/sweep.rs:23`. `outside_key`/`list_key` (`empty_state.rs:273-300`) do not answer `c`.
- **12d:** every place lands on HEAD (`places.rs:111`), so `Exit::Open` → cursor on HEAD should come for free. Could not confirm that a test covers the open-after-outside path.
- **13j:** `HttpRemote::serve(bare, user, password)` (`graft-fixtures/src/remote/http.rs:194`), plus `url()` and `url_anonymous()` (:277, :282). `HungRemote` is in `remote/hung.rs`.
- **First-launch:** nothing exists yet. Grepping for `launches` and `first_launch` finds nothing in `crates/graft/src`.
