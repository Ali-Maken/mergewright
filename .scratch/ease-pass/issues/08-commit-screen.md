# commit-screen — type first, `⏎` commits, what goes in vs what stays out

Status: ready (revised 2026-10-18 after scout 08: sign-off decided as a header-named flag and its keymap description reworded here (conductor, 2026-10-18), `Field` retires with `CommitButton`, amend ref `:344`, `Draft`/`LastCommit` state specified, `JustCommitted` choice site named; earlier: ids and types revised 2026-10-17 after the ease-vocabulary and commit-what-goes-in scouts; option listing and after-commit bar named after 03b; review 03 Minor 5's code half added)
Blocked by: 04-commit-what-goes-in, 03-ease-vocabulary, 03b-ease-vocabulary-rest
Build with: Opus — brownfield screen rewrite over a scouted, fully specified state model; no new cross-crate signature, nothing whose correctness hides from the tests.
Needs: from commit-what-goes-in (graft-ops): `Include`, `Chosen`, `Part`, `FileChoice`, `OutWhy`, `first_choice`, `looks_like_secrets`, `Refusal::NothingChosen`, `CommitParams.include` (one struct for commit **and** amend: `Operation::Commit(CommitParams)` / `Operation::Amend(CommitParams)`; `message: String`; `sign_off`, `no_verify`, `allow_empty` stay). From ease-vocabulary (graft-ui): `commit.toggle_amend`, `commit.toggle_no_verify`, `commit.toggle_allow_empty` (context `CommitEditor`, no default key). From 03b: the three ids listed in `?`/`⌃P` from the commit editor and dispatched through `commit_editor::toggle_from_palette` (`update/palette/run.rs:52`); `HintContext::GraphRow { row: RowBar::JustCommitted, verbs }` (`hints/context/row_bar.rs:34`, words `hints/lists/rows.rs:125`). No new `ContextId`: the file list is a focus inside `CommitEditor`.

## Read first
- `docs/design-notes/ease-pass.md`: Need (commit), Prior art rows "Commit start" → "Secrets file", Journeys commit-* and the Commit-all landing (lines 59-71), Mock behaviours C1–C15 (to line 114), Questions 8, 9 (answered a).
- `docs/design/02-keymap.md` §2.10. `docs/research/commit-stash.md` §1.4–1.5.
- `crates/graft-ui/MAP.md` `## commit_editor`, `## graph_screen::hints`; `text_input`, `detail` (`DiffView`, `BodyView`), `detail_pane::diff_body`. `crates/graft/MAP.md`: `update::commit_editor`, `update::palette`, `view::commit_editor`, `jobs::commit_message`, `jobs::detail`, `working_tree`, `update::status_read`, `update::undo`, `jobs::operate`.
- **`.scratch/ease-pass/scouts/08-commit-screen.md`** (every line ref below is from it, main `2610bce9`).
- `.scratch/ease-pass/scouts/04-commit-what-goes-in.md`; `.scratch/ease-pass/issues/03b-ease-vocabulary-rest.md` "Commit options in `?`/`⌃P`" and NOTES; `.scratch/ease-pass/issues/07-graph-rows.md` "Bar per row kind (G13)".
- `docs/reviews/ease-pass-03.md` Minor 5.
- The commit mock's visible text and key handlers: `prototypes/ease-mocks/mergewright-commit-screen.html` (bars :159, landing :169-178).

## Scope
In (graft-ui `commit_editor` redrawn as the commit screen, plus the one `commit.sign_off` description in `keymap/table/commit_editor.rs`; graft `update::commit_editor` + `commit_editor/keys.rs`, `view::commit_editor`, the after-commit landing, the undo hook, header push word; `docs/design/02-keymap.md` :529). `update/state.rs`, `update/events.rs`, `jobs/kinds.rs`: append at their ends only (G owns them in wave 4) — **one exception: retype `commit_draft` in place at `state.rs:252`** (one line; NOTES tells G). No edits to hints files or `update::palette`.
- **Layout (New UI):** full frame under the header: message box (subject 1 row, body 3), summary line, then `files 44 | diff`; <100 cols files only (diff hidden).
- **C1:** `c` opens with the cursor in the subject, placeholder `What did you change? (one line)`; header `committing to <branch>` + push state (mock words: `not pushed yet` / `N commit(s) to push`).
- **C2:** subject bold, counter `n/72` (`SUBJECT_LIMIT` 50 → 72, `commit_editor.rs:31`), warning past 72. **C3:** body placeholder `More detail, if it needs it (optional)`.
- **C4:** summary `N files go in, +a −d, as a new commit on <branch>. N stay(s) out.` / `Nothing goes in. Put a file in with space.`
- **C5:** `Going in (n)` rows `✓`; `Staying out (n)` rows dim `·`; empty groups `nothing yet` / `nothing`.
- **C6:** first choice from `first_choice(&WorkingTreeStatus)` over `AppState::working_tree` (`state.rs:96`), on every open — never carried in a draft. **C7:** a secrets-looking new file stays out with `!` and `looks like a secrets file`; its diff top reads `! left out: … space puts it in.`
- **C8:** diff of the file under the file cursor through `jobs/detail.rs:114 read_file_diff` (`DiffSpec::WorkingTreeVsHead`, `:63-67`), today's `+`/`−` body.
- **C9:** `⏎` in the subject commits when there is a message and ≥1 file going in (`include_of`); else the bar says `type a message first` / `put a file in first` and nothing is written. `Refusal::NothingChosen` from ops (backstop) shows `put a file in first`. `⌃⏎` (`commit.commit`) keeps committing from any focus, same checks.
- **C10:** `↑`/`↓` in the subject move the file cursor. **C11:** `⇥` Subject → Body → Files → Subject, `⇧⇥` back; body `esc` → subject; files `j`/`k` and `↓`/`↑` move, `space` toggles in/out, `⏎` commits, `esc` → subject. Routed by `CommitFocus` in `open_editor_key` (`keys.rs:35`, today's match on `Field` at :76-80).
- **`Field` retires** (`commit_editor.rs:41-56`) with `CommitButton`, `OPTIONS` :60, `is_option`, `label`, `next`/`previous` :91/:105, `sibling_option` :120, `button_key`/`option_key` (`keys.rs:305-307, :336`), `CommitEditorView`, `CommitEditorScreen`. `CommitFocus` replaces it. Migrate the five integration sites: `tests/it/commit_editor_writes.rs:182,402`, `daily_landing.rs:106`, `m3_walk.rs:295`, `remote_by_key.rs:262` — `Field::Subject` → `CommitFocus::Subject`; a `CommitButton` focus-then-`⏎` becomes `⏎` from the subject. Each still asserts what it asserted.
- **C12 — the three bars are screen code** (replacing `footer_subject` :178 / `footer_other` :192 / `footer` :212; no callers outside the file), words exactly the mock's (`:159`); keys through the keymap (`key_verb`), no literal keys (`tests/no_literal_keys.rs`).
- **C13 landing:** graph, cursor on the new commit, status `✓ Committed N files to <branch>. u undo · P push`, bar `HintContext::GraphRow { row: RowBar::JustCommitted, .. }`; top line's right end `N commit(s) to push` while HEAD's branch is ahead of its upstream (absent in sync). **Choice site:** the graft function that sets the graph's row `HintContext` — today it returns `WorkingTreeRow`/`BranchRow`/`PlainRow` (no `RowBar` use on main; find it with `grep -rn 'HintContext::BranchRow' crates/graft/src` outside tests; 07 rewrites it to pick `GraphRow`). Add **one arm, ahead of the row-kind arms and inside the `Marked`/`Searching` wrappers**: `if let Some(row) = landed_bar(state.last_commit.as_ref(), cursor_oid) { return GraphRow { row, verbs } }`. Whichever of 07/08 merges second keeps the other's arms. Picked only for `Operation::Commit`, not `Amend`.
- **`LastCommit` lifetime:** set when a `Commit` lands, `commit` = the HEAD oid `Landing::Head` puts the cursor on (`operate.rs:192`), `message` = the subject/body just written. Cleared by: any other operation landing (`operate/outcome.rs:121` path), an undo landing (consumed, below), `c` or `a` opening. Cursor moving is not a clear: off the commit the row's own bar shows, back on it `JustCommitted` again.
- **C14 / Q9 a:** `u` with `last_commit` set → undo as today (HEAD and index restored, nothing new in ops); on the undo landing, `last_commit.take()`'s `message` becomes `commit_draft` (`amending: None`); back on the graph. Next `c` opens with that message. Undo of any other entry leaves `commit_draft` alone. Hook site: where `update::undo` handles a landed undo (scout: `update/undo*` never touches the draft today).
- **C15 / draft:** `esc` in the subject closes and stores `Draft { subject, body, amending }` (`close()`, `keys.rs:231`). `c` resumes a draft with `amending: None`; `a` resumes one whose `amending` equals today's HEAD, else prefills HEAD's message. Options reset to off on every open (the header shows them, so nothing is hidden). `committed()` (`keys.rs:236-238`) keeps clearing `commit_draft`. `from_draft` keeps suppressing the template.
- **Hook fails:** stays on the screen, message kept, failure on its status row (`view/commit_editor.rs:56`, unchanged).
- **Q8 a — options off screen, never hidden:** no tick-box row. All four options are flags on `CommitEditor` (today's bools, `update/commit_editor.rs:48-82`) keyed by `CommitOption`. `⌃S` keeps **toggling `sign_off`** (`keys.rs:135-137`, `control_key` :125); ops passes git's `--signoff` as today. While on, the header names each after the push state, in this order: `· signing off`, `· skipping hooks`, `· empty allowed`; nothing when all off. Amend shows as the header's `amending <sha7> <subject>` instead. `commit.toggle_*` from `⌃P` (every focus) and `?` (non-text focus: Files) keep acting through `toggle_from_palette`.
- **`commit.sign_off` description** (conductor, 2026-10-18): `"Insert a sign-off trailer"` → `"Sign off: on / off"` (03r's toggle wording) in `keymap/table/commit_editor.rs:28-31`, and the `Ctrl+S` row in `docs/design/02-keymap.md:529` to match. Id and key unchanged.
- **Review 03 Minor 5 (code half):** one id → option map, `CommitOption::of_id`; `toggle_from_palette` (`keys.rs:110-121`) calls it and flips through `option_mut` — its own match (:114-118) goes. `opens_the_palette` (`keys.rs:88-105`) calls `opener` once (today :94 and :101); text focus = `Subject | Body` (:93).
- **Amend (`a`, `open(amend=true)` :285 → `head_message_arrived` :325 → `CommitEditor::amend` :112)** uses the same screen: header `amending <sha7> <subject>`, message prefilled; staged files go in, others out; `include_of(.., touched=false)` → `Include::Staged` (message-only amend stays possible); `space` sets touched → `Include::Chosen`. `params()` (:153, `include: Include::Staged` at :159) sets `include_of`. The edit-stop amend at **`:344`** keeps `..CommitParams::default()` (= `Include::Staged`, ops `types.rs:74`) — no edit.
Out: what a commit writes (W), keymap rows and descriptions other than `commit.sign_off`'s, hint list words and the palette listing (03, 03b, 03r), the graph header's own layout (06b `header_tail`; this group only supplies the push words).

## Signatures
graft-ui `commit_editor`:
```rust
pub const SUBJECT_LIMIT: usize = 72;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitFocus { Subject, Body, Files }
impl CommitFocus { pub fn next(self) -> Self; pub fn previous(self) -> Self; }   // Subject → Body → Files → Subject
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
graft app `update::commit_editor`:
```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitOption { Amend, SignOff, NoVerify, AllowEmpty }
impl CommitOption {
    /// The one id → option map (`commit.toggle_amend`, `commit.toggle_no_verify`, `commit.toggle_allow_empty`).
    pub fn of_id(id: &ActionId) -> Option<CommitOption>;
    /// Header word while on; `None` for `Amend` (the header says `amending …`).
    pub fn header_word(self) -> Option<&'static str>;
}
impl CommitEditor { pub fn option_mut(&mut self, option: CommitOption) -> &mut bool; }   // replaces option_mut(Field) :252
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft { pub subject: String, pub body: String, pub amending: Option<Oid> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastCommit { pub commit: Oid, pub message: Draft }
/// Untouched amend → `Staged`. Otherwise `Chosen` from the rows going in.
pub fn include_of(choices: &[FileChoice], touched: bool) -> Include;
/// `Some(RowBar::JustCommitted)` while the cursor rests on the commit just made.
pub fn landed_bar(last: Option<&LastCommit>, cursor: Option<Oid>) -> Option<RowBar>;
```
`AppState` (`update/state.rs`): `commit_draft: Option<Draft>` (retyped at :252, was `Option<CommitEditor>`); `last_commit: Option<LastCommit>` (appended). `Oid` is the type `HeadFacts::Detached` carries. `CommitParams.message` joins subject and body as `params()` does today.

## Touches
`Field`/`OPTIONS`/`CommitEditorView`/`CommitEditorScreen` callers and tests (five integration sites above; unit tests in both crates); `footer_*` snapshots (replaced); `params()` :153 and `:344` (unchanged); `toggle_from_palette` and `update/palette/run.rs:52` (call kept); `opens_the_palette`; `control_key` ⌃S; `keymap/table/commit_editor.rs` `commit.sign_off` description and anything that pins it (keymap tests, `?`/`⌃P` listing snapshots, help captures — grep `Insert a sign-off trailer` across `crates/` and `docs/`); `docs/design/02-keymap.md` §2.10 `Ctrl+S` row (:529, reworded); `jobs::commit_message` (template, amend message); after-write status read; `operate/outcome.rs:121` `landed` and `committed()`; `landing_of` `operate.rs:192`; `update::undo` landed path and the journal screen's undo (draft only on the `last_commit` case; journal entry words `commit "<subject>"` unchanged); the row-`HintContext` choice site (07's; one arm); the graph top line push words (06b's `header_tail` — supply, don't redraw); tour card commit words (`view::tour`); 03b's `update::palette::editor_tests`; graft dead-key and palette sweeps; commit editor captures (recaptured); `crates/graft-ui/MAP.md` `## commit_editor` (SUBJECT_LIMIT 72, `Field` entries out, new types in) and `crates/graft/MAP.md` `update::commit_editor`.

## Acceptance
- [ ] Journey `commit-all-subject`: `c`, type `fix toc`, `⏎` → HEAD has a new commit with all 3 modified files, `.env` not in it; cursor on it; status starts `✓ Committed 3 files to` (state test; pty walk is A's).
- [ ] C1/C3 placeholders and C4 summary words exact (snapshot at 120); C2 counter reads `8/72`, warning style past 72.
- [ ] C5/C7: `.env` under `Staying out` with `!` and `looks like a secrets file`; `space` on it moves it to `Going in`.
- [ ] C9: empty subject + `⏎` → bar `type a message first`, HEAD unchanged; all files out + `⏎` → `put a file in first`; `⌃⏎` from Files with a message commits.
- [ ] C10/C11: `↓` in the subject moves the file cursor; `⇥ ⇥` reaches Files; `⇥` from Files → Subject; `esc` from Files → Subject; `⏎` from Files commits.
- [ ] `Field` gone: `grep -rn 'Field::\|CommitButton\|CommitEditorView' crates/` empty outside unrelated types; the five migrated integration tests pass under their names.
- [ ] C12: each of the three bars' words equal the mock's `:159` (snapshot per focus at 120).
- [ ] Journeys `commit-with-body` (6) and `commit-leave-first` (6) land as the note says (state tests).
- [ ] C13: after landing, top line contains `1 commit to push`; bar reads `u undo the commit, your edits come back · P push · c commit again` (state: hint context is `GraphRow { row: RowBar::JustCommitted, .. }`); `j` → the row's own bar; `k` → `JustCommitted` again; after an amend lands → not `JustCommitted`. `landed_bar` unit tests: `None` last, other oid, same oid.
- [ ] `LastCommit` cleared by another operation landing and by `c` (state tests, one each).
- [ ] Q9: `u` after the commit → HEAD back, edits uncommitted, `c` opens with `fix toc` in the subject and files re-chosen by `first_choice`.
- [ ] C15: `esc` with a typed subject and body, then `c` → both still there, options off; a draft with `amending: Some(head)` resumes on `a`, not on `c`.
- [ ] Q8: no option row drawn; `⌃S` → header ends `· signing off`; again → gone; a commit made with it on carries `Signed-off-by:` (integration); `commit.toggle_no_verify` from `⌃P`, and from `?` in Files, shows `· skipping hooks`; `?` in the subject types `?`.
- [ ] `commit.sign_off` description reads `Sign off: on / off` (keymap test); `grep -rn 'Insert a sign-off trailer' crates/ docs/design/` empty; `docs/design/02-keymap.md:529` row matches.
- [ ] Minor 5: `grep -c` of each of the three ids in `crates/graft/src/update/commit_editor*` outside tests = 1 (in `of_id`); choosing each `commit.toggle_*` twice from `⌃P` returns it to off (test per id); `opener` called once in `opens_the_palette` (by reading; named in NOTES).
- [ ] Amend: `a` → header `amending <sha7> <subject>`, message prefilled; `⏎` untouched → `Include::Staged` message-only amend; `space` on an out file → `Include::Chosen`.
- [ ] Hook failure: screen stays, message kept, failure on the status row.
- [ ] Captures next to the commit mock: `docs/shots/ease-pass/commit-open-120.txt`, `commit-secrets-120.txt`, `commit-files-focus-120.txt`, `commit-refused-120.txt`, `commit-options-120.txt` (`· signing off · skipping hooks`), `commit-90.txt`, `commit-landed-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~80k). Split point if the conductor splits: screen, `Field` retirement, options, sign-off description, Minor 5, C1–C12 here; `Draft`, `LastCommit`, landing arm, `u`, amend as a follow-on (that half also waits on 07's choice-site rewrite if 07 has not merged).
