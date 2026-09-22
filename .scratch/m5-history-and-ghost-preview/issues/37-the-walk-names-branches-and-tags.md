# the-walk-names-branches-and-tags

Status: done
Blocked by: **36** (shares `graft`).
Crates: `graft`.
Build with: **Sonnet.** The harness exists and 23 scenarios already show the
pattern; this adds two `steps.rs` primitives and seven scenarios to it.
Scout: **not needed** — read `crates/graft/tests/it/m5_walk/` itself; the
handoff note below names every file and every gotcha.

## Why this exists

Ticket 11 built the M5 acceptance walk and got 23 scenarios in before it
compacted three times. `KNOWN_GAPS.md` carries the remainder as **broken** —
*"an unjudged acceptance group is a gap and the milestone is not done over
one"* (CLAUDE.md). This is the first of three tickets that close it.

**The 23 are all green** (`cargo nextest list -E 'test(m5_walk)'` → 28, of
which 5 are meta-tests). The five that ticket 11 left deliberately failing
were ticket 32's specification and are fixed.

**This ticket is the refs-and-names seam.** Tickets 38 and 39 take previews
and conflicts, and undo/push/secrets. They touch the same files, so they run
after you, not beside you — **do not start on their scenarios**, even if one
looks quick.

## Piece 1 — two harness primitives

`tests/it/m5_walk/steps.rs` needs `Step::Type` re-added (ticket 11 removed
it) and a `Wait::Prompt`. Everything in piece 2 types a name into a prompt,
so build these first and build them once.

## Piece 2 — seven scenarios

New file `tests/it/m5_walk/refs_and_names.rs` (the M4 split: scenarios grouped
by promise, largest file under 400 lines):

- `b-creates-and-checks-out`
- `b-refuses-bad-and-existing-names`
- `t-creates-a-tag`
- `delete-branch-then-u-restores`
- `delete-tag-then-u-restores`
- `stash-b-branches-from-stash`
- `move-a-branch-not-checked-out`

Each one: a corpus shape copied to scratch (`copy_into_temp`) plus scratch
commits via `graft_fixtures::git::run`; the real app driven headless through
`Harness::prelude` over the real pool; and **truth read back by running git** —
`for-each-ref`, `rev-parse HEAD`, `symbolic-ref HEAD`, file bytes, the journal
file — **never from `AppState`**. Every scenario ends `git fsck --no-dangling`
clean.

The **one** thing read from `AppState` is the claim: `PreviewState.plan`.

## Gotchas, from ticket 11's handoff — read before you write

- `Wait::Ready` needs `remotes` Ready too, not just refs.
- `menu_pick` matches a **label prefix**; the verbs' exact labels are in
  `update/context_menu/verbs.rs`. Do not guess a label.
- A `See` closure **must not read `PreviewState`** — there is a grep test that
  fails you for it.
- `ghost_equals_git` requires a **non-empty `ref_moves`**, so a scenario with
  no ref move asserts refs itself. Several below have none.

## Piece 3 — wire each one in

Add each to the `walk!` list in `every_scenario.rs`, and its entry to
`TESTING.md` "History": **renumber, and move it out of "Not yet walked"**.
Each entry says **concretely what right looks like** — exact box lines, status
strings, `for-each-ref` relations, byte equalities. Never "looks correct".
Quote only what a `See`/`expect` actually asserts.

## Context guard

One line per scenario in your report; a failing scenario ≤ 20 lines. **Never
read a capture or the contact sheet back into your context** — sizes and
`head -3` only. Gate output quiet, full output only on the failing part.

## NOT yours

- **Fixing what the walk finds.** A scenario that fails because the app is
  wrong is a **finding for the conductor**, filed in this ticket, not fixed
  here. Leave it failing and say so — that is what ticket 11 did, and ticket
  32 was built from it. Do not make a red test green by weakening it.
- Tickets 38 and 39's scenarios.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's.
- The heavy tier, the release perf run, running in a real terminal.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] `Step::Type` and `Wait::Prompt` exist and are used by more than one
      scenario — `Step::Type` is used by all seven new scenarios;
      `Wait::Prompt` is used by `b-creates-and-checks-out`,
      `t-creates-a-tag`, and `stash-b-branches-from-stash`. Verified by
      reading `crates/graft/tests/it/m5_walk/refs_and_names.rs` and by
      `cargo run -p graft --example m5_walk --list` including all seven.
- [x] Seven scenarios in `refs_and_names.rs`, each `ok` from
      `cargo run -p graft --example m5_walk --only <name>`:
      `b-creates-and-checks-out` ok, `b-refuses-bad-and-existing-names` ok,
      `t-creates-a-tag` ok, `delete-branch-then-u-restores` ok,
      `delete-tag-then-u-restores` ok, `stash-b-branches-from-stash` ok,
      `move-a-branch-not-checked-out` ok. No finding this ticket — the app
      behaved as designed in all seven; nothing was weakened to pass.
- [x] Each asserts truth from **git**: `for-each-ref` (via `truth()`'s
      `repo.refs`), `symbolic-ref HEAD` (`head_symbolic`), `cat-file -t`
      (lightweight-tag check in `t-creates-a-tag`), `stash list`
      (`repo.stash`, in `stash-b-branches-from-stash`), the journal file
      (`repo.journal`/`last_entry_committed`), `git fsck --no-dangling`.
      The only `AppState` read is `PreviewState.plan` via `Step::Claim` (in
      `delete-branch-then-u-restores`, `delete-tag-then-u-restores`,
      `stash-b-branches-from-stash`, `move-a-branch-not-checked-out`) and
      `state.name_prompt`/`name_refusal(state)` in `Step::See` closures
      (never `PreviewState`) for the two create scenarios and the refusal
      scenario, which take no preview at all. Test:
      `m5_walk::every_scenario::ghost_equals_git_is_the_only_claim_check`
      (now covers `refs_and_names.rs` too — added to its file list).
- [x] Every one is in the `walk!` list — test:
      `m5_walk::every_scenario::every_scenario_has_a_test` (passed under
      `cargo nextest run -p graft -E 'test(m5_walk)'`, 35/35).
- [x] Every one has a `TESTING.md` "History" entry saying concretely what
      right looks like, and is out of "Not yet walked". Quoting #30 whole:
      "**move-a-branch-not-checked-out** — `forked_on_main`; drag
      `feature`'s badge (not current) onto `main`'s row: the drop menu
      offers `Move feature to here`, not a `Reset` row; pick it, confirm:
      `ghost_equals_git`, `refs/heads/feature` equals `main`'s old commit,
      `HEAD` stays on `main`, the tree untouched, one `Reset` entry; `u`:
      `feature` back."
- [x] `git fsck --no-dangling` clean in each — asserted by
      `run_scenario` (`crates/graft/tests/it/m5_walk/run.rs`) calling
      `repo_truth::fsck_clean` after every scenario's `expect`, unconditionally,
      for all 30 scenarios including the 7 new ones.
- [x] Slowest new scenario under 4 s, no `sleep` or poll loop — timed each
      via the standalone example binary: b-creates-and-checks-out 1450ms
      (includes example-binary link/startup on first invocation),
      b-refuses-bad-and-existing-names 401ms, t-creates-a-tag 457ms,
      delete-branch-then-u-restores 597ms, delete-tag-then-u-restores
      595ms, stash-b-branches-from-stash 761ms,
      move-a-branch-not-checked-out 615ms. No `sleep`; every wait is
      `Walk::pump` against a real predicate.
- [x] Whole suite green; report before and after. Baseline from
      `docs/log/m5.md` (g42's entry): **2438/2438, 1 skipped**. After:
      **2445/2445, 1 skipped** (`cargo fmt --all --check`,
      `cargo clippy --all-targets --message-format=short -- -D warnings`,
      `cargo nextest run`, `cargo test --doc`, all workspace-wide, run
      after `git add -A`).

Rule 6: you do not tick "looks right". The user does.

## NOTES (builder)

- **Third primitive beyond the ticket's named two.** Piece 1 named
  `Step::Type` and `Wait::Prompt`. Building `stash-b-branches-from-stash`
  showed a third was needed: `S` dispatches `Command::ReadStashes`
  asynchronously (`update/stash.rs`, `StashState::read`), and no existing
  `Wait` pumps for it — `Wait::Ready`'s predicate never looks at
  `AppState::stash`. Pressing `b` before the list lands is a no-op
  (`open_branch_prompt` returns early when `selected_entry()` is `None`),
  so `Wait::Prompt` right after it would hang to the 120s timeout. Added
  `Wait::StashReady` (pumps on `matches!(state.stash.entries,
  Loading::Ready(_))`), documented in `steps.rs` with the reasoning above.
  Flagging this rather than silently exceeding the "two primitives" scope.
- **Gotcha not needed this ticket:** `ghost_equals_git` needs non-empty
  `ref_moves` — true for `delete-branch-then-u-restores`,
  `delete-tag-then-u-restores`, `stash-b-branches-from-stash`,
  `move-a-branch-not-checked-out` (all take `Step::Claim`).
  `b-creates-and-checks-out`, `t-creates-a-tag` and
  `b-refuses-bad-and-existing-names` take no `Claim` at all — `b`/`t` go
  straight to `Command::Operate` with no preview (a create moves no
  history), and the refusal scenario writes nothing.
- No captures added (not in this ticket's acceptance; `M5_STEMS` /
  contact-sheet machinery untouched).
- Not mine, untouched: `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/`.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** (`cargo test --doc` is vacuous workspace-wide — run it, never
offer it as evidence.)

`CARGO_TARGET_DIR=/tmp/graft-target-g37`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

How many of the seven are `ok` and how many are findings. Anything the
harness could not express. Anything unverified — which is not broken.
