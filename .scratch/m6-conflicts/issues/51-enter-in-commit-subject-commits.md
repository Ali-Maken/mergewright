# enter-in-commit-subject-commits: `⏎` in the commit editor's subject commits, as it does in the finish summary

Status: blocked (user answer)
From: `docs/reviews/m6-usability.md` M2 (:77-86).
Blocked by: user answer on P12 (`m6-conflicts.md` :1352) **and** on P16 (`docs/design-notes/m6-usability-fixes.md`).
- The rule "`⏎` in a subject commits" holds whichever way P12 goes. Both P12 and today's §2.6b keep the summary's
  subject `⏎` as commit (keymap :321; P12's note, `m6-conflicts.md` :1356).
- It is still a new key meaning for the editor, so P16 is needed. Present P12 and P16 to him together: both answer
  "what does `⏎` do where I type a message".
- If he answers P16 with "neither commits", this spec is void. The conductor asks the planner for a replacement.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: in the commit editor, list which keys each field answers, which function the `[ Commit ]` button and
`Ctrl+Enter` call, where the editor's key footer is drawn, and what pins that footer (captures, hint tests).
Needs: none. It calls the editor's existing submit path, as merged. The scout names it.
Crates: graft, graft-ui.

## Read first
- `docs/design-notes/m6-usability-fixes.md`: P16, and the Journeys row "Stage one file and commit".
- `docs/research/commit-stash.md` §1.1 (lazygit: `Enter` commits from the summary, `Tab` goes to the description,
  `commit_message_controller.go:~33`), §2 ("Enter vs commit", the two camps), §3 :169-171 (the inconsistency, raised as
  a question), §4.
- `docs/design/02-keymap.md` §2.6b :312-327 and §2.10 :361-370.
- `docs/design-notes/m3-working-tree-undo-and-stash.md` :159-166.
- `crates/graft/MAP.md` §`update::commit_editor` (`editor_key`, `CommitEditor::refusal`, `committed`);
  `crates/graft-ui/MAP.md` §`commit_editor`.
- Source, only after the MAPs: `graft/src/update/commit_editor.rs:595-612` (`subject_key`), and the submit path the
  scout names.

## Scope
**In:**
- `subject_key`: `KeyCode::Enter` submits exactly as `Ctrl+Enter` does (the same function), and `Down` still moves to
  the body. The body's `Enter` stays a newline.
- A refused `⏎` does what a refused `Ctrl+Enter` does today: nothing is sent, and the refusal row shows. That covers
  nothing staged, an empty subject, and an operation live.
- The editor's footer in subject focus names `⏎ commit` first. In body focus it keeps `⌃⏎ commit` (the summary's
  subject and body bars are the model: graft-ui MAP line 315).

**Out:**
- The finish summary (P12's spec).
- `Ctrl+Enter` delivery in Zed (KNOWN_GAPS :206).
- Review m8 (`subject empty` shown on open): a separate Minor. But see the Touches entry on refusals.
- The amend editor's prefill.

## The failing test first
`graft/src/update/commit_editor/tests.rs`: `enter_in_the_subject_commits`.
- Setup: something staged, the editor open, subject `fix: x` typed, focus on the subject. Press `Enter`.
- Assert: the same `Command::Operate(Operation::Commit(..))` that `Ctrl+Enter` emits from the same state, with the
  message `fix: x`.
- Red on today's code: focus moves to the body and nothing is emitted.

## Signatures
```rust
// graft/src/update/commit_editor.rs: signatures unchanged
fn subject_key(state: &mut AppState, key: &KeyPress) -> Vec<Command>;
//   KeyCode::Enter => <the same submit Ctrl+Enter calls>,   KeyCode::Down => editor.focus = Field::Body
// graft-ui: the commit editor footer's subject-focus list leads with `⏎ commit`
```

## Touches (each entry, and how the builder checks it)
- **A body typed first.** A draft with a body, then `⏎` in the subject, commits subject + blank line + body
  (`CommitEditor::message`).
  - Check: `enter_in_the_subject_commits_the_body_too`.
- **`commit.template`.** The template fills the body (m3 note :165). `⏎` in the subject commits whatever the body
  shows, with comments stripped exactly as `Ctrl+Enter` strips them.
  - Check: `enter_with_a_template_commits_what_ctrl_enter_would`.
- **Refusals.**
  - Check: `a_refused_enter_sends_nothing`, with three rows: nothing staged, empty subject, operation live. Each shows
    the same refusal row `Ctrl+Enter` shows.
- **Amend** (`a`). `⏎` in the amend editor's subject amends.
  - Check: `enter_in_the_subject_amends_in_the_amend_editor`.
- **Paste.** A paste into the subject still drops newlines (MAP `paste`). A pasted `\n` must never submit.
  - Check: `a_pasted_newline_does_not_commit`.
- **Tests pinning the old rule.**
  - Check: `grep -rn "Field::Body" crates/graft/src/update/commit_editor` for tests that press `Enter` in the subject
    and expect the body. Each is replaced by a `Down`/`Tab` equivalent; NOTES lists them.
- **Walks and journeys.**
  - Check: `grep -rn "Tab.*Tab.*Tab\|⇥ ⇥ ⇥" .scratch/*/pty-walk*.py crates/graft/tests/it`. The M3 walk's commit
    steps may still pass (`⇥` path). Add one leg that commits by `⏎`. NOTES records the observed count, 6 for "stage
    one file and commit".
- **Other screens that draw the same state.** The finish summary's subject: unchanged, and now the same rule.
  - Check: `finish/keys.rs` is not in the diff.
- **Captures.** The editor's footer changes.
  - Check: `grep -rln "⌃⏎ commit\|commit editor" docs/shots` (for example `m3-05-*`, `fix-editor-*`). Each is
    regenerated by its own test; NOTES lists them. The m3 and m6 contact sheets are the conductor's to rebuild.
- **Hint tests.** `every_key_the_bar_draws_is_answered` and `every_context` coverage.
  - Check: run them.
- **Journal and undo.** None new: the same commit operation, the same journal entry.
  - Check: the emitted `Command` is equal to the `Ctrl+Enter` one (the failing test).
- **Keymap** (the conductor edits it): §2.10 gains `| Enter | In the subject: commit (as §2.6b). In the body: a newline |`;
  `Tab` is unchanged.
- **Design note** (the conductor edits it): `m3-working-tree-undo-and-stash.md` :165, "Enter in Subject moves to Body",
  is marked superseded by P16.
- **TESTING.md.** The builder greps `TESTING.md` for `moves to the body\|Enter in Subject\|⇥ ⇥ ⇥` and lists the lines in
  NOTES for the conductor.
- **MAP.md:** graft §`update::commit_editor` `editor_key`, and the graft-ui commit editor footer.

## Acceptance (name the verifying test at tick time)
- [ ] `enter_in_the_subject_commits`: red first, then green.
- [ ] `enter_in_the_subject_commits_the_body_too`
- [ ] `enter_with_a_template_commits_what_ctrl_enter_would`
- [ ] `a_refused_enter_sends_nothing` (three rows)
- [ ] `enter_in_the_subject_amends_in_the_amend_editor`
- [ ] `a_pasted_newline_does_not_commit`
- [ ] `down_still_moves_to_the_body` and `enter_in_the_body_is_a_newline`
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-subject-enter-commits-120x40.txt` and `-80x24`.
  - Scene: the editor with `fix: x` typed, subject focused.
  - **What right looks like:** the footer starts `⏎ commit`. The body is empty. The options row ends `[ Commit ]`.
  - Second frame, after `⏎`: the editor is gone, and the status line names the new commit.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small to medium. One context.

## NOTES
