# the-walk-proves-undo-and-keeps-the-password

Status: ready
Blocked by: **38** (shares `graft`, and `steps.rs`/`every_scenario.rs`).
Crates: `graft`.
Build with: **Opus.** Undo, git writes and a secret field (CLAUDE.md's model
table routes all three to Opus).
Scout: **not needed** — read `crates/graft/tests/it/m5_walk/` itself.

## Why this exists

The last of three tickets closing the **broken** gap "the acceptance walk's
remaining scenarios were never walked". Ticket 37 took refs and names, 38 took
previews and conflicts. **When this lands the walk is whole**, and the gap row
that stops M5 being called done can close.

## Piece 1 — eleven scenarios

- `mergetool-absent-refuses-in-words`
- `e-opens-editor-at-first-marker`
- `u-refused-during-an-operation`
- `journal-row-says-in-progress`
- `u-after-push-*` x3 (`LocalRemote`, with a push first)
- `esc-cancels-undo-and-rolls-back`
- `ctrl-r-twice-leaves-one-history-job`
- `password-field-is-secret`
- `hints-at-80-show-three-and-more`

**This is the largest of the three tickets.** If you compact, say so and hand
off **after the `u-after-push` trio** — that is the seam: everything before it
is conflicts-and-journal, everything after is independent. The conductor
splits the remainder.

## Piece 2 — the password scenario is the one with a hard rule

`password-field-is-secret` uses `HttpRemote` and needs its password added to
the pty script's `SECRETS` list so the script can assert the bytes never
appear in a capture.

**ADR-0038 §2 and the standing rule: a password must never reach argv, env,
the journal, a log, a capture, a panic message or a `Debug` impl.** The mask
is **eight fixed glyphs, never the typed length** — a mask that tracks length
leaks the length.

Assert all of it, and assert it from outside the app: grep the capture bytes,
grep the journal file, grep the process's argv. A test that only checks the
rendered mask proves the smallest part of this.

Note what ticket 11 already recorded and do not overstate it: with `SECRETS =
[]` its `no_secret_leaked` check "has nothing to search for — say so, not
'proven'". **You are the first run that gives it something to search for.**
Say plainly what is now proven and what is still merely unsearched.

## Piece 3 — truth comes from git, as everywhere else

`for-each-ref`, `rev-parse HEAD`, `symbolic-ref HEAD`, `write-tree` of the
index, file bytes, `.git/index` bytes, `stash list`, the journal file —
**never from `AppState`**. The one `AppState` read is the claim:
`PreviewState.plan`. Every scenario ends `git fsck --no-dangling` clean.

`u-after-push` matters most: **undo after a push is the case where undo cannot
be complete**, because the remote has it. Assert what the app actually claims
and what git actually shows, and if those disagree with what the box promised
the user, that is a finding.

## Gotchas, from ticket 11's handoff — read before you write

- `Wait::Ready` needs `remotes` Ready too, not just refs.
- `menu_pick` matches a **label prefix**; exact labels are in
  `update/context_menu/verbs.rs`. Do not guess one.
- A `See` closure **must not read `PreviewState`** — a grep test fails you.
- `ghost_equals_git` requires a **non-empty `ref_moves`**; a scenario with no
  ref move asserts refs itself. Most of the refusal scenarios have none.
- `Step::Type` / `Wait::Prompt` came from **ticket 37**; use them.

## Piece 4 — wire each one in, then close the section

Add each to the `walk!` list in `every_scenario.rs`, and its entry to
`TESTING.md` "History": **renumber, and move it out of "Not yet walked"**.
When the last one lands, **"Not yet walked" should be empty — say whether it
is.** If anything remains, list it; an empty-looking section that is not
empty is the exact shape of a false tick.

Also update `TESTING.md`'s **"What the walk proves and what it does not"**: the
real `git mergetool` hand-over is still unexercised (the user runs it), there
is still no real terminal, no real `$EDITOR`, no interactive rebase, and
forecasts are still per-commit not chained.

## Context guard

One line per scenario in your report; a failing scenario <= 20 lines. **Never
read a capture or the contact sheet back into your context** — sizes and
`head -3` only. Gate output quiet, full output only on the failing part.

## NOT yours

- **Fixing what the walk finds.** A scenario failing because the app is wrong
  is a **finding for the conductor**, filed here, left failing.
- **Running `git mergetool` for real.** Ticket 11 marked it explicitly
  not-for-builders; it needs a real terminal and the user does it.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's.
- The heavy tier, the release perf run, running in a real terminal.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] Eleven scenarios, each `ok` from `cargo run -p graft --example m5_walk
      --only <name>` — list all eleven with their result. **A `FAIL` is an
      acceptable tick if it is a finding**; say which and quote <= 20 lines.
- [ ] The password never appears in a capture, the journal, argv, env or a
      panic message — test: name it, and name **each** place searched.
- [ ] The mask is **eight fixed glyphs regardless of typed length** — test:
      name it, and say what two different-length inputs it compares.
- [ ] `u` after a push is asserted against git, and what the box promised is
      compared with what actually happened — test: name it; if they disagree,
      that is a finding, quoted.
- [ ] Every one is in the `walk!` list — test:
      `m5_walk::every_scenario::every_scenario_has_a_test`.
- [ ] Every one has a `TESTING.md` "History" entry — quote one entry whole.
- [ ] **"Not yet walked" is empty** — say so explicitly, or list what remains.
- [ ] "What the walk proves and what it does not" names the real mergetool,
      the real terminal and the real `$EDITOR` as still unexercised — quote it.
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

`CARGO_TARGET_DIR=/tmp/graft-target-g39`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

How many of the eleven are `ok` and how many are findings. What the password
test now genuinely proves versus what is merely unsearched. Whether "Not yet
walked" is empty. Whether you compacted, and where you handed off. Anything
unverified — which is not broken.
