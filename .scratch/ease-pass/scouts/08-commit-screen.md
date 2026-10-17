# Scout 08 — commit screen (main 2610bce9)

## State, focus, options
- graft-ui `Field` enum `commit_editor.rs:41-56` (Subject, Body, Amend, SignOff, NoVerify, AllowEmpty, **CommitButton**); `OPTIONS` :60, `next`/`previous` :91/:105 (Tab cycles Subject→Body→Amend…→CommitButton), `sibling_option` :120. `SUBJECT_LIMIT = 50` :31, `BODY_LIMIT = 72` :34.
- App state `CommitEditor` `graft/src/update/commit_editor.rs:48-82`: `subject: TextField`, `body: TextArea`, `focus: Field`, bools `amend/sign_off/no_verify/allow_empty`, `head_short`, `template_applied`, `from_draft`, `subject_refused`, `reword_line`. `option_mut` :252, `options()` :264, `params()` :153 (`include: Include::Staged` :159), `operation()` :165.
- `a`: `open(amend=true)` :285 sends `ReadHeadMessage`; `head_message_arrived` :325 builds `CommitEditor::amend(short, &message)` :112. Edit-stop amend uses `..CommitParams::default()` at :344 (Include default = `Staged`, ops `types.rs:74`).
- Hook failure: editor not closed (only `committed()` closes, called from `operate/outcome.rs:131` on landed); view status = `super::status::problem(state.shown_status())` `view/commit_editor.rs:56`; `Esc` clears Failed/Sticky first `keys.rs:47-53`.

## Key routing (all in `graft/src/update/commit_editor/keys.rs`)
- `open_editor_key` :35: palette check :40 → any ctrl → `control_key` :125 → Esc/Tab/BackTab :47-67 → **match on `Field` focus** :76-80 (subject_key/body_key/button_key/option_key). No action ids anywhere; option rows take `space`/`⏎` via `option_key` :305-307.
- `⏎` in subject: literal `KeyCode::Enter` in `subject_key` :253 → `commit` :348. `⌃⏎` :134 and button :336 likewise. Keymap id `commit.commit` = `⌃⏎` (`keymap/table/commit_editor.rs:9-11`).
- `⌃S`: id `commit.sign_off` (`table/commit_editor.rs:28-31`, desc "Insert a sign-off trailer") but code **toggles the `sign_off` flag** `keys.rs:135-137`, routed by literal `Char('s')`.
- `toggle_from_palette` `keys.rs:110-121` (own id→flag match :114-118, duplicate of `option_mut`), re-exported `commit_editor.rs:37`, called from `update/palette/run.rs:52`.
- `opens_the_palette` `keys.rs:88-105`: `opener(*key)?` at :94 and again :101; `types` = focus Subject|Body :93.

## Data
- Working-tree status held at `AppState::working_tree: Option<Arc<WorkingTreeStatus>>` `update/state.rs:96`; `commit()` already reads it `keys.rs:349`.
- One file's diff: `jobs/detail.rs:114 read_file_diff(repos, &DiffKey, &FileEntry, stop)` via `diff_spec_for`; `DiffSpec::{Staged, Unstaged, WorkingTreeVsHead}` (`:63-67` WorkingTreeVsHead lists both).
- graft-ops: `Include`/`Chosen`/`Part` `operation/types.rs:72-100`; `FileChoice` :22, `OutWhy` :13, `first_choice(&WorkingTreeStatus)` :39 in `operation/choice.rs`; `looks_like_secrets` referenced `choice.rs:52`; `Refusal::NothingChosen` `commit.rs:32,49`.

## Landing and undo
- Success: `outcome.rs:121 landed` → `committed()` (`keys.rs:236`, clears editor **and** `commit_draft`) → refresh → `landing_of` (`operate.rs:192`, Commit/Amend → `Landing::Head` via `moves_head`). Status text from `applied.entry.description` (`outcome.rs:~115`).
- `RowBar::JustCommitted` exists in graft-ui (`hints/context/row_bar.rs:34`, words `lists/rows.rs:125`) but **no `RowBar` use anywhere in `crates/graft/src`** (grep) — choice site not present on main.
- `u`: `update/undo*` never touches `commit_draft`/`commit_editor` (grep); draft cleared at commit, so no message survives undo today.
- Draft type is `commit_draft: Option<CommitEditor>` `state.rs:252`; `close()` keeps it `keys.rs:231`. No `Draft` struct.

## Footer callers
`footer_subject` :178 / `footer_other` :192 / `footer` :212 — no callers outside `graft-ui/src/commit_editor.rs` (+ its tests, MAP.md:238-239). Ahead counts exist only in `graph_screen/badges.rs:80-106`.
`Field` used by integration tests: `tests/it/commit_editor_writes.rs:182,402`, `daily_landing.rs:106`, `m3_walk.rs:295`, `remote_by_key.rs:262`.

## Contradictions
1. Spec "⌃S inserts the sign-off as today" — code toggles a flag (`keys.rs:135-137`); keymap description says "Insert". Retiring the option rows leaves sign-off as an invisible flag.
2. Spec `Field` lines `:41-53` — enum runs :41-56 and includes `CommitButton` (unmentioned; integration tests above depend on it).
3. Edit-stop amend "`:340`" — actually :344 and uses `CommitParams::default()`, not literal `Include::Staged`.
4. Spec `Draft {subject, body}` — today's draft is the whole `CommitEditor` (`state.rs:252`); `committed()` drops it (`keys.rs:238`), so Q9 needs new keep-on-commit state.
5. `JustCommitted` choice site "(G's)" — not present in graft on main; could not confirm where it will be.
6. MAP.md:227 says SUBJECT_LIMIT 50 (matches code; spec changes to 72 — MAP update needed).
