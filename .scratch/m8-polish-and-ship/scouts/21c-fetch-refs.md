# Scout 21c — fetch records the refs it moved (code at 26f1286f)

## Fetch and pull paths (graft-ops)
- `plan_fetch` `operation/remote.rs:281–291`: one invocation `git fetch --progress [--prune] (--all | <name>)`. Multi-remote fetch = **one** `--all` git line, not one per remote.
- `plan_pull` `operation/remote.rs:296+`: fetch `<upstream.remote>` then a local `merge --ff-only`.
- `apply` `apply.rs:231`. Order: facts → `op.plan` (:253) → `pull::fetch_first` (:255) → `touches_of` (:256) → Command-Log vec (:260; fetched pull's fetch line pushed at :263) → set-aside → `pull::open_after` → `begin` (:285) → `run_restart_prep` → `commands.extend(to_run…)` + `pull::run_rest` (:306–307) → `closed(…)` (:309).
- Plain fetch: entry opens at :285 **before** git runs; git runs in `run_rest` (`pull.rs:~95`, `Ahead::Nothing`).
- Pull: `fetch_first` `apply/pull.rs:41–59` runs the fetch **before the entry opens** (`run_plan`, :50). Fetch failed → `Ahead::Failed`, `open_after` (`pull.rs:67–81`) opens the entry with the stop set aside. Fetch ok but `incoming` read fails → `Err` at :54–57, **no entry** (doc :37–39).
- `record::begin` `apply/record.rs:68`: before-reads `head_ref`, `read_refs(touches.refs)`, `read_merge_state`, `config_before` (:84–89). Entry built with `commands: Vec::new()` (:~127). **begin's reads are not on `commands`**; `finish` (`record.rs:220–240`) sets `entry.commands = commands` (the plan lines from `apply`), `read_after` (:249+) also unlogged, then `append_outcome` (:237).
- `ApplyError::Remote` words: `remote::explain` `apply/remote.rs:315`; journal reason `remote::short` :290.

## in_force / next_undo (graft-ops `journal.rs`)
- `took_effect` :162 (Aborted arm :165: `worktree_before.is_some() && changed_anything`).
- `changed_anything` **exists**, :173–186, private: refs_before≠after, head, index, worktree, merge. Callers: `took_effect` :165, `is_no_op_pull` :245. It is not the ticket's `wrote_anything` meaning.
- `in_force_flags` :190, `in_force_all` :206, `undoable_kind` :225 (Fetch, Push false), `undoable` :238, `in_force` :251, `next_undo` :264.

## J mark and words (graft)
- Mark: `view/journal.rs:184–206`. `in_force==false` ∧ not reversed ∧ Aborted → **`RowMark::Refused`** (:198); else `Undone`. **Contradiction:** ticket item 7 names `RowMark::NothingWritten`; code has none. Text is `"refused · nothing written"` (`graft-ui journal_screen/view.rs:42`), not bare `nothing written`.
- Summary `view/journal/summary.rs:31`; Fetch arm :57–61 = `remote-tracking refs · not undoable` + `with_failure`. **Contradiction:** ticket says a Committed fetch today's summary is replaced by `origin/main moved`/`nothing new` but never states current text is `remote-tracking refs · not undoable`. Failure words `failure` :133–150 (`failed: <reason>`).

## Expanded J row (graft-ui)
- `JournalRowView` `journal_screen/view.rs:~70–101` (`commands` :76, `failure` :79). Expanded draw `journal_screen/pieces.rs:204–208`: failure, then `COMMANDS_HEADING` (`journal_screen.rs:79`, "commands Graft ran").

## On-disk format
- JSONL, two line kinds; `Line::from_json` `journal/lines.rs:~110` picks by `status`. `parse` `journal.rs:75`; `render_outcome` :137. **No version field**; compat is serde `default` + `skip_serializing_if` per added field (`merge_before`, `config_before`, `put_back` lines.rs:~64–75; `merge_after`, `config_after` :96–101). `OutcomeLine::of` lines.rs:185; `apply_to` after it; `into_entry` :155.
- Literals: `JournalEntry {` graft-ops 46, graft 45, graft-core/graft-ui 0. `OutcomeLine {` literal: 0 outside `of`. `mark_interrupted` (`journal/file.rs:177`) clones the held entry (no literal); `recover_interrupted` `apply/recover.rs:31` — no literal found.

## Tests
- `aborted_fetch::a_fetch_that_moved_refs_before_one_remote_failed_is_in_force` **does not exist** in code; no `tests/it/aborted_fetch.rs`. Only mentioned in `21b-hygiene-round-2.md:243` ("not committed").
- Fetch `commands` asserted: `graft-ops/tests/it/remote_ops.rs:173`; pull: `pull_set_aside.rs:89`. `next_undo_steps_past_a_push_and_a_fetch` `journal/tests.rs:460`.
- "remote-tracking refs" row text in `graft/src/view/journal/tests.rs`, `graft-ui journal_screen/tests.rs`.
- `m4-07-journal-remote-rows-*` harness: `graft/tests/it/m4_walk/hangs_and_undo.rs`, `coverage.rs`.
