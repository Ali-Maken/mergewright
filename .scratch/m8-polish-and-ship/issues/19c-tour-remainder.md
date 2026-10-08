# G-tour remainder — task 7 from git's own answer, a fixed clock, the owed walks and captures

Status: ready (conductor 2026-10-08) — was draft (planner spec-only pass, 2026-10-08; supersedes ticket 19's §A3 task-7 rule and its open §A9 boxes)
Build with: three parts, **sequential** (b and c share `crates/graft/tests/it/main.rs` and need a's dates):
- **19c-a — Sonnet.** Every signature fixed below; the one judgement (does git call the clash merge a conflict) is a
  git-backed test, not a reading.
- **19c-b — Opus.** Driving the resolver and stash through the real job pool; a task that won't tick needs a cause,
  not a tweak.
- **19c-c — Opus.** Pty harness + captures; 19b compacted twice on this ground — keep it to the walk and the shots.
No Fable: no new structure; the only interface change is two parameters.
Needs: 19a, 19b (merged). 19c-b needs 19c-a. 19c-c needs 19c-a and 19c-b.

## Read first
- This file. Ticket 19 (`19-tour-practice-repo.md`): §A3, §A7, §A8, §A9, NOTES 19a and 19b — not the rest.
- `crates/graft-ops/MAP.md` `## tour`, `## journal` (lines 106-124), `## conflict` line 81, `## snapshot` lines 157-158.
- `crates/graft-git/MAP.md` `forecast_merge` (line 200), `GitInvocation::env` (line 262), `run_raw` (line 277).
- `crates/graft-core/MAP.md` `## journal` (`JournalEntry` fields, line 59).
- `crates/graft-fixtures/MAP.md` `capture` and `capture::machine` (lines 47-48), `git` (line 32), `FIXED_ROOT_VAR` (23).
- `crates/graft/MAP.md` `Clock` (line 89), `tests/it/quit_waits/pty_session.rs` (line 30), `drive`/places, the 18c clone
  walk entry. 19c-c: `docs/design-notes/m8-polish-and-ship.md` New UI "Tour card" (lines 474-476), "Where he lands"
  tour rows (190-192).

## R1. Task 7 — ruling: **a Committed `Merge` whose result git could not have made alone**
Evidence the old rule can't fire (19b verified `graft-core/src/journal.rs:89-92` by code and run; the MAPs agree):
graft-ops MAP `## journal` line 116 — a conflicted operation is **one** entry, `Pending` from the stop to the finish;
`## conflict` line 81 — the resolver's job side writes "no journal line"; `## snapshot` line 158 — a folded resolver
step is "No journal line, no entry". So a `ResolveConflict` entry never follows an in-app merge.

**Rule.** Task 7 ticks for a `Merge` entry, `EntryStatus::Committed`, whose result commit has two parents that
`graft_git::history_reads::forecast_merge(p1, p2)` answers `Forecast::Conflicts { .. }`. A committed merge git
couldn't do alone is a conflict someone resolved and finished — a fact about the result, not about how the
resolver journals its steps. Task 6 unchanged (any Committed `Merge`, so the clash merge ticks 6 too).
- Result commit: `head_after` = `HeadRef::Branch(name)` → `refs_after[name]`. Parents: an existing graft-git read
  that returns a commit's parents if there is one (name it in NOTES), else `rev-list --parents -n 1 <oid>` through the
  invocation `tour` already uses. One parent (fast-forward), detached/unborn `head_after`, or the ref absent from
  `refs_after` → not task 7; comment why that's safe (no recorded two-parent result = no proof of a resolution;
  a withheld tick, not a dropped error). A failed git read → `Err(TourError::Git { words })`, never a tick.
- `Forecast::NotCheckable(why)` → `Err(TourError::Git { words: why })`. Graft's floor is git 2.38
  (`docs/design/03-graph-layout-algorithm.md:74`), where the plain form (no `--merge-base`) is checkable, so this is
  an unrecognised answer, not an old git.
- `forecast_merge` **writes** the merged tree into the object store (graft-git MAP line 200). Here that is the practice
  repo's only — `tour_progress` is only ever called with `PracticeRepo::work()`. Doc it on `tour_progress`.

**Rejected.** (a) graft-core writes a `ResolveConflict` entry mid-merge: changes "one entry across the stop" (M5 hard
choice 2), which `in_force`/`next_undo` rely on (graft-ops MAP line 116) — a journal-semantics change for a tutorial
tick. (b) The entry's backup chain holding an `IndexState::Unmerged` link (MAP `## snapshot` 157-161): rests on three
internals — whether a block take chains, whether the after-seal re-parents the ref off the first-parent walk, what the
pre-op link reads as. (c) Cut task 7 and renumber: conflicts are the M6 promise and his real pain; a 7-task tour
would drop the one task that teaches it. Step counts unchanged: J12 3 / 1, J13 1; the card still says `of 8`.

**Builder confirms (19c-a, by test, not reading):** `forecast_merge`'s exact parameters (MAP shows the git, not the
Rust); the built repo's `clash` forecasts `Conflicts` (`menu.txt`) and `feature` `Clean` against `main`.
**19c-b confirms on the app path:** a finished in-app clash merge's entry is one `Merge`, `head_after` =
`Branch(refs/heads/main)`, `refs_after` holds the two-parent result. If any of these is false, stop and report.

## R2. Clock — ruling: **a parameter in graft-ops, an env var read once in graft**
What fixtures do: `graft_fixtures::git::run_at` stamps a caller-picked date (`GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE`),
because "a wall clock changes that id every run" (graft-fixtures MAP line 32). graft-ops can't use graft-fixtures
(dev-only), so it does the same thing with `GitInvocation::env` (graft-git MAP line 262).
- Every built commit gets its own minute, strictly increasing in build order — this also ends same-second row order
  in the shipped app, not only in tests.
- `PracticeAt::Now` = the app's: dates end a minute before `now_unix_secs`, which the caller takes from the session
  `Clock::now_seconds()` (graft MAP line 89: the one wall read for anything a capture shows). Folder name as today.
- `PracticeAt::Fixed` = reproducible runs: dates from `first_unix_secs`, **folder named
  `tributary-practice-<first_unix_secs>`** (no pid, no ms). Why the name too: the practice header draws the path
  (`◇ practice · <tmp>`, design note line 474), so a pid/ms name differs every run. `create_dir` still refuses an
  existing folder → `TempDir`, reported, no retry.
- `GRAFT_PRACTICE_EPOCH` (whole seconds since 1970) selects `Fixed`. Read in graft at the tour's start, not at launch.
  Unset → `Now`. Unparsable → the build-failure path: stays in his repo, status
  `tour: GRAFT_PRACTICE_EPOCH must be whole seconds since 1970, got <v>` (sanitised). G-rename (27) renames it with
  every `GRAFT_*` read — its grep test finds it.
- Rejected: always-fixed dates (the shipped tour would show "2y ago" commits); `SOURCE_DATE_EPOCH` (a build-tool
  convention, not a runtime one).

## R3. Signatures
```rust
// graft-ops::tour (19c-a)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeAt { Now { now_unix_secs: i64 }, Fixed { first_unix_secs: i64 } }
pub fn build_practice_repo(under: &Path, at: PracticeAt) -> Result<PracticeRepo, TourError>;
pub fn tasks_done(entries: &[JournalEntry], conflicted: &BTreeSet<EntryId>) -> [bool; 8]; // pure; 7 = Committed Merge ∈ conflicted
pub fn tour_progress(work: &Path) -> Result<[bool; 8], TourError>; // unchanged signature; builds `conflicted` (R1)
// graft (19c-a), beside the one `build_practice_repo` call in `drive`
pub(crate) const PRACTICE_EPOCH_VAR: &str = "GRAFT_PRACTICE_EPOCH";
pub(crate) fn practice_at(var: Option<&OsStr>, now_unix_secs: i64) -> Result<PracticeAt, String>; // pure
```
No new `TourError` variant. `tour_progress` stays stop-less (≤ 3 merges × 2 spawns in a tour); if `forecast_merge`
demands a `Stop`, pass the crate's existing never-stop and name it.

## 19c-a — graft-ops dates + task 7 (Sonnet, ~30k)
**Crate set:** `crates/graft-ops`; in `crates/graft` only the `build_practice_repo(` call sites (grep; `drive` in
`event_loop/places.rs` per the scout, plus any test that builds one) and `practice_at` beside the drive call.
**In:** R1, R2, R3. Rewrite 19a's `resolve_conflict_needs_a_resolution_inside_a_committed_merge` for R1. **Out:** any
app walk; captures.
**Acceptance** (graft-ops tests in `crates/graft-ops/src/tour/tests.rs`; graft unit tests beside `practice_at`):
- [ ] Same `Fixed` instant, two `under`s → identical oids for `main`, `feature`, `clash`, `origin/main`:
      `a_fixed_instant_builds_the_same_commits`.
- [ ] Commit dates strictly increasing, 60 s apart, in build order; `Now` ends one minute before `now`:
      `every_built_commit_has_its_own_minute`, `now_dates_end_a_minute_before_now`.
- [ ] `Fixed` names the folder `tributary-practice-<secs>`; a second build at the same instant under the same `under`
      is `TempDir`, nothing removed: `a_fixed_instant_names_the_folder_and_never_reuses_one`.
- [ ] Pure: 7 ticks only for a Committed Merge in `conflicted`; Pending/Aborted in `conflicted` never; a Committed
      Merge outside it ticks 6 only; a `ResolveConflict` entry alone ticks nothing:
      `resolve_ticks_on_a_committed_merge_git_could_not_make`, `a_conflicted_merge_not_committed_never_ticks_7`,
      `a_clean_merge_ticks_6_only`, `a_resolve_conflict_entry_alone_ticks_nothing`.
- [ ] Real repo: in a built practice repo, `git merge feature` and `git merge clash` (resolved, committed by git),
      then synthetic Committed `Merge` entries naming each result → `tour_progress` gives 7 for clash only; a
      one-parent result never: `tour_progress_ticks_7_for_the_clash_merge_and_not_for_feature`,
      `a_fast_forward_result_is_not_a_resolved_conflict`.
- [ ] `practice_at`: unset → `Now`; `"1700000000"` → `Fixed`; `"x"`, `"-1"`, `"1.5"` → `Err` with the words in R2:
      `practice_at_reads_whole_seconds_and_refuses_the_rest`.
- [ ] MAP.md graft-ops `## tour` (lines 212, 215, 216: dates, task 7, the odb write); graft MAP (`practice_at`, var).
**Touches:** `tasks_done` callers (grep), every `build_practice_repo(` caller, 19a tests, graft-ops/graft MAP, G-rename
27's var list (grep test covers it; say so), ticket 19 §A3 text (conductor marks superseded). graft-core untouched —
no journal format change (say how checked: `git diff --stat crates/graft-core` empty).

## 19c-b — each task ticks only by doing (Opus, ~50k)
**Crate set:** `crates/graft` — `tests/it/tour_tasks.rs` (new) + its line in `tests/it/main.rs`, test helpers only.
Production change → stop and report the cause, never adjust `tasks_done` to fit.
**Harness ruling:** headless over the real job pool (the `m5_walk` `run.rs` `Walk` kind) opened on
`build_practice_repo(scratch, PracticeAt::Fixed{..})` as a practice place, if a seam seeds `Arrival::practice`; else
the pty harness per task. Why headless is enough: ticks are a journal read inside one place; the place switches are the
full tour's (19c-c). Say which in NOTES. Each test: bit `k` false before; after the card line's keys (19b NOTES
lines) and the `TourProgress` product, bit `k` true in `AppState::tour` **and** in `tour_progress(work)`; other bits
unchanged; card shows the next unticked. Prerequisite tasks run in setup.
**Acceptance** (`crates/graft/tests/it/tour_tasks.rs`; replaces ticket 19 §A9 box 1, "8 pty walks"):
- [ ] `task_1_ticks_by_staging_a_hunk`, `task_2_ticks_by_committing`, `task_3_ticks_by_undoing_that_commit`,
      `task_4_ticks_by_setting_aside_and_bringing_back`, `task_5_ticks_by_making_a_branch` (asserts the entry kind is
      `BranchCreate`, §A2), `task_6_ticks_by_merging_feature_and_not_7`,
      `task_7_ticks_by_resolving_clash_and_finishing` (asserts R1's app-path facts: one `Merge` entry, Pending at the
      stop, Committed after `c`, two-parent result on `main`), `task_8_ticks_by_pushing`.
- [ ] `an_aborted_clash_merge_ticks_neither_6_nor_7`.
- [ ] `reading_the_card_ticks_nothing` — palette open/close, `?`, cursor moves, `⏎` on a commit: all 8 bits false.
**Touches:** `tests/it/main.rs`, any shared walk helper changed (list callers), graft MAP tests entry.

## 19c-c — the full tour in a pty, byte checks, captures (Opus, ~55k)
**Crate set:** `crates/graft` tests (`tests/it/tour_walk.rs` new + `main.rs` line), `docs/shots/`, the M8 contact-sheet
captions (grep how `m8-look-outside-repo-120x36` is captioned; do the same).
**Env for every walk:** `GRAFT_PRACTICE_EPOCH` = the instant existing pty captures are dated from (find it — the 18c
clone walk's or `graft_fixtures::git::FIXED_DATE`; name it in NOTES), `TMPDIR` = the `/tmp/graft-capture/<scenario>`
convention (`FIXED_ROOT_VAR`, graft-fixtures MAP line 23), `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE` = epoch + 1 h so the
commits he makes in the tour are stable too. **Confirm first** (grep `GIT_COMMITTER_DATE` in graft-git/graft-ops): no
Graft invocation overrides those for commit/merge; if one does, stop and report. Captures through
`graft_fixtures::capture` — asserted every run, never tolerant.
**Acceptance** (`crates/graft/tests/it/tour_walk.rs` unless named):
- [ ] Full tour in a pty from his repo: `⌃P tour ⏎`, the 8 tasks by the card's keys, all 8 ticked in the drawn card,
      lands in his repo, cursor on the oid he left, status `practice done ✓ · ⌃P tour goes again`:
      `the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` (ticket 19 §A9 "Full tour walked in a pty").
- [ ] His repo byte-for-byte untouched by that tour — `git_truth` (refs, HEAD, index tree, worktree tree) and the
      journal file's bytes before = after: same test, assertion `his_repo_is_untouched` (§A9 box "His repo…").
- [ ] State dir byte-identical — every file under the isolated config dir (recents, layout, first-launch `done`) and
      `.git/graft/` of his repo: same test, assertion `state_dir_is_unchanged` (§A9 box "State dir…").
- [ ] Practice dir gone after finish (no `tributary-practice-*` under the walk's `TMPDIR`): same test, assertion
      `practice_folder_is_gone`. After `Esc` and `q`: the existing `t_on_the_empty_state_starts_the_tour_and_esc_comes_back`
      and `q_inside_the_tour_removes_the_practice_folder` — **read them and cite the line that asserts absence**; if
      either asserts only the exit, add the absence check (§A9 box "Practice dir gone…").
- [ ] Captures, each with a caption saying what right looks like, colour **unverified**, ≤ 200 KB:
  - `docs/shots/m8-tour-step1-120x36.txt` — header `◇ practice · /tmp/graft-capture/…/work`; card at the graph pane's
    top, 3 rows, `practice 1 of 8 · stage a hunk` and `esc leave` on its border, line 1 of 19b NOTES' card lines, ticks
    row all open; graph: `main` 5 commits, `origin/main` one behind, `feature`, `clash`.
  - `docs/shots/m8-tour-step7-conflict-120x36.txt` — the resolver on `menu.txt`, the `dessert:` block, `pie` against
    `ice cream`; card (if drawn there) `practice 7 of 8`, ticks 1-6 ✓.
  - `docs/shots/m8-tour-80x24.txt` — step 1 at 80×24: card without its ticks row, `esc leave` still on the border.
  - `docs/shots/m8-tour-finished-120x36.txt` — his repo, no card, cursor on his commit, the finish status line.
  Test: `the_tour_captures_match` (or the full walk's capture steps — name them).
**Touches:** `tests/it/main.rs`, captures + contact sheet, the two existing Esc/q tests if extended, ticket 19's open
boxes (conductor ticks them naming these tests), KNOWN_GAPS unjudged rows for the 4 captures (conductor files).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide, every part.

## Size
a ~30k · b ~50k · c ~55k. Each one context; compaction → say so in NOTES.
