# the-walk-proves-a-conflict-can-be-finished

Status: **done** (2026-09-22). Seven of the eight `ok`; the eighth fails on
purpose as a finding — see **Findings**.
Blocked by: **37** (shares `graft`, and `steps.rs`/`every_scenario.rs`).
Crates: `graft`.
Build with: **Opus.** Conflicts and the rebase state machine (CLAUDE.md's
model table routes conflicts to Opus), plus a foxtrot fixture that does not
exist yet.
Scout: **not needed** — read `crates/graft/tests/it/m5_walk/` itself.

## Why this exists

The second of three tickets closing the **broken** gap "22 of the acceptance
walk's 45 scenarios were never walked" (the handoff names 26; the row
undercounts and the conductor has corrected it). Ticket 37 took refs and
names; ticket 39 takes undo, push and secrets. **Do not start on theirs.**

M5's promise is *"you see the result before you confirm."* These eight
scenarios are where that promise is hardest: a preview refused mid-operation,
a merge with nothing to do, a revert of a merge, and a conflict you have to
get out of.

## Piece 1 — a foxtrot fixture

The `fix-*` trio needs one. **The shape the radar tests already use** is the
starting point (ticket 11's handoff says so) — find it before building a new
one. If you must build it by hand, name it in your report and say why the
existing shape would not serve.

## Piece 2 — eight scenarios

In `tests/it/m5_walk/` (`preview.rs`, `merge_rebase.rs`, `pick_revert_reset.rs`,
`conflicts.rs` as the split dictates; largest file under 400 lines):

- `preview-refused-during-an-operation`
- `merge-already-up-to-date-offers-close`
- `revert-merge-uses-mainline`
- the `fix-*` trio (three scenarios, on the foxtrot fixture)
- `continue-refuses-until-resolved-then-one-entry-undoes`
- `rebase-stops-twice-same-entry`

`continue-refuses-until-resolved-then-one-entry-undoes` and
`rebase-stops-twice-same-entry` are the two that matter most: **a rebase that
stops twice must leave one journal entry, and one `u` must undo the whole
thing.** That is M3's promise ("you cannot lose work") holding inside M5's.
If either does not hold, it is a finding, not something to fix here.

Truth read back by **running git** — `for-each-ref`, `rev-parse HEAD`,
`symbolic-ref HEAD`, `write-tree` of the index, file bytes,
`rebase-merge/`/`MERGE_HEAD` presence, the journal file — **never from
`AppState`**. The one `AppState` read is the claim: `PreviewState.plan`.
Every scenario ends `git fsck --no-dangling` clean.

## Gotchas, from ticket 11's handoff — read before you write

- `Wait::Ready` needs `remotes` Ready too, not just refs.
- `menu_pick` matches a **label prefix**; exact labels are in
  `update/context_menu/verbs.rs`. Do not guess one.
- A `See` closure **must not read `PreviewState`** — a grep test fails you.
- `ghost_equals_git` requires a **non-empty `ref_moves`**; a scenario with no
  ref move asserts refs itself. `merge-already-up-to-date` has none.
- `Step::Type` and `Wait::Prompt` were added by **ticket 37** — use them, do
  not rebuild them.

## Piece 3 — wire each one in

Add each to the `walk!` list in `every_scenario.rs`, and its entry to
`TESTING.md` "History": **renumber, and move it out of "Not yet walked"**.
Each entry says concretely what right looks like — exact box lines, status
strings, `for-each-ref` relations, byte equalities. Never "looks correct";
quote only what a `See`/`expect` asserts.

## One thing this walk cannot do

**The real `git mergetool` hand-over is not exercised and must not be faked.**
Ticket 11 marked it explicitly not-for-builders: it needs a real terminal and
the user runs it. `mergetool-absent-refuses-in-words` (ticket 39) covers the
*absent* case in words only. If a scenario here would need a live mergetool,
stop and report rather than stubbing one.

## Context guard

One line per scenario in your report; a failing scenario <= 20 lines. **Never
read a capture or the contact sheet back into your context** — sizes and
`head -3` only. Gate output quiet, full output only on the failing part.

## NOT yours

- **Fixing what the walk finds.** A scenario failing because the app is wrong
  is a **finding for the conductor**, filed here, left failing. Ticket 32 was
  built from exactly that. Do not make a red test green by weakening it.
- Tickets 37's and 39's scenarios.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's.
- The heavy tier, the release perf run, running in a real terminal.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] **Reused, not built.** `Shape::FoxtrotMerge` — the shape
      `graft-ops/tests/it/mistake_radar.rs:303` (`check_foxtrot`) is written
      against — already has the foxtrot as `main`'s tip with `main` checked
      out, so all three `Fix:` rows appear on one row. `fixture::foxtrot`
      (`crates/graft/tests/it/m5_walk/fixture.rs`) scripts nothing; it
      asserts the shape by git (`log -1 --format=%P HEAD` has two parents,
      `symbolic-ref HEAD` is `refs/heads/main`) so a change to the shape
      fails there and not inside a silent menu. Tests:
      `m5_walk::every_scenario::fix_revert_the_merge`,
      `fix_reset_to_before_the_merge`, `fix_rebase_instead`.
- [x] Eight scenarios, each run as `cargo run -p graft --example m5_walk --
      --only <name>`:
      `preview-refused-during-an-operation` **ok**;
      `merge-already-up-to-date-offers-close` **ok**;
      `revert-merge-uses-mainline` **ok**;
      `fix-revert-the-merge` **ok**;
      `fix-reset-to-before-the-merge` **ok**;
      `fix-rebase-instead` **FAIL — finding 1**, quoted whole:
      `FAIL fix-rebase-instead: ghost 1: subject `Merge branch 'main' into
      feature`, git wrote `feature: work``;
      `continue-refuses-until-resolved-then-one-entry-undoes` **ok**;
      `rebase-stops-twice-same-entry` **ok**.
- [x] Test: `m5_walk::every_scenario::rebase_stops_twice_same_entry`
      (`conflicts.rs`, verdict `two_stops_one_entry`). The journal assertion
      is `the_one_entry(truth, OperationKind::Rebase)?`, which fails unless
      `entries_of(truth, Rebase) == 1`, applied at **both** stops (status
      `Pending` each time) and again after the second `c` (status
      `Committed`); the two stops are proved distinct by
      `first.repo.head != second.repo.head`. One `u` then holds:
      `branch(undone, "feature")? == branch(&marks.before, "feature")?` and
      `same_files("one u after two stops", &marks.before, undone)`.
- [x] Git commands the eight read truth from (`truth::truth`, `refs_of`,
      `in_progress`, `journal`, plus each verdict's own): `for-each-ref`,
      `rev-parse HEAD`, `rev-parse <oid>~n`/`<oid>^2`, `symbolic-ref -q
      HEAD`, `ls-files -u`, `ls-files -s`, `write-tree`, `log -1
      --format=%P`, `log -1 --format=%s`, `diff-tree -p`, `patch-id
      --stable`, `cat-file`, the `.git/MERGE_HEAD` and `.git/rebase-merge`
      markers, `.git/graft/undo-journal.jsonl`, and file bytes off disk.
      The only `AppState` read that judges anything is the claim
      (`Step::Claim` → `PreviewState.plan`); `Step::See` closures read only
      the drawn box, the status line, the menu and `banner_focus`, and
      `m5_walk::every_scenario::ghost_equals_git_is_the_only_claim_check`
      greps all eight source files — `radar_fixes.rs` was **added to its
      hand-enumerated list**.
- [x] Test: `m5_walk::every_scenario::every_scenario_has_a_test` (passes;
      it asserts the `walk!` list equals `scenarios()` in order).
- [x] All eight are in `TESTING.md` "History" → "The scenarios", renumbered
      1–38, and out of "Not yet walked". Quoted whole, entry 28:

      > 28. **rebase-stops-twice-same-entry** — `main` and `feature` both
      >     rewrote `shared.txt`, `feature` twice; `r` on `main` from
      >     `feature`, confirm: git stops, is resolved as `resolved once\n`,
      >     `c`, **stops again** at a different `HEAD`, is resolved as
      >     `resolved twice\n`, `c`. At both stops there is exactly **one**
      >     `Rebase` entry and it is `Pending`; after the second `c` that
      >     same one entry is `Committed` — one entry across two stops. The
      >     replayed branch's `~2` is `main`'s tip and `shared.txt` is
      >     `resolved twice\n`. One `u` puts `feature` back at its old
      >     commit with every file byte-identical to before the rebase.

- [x] `git fsck --no-dangling` is the **last line of `run_scenario`**
      (`m5_walk/run.rs`: `fsck_clean(fixture.repo())`), so every scenario
      ends with it and a scenario cannot opt out. All eight returned from it
      without error.
- [x] Slowest new scenario: `rebase_stops_twice_same_entry` at **1.479 s**
      (`cargo nextest run -E 'test(m5_walk::every_scenario)' --status-level
      pass`; next is `continue_refuses_…` at 1.244 s). No `sleep` and no
      poll loop anywhere: every wait is `Walk::pump`, which blocks on the
      real job-pool channel and fails by timeout rather than spinning.
- [x] Whole suite, workspace-wide `cargo nextest run`: **before 2445
      passed, 1 skipped** (`docs/log/m5.md:1246`); **after 2453 run, 2452
      passed, 1 failed, 1 skipped** — +8 tests, the one failure being
      finding 1, left failing on purpose. `cargo fmt --all --check` and
      `cargo clippy --all-targets -- -D warnings` clean; `cargo test --doc`
      run (vacuous, 0 tests — not offered as evidence).

Rule 6: you do not tick "looks right". The user does.

## Findings

### 1 (defect, Major) — a rebase draws a ghost for a merge git will drop

`fix-rebase-instead` is left **failing**. `Fix: rebase instead` replays
`main` (tip = the foxtrot merge) onto the merge's second parent. The range
is `{feature: work, the merge}`, and the plan draws **one ghost per commit
in it**, merge included, then points the branch's `RefMove` at the last
ghost — the merge's. `git rebase` without `--rebase-merges` drops merge
commits, so git wrote one commit where the box promised two, and the tip
the box predicted never exists.

Lines that prove it:

- `crates/graft-ops/src/preview/kinds.rs:113` — `for (index, commit) in
  range.commits.iter().enumerate()` pushes a `GhostCommit` for every commit,
  with no merge filter; `:128` points the `RefMove` at
  `Target::Ghost(plan.ghosts.len() - 1)`.
- `crates/graft-git/src/history_reads.rs:94` (`commits_between`) builds that
  range with `rev-list --reverse` and **no `--no-merges`** (args at
  :104–111).
- Observed: `FAIL fix-rebase-instead: ghost 1: subject `Merge branch 'main'
  into feature`, git wrote `feature: work``.

Everything else in that scenario holds and is proved: the replayed tip has
exactly one parent and it is `<merge>^2`, all three files survive, the last
entry is `Rebase`/`Committed`, and `u` puts `main` back on the merge. The
claim is judged **last** in `rebase_instead_holds` on purpose, so those
eight checks are reached rather than sitting unreachable under it (the m5/32
lesson).

Not fixed here — the ticket says a scenario failing because the app is wrong
is the conductor's.

### 2 (question, not a defect) — `u` right after a finished `--continue`

Pressing `u` immediately after the continue's own `Status::Done`, with the
walk's `Wait::Ready` already satisfied, was refused with `finish or abort
the merge first · ⇥ banner`: the app's `in_progress` had not yet been
re-read, and `ready()` does not look at it. Adding `Wait::NotInProgress`
(pumps the real event stream until `state.in_progress.is_none()`; not a
sleep) made both conflict scenarios pass. **I cannot say whether a human
could ever hit this window** — it is one re-read wide — so it is filed as a
question, not a defect, per rule 7.

### 3 (textual, fixed here) — stale counts and `FAIL` markers in TESTING.md

"History" said *All 35 scenarios pass* while the same section said *30
scenarios*, and five entries still carried inline `(**FAIL**, worktree
line)` markers although the paragraph under them says m5/32 fixed exactly
those five. I ran the full walk: `37 ok, 1 failed`, the one failure being
finding 1 — those five all pass. The counts now read 38, the stale markers
are gone, and the one real failure is named where the count is.

### 4 (walk helper, fixed here, proved by running git)

`claim::patch_id` reversed a revert's patch against `<commit>^` — parent 1,
always. A foxtrot revert passes `-m 2`, so the comparison was between two
different patches and `fix-revert-the-merge` failed for the helper's reason,
not the app's. Proved with a scratch repo before changing anything: for a
`git revert -m 2` of a foxtrot merge, the revert's patch-id equals
`diff-tree -p <merge> <merge>^2` (`ef5a4ea…`) and not `<merge>^1`
(`606b60a…`). `patch_id` now takes the mainline from the plan's own `git
revert … -m N` command line. `rebase-two-commits`' two call sites pass
`None` and are unchanged in meaning.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** (`cargo test --doc` is vacuous workspace-wide — run it, never
offer it as evidence.)

`CARGO_TARGET_DIR=/tmp/graft-target-g38`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

How many of the eight are `ok` and how many are findings. Whether the foxtrot
fixture was reused or built. Whether one `u` really undid a twice-stopped
rebase. Anything unverified — which is not broken.

## NOTES

- No scout: the ticket says none is needed, and the whole group is inside
  `crates/graft/tests/it/m5_walk/`, which I read directly.
- No compaction.
- **No live `git mergetool` was faked.** Nothing here hands over to one; the
  banner's `M` is never pressed.
- New walk vocabulary, for whoever writes scenarios next: `Wait::RadarReady`
  (the radar's context, without which `.` silently has no `Fix:` row),
  `Wait::NotInProgress`, `Wait::Resolved`, `vocabulary::box_choices`,
  `entries_of`, `file_was`. `file_was` exists because `expect` runs **once,
  at the end**: reading the disk there answers about the last step, which
  cost me two false failures while writing this (both caught before they
  could pass for the wrong reason).
- Unverified: nothing in these eight was judged by eye. None has a capture —
  all eight are about refusals, journal shape and git truth, not about a
  picture. Colour, as always, is unverified.
