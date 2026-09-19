# Review: M4 group 06 — credential-prompt-wiring

Reviewed: merge `704bfb6` against its first parent (`git diff 704bfb6^1 704bfb6`), `crates/graft` only.
Ticket: `.scratch/m4-remotes/issues/06-credential-prompt-wiring.md`.
Design: `docs/design-notes/m4-remotes.md` hard choice 2 ("must never" list), Proposed change 2, New UI: Credential prompt.

Read: CLAUDE.md, the ticket (Scope, Acceptance, Notes), the design-note sections above, KNOWN_GAPS rows M4/02–06, `--stat`, `crates/graft/MAP.md` + `ARCHITECTURE.md` diff, then the diff one file at a time. Whole files opened only to confirm: `crates/graft/src/update/remote.rs` (`stop_write`, cancel wording), `crates/graft-git/src/run/askpass.rs` (`RespondError`, serial `incoming()` loop), `crates/graft-git/src/run.rs` (`GIT_TERMINAL_PROMPT=0` stays under `askpass()`; `wait` checks `should_stop` first), `crates/graft-ui/src/text_input.rs` (`typable` refuses control chars).

## Gates (what was actually run)

A builder held the crate concurrently, so per the conductor only the fast tier ran, in `CARGO_TARGET_DIR=$PWD/target-review06` (deleted afterwards).

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo nextest run -p graft` | **734 passed, 0 failed, 19 skipped** (`#[ignore]`d captures) |
| suite time | 10.1 s (run 1), 8.9 s (run 2, `--status-level all`); 30.5 s wall incl. 19.5 s build |
| peak RSS | 684 MB (`/usr/bin/time -l`, cargo + tests) |
| `target-review06` size | 1.5 GB, removed |
| clippy (plain, heavy), doctests, heavy tier, release perf run, fuzz, bench | **not run** — conductor's instruction (shared crate). The builder's Notes report clippy ×2 and doctests clean workspace-wide; unverified here. |

Slowest 5 (all pre-existing): `remote_by_key::pull_then_undo_redo` 0.98 s, `m3_walk::every_scenario::stash_save` 0.96 s, `m3_walk::every_scenario::commit` 0.87 s, `m3_walk::every_scenario::undo_drift_guard` 0.83 s, `remote_by_key::push_then_undo_is_local_only` 0.82 s. This group's heaviest: `askpass_mode::*` 0.62–0.63 s each (spawns the binary), `credential_by_key::wrong_then_right_password_pushes` 0.47 s. Laptop-safe; no hangs (`drive` panics rather than waits when git asks more than scripted).

## Tick spot-check

All 13 acceptance ticks name a test inline. I ran every named test (26 tests + the `#[ignore]`d capture): **26 checked, 26 held**, capture present at `docs/shots/m4-credential-prompt-in-app.txt` (8.3 KB, under the 200 KB cap, header says TestBackend/no colour). Per-tick content was read against its claim; every claim in the tick text is asserted by the named test (e.g. `prompt_is_modal_and_swallows_globals` does assert `q`/`S`/`/` inert; `esc_cancels_the_operation` asserts the exact `[CancelCredential, CancelJob]` pair and the `· cancelling` status). No false tick.

## The password questions, answered first

**Can the secret reach argv / env / journal / log / capture / panic / Debug?**
- argv: the helper gets only the prompt; the answer goes down the socket. `cli::askpass` builds `Invocation::Askpass` from `args_os` (lossy) — and incidentally fixes KNOWN_GAPS M4/05 row 467 (`env::args()` panic on non-UTF-8), see Minor 4.
- env: only `GRAFT_ASKPASS_SOCKET`, `GIT_ASKPASS`, `SSH_ASKPASS*` on the git child (`graft-git` `run.rs:253–256`); no secret.
- journal: `assert_not_in_journal` scans the file's bytes for both passwords — held.
- logs: no `tracing`/`log` in `crates/graft/src` at all; nothing formats a `Command` or `AppState` outside tests.
- capture: masked `•`, `counter: None`; `the_credential_prompt_draws_over_everything_and_masks_what_is_typed` asserts `!contains("hunter2")`.
- panic: `report(&why.to_string())` prints `RespondError` — `Io(io::Error)` / static `Protocol` strs / `Cancelled`; none can carry the answer. `Waiting::lock` recovers a poisoned mutex with a stated reason; no `unwrap`/`expect` outside tests.
- Debug: `CredentialPrompt` (manual, `finish_non_exhaustive`, no `field`), `Answer` → `Answer(…)`, `Secret` → `Secret(…)`; `debug_redacts_secret` covers `AppState` and `Vec<Command>`.
- Cancelled job: `ChannelAskPass::ask` returns `None` within 100 ms; a late answer hits `Waiting::answer` → `None` entry → `false`, the `Secret` drops and zeroes (`late_answer_is_dropped`). A `send` `Err` hands the value back and drops it in the same statement.
- **Where it does not hold** — the plaintext in `TextField` (Major 1).

**Does the round trip prove git got the password from the app?** Yes. `HttpRemote` checks the password; `credential.helper` is blanked in the clone; the wrong password yields `requests().0 == 0` and an unchanged bare, the right one (typed only via `app.press` key events) moves the bare and `requests().0 >= 1`. The only way those bytes reach git is app → `AnswerCredential` → `Waiting` → socket → helper stdout → git.

**Remote-controlled text sanitised before drawing?** Yes on both paths that exist: `credential_needed` keeps `SanitizedText` for title and prompt (unit test with ESC/CR/LF); `view::tests::gits_words_and_a_remote_failure_cannot_carry_an_escape_to_the_screen` drives a hostile prompt *and* a hostile `Status::Sticky` through `draw`. The builder's own caveat stands — it is by convention, not by type (KNOWN_GAPS row M4/06 already written).

**Askpass exit codes.** 0: answer + `\n` on stdout, nothing on stderr. 1 (`Cancelled`): nothing on stdout, nothing on stderr. 2: nothing on stdout, one `graft: askpass: <reason>` line on stderr, no ESC. All three asserted by `askpass_mode::*` against a real `AskPassServer` / a dead socket. Since `GIT_TERMINAL_PROMPT=0` stays set on the child, git never falls back to `/dev/tty` on 1 or 2 — it dies "terminal prompts disabled", classified `AuthFailed`. Confirmed in `graft-git/src/run.rs:247,316`.

**Error swallowing / catch-alls.** `credential_key`'s `_ => {}` arms are the modal contract (swallow), commented. `runner::credential` binds the `bool` as `_delivered` with the reason in the doc comment. `say` drops a progress line on a poisoned sink lock — see Minor 2 for why that is a little worse for `CredentialNeeded` than for a progress line.

**Deviations (Notes 1–6):**
1. `Answer(Arc<Secret>)` instead of `Secret` in `Command` — right call; `Command` keeps `Clone + Eq`, `into_secret` moves when unique and copies-then-zeroes otherwise. Accept.
2. `JobRunner::with_askpass_helper` instead of `AppState::askpass_helper` — better than the ticket: the helper path is a process fact, `AppState` stays free of it, one seam instead of 24 `Operate` sites. Accept. `main.rs` never calls it, so the shipped binary uses `current_exe()` as specified.
3. `&str` prompt — fine.
4. `close_if_orphaned` at the top of `settle` — one place, runs after every event including prelude keys. Accept.
5. `Ctrl+C` falls through — matches the commit editor. Accept.
6. `Esc` via `stop_write` — accept; see Minor 1 for command order.

## Findings

### Critical

None.

### Major

**M1. The typed password lives in plain `String`s that are cloned on every event and never zeroed.**
`crates/graft/src/update/credential.rs:41` (`pub field: TextField`) and `crates/graft/src/update/mod.rs:64` (`let mut next = state.clone()`). `update` clones the whole `AppState` per event, so while the prompt is open every keystroke, job progress line and paint tick copies the partial password into a fresh heap `String`; the old one is freed unzeroed. `TextField::new("")` on Enter/Esc drops another unzeroed copy. The module doc of `jobs/askpass.rs:7–10` ("never copied into a `String` … zeroed when dropped") is true of the job side only; the loop side holds N plaintext copies in freed memory. This is what the ticket's signature asked for (`field: TextField`), so it is a spec limitation, not a builder error — but the claim "zeroes on drop" as stated in the ticket Notes and the MAP is broader than what holds. Why it matters: a core dump, a swap page or a later use-after-free-style read exposes it; the design's "must never" list does not name process memory, so this is a gap to record, not a rule broken. Fix: a KNOWN_GAPS row (unverified/limitation: "typed credential text is a plain `String` in `AppState`, copied per event, not zeroed"), and — if the user wants it closed — a `SecretField` wrapper in `graft-ui::text_input` whose `String` is wiped on drop, with `AppState`'s clone-per-event accepted as the remaining cost. Docs to soften now: `jobs/askpass.rs:7–10`, MAP `## update::credential` `Answer` line.

### Minor

**m1. `Esc` orders `CancelCredential` before `CancelJob`; reversing makes the journal wording deterministic.**
`crates/graft/src/update/credential.rs:197–203`. `CancelCredential` wakes the ask at once → helper exits 1 → git dies with an auth error on its own, while the stop flag from `CancelJob` is set a few µs later. `graft-git` `run.rs:392` classifies `Stopped` only if `should_stop()` is seen before the exit event; if git wins the race the outcome is `AuthFailed`, `operated` (`update/operate.rs:651`) clears `cancelling` and the user who pressed Esc sees "authentication failed · P to try again" and a journal entry that does not say `cancelled`. `esc_in_prompt_cancels_push` asserts `cancelled`, so this is a latent flake and a wrong status. Fix: push `CancelJob` (via `stop_write`) first, then `CancelCredential` — the ask polls `cancel` anyway, so nothing is lost.

**m2. A poisoned progress-sink lock silently drops `CredentialNeeded`.**
`crates/graft/src/jobs/runner.rs:568–577`. The comment ("dropping this one line of progress is safe") was written for progress; the same closure now carries the credential question. If dropped, the job blocks until the user presses Esc, with a footer that says only `… esc cancels`. The trigger (a panic inside the sink on the stderr thread) is remote. Fix: separate the two closures' failure handling or at least word the comment to say a dropped question leaves git waiting on Esc.

**m3. Mouse events reach the panes under the modal prompt.**
`crates/graft/src/update/mouse.rs` has no modal guard (none for `confirm`/`commit_editor` either — pre-existing pattern). A click while the prompt is open moves the selection or opens a context menu behind the box; a wheel scroll changes the graph. Not a secret leak; a consistency gap for all three modals. Fix once for all: early-return in `mouse_event` when `text_field_open() || confirm.is_some() || context_menu.is_some()`, or record as a gap.

**m4. KNOWN_GAPS row M4/05 "`env::args()` panics on non-UTF-8 argv in askpass mode" (line 467) is fixed by this group and should be closed.**
`cli::from_process` now uses `args_os` and `to_string_lossy`; `main.rs` no longer calls `env::args()`. Conductor's move (builders never edit KNOWN_GAPS): move the row to `docs/gaps/closed.md` naming `cli::askpass` and `askpass_mode_needs_the_socket_and_exactly_one_argument`.

**m5. A second 100 ms poll loop, beside the one KNOWN_GAPS row 460 marks "broken (rule)".**
`crates/graft/src/jobs/askpass.rs:22, 91–99`. The ticket specified `recv_timeout(100 ms)` so the builder followed it, and the latency is bounded by the test (`cancel_unblocks_ask` < 200 ms). But row 460 says a poll loop in `serve` is a rule-2 violation awaiting the user's OK; this one is the same shape. Either the user OKs both in one line (ADR-worthy: "cancellation polls at ≤ 100 ms") or row 460 grows to name this site too. Root-cause fix if wanted: make `Cancel` expose a `Receiver`/`Condvar` so the ask `select`s on cancel + answer.

**m6. Test doc overstates what is recorded.**
`crates/graft/tests/it/credential_by_key.rs:7–9` "records **every** event the loop saw" — `drive` records job events from `app.events` and status lines; key presses go through `app.press` and are not recorded (they would trivially contain the characters one per event). The assertion is still meaningful (progress texts, `Operated` errors, git's stderr) — say that instead.

**m7. Tautological assertion.**
`credential_by_key.rs:243–247` `assert_eq!(served.http.requests(), (0, served.http.requests().1))` compares the second element with itself. Fix: `assert_eq!(served.http.requests().0, 0)`.

**m8. `runner.rs` operate closure is now three nested closures deep.**
`crates/graft/src/jobs/runner.rs:547–605`: `queue_as(|id| Box::new(|cancel| { let say = …; let progress = …; askpass = ChannelAskPass::new(id, Box::new(|prompt| say(…)), …) }))`. Readable with the comments, but a `fn remote_sinks(id, sink, waiting, cancel) -> (ProgressSink, Arc<ChannelAskPass>)` would flatten it (PROMPT.md 6.2).

**m9. Capture nit (widget, not this group).** Title block draws `Credentials — 127.0.0.1:8443───` with no space before the rule, where the design mock shows `… 8443 ───`. Lives in `graft_ui::credential_prompt` (group 04). For the user's eye, not a blocker.

## Architecture / readability

- Crate rules hold: `app` speaks no socket protocol; `jobs::askpass` is behind `JobRunner`, `update::credential` is pure (no I/O, no jobs), `view::credential` maps state → `CredentialView`. ARCHITECTURE.md bullet added and accurate. MAP.md lines match the code (`with_askpass_helper`, `answer_credential`, `Hooks`, `Waiting`, `Answer`).
- `text_field_open()` extended and the C1 regression test runs through `update()` — the M3/06 lesson is applied.
- `credential_key` routed before the harness hand-back and the search prompt (`update/mod.rs:413–419`), doc'd why.
- UI thread: `answer_credential` is a brief mutex + `send`; `current_exe()` is one `readlink` on the loop thread. No git work on the loop.
- Data loss: none possible — nothing stored; Esc stops the whole write; a push that never authenticated is journaled `Aborted`.
- Unverified, as the builder says: colour; SSH path; the capture is a character grid.

## Verdict

No Critical. One Major that is a documented limitation to record (plaintext `String` copies of the password in `AppState`), not a broken promise; the round trip proves the real thing; ticks are honest. Ship with the KNOWN_GAPS row for M1 and the two-line order fix in m1.
