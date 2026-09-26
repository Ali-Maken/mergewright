# Review — m7-power-tools 06b absorb-staged-and-replay

Reviewer, 2026-09-26. Merge `e12b2c2b` (one commit, `e871f245`), read as `git diff e12b2c2b^1 e12b2c2b`. Every check
ran in a detached worktree at `e12b2c2b` in the session scratchpad, with `CARGO_TARGET_DIR=$TMPDIR/graft-target-m7-rev06b`.
The worktree has since been removed. Probes were a throwaway test module (`probe_rev06b.rs`, not committed) plus two shell
scripts. Every behaviour quoted below is what those runs printed (git 2.54.0).

**Counts: Critical 2 · Major 3 · Minor 6 · Questions 2.**

## Gates (one at a time, worktree at `e12b2c2b`)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` (run 1) | **3912/3912 pass**, 1 skipped, **224.0 s** (over 7.1's 3 min), load 5–7. Max RSS 1.05 GB |
| `cargo nextest run --workspace --status-level pass` (run 2) | 3912/3912 pass, 369.9 s, load 12–14 from other builders |
| `cargo test --doc --workspace` | pass (3 doctests) |
| `cargo nextest run --workspace -E 'test(/set_aside\|put_back\|stash/)'` | 238/238, as the ticket says |
| Heavy tier (`cargo test --features heavy-tests -- --test-threads=2`) | graft lib 1349 pass. **graft `it`: 535 pass, 1 fail**, 385.6 s (433 s wall). Max RSS 1.05 GB. The failure is `heavy.rs:217`, known (KNOWN_GAPS M7/heavy217). cargo stopped after that binary |
| Release perf (`frames:: heavy::budgets_hold`, alone) | **14/14 pass**, load 5–8, 94.6 s |
| Fuzz, benchmarks | Not run. No fuzz target covers the code changed here |

- **5 slowest (run 2):** `m5_walk::every_scenario::merge_resolved_with_the_keys` 18.4 s,
  `diff_fixture::parse_unified_matches_numstat_for_every_shape` 17.2 s, `conflict_ops::whole_file_shapes_resolve_like_git`
  16.5 s, `undo_cancel::cancel_at_every_step_boundary_is_clean` 14.0 s,
  `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 13.3 s. The `absorb_staged` tests
  take under 1.3 s each.
- **`target/`:** the main repo's is 16 GB. The review's own target is 7.6 GB (debug + release).
- **Release perf numbers:**
  - First paint: 24.4 ms (commit-graph), 120.6 ms (no commit-graph), 99.1 / 117.6 ms (realistic, with and without),
    77.5 ms (50k refs). Budget 150 ms.
  - Unicode p99: 0.76 ms (100k), 2.2 ms closed / 1.5 ms diff / 1.6 ms detail / 1.0 ms map (50k refs).
  - Pixel p99: 3.6 ms (50k refs), 11.7 ms (100k). 2 of 420 keys went over 16 ms (16.8 and 21.6 ms). The gate is the p99.

## Ticks spot-checked: 5 of 5 held

- **C1:** `an_index_only_stay_survives_absorb`, `an_index_only_mode_change_survives_absorb` and `a_stay_stays_staged`
  exist in `tests/it/absorb_staged.rs` and are green.
- **M2, M3, M6:** all four named tests exist and are green.
- **Undo:** `undo_after_absorb_restores_index_and_commits` is at `tests/it/absorb.rs:259`, and the five new undo tests
  are in `absorb_staged.rs`. All are green.
- **Set-aside filter:** 238/238, the same count the ticket gives.
- **Gates:** 3912/3912 with 1 skipped, the same count the ticket gives.
- **"Red first" cannot be verified from history:** the branch is one commit. It is indirectly supported for C1. Probe
  `q11` runs the same absorb with `index_base` cleared (the old put-back), and the index-only line is lost, so the C1
  tests would be red without the fix.

## The builder's two claims about M6's base, checked by probe

Shell probe (`base.sh`). Placed `B3` in `b.txt`, with the working tree edited on to `B3 and more`. A stay `O3` in
`old.txt`, both staged and in the working tree. `HEAD` was moved to old `HEAD` + placed, as the rewrite leaves it.

| Put-back | Result |
|---|---|
| ADR-0045 plain (`stash apply S`, base = old `HEAD`) | `UU b.txt` with markers: the M6 bug |
| Review 06's proposal (merge base = old index) | exit 0, `M b.txt` only. **`O3` is gone from the working tree and from the index** (`old.txt` line 3 = `o3`) |
| 06b (`S` rebuilt on `HEAD`+placed, `--index`) | exit 0, ` M b.txt` + `M  old.txt`, **index tree == old index tree**, working tree exact |

- **Claim 1** (the builder used a different base than the review proposed): **true.** Its base is old `HEAD` + every
  placed hunk (`rewrite/plan.rs:270-277`).
- **Claim 2** (the review's base would have deleted stays from the working tree): **true.**
  - With the old index as merge base, the stay is in base and theirs but not in ours, so the merge takes ours' removal.
  - Review 06's companion step (re-apply the stays with `apply --cached`) would have restaged `O3` in the index, but the
    working tree would still be reverted.
  - The review's suggestion was wrong. The builder was right to say so in NOTES rather than deviate silently.
- **Why 06b cannot clash:** `from_index_base` (`apply/set_aside.rs:564-586`) runs only when `HEAD^{tree}` equals the
  base's tree (`:570-573`). The merge's "ours" is then the merge base, so git takes `S`'s working tree whole. The index
  gets `base..old-index`, which is exactly the stays.

## What the absorb promise holds now (probes)

| Case | Result |
|---|---|
| Index-only stay, went through | kept staged (tests) |
| Index-only mode, went through | kept (test) |
| Worktree-only mode change (`p12`) | ` M old.txt` before and after |
| Stay and placed hunk in **one file**, index-only stay, tree edited on (`p5`) | index == before, working tree exact |
| Rename **with** an edit, tree edited on (`p6`) | `RM b.txt -> moved.txt`, index == before |
| One target emptied + another hunk placed (`p10`) | `drop f4` + `fixup! f1`, clean, undo exact |
| Stopped (edit row), `c` (`p1`) | index-only stay kept, stash dropped, undo exact |
| Stopped on a **real conflict**, resolved as planned, `c` (`q3`) | index-only stay kept, stash dropped, undo exact |
| Stopped, resolved differently, `c` (`q4`) | falls back to the plain put-back: `UU b.txt` with markers, `S` kept, undo exact. No loss |
| Stopped, **`a` aborts** (`p2`/`q2`) | **index-only line lost** (Critical 2) |
| Staged `git rm` of a file one commit made (`q8`) | **history gets a file named `dev/null`** (Critical 1) |

## Critical

### C1. A staged whole-file deletion that absorb places writes a file named `dev/null` into history

- **Where:**
  - `crates/graft-git/src/diff/patch.rs:282-318`: `write_diff_git_header` and `write_path_lines` write
    `diff --git a/b.txt b/b.txt`, `--- a/b.txt`, `+++ /dev/null`. There is no `deleted file mode` line.
  - Absorb reaches it through `crates/graft-ops/src/absorb.rs:611` (`with_context` → `hunk_patch`), then `tree_with`
    and `commit_pieces`.
- **What:** under a `diff --git` header with no `deleted file mode` line, `git apply` reads `+++ /dev/null` as a path.
  - **Shell:** that exact patch, run with `git apply --cached` on a temporary index, exits 0. `b.txt` is gone and an
    empty `dev/null` is in the index.
  - **Probe `q8`:** `f2` made all of `b.txt`, and he stages `git rm b.txt`. Absorb returns `Ok Committed`.
    - The todo is `pick f2, fixup! f2, pick f3`. It should have been M3's drop: `empties` compares trees, and the
      piece's tree has `dev/null`.
    - Afterwards `ls-tree HEAD` = `a.txt dev/null old.txt`, and `f2` is now `dev/null | 0`. Status is `D  dev/null`.
  - `u` put everything back (`q8`: state equal).
- **Why it matters:**
  - Absorb reports success and rewrites his history with a junk file. The staged deletion he asked for turns into
    "`f2` adds an empty `dev/null`".
  - It is also the most common case M3 exists for: "take out the file I added in that commit". 06b's drop never fires
    for it.
- **Origin:** this predates 06b. The patch writer and `with_context` did not change in this merge. Review 06 missed it.
  Its M2 probe placed a rename's deletion side through the same path, so it very likely hit this unseen. **Not a false
  tick:** M3's tick names the `DEBUG`-line case, which does hold.
- **Fix (cause):** have `hunk_patch` write git's extended header lines, `deleted file mode <old>` for `Deleted` and
  `new file mode <new>` for `Added`.
  - Test it in graft-git by **applying** the patch. Today `patch/tests.rs:201-202` only asserts the text.
  - Add the absorb test `a_staged_file_deletion_drops_the_commit_that_made_it`.
- **Knock-on:** the app's hunk stage, unstage and discard use the same writer (`crates/graft/src/update/operate.rs:466`).
  See Question 1.

### C2. Index-only content is still lost by every put-back except absorb's went-through one

- **Where:**
  - `crates/graft-ops/src/apply/set_aside.rs:515-519` (the plain `stash apply S`, no `--index`).
  - For absorb, `apply/resume.rs:414` drops `index_base` when `aborting`, and `apply.rs:358-361` drops it on failure.
- **Absorb aborted (06b's own scope):**
  - Probe `p2`/`q2`: index-only `O3` in `old.txt`, `B3` placed. The absorb stopped, then `Resume::Abort` →
    `Aborted`.
  - Afterwards the index `old.txt` = `o3`, the working tree = `o3`, and `stash list` is empty. `B3` also came back
    unstaged (` M b.txt`).
  - `u` restored all of it (`q2`: undo `Committed`, state equal).
  - The ticket's C1 was "a hunk that stays and lives only in the index is lost". After an abort, every hunk stays, so
    the fix is incomplete.
- **Absorb stopping is real, not theoretical.** Probe `q3`'s fixture: `g1` changes the line next to the placed one, and
  `g2` changes it back.
  - The commutation check passes, because `HEAD`'s context equals `f2`'s.
  - The replay conflicts at `g1`, then again at `g2`.
- **`r` / `R` / checkout (KNOWN_GAPS M7/rR-index, now with its line):**
  - Probe `q11` ran the same absorb with `index_base` cleared. That is the arm `R` takes (`rewrite/plan.rs:150`,
    `index_base: None`), and `r` passes `None` too (`apply.rs:358-361`).
  - Result: `Ok Committed`, status clean, stash empty, `O3` gone.
  - **Not run through `Operation::Rebase` itself.** It runs the same `put_back` arm.
  - `a` does **not** lose index-only lines: `r2` shows the whole index goes into the target. `a` **does** have M6's
    clash: `r1` gives `UU b.txt`, with `S` kept, so nothing is lost.
- **Why it matters:** M3's "you cannot lose work". Absorb and `R` report success, or "aborted", and the only copy is the
  undo backup. He finds out only if he notices.
- **Fix (cause), for ADR-0045:**
  - When `HEAD^{tree}` equals `S^1^{tree}` (an abort, or git refusing before it moved), run `stash apply --index S`. It
    is exact and cannot clash, by the same argument as 06b: ours is the merge base.
  - For a rewrite or switch that moved `HEAD`, decide in the ADR between two options, and keep ticket 30
    (M7/rmcached) in the same decision:
    - (a) `--index` first, falling back to the plain put-back and keeping `S` listed when index-only content exists;
    - (b) refusing before `y` with words.
  - Tests: `abort_after_absorb_stops_keeps_index_only_lines`, and an `r` row in the `set_aside` round trips with an
    index-only line.

## Major

### M1. The resume path is verified by probe but not tested, and KNOWN_GAPS M7/06b-resume's premise is wrong

- **Where:** NOTES "Unverified … no fixture makes absorb stop"; KNOWN_GAPS M7/06b-resume. The code is
  `apply/resume.rs:277` and `:414`.
- **What:** two cheap ways make absorb stop:
  - an `Edit` row set in the planned params, as `tests/it/rewrite_run.rs:366` (`edit_a1`) already does;
  - `q3`'s change-and-change-back fixture, a real conflict.
- **Probes:**
  - `p1` and `q3`: `c` finishes with the index-only stay still staged, `S` dropped, and undo exact.
  - `q4` (resolved differently): falls back to the plain put-back, with markers and `S` kept. That is what NOTES
    describe.
  - So the resume behaves as built, but only a throwaway probe shows it.
- **Fix:** add tests for continue as planned, continue resolved elsewhere, and abort (C2), each with an undo row. Then
  close the gap row.

### M2. `apply/set_aside.rs` is 690 lines and grew by 125; `absorb.rs` is 622 source lines and grew by 51

- **Where:**
  - `apply/set_aside.rs:548-622` (`from_index_base` and its doc from `:548`, `resolve` `:588`, `commit_tree` `:602`).
  - `absorb.rs:224-290` (`empties` `:229`, `tree_of` `:241`, `tree_with` `:254`).
  - PROMPT.md 6.2 sets about 400 lines. Review 06's M7 split of `absorb.rs` is still open.
- **Duplication:** `set_aside.rs::resolve` and `absorb.rs::tree_of` are the same `rev-parse --verify` helper, twice in
  one crate. `set_aside.rs::commit_tree` repeats the `commit-tree` call from `absorb.rs::commit_pieces`.
- **Fix:**
  - Move the rebuilt-stash unit to `apply/set_aside/rebuild.rs`.
  - Put the object-plumbing helpers (`rev-parse --verify`, `commit-tree`) in one crate-private module that both use.
  - Do 06's split of `absorb.rs`.

### M3. ADR-0045 now contradicts the code in two places

- **Where:** `docs/adr/0045-…:73` (§4 step 6: "`git stash apply <S>` (no `--index`)") and `:131` (Consequences: "loses
  the staged/unstaged split on the clean path too").
- **Why it matters:**
  - `rewrite.rs`'s `index_base` doc and the MAP say "supersedes ADR-0045", but the ADR does not know that yet.
  - The conductor owns the amendment. What it must state is below.

## What the ADR-0045 amendment must state

1. **Scope:** §4 step 6 and Consequences :131 no longer hold for absorb that **went through**. That is
   `Operation::Rewrite` with `index_base: Some`, and `HEAD^{tree}` equal to the base's tree at close.
2. **The mechanism, as built:**
   - `index_base` = old `HEAD` + every placed hunk, a commit with no ref. It is pinned by the entry (`record.rs:347-354`)
     and saved in `plan.json` (`SavedPlan::index_base`, `#[serde(default)]`: a plan saved before 06b reads as `None`).
   - At step 6, `S` is rebuilt (objects only): working tree `S^{tree}`, parents `base` and a commit of `S^2^{tree}` on
     `base`. Then `stash apply --index` of the rebuilt stash. `S` stays the listed copy and is dropped by id.
3. **The guarantee:** index == the old index, byte for byte. The working tree is exact, including an edit made on past a
   placed hunk. It cannot clash, because ours is the merge base.
4. **The rejected alternative, and why:** merging with the old index as base removes every stay from the working tree.
   Cite this review's table.
5. **The fallback:** failed, aborted, or resumed onto a different tree → the plain put-back, **with its losses**. Name
   index-only content (C2), not only "the split". Once C2 is decided, replace this with the chosen rule.
6. **The preview:** `SetAside.staged` is 0 for absorb (`operation.rs:1196-1199`). That is true only on the went-through
   path (Minor 1).
7. **§7 proof rows** to add: an index-only line, an index-only mode, a rename, edited on, stopped then `c`, stopped then
   `a`.

## Minor

1. **`SetAside.staged` is always 0 for absorb** (`operation.rs:1196-1199`), but after an abort or a failure the plain
   put-back loses the split. `q2`: `B3` came back ` M`. The preview's count is a forecast of the success path only. It
   becomes true for abort once C2's fix lands. Say so in the doc until then.
2. **The `Copied` arm is dead** (`absorb.rs:151`). `-M` (`:309`) never reports copies, so `StaysWhy::Renamed`'s doc and
   the MAP line "rename or copy" promise a case git never produces here. Drop the arm, or pass `-C` and test it.
3. **A deletion plus a similar new file is read as a rename, and the refusal words are untrue** (probe `p7`).
   - He runs `git rm b.txt` and adds `c.txt` with 4 of its 5 lines. Both sides stay as `Renamed`, and absorb refuses
     with "none of your staged lines belong to a commit on your branch · c commits them".
   - `b.txt`'s lines are `f2`'s. Word the case where everything stays by reason. This is related to review 06 Minor 2.
4. **`a_fixup_that_empties_its_target_drops_it` never checks the claim at `rewrite/plan.rs:254-257`** that the pick
   below "git keeps as it is". Assert that `f3`'s id is unchanged.
5. **`a_staged_rename_stays_whole` covers a pure rename only.** A rename with an edit and the tree edited on works
   (probe `p6`: `RM`, index equal) but is untested.
6. **Every absorb plan now writes one more commit** (`rewrite/plan.rs:270-277`): a read-tree, the applies, write-tree,
   commit-tree. That is true even when nothing stays, where the base equals the last fixup's tree. It is unmeasured on
   the monorepo (review 06 Minor 8). Measure it with the keys ticket's off-thread work.

## Questions

1. **The app's hunk stage, unstage and discard use the same `hunk_patch` header** (`crates/graft/src/update/operate.rs:466`).
   - For a **deleted** file's hunk, does staging or discarding write `dev/null` as C1 does?
   - Shell, for comparison: the **added**-file shape fails cleanly in git (`error: dev/null: does not exist in index`),
     but the deleted-file shape corrupts.
   - Not probed through the app. This is for graft-git and the app's owner.
2. **The only commit on the branch, emptied:** `plan` returns `Ok([Drop f1])` (probe `p9`), and only the run refuses:
   "every commit dropped · the branch would be its base again" (`q9`, staging kept). Does the preview run `validate`
   before it draws, or does he see a plan that `y` then refuses?

## Pipeline, architecture, safety

- **Pipeline:** this is a fix ticket. The note's Need, Prior art and Journeys were checked in review 06. No journey is
  built here: absorb's keys are the keys ticket, so there are no step counts to check.
- **Crate set:** graft-ops only, as the ticket says. No new dependency. Ops decides and nothing crosses into the app.
- **`unwrap`/`expect` outside tests:** none added. The four new ones are in `rewrite/saved.rs`'s test module.
- **Swallowed errors:** none. `from_index_base` failing returns `Kept::Git { back: false }`, so `S` stays listed and the
  words say where.
- **Git on the UI thread:** not touched here. Absorb's planning is still synchronous (review 06 Minor 8).
- **Escapes:** no new drawing.
