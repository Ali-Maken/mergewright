# Scout 30o-c: J (journal browser) at HEAD f522b6f3

## 1. State: `crates/graft/src/update/journal.rs`
- `RowKey` :42: `Graft(EntryId)` | `Outside{refname, old, new, at}` | `Lost(Oid tip)` | `LostEdit(EntryId)`. Maps from rows with `row_key(list, i)` :65. The reverse is `index_of` :80, a linear scan. `row_commit` :87 is the commit `b` uses.
- `JournalState` :122 has these fields: `shown`, `cursor: Option<RowKey>` (None = newest), `expanded: Option<RowKey>`, `touched`, `notice_seen`, `entries: Loading<Arc<JournalRead>>` (stage 1), `reading`, `unreadable`, `outside: Loading<Arc<Vec<OutsideRecord>>>` (stage 2), `lost: Loading<Arc<LostFound>>` (stage 3), `list: Option<Arc<JournalList>>`, `offered: Option<(RowKey,String)>`, `recovering: Option<(JobId,RowKey)>`.
- `selected()` :187 gives the index: `index_of(cursor)`, or 0. `finding_lost()` :220.
- `rebuild` :269 builds the rows with `graft_ops::journal_list::build`. The result is stored in `list`. It runs `preselect` :105 only when stage 3 is Ready and `!touched`. If the cursor's key is gone, the cursor falls to row 0. A stale `expanded` is dropped.
- `touched` is set on every key `shown_key` sees, except Tab (keys.rs:81). It is reset in `journal_enter` keys.rs:48.

## 2. Keys: `update/journal/keys.rs`
- `journal_key` :13. It returns false on Ctrl or Alt. If `shown`, it calls `shown_key`. Otherwise `J` opens J (`journal_enter` :39).
- `shown_key` :67, arms in order:
  - Tab/BackTab → false
  - `touched = true`
  - `J`|Esc → `close` :204
  - Up/k, Down/j → `move_to` :115. Moves use an index, and the cursor is stored as a RowKey.
  - `n` → `next_lost(true)`, `N` → `next_lost(false)` :95-96 (fn :125)
  - Space → expand
  - `b` → `branch_here` :147
  - Enter → `enter` :236
  - `_ => {}`, then returns true (swallows).
- Put-back box: Enter sends `RecoverFor`, and the answer opens `preview`. `boxes_key` (update/keys.rs:61, confirm then preview) is first in `key_event` :178, so the box gets Esc before J does.
- Global chain in `key_event` (update/keys.rs:161):
  1. drag, boxes, rewrite, finish, resolver, picker, banner, place
  2. context menu / picker, then editor
  3. Ctrl+Z suspend
  4. `u`/`U`/Ctrl+Z undo, if `!text_field_open()` (:230)
  5. Esc cancel_operating, then Esc status-dismiss (:250, `!text_field_open`)
  6. `pane_and_overlay_key` :281, where J is first when focus is Graph.
- `u` with J shown calls `journal::undo_from_j` (keys.rs:145).
- **Search runs before all of this.** At update/mod.rs:1075, `search_answers = search_prompt.is_some() || !modal_owns_keys()`. J is not in `modal_owns_keys` (state.rs:828-834 doc), so with J open:
  - `/` opens the **graph's** search prompt (search.rs:20). J never sees `/`, so it is not swallowed by J.
  - `n`/`N` go to the graph search's `step_search` when `next.search.is_some()` (search.rs:22). J's next-lost is reached only when no graph search is active.

## 3. Predicates: `update/state.rs`
- `text_field_open` :804: search_prompt, place::prompt_open, commit_editor, stash.prompt, credential, name_prompt, picker::shown, rewrite_editor::field_open, finish (focus≠Files). No journal field.
- `modal_owns_keys` :835: the above plus confirm, preview, resolver, finish, rewrite.editor, place::owns_keys, focused banner.
- Callers of `text_field_open`:
  - update/keys.rs:230, :250
  - drag.rs:151
  - clash.rs:185
  - remote/in_flight.rs:21
  - state.rs:836
  - tests: update/tests.rs:3703 (the sweep), name_prompt/tests.rs:98, picker/tests.rs:199,361, credential/tests.rs:146, rewrite_editor/tests.rs:654,1021

## 4. Paste
- `Event::Paste` update/mod.rs:1121-1132 tries these in order: `credential_paste`, `name_prompt::name_prompt_paste` (:1128), `finish::paste`, then `commit_editor::paste`.
- `name_prompt_paste(state:&mut AppState, text:&str)->bool` name_prompt.rs:293 returns false if closed. Otherwise it clears `refusal` and calls `field.insert_str`.

## 5. Row words (model: graft-ops/src/journal_list.rs)
- `ListRow` :117: `Graft(EntryId)` | `Outside(OutsideStep)` | `Lost{event, commit: LostCommit}` | `LostEdit{event, edit: LostEdit}`. `JournalList{rows, scan: LostScan}` :156.
- ● rows: `JournalEntry.description` (graft-core journal.rs:266), drawn at view/journal.rs:184.
- $ rows: `OutsideStep.words`, built by `journal_list::words::words(record, back)` (words.rs:27), plus `record.subject` as `git_subject` (view :115-117).
- ✗ rows: `LostCommit{run: LostRun{tip, under}, subject: Option<String>, held_by: HeldBy::Branch{name,gone}|Head}` :93. Text comes from view `lost_words` :199 (sha7, "subject", branch, "+N under it").
- Lost edit: `LostEdit{entry, backup_tree, paths: Vec<PathBuf>}` :106. Text comes from view `lost_edit_words` :222.

## 6. View
- `graft/src/view/journal.rs:32` `journal_view(state, now_seconds)->JournalView`. `selected` is `journal.selected()` clamped. `rows_for` :98.
- `JournalView` (graft-ui journal_screen/view.rs:125) fields: `location`, `rows: Vec<JournalRowView>`, `selected`, `status`, `title_suffix`, `status_row`, `enter_words`, `branch_here`, `box_open`.
- Title: journal_screen.rs:194 `"JOURNAL · what happened in this repo, newest first"`, followed by `" · "+suffix`.
- Bar: `footer` :206 builds `[enter_words] "b new branch here" "space command log" "J/esc close"`, joined with two spaces. When space runs short it drops middle keys. When not Ready it shows only `"J/esc close"`. **No `n`/`N` hint is drawn.**
- Cursor: pieces.rs:16 `"▸ "` / `"  "`.

## 7. Tests
- `tests/it/journal_screen.rs`: `in_j(app,&state)->AppState` :31 (presses J, then pumps until `!finding_lost`) and `from_bottom` :26. It imports from `m7_walk::journal`.
- Helpers in `tests/it/m7_walk/journal.rs`:
  - `lost_work()->LostWork` :94 (`.work()` is the repo path)
  - `opened(repo)->(App,AppState)` :109
  - `drawn(&state,cols,rows)->Vec<String>` :99
  - `text(&[String])->String` :104
  - `selected_row(&[String])->&str` :160
  - `open_j(&mut Route,..)` :119 (private)
- `App::key(&self,&AppState,KeyCode)->AppState` app_harness.rs:177; `App::pump(state, what, done)` :243. No paste helper found in App (could not confirm one exists).
- Captures use `check_shot` (from `crate::captures`, imported at :30). `journal-expanded-120x40.txt` is written by test `journal_expanded_capture` :432, and `check_shot` is called at :464.

## 8. `graft-ui/src/text_input.rs` `TextField` :87
- Methods: `new(&str)` :97, `text` :105, `cursor`, `len`, `is_empty`, `insert(char)` :128, `insert_str` :140, `backspace` :147, `delete`, `left`, `right`, `home`, `end`, `word_left`, `word_right`.
- **No `clear`/Ctrl+U on TextField.** `SecretField::clear` :402 exists.

## Contradictions vs keymap §2.11 (02-keymap.md:449+)
- §2.11 gives J its own `/` find (row words), with `n`/`N` as match nav and Esc dropping the find first. The code has no J find. `/` opens the graph search over J (update/mod.rs:1075, search.rs:20). With a graph search active, `n`/`N` step the graph search, not J (search.rs:22).
- The `modal_owns_keys` doc (state.rs:828-830) says `/ search` "works there" in J. That is the graph search, not the §2.11 find.
