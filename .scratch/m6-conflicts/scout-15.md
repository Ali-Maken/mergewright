# Scout 15 — take advances to next block (main c56b64e)

Paths relative to `crates/graft/src/update/`.

## What exists
- Keys: `resolver.rs:269` `o|t|b` → `take::take_key`; `:271-272` `n`/`N` → `walk::walk_blocks`. Keymap §2.6 (`docs/design/02-keymap.md:260-262,267`) already carries the P2 rows (advance; `n`/`N` include taken blocks).
- `take.rs:43 take_side`: `write_refusal` (`resolver.rs:216`, words from `busy_words` `resolver.rs:193-202` → "still writing the last take") → refused at `take.rs:58-61`. Base load refusal `:68-71`. Sets `open.job = Some(id)` (`:83`), `reading=false`, pushes `Command::TakeSide{path,file,block,take}` (`:86`). **Scout Q1 answered.** Cursor is not touched.
- Whole-file card: `take.rs:24-35` goes to `whole_file::whole_file_take` first (stages via `open.staging`).
- Answer: `answers.rs:118 taken` → `Ok` → `take_landed` (`:157`). Failure: `job=None`, notice `(words,true)` (`:136-137`); `failure.reread` → new `OpenConflict` job, `reading=true` (`:141-150`).
- `take_landed` (`:157-190`): reads `cursor` (`:161`), sets `file`, `job=None`, `notice=None`, `remember`, re-scrolls **the same** block (`:173-179`), status `took <side> in <path> · k of n open`. **Scout Q2: `cursor_block` is never assigned on a Taken answer — confirms ticket 14's builder: no auto-advance today.**
- `walk.rs:14 walk_blocks`: `next_open_block` (`walk.rs:72`, open-only, no wrap); none left → forward only `next_conflicted_file` (`walk.rs:88`, **`banner::unmerged_paths` order**, not the file list) → `reopen`; backward stays; else Sticky `no more conflicts` / `⏎ marks <path> resolved` (`:60-65`).
- `resolved_advance` (`answers.rs:230`): on `staging` landing, next file in **`listed_files` order**, cycling, skipping `RowState::Done` (`:257-270`). This is the list-order crossing item 3 wants.
- `ResolverState` is in `resolver.rs:97` (no `resolver/mod.rs`). No `queued` field. `graft_ops::conflict::apply_take` is the worker write (`events.rs:627`).
- Tests: `resolver/tests.rs` — `n_on_a_card_crosses_to_the_next_file` :58, `n_on_the_last_card_says_no_more` :72, `a_take_or_a_base_in_flight_keeps_its_own_words` :381 (asserts the refusal the queue replaces), `a_take_waits_for_the_stage_in_flight` :583.

## Contradictions
1. Signatures name `update/resolver/mod.rs`; the struct is at `update/resolver.rs:97`.
2. Item 2 "records `AppState::choices`" — no such field (`grep choices state.rs` empty). Ticket 19 (finish summary) owns it and is not in `Blocked by`.
3. `FileState::Done` (Needs line) does not exist; the type is `RowState::Done` (`files.rs:23,34`).
4. Order of item 3 vs keymap `02-keymap.md:267`: keymap crosses to next file **first**, then wraps in-file; ticket wraps in-file first (`wrapped · k still open`), then crosses. Disagree.
5. Today's end words are `⏎ marks <file> resolved` (`walk.rs:63`); ticket quotes `… done`.
6. `walk.rs:88` crosses in `unmerged_paths` order; ticket wants list order (`resolved_advance`'s).
7. Existing test at `tests.rs:381` asserts the refusal words for a take in flight; item 2 changes that behaviour.

## Could not confirm
- Whether `ConflictFile::take` is pure / public in `graft_git` (not read).
