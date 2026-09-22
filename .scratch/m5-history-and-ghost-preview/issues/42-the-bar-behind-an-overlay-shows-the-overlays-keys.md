# the-bar-behind-an-overlay-shows-the-overlays-keys

Status: ready
Blocked by: **41** (shares `graft`, `graft-ui`).
Crates: `graft`, `graft-ui`.
Build with: **Opus.** Key routing across a modal boundary; getting it wrong
either re-opens the dead-key class or makes an overlay leak keys to the graph.
Scout: **not needed** — ticket 36 verified the defect empirically and named
the mechanism; the design answer is quoted below.

## The defect

Ticket 36 investigated the review's Q1 and found it is **real**, not a
question. With an overlay shown (Map, Journal, Stash) and the **Detail tab
focused**, the pane bar draws `b t c m r y S J /` — and **every one of them is
dead.** The overlay's `shown_key` claims the keyboard before `graph_key` runs.

Verified with a throwaway probe, not committed. The bar is drawn from
`pane_hint_context(state)` at `crates/graft/src/view/frame.rs:415`, which
returns the graph's context regardless of the overlay.

**This is the same family as ticket 33's `<> resize`** — a bar advertising keys
nothing will answer — and that one was fixed immediately rather than deferred,
because a hint that lies is worse than no hint: the user cannot learn the
difference from the bar.

## The design already decides this — it is not an open call

Ticket 36 stopped and reported because it looked like a design question. It
is not. `docs/design/02-keymap.md` §2.1 settles it:

> **A pane dense enough to need the alphabet for its own vocabulary is modal:
> it answers every key itself, `Ctrl+C` excepted, and nothing outside it is
> live while it has focus. The hint bar always shows what's active, so this
> isn't a hidden mode.**

An overlay is exactly that pane. So:

- **Not** "route the graph's keys through". That would make the overlay
  non-modal, contradicting §2.1 and reopening every collision the modal rule
  exists to prevent.
- **Yes** — the bar shows **the overlay's** verbs while the overlay has the
  keyboard. §2.1's "the hint bar always shows what's active" is the promise
  being broken here, and honouring it is the fix.

CLAUDE.md: shared namespaces are decided once, in the design, not per
milestone. This is that decision being applied, not a new one. **If you find a
case where §2.1 genuinely cannot be honoured, stop and report** — do not
invent a third behaviour.

## What to build

1. The Detail tab's bar, with an overlay up, draws the **overlay's** context.
2. **Extend the sweep so it can reach this state.** `screen_for`
   (`crates/graft/src/update/tests.rs`) has no overlay-plus-pane state, which
   is the only reason ticket 31's sweep passed a bar full of dead keys. Ticket
   31's invariant is right; its coverage had a hole exactly where the defect
   was. Adding the state is most of the value of this ticket.
3. Ticket 31's rule then holds here by the existing test — **every key the bar
   draws, in any context, at any width, is answered by the handler that has
   focus in that state.**

## NOT yours

- The three **bar-content** gap rows (`space fold`, `1 first-parent`,
  `a amend`) — those are "which live keys deserve a slot", an M8 pass. This
  ticket is about keys that are **dead**, which is a defect now.
- The acceptance walk (37, 38, 39).
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's. Report rows.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] With an overlay shown and the Detail tab focused, every key the bar
      draws is answered — test: name it. **Must fail before the fix**; quote
      the dead keys it reports.
- [ ] `screen_for` builds overlay-plus-pane states, and the existing sweep
      `every_key_the_bar_draws_is_answered` now covers them — say how many
      new states it walks.
- [ ] The overlay still claims the keyboard — i.e. the fix changed **what the
      bar draws**, not what the overlay answers — test: name it.
- [ ] Captures regenerated if any grid changed — and note that captures now
      assert on every run (ticket 40), so say whether any changed at all.
- [ ] Whole suite green; report before and after. Baseline is set by 41 — read
      it from `docs/log/m5.md`, do not assume a number.

Rule 6: you do not tick "looks right". The user does.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.**

`CARGO_TARGET_DIR=/tmp/graft-target-g42`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

How many states the sweep gained. Whether §2.1 could be honoured everywhere.
Anything unverified — which is not broken.
