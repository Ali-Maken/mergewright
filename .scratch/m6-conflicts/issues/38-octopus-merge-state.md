# octopus-merge-state: an octopus merge in progress makes every journaled operation fail to begin

Status: done
Blocked by: 20 (Part A `bf218b44` + Part B, merged together). Same crate as nothing else then building.
Was: filed by the conductor 2026-09-24 from 20 Part A's hand-back. A regression Part A introduced; it was not there at `09db5433`. Found by reading; no test yet.
Build with: **Opus** (medium) — git writes, undo.
Needs: 20 merged.
Crates: graft-ops (graft-core only if the journal's `MergeState` shape must change).

## The defect (lines from the Part A builder)
- `crates/graft-ops/src/merge_state.rs:196`: a `MERGE_HEAD` with more than one line → `Octopus` error.
- `merge_state.rs:86`: `read_merge_state` propagates it with `?`.
- `apply/record.rs:66-67`: `begin` maps it to `PrepareError` with `?`, so **every** entry's `begin` fails while an octopus merge is in progress (e.g. `git merge --no-commit a b`). Reachable from the shell; rare.

## Scope
In:
1. Failing test first (real git: two branches, `git merge --no-commit --no-ff a b` → `MERGE_HEAD` has 2 lines). Run any journaled op (e.g. a stage or a branch create) → today `PrepareError`. Red on unchanged code.
2. Fix at the cause: reading the merge state never blocks an unrelated operation. The recorded state must hold an octopus `MERGE_HEAD` **byte-exact**, so undo puts it back as it was. Test: after `u`, `MERGE_HEAD` is byte-equal, both lines.
3. `FinishMerge` (`c`) on an octopus merge: finish it or refuse it with a named refusal (never `PrepareError`). Say which in NOTES and why. Test that.
4. MAP.md (graft-ops §merge_state, §apply).

Out: an octopus fixture in `graft-fixtures` beyond what the tests need.

## Acceptance (each tick names its test, at tick time)
- [x] Failing test first, red on unchanged code — test: `graft-ops/tests/it/octopus_merge.rs` (first form, `u` of a pre-merge entry during `git merge --no-commit --no-ff a b`) failed on unchanged src with `undo: the merge in progress: an octopus merge is in progress (2 heads); Graft records one MERGE_HEAD`. The final tests use the new API, so on old src they fail to compile rather than at run time.
- [x] An unrelated op begins and records during an octopus merge — test: `octopus_merge::an_unrelated_operation_begins_and_records_during_an_octopus_merge` (Undo entry journaled; `merge_before` has 2 heads, `merge_head_bytes()` == disk bytes; `merge_after` equal)
- [x] Undo restores the 2-line `MERGE_HEAD` byte-exact — tests: `octopus_merge::undo_during_an_octopus_merge_keeps_merge_head_byte_exact` (all three merge files byte-equal after `u`) and `octopus_merge::set_merge_state_writes_an_octopus_merge_head_back_byte_exact` (files removed with `git merge --quit`, `Step::SetMergeState` via `restore::execute` writes them back byte-equal, and `rev-parse --verify MERGE_HEAD` succeeds)
- [x] `c` on an octopus merge: finishes or names its refusal — test: `octopus_merge::c_on_an_octopus_merge_names_its_refusal` (`Refused(OctopusMerge { heads: 2 })`, words name `git commit`, no entry, merge files untouched)
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest (2997 passed, 1 skipped), doctests — all pass

## NOTES
- **Cause fixed**: `merge_state::commits` reads every `MERGE_HEAD` line; `MergeState` (graft-core) gains `other_heads: Vec<Oid>` (`serde(default, skip_serializing_if = "Vec::is_empty")` — old journal lines read and re-render byte-identical, tested in `graft-core` `an_octopus_merge_state_keeps_every_head_in_order`). `MergeStateError::Octopus` removed. Backup anchors now take every head (`record::with_anchors`, `recover`).
- **Which operations actually begin during a shell-begun merge**: not stage/unstage/fetch — with no Graft entry behind the merge they fold (`apply/fold.rs:54-58`) and never call `begin`. What begins is `u`/redo/revert-to (`restore::execute`) and `c`. So the reachable break was: **`u` refused during any octopus merge**. The ticket's "e.g. a stage or a branch create" does not reproduce (stage folds; branch create is refused `InProgress` by `check`).
- `u` during the octopus asks first (`a merge is in progress now — u again to go ahead`, merge drift `Begun`), then runs on the second `u`. That is the existing drift rule, not new; the tests press both.
- **`c` refuses, `OctopusMerge { heads }`**: `this merges 2 branches at once (an octopus merge); finish it in the shell: git commit`. Why refuse, not finish: `finish::finish_summary` classifies each path against one `MERGE_HEAD:path` (`ResultIs::Theirs`) and `merged_from` parses only `Merge branch 'x'`, so the summary shown before confirm would describe a merge of one branch — M5's promise. `FinishMergeParams.merge_head` also pins one commit. Before this, the same case said `NotInProgress` ("nothing Graft started is in progress…"), which was false. Checked before `refuse_unmerged`, so an octopus with unmerged paths (if git can leave one — not tested) gets the octopus words, not the unmerged ones.
- **Unverified in the app**: the app crate (graft) shows refusals through `Refusal::explain`; no capture of the banner's `c` line on an octopus was taken (out of crate set). Its `operate.rs:885` match has a `_ => None` arm, so it compiles and falls to the plain explain line.
- **Left as is (not octopus)**: a `MERGE_HEAD` holding a non-id line (hand-edited, corrupt) still makes `read_merge_state` fail → `u` refused `PrepareError` while it stands. Same shape of defect as this one for a far rarer input; the conductor may want a gap row. `merge_head_bytes` re-renders git's form, so a hand-edited `MERGE_HEAD` (blank lines, spaces) comes back normalised after `u`, not byte-exact — byte-exact holds for every file git wrote.
- Scout: none. The ticket named the three lines; `merge_state.rs` (313 lines) was the file being changed, and the rest was read by grep at the named call sites.
- Not compacted.
