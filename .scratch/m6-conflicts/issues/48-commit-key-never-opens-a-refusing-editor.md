# commit-key-never-opens-a-refusing-editor: `c`/`a` with an operation live answer in words and never open the editor to refuse

Status: ready
From: `docs/reviews/m6-usability.md` M3 (:88-97), the part that needs no answer.
Blocked by: none. Keymap :309 already says "Commit and amend refuse … and name the same keys". This moves the refusal out
of an editor that exists only to refuse. P14 (`c` → the banner's `c`) builds on it later
(`commit-key-finishes-the-live-operation`).
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: every caller of `commit_editor::open` (keys, the `.` menu, clicks, the `a` head-message path), the `u`
refusal's `Status` variant and words, and every test, walk and capture that opens the editor while an operation is live.
Needs: none. It calls `graft::update::commit_editor::{live_operation, CommitEditor::refusal}`, as merged.
Crates: graft.

## Read first
- `docs/design-notes/m6-usability-fixes.md`: "Decided here" 2, the Journeys row "Finish a merge from the pane", and the
  conductor edits at the end.
- `docs/design/02-keymap.md` §2.4 :239, §2.6a :278-310.
- `.scratch/m6-conflicts/issues/39-editor-words-name-the-banner.md`: Step 0 (the pin this spec reverses) and NOTES
  (`n` = 2).
- `crates/graft/MAP.md` §`update::commit_editor` (`refusal`, `Begun`, `live_operation`, `open`, `head_message_arrived`).
- Source, only after the MAP: `graft/src/update/commit_editor.rs:195-246`, `:278-340`; `update/keys.rs` around
  `is_commit_key` (`keys/predicates.rs:114-123`).

## Scope
**In:**
- **`open` checks `live_operation(state)` first.** This is the cause: `open` never looks at `in_progress`,
  `commit_editor.rs:289-318`. When an operation is live:
  - the editor is not opened;
  - no `ReadCommitTemplate` or `ReadHeadMessage` is sent;
  - `commit_draft` is untouched;
  - the status line gets the pane form of the refusal, with the same `Status` variant the `u`/`U` refusal uses (keymap
    :305-306).
- **`head_message_arrived`:** if an operation became live while `a`'s read was in flight, it refuses the same way
  instead of opening.
- **One function makes both word forms:**
  - the editor form, unchanged: `… — esc, ⇥ banner, then …`;
  - the pane form, without `esc, `.
  - The exact words are in the design note, "Decided here" 2.
- **The walk's `editor-to-finish` scenario** starts in the changes pane: `c` (refused) `⇥ ⇥ c ⏎`. The count stays 5.

**Out:**
- What `c` does beyond refusing (P14).
- The graph or pane bar still showing `c commit` during an operation. List it in NOTES as a finding for the conductor;
  don't change hints.
- The editor's refusal row itself (ticket 40). It stays for an editor already open when an operation begins, for
  example one started in the shell.

## The failing test first
`crates/graft/tests/it/resolver/editor_words.rs`: `c_and_a_with_a_merge_live_answer_in_words`.
- Setup: 39's InGraft and InShell states, changes pane focused.
- Press `c`, then assert:
  - `commit_editor.is_none()`;
  - no `ReadCommitTemplate` emitted;
  - the status is exactly `merge in progress — ⇥ banner, then c continue  a abort` (InGraft) or
    `merge begun in the shell — ⇥ banner, then c finishes it` (InShell).
- Press `a`, then assert: no `ReadHeadMessage`, the same words.
- Red on today's code: the editor opens.
- **Reverses 39's pin** `after_esc_the_panes_c_and_a_are_commit_and_amend`. That test is rewritten to this rule, not
  deleted, and NOTES says so.

## Signatures
```rust
// graft/src/update/commit_editor.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusedFrom { Editor, Pane }
/// Why a commit cannot start while `live` is open, worded for where the user stands.
#[must_use] pub fn operation_refusal(live: (InProgress, Begun), from: RefusedFrom) -> String;
// CommitEditor::refusal's operation arms call operation_refusal(.., RefusedFrom::Editor): words unchanged.
pub fn open(state: &mut AppState, amend: bool, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>; // unchanged signature
```

## Touches (each entry, and how the builder checks it)
- **Callers of `open`.**
  - Check: `grep -rn "commit_editor::open\|open(state, .*amend" crates/graft/src`. Each caller (the `c`/`a` keys, the
    `.` menu if it has Commit or Amend, any click) is covered by one row of `every_way_to_open_the_editor_refuses_mid_operation`.
- **The draft.**
  - Check: `a_refused_c_keeps_the_draft`: a draft set, then `c` mid-merge, and afterwards `commit_draft` is the same
    value.
- **The amend race.**
  - Check: `head_message_arriving_mid_operation_does_not_open`.
- **Bisect and shell rebase words** (no banner keys): the pane form equals the editor form for these arms.
  - Check: `the_pane_form_drops_only_esc`, a table test over every `(InProgress, Begun)`.
- **Width.**
  - Check: `the_pane_words_fit_at_80`: the longest pane form (`cherry-pick in progress — ⇥ banner, then c continue  a abort`)
    is whole on the 80×24 status line.
- **39's tests.** `after_esc_the_panes_c_and_a_are_commit_and_amend`, `the_keys_the_commit_editor_names_reach_the_summary`
  and `the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge` open the editor from the pane
  mid-operation.
  - Rewrite each to open the editor first and then make the operation live (the shell route). Or, where it tests the
    route, start it from the pane's refusal. NOTES lists which is which.
- **Ticket 40's refusal-row captures and tests** (`fix-editor-*` under `docs/shots/m6/`).
  - Check: `grep -rln "esc, ⇥ banner" docs/shots crates`. Each capture that reached the editor by `c` mid-operation
    now gets its state by construction, and must come out byte-identical. A changed byte is a stop: report it.
- **The walk.** `crates/graft/tests/it/m5_walk/conflict_journeys.rs` (`editor-to-finish`) and
  `.scratch/m6-conflicts/pty-walk-m6.py`: the key string becomes `c ⇥ ⇥ c ⏎`, starting from the changes pane, still 5.
  - Check: `journey_counts_match_the_note` is green with the note's `keys` column unchanged (5).
- **Other screens that draw the same state.** The banner (`every file done ✓ · c finishes the merge`) and the `u`
  refusal: unchanged.
  - Check: grep the literals, no diff.
- **Journal and undo.** None: a refusal writes nothing.
  - Check: the failing test asserts no `Command::Operate`.
- **Keymap.** No edit: :309 already says they refuse. **TESTING.md** (the conductor edits it): :2138 and :2332, text below.
- **MAP.md:** graft §`update::commit_editor`: `open`, `operation_refusal`, `RefusedFrom`, and the `Begun` line's route
  sentence.

## Acceptance (name the verifying test at tick time)
- [ ] `c_and_a_with_a_merge_live_answer_in_words`: red first (the editor opens), then green.
- [ ] `every_way_to_open_the_editor_refuses_mid_operation`: the callers from the grep, in NOTES.
- [ ] `a_refused_c_keeps_the_draft`
- [ ] `head_message_arriving_mid_operation_does_not_open`
- [ ] `the_pane_form_drops_only_esc`
- [ ] `the_pane_words_fit_at_80`
- [ ] 39's three tests rewritten and green; NOTES says how each changed.
- [ ] `journey_counts_match_the_note` green; `editor-to-finish` walked as `c ⇥ ⇥ c ⏎` = 5.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-c-mid-merge-answers-in-words-187x50.txt` and
  `-80x24`.
  - Scene: a shell-begun merge, resolved, the changes pane focused, `c` pressed.
  - **What right looks like:** no editor is drawn. The status line reads exactly
    `merge begun in the shell — ⇥ banner, then c finishes it`. The header reads `MERGING`.
- [ ] MAP.md updated (doc, no test).

## For the conductor (exact text)
- `TESTING.md:2138`: replace `The commit editor never finishes an operation: \`c\`/\`a\` in the panes stay commit/amend
  (keymap §2.4), and its words name the way to the banner — \`esc, ⇥ banner, then c\` (ticket 39).` with
  `\`c\`/\`a\` in the panes never open the editor while an operation is live: the status line names the way to the
  banner — \`⇥ banner, then c …\` (usability fix M3a).`
- `TESTING.md:2332`, the `editor-to-finish` row: the start becomes `… resolved in the shell, the changes pane focused`,
  the keys `c ⇥ ⇥ c ⏎`, the count 5, and the check `the first \`c\` answers in words, no editor; \`⇥ ⇥\` reaches the banner`.
- `m6-conflicts.md` :483 and :505: the text in `docs/design-notes/m6-usability-fixes.md`, "For the conductor".

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small to medium. One context.

## NOTES
