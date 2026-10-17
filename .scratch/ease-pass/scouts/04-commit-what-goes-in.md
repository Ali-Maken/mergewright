# Scout 04 — commit-what-goes-in (graft-ops commit/amend)

## How commit and amend write today
- One struct for both: `Operation::Commit(CommitParams)` / `Operation::Amend(CommitParams)` — `crates/graft-ops/src/operation.rs:80,82`. **There is no separate amend-params struct.**
- `CommitParams` — `operation/types.rs:55`: `message: String`, `sign_off`, `no_verify`, `allow_empty`.
- Plan: a single `git commit -F -` (message on stdin), plus `--amend`, `--signoff`, `--no-verify`, `--allow-empty` — `operation/commit.rs:47-66`. Uses the **real index**; hooks run unless `no_verify`.
- `Invocation` (`operation/types.rs:102-110`) has `args`, `stdin`, `remote` — **no env field**. A `GIT_INDEX_FILE` scratch index exists only in snapshot code (`snapshot/plumbing.rs:117,132,247`, `snapshot/commit.rs:74`, `snapshot/raw.rs:172,211`), not in the operation plan path.
- Checks: commit refuses `NothingStaged` when `status.staged()` empty and `!allow_empty` (`commit.rs:19-27`); amend refuses only `UnbornHead` and `EmptySubject` (`commit.rs:30-35`). Wired at `operation/check.rs:90-91,129-130`.
- Touches: `(vec![head_ref_name], index=true, worktree=false)` — `operation/check.rs:207-208`. No pins (`apply/record.rs:436-437`).
- Describe: `commit "<subject>"` / `amend "<subject>"` — `operation/queries.rs:284-285`. No file names in the journal wording.

## Journal / undo
- Because `touches.index`, the entry records `index_before` from the backup (`apply/record.rs:120`) and `index_after` (`record.rs:266-275`); ref before/after for the HEAD ref.
- `u`: `undo.rs:300-335` generic steps: ref back, then `Step::RestoreIndex { index_before, paths: ChangedBetween{from: index_after, to: before} }` via `update-index --index-info` (`undo.rs:55-62`). Worktree is not touched (worktree=false). So today `u` already restores HEAD + index; worktree unchanged by design.

## Every CommitParams construction
- Production: `crates/graft/src/update/commit_editor.rs:153` (`params()`, used by `operation()` at :165/:167 for both commit and amend) and `commit_editor.rs:340` (amend at an `edit` stop, all flags false).
- Tests: `graft-ops/src/operation/tests.rs:111,228,238,563,1208`; `graft-ops/src/operation/remote/tests.rs:99`; `graft-ops/tests/it/{aborted_fetch,edit_stop,error_rows,journal_tie,ops_execution_hard,ops_execution,remote_ops,undo_gate}.rs`; `graft/src/update/operate/tests/{landing.rs:132,lands_on_head.rs:27,staging_not_kept.rs:102,166}`.
- Reword/fixup/squash do **not** use CommitParams: they are `Operation::Rewrite` (`operation.rs:123`, `rewrite/plan.rs`); amend-into-older uses `Refusal::NothingStagedFor` (`rewrite/plan.rs:235`, `graft/src/update/amend_into.rs:175`).
- Other matchers on Commit/Amend: `operation/queries.rs:17-18,87-88`, `operation/types.rs:209`, `graft/src/update/operate.rs:205-206`, `graft/src/update/operate/outcome.rs:60`.

## Partially staged file in StatusEntry
- `graft-git/src/status.rs:142-162`: one entry with both `staged: Option<Change>` and `unstaged: Option<Change>` Some. `untracked: bool` separate. Iterators `staged()/unstaged()/untracked()` at :263-273; `entry(path)` :297. No explicit "partial" flag.

## Contradictions with the ticket
- Signatures say `message: Vec<u8>`; code is `message: String` (`types.rs:57`), and `subject()`/`message_bytes()` take `&str` (`commit.rs:11,68`). Ticket also says "existing fields keep their types" — self-contradictory.
- "Name the amend params struct": none exists; Amend shares `CommitParams`, so adding `include` covers both.
- Recommended scratch-index approach: the operation plan cannot set `GIT_INDEX_FILE` (`Invocation` has no env, `types.rs:102`); needs executor/Invocation change in graft-ops or a different route. Could not confirm how the executor spawns invocations (not read).
- `Refusal::NothingChosen` does not exist (`operation/refusal.rs:33` has `NothingStaged`); new variant needs `explain.rs` wording (`explain.rs:55`).
