# commit-editor: `c`/`a`, the in-app message editor, and `$EDITOR` with a clean suspend/resume

Status: done
Blocked by: 01-git-reads-and-patches, 02-ops-model, 04-changes-pane
Build with: Opus — the `$EDITOR` path restores and re-enters the terminal mid-run (terminal protocol; principle 8), and the same suspend path serves ADR-0002's `keys.suspend_on_ctrl_z`. The editor widget itself is plain and rides along.
Needs: `changes-pane` (`Command::Operate`, `Product::Operated`, `update::operate::{operated, operation_in_flight}`, `Status::{Done, Failed, Busy}`, grouped `WorkingTreeStatus` on `AppState`), `ops-model` (`Operation::{Commit, Amend}`, `CommitParams`, `Refusal`), `git-reads-and-patches` (`RepoHandle::git_path` for `COMMIT_EDITMSG`).

## Read first

- `docs/design-notes/m3-working-tree-undo-and-stash.md` — New UI "Commit editor", "Status line"; Proposed changes 3, 9; Hard choice 4; Genuinely risky groups (external editor)
- `docs/design/02-keymap.md` §2.2 lines 66–70 (why `Ctrl+Shift+Z` is unreliable — the same reasoning gives the `[ Commit ]` button), §2.3 (`c`, `a`, `e`), §2.10
- `PROMPT.md` §3 "Commit message editor" (lines 265–269), §4 "Committing" (lines 289–294)
- `docs/adr/0002-ctrl-z-is-undo.md` (whole: suspend/resume consequences)
- `crates/graft/MAP.md` §terminal (`TerminalSession`, `restore`, `restorer`, `RawMode`), §event_loop (`run`, `pump`, `Wake`, `input::Forwarder`, `InputMux::hold` note), §session, §update (`Command`, `AppState::screen`, `Harness`), §config (`Config`, `load`, the `[diff]`/`[radar]` seeding precedent)
- `crates/graft-ui/MAP.md` §map_screen (the overlay-replaces-graph precedent in `view::frame`), §fit, §paint, §theme
- `crates/graft-term/MAP.md` §sanitize (`sanitize_line`)

## Scope

In — in this order, one commit each, turns reported:

1. **Text input widget** (`crates/graft-ui/src/text_input.rs`, new; shared later by the stash prompt): `TextField` (single line: cursor, insert, delete, home/end, word-left/right, horizontal scroll to keep the cursor visible) and `TextArea` (lines, cursor, newline, vertical scroll); pure state + a `Widget` that draws the text, a `┊` guide at a given column, and a `n/limit` counter; unicode-width aware through `fit`. Paste (bracketed) inserts as typed.
2. **Editor state and view** (`crates/graft/src/update/commit_editor.rs`, `crates/graft-ui/src/commit_editor.rs`, `crates/graft/src/view/commit_editor.rs`, new): `CommitEditor { subject: TextField, body: TextArea, focus: Field, amend, sign_off, no_verify, allow_empty, template_applied }`; `c` opens it (from the graph or the pane), `a` opens it with `amend = true` and the subject/body prefilled from `HEAD`'s message (`Product::HeadMessage` from a small job reading `read_commit(HEAD)`; the editor opens at once and the text lands when the job answers, keys before that are queued into the field — or simpler: the job runs first and the editor opens on its product; choose the simpler and say so). Keys per §2.10 plus: `Enter` in Subject → Body; `Tab`/`Shift+Tab` cycle Subject → Body → options → `[ Commit ]` → Subject; `Space` on an option toggles it; `Enter` on `[ Commit ]` or `Ctrl+Enter` anywhere → `Command::Operate(Commit|Amend)`; `Ctrl+S` toggles `sign_off`; `Ctrl+Shift+C` appends `Co-authored-by: ` on a new body line and moves the cursor there; `Esc` closes (a non-empty draft is kept in `AppState` for the next `c` this session — say so in the title `draft kept`). Title row per the note; the refusal preview (`nothing staged`, `subject empty`) computed purely from `AppState.working_tree` + the fields, drawn beside a dimmed button. While the job runs the title is `committing…` and keys are ignored except `Esc` (cancels the job → `Command::CancelJob`).
3. **Template:** `Command::ReadCommitTemplate` job at editor open when the body is empty: `git config --get commit.template` → file contents → `Product::CommitTemplate(Option<String>)`; lines starting `#` are dropped from the prefilled body (git would strip them) — say so in the ticket if you keep them instead.
4. **`$EDITOR` and suspend** (`crates/graft/src/event_loop/outside.rs`, new): `run_outside_terminal(session, f)` — leave the alternate screen, restore raw mode and every mode `TerminalSession` turned on, `InputMux::hold()`, run `f`, re-enter everything, re-probe nothing (cell size/theme are re-queried by the existing focus/resize paths), force a full redraw. `Ctrl+E` writes the current text to `git_path("graft/COMMIT_EDITMSG")`, runs `$GIT_EDITOR` / `core.editor` / `$VISUAL` / `$EDITOR` (git's own precedence; `git var GIT_EDITOR` gives it in one spawn) with that path through `std::process::Command` inheriting the terminal, then reads the file back into the fields (comment lines dropped). A non-zero editor exit keeps the old text and says `editor exited N, message unchanged`. `e` in the changes pane opens the cursor's file at the cursor's new-line number the same way (`<editor> +<line> <path>` for the common editors; plain `<editor> <path>` otherwise — list: vim/nvim/vi/nano/emacs/micro/hx take `+N`; `code`/`zed` take `<path>:<line>` via `--goto`/`path:line`; anything else no line). `keys.suspend_on_ctrl_z = true` in config → `Ctrl+Z` runs `run_outside_terminal` around `libc::raise(SIGTSTP)`… no `libc` in `graft`: use `nix`? Not a dependency either — send `SIGTSTP` via `std::process::Command::new("kill").args(["-TSTP", pid])`, which is the one-line, dependency-free way; note it in the ticket.
5. **Wiring, hints, MAP.md, captures:** `view::frame` draws the editor in the graph area (`AppState::commit_editor.is_some()`), the detail pane stays; footer while open per the note; `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`; `docs/shots/m3-commit-editor-{160x48,80x24}.txt`, `m3-commit-editor-refused.txt` (nothing staged), `m3-amend.txt` via `TestBackend`.

Out: message history (KNOWN_GAPS, M7), GPG/SSH signing prompts (the user's config applies; a pinentry that needs the tty is unverified — KNOWN_GAPS row, "cancel with Esc if it hangs"), hook output beyond stderr's first line in the status line (the full stderr lands in the journal entry's Aborted reason and the browser shows it — `undo-and-stash-ui`), the command palette.

## Signatures

```rust
// crates/graft-ui/src/text_input.rs
pub struct TextField { /* text, cursor (char idx), scroll */ }
impl TextField { pub fn new(text: &str) -> Self; pub fn text(&self) -> &str; pub fn insert(&mut self, c: char); pub fn insert_str(&mut self, s: &str); pub fn backspace(&mut self); pub fn delete(&mut self); pub fn left(&mut self); pub fn right(&mut self); pub fn home(&mut self); pub fn end(&mut self); pub fn word_left(&mut self); pub fn word_right(&mut self); }
pub struct TextArea { /* lines, cursor (line, char), scroll */ }
impl TextArea { pub fn new(text: &str) -> Self; pub fn text(&self) -> String; pub fn lines(&self) -> &[String]; pub fn insert(&mut self, c: char); pub fn newline(&mut self); pub fn backspace(&mut self); pub fn delete(&mut self); pub fn up(&mut self); pub fn down(&mut self); pub fn left(&mut self); pub fn right(&mut self); pub fn home(&mut self); pub fn end(&mut self); pub fn append_line(&mut self, line: &str); }
pub struct FieldView<'a> { pub label: &'a str, pub text: SanitizedText, pub cursor: Option<usize>, pub guide_at: Option<u16>, pub counter: Option<(usize, usize)>, pub focused: bool }
pub struct FieldWidget<'a> { pub view: &'a FieldView<'a>, pub theme: &'a Theme }   // Widget, one row
pub struct AreaView<'a> { pub label: &'a str, pub lines: Vec<SanitizedText>, pub cursor: Option<(usize, usize)>, pub scroll: usize, pub guide_at: Option<u16>, pub focused: bool }
pub struct AreaWidget<'a> { pub view: &'a AreaView<'a>, pub theme: &'a Theme }

// crates/graft-ui/src/commit_editor.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Field { Subject, Body, Amend, SignOff, NoVerify, AllowEmpty, CommitButton }
pub struct CommitEditorView<'a> { pub title: SanitizedText, pub subject: FieldView<'a>, pub body: AreaView<'a>, pub options: [(Field, bool); 4], pub focus: Field, pub refusal: Option<SanitizedText>, pub busy: bool }
pub struct CommitEditorScreen<'a> { pub view: &'a CommitEditorView<'a>, pub theme: &'a Theme }   // Widget; fills the area it is given

// crates/graft/src/update/commit_editor.rs (pure)
pub struct CommitEditor { pub subject: TextField, pub body: TextArea, pub focus: Field, pub amend: bool, pub sign_off: bool, pub no_verify: bool, pub allow_empty: bool, pub head_short: Option<String> }
impl CommitEditor { pub fn new() -> Self; pub fn amend(head_short: String, message: &str) -> Self; pub fn params(&self) -> CommitParams; pub fn refusal(&self, status: &WorkingTreeStatus) -> Option<String>; pub fn message(&self) -> String; pub fn load_message(&mut self, text: &str); }
pub fn open(state: &mut AppState, amend: bool, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
pub fn editor_key(state: &mut AppState, key: &KeyPress, next_id: &mut impl FnMut() -> JobId) -> Option<Vec<Command>>;   // Some while open (modal)
pub fn template_arrived(state: &mut AppState, template: Option<String>);
pub fn head_message_arrived(state: &mut AppState, message: String);
pub fn editor_returned(state: &mut AppState, result: Result<String, String>);   // $EDITOR's file, or why not

// crates/graft/src/update/events.rs (additions)
pub enum Command { /* … */ ReadCommitTemplate(JobId), ReadHeadMessage(JobId), OpenExternalEditor { path: PathBuf, line: Option<u32> }, Suspend }
pub enum Product { /* … */ CommitTemplate(Option<String>), HeadMessage(Result<String, String>), ExternalEditor(Result<String, String>) }

// crates/graft/src/event_loop/outside.rs
pub fn run_outside_terminal<T>(session: &mut TerminalSession, mux: &InputMux, f: impl FnOnce() -> T) -> Result<T, OutsideError>;
pub fn editor_command(git_editor: &str, path: &Path, line: Option<u32>) -> std::process::Command;   // pure; the `+N` table
pub fn git_editor(repo: &RepoHandle) -> Result<String, RunError>;   // `git var GIT_EDITOR`

// crates/graft/src/config (addition)
pub struct Keys { pub suspend_on_ctrl_z: bool }   // `[keys]`, default false (ADR-0002)
```

## Acceptance

- [x] `TextField`/`TextArea` unit tests: insert/delete around a wide char and a combining mark keep the cursor on a char boundary; `word_left` over `foo  bar`; `TextArea::text()` round-trips `\n`; `append_line` on an empty area does not add a leading blank line.
- [x] Update tests: `c` opens the editor (`AppState::commit_editor.is_some()`), title contains `2 files staged` from a scripted status; typing lands in Subject; `Enter` moves focus to Body and a newline is *not* inserted in Subject; `Tab` ×3 reaches `CommitButton` (the spec said ×4; the walk is Subject→Body→Amend→Button, which is three — m3/05 review); `Enter` there emits `Operate(_, Commit(params))` with `message == "subject\n\nbody\n"`, `sign_off` per `Ctrl+S`; with nothing staged the button emits nothing and the view's `refusal` reads `nothing staged`; `[ ] allow empty` toggled → emits; `a` prefills from `Product::HeadMessage` and sets `amend`; `Product::Operated(Ok(Everything))` closes the editor and clears the draft; `Err` keeps it open with `Status::Failed`; `Esc` keeps the draft and the next `c` reopens it.
- [x] `editor_command` table: `vim` → `vim +12 path`, `code` → `code --goto path:12`, `zed` → `zed path:12`, `unknown-editor` → `unknown-editor path`; an editor string with arguments (`"emacs -nw"`) keeps them.
- [x] `run_outside_terminal`: a scripted `RawMode`/`TerminalSession` test asserts the sequence leave-alt-screen → restore modes → `f` → modes back → alt-screen back, and that a panic inside `f` still restores (the guard). Real-terminal behaviour is the acceptance walk's pty check.
- [x] Real-runner test on a `copy_into_temp` fixture: stage a file, open the editor, type a subject, `Tab` to the button, `Enter` → after the product, `git log -1 --format=%s` equals the subject and the journal has a `Commit` entry with `refs_before[main] != refs_after[main]`; then `a`, change the subject, commit → `git log -1 --format=%s` is the new subject and `git rev-list --count HEAD` is unchanged.
- [x] `[keys] suspend_on_ctrl_z = true` reaches `AppState` (the `[radar]` seeding precedent); with it false `Ctrl+Z` is left for `undo-and-stash-ui`'s undo binding (no-op here, but the key must not be swallowed).
- [ ] Captures from piece 5 exist, under 8 KB; the `┊` guide glyphs and the `12/50` counter are visible in the character grid (colour is not provable — do not claim it).
  **Unticked at the m3/05 review.** The captures exist and the guides and the
  counter are in the grid; the 8 KB half **cannot be met and is a spec
  defect** — see NOTES. Every file is under CLAUDE.md's 200 KB cap, which is
  the rule that binds.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

~1,000 lines + ~450 test. Seams: after piece 2 (editor commits, no template, no `$EDITOR`), after piece 4. Scout `terminal.rs`/`event_loop/startup.rs` for how modes are applied and restored rather than reading them whole; the panic-safe restore must reuse `TerminalSession::restore`'s idempotence, not duplicate it.

## Evidence for each tick

Every box above names the test that earned it. Nothing here was ticked from
reading the code.

1. **`TextField`/`TextArea`** — `graft-ui` `text_input::tests`:
   `a_wide_character_is_inserted_and_deleted_whole`,
   `a_combining_mark_is_its_own_character_to_the_cursor`,
   `delete_forwards_over_a_wide_character`,
   `word_left_walks_back_over_the_gap_then_the_word`,
   `text_round_trips_through_newlines`,
   `append_line_on_an_empty_area_adds_no_leading_blank`.
2. **Update tests** — `update::commit_editor::tests`, 28 of them, each driving
   `update()` rather than a private helper:
   `c_opens_the_editor_and_typing_lands_in_the_subject`,
   `the_title_counts_the_staged_files`,
   `enter_in_the_subject_moves_to_the_body_and_inserts_nothing`,
   `tab_walks_subject_body_options_button_and_back`,
   `enter_on_the_button_commits_the_subject_and_the_body` (asserts
   `message == "subject\n\nbody\n"`), `ctrl_enter_commits_from_anywhere`,
   `ctrl_s_turns_the_sign_off_on_and_it_reaches_the_params`,
   `nothing_staged_refuses_and_the_button_does_nothing`,
   `an_empty_subject_refuses`, `a_merge_in_progress_refuses_and_names_the_command`,
   `allow_empty_lets_a_commit_through_with_nothing_staged`,
   `a_asks_for_heads_message_and_opens_on_the_answer`,
   `esc_keeps_a_draft_and_the_next_c_picks_it_back_up`,
   `a_commit_that_failed_keeps_the_editor_and_the_message`. The success half
   (`Operated` closes the editor and clears the draft) is asserted over the
   real runner instead — see 5 — because a scripted `Applied` proves only
   that the arm was taken.
3. **`editor_command`** — `event_loop::outside::tests`:
   `the_vi_lineage_takes_a_plus_line`, `vs_code_wants_goto_and_zed_wants_a_suffix`,
   `an_editor_nobody_knows_gets_the_file_and_nothing_else`,
   `no_line_asked_for_means_no_line_argument`,
   `an_editor_with_arguments_keeps_them_in_front`,
   `the_table_matches_the_programs_name_not_its_path`.
4. **`run_outside_terminal`** — `event_loop::outside::tests`:
   `the_terminal_is_given_up_before_f_runs_and_taken_back_after` (a scripted
   `RawMode` records the transcript; the assertion is the order),
   `a_panic_inside_f_still_re_enters_the_terminal`,
   `the_input_reader_is_reading_again_afterwards`. Underneath, `terminal::tests`:
   `suspend_undoes_every_mode_and_resume_puts_the_same_ones_back`,
   `resume_does_not_turn_on_a_mode_suspend_did_not_find`,
   `suspending_an_already_restored_session_is_a_no_op`. And
   `input::mux::tests::a_paused_mux_leaves_the_input_for_whoever_has_the_terminal`
   reads the typed bytes from a second handle on the same socket while the mux
   is paused — the property that matters, not that `pause` was called.
   **Not verified: a real `$EDITOR` on a real terminal.** No test in this repo
   can prove the user's terminal is usable afterwards; that is the acceptance
   ticket's pty walk, and until it runs this ships unverified.
5. **Real runner** — `tests/it/commit_editor_writes.rs`:
   `the_button_commits_what_was_typed_and_then_amends_it` (real `git`, real job
   pool: `git log -1 --format=%s`, `rev-list --count`, and the journal entry's
   `refs_before[main] != refs_after[main]`), `nothing_staged_makes_no_commit`.
**Added in the review round** (see the section at the end of this file):
`graft-ui` `commit_editor::tests::{a_refused_commits_reason_is_drawn_on_the_editor_not_only_behind_it,
the_key_hints_come_back_once_there_is_nothing_to_report,
a_failure_costs_the_editor_no_rows}`;
`update::commit_editor::tests::{a_paste_lands_in_the_subject,
the_subject_flattens_a_paste_and_the_body_keeps_its_lines,
a_paste_with_no_editor_open_is_ignored,
a_refused_commits_reason_reaches_the_editors_own_view,
esc_dismisses_the_reason_first_and_closes_the_editor_second}`;
`update::operate::tests::{e_on_a_file_asks_for_that_files_porcelain_path,
e_in_the_diff_carries_the_cursors_new_side_line}`;
`event_loop::outside::tests::{a_porcelain_path_is_joined_onto_the_worktree_not_graft_s_own_directory,
an_absolute_path_is_left_alone,
a_quoted_editor_path_is_run_by_a_shell_the_way_git_runs_it,
a_plain_word_list_is_not_handed_to_a_shell,
the_shell_form_keeps_its_editors_line_style}`;
`graft-term` `input::gate::tests::{a_reader_that_never_parks_is_a_refusal_not_a_shrug,
a_reader_that_has_ended_lets_the_terminal_go,
a_live_reader_parks_and_the_pause_comes_straight_back}`;
`graft-git` `run::exec::tests::{a_stopped_child_is_asked_first_so_its_cleanup_can_run,
a_child_that_ignores_the_ask_is_killed_anyway,
a_child_that_already_exited_is_only_reaped}` and
`editor::tests::{git_names_an_editor_for_any_directory_it_will_run_in,
a_directory_that_is_not_there_is_reported_not_guessed_at}`.

6. **`[keys]`** — `config::tests::keys_suspend_on_ctrl_z_parses_and_defaults_off`
   (parses, warns about nothing, default off) and
   `event_loop::startup::tests::keys_suspend_on_ctrl_z_reaches_app_state`
   (the seeding line, the way the `[radar]` test guards its own), and
   `update::commit_editor::tests::ctrl_z_suspends_only_when_the_config_says_so`
   (off → the key is left alone for undo, not swallowed).
7. **Captures** — the four files below exist and the `┊` guides and the `26/50`
   counter are in the character grid. **The 8 KB line in the acceptance was not
   met and cannot be:** a 160x48 grid is 7,824 bytes before a single multi-byte
   glyph, so `m3-commit-editor-160x48.txt` is 9,289 (group 04's equivalent is
   8,878, the same way). The rule that binds is CLAUDE.md's 200 KB cap, which
   every file is two orders of magnitude under. Colour is not claimed — the
   pipeline records no attributes.

## NOTES

**Captures** (`docs/shots/`), the user judges these by eye, not me:
`m3-commit-editor-160x48.txt`, `m3-commit-editor-80x24.txt`,
`m3-commit-editor-refused.txt`, `m3-amend.txt`.

**The spec's `InputMux::hold()` is wrong for this, and I did not use it.**
`hold` only clears the outlet's sender; the reader thread keeps polling and
consuming stdin. `$EDITOR` would have raced Graft for every keystroke — the
user's arrow keys landing half in vim and half in a Graft that is not drawing.
`InputMux::pause`/`resume` are new: a self-pipe the reader polls beside the
input fd, and `pause` returns only once the thread has actually stopped
touching it. `graft-term/src/input/gate.rs` is the new module.

**Where else this deviates from the spec text:**
- `refusal` takes `(Option<&WorkingTreeStatus>, Option<InProgress>)`, not
  `&WorkingTreeStatus`: ADR-0034's merge/rebase/cherry-pick refusal needs the
  in-progress state, and the status can be absent before the first read.
- `ReadCommitTemplate`/`ReadHeadMessage` are not new `Command` variants; both
  ride the existing job path (`jobs::commit_message::{template, head_message}`)
  and answer as `Product::CommitTemplate`/`HeadMessage`, so no new seam.
- `head_message_arrived`/`template_arrived`/`editor_returned` take the `JobId`
  as well, so an answer for an editor the user has since closed is dropped
  rather than overwriting a retyped draft
  (`a_message_from_an_editor_the_user_has_closed_is_dropped`).
- `OpenExternalEditor`, `EditCommitMessage` and `Suspend` are carried out by
  the frame loop, not the job pool — only the thread owning the session and the
  mux can hand the terminal over. The loop answers with
  `Product::ExternalEditor`/`Product::HandedOver`.
- `git_editor` is a plain `std::process::Command` (`git var GIT_EDITOR`), not a
  `GitInvocation`: it is a read, and the seam guard only governs writes.
- The refusal is **not** in the title. The spec had it both places; the design
  note puts it beside the dimmed button, and doubling it cost the staged count
  its row.
- `SIGTSTP` goes through `kill -TSTP <pid>` as the spec suggested — no `libc`
  in `graft`, and adding one for one signal is not worth it.

**Two filed gaps fixed, at the cause, not the symptom:**
- A write failure was being wiped off the status line by the refresh it
  triggers. `Status::WriteFailed` now tells a write's failure from a read's, so
  the refresh walk's `JobFinished` no longer clears the reason. This is the
  "tell write-failures from read-failures" fix, not a patch.
- `Esc` now clears `Status::Failed`/`WriteFailed` before it means anything else.

**A bug the captures found, not the tests.** Under 100 columns the detail pane
is a `DetailLayout::Tab`, and the editor was being drawn into that split —
behind a tab whose selecting key the modal editor swallows. An editor nobody
can reach is an editor that did not open. `view::frame::draw_commit_editor`
now gives it the whole area when the layout is tabbed. This is why the 80x24
capture exists; a review reading only tests would not have seen it.

**A bug the acceptance test found.** `[keys]` parsed and took effect while the
config loader's warning pass still called the whole section unknown — the user
would have been told a setting that works does nothing. `SCHEMA` in
`config/load.rs` is the one place section names are written out and `keys` was
missing. The existing unknown-section test happened to use `[keys]` as its
example of a name Graft has no meaning for; it now uses `[nokeys]`.

**`e`'s test claim below was wrong** and is corrected in the review-fix
section at the end of this file: there was no test for `e` at all.

**Left for someone else, deliberately:**
- README has no "What M3 actually does" section yet (group 04 did not add one
  either), so `[keys] suspend_on_ctrl_z` is documented nowhere a user reads.
  The acceptance ticket is the place for it.
- No footer hint for `e` on the changes pane. The key works and is tested; the
  hint row is the changes pane's, not the editor's.
- GPG/SSH signing is out of scope and untested here: a pinentry that wants the
  tty runs while Graft still owns it. Worth a KNOWN_GAPS row as **unverified**,
  with "cancel with Esc if it hangs" — the spec's own words.

**This session compacted once**, between the terminal-handover commit and the
captures. Nothing was left half-done across it: every piece below was
committed at its seam.

**Commits:** `1998786` (text input), `83e5532` (editor state, screen, template),
`a5c6ca9` (terminal handover, the mux gate), `97f2f1d` (wiring, config, keys),
`dd13914` (real-runner test, the tabbed fix, MAP.md), `644aaac` (the `[keys]`
schema bug, the failed-commit test).

---

# Review round (m3/05), one fix pass

`docs/reviews/m3-05-commit-editor.md`: 0 Critical, 8 Major, 14 Minor. All
eight Majors fixed, plus Minors 1, 4 and 5. Commits `a6a3c34`, `acd2161`,
`912380f` and this one.

## The `index.lock` row: **true, and now fixed**

The review filed it unverified: "`Esc` mid-commit SIGKILLs git — lock-file
residue unverified". It is real. Measured, not reasoned about.

A C harness (`fork` → `exec git commit` → `usleep(N)` → signal → `waitpid` →
sweep `.git` for `*.lock`) swept the kill instant across a real `git commit`
on a 20,000-file index in 250 µs steps, git 2.54, macOS:

| signal | kills landing while git was running | stranded locks |
|---|---|---|
| `SIGKILL` (what `Child::kill` sends) | 267 | **43** |
| `SIGTERM` | 288 | 6 |
| `SIGTERM`, narrowed to the worst 3.5 ms | 288 | 6 |

The stranded files were `.git/index.lock`, `.git/HEAD.lock`,
`.git/refs/heads/main.lock` and `.git/packed-refs.lock`. Inside the ~7 ms git
holds `index.lock` on that index, near **every** `SIGKILL` stranded it. A
first attempt with 120 shell-driven kills found nothing — every kill had
landed after the commit finished (`rev-list --count` went up by 120), which
is why the count and not just the absence is recorded here.

**Cause:** git removes its lock files from a signal handler
(`SIGTERM`/`INT`/`HUP`/`QUIT`) and from `atexit`. Neither runs on `SIGKILL`.
`graft-git`'s runner killed with `Child::kill`, which is `SIGKILL`.

**Fix, at the cause:** `run::exec::kill` is now `SIGTERM`, a 2 s grace, then
`SIGKILL` only for a child that sat through the ask — so a cancel still
cannot hang on a hook that ignores the signal. No `libc` in `graft-git` and
no `SIGTERM` in `std`, so the signal goes through `kill(1)`, the route
`graft` already uses for `SIGTSTP`.

**What it costs the user:** nothing on the ordinary path. A cancel of a git
command that ignores `SIGTERM` now takes up to 2 s instead of being instant.

**What remains, and it is not closable from here:** the 6 residues under
`SIGTERM` are a race *inside git* — `create_tempfile` opens the lock file
before it marks the tempfile active, so a signal arriving between those two
finds nothing to remove. That is the same race a user's `Ctrl+C` in a
terminal already runs. KNOWN_GAPS row, **unverified**, ~2% of cancels that
land in the index-write window.

## Majors

1. **Hook refusal had nowhere to draw.** `CommitEditorView` gains `status`;
   it takes the footer row in `Token::Warning` (the graph footer's `problem`
   takes the hints the same way). `Esc` dismisses it before it closes the
   editor — the two-stage `Esc` `keys.rs` already had. Cause, not symptom:
   the previous round fixed `WriteFailed` so a rejected commit kept its
   message, and the editor was the half that still swallowed it.
2. **`e` opened the wrong file.** `Handover` carries `workdir`;
   `editor_command` joins the porcelain path onto it and sets
   `current_dir`. Cause. **The NOTES claim that `e` "works and is tested"
   was false** — there was no test naming `OpenExternalEditor` or
   `edit_target` anywhere. There are three now.
3. **Paste was dropped.** `update::commit_editor::paste` routes
   `Event::Paste` to the focused field. Cause (the arm was a stub).
4. **`index.lock`** — above.
5. **Quoted `GIT_EDITOR`.** Shell syntax in the string now goes through
   `sh -c '<editor> "$@"'`, git's own rule. A plain word list still runs
   directly. Cause.
6. **`Gate::pause` swallowed a live reader.** The reader says when its loop
   ends; a park timeout with no such word is now an `Err` the hand-over
   refuses on. Cause.
7. **`Status::WriteFailed` → `Status::Sticky`.** Three of its uses are failed
   *reads*; the name said write. The rule — "a failure that must outlive the
   refresh it triggers" — is on the variant now, and in MAP.md.
8. **The last bare `git` spawn.** `git var GIT_EDITOR` goes through
   `graft_git::editor::git_editor` → `run`, so it gets the fixed
   environment, the output cap and the stop policy. It lives in `graft-git`
   because `seam_rule.rs` allows a `GitInvocation` under `graft/src` only
   inside `jobs/` and this call must run on the frame thread.
   ARCHITECTURE.md has a "Handing the terminal over" section: why a job
   cannot do it, the four steps of `run_outside_terminal`, where the git
   spawn goes.

   **Half not done, deliberately.** The review also asked for `GIT_EDITOR`
   to be resolved *as a job at editor open* and stored in `AppState`. That
   needs a new `Command`/`Product` pair, an `AppState` field, and a
   two-step for `e` (which can fire before any editor has been opened, so
   the answer would not be there yet). Minor 9 of the same review concedes
   the UI-thread spawn is "acceptable because the editor blocks next
   anyway", and the substance of Major 8 — invisible to the seam guard, off
   the `run` rails — is fixed. Recorded rather than half-built.

## Minors fixed

- **1.** `reenter` put the reader back *before* raw mode, so a key typed in
  that window was cooked and echoed over the screen Graft was about to
  redraw. Modes first now. The guard's doc claimed it was what saved the
  terminal on a panic; `TerminalSession::Drop` is, and it says so.
- **4.** The 80x24 capture header claimed both "the detail pane is not
  drawn" and "the detail pane is still there beside the editor". The pane
  line is per size now.
- **5.** Tick 7 unticked, the 8 KB line recorded as a spec defect (below).

## Minors not fixed (for KNOWN_GAPS or a later ticket)

2, 3, 6, 7, 8, 10, 11, 12, 13, 14. None changes behaviour the user can see
except 8 (`strip_comments` hardcodes `#` and ignores `core.commentChar`),
which is worth a row.

## Spec defect: the 8 KB capture cap

The acceptance line "Captures … under 8 KB" **cannot be met at 160×48 and is
not a build failure**. A bare 160×48 grid is 48 × (160 + 2 quotes + newline)
= **7,824 bytes** before a single multi-byte glyph or a line of preamble;
`m3-commit-editor-160x48.txt` is 9,001. The reviewer confirmed the
arithmetic independently. CLAUDE.md's rule is the **200 KB** cap, which every
capture is two orders of magnitude under. The acceptance ticket should not
inherit the 8 KB line.

## Captures

Regenerated, all under the 200 KB cap, the user judges them by eye:

- `docs/shots/m3-commit-editor-160x48.txt` (9,001 B) — header split per size
- `docs/shots/m3-commit-editor-80x24.txt` (3,300 B) — header split per size
- `docs/shots/m3-commit-editor-rejected.txt` (5,784 B) — **new**: a
  pre-commit hook's refusal on the footer row, the evidence for Major 1.
  The warning colour it is drawn in is **not** provable from a capture.

`m3-commit-editor-refused.txt` and `m3-amend.txt` are unchanged: neither
draws a status.

## Gates after the round

`cargo fmt --check` clean · `cargo clippy --all-targets -- -D warnings`
clean · `cargo test` **1,526 passed, 0 failed, 15 ignored** (1,503 before
the round). Fast tier only.
