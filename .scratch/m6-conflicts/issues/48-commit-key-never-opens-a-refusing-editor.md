# commit-key-never-opens-a-refusing-editor: `c`/`a` with an operation live answer in words and never open the editor to refuse

Status: done
From: `docs/reviews/m6-usability.md` M3 (:88-97), the part that needs no answer.
Blocked by: none. Keymap :309 already says "Commit and amend refuse … and name the same keys". This moves the refusal out
of an editor that exists only to refuse. P14 (`c` → the banner's `c`) builds on it later
(`commit-key-finishes-the-live-operation`).
Build with: builder-sonnet. Routing and words with one production caller (`keys.rs:280`); the rule is fixed below.
Needs: none. It calls `graft::update::commit_editor::{live_operation, CommitEditor::refusal}`, as merged.
Crates: graft.

## Read first
- `docs/design-notes/m6-usability-fixes.md`: "Decided here" 2, the Journeys row "Finish a merge from the pane", and the
  conductor edits at the end.
- `.scratch/m6-conflicts/scouts/48.md` (the code facts this spec is amended from).
- `docs/design/02-keymap.md` §2.4 :239, §2.6a :278-310.
- `.scratch/m6-conflicts/issues/39-editor-words-name-the-banner.md`: Step 0 (the pin this spec reverses) and NOTES
  (`n` = 2).
- `crates/graft/MAP.md` §`update::commit_editor` (`refusal`, `Begun`, `live_operation`, `open`, `head_message_arrived`).
- Source, only after the MAP: `graft/src/update/commit_editor.rs:58-66`, `:188-246`, `:289-335`;
  `update/undo.rs:103-109` (the `u` refusal); `view/status.rs:40-89` (how `Sticky` draws).

## The status form (decided; do not reinterpret)
The pane refusal is the `u` refusal's variant, `Status::Sticky { reason, keys }` (`undo.rs:105-108`), split where the
editor's words have their em dash:
- InGraft: `reason` = `"{kind} in progress"`, `keys` = `Some("⇥ banner, then c continue  a abort")`.
- Merge begun in the shell: `reason` = `"merge begun in the shell"`, `keys` = `Some("⇥ banner, then c finishes it")`.
- Bisect, and a rebase, cherry-pick or revert begun in the shell: `reason` = today's whole editor sentence, `keys` =
  `None` (no banner keys to name).

It draws as every `Sticky` does, never a new format:
- the graph footer (`graph_problem`, `status.rs:85-88`): reason and keys apart, keys kept whole
  (`fit_status_line`); `esc dismiss` where `keys` is `None`;
- the one-line form (`problem`, `status.rs:45-48`): `merge begun in the shell · ⇥ banner, then c finishes it`.

The editor's own row is unchanged, byte for byte: `{reason} — esc, {keys}`, or `reason` alone where `keys` is `None`.
One value makes both, so they cannot drift.

## Scope
**In:**
- **`open` checks `live_operation(state)` first.** This is the cause: `open` never looks at `in_progress`
  (`commit_editor.rs:289-318`, scout-confirmed). When an operation is live:
  - the editor is not opened;
  - no `ReadCommitTemplate` or `ReadHeadMessage` is sent;
  - `commit_draft` is untouched;
  - `state.status = operation_refusal(live).status()`.
- **`head_message_arrived`** (`:321-335`, called from `mod.rs:461`): if an operation became live while `a`'s read was in
  flight, it refuses the same way instead of opening.
- **`CommitEditor::refusal`'s operation arms** call `operation_refusal(live).editor_words()`.
- **The walk's `editor-to-finish`** starts in the changes pane: `c` (refused) `⇥ ⇥ c ⏎`, still 5 (below).

**Out:**
- What `c` does beyond refusing (P14).
- The graph or pane bar still showing `c commit` during an operation (`keys/bound.rs:47-48`). List it in NOTES as a
  finding for the conductor; don't change hints.
- The editor's refusal row itself (ticket 40). It stays for an editor already open when an operation begins, for
  example one started in the shell.

## The failing test first
`crates/graft/tests/it/resolver/editor_words.rs`: `c_and_a_with_a_merge_live_answer_in_words`.
- Setup: 39's InGraft and InShell states, changes pane focused.
- Press `c`, then assert:
  - `commit_editor.is_none()`;
  - no `ReadCommitTemplate` emitted;
  - `status == Status::Sticky { reason: "merge in progress", keys: Some("⇥ banner, then c continue  a abort") }`
    (InGraft), or `{ reason: "merge begun in the shell", keys: Some("⇥ banner, then c finishes it") }` (InShell).
- Press `a`, then assert: no `ReadHeadMessage`, the same status.
- Red on today's code: the editor opens.

## Signatures
```rust
// graft/src/update/commit_editor.rs
/// Why a commit cannot start while an operation is live: a reason, and the keys that lead on from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationRefusal {
    pub reason: String,
    /// `None` where there is no banner to point at (bisect; a shell-begun rebase, cherry-pick or revert).
    pub keys: Option<String>,
}
impl OperationRefusal {
    /// The editor's row: `{reason} — esc, {keys}`, or `reason` alone. Today's words, byte for byte.
    #[must_use] pub fn editor_words(&self) -> String;
    /// The pane's answer: `Status::Sticky { reason, keys }`, the `u` refusal's variant.
    #[must_use] pub fn status(&self) -> Status;
}
#[must_use] pub fn operation_refusal(live: (InProgress, Begun)) -> OperationRefusal;
pub fn open(state: &mut AppState, amend: bool, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>; // unchanged signature
```

## Touches (each entry, and how the builder checks it)
- **Callers of `open`:** `update/keys.rs:280` is the only production caller; `operate.rs:2278` is a test whose state has
  `in_progress: None` (unaffected). No `.` menu Commit/Amend, no mouse caller (scout grep).
  - Check: re-run `grep -rn "commit_editor::open" crates/graft/src`; paste it in NOTES.
    `every_way_to_open_the_editor_refuses_mid_operation` has one row per production hit (today: `c` and `a` through
    `keys.rs:280`, in the graph and the changes pane).
- **The draft.** Check: `a_refused_c_keeps_the_draft`.
- **The amend race.** Check: `head_message_arriving_mid_operation_does_not_open`.
- **The words, both forms.** Check: `the_editor_words_are_unchanged`, a table test over every `(InProgress, Begun)`:
  `editor_words()` equals today's string. Write it **before** the refactor, against today's `refusal`; it must pass
  unchanged after. The existing `esc, ⇥ banner` tests in `src/update/commit_editor/tests.rs` stay green unchanged.
  - Check: `the_pane_status_is_the_editor_words_split`, the same table: `status()` is `Sticky` with the reason and keys
    above.
- **Width.** Check: `the_pane_words_fit_at_80`. Cherry-pick in Graft, 80×24 frame: the footer holds
  `⇥ banner, then c continue  a abort` whole and no row is cut with `…` inside the keys.
- **Tests that open the editor from the pane mid-operation** (all go through the helper `editor_open`,
  `tests/it/resolver/editor_words.rs:59`):
  - `editor_words.rs:152` `after_esc_the_panes_c_and_a_are_commit_and_amend` (via `pin_the_panes_keys` :124). This is
    39's pin; it is **rewritten to this rule, not deleted**: after `Esc`, `c` and `a` in the pane answer in words.
  - `editor_words.rs:192` `the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge`, `:228`
    `the_graft_merge_words_fit_at_80_by_24`, `shell_begun.rs:616` `the_keys_the_commit_editor_names_reach_the_summary`,
    `:656` `the_shell_merge_words_fit_at_80_by_24`, `:667` `capture_fix_editor_names_the_banner`.
  - `editor_open` changes to reach the editor **by construction**, as ticket 40's retained case: from the launched
    state, clear `in_progress` and `live_entries` on a copy, press `c` in the changes pane (the editor opens, its reads
    answer), then put both fields back. The operation is now live under an open editor. If `AppState` does not allow
    this from the test crate, **stop and report** the field and line.
  - NOTES lists each of the six and how it changed.
- **Captures.** Ticket 39's `docs/shots/m6/fix-editor-names-the-banner-120x40.txt` and `-80x24.txt`, written by
  `capture_fix_editor_names_the_banner` (`shell_begun.rs:664-667`).
  - Check: regenerate. Both must come out byte-identical. A changed byte is a stop: report it. `m6-contact-sheet-1.txt`
    and `m6-contact-sheet-captions.tsv` quote the same words and are assembled from those files; `git status docs/shots`
    shows only this ticket's two new captures.
- **The walk.** `editor-to-finish` is walked only in Rust: `tests/it/m5_walk/conflict_journeys.rs:1003-1035`, run by
  `journey_editor_to_finish` (`every_scenario.rs:127`). Its count is asserted by `journey_counts_match_the_note`
  (`conflict_journeys.rs:1075`), which parses the `journey-counts` fence of `docs/design-notes/m6-conflicts.md`
  (`journeys.rs:21-23`), row :505, `keys` = 5. `pty-walk-m6.py` does not walk it (scout grep: no `editor-to-finish`
  literal); **leave the script alone** and say so in NOTES.
  - Change the scenario: `route()` moves to just before the first `key(ch('c'))`; the `See` after it asserts
    `commit_editor.is_none()` and the InShell `Sticky` above; `key(KeyCode::Escape)` goes. The route is then
    `c ⇥ ⇥ c ⏎` = 5.
  - Check: `journey_editor_to_finish` and `journey_counts_match_the_note` green, the note's `5` unchanged. The
    conductor edits only the comment on :505.
- **Other screens that draw the same state.** The banner (`every file done ✓ · c finishes the merge`) and the `u`
  refusal: unchanged. Check: grep the literals, no diff.
- **Journal and undo.** None: a refusal writes nothing. Check: the failing test asserts no `Command::Operate`.
- **Keymap.** No edit: :309 already says they refuse. **TESTING.md** (the conductor edits it): :2138 and :2332, text
  below.
- **MAP.md:** graft §`update::commit_editor`: `open`, `operation_refusal`, `OperationRefusal`, and the `Begun` line's
  route sentence.

## Acceptance (name the verifying test at tick time)
- [ ] `the_editor_words_are_unchanged` (`src/update/commit_editor/tests.rs`): green **after** only. Not run before the refactor — the same edit added `operation_refusal`, so the file did not compile against the old code. The table's strings were copied from the base's literals (`commit_editor.rs:213`, `:225`, `continue_command`); the pre-existing `a_merge_in_progress_*`/`a_bisect_*`/shell tests on the editor row are green. Left unticked: the "before" half is unobserved.
- [x] `c_and_a_with_a_merge_live_answer_in_words` (`tests/it/resolver/editor_words.rs`): red with the impl stashed (`` `c` opened the editor``), then green.
- [x] `the_pane_status_is_the_editor_words_split` (`commit_editor/tests.rs`)
- [x] `every_way_to_open_the_editor_refuses_mid_operation` (`editor_words.rs`): graph and changes pane × `c`, `a`; the grep in NOTES.
- [x] `a_refused_c_keeps_the_draft` (`editor_words.rs`)
- [x] `head_message_arriving_mid_operation_does_not_open` (`editor_words.rs`)
- [x] `the_pane_words_fit_at_80` (`editor_words.rs`; state constructed, see NOTES)
- [x] The six editor tests rewritten and green (`cargo nextest run --workspace`, 3062/3062); NOTES says how each changed.
- [x] `fix-editor-names-the-banner-{120x40,80x24}` byte-identical: `capture_fix_editor_names_the_banner` green in compare mode; `git status docs/shots` shows no change to them.
- [x] `journey_editor_to_finish` and `journey_counts_match_the_note` green; `editor-to-finish` walked as `c ⇥ ⇥ c ⏎` = 5 (note's 5 unchanged).
- [x] Capture (`capture_fix_c_mid_merge_answers_in_words`, `tests/it/resolver/shell_begun.rs`; rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-c-mid-merge-answers-in-words-187x50.txt` and
  `-80x24`.
  - Scene: a shell-begun merge, resolved, the changes pane focused, `c` pressed.
  - **What right looks like:** no editor is drawn. The header reads `MERGING`. The footer holds the reason
    `merge begun in the shell` and the keys `⇥ banner, then c finishes it`, the keys whole at both sizes. The builder
    pastes the exact footer row of each size into NOTES.
- [x] MAP.md updated (doc, no test): `OperationRefusal`, `operation_refusal`, `open`, `head_message_arrived`, the `Begun` route sentence.

## For the conductor (exact text)
- `TESTING.md:2138`: replace `The commit editor never finishes an operation: \`c\`/\`a\` in the panes stay commit/amend
  (keymap §2.4), and its words name the way to the banner — \`esc, ⇥ banner, then c\` (ticket 39).` with
  `\`c\`/\`a\` in the panes never open the editor while an operation is live: the footer names the way to the
  banner — \`merge in progress\` · \`⇥ banner, then c …\` (usability fix M3a).`
- `TESTING.md:2332`, the `editor-to-finish` row: the start becomes `… resolved in the shell, the changes pane focused`,
  the keys `c ⇥ ⇥ c ⏎`, the count 5, and the check `the first \`c\` answers in words, no editor; \`⇥ ⇥\` reaches the banner`.
- `m6-conflicts.md` :483 and :505: the text in `docs/design-notes/m6-usability-fixes.md`, "For the conductor".

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small to medium. One context.

## NOTES

Built 2026-09-24 (builder, Opus). Scout used: `.scratch/m6-conflicts/scouts/48.md`. No compaction.

**Cause, not symptom:** `open` never read `in_progress` (`commit_editor.rs`, base :289-318); it now checks
`live_operation` first. Cost to the user: none; `c`/`a` mid-operation answer in the footer instead of an editor that
could only refuse.

**Grep** `grep -rn "commit_editor::open" crates/graft/src`:
```
crates/graft/src/update/keys.rs:280:        commands.extend(super::commit_editor::open(next, amend, &mut || {
crates/graft/src/update/operate.rs:2278:        let _template_read = crate::update::commit_editor::open(&mut state, false, &mut editor_ids);
crates/graft/src/update/keys/bound.rs:47:    ("c", "commit"),       // `is_commit_key` → `commit_editor::open`
```
`keys.rs:280` is the one production caller (rows: graph and changes pane × `c`, `a`); `operate.rs:2278` is a test with
`in_progress: None`, green; `bound.rs:47` is a comment.

**Footer rows** (exact):
- 187x50: ` merge begun in the shell · ⇥ banner, then c finishes it`
- 80x24: ` merge begun in the shell · ⇥ banner, then c finishes it`

**The six editor tests.** `editor_open` now reaches the editor by construction (`with_the_operation_hidden`: clear
`in_progress` + `live_entries`, `c`, pump the template read, put both back) — ticket 40's retained case; both fields
are `pub`, no stop needed.
- `after_esc_the_panes_c_and_a_are_commit_and_amend` → renamed `after_esc_the_panes_c_and_a_answer_in_words`: after
  `Esc`, `c` and `a` answer in words (both legs). `Landing`/`esc_then` removed (no users left).
- `the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge`, `the_graft_merge_words_fit_at_80_by_24`,
  `the_keys_the_commit_editor_names_reach_the_summary`, `the_shell_merge_words_fit_at_80_by_24`,
  `capture_fix_editor_names_the_banner`: body unchanged; they reach the editor through the new `editor_open`.
- Also changed (not in the six): three unit tests in `src/update/commit_editor/tests.rs`
  (`a_merge_in_progress_refuses_and_names_the_keys`, `a_bisect_in_progress_still_names_the_shell_command`, and
  `shell_refusal` behind `a_merge_in_progress_points_at_c` etc.) opened the editor with `c` over a live operation; they
  now open it through `opened_under` (same construction). Assertions unchanged. The ticket said these stay unchanged —
  impossible once `c` refuses; flagged.

**`the_pane_words_fit_at_80`:** no fixture stops a Graft cherry-pick on a conflict; the test takes the Graft-begun merge
state and sets `in_progress.kind = CherryPick` (words depend only on `(kind, begun)`). Frame asserted to hold the keys
whole and no `…` on a row with `⇥ banner`.

**`head_message_arrived`:** when an operation is live on arrival it refuses whatever the read said, including an `Err` —
the refusal replaces `amend: <why>` because the amend could not have opened either way.

**Touches:**
- Callers of `open`: grep above; `every_way_to_open_the_editor_refuses_mid_operation`.
- Draft: `a_refused_c_keeps_the_draft`. Amend race: `head_message_arriving_mid_operation_does_not_open`.
- Words both forms: `the_editor_words_are_unchanged`, `the_pane_status_is_the_editor_words_split`; editor row
  literals in existing tests green.
- Captures: `fix-editor-names-the-banner-*` byte-identical (compare mode green); two new captures; captions tsv +2 rows
  after ticket 40's; contact sheet regenerated (part 1 198 194 B, part 2 105 976 B, both ≤200 000).
- Walk: `conflict_journeys.rs` scenario changed; `journey_editor_to_finish`, `journey_counts_match_the_note` green.
  `pty-walk-m6.py` left alone (no `editor-to-finish` literal, per scout).
- Banner / `u` refusal: `git diff` over `update/undo.rs`, `update/banner.rs`, `view/` is empty.
- Journal/undo: none; `answers_in_words` asserts no `Command::Operate`.
- Keymap, TESTING.md, design note: not edited (conductor's).

**Finding for the conductor (not changed):** the graph/pane hint bar still shows `c commit` / `a amend` during an
operation (`keys/bound.rs:47-48`).

