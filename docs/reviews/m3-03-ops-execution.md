# Review: m3-working-tree-undo-and-stash/03 — ops-execution

Reviewer: Fable (`reviewer`), 2026-09-19. Range `7ebfa52..HEAD`, the
conductor's `d293ca6` skipped. Read: the spec incl. its `## NOTES` (15
deviations), as-built, group 02's `## NOTES` (the contract), the design note,
DESIGN.md 06 §6.1/§6.8, `graft-ops/MAP.md`, then the diff crate by crate
(journal/file → snapshot → facts → apply → restore → undo/execute → the three
test files). Opened no source file whole; `undo.rs` and `many_refs.rs` were
grepped and read in 20-line windows to confirm findings. Claims marked
*verified* were run against git 2.54.0 in a throwaway repo (untracked, deleted).

**Counts: 1 Critical, 6 Major, 11 Minor.**

## Gates

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo test` (fast tier) | 1402 tests listed, 0 failed; **28.3 s wall** (26.1 user + 17.1 sys); peak RSS **339 MB**. Builder's 42 s included a compile |
| `cargo test --features heavy-tests -- --test-threads=2` | **1396 passed, 0 failed, 9 ignored**; 161.9 s wall; peak RSS 726 MB |
| release perf run (`frames:: heavy::budgets_hold`, `--test-threads=1`) | **13 passed, 0 failed**; 72.6 s; numbers below |
| fuzz / benches | none exist |
| `unwrap()`/`expect()` outside tests | none. `let _removed` in `restore_worktree` and `TempRepo::drop` are commented. `unwrap_or_default()` in `apply::finish` is the banned pattern standing in for a failure — Major M1 |

Five slowest fast-tier tests (single-threaded, wall ms): `diff_fixture::parse_unified_matches_numstat_for_every_shape` 6 114; `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 5 912; `corpus::two_builds_produce_the_same_object_ids` 2 715; `diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 1 410; `corpus::a_second_build_in_the_same_place_is_a_no_op` 1 011. None of this group's 49 tests is in the top six; they spawn real git on temp repos and are laptop-safe. `target/`: **17 GB** (16 at the 02 review).

Release perf: `first_paint_us` 12 515 / 14 342 / 12 649 / 15 016 (100k with/without commit-graph, plain/realistic), 35 950 with 50k refs — budget 150 ms. Frame p99: Pixel 50k refs 8 692 us (max 10 317), Unicode 50k refs 8 220 us, detail pane 8 426, diff scroll 8 656, map 7 989, 100k fixture Pixel 8 493 / Unicode 8 347 — budget 16 ms. In-test peak RSS 98.7 / 30.5 / 95.3 / 35.7 / 102.2 MB — budget 300 MB. `bottom_ms` 49–51 with a commit-graph, 464–516 without.

**The 50k-refs debug guard (`many_refs::reading_tips_from_50k_real_refs_is_within_budget`, 200 ms):** passed in the heavy tier at `--test-threads=2`, and three isolated debug runs measured **138.2 / 140.1 / 138.4 ms** (25 543 tips). The 275 ms the 01 review saw was a contended run, not a regression: 30d's own baseline before its fix was 424 ms debug, so the code is where it was left. Verdict: **guard, not regression.** The number has 1.4x headroom and the test holds `MEASURING` but not the machine, so it will flake again under `--test-threads=2`. Per ADR-0021 (only release timings gate) the right change is to make the debug run record-not-assert like the frame budgets, not to raise 200. Conductor's call; the KNOWN_GAPS row can close with these numbers either way.

## What this group is, and where it stands

Snapshots, the §6.2 protocol, restore, undo and redo — the half of M3 the promise rests on. The core is right: both trees from one index copy that never touches the real index (test `reading_the_repository_never_rewrites_the_real_index`), Pending line and backup ref on disk before the first write, outcome line with after-state on abort as well as success (the conflicted-pop test asserts `head_after` and `refs_after["refs/stash"]` on the aborted entry — group 02's contract is honoured, not only on the success path), refs restored by compare-and-swap before any file is written, `update-index --index-info` instead of `read-tree`, and every assertion in the tests reads git back. Seven kinds round-trip apply → undo → redo → undo against `for-each-ref` + `ls-files -s` + every file's bytes + HEAD. The PROMPT §3 binary-bytes test is there verbatim, in three variants.

What is not right is reachability: the entry records tree and commit ids that nothing keeps alive, so `u` works today and stops working after `git gc`'s two-week prune. That is C1, and it is the one thing in this group that breaks the promise rather than narrowing it.

## Critical

### C1. After-trees of index-only operations, the after-tree of a conflicted pop, and the pre-amend commit hang off no ref; `git gc` prunes them and undo/redo then fail

`crates/graft-ops/src/apply.rs:391-425` (`finish`). The after-commit is sealed only when the tuple `(touches.worktree, backup_ref, before_commit, Some(index_after), Some(worktree_after))` is all present. Three cases fall through, and each leaves an object id in the journal that no ref reaches:

1. **Every index-only operation** — Stage, Unstage, Commit, Amend (`touches.worktree == false`). `index_after` is a fresh `write-tree` over the index copy: a loose tree object that no commit contains until the user commits it. The undo plan is `RestoreIndex { tree: index_before, paths: ChangedBetween { from: index_after, to: index_before } }` (`undo.rs:233`), and `scoped_paths` runs `diff-tree index_after index_before`. Once `index_after` is pruned, that spawn fails and the undo is journaled `Aborted`. *Verified:* `GIT_INDEX_FILE=<copy> git add -A; write-tree` then `git prune --expire=now` → `cat-file -e` on the tree fails. `gc.pruneExpire` defaults to `2.weeks.ago`, `gc --auto` runs after ordinary commits, so **`u` on any stage/unstage/commit older than two weeks fails.** Redo of an undone stage needs the same tree.
2. **A stash pop that conflicts** (`touches.index && touches.worktree`, `index_after == None` because the index is unmerged). The tuple needs `Some(index_after)`, so no after-commit is sealed, yet `worktree_after` stays `Some(tree)` — the doc comment two lines above ("the entry says so by carrying no after tree that any ref reaches") describes the `sealed.is_err()` branch and is untrue for this one. `commit_trees` already takes `Option<Oid>` for exactly this shape and is never called with `None` from `finish`.
3. **Amend.** `pins_for` (`apply.rs:433`) pins nothing, so the pre-amend commit is reachable only through the `HEAD` reflog. *Verified:* `commit --amend` then `reflog expire --expire-unreachable=now --all && gc --prune=now` → the old commit is gone, and `update-ref refs/heads/main <old> <new>` fails with `trying to write ref … with nonexistent object`. `gc.reflogExpireUnreachable` defaults to 30 days. After that, undo of an amend is impossible and the old commit's message and tree are **lost** — the one case here that is not just a failed undo. Commit's post-undo redo has the same shape (the committed commit is reflog-only once `main` moves back).

Why it matters: DESIGN.md 06 §6.1 says "a backup ref keeps its objects reachable for ordinary `git gc`, so no extra pinning mechanism is needed while the ref exists" — the design's whole retention story (§6.8, "kept indefinitely") assumes every id an entry records hangs off its ref. The journal browser and ADR-0031's point-in-time query (M5) will read these ids months later. No test can see it because every test undoes within the same second.

Fix (one place, cheap — one `commit-tree` and one `update-ref` per entry, which `finish` already pays for worktree ops): seal an after-commit **whenever `backup_ref` is `Some`**, passing `entry.index_after` through as the `Option` it already is, and move the ref to it. For Amend and Commit, parent the before-commit on `head_before`'s commit (add it to `pins` in `pins_for`, which then needs `facts` — or read it in `begin` from `head_before`), so the pre-amend commit is reachable from the backup ref. Then add one test per case that runs `reflog expire --expire-unreachable=now --all && gc --prune=now` between apply and undo — the pattern `a_dropped_stash_survives_gc_and_comes_back` already uses — for a Stage, an Amend and the conflicted pop.

## Major

### M1. `finish` and `recover_interrupted` swallow read failures; for refs the result is an entry that can never be undone and nobody is told why

`apply.rs:377-388`: `entry.head_after = facts::head_ref(ctx.repo).ok(); entry.refs_after = facts::read_refs(…).unwrap_or_default(); … .ok()` ×2. Deviation 15 argues `None` is "the honest record" because `drift` reports it as unknown. That holds for `head_after` and the two trees. It does not hold for refs: an empty `refs_after` is not "unknown", it is *missing a key of `refs_before`*, which `generic_steps` (`undo.rs:215`) turns into `NotInvertible::Incomplete { field: "refs_after" }`, and rule 9 keeps the entry in force. So a transient ref-store read failure after a commit produces an entry that `u` refuses for ever ("the journal never recorded refs_after"), shadowing everything older, with the actual error discarded at the line that discarded it. The same `unwrap_or_default()` is the pattern CLAUDE.md names. `recover_interrupted` (`apply.rs:229`) does the same one level up: `Err(_unreadable) => None` closes the entry as plain `interrupted` and the reason the repository could not be read at launch is never surfaced.

`finish` also passes `ctx.stop` to the after-reads. The stop that cancelled the operation mid-plan (Esc during a slow pop) also cancels the after-state read, so a user-cancelled op always records `None` trees and forces the `Everything` + confirm path, although the repository is perfectly readable a millisecond later.

Fix: the outcome line must be written regardless (agreed), but (a) `read_refs` failure should be recorded as a distinct, visible thing — simplest is to keep `refs_after` empty **and** put the read error into the `Aborted`/outcome reason and return it from `apply` as `ApplyError::Facts` after the line is written, so the status line says why `u` will refuse; (b) `recover_interrupted` returns the unreadable reasons alongside the closed ids; (c) the after-reads take `&()` (never stop) — the operation already ran; cancelling the read only widens the restore.

### M2. `StashApply` has no execution test of any kind; `StashPop`/`StashDrop` have undo but no redo; parity covers 6 of 9

`grep StashApply crates/graft-ops/tests/it/` finds nothing. Deviation 12 says the four unparitied kinds "are covered by round-trip and inverse tests" — true for Amend (`amend_round_trips`), half true for Pop and Drop (`undoing_a_pop_puts_the_entry_back_and_the_tree_with_it` and `a_dropped_stash_survives_gc_and_comes_back` stop at the first undo; redo of an undone pop — `Redo` with the forward `StashDrop` step and `pins` — never runs), false for StashApply. The acceptance boxes "Parity, every kind" and "Undo restores exactly, every kind" are ticked and are not met. Apply is the only kind whose success leaves the stash *and* changes both trees, i.e. the one whose undo must touch two trees and leave `refs/stash` alone — untested. Fix: `stash_apply_round_trips` and `round_trip` (not one undo) for pop and drop; add the four kinds to `parity` — the harness takes any `Operation`, so it is four ten-line tests.

### M3. No test drives a multi-invocation plan through `apply`, and none fails or stops after a first invocation succeeded; `StopAfter` was written for that and is unused

Every plan the tests run has one invocation (a Discard with staged+unstaged on one path plans one `restore --staged --worktree`; the conflicted pop is one `stash pop`). The plans with two or three — Discard across the staged/unstaged/untracked classes, `Discard(Patch { side: Index })` (`apply -R -` then `apply --cached -R -`, the very case group 02 ordered "working tree first because that is the step that can fail") — never go through `apply`'s loop. `tests/it/support.rs:141` defines `StopAfter` under a file-wide `#![allow(dead_code)]` and nothing uses it. The "crash mid-operation" tests simulate by truncating the journal *after a completed* operation, which exercises recovery but not an entry whose repository is half-way through its own plan. Fix: one test with a three-class Discard and `StopAfter(n)` set to stop between invocations — assert the entry is `Aborted { reason: "stopped" }`, `recoverable`, and that `undo` returns the tree; one with `Target::Patch { side: Index }` on a file whose worktree hunk overlaps so the first `apply -R` fails. Then drop `allow(dead_code)`.

### M4. `tree_entries` silently skips a malformed `ls-tree` record, and an absent record means "delete this index entry"

`restore.rs:497-513`: three `continue`s (no tab, fewer than three fields, `Oid::from_hex` failure). A path missing from `present` is written to `update-index --index-info` as `0 <zero>\t<path>` (`restore.rs:238`), which removes it from the index. So a parse slip on one record deletes a staged entry with no error. `stash_reflog` in the same file (`restore.rs:389-397`) treats the same class of surprise as an error with a written reason — the two are inconsistent, and the one that can lose state is the lenient one. Fix: return `RestoreError::Run` with the offending record, as `stash_reflog` does.

### M5. The unmerged-index fallback drops the conflict's stage entries, and the `Unmerged` short-circuit in `reverse` skips the ref, `HEAD` and worktree drift checks

`snapshot.rs:150-156` + `undo/execute.rs:76-88`. The fallback is right as far as it goes — the conflicted working tree, markers and all, is captured, and the test proves undo of a conflicted pop restores the pre-pop state. Two consequences are unstated: (a) the undo entry's `index_before` is `None`, so **redo** of that undo (re-entering the conflicted state) restores the markers with a clean index — a working tree that says "conflict" and an index that says "resolved", a state git itself never produces; (b) on `Unmerged`, `reverse` returns `Drift { index: true }` without reading `HEAD`, the named refs or the worktree, so the confirm box says "the index changed" when a branch may also have moved (the CAS catches it later, after the confirm) or the worktree edited (the backup catches it, silently). With `force`, nothing is compared at all. Fix: read head/refs/worktree in `read_state_now` even when the index tree cannot be written (return `index_tree: Option<Oid>`), and let `drift` say all of it; file (a) as a limitation in KNOWN_GAPS — recording stage-1/2/3 entries needs a different backup shape and is not M3's.

### M6. `journal_path` and `Context::open` aside, `facts::read_refs` scans the whole ref store to answer for one or two names

`facts.rs:109-124`: `repo.references()` into a `BTreeMap` of every ref, then a lookup. `begin`, `finish` and `read_state_now` each call it, so a commit on a 50k-ref repository (PROMPT §6.3) pays three full ref-store passes per operation, and an undo of it three more. Off-thread, so not a frame problem, but it is the difference between an operation that is bounded by git's own commit and one bounded by our bookkeeping. `RepoHandle` already resolves single refs elsewhere (`upstream`, `head`). Fix: resolve the named refs directly; if `graft-git` has no single-ref API, add `RepoHandle::reference(name)` — it is one gix call.

## Minor

1. `apply.rs:187-207` / `restore.rs:97-112`: when `finish` fails to write the outcome line, the git failure in `failure` is dropped and the caller sees only `ApplyError::Journal`. Attach it (`Journal { error, after: Option<RunError> }`) or at least log both in the returned message.
2. `apply.rs:88-108` `ApplyError::explain` re-implements `reason_for` (`apply.rs:447`). Call it.
3. `undo/execute.rs:70` `&ctx.scratch.clone()` — needless clone; `read_state_now` borrows three fields of `ctx` immutably. `Context::scratch()` exists (`apply.rs:170`) and is used in `begin` but not in `finish` (`&ctx.scratch`) — pick one.
4. `snapshot.rs:243-249` `TempIndex::drop` runs `debug_assert!(false, …)` on a failed removal. A panic in `Drop` during an unwinding test turns the real failure into an abort with no message. Use `eprintln!`/tracing, or nothing with the comment that is already there.
5. `facts.rs:135-146` `resolve`: a symbolic ref whose target is not UTF-8 is `.ok()?`-ed into "absent", the opposite of what `ref_text` does two lines away (`NonUtf8Ref`). Both should error.
6. `restore.rs:312-318` `stash_store`: any `RunError` from `log -1` except `Stopped` becomes `StashGone { commit }` — git not on PATH would read as "the stashed commit is gone". Match `Exited` only.
7. `restore.rs:185-191` `update_ref`: `RefMoved` is detected by `contains("cannot lock ref")`; git prints the same prefix for a lock held by another process (`Unable to create '….lock': File exists` is a different line, but `cannot lock ref` also covers a concurrent `update-ref`). The message "is not where it was" would then be the wrong diagnosis. Safe (nothing was changed), wrong words; match on `is at … but expected` / `reference already exists`. Also the 2.38 concern the conductor already filed.
8. `apply.rs:284` `begin` re-reads `HEAD` although `apply` has `facts.head` from `read_facts` two calls earlier; two reads of one thing give the entry a chance to disagree with what `check` saw. Pass `facts.head` in.
9. `ApplyError::explain`/`RestoreError::explain` put git's first stderr line and the entry's description (user paths, commit subjects) straight into the status-line string. Nothing here sanitises control bytes; groups 04–06 must, and this should be said in MAP.md next to `explain` so the UI builder does not assume it is clean.
10. `journal/file.rs:60-69` `Journal::open` truncates the file. Correct (a torn tail carries nothing), but it is a write inside a function named `open`, and a read-only caller (a future `graft journal` listing, or two Graft instances on one worktree) will not expect it. Say so in the name or the MAP line, and consider `O_APPEND` writers from two processes as a KNOWN_GAPS row: the in-memory copies diverge and `append_outcome`'s duplicate check is per process.
11. Snapshot cost (`docs/perf/m3-snapshot.txt`, not re-run): warm 184–205 ms at 20k+2k files. The numbers do not imply a design change, but they should be read as **per snapshot**, and an undo pays three (`read_state_now`'s two `write-tree`s, then `begin`'s `take`), an apply of a worktree op two (`take` + `finish`'s two trees). On that repository `u` is ~600 ms warm, off-thread. Worth stating in the KNOWN_GAPS row before the user judges "does staging feel right".

## The fifteen deviations, on merit

Better than the spec or neutral: **1** (`Option` index tree — the right cause, though see C1 case 2 and M5), **2** (pins force a backup — the acceptance criterion was unreachable without it), **3** (empty old value — hash-length independent), **4** (`StashGone` repurposed — but see Minor 6), **5** (torn tail cut — correct; see Minor 10), **6** (`mark_interrupted` takes the after-state — necessary, or every interrupted entry is `Incomplete` for ever), **7**, **8** (one index copy, 15 vs 236 ms — the spec's two copies were wrong), **9**, **10** (both are the spec's intent made concrete), **11**, **13** (`%(tree)` not `%(objectname)` — right; the flake it caught was real), **14** (file-scoped allows with a reason — acceptable).

Contested: **12** — a regression against the spec's "every kind", and the note's claim of round-trip coverage is wrong for StashApply (M2). **15** — right for `head_after` and the trees, wrong for `refs_after`, where `None`-by-omission is `Incomplete`, not unknown (M1).

## Spec compliance and Definition of Done

All 13 acceptance boxes are ticked. Not met as written: "Parity, every kind" (6 of 9) and "Undo restores exactly, every kind" (StashApply untested; Pop/Drop have no redo leg) — M2. "Failed midway is recoverable" is met for the one-invocation case only — M3. "Crash between Pending and outcome" is met exactly as the spec words it (hand-written Pending line, `unresolved`, `mark_interrupted`, undo restores). CAS guard, drift guard, ADR-0010 linked worktree, stash inverses through `gc --prune=now`, unborn head, index bytes, backup commit shape, the perf script: met and read back from git. Signatures match the spec except the documented `mark_interrupted(id, after)`, `Snapshot.index_tree: Option`, `ApplyError::Git { entry: Box<_> }`, plus `commit_trees`/`move_backup_ref`/`recover_interrupted`/`head_ref`/`read_refs` added — all in MAP.md. Architecture: `graft-ops` still depends on `graft-git` + `graft-core` only; ARCHITECTURE.md's ops row names the four new files correctly. Readability: names, doc comments and function sizes are good throughout; `restore.rs` at 620 lines is the one file over the spec's estimate and reads fine.

## Does the promise hold?

For what the tests cover — seven kinds, within the same session — yes, byte for byte, and the assertions read git rather than trusting returns. For the journal as a record that outlives a fortnight — no, until C1 is fixed: the entry names trees and a commit that nothing keeps alive. That is a one-function fix in `finish` plus a pin for amend, and the gc-then-undo test pattern already exists in the file.

## For KNOWN_GAPS

- **Broken (after a window):** C1 — undo/redo of index-only ops fails once `gc` prunes the recorded after-tree (default 2 weeks); undo of an amend loses the pre-amend commit once its reflog entry expires (default 30 days). Expiry: fix before any key is wired (group 04), not a gap to carry.
- **Unverified:** `StashApply` execution (no test); redo of an undone pop/drop; a plan with several invocations that fails or is stopped after the first; parity for Amend/StashApply/StashPop/StashDrop (M2, M3).
- **Limitation, not broken:** redo of an undone conflicted pop restores the markers with a clean index (M5a).
- **Close with numbers:** the 50k-refs debug guard row — 138–140 ms isolated debug against 200 ms; guard, not regression; make it record-not-assert in debug per ADR-0021 or leave it and expect contention flakes.
- The 2.38 row, `stash@{0}` re-listing, multi-line messages, the no-newline hunk: already filed, unchanged by this group.
