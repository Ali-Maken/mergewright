# credential-prompt-wiring: the askpass mode of the binary, the in-app prompt state and keys, secrets across the job pool, the wrong-password round trip

Status: done
Blocked by: 02 (done), 03, 05
Build with: Opus — taking the planner's recommendation: a secret crossing three threads, and a field that must join `text_field_open()` (M3 group 06's Critical).
Crates: graft
Needs: `remote-keys` — `Operated`, `update::remote`, `jobs/operate.rs` hooks, `Progress::RemoteProgress`; `git-remote-and-askpass` — `graft_git::run::askpass::{AskPass, AskPassServer, Secret, respond, RespondError}`; `ui-credential-prompt-and-hints` — `graft_ui::credential_prompt::{CredentialView, CredentialPrompt, credential_area, MIN_COLUMNS, MIN_ROWS}`, `FieldView.masked`. Tests: `graft_fixtures::remote::HttpRemote`.

## Read first

- `docs/design-notes/m4-remotes.md` — Hard choice 2 (whole, including "must never"); Interfaces (`graft (app)` block, last three lines); Proposed change 2; New UI: Credential prompt; Genuinely risky groups (G5b)
- `.scratch/m4-remotes/scout.md` §Text input (56–61), §Key routing (68–75), §Job runner (6–19)
- `crates/graft/MAP.md` §lib.rs (`cli`, `event_loop`), §cli (400–405), §update (`text_field_open` 166; `Harness`/`prelude` 116; `Command`/`Progress`/`Product` 109–111, 171), §update::commit_editor (`editor_key`, `paste` — the modal shape to copy), §update::stash (`StashPrompt` — the text-field-in-an-overlay shape), §jobs (`JobRunner::submit/cancel`, `Cancel`), §event_loop (`run`, `pump`)
- `crates/graft-git/MAP.md` §run/askpass (after G2); `crates/graft-ui/MAP.md` §credential_prompt, §text_input (after G4)
- `docs/design/02-keymap.md` §2.14 (lines 203–210)
- M3 review 06 C1 (`docs/reviews/m3-06-undo-and-stash-ui.md` lines 95–124)

## Scope

In — in this order (each piece a commit):

1. **Askpass mode of the binary.** `cli::Invocation::Askpass { socket: PathBuf, prompt: String }`, chosen in `cli::from_process` when the environment has `GRAFT_ASKPASS_SOCKET` and exactly one positional argument (git and ssh pass the prompt as the single argument). `main`: for `Askpass`, call `graft_git::run::askpass::respond(&socket, &prompt, &mut stdout.lock())` **before** any config load, terminal probe or logging; exit 0 on `Ok`, 1 on `Cancelled`, 2 on any other error with one line on stderr naming it (git shows askpass stderr to no one; the user sees git's own "could not read Password" — good enough, and the failure is classified `AuthFailed`). Nothing in this path ever prints the answer anywhere but stdout.
2. **Prompt state.** `update/credential.rs`: `CredentialPrompt { title, prompt, field: TextField, masked, job: JobId }` — `title` from the URL/host in the prompt (`Credentials — <host[:port]>`, or `Credentials` when no URL is found), `prompt` = git's text, sanitised (`sanitize_line`); `masked = prompt contains "assword" || "assphrase"` (case-insensitive) — a `yes/no` host-key question and a username ask are unmasked; manual `Debug` printing `CredentialPrompt { title, masked, job }` only. `AppState::credential: Option<CredentialPrompt>`; **`AppState::text_field_open()` returns true while it is `Some`** (the rule from M3 06 C1, restated in the MAP line). `credential_key` is modal, routed **before** the undo check and before every pane (beside the commit editor's arm in `update/keys.rs`): printable chars → `field.insert`; paste → `field.insert_str`; `Backspace`/`Delete`/`←`/`→`/`Home`/`End` as the field does; `Ctrl+U` clears; `Enter` → `Command::AnswerCredential(job, Secret::new(field.text().into_bytes()))`, then close the prompt and **clear the field's text** (`TextField::new("")`); `Esc` → `Command::AnswerCredential(job, None)`-equivalent (`Command::CancelCredential(job)`) **and** `Command::CancelJob(job)` — the whole operation stops, never half-authenticated; status `<desc> · cancelling…`. `Tab` no-op (one field). Every other key swallowed.
3. **The job side.** `jobs/askpass.rs`: `ChannelAskPass { events: <the job's progress sender>, answers: Mutex<Option<Receiver<Option<Secret>>>>, cancel: Cancel }` implementing `AskPass::ask`: sends `Progress::CredentialNeeded { prompt }` under the job's id, then loops `recv_timeout(100 ms)` on the answer channel until an answer arrives or `cancel.is_cancelled()` (→ `None`). `JobRunner::answer_credential(id, answer: Option<Secret>)`: routes to the job's `Sender` registered in a `Mutex<HashMap<JobId, Sender<Option<Secret>>>>` at `ask` time and removed after; an unknown id is a no-op that **returns `false`** and the loop logs nothing (the job finished; the answer is dropped — `Secret` zeroes on drop). `Command::AnswerCredential(JobId, Secret)` / `Command::CancelCredential(JobId)` are carried out by the loop calling `answer_credential`. `run_operation`: `RemoteHooks.askpass = Arc::new(ChannelAskPass::new(...))`, `helper = std::env::current_exe()` (error → `Context.remote = None` + the status reason, as G5a specified).
4. **`update` plumbing.** `Progress::CredentialNeeded { prompt: String }` → `credential::credential_needed(state, id, prompt)`: opens the prompt only when `id == operating`; a second ask while one is open replaces it (git asks username, then password — two sequential asks). When the operation ends for any reason, close any open prompt (a stale prompt after the job died would answer into nothing). `view`: when `credential` is `Some`, draw `CredentialPrompt` over everything (after the confirm box in precedence), `FieldView { masked, counter: None, .. }`, note `typed here only · never stored by Graft`, footer `⏎ send  ⌃U clear  esc cancel the <push|pull|fetch>`. `Harness::prelude` proceeds when the credential prompt is open (a text field waiting for a key counts as "ready"), otherwise the acceptance walk deadlocks waiting for `operating` to clear.
5. **Tests.** Unit: `credential_key` table (masking rule, `Ctrl+U`, `Enter` emits the command and clears the field, `Esc` emits cancel + `CancelJob`); `text_field_open()` true while open; `u`/`U`/`Ctrl+Z`/`f`/`p`/`P` typed into the prompt insert or are swallowed, never dispatch (the C1 regression test, through `update()`); `Debug` of `AppState` with a prompt holding `hunter2` does not contain `hunter2`. Integration (`tests/it/credential_by_key.rs`, real `JobRunner`, `HttpRemote`, `GIT_ASKPASS` = `env!("CARGO_BIN_EXE_graft")` set by the runner's own `askpass()` — the test only needs `run_operation` to use `current_exe()`, which under `cargo test` is the test binary, **not** `graft`; so `run_operation` takes the helper path from `Startup`/`Session` (`helper: PathBuf`, defaulting to `current_exe()` in the binary and set to `CARGO_BIN_EXE_graft` by tests) — add `AppState::askpass_helper: Option<PathBuf>` threaded into `Command::Operate`'s job, and say so in MAP): see Acceptance.
6. MAP.md lines; ARCHITECTURE.md bullet: "Changing the credential prompt → `app::update::credential` (state, keys), `app::jobs::askpass` (the job's `AskPass`), `cli` (the binary's askpass mode), `ui::credential_prompt` (the widget)".

Out: storing or caching credentials (never — the user's helpers do that); SSH verification in tests (set by G2, unverified, KNOWN_GAPS row written by the acceptance group); two-field prompt (Proposed change 2).

## Signatures

```rust
// crates/graft/src/cli.rs
pub enum Invocation { …, Askpass { socket: PathBuf, prompt: String } }
// crates/graft/src/update/credential.rs
pub struct CredentialPrompt { pub title: SanitizedText, pub prompt: SanitizedText, pub field: TextField, pub masked: bool, pub job: JobId }
impl fmt::Debug for CredentialPrompt {}   // redacts `field`
pub fn credential_needed(state: &mut AppState, id: JobId, prompt: String);
pub fn credential_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool;
pub fn credential_paste(state: &mut AppState, text: &str) -> bool;
pub fn is_masked_prompt(prompt: &str) -> bool;
pub fn host_of(prompt: &str) -> Option<String>;
// crates/graft/src/update/mod.rs (additive)
pub enum Progress { …, CredentialNeeded { prompt: String } }
pub enum Command { …, AnswerCredential(JobId, Secret), CancelCredential(JobId) }
pub struct AppState { …, pub credential: Option<CredentialPrompt>, pub askpass_helper: Option<PathBuf> }
// AppState::text_field_open() — now also `self.credential.is_some()`
// crates/graft/src/jobs/askpass.rs
pub struct ChannelAskPass { /* … */ }
impl AskPass for ChannelAskPass {}
// crates/graft/src/jobs/runner.rs (additive)
impl JobRunner { pub fn answer_credential(&self, id: JobId, answer: Option<Secret>) -> bool; }
```

## Acceptance

Every tick names its verifying test inline, at tick time (ADR-0035).

- [x] `graft` with `GRAFT_ASKPASS_SOCKET` set and one argument runs `respond` and exits 0/1/2 without touching the terminal — `tests/it/askpass_mode.rs::binary_answers_over_socket` (spawns `CARGO_BIN_EXE_graft` against a real `AskPassServer`: stdout = secret + `\n`, exit 0, empty stderr, prompt arrived verbatim), `binary_exits_1_with_nothing_on_stdout_when_cancelled`, `binary_exits_2_naming_the_error_when_graft_is_gone`; the mode's rule alone in `cli::tests::askpass_mode_needs_the_socket_and_exactly_one_argument`
- [x] `text_field_open()` is true while the prompt is open; `u`/`U`/`Ctrl+Z`/`f`/`p`/`P` typed there dispatch nothing — `update::credential::tests::prompt_is_modal_and_swallows_globals` (through `update()`; also asserts `q`, `S` and `/` are inert)
- [x] Masking rule: `Password for …` and `Enter passphrase for key …` masked; `Username for …` and `Are you sure you want to continue connecting (yes/no…` unmasked — `update::credential::tests::is_masked_prompt_table`
- [x] `Enter` emits `AnswerCredential` with the typed bytes and clears the field; `Esc` emits `CancelCredential` + `CancelJob` — `update::credential::tests::enter_answers_and_clears`, `update::credential::tests::esc_cancels_the_operation` (asserts the exact command pair and the `· cancelling` status)
- [x] `{:?}` of an `AppState` holding `hunter2` in the prompt does not contain `hunter2` — `update::credential::tests::debug_redacts_secret` (and the same for the `Command` carrying it away)
- [x] End to end over `HttpRemote`: `P`, `Enter`; wait for `Progress::CredentialNeeded`; type `wrong`, `Enter` → `Sticky` "authentication failed … · P to try again", `requests() == (0, ≥1)`; `P`, `Enter`, type the right password, `Enter` → bare's `main` == local, `requests().0 ≥ 1`; the journal file, every recorded status line and every recorded event contain neither `wrong` nor the password — `tests/it/credential_by_key.rs::wrong_then_right_password_pushes`
- [x] `Esc` in the prompt mid-push: `operating` clears, entry `Aborted` `cancelled`, bare unchanged, `HttpRemote::requests().0 == 0` — `credential_by_key::esc_in_prompt_cancels_push`
- [x] Username-then-password: `url_anonymous` → two prompts in sequence (first unmasked, second masked), both answered, push succeeds — `credential_by_key::username_then_password`
- [x] An answer arriving after the job ended is dropped and `answer_credential` returns `false`; no panic — `jobs::askpass::tests::late_answer_is_dropped`
- [x] Cancelling the job unblocks a waiting `ask` with `None` within 200 ms — `jobs::askpass::tests::cancel_unblocks_ask` (also `an_already_cancelled_job_does_not_ask`)
- [x] `Harness::prelude` feeds the next key while the prompt is open — `update::tests::prelude_proceeds_into_credential_prompt`
- [x] MAP.md, ARCHITECTURE.md updated — `crates/graft/MAP.md` (§cli, §update, §update::credential, §jobs, §view), `ARCHITECTURE.md` "Changing the credential prompt"
- [x] Every string git sends is sanitised before it is drawn (the G2 review's unenforced rail) — `update::credential::tests::gits_words_are_cleaned_before_they_are_kept` (the kept `SanitizedText`) and `view::tests::gits_words_and_a_remote_failure_cannot_carry_an_escape_to_the_screen` (no ESC/BEL in any drawn cell, prompt *or* `Status::Sticky` failure line)
- [x] The prompt is drawn, judged by eye — `docs/shots/m4-credential-prompt-in-app.txt` (8.3 KB; 120x40 password over a running push, 80x24 username), from the `#[ignore]`d `view::tests::capture_m4_credential_prompt_in_app`

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` · `cargo nextest run -p graft` · `cargo test --doc -p graft`. Plus, once: `grep -rn "hunter2\|<test password>" target/nextest` finds nothing (the builder runs it, records it).

## Notes

Scout: yes — `Explore` (sonnet) over the askpass mode, `jobs/runner.rs` + `jobs/operate.rs`, and the overlay/key routing, before any edit.

Gates, all workspace-wide (never `-p`, per CLAUDE.md): `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean; the same with `--features heavy-tests` clean; `cargo nextest run` **1787 passed, 0 failed, 22 skipped**; `cargo test --doc` clean. The one-off leak grep found the test password in no file the tests wrote (`target-g06/tmp`, `target-g06/fixtures`) and in no artefact but the compiled test binaries, which contain their own source literals — there is no `target/nextest` store in this layout, so the grep was run over the whole target dir instead, and that is what it found.

Deviations from the ticket's signatures, all deliberate:

1. **`Command::AnswerCredential(JobId, Answer)`, not `(JobId, Secret)`.** `Command` is `Clone + PartialEq + Eq` (every `update` test compares command vectors); `Secret` is deliberately none of those. `update::credential::Answer` is a newtype over `Arc<Secret>` with content equality and a `Answer(…)` `Debug`; `into_secret` unwraps the `Arc` when it is the last reference and copies otherwise, and both allocations zero on drop. Without this, either `Command` loses its derives or the secret gets cloned as a plain `Vec<u8>`.
2. **No `AppState::askpass_helper`.** The helper path is a property of the process, not of the app state, and `Command::Operate` has 24 call sites that would all have had to carry it. Instead `JobRunner::with_askpass_helper(PathBuf)` overrides the default `current_exe()`; the integration test names `CARGO_BIN_EXE_graft` there. Same effect, one seam instead of twenty-four, and `AppState` never holds a path it does not use.
3. **`credential_needed(&mut AppState, JobId, &str)`** — `&str`, not `String`; it sanitises into its own `SanitizedText` and never keeps the caller's buffer.
4. **The prompt is closed by `close_if_orphaned` at the top of `settle`**, not by a hook on each way an operation can end. One place, so no exit path can leave a prompt that answers into a dead job.
5. **`Ctrl+C` is not swallowed** — it falls through to quit. Everything else, `Tab` included, is.
6. `Esc` routes through `update::remote::stop_write`, so the footer says `push main → origin/main · cancelling` rather than the prompt vanishing silently.

Unverified (rule 5 and rule 1):

- **Colour.** The capture is a `TestBackend` character grid with no colour attributes. The prompt's colours ship unverified.
- **SSH.** Only the HTTP path is exercised end to end. `SSH_ASKPASS` is set by G2's hooks and the binary's mode is agnostic, but no test drives an SSH remote.
- The capture is not a pty recording; it proves layout and masking, not what a real terminal draws.

Found on the way, not in scope (for the milestone review, not fixed here): nothing in `app` forces a *new* remote-controlled string through `sanitize` — the two tests above pin the two paths that exist now, but the guarantee is by convention, not by type, once a third path appears. A `SanitizedText`-only field on the view models would close it.

## Size

One context; brownfield — scout `update/keys.rs` (after G5a), `event_loop/running.rs`'s command dispatch, `jobs/runner.rs` before editing; say so. Stop seam: after piece 3 (binary mode + job side, prompt not yet drawn) with 4–5 in a handoff note. Past 150 turns: stop, `partial`, handoff.
