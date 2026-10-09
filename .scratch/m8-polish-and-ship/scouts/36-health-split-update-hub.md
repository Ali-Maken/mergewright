# Scout 36 — split graft `update/mod.rs`
Root main @ fc6972fe. Crate set: **graft only** (`src/update/`, `MAP.md`).

## File
`crates/graft/src/update/mod.rs` = **1420 lines** (ticket says 1406).
- :1–20 doc; :21–66 46 `mod` lines; :68–80 re-exports; :82–87 `use`.
- :103 `opening`, :143 `lands_at_open` (pub) (~60).
- :164–201 `pub fn update` (38). :202 `keep_unseen_failure`, :215 `settle` (~70).
- :272–526 `answer` (255, one fn).
- answers :527–1004: `mergetool_returned`:527, `journal_answer`:569, `stash_answer`:578, `reverted_to_answer`:584, `undone_answer`:599, `started`:616, `rewrite_answer`:672, `commit_editor_answer`:703, `operate_answer`:733, `repository_answer`:783, `repository_opened`:791, `history_started`:813, `working_tree_read`:845, `working_tree_answered`:898, `stash_entries`:943, `stash_entries_read`:953, `stash_only_answered`:976 (~478).
- :1005 `took_fact`, :1089 `refs_failed`, :1102 `repository_open_failed`, :1123 `name_prompt_claimed`, pub :1136 (~140).
- :1143 `dispatch`, :1185 `resized`, :1206 `core_event`, :1284 `focus_gained`, :1309 `job_cancelled`, :1349 `job_failed`, :1396 `side_job_ended`, :1409 `show_rows` (~276).
- :1419–1420 `#[cfg(test)] mod tests;` (tests.rs 128 + `tests/`).

## Seams / counts
header+update+settle ~200 stays; `answer` 255 own file; answer helpers ~478 → two (:527–783 / :783–1004); facts/failures ~140; event dispatch ~276.

## Pins
`update/tests/purity.rs`: walks `update/` (:37), special-cases `mod.rs` test marker (:76–79) — new children are scanned. `tests/it/seam_rule.rs` names only `update/keys.rs` (:68–75), not mod.rs. Keymap sweeps under `update/tests/`.

## Contradictions
- `over_400.txt` / `file_size` test **absent** (ticket 33 unlanded).
- Ticket 40 (geometry-one-source) edits `update/` (geometry, tour, banner): serialise, as ticket says.
- graft MAP shared with 35 and 39.
