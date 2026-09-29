# 30o: J review fixes — journey 15 in the app, walks that assert landings, one `u`, `/` find, seven minors (review m7-24-and-30n C1, M1, M3, M4, Minor 1–7, 13)

Status: ready
Build with (three parts, one context each, **a → b → c**, sequential: they share walks, captures, TSV, sheet, TESTING rows):
- **30o-a: Opus.** A graft-ops refusal is replaced under the M3 promise. The premise (the put-back's before-snapshot keeps
  what it replaces) is proved by a real-git `u` test, and the part has a stop condition if that test fails. Nothing is
  structural or hard to undo, so not Fable.
- **30o-b: Opus.** `u` gets one launch outcome that every undo path reads; plus seven local fixes, each pinned by a
  named test.
- **30o-c: Opus.** A new text field in J. The trap (`AppState::text_field_open`) is named and has a red test.
Blocked by: **30m** (running: splits graft `update/journal.rs`, `view/journal.rs`, `update/rewrite_editor*`,
`update/operate.rs`, `update/preview.rs`; graft-ui `journal_screen.rs`). **Everything below is named by function, test or
MAP entry, never by line.** After 30m, find each by grep. The review's lines are 4cf49c2d's.
Scout: 30o-a none (the review names every site; re-grep). Before 30o-b and 30o-c, the conductor decides on a one-page
scout of J's modules as 30m left them.
Needs: 30m merged. No new signature from another ticket.

Source: `docs/reviews/m7-24-and-30n.md` C1 (:74), M1 (:101), M3 (:142), M4 (:157), Minor 1–7, 11, 13; ticket 24
(Decided "Keys", "Landings", the `u` ruling, NOTES 24-b); note `docs/design-notes/m7-power-tools.md` fence :1260-1271
and "Where he lands" :1273-1282; keymap §2.11.

## Rulings (planner, 2026-09-30; not open)

**R1 (M1): journey 15 happens in the app. The refusal gives way, and the journey and its count stay.**
- **Decision:** a `Files` put-back no longer refuses a changed path. It writes the lost edit over what is in the file
  now. The put-back's own journal entry takes a before-snapshot that holds what it replaced. The snapshot's
  `worktree_tree` is tracked content plus untracked files (graft-ops MAP `snapshot::worktree_tree`), and `u` is the
  generic inverse (MAP `Operation::Recover`). So `u` brings the replaced content back. The box says what is replaced
  before `y`.
- **Rejected: "set it aside, put it back after"** (`FilesEffect::Changes.set_aside`). The set-aside copy is the same
  path, so "put back after" writes the undo's copy back over the edit he just recovered, or it clashes. That brings back
  the loss the journey exists to undo.
- **Rejected: a refusal that offers "discard it first".** That is +2 steps (a discard, then its confirm) for a backup
  the put-back already takes. It is ux-principles §3.5: "undo permits fewer confirmations". It also breaks §3.1 "land
  where the next action is".
- **Rejected: keep the refusal and change the count.** The only in-app route today is `c`/`S`. Both keep the content
  he had discarded, and the refusal names them as the next step: an H9 recovery that is false.
- **Why this option:**
  - M3 "you cannot lose work" holds through the entry's snapshot.
  - H5 is met: the preview shows what changes before `y`.
  - H4: `u` itself already replaces an edit and keeps it. The drift box says `Undo replaces your edit to fix.txt; a
    backup keeps it, and U brings it back`. The put-back now does the same.
- **The one case still refused:** a path that is on disk, not in the index and **ignored**. The snapshot's `add -A`
  skips ignored files, so the snapshot cannot hold it. It is refused `InTheWay`, as 28a refuses a `MoveRef`.
- **Box words** (`files_words`, a new clause after `come back` / `go`): `what's in src/a.rs now is replaced`, or with
  several paths `what's in src/a.rs +N more now is replaced`. The paths are those changed against `HEAD`: staged,
  unstaged or untracked. That is exactly the set `UnsavedEditsTo` refused on. The box's last line is already `u undoes
  it`, so the clause names no key.
- **Fence: unchanged.** Exact row, as it stands:
  `edit-in-undo-backup           4     # +1 · today the shell · J n ⏎ y`
  Start: after journeys 13 and 14 in the same repo (as F4), with the file as the forced undo left it. Do **not** clean
  the fixture; the dirty file is the real case (review M1).
- **For the conductor** (the note's "Where he lands" table, a row after `Put-back refused (the ref moved on)`):
  `| Put-back of a lost edit over the file as it is now | The box says what's in the file now is replaced; `y` → the changes pane on the file, `✓ your edit to <path> is back · u undoes`; `u` brings the replaced copy back |`

**R2 (C1): every walk asserts its landing from the frame, and journey 15 is walked with no shell step.** The assert
lists are in 30o-a's Acceptance.

**R3 (M4): one outcome for every undo launch; J reads it in one place.**
- `undo` says what it did: `Launched | Asked | Refused`. It does not infer that from `commands.len()`.
- `u`/`U` in J and `⏎` on the `← u`/`← U` row both call `journal::undo_from_j`:
  - `Launched`: J closes, and 03's undo landing follows.
  - `Asked` (the undo-after-push box): J stays under the box. If the box's choice launches (`Undo locally`, `Re-push old
    tip`), J closes. `Esc` on the box brings J back as it was, on the same `RowKey`.
  - `Refused`: J stays, and the words are on its status row.
- Why: H4 ("same key, same meaning"); the 24 ruling's premise that `u` lands as `⏎`; §3.1. A refusal must show where
  he is looking. This is the m6 usability finding (`⏎` vs `y`) again.
- **`Refused` always carries words.** V1 checks whether `u` while a write runs says anything today. If it is silent, it
  gains `operate::InFlight::words()`, the one wording every refused launch uses (m6-conflicts/78). That applies on every
  screen, not only in J.

**R4 (M3): build `/` in J. It is not cut from M7.** It is one text field over a list J already holds, with a key
promised by §2.11 and bound nowhere else in J.
- **Consistency rule (H4):** J's find behaves key for key as blame's find (keymap §2.8 `/`, `n`/`N`; graft
  `place_key`'s find arm; graft-ui `FindTitle`, `blame_place_list`). The shape below is the expected one. Where blame
  differs on any key's meaning, incremental typing or bar words, **blame's wins**, and NOTES names the site (V3).
- **Expected shape:**
  - `/` opens the find on J's title row, as blame's `FindTitle` tail (`/spike  2 of 5`).
  - Typing moves the cursor to the first hit at or below the row it was on when `/` was pressed, wrapping.
  - `⏎` keeps the find and hands the keys back to J.
  - With a find kept, `n`/`N` step the hits, wrapping. Without a find they keep their 24 meaning: next/previous `✗`.
  - `Esc` while typing drops the find, and the cursor goes back to the row it was on.
  - `Esc` with a find kept drops it; the cursor stays.
  - The next `Esc` closes J.
  - No hit: blame's no-match words, and the cursor does not move.
- **What a row is found by: the words he reads that name something.**
  - `●`: the entry's description.
  - `$`: `graft_ops::journal_list::words(record)`, git's subject and the short ref name.
  - `✗` commit: the sha7, the subject (when read) and the branch it names.
  - Lost edit: its paths.
  - **Not** the verdict words (`aborted by you`, `failed: …`). Those are state, not names.
  - Case-insensitive literal, with the same case folding as `search::Query`.
- **Hits are held as `RowKey`s,** so rows arriving in stages 2 and 3 keep the find. The cursor moves only on his keys
  (24's rule). `/` counts as a key, so no preselect follows it.
- **The bar:** J's bar shows `/` and `n N` exactly where and how `blame_place_list` shows them, in each state.
  - If blame's bar does not list `/`, J's does not either.
  - Every J capture whose bar changes is re-shot (list under 30o-c).

**R5 (Minors):**
1. **The switch-subject parse:** there is one parse, `update::journal`'s. It becomes `pub(crate) fn switched_from`,
   and its return type stays what the update copy returns today. The view formats that answer and deletes its copy.
2. **The bar's `b` hint:** `b new branch here` is drawn only when `row_commit(list, selected)` is `Some`, the same
   function `b` reads. `JournalView` carries `branch_here: bool`.
3. **`b` on a `●` row:** status row `b makes a branch on a $ or ✗ row`. A `●` entry can move several refs or none, so
   "its commit" has no single answer. The keymap row is amended to match (R6).
4. **`⏎` on a step Graft doesn't undo** (fetch, push; whatever `plan_revert_to` skips):
   - The bar shows no `⏎` hint there.
   - `⏎` sets the status row to `Graft doesn't undo a <kind> · nothing to go back to on this row`. `<kind>` is the word
     J already draws for it.
   - Why nothing is lost: "before this" on that row equals "before" the next newer `●` row, which keeps its `⏎`.
   - One predicate: the one `plan_revert_to` skips by. Make it `pub` in graft-ops if it is private, and name it.
   `enter_graft` and the bar both read it.
5. **A lost commit's unread subject:**
   - `LostCommit.subject: Option<String>`; `None` when it could not be read.
   - J draws the sha7 alone, with no quotes. The box title and the `✓` line already fall back to the sha7 through
     `subject_of`.
   - `find_lost`'s `Err` arm gets a comment saying where the failure shows (review Minor 14).
6. **The m3 J captures:** the fixture `journal_state` (graft `view/tests.rs`) opens J through `journal::journal_enter`,
   not `shown = true`. Its returned commands are asserted, not dropped. Then the three m3 J captures are re-shot.
7. **A put-back's `Esc` words:**
   - The put-back gets its own cannot-stop words: `runs to the end · u undoes it after` (a new const beside
     `HISTORY_CANNOT_STOP`).
   - `confirm_preview`'s `Recover` arm closes J **only once the launch is `Ok`**. A launch refusal stays on J's status
     row.
13. **The `u` test:** `u_in_j_undoes_the_next_entry_from_any_row` asserts that the selected row holds `✗ lost` before
    `u`.

**R6 (Minor 11): the keymap §2.11 clause, for the conductor to apply** (`docs/design/02-keymap.md` §2.11). Replace the
`Enter`, `b`, `/` and `Esc` rows with:
```
| `Enter` | Per row: `●` revert to just before it (Ghost Preview, `→ ⏎`, unchanged; none on a step Graft doesn't undo, such as a fetch); `$` put that ref back to before it, and on a switch row switch back to the branch it left; `✗` put back what held the commit, or a lost edit's file, replacing what is in that file now (Ghost Preview, `y`; `u` brings it back) (M7) |
| `b` | New branch at a `$` or `✗` row's commit (name prompt; prefilled `<branch>-restored` when the row names one); on a `●` row it says so (M7) |
| `/` | Find a row by the words that name it, as blame's find (§2.8): `Enter` keeps it, `Esc` drops it (M7) |
| `Esc`, `J` | Close (a find first: `Esc` drops it) |
```
Append to the amendment line under the table:
` Amended 2026-09-30 (m7 30o: Enter on switch and fetch rows, a lost edit put back over the file as it is, b on ● rows, J closes, / as blame's).`

## Verify first (rule 7: file:line into NOTES before building on it)
- **V1 (30o-b):** what `undo_key` does while another write is in flight: does it set a status, or return nothing and
  say nothing? Name the file:line. Silent → R3's words; it has words → nothing to add.
- **V2 (30o-b):** whether the undo-after-push confirm box draws over J (a frame with J up and the box open). It is
  hidden under J → dock it in `view/frame/overlays.rs`'s J draw, as 24-a2 docked `draw_preview`. Name the site.
- **V3 (30o-c):** blame's find, key by key: `/`, typing (incremental or on `⏎`), `⏎`, `Esc` twice, `n`/`N`, no-match
  words, `Ctrl+U`, paste, and the bar words from `blame_place_list` in each state. file:line each. Where it differs from
  R4's expected shape, J copies blame; say which.
- **V4 (30o-a):** how `Operation::Recover`'s `check` finds a `Files` path "changed" (`UnsavedEditsTo`), and whether
  `in_the_way::blocking` can tell an ignored path from an untracked one. file:line each.

## Part 30o-a: journey 15 in the app, and the walks (R1, R2)

### Files (exact; outside this list → stop, `blocked: needs <file>`)
- **graft-ops:**
  - The file holding `Operation::Recover`'s `check` and plan for `RecoverTarget::Files` (MAP `Operation::Recover`;
    review: `src/operation/recover.rs`).
  - The file defining `FilesEffect` and the site that builds it for a `Recover` preview (grep `FilesEffect::Changes`).
  - The file holding `pub enum Refusal` and `explain` (remove `UnsavedEditsTo` if no other user; grep).
  - `in_the_way` (only if V4 finds it cannot tell ignored from untracked).
  - Their tests: `src/operation/tests.rs` and `tests/it/` files that grep `UnsavedEditsTo` or `has unsaved edits`.
  - `MAP.md`: `Operation::Recover`, `FilesEffect`, `Refusal::UnsavedEditsTo`.
- **graft:**
  - `update/recover.rs` (`files_words`) and its tests.
  - The function holding `operate::refusal_status`, only its `UnsavedEditsTo` arm if it has one.
  - `tests/it/m7_walk/journal.rs` (`the_three_lost_work_journeys`, `confirm`).
  - `tests/it/journal_screen.rs` (`a_refusal_in_j_shows_on_its_status_row`).
  - `tests/it/put_back_box.rs`, only a test that pins the `Files` refusal.
  - `MAP.md` (`update::recover`).
- **graft-fixtures: none.** `lost_work` stays as built.
- **Docs:**
  - New `docs/shots/m7/put-back-edit-preview-187x50.txt` and `docs/shots/m7/put-back-edit-landed-187x50.txt`.
  - `docs/shots/m7/m7-contact-sheet-captions.tsv`: two rows after `journal-after-put-back-120x40.txt`, in journey
    order.
  - `docs/shots/m7/m7-contact-sheet.txt`, rebuilt with `scripts/contact-sheet.sh --captions-order m7 docs/shots/m7`.
  - `TESTING.md` "M7: the journal, J": two rows after "`J` again after the put-back". Any row quoting `has unsaved
    edits` or `c commits or S stashes them first` (grep).

### Signatures
```rust
// graft-ops — FilesEffect: existing fields unchanged; Changes gains one.
FilesEffect::Changes { /* come_back, go, set_aside as today */ replaced: Vec<PathBuf> }
//   Recover Files: `replaced` = the target paths changed against HEAD (staged, unstaged or untracked).
//   Every other builder of Changes: `replaced: Vec::new()`.
// graft-ops — Operation::Recover check, RecoverTarget::Files:
//   no `UnsavedEditsTo`; `InTheWay` for a path on disk, not in the index, ignored (the snapshot can't hold it).
// graft-ops — Refusal::UnsavedEditsTo { paths }: removed (MAP: "Recover Files only since m7/28a"). Another user → keep it, say so.
// graft::update::recover — unchanged signature; one clause added (R1 words):
pub(crate) fn files_words(effect: &FilesEffect) -> String;
```

### Acceptance (name the verifying test at tick time; **(red)** = write it, see it fail on today's code, then build)
- [ ] **V4** in NOTES, file:line.
- [ ] **The put-back runs over the undo's copy** (graft-ops, real git, `lost_work` fixture as built):
  - `a_file_put_back_over_a_changed_file_runs` (red: today `UnsavedEditsTo`): `src/a.rs` = `LOST_EDIT` after.
  - `the_file_put_back_names_what_it_replaces` (red): `FilesEffect::Changes.replaced == [src/a.rs]`.
  - `a_file_put_back_over_an_ignored_file_is_refused` (red unless V4 finds it already refused): `InTheWay`, file
    untouched.
- [ ] **The premise (M3):**
  - `u_after_a_file_put_back_brings_back_what_it_replaced` (real git): after the put-back, `u` → `src/a.rs` is
    **byte-exact** `DISCARDED_EDIT`.
  - `u_brings_back_an_untracked_file_a_put_back_replaced`: the same for an untracked path.
  - **If either is red after the build, stop:** `blocked: the before-snapshot does not hold <what>`. Do not add a
    set-aside or a copy to make it pass; that is the planner's call.
- [ ] **Words** (graft `update/recover.rs` tests):
  - `files_words_names_what_is_replaced` (red): one path → `what's in src/a.rs now is replaced`; three →
    `what's in src/a.rs +2 more now is replaced`. Clause order: come back · go · replaced · set aside.
  - 24-a2's `files_words_joins_the_clauses_that_apply` still green.
- [ ] **The refusal test moved:** `a_refusal_in_j_shows_on_its_status_row` now refuses with `b` on the lost-edit row
  (`a lost edit has no commit · ⏎ puts it back`), still asserted from the frame. Every other test that pinned
  `UnsavedEditsTo` is listed in NOTES as removed or re-pointed.
- [ ] **Walk 13 (unchanged)** in `the_three_lost_work_journeys`: its existing asserts green.
- [ ] **Walk 14, asserted from the frame after `J n ⏎ y`** (`Route::within_count`, ≤ 4):
  - J closed.
  - `▸` on the row of E (`Try redis cache`), and `↺ put back` on that row.
  - The bottom row holds `spike is back at "Try redis cache"`, `✓` and `u undoes`.
  - `spike` = E by `rev-parse`, `main` untouched.
  - Not red-first: the landing exists, so these asserts are new. Check them by mutation: drop `put_back_tags`'
    `PutBack` and the walk fails. Say so.
- [ ] **Walk 15, with no shell step** (red: today `⏎` is refused). `J n ⏎ y`, `Route::within_count` ≤ 4:
  - After `n`: the selected row holds `your edit to src/a.rs`.
  - After `⏎`: the box over J, titled `Preview: put back your edit to src/a.rs`, with the line
    `your files: src/a.rs comes back · what's in src/a.rs now is replaced` and `u undoes it`.
  - After `y`: J closed; the changes pane shown with its cursor row on `src/a.rs`; the bottom row holds
    `your edit to src/a.rs is back`, `✓` and `u undoes`; `src/a.rs` = `LOST_EDIT` on disk.
  - Uncounted, after the journey: `u` → `src/a.rs` = `DISCARDED_EDIT`.
  - The uncounted `git checkout -- src/a.rs` and the refusal assert are **deleted**.
- [ ] **Captures** (each ≤ 200 KB, written by `the_three_lost_work_journeys`, asserted with `check_shot`):
  - `put-back-edit-preview-187x50.txt`. **Right:** J's rows above, the lost-edit row selected; the box titled
    `Preview: put back your edit to src/a.rs`; `your files: src/a.rs comes back · what's in src/a.rs now is replaced`;
    `u undoes it`; footer starts `y put it back`; J's bar blank.
  - `put-back-edit-landed-187x50.txt`. **Right:** the graph with the changes pane open, cursor on `src/a.rs`; the
    bottom row `your edit to src/a.rs is back ✓ · u undoes` (as `done_status` orders it).
  - TSV rows and TESTING rows say exactly those Right lines, plus the test name and "Colour unverified (rule 5)". Sheet
    rebuilt.
- [ ] `put-back-preview-187x50.txt` (a `MoveRef` box, `replaced` empty) is byte-identical:
  `the_put_back_box_draws_over_j` green.
- [ ] MAP.md (graft-ops, graft). Gates workspace-wide.

### Touches (check each; say how on the **Touches:** line)
- **`Refusal`:** every exhaustive match (graft-ops `explain`, graft `refusal_status`; grep `Refusal::` in both).
  Also grep `has unsaved edits` and `stashes them first` across `crates/`, `docs/shots/` and `TESTING.md`.
- **`FilesEffect::Changes`:** every constructor and pattern (graft-ops preview, graft `files_words`,
  `preview_lines`). The `MoveRef` set-aside line is unchanged.
- **`lost_edits`:** unchanged. After the put-back, the lost-edit row is gone from J, because the file equals the
  backup. The walk's next `J` shows it (assert it if the walk reopens J; else say so).
- **Journal/undo:** there is no new operation. `u` on the put-back is the generic inverse. The drift guard does not
  fire on an untouched file (the premise test).
- **Design note and keymap:** R1's table row and R6's clause are the conductor's to apply.

## Part 30o-b: one `u`, and the minors (R3, R5)

### Files (exact; after 30m, find each by function)
- **graft:**
  - The module holding `undo::undo_key`, `undo::launch` and `after_push_chosen` (`update/undo.rs`) + tests.
  - `update/keys.rs` (`undo_key_anywhere` only).
  - J's update modules as 30m split them: `enter_graft`, `row_commit`, the `b` arm, the switch-subject parse, and
    `journal_enter` read-only. Plus their tests.
  - J's view modules (the `✗` words, the bar words, the `$` switch words).
  - `view/frame/overlays.rs` (only if V2 finds the push box hidden).
  - The module holding `confirm_preview`'s `Recover` arm and `HISTORY_CANNOT_STOP` (`update/preview*`).
  - `update/recover.rs` (only `subject_in` / `subject_of`, if Minor 5 reaches them).
  - `jobs/journal_list.rs` (`find_lost`'s `Err` arm and `LostFound::subject_of`).
  - `view/tests.rs` (`journal_state`, `capture_m3_journal`).
  - `tests/it/journal_put_back.rs`, `tests/it/journal_screen.rs`, `tests/it/undo_by_key.rs` (only for V1's words).
  - `MAP.md`.
- **graft-ui:** J's model module (`JournalView.branch_here`) and layout/draw (the bar's `b`) + tests; `MAP.md`.
- **graft-ops:**
  - `journal_list/rows.rs` (`LostCommit.subject`) and the struct's file.
  - The file holding `plan_revert_to` (the predicate made `pub`, if private).
  - Their tests; `tests/it/lost_work.rs` (if it asserts the placeholder); `MAP.md`.
- **Docs:**
  - `docs/shots/m3-journal-{80x24,160x48,expanded}.txt` and `docs/shots/m3-contact-sheet.txt`, rebuilt with
    `scripts/contact-sheet.sh m3 docs/shots`.
  - Any m7 J capture `check_shot` now fails on, re-shot with its TSV and TESTING rows. Expected: none, because no
    capture's selected row is a fetch, a `●` offering `b`, or a no-commit `$` row. Say which.

### Signatures
```rust
// graft::update::undo
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UndoStart {
    /// `Command::Undo`/`Redo` pushed, `operating` set.
    Launched,
    /// The undo-after-push box is up; nothing runs until its choice.
    Asked,
    /// Nothing runs; the words are on `AppState::status` (always words, R3).
    Refused,
}
/// Every `u`/`U` launch goes through here. No caller reads `commands.len()`.
pub(crate) fn start_undo(state: &mut AppState, commands: &mut Vec<Command>, redo: bool,
                         next_id: /* undo_key's type */) -> UndoStart;
// undo_key(state, redo, next_id) -> Vec<Command>: kept as a wrapper over start_undo for callers that don't care.
// after_push_chosen: a choice that launches closes J when J is shown (journal::close); Cancel leaves J.

// graft::update::journal (J's keys module after 30m)
/// `u`/`U` in J and `⏎` on the `← u`/`← U` row: Launched → close J; Asked, Refused → J stays.
pub(crate) fn undo_from_j(state: &mut AppState, commands: &mut Vec<Command>, redo: bool,
                          next_id: /* as above */);
/// The one parse of `checkout: moving from X to Y`; return type as the update copy's today.
pub(crate) fn switched_from(subject: &str) -> Option</* as today */>;

// graft-ui::journal_screen
// JournalView gains `branch_here: bool` (the bar's `b new branch here`; from graft's row_commit).

// graft-ops::journal_list
// LostCommit.subject: String → Option<String>.
// graft-ops::undo — the predicate plan_revert_to skips by, `pub`, named in NOTES.
```

### Acceptance
- [ ] **V1, V2** in NOTES, file:line.
- [ ] **M4, one path:**
  - `start_undo_says_what_it_did` (red: the enum doesn't exist): in progress → `Refused` with words; after a push →
    `Asked`, no command; otherwise → `Launched`, one `Undo`.
  - `enter_on_the_undo_row_during_an_operation_keeps_j` (red: today J closes first, and the words land on the graph).
    Checked from the frame: J up, `finish or abort the <kind> first · ⇥ banner` on J's status row.
  - `enter_on_the_undo_row_after_a_push_opens_the_box_over_j` (red).
  - `u_in_j_after_a_push_then_undo_locally_closes_j` (red: today J stays).
  - `esc_on_the_push_box_from_j_returns_to_j_on_the_same_row`.
  - If V1 found it silent: `u_while_a_write_runs_says_wait` (red), on the graph and in J.
  - Green, named: `enter_on_the_next_undo_row_is_u_and_closes_the_browser`,
    `enter_on_the_next_redo_row_is_still_capital_u`, `u_in_j_undoes_the_next_entry_from_any_row`,
    `u_in_j_during_an_operation_shows_the_refusal_on_js_status_row`, `under_a_modal_the_landing_is_dropped`,
    `u_never_acts_on_outside_rows`.
  - Grep: `commands.len()` no longer appears in `undo_key_anywhere`.
- [ ] **Minor 1:** grep shows one `moving from` parse in `crates/graft/src`. `the_bar_names_what_enter_does_on_each_row`
  green. Not red-first: it removes a duplicate.
- [ ] **Minor 2:** `no_b_hint_on_a_step_that_names_no_commit` (red), from the frame.
- [ ] **Minor 3:** `b_on_a_graft_row_says_where_b_works` (red: today `this step names no commit`). `b_on_a_step_with_no_commit_says_so` still green.
- [ ] **Minor 4:**
  - `no_enter_hint_on_a_fetch_row` (red).
  - `enter_on_a_fetch_row_says_graft_doesnt_undo_it` (red: today a `RevertTo` preview opens). No preview; the words
    exact.
  - `enter_on_a_graft_row_is_the_revert_as_before` green.
- [ ] **Minor 5:**
  - graft-ops `an_unreadable_subject_is_none` (red).
  - graft `a_lost_commit_with_no_subject_draws_its_sha_alone` (red: today `"(its subject could not be read)"`
    quoted). The box title and the `✓` line say the sha7 alone
    (`the_box_names_the_sha_when_the_subject_is_unknown` green).
- [ ] **Minor 6:**
  - `capture_m3_journal` builds J through `journal_enter` and asserts the frame has no `J to inspect` (red on
    today's fixture).
  - The three m3 J captures and the m3 sheet are re-shot. Their header's "Judge by eye" line no longer mentions the
    notice. Grep: 0 `J to inspect` in `docs/shots/m3-journal-*`.
- [ ] **Minor 7:**
  - `a_put_back_runs_with_its_own_esc_words` (red): while it runs, `runs to the end · u undoes it after`.
  - `a_put_back_refused_at_launch_stays_on_j` (red): a write marked in flight before `y` → J shown, the refusal
    (`InFlight::words()`) on J's status row.
- [ ] **Minor 13:** `u_in_j_undoes_the_next_entry_from_any_row` asserts that the selected row holds `✗ lost` before
  `u`.
- [ ] MAP.md (graft, graft-ui, graft-ops). Gates workspace-wide.

### Touches
- **`undo_key` callers** (grep `undo_key(`): the graph's `u`/`U`, the banner, J's `⏎`, `undo_key_anywhere`,
  `after_push_chosen`. Their tests stay green; `undo_by_key.rs`, `undo_over_drift.rs`, and m3/m5/m6 walks with `u`.
- **The drift box:** `u` in J → `Launched` → J closed → a `Drift` answer's box on the graph (as `⏎`). Check with
  `undo_over_drift.rs`.
- **`journal::close`:** it also cancels J's stage jobs. It is safe when J is hidden (say how it was checked).
- **`LostCommit.subject`:** every reader (grep `.subject` on `LostCommit`, `subject_of`, `subjects`). graft-ops
  `lost_work.rs`.
- **The `plan_revert_to` predicate:** `plan_revert_to` itself is unchanged in behaviour (its tests green).
- **The m3 captures:** only the notice line changes (diff in NOTES). m4/m5 J captures unaffected (say how).
- **Out of scope, conductor files a gap row:** the legend `● done in Graft: u can undo it` is also true of a fetch
  row that `u` can't undo; and the legend's `✗` covers lost edits too (24-b's list).

## Part 30o-c: `/` find in J (R4)

### Files (exact; after 30m, find each by function)
- **graft:**
  - New `update/journal/find.rs` (J's find: state, `hits`, `find_key`, paste) + tests.
  - J's key module (the `/` and `n`/`N` arms and the `Esc` ladder only).
  - `JournalState` (the `find` field).
  - `AppState::text_field_open` (J's find while typing).
  - The paste router (where `name_prompt_paste` is called).
  - J's view modules (the find tail, the bar).
  - `tests/it/journal_screen.rs` and the m7 walk (the capture).
  - `MAP.md`.
- **graft-ui:** J's model and draw (`JournalView.find`, the title tail, the bar per R4) + tests; `MAP.md`. Reuse
  blame's title-tail formatter if it is a function; name it.
- **Docs:**
  - New `docs/shots/m7/journal-find-187x50.txt`; its TSV row after `journal-expanded-120x40.txt`, the sheet rebuilt,
    and a TESTING row in "M7: the journal, J".
  - **If the bar gains `/`** (R4, blame's rule): re-shoot every J capture: m7 `journal-{187x50,140x40,100x30,187x24}`,
    `journal-after-put-back-120x40`, `journal-expanded-120x40`; `m3-journal-{80x24,160x48,expanded}`,
    `m3-08-journal-after-unstage-{80x24,160x48}`, `m4-07-journal-remote-rows-{80x24,160x48}`. Also the m3/m4/m7 sheets,
    and every TSV/TESTING row that quotes J's bar (grep `J/esc close`).

### Signatures
```rust
// graft::update::journal::find (new)
pub(crate) struct JournalFind {
    pub(crate) field: graft_ui::text_input::TextField,
    pub(crate) typing: bool,           // the field has the keys
    pub(crate) before: Option<RowKey>, // the cursor when `/` was pressed; Esc while typing returns to it
}
/// Rows whose naming words hold `query` (R4's list), newest first. Blank → empty. Case folding as search::Query.
pub(crate) fn hits(journal: &JournalState, query: &str) -> Vec<RowKey>;
/// `/`, typing, Ctrl+U, ⏎, Esc, and n/N while a find is kept. true = handled.
pub(crate) fn find_key(state: &mut AppState, key: KeyPress) -> bool;
pub(crate) fn find_paste(state: &mut AppState, text: &str) -> bool;
// JournalState gains `find: Option<JournalFind>`. AppState::text_field_open: true while journal.find is typing.

// graft-ui::journal_screen
pub struct JournalFindView { pub query: SanitizedText, pub at: Option<usize>, pub of: usize, pub typing: bool }
// JournalView gains `find: Option<JournalFindView>`.
```

### Acceptance
- [ ] **V3** in NOTES, file:line. Every difference from R4's expected shape is named and taken from blame.
- [ ] **Unit** (`update/journal/find` tests):
  - `hits_find_a_lost_subject_a_branch_and_a_path` (red).
  - `hits_skip_verdict_words` (red): `aborted` finds nothing on an aborted row.
  - `a_blank_find_has_no_hits`.
- [ ] **Keys** (`tests/it/journal_screen.rs`, asserted from the frame):
  - `slash_opens_the_find_on_js_title_row` (red: today `/` is swallowed).
  - `typing_moves_to_the_first_row_that_holds_it` (red).
  - `enter_keeps_the_find_and_n_steps_the_hits` (red; `n` wraps, `N` goes back).
  - `esc_while_typing_puts_the_cursor_back` (red).
  - `esc_drops_a_kept_find_then_closes_j` (red).
  - `no_match_says_so_and_moves_nothing` (red).
  - `a_paste_goes_into_the_find` (red).
- [ ] **The trap:** `u_typed_into_the_find_is_text` (red: today `u` undoes). The journal is unchanged by `rev-parse`
  and by the entry count.
- [ ] **Stages:** `a_find_survives_rows_arriving`: stage 3 lands mid-find, the cursor keeps its `RowKey`, `of`
  counts the new `✗` hit, and nothing is preselected.
- [ ] Green, named: `n_goes_to_the_next_lost_row`, `no_preselect_after_a_key`, `escape_closes_it_…`, J's put-back
  box tests (`Esc` there still goes to the box first).
- [ ] **Capture** `docs/shots/m7/journal-find-187x50.txt` (≤ 200 KB) from a named test on the `lost_work` fixture:
  `/spike ⏎`. **Right:** the title ends in blame's find tail for `spike` (`1 of 2` or as blame words it); `▸` on the
  first row naming spike; the bar as R4. TSV and TESTING rows say exactly that, plus "Colour unverified (rule 5)".
  Sheet rebuilt.
- [ ] Re-shot captures (only if the bar gained `/`): each listed in NOTES as re-shot, with the one-line diff.
- [ ] MAP.md (graft, graft-ui). Gates workspace-wide.

### Touches
- **`text_field_open`:** every global binding asks it (`u`, `U`, `Ctrl+Z`, `Esc` on a held failure; MAP
  `AppState::text_field_open`). The trap test covers `u`; say how `U` and `Ctrl+Z` were checked.
- **J's `Esc` ladder:** box → find typing → find kept → close. 24's Esc tests stay green.
- **`n`/`N`:** without a find, 24's meaning unchanged.
- **Preselect:** `/` is a key (`touched`).
- **The hint catalogue:** if J's bar reuses `blame_place_list` words, the coverage slots are unchanged, because J's
  bar is its own widget (say how it was checked).
- **Keymap:** `/` is §2.11's own row (R6). No other key taken. `?` help for J, if it lists keys (grep).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
- **30o-a** ~45k.
- **30o-b** ~65k; if it compacts, M4 first, then the minors.
- **30o-c** ~50k; ~60k if every J capture is re-shot.

If any part compacts, say so; the conductor splits the rest.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Keys: only those in
§2.11 and R6. Don't edit the keymap doc or the design note; R1's row and R6 are the conductor's.

## NOTES (builder)
