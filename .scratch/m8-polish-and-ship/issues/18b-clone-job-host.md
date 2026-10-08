# 18b — the empty state hosts a clone job and the credential prompt

Status: ready (conductor 2026-10-08)
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
- [ ] `clone_repo` on a `file://` bare → `Product::Cloned(Done)`. (`jobs::clone::tests::clones_a_file_bare`)
- [ ] HTTP Basic: `Progress::CredentialNeeded` arrives; `answer_credential` → `Done`.
      (`jobs::clone::tests::asks_through_the_pool_and_clones`)
- [ ] `HungRemote` + `cancel` → `Ended { "clone stopped · nothing kept" }`, folder absent.
      (`jobs::clone::tests::cancel_stops_it_and_leaves_nothing`)
- [ ] `outside_job`: progress updates `Running.progress`; ask opens `state.credential`; ended closes it and sets
      `Ended`; done → `OutsideStep::Cloned`. (`empty_state::tests::clone_job_events_table`)
- [ ] Credential core: M4/M5 credential tests and `m4_walk` credential scenarios green **unedited** (names in NOTES);
      `prompt_key` table (`⏎` Send, `esc` Cancel, `⌃U` clears, `⌃C` Quit, others Kept).
      (`update::credential::tests::prompt_key_table`)
- [ ] `Ctrl+C` while cloning → pool shutdown cancels it within `SHUTDOWN_GRACE`, no folder, exit.
      (`event_loop::places::tests::quitting_mid_clone_stops_it`)
- [ ] Notice carried: askpass close failure → next place's first frame has it; any other `Exit::Open` → none.
      (`event_loop::places::tests::a_clone_notice_reaches_the_next_place_once`)
- [ ] Repo screen remote writes unchanged: `jobs::operate` tests green unedited. (names in NOTES)

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
