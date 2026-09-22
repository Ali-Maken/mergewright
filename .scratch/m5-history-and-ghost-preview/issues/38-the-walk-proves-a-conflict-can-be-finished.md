# the-walk-proves-a-conflict-can-be-finished

Status: ready
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

- [ ] A foxtrot fixture exists and the `fix-*` trio runs on it — say whether
      it reuses the radar shape or is new, and why.
- [ ] Eight scenarios, each `ok` from `cargo run -p graft --example m5_walk
      --only <name>` — list all eight with their result. **A `FAIL` is an
      acceptable tick if it is a finding**; say which and quote <= 20 lines.
- [ ] A rebase that stops twice leaves **one** journal entry and **one** `u`
      undoes it — test: name it, and quote the journal assertion.
- [ ] Each asserts truth from **git**, not `AppState` — name the git commands,
      and confirm the only `AppState` read is `PreviewState.plan`.
- [ ] Every one is in the `walk!` list — test:
      `m5_walk::every_scenario::every_scenario_has_a_test`.
- [ ] Every one has a `TESTING.md` "History" entry and is out of "Not yet
      walked" — quote one entry whole.
- [ ] `git fsck --no-dangling` clean in each — say how it is asserted.
- [ ] Slowest new scenario under 4 s, no `sleep` or poll loop — give the time.
- [ ] Whole suite green; report before and after. Read the baseline from
      `docs/log/m5.md`; do not assume a number.

Rule 6: you do not tick "looks right". The user does.

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
