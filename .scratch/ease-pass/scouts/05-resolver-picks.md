# Scout — 05 resolver-picks (main d2af08f2)

Paths under `crates/`. `graft/src/update/` abbreviated `u/`.

## Where a stop lands, P11, `⏎`, "done"
- Stop → **graph, banner focused**, not the resolver: `u/relaunch.rs:49 land_on_live_operation` sets `banner_focus` (:85) and calls `land_on_the_work(first unmerged)` (:105). Resolver opens only on banner `⏎` (`u/banner/pressed.rs:71`) or pane.
- `⏎` = `mark_resolved` (`u/resolver/resolved.rs:25`): refuses while a block is open (`graft_ops::conflict::mark_resolved_refusal`, :39), else launches `Operation::Stage(Target::Paths([path]))` (:55); `staging` id held (:71).
- "File done" = git no longer calls it unmerged: `DoneOpen`/`DoneShape` (`u/resolver/done.rs`), read-only; refusal words `<path> is done · d starts it over` (`done.rs:57 done_refusal`). Unmerged list = `u/banner.rs:106 unmerged_paths` (status `entry.unmerged`).
- Last file staged → `stay_after_last` (`resolved.rs:211`) sets `finish_owed` for merges (:220); `finish_if_owed` (:286) opens the summary via `banner::continue_pressed(ReturnTo::Resolver)` (:316). Cancelled by `esc j k n N d e M ↑ ↓` (`resolved.rs:260`).
- Takes: `o/t/b` → `take_block` (`u/resolver/take.rs:52`) → `Command::TakeSide` (:163), queued behind in-flight (:158); cursor moves at keypress via `walk::after_take` (`take.rs:202`). **Re-taking a taken block today replaces its take** (`graft-git/src/conflict/document.rs:63-66`) — no refusal. `Ctrl+O/T` (`take_all_key`), `O/T` (`settle_key`, stages the whole file, `take.rs:266-267`) — **not addressed by spec** (do they push Picks? `O/T` stage in a merge).

## finish_summary / relaunch
- `finish_summary(repo)` (`graft-ops/src/finish.rs:139`) reads **index status**: `StillUnmerged{count}` while any path unmerged (:147-150); `by_you`/`by_git` from staged diff. No `stop` param (uses `&()`, :141).
- Relaunch finds unmerged via the status read (`banner.rs:106`); no marker check anywhere on that path.

## Resume / FinishMergeParams
- `Resume::Continue` is a **unit variant**; `ContinueWith { message: Vec<u8> }` (`graft-ops/src/apply/resume.rs:38-52`).
- `FinishMergeParams { merge_head: Oid, message: Option<Vec<u8>> }` (MAP.md:231; def `graft-ops/src/operation/finish_merge.rs:15`). Built only at `graft-ops/src/apply/resume/shell.rs:59` (from `Resume`), applied `resume.rs:134`; test `operation/tests.rs:1266`.
- `Resume` construction (non-test): `u/finish/keys.rs:236,251,253`; `u/banner/pressed.rs:91`; matches `u/banner/resume.rs:32`, `banner/resume/answers.rs:150,165`, `jobs/runner/operations.rs:36`. Tests: `finish/tests.rs`, `banner/tests.rs`, `quit/tests.rs:304,753`, `keys/live_c_tests.rs:254`, `banner/rewrite_stop_tests.rs`.
- Shell-begun merge: **no `Pending` entry** — `resume_target` returns `Target::ShellMerge` and goes through `apply(FinishMerge)` (`resume.rs:100-105,134`). Same `resume` entry, so not a separate path; but see contradiction 4.

## Stash clash
- In: `land_on_clash` (`u/resolver/open.rs:52`) → `open_resolver(.., OpenedFrom::Pane)`. Out: `close_after_last` (`resolved.rs:161`) → `Operation::DropPutBack` if `clash::drops(why)` else `kept_status`. `u` answer: `clash::undo_clause` (`resolver.rs` `undo_answered`).

## Key routing
- `act` (`u/resolver.rs:343-385`) matches raw `KeyCode`s; `y` today falls to `_ if plain => scroll_key` (:382) — no arm. Ids reach the resolver only by literal match; `asks_undo` (:394) is the one id lookup (`Keymap::shipped().chords`).
- `u` in resolver → `undo_answered` (:409): live-op refusal words (§2.6a) / clash clause / graph hint. No command.
- Finish summary `u`: **swallowed silently** in Files focus (`u/finish/keys.rs:106 _ => {}`); in **Subject/Body focus `u` is typed text** (`subject_key` :117, `body_key`). `y` commits in Files focus only (:100). `esc` → `finish::escape` (:44).
- Resolver `esc` → `leave.rs:40 escape` → `close` (:50): `banner_focus` set **only if `opened_from == Banner`** (:55-58); from pane/graph it is not focused.
- `N` = `walk_blocks(forward=false)` (`resolver.rs:373`, `walk.rs:29`): prev block in file, then crosses files, then wraps (`wrapped · K still open`), else `no more conflicts this way` (`walk.rs:26`). `n` same forward — already crosses files.

## Scratch index
- `snapshot/plumbing.rs:117` is **`read_ref`**, not `GIT_INDEX_FILE`. The env route is `plumbing.rs:15-39` (`write-tree`) and `:145-154` (`add -A`), read-only plumbing. `IndexFile::Scratch` (`graft-ops/src/operation/types.rs:150,186,214`) is an executor `Operation` command-line feature.

## Keymap
- `resolver.next_block` `table.rs:323`, `resolver.prev_block` :324, `resolver.finish` :325, `resolver.mark_done` :327, `app.back` :77, `app.undo` :78, `confirm.yes` :235 (borrow comment :202; `ContextId::borrows` `graft-ui/src/keymap.rs:190-204`).
- `keymap/tests/ease.rs:26` asserts `y` None in Resolver. `bound.rs:127,130` lists resolver ids.
- Sweeps: `graft/src/update/tests/sweep.rs`, `tests/palette_live.rs`, `tests/keymap_edge/sweep.rs`; ui `graft-ui/src/keymap/sweep.rs`.

## Picks not journaled
- Confirmed for Graft-begun ops: `graft-ops/src/conflict.rs:4-8` (take = fs write inside `Pending` span, backup there).

## Contradictions
1. `confirm.yes` cited `keymap.rs:204` — it is `keymap/table.rs:235`; `keymap.rs:202-204` is the borrows comment.
2. `plumbing.rs:117` is `read_ref`; `GIT_INDEX_FILE` at :24, :39, :154.
3. `Needs: ConflictFile::reopen` from clash-reads — **already exists** `graft-git/src/conflict/document.rs:89`; `NoSuchBlock` at `graft-git/src/conflict.rs:329` ok. `write_back` takes fingerprint (`graft-ops/src/conflict.rs:161,185`).
4. "a pick is covered by the operation's backup": a shell-begun merge has **no Pending entry/backup** (`resume.rs:100-105`) — that premise fails there.
5. Spec "on a taken block: notice, no write" — today re-take replaces (`document.rs:63`); behaviour change, not preservation.
6. Q7 "esc → banner focused": true only when opened from banner (`leave.rs:55`).
7. Review `u` → resolver: in Subject/Body focus `u` is text input (`finish/keys.rs:117`).
8. `O/T`, `Ctrl+O/T` unmentioned; `O/T` stage today.
9. Spec "resolver.rs:363 → escape, :379 asks_undo" ok (lines 363, 379).
