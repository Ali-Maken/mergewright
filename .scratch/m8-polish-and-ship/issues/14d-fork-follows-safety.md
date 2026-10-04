# G-daily 3c — a fork follows only when it is safe: carried branches, pushed, base unchanged

Status: draft
Build with: Opus — three gates in one private module plus git-truth tests; the rules are decided (ADR-0059 §6), no
structural choice left. Not Fable: correctness is judged by the git-truth tests below.
Blocked by: none
Needs: none (14b/14c merged: `Fork`, `ForkFate`, `WhyStays`, `ForkSection`, `plan_as_run`).
File set: graft-ops `preview/forks.rs`, `preview/effects.rs` (`WhyStays` + `words`, the `Elsewhere` doc),
`preview/rewrite.rs` (`holds_any` only), `tests/it/fork_follows.rs`, MAP.md graft-ops; graft only where it matches
`WhyStays` exhaustively (grep) and `tests/it/m7_walk/fork_rows.rs` + its shots; `CONTEXT.md` (one entry).
Scout first (brownfield, ADR-0035): ≤1 page on `forks.rs` — where candidates are claimed as `inside` (:128-131,
:231-248), where `holds_any` is called (:363-370), where `after_row` is chosen (:323-331), the order reasons are
checked in, and every exhaustive `match` on `WhyStays` in the workspace.

## Why (review `docs/reviews/m8-14bc-fork-follows.md`, Majors; M3 "you cannot lose work", M5 "you see the result before you confirm")
- M1: a branch on a following fork's chain is moved unweighed — one checked out in a linked worktree was moved (that
  worktree then showed `M  c1.txt`, staged, never made by its owner); a pushed one was moved; the preview named neither.
- M2: `pushed` checks only `at..F`: a fork whose upstream holds `at` (or a replaced commit under it) follows and diverges.
- M3: a fork off a dropped commit follows without it (`git show feature-x:c2.txt` fails after); reorder unruled.

## Read first
- `docs/adr/0059-a-branch-forked-off-a-rewritten-commit-follows-when-clean.md` §2 and **§6** (the rules; amended
  2026-10-04)
- The review's M1–M3 (above file, lines 34–86): the probes to reproduce
- `crates/graft-ops/MAP.md` `## preview` (`Fork`, `ForkFate`, `WhyStays`, `published`), `## rewrite` (`ForkSection`,
  `squash_groups`, `dependents` — its worktree check)
- `crates/graft/MAP.md` `update::preview` fork rows (`fork_rows`, tones), rewrite editor right pane
- This ticket's scout file

## Scope
In: ADR-0059 §6.1–6.4 in `preview::forks`; two new `WhyStays` variants and their words; the `Elsewhere` doc fix;
the `after_row` rule for squash/fixup groups; tests; the editor shot's retake; the CONTEXT.md **Fork** entry.
Out: `r` (forks empty, gap M8/fork-follows-r); the default-branch override (review m1); `plan.json` reader (m2);
`PastCap` (m3); the shared predicate (m4); the app walk through `y` (m5, owed by `accept-m8`); perf (m8).

## Signatures
```rust
// graft-ops::preview (effects.rs) — WhyStays gains:
pub enum WhyStays {
    // ... existing variants unchanged ...
    /// A branch on the fork's own commits fails a gate (ADR-0059 §6.1). `why` is only
    /// `Elsewhere`, `Pushed` or `DefaultBranch`.
    Carries { branch: RefName, why: Box<WhyStays> },
    /// Following would drop a commit F is built on (§6.3): dropped, or moved after `at`'s row.
    WouldLose { commit: Oid },
    /// Following would add a commit F was not built on (§6.3): moved to or under `at`'s row, or folded into it.
    WouldGain { commit: Oid },
}
// words(): Carries → "<short branch> is <why.words()>"   e.g. "feature-w is checked out in ../wt",
//                    "feature-w is pushed to origin/feature-w", "main is the default branch"
//          WouldLose → "would lose <sha7>"     WouldGain → "would gain <sha7>"
// Pushed { upstream } keeps its words "pushed to <ref>"; `upstream` may now name any remote-tracking ref (§6.2).
```
Row text is unchanged in form: `feature-x stays on a1b2c3d · <words>`. New variants draw **muted** (not the `!`
warning path, which stays `WouldConflict` only).

## Acceptance (truth read with git, never with Graft's reads; each tick names its test at tick time)
**M1 — carried branches** (`feature-a` = main─c1─c2─c3, HEAD; `feature-x` = c2 + x1 x2; `feature-w` at x1)
- [ ] Linked worktree: `git worktree add <tmp>/wt feature-w`; stage `c1.txt`, `a` into c1, run. Preview:
      `feature-x` is `Stays(Carries { feature-w, Elsewhere })`, row `feature-x stays on <c2> · feature-w is checked out
      in <path>`; no `RefMove` for `refs/heads/feature-x` or `feature-w`. After: `git rev-parse feature-x feature-w`
      unchanged; `git -C <wt> status --porcelain` empty; the line moved (`feature-a~1` changed).
- [ ] Fork itself checked out in a linked worktree (extend `a_fork_checked_out_elsewhere_stays`): after the run,
      `git -C <wt> status --porcelain` empty and its `HEAD` unchanged.
- [ ] Carried branch pushed (bare remote, `origin/feature-w` = x1, upstream set): `Carries { feature-w, Pushed }`;
      both refs unchanged after the run.
- [ ] Carried branch is the default branch (set up as `the_default_branch_stays` does): `Carries { .., DefaultBranch }`.
- [ ] Control — carried branch clean, unpushed, nowhere else: both follow; `git rev-parse feature-w` ==
      `git rev-parse feature-x~1`; `git merge-base --is-ancestor feature-a~1 feature-w`.
- [ ] `effects.rs` `Elsewhere` doc no longer says git refuses (grep `refuse` in its doc); same grep over
      `rewrite/dependents.rs`'s worktree-check docs, corrected if it says so.

**M2 — pushed** (each with a bare remote; `R`, reword c1)
- [ ] `park` at c2, 0 own commits, upstream `origin/park` = c2 → `Stays(Pushed { origin/park })`; `park` unchanged.
- [ ] `feature-x` = c2 + x1, upstream holds c2 only (pushed before x1) → stays, pushed.
- [ ] `feature-x` upstream holds c1 only (a replaced commit under `at`) → stays, pushed.
- [ ] No upstream, but `refs/remotes/origin/feature-x` = x1 (pushed without `-u`) → `Pushed { origin/feature-x }`.
- [ ] Control — upstream `origin/main` (holds only `onto` and below) → follows.

**M3 — base unchanged** (`R` on `feature-a` = c1 c2 c3 c4; `feature-x` = c2 + x1, so `at` = c2)
- [ ] Drop c2 → `Stays(WouldLose { c2 })`, words `would lose <c2>`; after: `git rev-parse feature-x` unchanged,
      `git show feature-x:c2.txt` succeeds.
- [ ] Drop c1 → `WouldLose { c1 }`.  Move c1 after c2 → `WouldLose { c1 }`.
- [ ] Move c3 before c2 → `WouldGain { c3 }`.  Fixup c3 into c2 → `WouldGain { c3 }`.
- [ ] Controls (follow, `git diff feature-x~1 feature-x` == x1's old diff): drop c4; swap c3 and c4; fixup c2 into c1
      (F follows onto the folded commit: `git rev-parse feature-x~1` == `feature-a~2`'s new id, its tree == old c2's).
- [ ] `a` into c2 itself (fork point is the amend target): follows onto the amended commit —
      `git show feature-x~1:<staged file>` has the staged change; the todo's `label` comes after the fixup row
      (oracle: `rewrite_oracle` hand-run gives the same refs and trees).
- [ ] A gate that fails runs no forecast for that fork (unit test on `forks.rs`'s order, or a spawn count).

**Words and shots**
- [ ] `WhyStays::words` table test: every new variant, and `Carries` over each of its three inner kinds.
- [ ] graft row tone: a `Carries`/`WouldLose`/`WouldGain` stays row is muted, no `!` (row test, preview box and editor).
- [ ] Retake `docs/shots/m8/fork-follows-rewrite-editor-187x44.txt` (`m7_walk::fork_rows::the_rewrite_editor_names_each_fork`):
      the dropped-base fork now reads `stays on <sha7> · would lose <sha7>`; the shot still shows at least one
      `follows` row (adjust the fixture if not). Rerun the other two `fork_rows` walks; diff each shot line by line and
      say what changed. Contact sheet: conductor. Colour **unverified**.
- [ ] `CONTEXT.md`: entry **Fork** — "A local branch started from a commit in the middle of his branch (not built on
      its tip). When he rewrites that commit the fork follows if it is safe (ADR-0059 §2, §6); on screen `follows` /
      `stays on <sha7> · <why>`. _Avoid_: left behind (except `r`), child branch." (review m6)
- [ ] MAP.md graft-ops: `WhyStays` line gains the three variants; `ForkSection.after_row` notes "last row of `at`'s
      squash/fixup group".

## Touches
- `Fates` / candidate claiming: fewer followers → fewer `ForkSection`s, fewer `rewrite::tips` (check's `Moved`,
  journal refs) — no format change; `plan.json` `forks` unchanged in shape.
- Fork rows in every rewrite preview and the editor (graft): new words, muted path; the `!` path unchanged.
- Existing `fork_follows` tests whose fixtures carry a branch or a remote (grep `inside`, `origin/`): rerun, say which
  changed fate and why.
- Captures: the three m8 `fork-follows-*` shots; m7 `amend-into-left-behind`, `rewrite-editor-left-behind` (rerun;
  expected unchanged — their fork has no remote, no carried branch, no drop). `TESTING.md` rows naming fork words (grep).
- Forecast cost: gates before the forecast can only lower spawns (review m8).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k (≈20 git-truth tests share one fixture builder).
