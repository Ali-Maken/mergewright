# G-daily 3a — a branch forked off a rewritten commit follows when clean (engine + preview data)

Status: done
Build with: Opus (conductor, after scout `scouts/14b-fork-follows.md`; `r` builds its todo via rebase_rows apply/engine.rs:162 → splits off per this ticket's rule; m5 left-behind capture may change)
Blocked by: 14
Needs: none (no signature from another ticket). Order: after `daily-ops-fixes` (12b; same graft-ops rewrite files).
Exposes the types `fork-follows-preview-words` draws.
File set: graft-ops `rewrite/` (plan, dependents, todo, saved, progress), `preview/` (effects, rewrite, chain),
`operation/rewrite.rs`, `apply/engine.rs` (todo hand-off only), graft-ops tests. **Not** graft: `left_behind` stays
filled (see Signatures) so the app compiles and draws as today until 3b.

## Read first
- `docs/adr/0059-a-branch-forked-off-a-rewritten-commit-follows-when-clean.md` (whole, 1 page; Accepted 2026-10-02)
- `docs/adr/0047-rewrite-engine-and-dependent-branches.md` §1, §2, §4
- `docs/design-notes/m8-polish-and-ship.md` "Pass 2c"
- `crates/graft-ops/MAP.md` `## rewrite`, `## preview` (`PreviewPlan` m7/37 fields, `ConflictForecast::StopsAt`,
  `plan_as_run`), `## operation` (`Operation::tips`), `## journal` (`touches_of`)
- Ticket 14 NOTES "Item 2 — what it needs" (branch `worktree-agent-a67428ac4073c7eaf`)

## Scout questions (answer before `Build with` is set; ≤1 page, code not intent)
1. Does `r` (rebase onto) build its todo through `RewriteParams`, or by another path? If another, `r` splits off.
2. Where is `LeftBehind` computed today (file:line), and which inputs does it already hold (merge-base, cap)?
3. Which tests assert `left_behind` for a fork whose replay would be clean (list each, file:line)?
4. Who reads `RewriteProgress` (graft banner and any other), and does any count `update-ref` lines?
5. `plan.json`: where is it read and written, and how does an unknown/missing field behave today?
6. Where do `Operation::tips` / `rewrite::tips` feed `check`'s `Moved` and the journal's `refs_before`/`refs_after`?
7. Every `PreviewPlan` and `RewriteParams` literal site, by crate (the compiler list).

## Scope
In: fork detection and fates; forecast per fork; `label`/`reset` sections in the todo; saved plan; `progress`;
operation tips / journal refs; ghosts and ref moves for followers in `PreviewPlan`. Every kind that reports
`left_behind` today: `Operation::Rewrite` (`R`, `a`-into, `a` on HEAD with dependents, absorb) and `r`.
**If the scout finds `r`'s params don't reach the todo through `RewriteParams`, `r` is split off — say so, don't guess.**
Out: drawing the fates (3b); ADR-0047 §4's tip-fork refusal (unchanged) — **including a branch started from a
dependent's commit, which is a second dependent and stays refused (ADR-0059 §1, §5; decided 2026-10-02)**; forks of
forks (they stay, `Nested`).

## Signatures
```rust
// graft-ops::preview (effects.rs), re-exported from preview
pub struct Fork {
    pub branch: RefName,   // full
    pub at: Oid,           // the replaced commit it leaves the line at (old id) == LeftBehind.on today
    pub commits: usize,    // its own: at..branch
    pub fate: ForkFate,
}
pub enum ForkFate { Follows { onto: Target }, Stays(WhyStays) }   // onto: the ghost of at's row
pub enum WhyStays {
    WouldConflict { commit: Oid, paths: Vec<PathBuf> },  // one of its own commits
    StopsBelow,                                          // the line's forecast stops at or below at's row
    HasMerge { merge: Oid },
    Pushed { upstream: RefName },
    Elsewhere { worktree: PathBuf },
    Nested { on: RefName },                              // forked off another fork
    DefaultBranch,
    PastCap,
    NotChecked(String),
}
impl WhyStays { pub fn words(&self) -> String; }   // graft-ops' clause, no keys: "would conflict in src/a.rs"
// PreviewPlan gains `pub forks: Vec<Fork>, pub forks_more: usize`.
// `left_behind`/`left_behind_more` stay, filled from the Stays forks only (3b removes them).

// graft-ops::rewrite
pub struct ForkSection {
    pub after_row: usize,     // todo index whose result is the fork's new base
    pub branch: Tip,          // pinned old tip
    pub picks: Vec<TodoLine>, // at..branch, oldest first, all Pick
    pub inside: Vec<Tip>,     // other local branches on at..branch's first-parent chain, bottom up
}
// RewriteParams gains `pub forks: Vec<ForkSection>`; empty → the todo is today's, byte for byte.
// RewriteProgress gains `pub fork: Option<String>` (no refs/heads/): the section git is in.
// plan_as_run: signature unchanged; fills params.forks (followers) and PreviewPlan.forks (all fates).
// rewrite::tips (operation/rewrite.rs): adds every follower's branch + inside.
```

## Acceptance (truth read with git, never with Graft's own reads)
- [x] Clean fork follows (`fork_follows::a_clean_fork_follows_an_amend_into`): `main─c1─c2─c3` (`feature-a`, HEAD), `feature-x` = c2 + x1 (other file); stage a change,
      `a` into c1, run. `git rev-parse feature-x~1` == `git rev-parse feature-a~1`; `git diff feature-x~1 feature-x`
      == x1's old diff; subject kept; HEAD home on `feature-a`.
- [x] Conflicting fork stays (`fork_follows::a_conflicting_fork_stays_and_the_line_moves`): x1 edits the line the amend changes. Preview: `Stays(WouldConflict{commit: x1, ..})`;
      after `y`, `git rev-parse feature-x` unchanged; the line moved.
- [x] Forks with a dependent (`fork_follows::a_branch_off_a_dependents_middle_is_refused_as_a_second_dependent` (a), `fork_follows::a_dependent_and_a_fork_off_his_middle_both_follow` (b)) (ADR-0059 §1, §5; corrected 2026-10-02 — the first text assumed a fork off the
      dependent's middle follows; it is a second dependent and is refused, as today):
      (a) `feature-b` = feature-a + b1 b2 b3 (dependent), `feature-x` = b2 + x1. Reword HEAD (`RewordHead`) → refused
      before anything runs, words name both branches (`feature-b and feature-x are both built on yours`, ADR-0047 §4);
      `git for-each-ref refs/heads` unchanged. No `Fork` for `feature-x` in the preview.
      (b) Same `feature-b`; `feature-x` = c2 + x1 (off his own middle commit). `R`, reword c1, run → both follow (git):
      `git rev-parse feature-x~1` == `git rev-parse feature-a~1`; `git merge-base --is-ancestor feature-a feature-b`;
      `feature-b`'s three own diffs unchanged; HEAD home on `feature-a`.
- [x] `u` after a follow (`fork_follows::undo_after_a_follow_puts_every_ref_back`): every ref (`for-each-ref refs/heads`) equals before; one journal entry.
- [x] Oracle (`rewrite_oracle::a_fork_follows_in_the_same_rebase`): the generated todo hand-run with `git rebase -i` gives the same trees and refs (`rewrite_oracle.rs`).
- [x] Verified against git (`rewrite_oracle::a_fork_follows_in_the_same_rebase` — plain `rebase -i`, no `--rebase-merges`, engine flags `operation/history.rs:790-795`; `fork_follows::an_abort_at_a_stop_after_a_section_leaves_the_fork`): `label`/`reset` run without `--rebase-merges`; abort during a stop after a section leaves
      the fork ref unchanged (`git rev-parse`). If either fails: stop, report, don't work around (ADR-0059 §3).
- [x] `progress` during a hand-run (`fork_follows::progress_inside_a_section_names_the_fork`) of Graft's todo with a `break` inside a section: `fork == Some("feature-x")`,
      `step` equals the pick-like rows of `rebase-merge/done`.
- [x] Stays reasons each tested (`fork_follows::a_pushed_fork_stays`, `a_fork_checked_out_elsewhere_stays`, `a_fork_with_a_merge_stays`, `a_fork_of_a_fork_stays_nested`, `the_default_branch_stays`, `the_ninth_fork_is_past_the_cap_and_stays`, `an_unchecked_forecast_never_follows`): pushed (upstream holds x1), elsewhere (worktree), merge in `at..F`, nested, default
      branch, ninth fork `PastCap`, forecast `NotChecked` (git < 2.40 path or cap) → never `Follows`.
- [x] (`fork_follows::a_fork_moved_after_the_preview_is_refused_moved`) Fork moved between preview and `y` → `Moved`, nothing written (git).
- [x] (`rewrite::saved::tests::a_plan_saved_before_forks_reads_as_none`, `fork_follows::a_stop_resumed_after_relaunch_finishes_the_section`) `plan.json` without `forks` (an older plan) reads; a stop resumed after relaunch finishes the section.
- [x] No fork (`rewrite::tests` todo-text tests + every `rewrite_oracle` row green, graft-ops run 2026-10-02): todo text byte-equal to today's (existing oracle tests unchanged and green).

## Touches
- m7/37 tests asserting `left_behind` for a **clean** fork now see it follow: list each, update the expectation, say so.
- `PreviewPlan` and `RewriteParams` literal sites (compiler-enumerated, graft tests included: field defaults only,
  no graft logic). `Operation::tips` → `check`'s `Moved` and the app's re-check before `y`. Journal `touches_of`
  (refs_before/after), undo/redo of a rewrite. `progress` readers: graft banner (`branch` index must not count a
  follower's `update-ref`). Saved-plan format (`plan.json`, a contract). `rewrite_oracle.rs`. MAP.md graft-ops.
- Captures showing `left behind:` (`docs/shots/m7/amend-into-left-behind-187x44.txt`, `rewrite-editor-left-behind-…`,
  m5 contact sheet): unchanged by 3a (app untouched); 3b retakes them.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~75k. If the scout finds `r` needs its own params path, split `r` off first.

## Handoff (partial, 2026-10-02) — builder compacted once

**Why stopped:** conductor asked me to `git checkout worktree-agent-aa4abba36c4d3ab03` (worktree had been moved to
`review/m8-14` by a reviewer). Git refused: local changes to `crates/graft/src/update/tests.rs` would be overwritten
(the two bases differ by 2 lines there; my edit adds one `fork: None`). Nothing committed. Backup of all WIP:
scratchpad `14b-wip.patch` (tracked diff) + `forks.rs` (new, untracked `crates/graft-ops/src/preview/forks.rs`).

**Done (uncommitted, workspace compiles):** engine + preview per ADR-0059 — `ForkSection`, `RewriteParams.forks`,
`RewriteProgress.fork`, todo sections (label/picks/inside update-refs/update-ref/reset), `plan.json` `forks` (serde
default, test `a_plan_saved_before_forks_reads_as_none`), progress parsing of label/reset/non-line update-refs,
engine guard (`after_row >= rows run` → `Moved`), `Operation::tips` includes forks + inside tips, preview `Fork`/
`ForkFate`/`WhyStays`, `PreviewPlan.forks`/`forks_more`, `plan_as_run` fixes Rewrite forks. `r` untouched (forks
empty; KNOWN_GAPS row). graft forced sites: literals only, plus `update/amend_into.rs::is_put_in` ignores `forks`
(test `a_put_in_carrying_forks_is_still_the_put_in`).

**Test state:** graft-ops 1008/1011 pass. 3 expected failures, m7/37 expectations to update (clean fork now follows):
`rewrite_preview.rs:973` (left_behind → [], assert feature-b in `forks` as Follows), `:986` (left_behind = [backup]
only; a backup is not a fork), `:1032` (cap: `off_line` names 8; all 8 follow → left_behind [], left_behind_more 2,
forks.len 8, forks_more 2).

**Next exact steps:** 1) resolve the branch switch (conductor). 2) update the three tests above. 3) new
`tests/it/fork_follows.rs` (+ `mod` in main.rs) for acceptance 1, 2, 4, 6b (abort), 7, 8, 9, 10; and the substitute
for 3. 4) workspace gates. 5) MAP.md graft-ops. 6) commit.

**Acceptance 3 contradicts ADR-0059 §5:** a branch forked off the dependent's middle commit is itself a dependent
(`rewrite/dependents.rs:110-119`: any branch whose first-parent chain reaches his tip), and two dependents are refused
(`:38-40`, `ForkedDependents`). Proposed: test the refusal, plus R + dependent + fork off his own mid-chain commit.
**Ruled 2026-10-02 (planner):** the code and ADR-0059 §5 are right; acceptance 3 was wrong and is rewritten as (a)
refusal + (b) follow, the builder's proposal. ADR-0059 §1/§5 now say so explicitly. No code change.

**Semantics for 14c:** `PastCap` never appears in `forks` (forks past 8 are only counted in `forks_more`);
`pending` excludes sections not yet entered, the open section's rest first; `step` total counts every pick-like row.

## Paused at (2026-10-02, continuation 1, conductor pause for CPU) — no compaction

**Done:** 3 m7/37 tests updated (`rewrite_preview.rs` `left_behind_names_a_branch_forked_mid_chain`,
`..._parked_on_a_replaced_commit`, `left_behind_caps_at_eight`; each cites ADR-0059 line 27-28/34/36). New
`tests/it/fork_follows.rs`, 16 tests, all pass (ticks above name them). Clippy too-many-lines refactors in
`apply/engine.rs`, `preview.rs` (`kind_plan`), `preview/rewrite.rs` (`steps_of`, `draw_forks`), `preview/forks.rs`
(`Env`, `Fates`, `candidates_of`, `replayed_fate`), `progress.rs` (`next_back`). MAP.md graft-ops updated.

**Last gates:** fmt ok; doctests ok; clippy FAILS only in graft: `update/confirm.rs:78` `large_enum_variant` —
`RewriteParams` grew 232→256 bytes (`forks: Vec`), so `ConfirmThen::Operate(Operation)` crossed clippy's 200-byte
variant gap (second variant 48). nextest workspace: 4944/5270 run, 6 failed, 9 skipped (run cut at a 600 s
filter; named: `graft::it m7_walk::rewrite_editor::the_box_says_who_stays_behind`,
`the_rewrite_editor_after_says_who_stays_behind` — the other 4 names not captured; rerun to list).

**Question for the conductor (two scope breaks, both in graft, outside my set):**
1. Clippy: options — (a) box `ConfirmThen::Operate` (21 sites, 11 graft files); (b) `#[expect(large_enum_variant)]`
   on `ConfirmThen` (1 line); (c) `forks: Box<[ForkSection]>` in graft-ops (exactly at the 200 threshold: fragile;
   deviates from the Signatures). Recommend (a).
2. The m7 walk tests: the fixture fork (`feature-b` off part 1, own file) now follows cleanly, so the app no longer
   draws `left behind: feature-b` and the two captures (`docs/shots/m7/amend-into-left-behind-187x44.txt`,
   `rewrite-editor-left-behind-187x44.txt`) change. The ticket's Touches said "unchanged by 3a" — wrong, because
   `left_behind` is now filled from Stays forks only (as Signatures ask). Belongs to 14c (draw fates, retake captures)
   or a fixture change making the fork stay (a conflict works for `a`, not for a no-edit `R`).

**Next step:** conductor rules on 1 and 2; then rerun `cargo nextest run --workspace --no-fail-fast` and list the
other 4 failures; gates; commit `m8-14b: ...`.

## Continuation 2 (2026-10-03) — no compaction; scout `scouts/14b-fork-follows.md` (from the first session) used

**Conductor rulings applied:**
1. Clippy `large_enum_variant` (`update/confirm.rs`): `ConfirmThen::Operate(Box<Operation>)` + constructor
   `ConfirmThen::operate(op)`. 21 sites: 17 constructions renamed to `operate(..)`; pattern sites rewritten
   (`confirm.rs` `takes_y` now matches `&**op` inside one `Operate` arm, same answers; `confirm_key` launches `*op`;
   `operate/tests.rs`, `remote/tests/lease.rs`, `tests/it/remote_by_key.rs` deref the box). graft MAP.md updated.
2. m7 walk: the fixture's `feature-b` (off part 1, own file) now follows. Re-shot with `GRAFT_UPDATE_SHOTS=1`:
   `docs/shots/m7/amend-into-left-behind-187x44.txt`, `docs/shots/m7/rewrite-editor-left-behind-187x44.txt`. Diffs
   read line by line — only the fork-follows change: feature-b's label moves off the old b1 onto a new ghost b1 on
   the new part 1; `3 commits replayed` → `4`; `left behind: feature-b …` → `feature-b f81a2f8 → 2 new commits ·
   built on yours · follows` (box) / `built on yours: feature-b (Graft Fixture, 1 commit) follows` (editor AFTER).
   Tests `the_box_says_who_stays_behind`, `the_rewrite_editor_after_says_who_stays_behind` (names kept) now assert
   the follows line in the same place and no `left behind`; unused `old_part_one` removed.

**For 14c (question, not a defect):** the app words a follower fork with the dependent's phrase `built on yours`
(capture above). ADR-0059 distinguishes a fork from a dependent; whether the words should differ is 14c's call.
File names `*-left-behind-*` now show a follow; 14c retakes/renames.

**Environment:** 6+ graft tests (`askpass_mode::*`, `box_owns_the_bar::capture_fix_box_owns_the_bar`,
`credential_by_key::*`, `daily_landing::j18_…`) fail when `$TMPDIR` is this harness's long path (`path must be
shorter than SUN_LEN`, askpass socket). Not this ticket; gates run with `TMPDIR=/tmp`.

## Continuation 3 (2026-10-03) — gates only, no code change, no compaction
Workspace gates on this tree (`TMPDIR=/tmp`): `cargo fmt --check` ok; `cargo clippy --all-targets -- -D warnings`
exit 0; `cargo nextest run --workspace --no-fail-fast` 5270/5270 passed, 9 skipped (757 s); `cargo test --doc` ok.
Captures re-read: both m7 files 4.7 KB, diff shows only the follow (feature-b ghost on new part 1, `follows` line,
no `left behind`). Ticks 10/10 name tests.
