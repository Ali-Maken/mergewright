# commit-screen — type first, `⏎` commits, what goes in vs what stays out

Status: ready
Blocked by: 04-commit-what-goes-in, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `Include`, `Chosen`, `Part`, `FileChoice`, `first_choice`, `looks_like_secrets`, `NothingChosen`, amend `include` (commit-what-goes-in); `CommitFiles` context, `commit_files.*`, `commit.toggle_*`, commit and `AfterCommit` `HintContext`s (ease-vocabulary).

## Read first
- `docs/design-notes/ease-pass.md`: Need (commit), Prior art rows "Commit start" → "Secrets file", Journeys commit-* and the Commit-all landing, Mock behaviours C1–C15, New UI (commit screen), Questions 8, 9 (answered a).
- `docs/design/02-keymap.md` §2.10. `docs/research/commit-stash.md` §1.4–1.5.
- `crates/graft-ui/MAP.md`: `commit_editor`, `text_input`, `detail` (`DiffView`, `BodyView`), `detail_pane::diff_body`. `crates/graft/MAP.md`: `update::commit_editor`, `view::commit_editor`, `jobs::commit_message`, `jobs::detail` (one file's diff), `working_tree`, `update::status_read`, `update::undo`, `jobs::operate`.
- The commit mock's visible text and key handlers: `prototypes/ease-mocks/mergewright-commit-screen.html`.
- The scout's file.

## Scout questions (≤1 page)
- The editor's state, focus model and option toggles today; how `a` (amend) enters it; how a hook failure lands on its status row.
- Where the working-tree status the screen needs is already held; which job reads one working file's diff (staged and unstaged).
- What the commit's success lands on today (cursor, status line) and what `u` after a commit does to the editor's draft.

## Scope
In (graft-ui `commit_editor` redrawn as the commit screen; graft `update::commit_editor`, `view::commit_editor`, the after-commit landing; header push word). Adds to `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` only at their ends (G owns them in wave 4).
- **Layout (New UI):** full frame under the header: message box (subject 1 row, body 3), summary line, then `files 44 | diff`; <100 cols files only (diff hidden).
- **C1:** `c` opens with the cursor in the subject, placeholder `What did you change? (one line)`; header `committing to <branch>` + push state (mock words).
- **C2:** subject bold, counter `n/72` (`SUBJECT_LIMIT` 50 → 72), warning past 72. **C3:** body placeholder `More detail, if it needs it (optional)`.
- **C4:** summary `N files go in, +a −d, as a new commit on <branch>. N stay(s) out.` / `Nothing goes in. Put a file in with space.`
- **C5:** `Going in (n)` rows `✓`; `Staying out (n)` rows dim `·`; empty groups `nothing yet` / `nothing`.
- **C6:** first choice from `first_choice` (nothing staged → every change in; staged → only those). **C7:** a secrets-looking new file stays out with `!` and `looks like a secrets file`; its diff top reads `! left out: … space puts it in.`
- **C8:** diff of the file under the file cursor, `+`/`−` lines (today's diff body).
- **C9:** `⏎` in the subject commits when there is a message and ≥1 file going in (`Include::Chosen`); else the bar says `type a message first` / `put a file in first` and nothing is written.
- **C10:** `↑`/`↓` in the subject move the file cursor. **C11:** `⇥` subject → body → files; body `esc` → subject; files `j`/`k`, `space` toggles in/out, `⏎` commits, `esc` → subject. **C12:** the three bars (V's contexts).
- **C13 landing:** graph, cursor on the new commit, status `✓ Committed N files to <branch>. u undo · P push`, bar `u undo the commit, your edits come back · P push · c commit again`; the top line's right end gains `N commit(s) to push` while HEAD's branch is ahead of its upstream (absent when in sync).
- **C14 / Q9 a:** `u` right after → back on the graph, the edits uncommitted again, the undone commit's message kept as the draft for the next `c`. `c` again → a fresh screen.
- **C15:** `esc` in the subject closes; the draft is kept for the next `c` this run.
- **Hook fails:** stays on the screen, message kept, the failure on its status row (today's).
- **Q8 a — options off screen:** no tick-box row. `⌃S` inserts the sign-off as today; amend, skip hooks, allow empty toggled from `?`/`⌃P`. While one is on, the header names it after the push state (`· skipping hooks`), nothing when all are off (planner decision: a hidden switch that is on must be visible).
- **Amend (`a`)** uses the same screen (planner decision, mock gap): header `amending <sha7> <subject>`, message prefilled; staged files go in, others stay out (`Include::Staged` when untouched — a message-only amend stays possible); `space` puts others in (`Include::Chosen`).
Out: what a commit writes (W), keymap rows and bar words (V).

## Signatures
graft-ui `commit_editor`:
```rust
pub const SUBJECT_LIMIT: usize = 72;
pub enum CommitFocus { Subject, Body, Files }
pub enum FileGroup { GoingIn, StayingOut }
pub struct CommitFileRow { pub path: SanitizedText, pub group: FileGroup, pub added: u32, pub removed: u32, pub secret: bool }
pub enum Summary { Goes { files: u32, added: u32, removed: u32, branch: SanitizedText, out: u32 }, Nothing }
pub enum Refusal { NoMessage, NoFile }
pub struct CommitHeader { pub branch: SanitizedText, pub amending: Option<SanitizedText>, pub push: SanitizedText, pub options: Vec<&'static str> }
pub struct CommitScreenView { pub header: CommitHeader, pub subject: FieldView, pub body: AreaView,
    pub summary: Summary, pub files: Vec<CommitFileRow>, pub cursor: usize, pub focus: CommitFocus,
    pub diff: BodyView, pub refusal: Option<Refusal>, pub status: Option<SanitizedText>, pub busy: bool }
pub struct CommitScreen<'a> { pub view: &'a CommitScreenView, pub theme: &'a Theme, pub keys: &'a Keymap }   // Widget
```
graft app (`update::commit_editor`):
```rust
pub struct Draft { pub subject: String, pub body: String }       // kept for the run (C15, Q9)
pub fn include_of(choices: &[FileChoice], touched: bool) -> Include;   // untouched amend → Staged
```

## Touches
Every caller and test of `CommitEditorView`/`Field`/`OPTIONS` (retired or kept as the scout finds); `footer_subject`/`footer_other`; `jobs::commit_message` (template, amend message); the after-write status read; `u` and the journal entry words; the graph top line (`commit(s) to push`); tour card's commit task words (`view::tour`); captures of the commit editor (recaptured); `docs/design/02-keymap.md` §2.10 (V's Proposed edit).

## Acceptance
- [ ] Journey `commit-all-subject`: `c`, type `fix toc`, `⏎` → HEAD has a new commit with all 3 modified files, `.env` not in it; cursor on it; status starts `✓ Committed 3 files to` (state test; pty walk is A's).
- [ ] C1/C3 placeholders and C4 summary words exact (snapshot at 120); C2 counter reads `8/72`, warning style past 72.
- [ ] C5/C7: `.env` under `Staying out` with `!` and `looks like a secrets file`; `space` on it moves it to `Going in`.
- [ ] C9: empty subject + `⏎` → bar `type a message first`, HEAD unchanged; all files out + `⏎` → `put a file in first`.
- [ ] C10/C11: `↓` in the subject moves the file cursor; `⇥ ⇥` reaches files; `esc` from files → subject; `⏎` from files commits.
- [ ] Journeys `commit-with-body` (6) and `commit-leave-first` (6) land as the note says (state tests).
- [ ] C13: after landing, top line contains `1 commit to push`; bar reads the mock's words.
- [ ] Q9: `u` after the commit → HEAD back, edits uncommitted, `c` opens with the old message in the subject.
- [ ] C15: `esc` with a typed subject, then `c` → subject still there.
- [ ] Q8: no option row drawn; `⌃S` adds `Signed-off-by:`; toggling skip hooks from `?` shows `· skipping hooks` in the header; off → gone.
- [ ] Amend: `a` → header `amending`, message prefilled; `⏎` with nothing touched = message-only amend.
- [ ] Hook failure: screen stays, message kept, failure on the status row.
- [ ] Captures next to the commit mock: `docs/shots/ease-pass/commit-open-120.txt`, `commit-secrets-120.txt`, `commit-files-focus-120.txt`, `commit-refused-120.txt`, `commit-90.txt`, `commit-landed-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~75k). Split point: screen + C1–C12 here; landing, `u`, draft keep and amend as a follow-on.
