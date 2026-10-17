# Scout 02-clash-reads (graft-git)

## forecast_merge — `crates/graft-git/src/history_reads/forecast.rs`
- Sig `:63`: `forecast_merge(repo, git: &GitVersion, base: Option<Oid>, ours, theirs, stop: &dyn Stop) -> Result<Forecast, RunError>`.
- Version gate `:71-81`: <2.38 (no base) / <2.40 (base) → `NotCheckable(reason)`.
- Runs `merge-tree --write-tree --name-only --no-messages -z [--merge-base=] ours theirs` (`:83-101`), `run_raw`.
- Exit 0 → `Clean` (tree id discarded) `:105`; exit 1 + tree id → `Conflicts{paths}` raw bytes `:106-118`; exit 1 empty stdout or other code → `NotCheckable(first stderr line)` `:124-127`.
- `Forecast` enum `:35-49`: `Clean | Conflicts{paths} | NotCheckable(String)`. Unrelated histories: no code path special-cases it; result = whatever merge-tree's exit gives — *could not confirm* which (not tested in `tests/it/history_reads.rs:238-410`).
- Sibling `merge_tree_step` `:~170` same read, keeps the clean tree id.
- **Output does NOT give changed paths or hunk counts**: `--name-only` prints only conflicting paths; no code reads the tree id on exit 1 or diffs it. Changed files and per-file clash places need extra reads (not present in the crate).
- Callers (stay put): `graft-ops/src/tour.rs:308`, `graft-ops/src/preview/landing.rs:137`.
- Tests: `graft-git/tests/it/history_reads.rs:238,286,410`, `src/history_reads/tests.rs:60` (old git).
- Stop form: `run.rs:118` `trait Stop: Sync`; callers pass `&()`.

## CommitSummary — `crates/graft-git/src/history_reads.rs:60-70`
- Fields: `id: Oid, parents: Vec<Oid>, subject: String, author: Identity`.
- `Identity` (graft-core `commit.rs`) already carries `when: Timestamp`; `parse_summary_line` fills it from `%ad --date=raw` (`history_reads.rs:206-246`, format string `:162`).
- Literal sites: `history_reads.rs:~236` (parser); `graft/src/update/resolver/names.rs:181`; `graft/src/view/tests.rs:3539`; `graft/src/update/remote/tests/lease/leaves.rs:22`. (grep `CommitSummary {` over crates/.)

## Region / ConflictFile — `crates/graft-git/src/conflict.rs`
- `Region::Resolved { block: Block, take }` `:220-232`; doc says "The block as parsed, intact" `:227`. `take()` moves the whole `Block` in (`conflict/document.rs:66-82`); `take_all` same `:88-100`. So yes, whole block kept; `Block` holds ours/base/theirs/labels/marker ends/reproduced (`conflict.rs:151-168`).
- `NoSuchBlock(pub usize)` **already exists** `conflict.rs:329` with Display/Error; used by `take` and `graft-ops/src/conflict.rs:125,160`.
- No `reopen`/untake exists (grep).
- `FileDiff` `diff/mod.rs:147`: entry, hunks (`Hunk` `:106`, old/new start+lines), binary, truncated, modes.

## Contradictions with the spec
1. Spec `CommitSummary { subject: Vec<u8>, author: Vec<u8>, when }` — code has `subject: String`, `author: Identity` (`history_reads.rs:66-69`). "existing fields keep their types" contradicts the listed types.
2. Spec adds `when` for author date — already present as `author.when` (`history_reads.rs:241-245`); `parse_rev_list` already reads it. Acceptance "parse_rev_list reads `when`" is already true.
3. Spec `pub struct NoSuchBlock(pub usize)` as new — exists `conflict.rs:329`.
4. `merge_forecast` sig has no `git: &GitVersion`; today's not-checkable path depends on it (`forecast.rs:65,71-81`). Could not confirm where a version would come from inside the crate without it.
5. Scope assumes merge-tree gives changed paths/places — code reads only conflicting names (`forecast.rs:89`).
