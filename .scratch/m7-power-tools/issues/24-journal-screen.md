# journal-screen — `J` shows the shell's steps and the lost commits as rows; `⏎` puts one back; he lands on it

Status: ready — respecified 2026-09-27 against `scouts/24.md` (main @ b90a49e4). Scope unchanged.
Build with: **Opus**, both parts. A screen over a decided list and a decided operation; every choice below is fixed
here, and each is judged by a named test, a walk or a capture. The one structural piece (the landing target) is a
small widening of 03's mechanism, pinned by 03's own tests → not Fable.
Scout: done (`.scratch/m7-power-tools/scouts/24.md`). No second scout. Two items it could not confirm are
**verify-first** steps below (rule 7): the builder checks them and writes file:line in NOTES before building on them.
Crates and files (ADR-0051; outside this list → stop, `blocked: needs <file>`):
- graft: `update/journal.rs` (+ tests), `update/recover.rs` (new), `jobs/journal_list.rs` (new), `jobs/runner.rs`
  (dispatch arms only), `jobs/mod.rs` (the `Command`/`Product` variants), `update/mod.rs` (the new `Product` arms only),
  `update/operate.rs` (`Landing`, `landing_of`, the `Done` arm), `update/banner/stop.rs` (`finished_lands_on`,
  `HeadLanding.on`), `state.rs` (`lands`), `update/name_prompt.rs` (`open_name_prompt_with`), `view/journal.rs`,
  `view/entries.rs` (tag read only), `tests/it/` (walks, hint bar), the m7 walk harness.
- graft-ui: `journal_screen.rs` (+ tests), `graph_screen/mod.rs` (`RowTag` variants and their draw).
- graft-ops, **two spots only**: `operation.rs` `describe_in`'s `DropPutBack` arm (:847 below) and `journal_list`'s
  `words` (the `commit (initial)` row, below), each with its tests.
- Only if verify-first V3 finds the J1 box undrawn: `update/preview.rs` and the graft-ui preview box file. Name it.
- Docs: `docs/shots/m7/`, the m3/m4/m5 J captures it re-shoots, the contact-sheet captions, `TESTING.md` rows, MAP.md
  (graft, graft-ui, graft-ops).

Blocked by:
- **22** journal-list, **23** recover-op, **03 part -a** (landing; merged cad5ed86), **09** (`RowTags`) — merged.
- **10** branch-picker (running): may generalise `return_to_journal` in `update/preview.rs` and holds the modal predicate.
- **30j**: holds `jobs/runner.rs` (the exhaustive `Command` match) and `update/mod.rs`. 24 adds `Command` variants, so
  it builds on 30j's exhaustive match. Not concurrent with **21c** either (also `jobs/runner.rs`).
- **03b**: holds graft-ops `operation.rs` and graft's pull `Done` words in `update/operate.rs`.
- **09b**, if 09b takes `operation.rs` or `update/preview.rs` (its Files allow both, conditionally). Conductor checks
  09b's NOTES; if it took neither, 24 does not wait for it.
- **The `lost_work` fixture order** (Ruling below; graft-fixtures + graft-ops tests, outside 24's crates). Part B's
  walks only. Not landed → part B reports `blocked: needs graft-fixtures (lost_work order)` at the walks and stops.
- **Not concurrent with 08** rewrite-editor: both take the m7 walk harness, the `journeys` parser, the m7 captions
  TSV, `docs/shots/m7/` and the key route. Either order.

Source: `docs/design-notes/m7-power-tools.md`: pass 1b "Journal"; "Pass 2b" F1, F4, F5, J1, "Where he lands" (2b),
keymap diff (2b) §2.11, "Journey counts (2b)", "Where I'd differ (2b)" 9–10; field notes :72, :78; ADR-0044 §1–2,
ADR-0046; review `docs/reviews/m7-22-journal-recover.md` M4.

Needs (as merged; MAP lines):
- graft-ops `journal_list::{read_outside, find_lost, build, words, recover_for, recover_scope, lost_edits,
  JournalList, ListRow, OutsideRecord, OutsideStep, LostCommit, LostEdit, HeldBy, LostScan, Recover}` (MAP :330-340).
- `Operation::Recover(RecoverParams)`, `RecoverTarget::{MoveRef, Recreate, Files}`, `FilesEffect` (MAP :63-65).
- graft `update::banner::finished_lands_on_head`, `AppState::{lands_on_head, land_on_head, head_landing}` (MAP :136,
  :266, :274, :378) — **there is no `land_on_result`**; this ticket widens these (below).
- graft `update::row_tags::RowTags::{set, on, clear}` (MAP :233), `RowTag` (graft-ui `graph_screen/mod.rs:389`).
- graft `update::switch::{switch_to, SwitchTarget}` (MAP :592-593), `update::preview::open_preview`,
  `PreviewSubject::Operation`, `update::name_prompt::{open_name_prompt, NameKind}`.

## Background (from KNOWN_GAPS; builders don't read it)
Rows this ticket closes:
- **M3 :217:** no status line while `J` is open, so a refusal inside `J` has nowhere to show.
- **M6/68:** an entry aborted by the user reads `failed: aborted`. It should read `aborted by you · nothing changed`.
- **M6 usability (:848):** a row cut to fit runs words together (`the woundone`). Cut at a word boundary, with `…`.
- **M6 usability (:847; `docs/reviews/m6-usability-2.md:181`):** the set-aside drop has two names. The done line says
  `set-aside copy dropped`; the journal's undo row says `undo: drop stash@{0}`. One name: the row reads
  `drop the set-aside copy (stash@{0})`. The done line already says `set-aside copy` and stays.

## What changed from the first spec (scout 24)
| First spec said | Code says (scout) | Now |
|---|---|---|
| `land_on_result(state, kind, new_head)` (03) | Does not exist. 03-a gave `Operation::moves_head`, `AppState::lands_on_head: Option<JobId>`, `finished_lands_on_head` (`banner/stop.rs:91`), which seeks `refs.head()` only. 03 excludes `Recover` ("journal-screen owns its landing"). | Widen 03's mechanism to a **target**: `Landing::{Head, Commit(Oid), Changes(PathBuf)}`, decided at launch by `landing_of(op)`. `finished_lands_on_head` stays as a wrapper, so 03's and 10's callers are unchanged. |
| `⏎` on `●` = revert-to, unchanged | `⏎` on the `← u` row runs `u` and on the `← U` row runs `U`, no preview (`update/journal.rs:272-275`, `:311-313`); older rows open `RevertTo` | **Stays.** "Unchanged" means today's behaviour. `u`/`U` are one-key and themselves undoable. Adding a preview is a behaviour change nobody asked for, and it breaks the m3 walks. The bar says so per row: `⏎ undo this` / `⏎ redo this` / `⏎ revert to before this`. |
| :847 fixed in graft | The row's words are made in graft-ops (`operation.rs:1395`, `:1419` `describe_in`'s `DropPutBack` arm). The stash screen's `git stash drop {name}` (`update/stash.rs:505`) is the plain `d` box, a different op. | Fix in graft-ops `describe_in`, one arm. graft-ops `operation.rs` is added to the file list, so 24 waits for 03b. The stash screen is **not** touched. |
| `b` prefilled `<b>-restored` | `open_name_prompt` has no prefill; `TextField::new("")` (`update/name_prompt.rs:157`) | New `open_name_prompt_with(state, kind, prefill)`; `open_name_prompt` calls it with `""`. No caller changes. |
| The cursor stays on the same row (identity) | Cursor is `selected: usize`, `expanded: Option<usize>` (`update/journal.rs:24`, `:27`) | `RowKey`, an identity per row. `selected`/`expanded` become `Option<RowKey>`. The index is found at draw time. |
| `b` lands on the new branch's tip | `BranchCreate { checkout: false }` never lands (03: `moves_head` false) | `landing_of` gives it `Commit(at)`. From the graph, `b` names the cursor's own commit, so the landing re-seeks where he already is. One rule, no J-only flag. `moves_head` is unchanged. |
| Scout path `scouts/journal-screen.md` | `scouts/24.md` | `scouts/24.md`. |

## Decided (the note; not open)
- **Stages:** `J` opens at once with the `●` rows (stage 1), as today.
  - A job reads the outside rows (stage 2), then another runs the lost scan and `lost_edits` (stage 3). Both are in
    `jobs/journal_list.rs`; `runner.rs` only dispatches.
  - Each lands by calling `build` again. **The cursor keeps its `RowKey`** across stages, unless the preselect applies.
    A key that no longer exists after a rebuild falls to the newest row.
  - Title suffixes (J1): `· finding lost commits…` / `· looked at the newest 500 steps` / `· couldn't look for lost
    commits: <why>`. Nothing once done.
- **Preselect:** the Ruling below (it replaces F1's first wording).
- **Rows:** pass 1b's mock-up.
  - Columns: glyph `●`/`$`, the source word `Graft`/`outside Graft`, time, words, and `git: "<subject>"` on `$` rows.
  - `✗` rows are indented `└ ✗ lost`, with the sha7, the quoted subject, and `was <b>'s tip · no branch holds it` /
    `· <b> is gone` / `was HEAD`, plus `+N under it`.
  - Lost-edit rows: `└ ✗ lost  your edit to <path> · replaced by this undo · kept in its backup`, with no hash.
  - The legend line: `● done in Graft: u can undo it · $ done outside Graft: u never touches it · ✗ a commit no branch
    holds`.
  - The muted last line: `a branch deleted outside Graft and never checked out leaves no trace here`.
  - **The first commit's record** (22b's question): git's `commit (initial): Start the api` reads
    `committed "Start the api" on main`, the same words as any commit. `commit (initial)` is a commit. Made in
    graft-ops `journal_list::words` (one arm).
- **Breakpoints (J1):**

  | Width | Columns |
  |---|---|
  | ≥180 | glyph · source · time · words · `git: "…"` |
  | 120–179 | no `git:` column; the selected `$` row shows it on a line under it |
  | <120 | no source word (the legend carries it) |

  At 24 rows the title, legend, status row (when shown) and bar stay, and the rows scroll.
- **The command log** of an expanded `●` row: `┆ git …` lines under `commands Graft ran` (was `    $ git …`,
  `graft-ui/journal_screen.rs:283`).
- **Keys (§2.11, keymap diff 2b):**

  | Key | Does |
  |---|---|
  | `j`/`k`, `↑`/`↓` | move |
  | `⏎` on the `← u` / `← U` `●` row | `u` / `U`, as today (no preview) |
  | `⏎` on an older `●` | revert-to, as today (`PreviewSubject::RevertTo`, `→ ⏎`, ADR-0044 §2) |
  | `⏎` on `✗` / `$` / lost edit | the `RecoverFor` job (`recover_scope` facts, then `recover_for`). `Run(params)` → `open_preview(PreviewSubject::Operation { op: Recover(params), opened_by: "⏎" }, return_to_journal: true)`; `y` confirms (ADR-0044 §1). `OfferBranch` → no preview; J's status row shows `<why> · b makes <name> at <sha7>`. `Err(refusal)` → the status row, its words |
  | `⏎` on a `$` switch row | `switch_to(state, SwitchTarget::Branch(<from>), "⏎", next_id)` (the M7 switch rule) |
  | `b` | on any row with a commit: `open_name_prompt_with(NameKind::Branch { at, checkout: false }, prefill)`, prefill `<b>-restored` when the last `recover_for` on this row offered it, else `""`. On a lost-edit row: status row `a lost edit has no commit · ⏎ puts it back` |
  | `/`, `n`/`N` | find; next/previous match. With no find active, next/previous `✗` row |
  | `Space` | as today (the command log), keyed by `RowKey` |
  | `Esc`/`J` | close |

  Every other key stays swallowed. The bar is per row, from the row alone (no facts): `⏎ undo this` / `⏎ redo this` /
  `⏎ revert to before this` / `⏎ put <b> back to it` / `⏎ make <b> again` / `⏎ put your edit back` /
  `⏎ switch back to <b>`.
- **The preview box** is the note's J1 box, exactly: `now` / `after y` / `your files:` / `u undoes it`, and `y put it
  back  esc cancel`. `Esc` → J on the same `RowKey` (`close_preview`, `return_to_journal`). A `Refused` answer → J
  back, the refusal on its status row.
- **The name prompt from J:** J hides under it, as the stash overlay does (`stash.shown = false`,
  `update/name_prompt.rs:434`). `Esc` → J on the same `RowKey`. `⏎` creating → J closed, the landing below.
- **Landings** (via `landing_of` and `finished_lands_on`):
  - **MoveRef:** the graph, cursor on `to`, detail pane on it; `to` tagged `↺ put back`, `expected` (the old tip)
    `○ main was here`. Status `✓ put back · main has "<subject>" again · u undoes`. Not checked out → still lands on
    `to`, not HEAD.
  - **Recreate:** cursor on `at`, tagged `↺ put back`. Status `✓ spike is back at "<subject>" · u undoes`.
  - **Files:** the graph with the changes pane on the first path. Status `✓ your edit to src/a.rs is back · u undoes`
    (`+N more` for several).
  - **`b`:** cursor on the new branch's tip. Status `✓ made main-restored at "<subject>" · u undoes`.
  - `<subject>` comes from the list's `LostCommit.subject`; unknown → the sha7, never blank.
  - Tags are set on `Done`, after `claim_write` cleared the old ones, and clear on the next operation, as `✎ changed`
    does.
  - `J` again: the put-back is a `●` row (`put main back (the shell's reset)`), and its `✗` row is gone.
- **`u`** undoes the put-back (it's a journal entry). In J, `u` stays swallowed; it never acts on a `$` or `✗` row.
- **Status row in `J`:** one line under the rows. It shows `AppState::status`'s problem, else its notice, the way
  `draw_status_or_hints` picks them, **only while there is one** (status rule). Refusals inside J set
  `AppState::status` as everywhere else; nothing J-private.
- **:847:** `describe_in`'s `DropPutBack` arm reads `drop the set-aside copy (stash@{n})` when the drop is a
  set-aside's (`PutBackWhy::SetAside`); a popped stash's drop keeps `drop stash@{n}` (its done line says
  `stash@{0} dropped`). Entries already on disk keep their recorded words (descriptions are stored at `apply`); say
  so in NOTES.

## Verify first (rule 7: file:line into NOTES before building on it)
- **V1** Whether `J` stays shown under the name prompt today, and what the stash overlay's `b` does on `Esc`
  (`update/stash.rs:238-251`, `update/name_prompt.rs:434`, `:446-448`). Build J's behaviour to "Decided" above either
  way. Write the finding down.
- **V2** Any J frame under m6: grep `docs/shots/m6/` and the m6 captions TSV for `UNDO JOURNAL` and the old footer.
  The scout found none. List the result in NOTES before the re-shoot list is final.
- **V3** Whether a `PreviewSubject::Operation` holding `Operation::Recover` already draws the J1 box (`now` /
  `after y` / `your files:` from `FilesEffect`). Check 23's NOTES and graft's preview draw. Undrawn → it's in scope,
  in the conditional files above. Name them.
- **V4** Whether `DropPutBack` carries its `PutBackWhy`. It doesn't → stop, report `blocked: DropPutBack has no why`
  with the line. Don't infer it from the stash message.
- **V5** `ReflogRecord`'s fields (via `OutsideStep.record`): which ones make `RowKey::Outside` stable across a
  re-read. Use its own field names.

## Read first
- This ticket, then `scouts/24.md` (all).
- The note: pass 1b "Journal"; "Pass 2b" F1, F5, J1, keymap diff (2b) §2.11, "Journey counts (2b)", "Where he lands"
  (2b).
- `docs/design/02-keymap.md` §2.11, §2.3b (`y`); `docs/design/01-view-mockups.md` §1.10.
- `docs/research/ux-principles.md` §3.1, §3.6, checklist 3, 16–17.
- 22's, 23's, 03's (Decided, Handoff) and 09's NOTES.
- `crates/graft/MAP.md` :136, :188, :221, :232-233, :264-274, :365-378, :444-460, :495-500, :530-541, :592-596.
  `crates/graft-ops/MAP.md` :59-66, :330-340. `crates/graft-ui/MAP.md`: `journal_screen`, `graph_screen` `RowTag`.

## Signatures (exact; visibility as written)
```rust
// graft::update::journal
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowKey {
    Graft(EntryId),
    Outside { reference: String, new: Oid, time: i64 }, // from OutsideStep.record; V5 names the real fields
    Lost(Oid),                                          // the lost run's tip
    LostEdit(EntryId),                                  // LostEdit.entry
}
pub(crate) fn row_key(list: &JournalList, index: usize) -> Option<RowKey>;
pub(crate) fn index_of(list: &JournalList, key: &RowKey) -> Option<usize>;
/// The Ruling below. Pure: no state, no terminal.
pub(crate) fn preselect(list: &JournalList, is_fetch: impl Fn(EntryId) -> bool) -> Option<usize>;
// JournalState: `selected: usize` → `cursor: Option<RowKey>`; `expanded: Option<usize>` → `Option<RowKey>`;
// adds `touched: bool` (a key since `J`), `outside: Loading<Arc<Vec<OutsideRecord>>>`,
// `lost: Loading<Arc<LostFound>>`, `list: Option<Arc<JournalList>>`, `offered: Option<(RowKey, String)>` (the prefill).
/// `J`: stage 1 now, and the stage-2 read. `next_id` as `journal_key` takes it.
pub(crate) fn journal_enter(state: &mut AppState, next_id: /* journal_key's type */) -> Vec<Command>;

// graft::jobs::journal_list (new)
pub(crate) struct LostFound { pub unreached: Unreached, pub edits: Vec<LostEdit>, pub scan: LostScan }
// graft::jobs (Command / Product)
Command::ReadOutside { id: JobId }                                  → Product::Outside(Result<Arc<Vec<OutsideRecord>>, String>)
Command::FindLost { id: JobId, outside: Arc<Vec<OutsideRecord>> }   → Product::LostFound(Result<Arc<LostFound>, String>)
Command::RecoverFor { id: JobId, list: Arc<JournalList>, row: usize } → Product::RecoverFor(Result<Recover, Refusal>)

// graft::update::operate
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Landing { Head, Commit(Oid), Changes(PathBuf) }
/// Head iff op.moves_head(); Recover MoveRef{to} → Commit(to), Recreate{at} → Commit(at),
/// Files{paths} → Changes(paths[0]); BranchCreate{checkout: false, at} → Commit(at); else None. Exhaustive.
pub(crate) fn landing_of(op: &Operation) -> Option<Landing>;
// AppState::lands_on_head: Option<JobId>  →  AppState::lands: Option<(JobId, Landing)>; set by launch_with.

// graft::update::banner (stop.rs)
// HeadLanding { refs, walk } gains `on: Landing`; the private land_on_head seeks by `on`.
// Changes(path): waits for the working-tree read instead, then the changes pane on `path` (the KeptCursor shape
// relaunch::land_on_the_work uses; name the site).
pub(crate) fn finished_lands_on(state: &mut AppState, commands: &[Command], on: Landing); // params as finished_lands_on_head's
pub(crate) fn finished_lands_on_head(state: &mut AppState, commands: &[Command]);          // = finished_lands_on(.., Landing::Head)

// graft::update::recover (new)
pub(crate) fn done_status(params: &RecoverParams, subject_of: impl Fn(Oid) -> Option<String>) -> Status;
pub(crate) fn put_back_tags(params: &RecoverParams) -> Vec<(Oid, RowTag)>;

// graft::update::name_prompt
pub fn open_name_prompt_with(state: &mut AppState, kind: NameKind, prefill: &str); // same visibility as open_name_prompt

// graft-ui::graph_screen
pub enum RowTag { Changed, PutBack, WasHere { branch: String } }   // `↺ put back`, `○ <branch> was here`
// graft-ui::journal_screen
pub enum RowKind { Graft, Outside, Lost, LostEdit }
// JournalRowView gains: kind: RowKind, git_subject: Option<String>, under: Option<usize>
pub struct JournalStatusRow { pub words: String, pub problem: bool }
// JournalView gains: title_suffix: Option<String>, status: Option<JournalStatusRow>, enter_words: Option<String>
pub fn cut_at_word(text: &str, width: usize) -> String; // reuse an existing graft-ui word-cut if one exists; name it
// graft-ops (unchanged signatures): describe_in's DropPutBack arm; journal_list::words' `commit (initial)` arm.
```

## Acceptance (name the verifying test at tick time; **(red)** = write it, see it fail on today's code, then build)
Part A:
- [ ] **Verify-first:** V1–V5 in NOTES, each with file:line.
- [ ] **Identity:** `the_cursor_stays_on_its_row_when_outside_rows_arrive` (red),
  `the_cursor_stays_on_its_row_when_lost_rows_arrive` (red), `a_vanished_row_falls_to_the_newest`,
  `space_expands_by_row_not_by_index` (red).
- [ ] **Stages:** `j_opens_with_graft_rows_at_once`, `outside_rows_arrive_second`,
  `lost_rows_arrive_third_and_the_title_says_while`, `a_failed_scan_says_why_in_the_title`.
- [ ] **Preselect** (Ruling): `the_newest_lost_row_is_preselected` (red), `a_fetch_above_does_not_stop_the_preselect`,
  `no_preselect_when_a_graft_row_is_newer`, `a_lost_row_under_a_graft_row_is_never_preselected`,
  `no_preselect_after_a_key`, `n_goes_to_the_next_lost_row`.
- [ ] **Enter:** `enter_on_the_next_undo_row_is_u_as_before`, `enter_on_the_next_redo_row_is_redo_as_before`,
  `enter_on_a_graft_row_is_the_revert_as_before`, `enter_on_a_lost_commit_opens_the_put_back_preview` (red),
  `enter_on_a_gone_branch_opens_make_it_again`, `enter_on_an_outside_row_puts_that_ref_back`,
  `enter_on_a_switch_row_switches_back`, `enter_on_a_lost_edit_puts_the_file_back`,
  `esc_on_the_put_back_preview_returns_to_j_on_the_same_row`, `moved_on_shows_the_words_and_b_prefills_the_name`,
  `b_on_any_row_opens_the_name_prompt`, `esc_on_the_name_prompt_returns_to_j_on_the_same_row`,
  `open_name_prompt_with_prefills_and_ctrl_u_clears`.
- [ ] **Landing:** `landing_of_names_every_kind`,
  `put_back_lands_on_the_restored_commit_with_tags_and_words` (red),
  `a_put_back_of_an_unchecked_branch_lands_on_the_restored_commit_not_head`,
  `a_file_put_back_lands_on_the_changes_pane_on_the_file`, `b_from_j_lands_on_the_new_branch`,
  `b_from_the_graph_lands_where_it_was`, `the_tags_clear_on_the_next_operation`, `u_undoes_the_put_back`,
  `u_never_acts_on_outside_rows`. 03's landing tests and 10's landing tests green, named.
- [ ] **Carried:** `a_stash_drop_has_one_name` (red; graft-ops), `the_first_commit_reads_committed` (red; graft-ops),
  `a_refusal_in_j_sets_the_status` (red).

Part B:
- [ ] **Drawing:** `an_aborted_entry_reads_aborted_by_you` (red; M6/68, replaces `view/journal.rs` test :488),
  `a_cut_row_cuts_at_a_word` (red; :848), `a_refusal_in_j_shows_on_its_status_row` (red; :217),
  `no_status_row_when_nothing_is_wrong`, `the_command_log_uses_the_dotted_rule` (red; replaces
  `journal_screen.rs` :425/:435), `the_bar_names_what_enter_does_on_each_row`, `j_again_shows_a_graft_row_and_no_lost_row`,
  `put_back_and_was_here_tags_draw`.
- [ ] **UI snapshots:** 187×50, 140×40, 100×30, and 187×24 (title, legend, status row and bar kept).
- [ ] **Walks** (`m7_walk`), on the reordered "lost work" fixture, each landing asserted from the frame and
  `len(keys) <= noted_keys(id)`: `lost_to_shell_reset` `J ⏎ y` = 3; `deleted_branch_back` `J n ⏎ y` = 4 (after 13, as
  F4); `edit_in_undo_backup` `J n ⏎ y` = 4. Stage 3 is a wait, not a key.
- [ ] **Captures**, each ≤ 200 KB, each with a Right line; they lead the contact sheet:
  - `docs/shots/m7/journal-187x50.txt`: `●`/`$`/`✗` rows, the preselected `✗` row, the legend, the muted last line.
  - `journal-140x40.txt`: no `git:` column; the selected `$` row's subject under it.
  - `journal-100x30.txt`: no source word.
  - `put-back-preview-187x50.txt`: the box with `now` / `after y` / `your files:`.
  - `put-back-landed-187x50.txt`: `↺ put back`, `○ main was here`, the `✓` line.
  - `journal-after-put-back-120x40.txt`: the new `●` row; the D row gone.
  - `journal-expanded-120x40.txt`: `┆ git …` under `commands Graft ran`.
- [ ] **Re-shot (ADR-0048):** `docs/shots/m3-journal-expanded.txt` and `docs/shots/m3-contact-sheet.txt` no longer
  show `$ git`; name the capture test.
- [ ] MAP.md (graft, graft-ui, graft-ops) entries. NOTES: V1–V5, the scout answers used, every J capture by file as
  re-shot or unchanged, with why.

## Touches (check each; the report's **Touches:** line says how)
- **03's landing** (`lands_on_head` → `lands`, `HeadLanding.on`): every reader of the field, the harness `settled` and
  prelude ("no key while a landing is owed"), `resumed`'s `--continue` landing, the m6 `LandedOnHead` wait. Check:
  grep `lands_on_head`/`land_on_head`/`head_landing`, 03's tests, m6 journeys green.
- **10's picker landing** (via `moves_head` → `Landing::Head`): its tests green.
- **Graph `b`** now owes a landing on its own commit: m5 walks using `b` green (they wait on the owed landing).
- **The revert-to and `u`/`U` rows:** unchanged. Check: `tests/it/preview.rs:453`, `undo_by_key.rs`,
  `undo_over_drift.rs`, `m3_walk.rs` `journal_and_reads` (:1486) green.
- **Every reader of `JournalState::selected`/`expanded`:** grep; each moved to `RowKey`.
- **The name prompt:** existing callers (`open_from_graph`, stash `b`, any 10 added) unchanged; their tests green.
- **The preview's `return_to_journal`** (and 10's generalisation, if made): the journal and stash `Esc`-back tests.
- **Row tags:** `RowTag` matches in graft-ui and `view/entries.rs:348`; 09's tag tests (`amend_into/tests.rs:600-680`).
- **`jobs/runner.rs`:** the new arms in 30j's exhaustive match; no catch-all added.
- **graft-ops `describe_in`:** tests asserting `drop stash@{`; m6 walks and captions quoting the old undo row
  (`m6_stash_put_back`, `set_aside.rs`). CONTEXT.md already has "set-aside copy"; check.
- **graft-ops `words`:** 22's `lost_work` tests that quote `commit (initial)`.
- **Hint-bar tests** (`tests/it/hint_bar.rs:524-590`): the per-row bar, updated and named.
- **J captures** (scout Q6): m3-journal-80x24, m3-journal-160x48, m3-journal-expanded,
  m3-08-journal-after-unstage-{80x24,160x48}, m4-07-journal-remote-rows-{80x24,160x48},
  m5-ui-preview-time-machine-{160x48,80x24}, m3-contact-sheet, m5-contact-sheet-captions.tsv, plus V2's m6 result.
  The legend, footer and command log change them; each re-shot or said unchanged with why.
- **The stash screen:** not touched (:847 is the journal row only). Check: its snapshot unchanged.

## Ruling (planner, 2026-09-26) — F1's preselect; review `docs/reviews/m7-22-journal-recover.md` M4
When stage 3 lands and no key has been pressed since `J`, the cursor moves to **the newest `✗` row that sits under a
`$` row, when that `$` row is newer than every `●` row except a fetch** (`OperationKind::Fetch`). Otherwise it stays
on the newest row. A lost edit (always under a `●`) and a commit lost under a `●` row are never preselected; `n`
reaches them. `preselect` holds the rule; the tests call it directly.
- Why: it is what he approved (rev 2's frame `docs/shots/m7-proto2-c-journal.txt:18-20`; the note's mock-up :239-241
  both put `● fetch 10:44` over the preselected `✗`). A fetch moves nothing he holds (J1, :1173). The shell's loss is
  the one only the journal answers; a `●` newer than the `$` means he worked in Graft since (UX checklist 3).
- Counts unchanged: 13 `J ⏎ y` = 3; 14 and 15 `J n ⏎ y` = 4 (note :1250-1252).
- **The fixture changes first** (graft-fixtures `lost_work` order and times, its tests; graft-ops
  `tests/it/lost_work.rs` row order), routed by the conductor. Newest first the list reads: `● fetch` ·
  `$ reset └ ✗ D` · `$ switch spike→main └ ✗ E` · … · `● undo └ ✗ your edit` · `● discard`. The shell steps after the
  discard/undo leave the lost edit showing and never commit `src/a.rs` into E.
- The note's F1 text and `02-keymap.md:456` are the conductor's to edit.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. `b`, `/` and `n` in `J`
are given by the 2b keymap diff; take no other key.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Two parts, one context each, **A then B**; the conductor dispatches them as two groups.
- **24-a** (~70k): verify-first, `RowKey` and the cursor, stages and jobs, preselect, keys, `RecoverFor`,
  `Landing`/`landing_of`/`finished_lands_on`, `update/recover.rs`, `RowTag` variants and their draw,
  `open_name_prompt_with`, the two graft-ops arms, and Part A's tests.
- **24-b** (~70k): `view/journal.rs` and `journal_screen` (`RowKind`, breakpoints, legend, status row, per-row bar,
  command log, `cut_at_word`, aborted words), snapshots, walks, captures, and the old J captures re-shot. Needs 24-a
  merged and the fixture order.
If either compacts, say so; the conductor splits the rest.

## NOTES (builder 24-a, 2026-09-29, base a91ae83c) — **blocked at V4**
Verify-first done; nothing built. V4 hits the ticket's own stop condition.

- **V4 — blocked: DropPutBack has no why.** `Operation::DropPutBack { stash: Oid }`,
  `crates/graft-ops/src/operation.rs:156-159`; `describe_in` sees only `stash` (:1437-1446). The one constructor,
  `crates/graft/src/update/resolver/resolved.rs:170`, has `clash.why` in hand but drops it. Fixing :847 needs a
  `why: PutBackWhy` field on the variant (graft-ops `operation.rs` beyond the one arm, plus `resolved.rs` and
  `update/operate/tests/edits_words.rs:14`, both outside the Files list). Spec change → conductor.
- **V1.** Stash `b`: the prompt opens over the overlay without touching `stash.shown`
  (`update/stash.rs:238-251`). `Esc` only clears the prompt (`update/name_prompt.rs:263`), so the overlay is back.
  `stash.shown = false` only when ⏎ creates (`name_prompt.rs:428`), set back if the preview fails (:444).
  J today: `b` is not bound in J; `name_prompt.rs` never reads `journal` (grep), so J is untouched by the prompt.
  Whether the prompt box draws over the J overlay: not checked (moot until J binds `b`).
- **V2.** No J frame in m6: `grep -rl "UNDO JOURNAL\|space command log\|J/esc close" docs/shots/m6` → nothing
  (183 files, `m6-contact-sheet-captions.tsv` included).
- **V3.** Undrawn. `crates/graft/src` has no reference to `Operation::Recover` or `FilesEffect` (grep), and no file
  in graft or graft-ui draws `after y` (grep; only resolver `three_panes` tests match). The conditional files come
  into scope: `crates/graft/src/update/preview.rs` and `crates/graft-ui/src/preview_box.rs` (+ `preview_box/`).
- **V5.** `ReflogRecord { refname: String, old: Option<Oid>, new: Option<Oid>, at: i64, subject: String }`
  (`crates/graft-git/src/reflog.rs:22-32`). Stable key: `refname`, `old`, `new`, `at`. **`new` is `Option<Oid>`**,
  so the signature's `Outside { reference, new: Oid, time }` must be `Outside { refname: String, old: Option<Oid>,
  new: Option<Oid>, at: i64 }` (`old` added: two records of one ref in one second can share `new`).

Next step (after the conductor rules on V4): build 24-a as specified; the V4 arm either gets its field (another
ticket, or 24-a's Files widened to `resolved.rs` + `edits_words.rs`) or is dropped from 24-a.
