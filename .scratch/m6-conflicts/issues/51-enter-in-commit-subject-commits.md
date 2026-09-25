# enter-in-commit-subject-commits: `⏎` in the commit editor's subject commits, as it does in the finish summary

Status: ready (unblocked 2026-09-24: P16 approved, picks item 14; ADR-0044 §3; keymap §2.10 edited in the spec pass.
Reconciled with scout 51 on 2026-09-25)
From: `docs/reviews/m6-usability.md` M2 (:77-86).
Build with: builder-opus. Why: brownfield in two crates; a private signature change and a footer that becomes
focus-dependent. Not Fable: it reuses the existing submit path, and the failing test compares against `⌃⏎`.
Scout: `.scratch/m6-conflicts/scouts/51.md` (main @ 3c480347). Read it before any source.
Needs: none by signature. It calls `commit(state, next_id)` (`update/commit_editor.rs:740`) as merged. Sequenced after
50 (both hold graft + graft-ui, ADR-0036). 52 also edits `update/commit_editor.rs` (`pane_c_acts`, next to
`live_operation`): take its result as the base; no function overlaps.
Crates: graft (`update/commit_editor.rs`, its tests, `update/tests.rs`, `view/frame/tests.rs`, `view/tests.rs`),
graft-ui (`commit_editor.rs`).
Lane: wave 2, fifth.

## Read first
- `docs/design-notes/m6-usability-fixes.md` P16 (:146-155), Journeys row "Stage one file and commit" (:60).
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §3 item 4.
- `docs/research/commit-stash.md` §1.1, §2 ("Enter vs commit"), §3 :169-171, §4.
- `docs/design/02-keymap.md` §2.6b and §2.10 (both edited 2026-09-24).
- `docs/design-notes/m3-working-tree-undo-and-stash.md` :159-166.
- `crates/graft/MAP.md` §`update::commit_editor` (`editor_key`, `CommitEditor::refusal`, `committed`);
  `crates/graft-ui/MAP.md` line 11 and §`commit_editor` (:304-315; :315 is the summary's subject/body bars, the model).
- Source, after the MAPs, at scout 51's lines (the old spec's :595-612 is stale): `subject_key` :655-671,
  `open_editor_key` :503-545, `editor_key` :479-498, `commit` :740; graft-ui `commit_editor.rs:171, :212-216`.

## Scope
**In:**
- `subject_key`: `KeyCode::Enter` → `commit(state, next_id)`, the function `⌃⏎` (`control_key` :558) and the button
  (`button_key` :728) call. `Down` still → `Field::Body`. `Tab`/`BackTab` are handled before the per-field dispatch
  (:526-533) and are not touched. The body's `Enter` stays a newline.
- **The signature changes** (the old spec's "unchanged" cannot hold): `subject_key` has no `next_id` and `commit` needs
  one. `open_editor_key` already has it (it calls `control_key`); pass it through at :540-545.
- **"Operation live" is two cases; both do exactly what `⌃⏎` does today from the same state:**
  - **(a) A job running** (`state.operating`): `editor_key` eats every key but `Esc` at :490-498, before
    `subject_key`. `⏎` is eaten there too: nothing sent, focus and draft unchanged. **No code change** for this case;
    a test pins it.
  - **(b) A merge, rebase, cherry-pick, revert or bisect in progress** (`live_operation(state).is_some()`, reached if one
    began while the editor was open): `⏎` reaches `commit` → `CommitEditor::refusal` (:266, first in its order) →
    nothing sent. The refusal row shows the live-operation words; it is drawn from the view at all times
    (graft-ui :146, :235), so nothing is "triggered" — the test asserts the row's text, not a change.
  - Nothing staged and an empty subject go the same way as (b).
  - In every refused case focus stays on the subject (no jump to the body).
- **The footer follows focus** (decided here; pick 14 "⏎ commits; ⇥ goes to the body"; UX §2 #6, keys visible at the
  point of use; the summary's subject/body bars, graft-ui MAP :315, are the model):
  - subject focused: `⏎ commit  ⇥ body  ⌃E $EDITOR  ⌃S sign-off  esc cancel`
  - any other field: today's string, unchanged: `⌃⏎/Commit commit  ⇥ next  ⌃E $EDITOR  ⌃S sign-off  esc cancel`
  - The word stays `commit` in the amend editor too (decided here; the old spec's `⏎ amend` is dropped): today's footer
    and the button both say commit in both editors, and the footer is one word per key, not per mode.
  - `view.status` still takes the row when set (:212-215), unchanged.

**Out:**
- The finish summary (`finish-summary-is-a-preview`); `finish/keys.rs`.
- `⌃⏎` delivery in Zed (KNOWN_GAPS :206).
- Review m8 (`subject empty` shown on open).
- The amend editor's prefill; an `amend` verb in either footer.
- Sweeping the editor footer with `every_key_the_bar_draws_is_answered`: it is not a hint context (`every_context`,
  `catalogue.rs:34`, has none), and adding one is a scope change.

## The failing test first
`graft/src/update/commit_editor/tests.rs`: `enter_in_the_subject_commits`.
- Setup: something staged, the editor open, subject `fix: x` typed, focus on the subject. Press `Enter`.
- Assert: the commands equal those `⌃⏎` emits from a clone of the same state: one
  `Command::Operate(Operation::Commit(..))`, message `fix: x`.
- Red today: focus moves to the body, nothing emitted.

## Signatures
```rust
// graft/src/update/commit_editor.rs: private signature changes
fn subject_key(state: &mut AppState, key: &KeyPress, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
//   next_id: the exact type `commit` takes (:740); if it differs from the line above, use `commit`'s.
//   KeyCode::Enter => commit(state, next_id),   KeyCode::Down => editor.focus = Field::Body
// open_editor_key :540-545: the Subject arm passes next_id.

// graft-ui/src/commit_editor.rs: the one constant (:171) becomes two, chosen by focus
const FOOTER_SUBJECT: &str = "⏎ commit  ⇥ body  ⌃E $EDITOR  ⌃S sign-off  esc cancel";
const FOOTER_OTHER: &str = "⌃⏎/Commit commit  ⇥ next  ⌃E $EDITOR  ⌃S sign-off  esc cancel"; // today's FOOTER
fn footer(focus: Field) -> &'static str;   // Field::Subject → FOOTER_SUBJECT, else FOOTER_OTHER; drawn at :216
```

## Touches (each entry, and how the builder checks it)
- **A body typed first.** `⏎` in the subject commits subject + blank line + body (`CommitEditor::message`).
  Check: `enter_in_the_subject_commits_the_body_too`.
- **`commit.template`** (m3 note :165). Check: `enter_with_a_template_commits_what_ctrl_enter_would`.
- **Refusals.** Check: `a_refused_enter_sends_nothing`, rows: nothing staged; empty subject; a merge in progress
  (case b). Each row: commands equal `⌃⏎`'s from the same state (empty), focus `Subject`, draft unchanged, the refusal
  row's text as `⌃⏎` leaves it.
- **A job running (case a).** Check: `enter_while_a_job_runs_is_eaten_like_ctrl_enter`: commands empty, editor open,
  focus and draft unchanged; `Esc` still answered.
- **Amend** (`a`). Check: `enter_in_the_subject_amends_in_the_amend_editor`; its footer still reads `⏎ commit`.
- **Paste.** `paste` (:609) → `insert_str` drops newlines; never reaches `subject_key`.
  Check: `a_pasted_newline_does_not_commit`.
- **Tests pinning the old rule** (`commit_editor/tests.rs`):
  - `enter_in_the_subject_moves_to_the_body_and_inserts_nothing` (:104) → replaced by
    `down_and_tab_still_move_to_the_body`.
  - `enter_on_the_button_commits_the_subject_and_the_body` (:142) presses `Enter` to reach the body (a comment, not a
    `Field::Body` match, so the old grep missed it) → press `Down`/`Tab` instead; its assertion unchanged.
  - Check also `grep -rn "Enter" crates/graft/src/update/commit_editor` for any other subject-`Enter`; NOTES lists
    each change. Other `Enter` uses at :164-779 are `control(Enter)` (scout 51) and stay.
- **Footer pins:**
  - `view/frame/tests.rs:600`: `BEHIND` needle `"⌃⏎/Commit"`. The editor opens on the subject, so that frame now
    shows `⏎ commit`. Update the needle to what the frame's focus draws; NOTES says which.
  - `view/tests.rs:2364` doc line; `update/tests.rs:4021` "⌃⏎" press map: add `⏎` → `Enter` for the subject form if
    the map needs it.
  - Check: `the_footer_follows_focus` (graft-ui: subject → `FOOTER_SUBJECT`; body, options, button → `FOOTER_OTHER`),
    and the pins above green.
- **Captures** (scout 51's list): `m3-04-commit-editor-{80x24,160x48}`, `m3-05-commit-refused-{80x24,160x48}`,
  `m3-amend`, `m3-commit-editor-{80x24,160x48}`, `m3-commit-editor-refused`,
  `m6/fix-editor-names-the-banner-{80x24,120x40}`. Each regenerated by its own test; NOTES lists file → test. A
  capture with the body focused keeps its footer byte-identical. The m3 and m6 contact sheets are the conductor's.
- **Walks.** Check: `grep -rn "⇥ ⇥ ⇥\|Tab.*Tab.*Tab" .scratch/*/pty-walk*.py crates/graft/tests/it` (scout 51 could
  not); NOTES lists the hits. The existing `⇥` legs stay (they still work). Add one leg committing by `⏎` from the
  subject; record the observed count for "stage one file and commit" (expect 6).
- **Other screens drawing the same state:** the finish summary's subject: unchanged. Check: `finish/keys.rs` not in
  `git diff --stat`.
- **Hint tests:** `every_key_the_bar_draws_is_answered`, `every_context` coverage: run, expect green and unchanged.
- **Journal and undo:** none new. Check: the failing test's command equality.
- **Keymap:** already edited (§2.10 `Enter` row). Read, no edit.
- **Design note** (conductor edits): `m3-working-tree-undo-and-stash.md` :165 marked superseded by P16.
- **TESTING.md:** grep `moves to the body\|Enter in Subject\|⇥ ⇥ ⇥`; NOTES lists the lines for the acceptance ticket.
- **MAP.md:** graft §`update::commit_editor` (`subject_key`'s `Enter`); graft-ui §`commit_editor` (the two footers).

## Acceptance (name the verifying test at tick time)
- [ ] `enter_in_the_subject_commits`: red first, then green.
- [ ] `enter_in_the_subject_commits_the_body_too`
- [ ] `enter_with_a_template_commits_what_ctrl_enter_would`
- [ ] `a_refused_enter_sends_nothing` (three rows)
- [ ] `enter_while_a_job_runs_is_eaten_like_ctrl_enter`
- [ ] `enter_in_the_subject_amends_in_the_amend_editor`
- [ ] `a_pasted_newline_does_not_commit`
- [ ] `down_and_tab_still_move_to_the_body`, `enter_in_the_body_is_a_newline`
- [ ] `the_footer_follows_focus`
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-subject-enter-commits-120x40.txt` and
      `-80x24`.
  - Scene: the editor with `fix: x` typed, subject focused; second frame after `⏎`.
  - **What right looks like:** frame 1's footer reads `⏎ commit  ⇥ body  ⌃E $EDITOR  ⌃S sign-off  esc cancel`, whole
    at both sizes; the body is empty; the options row ends `[ Commit ]`. Frame 2: the editor is gone and the status
    line names the new commit.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small to medium. One context.

## NOTES
