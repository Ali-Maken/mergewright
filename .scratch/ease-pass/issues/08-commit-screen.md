# commit-screen — type first, `⏎` commits, what goes in vs what stays out

Status: ready (ids and types revised 2026-10-17 after the ease-vocabulary and commit-what-goes-in scouts; this group's own scout not yet run)
Blocked by: 04-commit-what-goes-in, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout)
Needs: from commit-what-goes-in (graft-ops): `Include`, `Chosen`, `Part`, `FileChoice`, `OutWhy`, `first_choice`, `looks_like_secrets`, `Refusal::NothingChosen`, `CommitParams.include` (one struct for commit **and** amend: `Operation::Commit(CommitParams)` / `Operation::Amend(CommitParams)`; `message: String`). From ease-vocabulary (graft-ui): `commit.toggle_amend`, `commit.toggle_no_verify`, `commit.toggle_allow_empty` (context `CommitEditor`, no default key); the graph's after-commit `HintContext` state (name in V's NOTES). No new `ContextId`: the file list is a focus inside `CommitEditor`.

## Read first
- `docs/design-notes/ease-pass.md`: Need (commit), Prior art rows "Commit start" → "Secrets file", Journeys commit-* and the Commit-all landing, Mock behaviours C1–C15, New UI (commit screen), Questions 8, 9 (answered a).
- `docs/design/02-keymap.md` §2.10. `docs/research/commit-stash.md` §1.4–1.5.
- `crates/graft-ui/MAP.md`: `commit_editor`, `text_input`, `detail` (`DiffView`, `BodyView`), `detail_pane::diff_body`. `crates/graft/MAP.md`: `update::commit_editor`, `view::commit_editor`, `jobs::commit_message`, `jobs::detail` (one file's diff), `working_tree`, `update::status_read`, `update::undo`, `jobs::operate`.
- `.scratch/ease-pass/scouts/03-ease-vocabulary.md` (commit editor lines), `.scratch/ease-pass/scouts/04-commit-what-goes-in.md`.
- The commit mock's visible text and key handlers: `prototypes/ease-mocks/mergewright-commit-screen.html`.
- The scout's file.

## Scout questions (≤1 page)
- The editor's state, focus model and option toggles today (`Field::{Amend,SignOff,NoVerify,AllowEmpty}`, `graft-ui/src/commit_editor.rs:41-53`, handled in `graft/src/update/commit_editor.rs`); how `a` (amend) enters it; how a hook failure lands on its status row.
- **Key routing (same kind of mismatch as V's scout):** how today's option rows take `space` (match on `Field` focus, not an id?) — the file list's `space`/`j`/`k`/`⏎`/`esc` follow the same route. Whether `⏎` in the subject and `⌃S` are action ids or focus-routed matches, and which id `⌃S` has. How a keyless palette/`?` row (`commit.toggle_*`) reaches the editor's handler.
- Where the working-tree status the screen needs is already held; which job reads one working file's diff (staged and unstaged).
- What the commit's success lands on today (cursor, status line) and what `u` after a commit does to the editor's draft (index and HEAD are already restored by today's undo — `graft-ops` `undo.rs:300-335`, scout 04).
- Whether `CommitEditorView`'s footer (`commit_editor.rs:178 footer_subject`, `:192 footer_other`, `:212 footer(focus)`) is called from anywhere but the editor.

## Scope
In (graft-ui `commit_editor` redrawn as the commit screen; graft `update::commit_editor`, `view::commit_editor`, the after-commit landing; header push word). Adds to `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` only at their ends (G owns them in wave 4).
- **Layout (New UI):** full frame under the header: message box (subject 1 row, body 3), summary line, then `files 44 | diff`; <100 cols files only (diff hidden).
- **C1:** `c` opens with the cursor in the subject, placeholder `What did you change? (one line)`; header `committing to <branch>` + push state (mock words).
- **C2:** subject bold, counter `n/72` (`SUBJECT_LIMIT` 50 → 72), warning past 72. **C3:** body placeholder `More detail, if it needs it (optional)`.
- **C4:** summary `N files go in, +a −d, as a new commit on <branch>. N stay(s) out.` / `Nothing goes in. Put a file in with space.`
- **C5:** `Going in (n)` rows `✓`; `Staying out (n)` rows dim `·`; empty groups `nothing yet` / `nothing`.
- **C6:** first choice from `first_choice` (nothing staged → every change in; staged → only those). **C7:** a secrets-looking new file stays out with `!` and `looks like a secrets file`; its diff top reads `! left out: … space puts it in.`
- **C8:** diff of the file under the file cursor, `+`/`−` lines (today's diff body).
- **C9:** `⏎` in the subject commits when there is a message and ≥1 file going in (`include_of`); else the bar says `type a message first` / `put a file in first` and nothing is written. A `Refusal::NothingChosen` from ops (backstop) shows the same `put a file in first`.
- **C10:** `↑`/`↓` in the subject move the file cursor. **C11:** `⇥` subject → body → files; body `esc` → subject; files `j`/`k`, `space` toggles in/out, `⏎` commits, `esc` → subject — routed by focus as today's option rows are.
- **C12 — the three bars are this group's screen code** (today's `footer_subject`/`footer_other`/`footer(focus)`), words exactly the commit mock's; keys spelled through the keymap (`key_verb`), no literal keys (`tests/no_literal_keys.rs`).
- **C13 landing:** graph, cursor on the new commit, status `✓ Committed N files to <branch>. u undo · P push`, bar from V's after-commit state (`u undo the commit, your edits come back · P push · c commit again`); the top line's right end gains `N commit(s) to push` while HEAD's branch is ahead of its upstream (absent when in sync).
- **C14 / Q9 a:** `u` right after → back on the graph, the edits uncommitted again (today's undo restores HEAD and index; nothing new in ops), the undone commit's message kept as the draft for the next `c`. `c` again → a fresh screen.
- **C15:** `esc` in the subject closes; the draft is kept for the next `c` this run.
- **Hook fails:** stays on the screen, message kept, the failure on its status row (today's). Hooks now run against a scratch index for a `Chosen` commit (commit-what-goes-in) — the status row words unchanged.
- **Q8 a — options off screen:** no tick-box row (the `Field::{Amend,SignOff,NoVerify,AllowEmpty}` rows retire). `⌃S` inserts the sign-off as today; amend, skip hooks, allow empty toggled by `commit.toggle_*` from `?`/`⌃P`. While one is on, the header names it after the push state (`· skipping hooks`), nothing when all are off.
- **Amend (`a`)** uses the same screen: header `amending <sha7> <subject>`, message prefilled; staged files go in, others stay out (`Include::Staged` when untouched — a message-only amend stays possible); `space` puts others in (`Include::Chosen`). Same `CommitParams`, `Operation::Amend`.
Out: what a commit writes (W), keymap rows and graph bar words (V).

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
/// Untouched amend → `Staged`. Otherwise `Chosen` from the rows going in (ops plans a
/// `Chosen` equal to the staged set as `Staged` itself).
pub fn include_of(choices: &[FileChoice], touched: bool) -> Include;
```
`CommitParams.message` is a `String`: the screen joins subject and body as today's `params()` (`graft/src/update/commit_editor.rs:153`) does.

## Touches
Every caller and test of `CommitEditorView`/`Field`/`OPTIONS` (retired or kept as the scout finds); `footer_subject`/`footer_other`/`footer` and their snapshots; `params()` (`commit_editor.rs:153`) and the edit-stop amend (`:340`) — W set `include: Include::Staged` there; this group sets the real value at `:153`, `:340` stays `Staged`; `jobs::commit_message` (template, amend message); the after-write status read; `u` and the journal entry words (`commit "<subject>"`, unchanged); the graph top line (`commit(s) to push`); tour card's commit task words (`view::tour`); `?`/`⌃P` listing for `CommitEditor`; captures of the commit editor (recaptured); `docs/design/02-keymap.md` §2.10 (V's Proposed edit).

## Acceptance
- [ ] Journey `commit-all-subject`: `c`, type `fix toc`, `⏎` → HEAD has a new commit with all 3 modified files, `.env` not in it; cursor on it; status starts `✓ Committed 3 files to` (state test; pty walk is A's).
- [ ] C1/C3 placeholders and C4 summary words exact (snapshot at 120); C2 counter reads `8/72`, warning style past 72.
- [ ] C5/C7: `.env` under `Staying out` with `!` and `looks like a secrets file`; `space` on it moves it to `Going in`.
- [ ] C9: empty subject + `⏎` → bar `type a message first`, HEAD unchanged; all files out + `⏎` → `put a file in first`.
- [ ] C10/C11: `↓` in the subject moves the file cursor; `⇥ ⇥` reaches files; `esc` from files → subject; `⏎` from files commits.
- [ ] C12: each of the three bars' words equal the mock's (snapshot per focus at 120).
- [ ] Journeys `commit-with-body` (6) and `commit-leave-first` (6) land as the note says (state tests).
- [ ] C13: after landing, top line contains `1 commit to push`; bar reads the mock's words.
- [ ] Q9: `u` after the commit → HEAD back, edits uncommitted, `c` opens with the old message in the subject.
- [ ] C15: `esc` with a typed subject, then `c` → subject still there.
- [ ] Q8: no option row drawn; `⌃S` adds `Signed-off-by:`; `commit.toggle_no_verify` from `?` shows `· skipping hooks` in the header; off → gone.
- [ ] Amend: `a` → header `amending`, message prefilled; `⏎` with nothing touched = message-only amend.
- [ ] Hook failure: screen stays, message kept, failure on the status row.
- [ ] Captures next to the commit mock: `docs/shots/ease-pass/commit-open-120.txt`, `commit-secrets-120.txt`, `commit-files-focus-120.txt`, `commit-refused-120.txt`, `commit-90.txt`, `commit-landed-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~75k). Split point: screen + C1–C12 here; landing, `u`, draft keep and amend as a follow-on.
