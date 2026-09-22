# the-walk-names-branches-and-tags

Status: ready
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

- [ ] `Step::Type` and `Wait::Prompt` exist and are used by more than one
      scenario — name two that use them.
- [ ] Seven scenarios in `refs_and_names.rs`, each `ok` from
      `cargo run -p graft --example m5_walk --only <name>` — list all seven
      with their result. **A `FAIL` is an acceptable tick if it is a finding**;
      say which and quote ≤ 20 lines.
- [ ] Each asserts truth from **git**, not `AppState` — name the git commands
      used, and confirm the only `AppState` read is `PreviewState.plan`.
- [ ] Every one is in the `walk!` list — test:
      `m5_walk::every_scenario::every_scenario_has_a_test`.
- [ ] Every one has a `TESTING.md` "History" entry saying concretely what
      right looks like, and is out of "Not yet walked" — quote one entry whole.
- [ ] `git fsck --no-dangling` clean in each — say how it is asserted.
- [ ] Slowest new scenario under 4 s, no `sleep` or poll loop — give the time.
- [ ] Whole suite green; report before and after. Baseline is set by 36 —
      read it from `docs/log/m5.md`, do not assume 2345.

Rule 6: you do not tick "looks right". The user does.

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
