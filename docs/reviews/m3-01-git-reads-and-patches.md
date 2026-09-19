# Review: m3-working-tree-undo-and-stash/01 — git reads and patches

Reviewer: Fable (`reviewer`), 2026-09-19. Range `fbbbb4e..93de3c2` (the
conductor's `636b3a7` skipped). Read: spec, design note (Interfaces, changes
1/8/9), PROMPT.md Technology "Writing to the repo", M2 review R2 Major 1,
KNOWN_GAPS row m2/01, `crates/graft-git/MAP.md`, then the diff crate by crate.
Opened no source file whole. Every claim below marked *verified* was run
against git 2.54.0 in a throwaway repository under `target/review/` (scripts
`probe.sh`, `probe2.sh` there, untracked).

**Counts: 1 Critical, 5 Major, 9 Minor.**

## Gates

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo test` (fast tier) | all green (workspace: 467+170+150+186+97+49+… tests, 0 failed, 9 ignored); **83.1 s wall**, 65 s user + 34 s sys; peak RSS **713 MB** |
| `cargo test --features heavy-tests -- --test-threads=2` | **1 failure** (debug-only timing guard, not this group; see "Heavy tier"); 157.5 s wall, peak RSS 733 MB |
| `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture` | **13 passed, 0 failed**, 90.7 s wall, peak RSS 471 MB; numbers under "Heavy tier" |
| fuzz target `parse_porcelain_v2` | builds (clippy heavy + builder's note); not run — `cargo-fuzz` needs nightly, out of the 7.1 laptop budget for this group review |
| `Command::new("git")` outside `run/` in `crates/graft-git/src/` | none in non-test code; one **new** one in `state/tests.rs:31` (Minor 7); `fixture.rs:105` and `repo/tests.rs:231` pre-date the group |

Five slowest fast-tier tests (single-threaded run, wall ms): `diff_fixture::parse_unified_matches_numstat_for_every_shape` 13 256; `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 11 706; `corpus::two_builds_produce_the_same_object_ids` 5 200; `corpus::a_second_build_in_the_same_place_is_a_no_op` 1 892; `raster_pixels::reports_per_strip_cost` 1 714. This group's slowest: `diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shapes_root_commit` 1 446 ms. `target/`: **12 GB**.

## Heavy tier

- Debug heavy run: 35 passed, **1 failed**: `many_refs::reading_tips_from_50k_real_refs_is_within_budget`
  (`crates/graft/tests/it/many_refs.rs:125`, a 200 ms *debug* guard on
  `commit_tips`). Not this group's code — nothing in `fbbbb4e..HEAD` touches
  refs or tips — and KNOWN_GAPS row "50k refs slow the graph's first paint in
  debug builds" already records the user's decision that only release timings
  gate (ADR-0021). Re-run alone, single-threaded, debug: **274.7 ms vs the 200 ms guard, deterministic**, not load — `commit_tips took 274.673667ms, over the 200ms budget` (25 543 tips). Belongs in KNOWN_GAPS as
  **unverified**: a debug-only timing assertion that fails on this laptop even alone
  is a red gate, not a product gap; the test should either move its
  assertion behind `cfg!(not(debug_assertions))` like the frame budgets, or the
  row should say the heavy tier is expected red in debug.
- Release perf run: 13 passed. `first_paint_us` 23 925 / 25 089 / 27 907 / 29 051 (100k, with/without commit-graph, plain/realistic) and 71 632 with 50k refs — budget 150 ms. Frame p99: Unicode 100k 5 405 us, Pixel 100k 5 712 us (max 9 891), 50k refs detail pane 5 077 us, map 4 733 us — budget 16 ms. Peak RSS in-test: 31.0 / 35.3 / 95.2 / 101.7 MB, 100.0 MB with 50k refs — budget 300 MB. `bottom_ms` 94–97 with a commit-graph, 923–1 036 without (provisional, overflow 100k).

## Critical

### C1. The porcelain-status oracle test runs on no shape at all

- **Where:** `crates/graft-git/src/status/tests.rs:404-421`
  (`read_matches_gits_own_counts_and_letters_on_every_shape`), guard at `:416`
  `if files.len() < 6 { continue; }`.
- **What:** the test scripts six dirty states over the fixture's tracked
  files and skips any shape with fewer than six. **No corpus shape has six.**
  Counted with `git ls-files` in `target/fixtures/76bc0e2e392220e9/*`:
  `feature-branches` 5, `evil-merge`/`octopus-merge`/`tags` 4, everything else
  0–3. The loop therefore `continue`s 21 times and asserts nothing; the test
  passes vacuously. The comment at `:412-415` ("it still gets covered as
  `Linear`/`FeatureBranches` and the like, which have plenty") is false.
- **Why it matters:** this is the spec's first acceptance criterion and the
  only test that runs `status::read` against a real dirty tree and checks the
  record letters git prints (rename `2 R.`, delete `.D`, both-sides `MM`).
  Everything downstream in M3 — the Changes pane, stage/unstage targets,
  `Touches` — reads this. The pure-parser unit tests use hand-typed records,
  so a mismatch between the parser's field layout and git's real `-z` output
  would ship undetected. (I verified the rename record shape by hand:
  `2 R. N... 100644 100644 100644 <h> <h> R100 h.txt\0f.txt\0` — the parser
  is right for that case, but the test does not prove it.)
- **Fix:** script the six states on a fixture the test *makes* (`Fixture`
  with six committed files), or lower the need to the states the shape can
  carry and assert the shape count that actually ran is `> 0`. A loop that
  can skip every iteration must assert it ran at least once.

## Major

### M1. `hunk_patch(Lines)` builds only the forward direction; `git apply -R` (unstage, discard) refuses it

- **Where:** `crates/graft-git/src/diff/patch.rs:1-12` (module doc: "what
  `ops`'s hunk staging (`git apply --cached`) and discard (`git apply -R`)
  both spawn against"), `:109-152` (`selected_lines`), `Selection::Lines`
  doc `:24-28`.
- **What:** an unselected `Added` line is dropped and an unselected `Removed`
  line becomes context. That is `git add -p`'s *forward* rule. Reverse-applying
  such a patch (`-R`: unstage from `Staged`, discard from `Unstaged`) needs the
  opposite — unselected `Added` kept as context, unselected `Removed` dropped —
  because the preimage `-R` matches is the patch's *new* side, and the file
  still holds the dropped lines. *Verified:* modified `f.txt`
  (`one/two/three` → `one/twoo/three/four`), select only `+four`, builder's
  patch, `git apply -R` → `error: patch failed: f.txt:1`, worktree untouched;
  same for `apply --cached -R` on a staged edit. Only `WholeHunk` works both
  ways.
- **Why it matters:** line-level unstage and discard are M3's promise
  ("you cannot lose work" needs the discard to work at all). `ops` (groups
  02/03) will call this with `-R` and get a refusal on every partial
  selection that leaves an `Added` line unselected. No data loss — git
  refuses rather than misapplies — but the feature is absent.
- **Fix:** give `hunk_patch` (or `Selection`) a direction — `Forward` /
  `Reverse` — and swap the two rules for reverse; or build the reverse patch
  by selecting on the reversed hunk. Add a test that reverse-applies a
  partial selection with `git apply -R` and `git apply --cached -R` and
  asserts the resulting file/index content, not just the exit code.

### M2. A `\ No newline at end of file` context line inside a `Lines` patch silently corrupts content

- **Where:** `patch.rs:135-151` (a `Removed` line with `no_newline_at_eof`
  becomes `Context` and keeps the marker), `:244-251` (marker written after
  any line kind).
- **What:** old file `one\ntwo` (no trailing newline), new `one\ntwo\nthree`
  (none). Git's hunk: ` one`, `-two` + marker, `+two`, `+three` + marker.
  Select only `+three`: the builder emits ` two` + marker, then `+three` +
  marker. *Verified:* `git apply --cached` **exits 0** and stages
  `one\ntwothree` — the two lines fused. Git accepts the patch because a
  context line with the marker is legal, then appends `three` to a line that
  has no newline.
- **Why it matters:** silent wrong content in the index on a stage; with M1
  fixed, the same shape would reach the worktree on a discard. Files without
  a trailing newline are common (generated files, some editors).
- **Fix:** when an unselected `Removed` line carrying the marker turns into
  context and any *later* selected line exists, git's own `add-patch.c`
  handles this by refusing to split there; the simplest honest rule is to
  return a new `PatchError::NoNewlineBoundary` when a marker line would
  become context that is not the hunk's last line, and let the UI say "select
  the whole hunk". Add the case as a test with real `git apply` and a content
  assertion.

### M3. The whole-hunk oracle covers one one-line hunk per shape

- **Where:** `crates/graft-git/src/diff/patch/tests.rs:36-101`
  (`check_shape_whole_hunks`), `:103-108`.
- **What:** "every hunk of every file in the root commit" is, for every
  shape, exactly one hunk (`git diff-tree -p --root` on each corpus root:
  `root_hunks=1` for all 21), and for 20 of 21 it is a one-line file whose
  header is `@@ -0,0 +1 @@`. So the byte-for-byte claim is tested on: no
  `,N` counts other than `0,0`, no removed lines, no context lines, no
  `\ No newline` marker, no hunk heading, no rename, no mode line. The
  `range()` bare-count form *is* exercised (good), nothing else is. The
  `Lines` test asserts git accepts the patch and `+four` is staged, but never
  asserts the recomputed header (`@@ -1,3 +1,4 @@`) the spec names.
- **Why it matters:** the spec's "reproduces git's own hunk byte-for-byte"
  is the property `ops` relies on; the test would pass with a builder that
  only handles additions.
- **Fix:** run the same oracle on the corpus's *tip* commits (`Commit
  { commit: HEAD }` per shape — they have edits, not just creations) and on
  a scripted fixture with a two-hunk file, a no-newline file and a rename;
  assert the `Lines` header text.

### M4. `status::read` ignores a capped output — a partial status reads as complete

- **Where:** `crates/graft-git/src/status.rs:366-379` (`read`), compare
  `diff/read.rs:230-233` and `:269-272` which check `output.truncated` and
  return `DiffError::TooLarge`.
- **What:** `run` caps stdout at 64 MiB and returns `Ok` with `truncated`
  set. `read` parses the bytes as-is. A cut at a record boundary yields a
  shorter list with no error; a cut mid-record yields `Malformed`, which
  names the wrong cause.
- **Why it matters:** CLAUDE.md "never swallow an error"; the diff module
  already treats the same condition as an error. In M3 the status list will
  drive stage-all / snapshot scope; a silently short list is the wrong kind
  of wrong.
- **Fix:** `if output.truncated { return Err(StatusError::TooLarge { bytes }) }`
  (or reuse `Malformed` with a message naming the cap), one line.

### M5. `stash::has_untracked_commit` swallows every error into `false`

- **Where:** `crates/graft-git/src/stash.rs:166-174`.
- **What:** `let Ok(oid) = … else { return false; }` and
  `find_commit(oid).is_ok_and(…)`. A stash commit gix cannot read (corrupt
  object, unreadable pack) reports "no untracked commit" and `paths` silently
  omits the untracked files; the entry still lists as healthy.
- **Why it matters:** `StashEntry::paths` is what the design note says the
  browser shows and what `Touches`/snapshot scope will read for
  `StashPop`/`StashApply`. A stash whose untracked half is invisible is a
  stash the user can lose on drop without the UI naming the files. Also the
  literal rule: no `Err(_)`-to-default.
- **Fix:** return `Result<bool, StashError>` and add a `StashError::Object`
  (or reuse `Reflog`) arm; `?` it in `stash_paths`.

## Minor

### m1. `staged_against` folds every head-read error into `NoHead`
`diff/read.rs:308-312`: `repo.head().map_err(|_| DiffError::NoHead)`. An I/O
error reading `HEAD` reads as "no head". Pre-existing pattern in
`ensure_head`; carry the source (`DiffError::Head(…)`) or at least keep the
message.

### m2. `WorkingTreeStatus::counts()` counts an unmerged path nowhere
`status.rs:176-182`: a conflict-only tree has `counts().is_clean() == true`
while `is_clean()` on the same struct says `false`. Two answers to one
question; the uncommitted-changes row will read "clean" mid-merge. Either
count unmerged under `unstaged` (git's short status shows them there) or add
an `unmerged` field to `WorkingTreeCounts`.

### m3. `hunk_patch` for an `Added` file cannot be applied forward
`patch.rs:190-196`: `--- /dev/null` after a `diff --git` line with no
`new file mode <mode>` line. *Verified:* `git apply --cached` →
`error: dev/null: does not exist in index`. Reverse (`-R`, unstage) works,
and deletion (`+++ /dev/null` without `deleted file mode`) works both ways.
Not reachable in M3 (untracked files have no hunks, ADR-0034), reachable in
M7 (`add -N`). Emit `new file mode {new_mode:o}` / `deleted file mode
{old_mode:o}` when the mode is known; note in KNOWN_GAPS as **unverified
until M7** if left.

### m4. `state::in_progress` reports `git am` as a rebase
`state.rs:79-95`: `rebase-apply/` is also `git am`'s directory
(*verified:* a failing `git am` leaves `rebase-apply/applying`). The advice
`git rebase --abort` is wrong there (`git am --abort`). Check
`rebase-apply/applying` → a new `InProgress::Am`, or say "am/rebase" in the
name.

### m5. `RepoHandle::git_path` takes no `Stop` and stringifies the path lossily
`repo/mod.rs:189-206`: `.run(&())` — a wedged git hangs the caller with no
way out (`version::system_git` built a `Deadline` for the same reason);
`relative.to_string_lossy()` — paths are bytes everywhere else in this crate.
Callers today pass ASCII constants; take `&dyn Stop` anyway, and a
`GitArg::Word` from bytes if the arg type allows.

### m6. `stash::list` caps at 50 without telling the caller
`stash.rs:25`, `:100-102`. The browser cannot say "50 of 73". Return the
total reflog length alongside, or a `truncated: bool`.

### m7. New `Command::new("git")` in `state/tests.rs:31-49`
Acceptance says none outside `run/`; this one is test code, with a 20-line
copy of `graft_fixtures::git`'s environment. Reason: the merge is expected to
exit non-zero and `Fixture::git` panics on that. Add a status-tolerant
`graft_fixtures::git::run_expecting_failure` (or return the `Output`) and
delete the copy. Its stderr also leaks into the test log ("Automatic merge
failed; fix conflicts…") — pipe it.

### m8. Test doc claims an oracle it does not use
`diff/read/tests.rs:568-571`: "lists exactly the paths a real `git diff
--cached --numstat` oracle does" — the test asserts the hand-written set
`{a.txt}`. Fix the comment or run the oracle. (`file_diff_staged_and_unstaged_match_porcelain_diff_oracles`
does use a real oracle — good.)

### m9. `commit_graph::write` switched from `--git-dir <common>` to `-C <common>`
`commit_graph.rs:130-134`, MAP.md updated to match. Works (fast tests green)
because git discovers the directory it is run inside as the git dir; but the
spec and the KNOWN_GAPS row said `--git-dir`, and `GitInvocation`'s doc still
says `-C <workdir or git_dir>`. Either is fine; say why in the doc comment
(one line) so the next reader does not "fix" it back.

## Verified, no finding

- `.git/index` bytes unchanged across `status::read`, `list_files(Staged|Unstaged)`,
  `file_diff(Staged|Unstaged)` on a stat-dirty tree — one test, five spawns,
  byte-compared (`diff/read/tests.rs:702-778`). Good.
- Every production git spawn in `graft-git` goes through `run`;
  `system_git` gets a poll-based `Deadline` (no timer thread), `write` maps
  `RunError` without dropping a variant.
- `parse_porcelain_v2` rejects garbage without panicking (unit test + fuzz
  target); the `2` record's origin-path pairing matches git's real `-z`
  output (*verified*).
- `stash::list` newest-first, messages and commit ids checked against
  `git stash list --format=%H`; the `--root <stash>^3` requirement for the
  untracked commit is real and handled.
- `git_path`: relative in the main worktree, absolute in a linked one
  (*verified*, git 2.54) — the code handles both.
- `GitInvocation::env` applied after the fixed vars; later key wins.
- No `unwrap()`/`expect()` in non-test code in the diff; no terminal output in
  this group.

## For KNOWN_GAPS

- **Broken:** M1 (`Lines` + `-R`), M2 (no-newline fusion), C1 (oracle test
  vacuous) — expiry: before group 02 (`ops-model`) calls `hunk_patch`.
- **Unverified:** m3 (Added-file forward patch; M7), the heavy tier's debug
  `reading_tips` guard (see "Heavy tier").
