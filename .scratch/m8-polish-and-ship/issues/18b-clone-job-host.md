# 18b — the empty state hosts a clone job and the credential prompt

Status: done (builder 2026-10-08)
Build with: **Opus** — first job pool and first modal prompt in a place that had neither; the credential logic is
shared with every remote write, so a slip there breaks M4's walk.
Blocked by: 18a, 18d (18d edits `places.rs` and `empty_state.rs` first)
Needs: `graft_ops::clone::{CloneParams, Cloned, CloneError, clone, name_from_url}` (18a)
Crate set: **graft** only (`jobs`, `event_loop::places`, `empty_state`, `update::credential`)

## Read first
- `.scratch/m8-polish-and-ship/issues/18-rulings.md` R4, R5, R6
- `.scratch/m8-polish-and-ship/scouts/18-first-run.md` Q2
- `crates/graft/MAP.md` `## jobs` (`operate`, `askpass`, `JobRunner::{submit, cancel, answer_credential, shutdown}`,
  `SHUTDOWN_GRACE`), `## update::credential`, `## event_loop` (`Wake`, `ChannelWakes`, `Ran`, `places::drive`),
  `lib.rs` `empty_state`
- `crates/graft-ops/MAP.md` `## clone` (18a)
- Source after the MAPs: `places.rs:296-363`, `empty_state.rs:67-79, 258-300`, `jobs/operate.rs:181-202`,
  `jobs/askpass.rs:126`, `update/credential.rs:283-310`

## Check first (NOTES, with lines, before writing)
1. Does `outside_loop`'s `ChannelWakes` carry job `Wake`s (`Progress::*`, `Core::Job*`)? Quote `ChannelWakes::new`
   and how `run_repo` wires its `JobRunner` events. Not wired → wire it the same way.
2. What `operate.rs:181-202` reads to build `RemoteHooks`; the extracted function's exact signature.

## Scope
In: `jobs::clone` + `JobRunner::clone_repo`; one hooks builder for operate and clone; lazy `JobRunner` in the
outside loop, `Ran` carrying its `Shutdown`; job wakes routed to `OutsideState`; credential core (R5) and the
outside host's slot; `Exit::Open` on done with an optional notice carried into the next `Arrival`; MAP.md.
Out: the `c` key, the prompt's keys and drawing, bar words, the walk (18c).

## Signatures
```rust
// graft::jobs::clone (new, pub(crate))
pub(crate) struct CloneRequest { pub url: String, pub into: PathBuf }
pub(crate) enum CloneAnswer { Done { path: PathBuf, notice: Option<String> },   // notice: askpass close failure
    Ended { words: String } }                                                   // CloneError::explain, sanitised
impl JobRunner { pub(crate) fn clone_repo(&self, req: CloneRequest) -> JobId; }  // stoppable; job's Cancel = Stop
// answered as Product::Cloned(CloneAnswer); progress Progress::RemoteProgress; asks Progress::CredentialNeeded
// graft::jobs — operate.rs:181-202's body, shared (args as check-first 2 finds them, minus the Operation)
pub(super) fn remote_hooks(/* … */) -> Option<RemoteHooks>;

// graft::update::credential — host-agnostic core (R5); today's four fns keep their signatures, as wrappers
pub(crate) enum PromptKey { Kept, Send(JobId, Answer), Cancel(JobId), Quit }
pub(crate) fn open_prompt(slot: &mut Option<CredentialPrompt>, running: Option<JobId>, cancelling: bool,
    id: JobId, git_prompt: &str, pulse_seed: bool);
pub(crate) fn prompt_key(slot: &mut Option<CredentialPrompt>, key: KeyPress) -> PromptKey;
pub(crate) fn prompt_paste(slot: &mut Option<CredentialPrompt>, text: &str) -> bool;
pub(crate) fn close_orphaned(slot: &mut Option<CredentialPrompt>, running: Option<JobId>);

// graft::empty_state
pub(crate) enum Prompt { Open(TextField), ConfirmInit, Clone(CloneState) }
pub(crate) struct CloneState { pub url: TextField, pub run: CloneRun }
pub(crate) enum CloneRun { Typing, Running { job: JobId, into: PathBuf, progress: Option<RemoteProgress> },
    Ended { words: String } }
// OutsideState gains `credential: Option<CredentialPrompt>`
// OutsideStep gains:
StartClone(CloneRequest), StopClone(JobId), Credential(JobId, Option<Answer>), Cloned { path: PathBuf, notice: Option<String> }
pub(crate) fn clone_request(folder: &Path, url: &str) -> Result<CloneRequest, String>; // name_from_url; words R6
pub(crate) fn clone_started(state: &mut OutsideState, job: JobId, into: PathBuf);
pub(crate) fn outside_job(state: &mut OutsideState, wake: &Wake, pulse_seed: bool) -> OutsideStep; // Stay | Cloned
// Arrival gains `notice: Option<String>`: one status notice on the next place's first frame; None elsewhere.
```
Loop: `StartClone` → start the pool if none, `clone_repo`, `clone_started`. `StopClone` → `JobRunner::cancel`.
`Credential(id, a)` → `answer_credential` (+ `cancel` when `None`). `Cloned` → `Exit::Open(path)` after the pool's
shutdown (job done: immediate), notice handed to `drive`. Job ended with the prompt open → `close_orphaned`.

## Acceptance (each tick names its test, at tick time)
- [x] `clone_repo` on a `file://` bare → `Product::Cloned(Done)`. (`jobs::clone::tests::clones_a_file_bare`)
- [x] HTTP Basic: `Progress::CredentialNeeded` arrives; `answer_credential` → `Done`.
      (`jobs::clone::tests::asks_through_the_pool_and_clones`)
- [x] `HungRemote` + `cancel` → `Ended { "clone stopped · nothing kept" }`, folder absent.
      (`jobs::clone::tests::cancel_stops_it_and_leaves_nothing`)
- [x] `outside_job`: progress updates `Running.progress`; ask opens `state.credential`; ended closes it and sets
      `Ended`; done → `OutsideStep::Cloned`. (`empty_state::tests::clone_job_events_table`)
- [x] Credential core: M4/M5 credential tests and `m4_walk` credential scenarios green **unedited** (names in NOTES);
      `prompt_key` table (`⏎` Send, `esc` Cancel, `⌃U` clears, `⌃C` Quit, others Kept).
      (`update::credential::tests::prompt_key_table`)
- [x] `Ctrl+C` while cloning → pool shutdown cancels it within `SHUTDOWN_GRACE`, no folder, exit.
      (`event_loop::places::tests::quitting_mid_clone_stops_it`)
- [x] Notice carried: askpass close failure → next place's first frame has it; any other `Exit::Open` → none.
      (`event_loop::places::tests::a_clone_notice_reaches_the_next_place_once`)
- [x] Repo screen remote writes unchanged: `jobs::operate` tests green unedited. (names in NOTES)

## Touches
- `jobs/operate.rs` hooks → shared builder: every remote op (fetch/pull/push) — `m4_walk` green.
- `update::credential` wrappers: `AppState` callers (`rg -n 'credential_(needed|key|paste)|close_if_orphaned'`).
- `Ran`, `Pool::stop`, exit lines: clone is stoppable, so no waiting line (`writes_running` excludes it — test above).
- `empty_state::sweep_tests`: no new key here; unchanged. Keymap: none (18c binds `c`).
- `view` / `draw_modal_boxes`: not touched (outside draws its own, 18c). MAP.md graft.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide.

## Size
One context, ~65k tokens.

## NOTES (builder, 2026-10-08)

### Check first
1. **Not wired before this ticket.** `ChannelWakes::new` (`event_loop/mod.rs:194`) returns `(Self, Sender<Wake>)`,
   and `impl JobEvents for Sender<Wake>` (`mod.rs:231-238`) sends every job `AppEvent` as `Wake::Event(event)` — so
   the outside loop's channel *can* carry `Progress::*` / `Core::Job*`. `run_repo` wires it as
   `let (mut wakes, sender) = ChannelWakes::new();` (`startup.rs:293`) then
   `JobRunner::with_default_size(sender.clone(), repos)?` (`startup.rs:302`). `run_outside` (`places.rs:246-249`)
   moved its only `sender` into `input::spawn_reader`: no clone kept for a pool. Wired the same way: a clone kept
   for the lazy pool.
2. `operate.rs:181` was `fn remote_hooks(op: &Operation, hooks: Hooks) -> Option<RemoteHooks>`; it reads
   `op.is_remote()` and the three `Hooks` fields (`progress`, `askpass`, `helper`) only. Extracted as
   `pub(super) fn remote_hooks(hooks: Hooks) -> Option<RemoteHooks>`; `run_operation` keeps the `is_remote` gate.
   The runner's closure that *builds* `Hooks` (`runner.rs:944-978`: progress sink, `ChannelAskPass`) moved with it
   to `operate::job_hooks`, so clone and operate share both halves.

### Unedited tests, green (workspace nextest 2026-10-08: 5959 passed, 12 skipped; `git diff HEAD` on each file
shows insertions only / no change)
- `update::credential::tests` (only appended to): `is_masked_prompt_table`,
  `the_title_names_the_host_when_the_prompt_has_a_url`, `gits_words_are_cleaned_before_they_are_kept`,
  `prompt_is_modal_and_swallows_globals`, `ctrl_u_clears_the_field`, `enter_answers_and_clears`,
  `an_empty_answer_is_still_an_answer`, `esc_cancels_the_operation`,
  `editing_keys_work_and_paste_lands_in_the_field`, `secret_never_in_debug`,
  `a_second_ask_replaces_the_first_and_a_stale_one_is_ignored`, `a_prompt_whose_write_has_ended_closes_itself`.
- `tests/it/credential_by_key.rs` (whole file), `m5_walk` credential steps, and `m4_walk` `credentials()`:
  `wrong_password_then_right`, `password_prompt_esc_cancels`, `username_then_password` — files untouched.
- `jobs::operate::tests` (file untouched): `a_resume_git_fails_answers_failed`,
  `a_failure_carries_where_graft_ops_says_the_edits_are`, `a_failed_checkout_that_moved_head_rereads_everything`,
  `answer_maps_staging_not_kept_alone_to_its_own_outcome`, `answer_paused_lands_as_conflicted`.

### Deviations from the signatures (say if wrong)
- `CloneRun::Running.progress` is `Option<String>`, not `Option<RemoteProgress>`: the wake carries
  `Progress::RemoteProgress { brief: String }`, nothing richer.
- `outside_job(state, &wake)` has no `pulse_seed`: `Progress::CredentialNeeded` carries its own.
- New `jobs::Outcome::Answered(Product)`: `pool::run_job` turned *every* product of a cancelled job into
  `JobCancelled`, which would drop a stopped clone's "… was left behind" words. `Answered` is delivered even when
  cancelled; only the clone uses it.
- The JobRunner needs a `RepoStore`: the outside pool gets `RepoStore::new(folder, None)`, never opened (the clone
  job reads none).
- Credential keys are routed in `outside_edge` here (modal over every key while `state.credential` is open), since
  the prompt the host owns must take its keys; 18c owns the *clone* prompt's keys and drawing, not this.
- `cancelling` is passed `false` for outside asks: the empty state has no `remote.cancelling`; a `StopClone`
  cancels and the ended job closes the question (`close_orphaned`).
- `drive`'s Outside arm moved to `places::leave_outside` (clippy `too_many_lines`); the notice hand-off
  (`carried.notice = notice`) there is checked by construction and by `a_clone_notice_reaches_the_next_place_once`
  driving `Carried::repo_arrival` + `startup::arrive` directly, not through `drive` on a terminal.
- `jobs::clone::tests::asks_through_the_pool_and_clones` needs the graft binary built beside `deps/` (askpass
  helper); workspace nextest builds it. It runs the real half in a child with `GIT_CONFIG_NOSYSTEM=1` and an
  empty `GIT_CONFIG_GLOBAL`, so the user's keychain helper never answers or stores.
- Items only 18c uses carry `cfg_attr(not(test), expect(dead_code))`: `CloneRun::Typing`,
  `OutsideStep::StartClone`/`StopClone`, `clone_request`.

### Touches (how checked)
- `jobs/operate.rs` + `runner.rs` hooks → shared builder: operate tests, `m4_walk`, `credential_by_key` green.
- `update::credential` wrappers: callers by `rg 'credential_(needed|key|paste)|close_if_orphaned'` — signatures
  unchanged, so callers untouched; existing tests green.
- `Product` gained `Cloned`: exhaustive matches in `event_loop/label.rs`, `update/mod.rs` (`answer`,
  `took_fact`) — compiler-found, all updated.
- `OutsideState` gained `credential`: literals in `empty_state/{tests,sweep_tests,palette_tests}.rs` +1 line each.
- `Ran` / `Pool::stop_to`: no waiting line for a clone — `quitting_mid_clone_stops_it` asserts stderr empty.
- `startup::run_repo` now calls `arrive` (extracted, behaviour same): every repo-screen test green.
- Keymap, `view`, `draw_modal_boxes`, captures: not touched (18c).

### Unverified
- `drive` / `run_outside` glue under a real terminal (no pty walk; 18c's walk covers it).
- Compacted once mid-ticket (context); nothing left over — all items above done.
