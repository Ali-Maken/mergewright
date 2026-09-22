# the-bar-behind-an-overlay-shows-the-overlays-keys

Status: **done** (g42, 2026-09-22)
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

- [x] With an overlay shown and the Detail tab focused, every key the bar
      draws is answered — tests:
      `update::tests::the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows`
      (all three overlays × both pane focuses, at the width the Detail tab
      owns the screen) and, through a real repository and a drawn frame,
      `tests/it/hint_bar.rs::the_bar_behind_an_overlay_draws_what_the_pane_still_answers`.
      **Both fail before the fix.** The unit test reported **71 dead keys**,
      e.g. `Map (opens true) draws \`b branch\`` and then `r rebase`,
      `. more`, `t tag`, `c commit`, `y sha`, `m merge`, `q quit`,
      `F2 status`, `F3 renderer`, `x compare`, `+ lanes` — the same twelve
      for each of Map/Journal/Stash × opens true/false, minus `b` under the
      stash (whose own `b` answers). The frame test failed on the drawn row:
      `" ⏎ open  b branch  r rebase  . more  t  c  y  m  u  /  q  F2  F3  x
      +  j/k"` where it must read `" ⏎ open  j/k move  u undo  / search"`.
      The extended sweep fails too, with **1586** entries.
- [x] `screen_for` builds overlay-plus-pane states, and the existing sweep
      `every_key_the_bar_draws_is_answered` now covers them — measured:
      `every_context()` **29 → 32** contexts (`Map`,
      `PaneOverOverlay { opens: false }`, `PaneOverOverlay { opens: true }`),
      and the states the sweep walks **348 → 432** (+84). `screen_for` now
      returns a **list**, because one context can be the bar over three
      different screens: `PaneOverOverlay` is built once per overlay, since
      the three swallow different keys (the Map view eats `p`, `d`, `s`;
      the stash answers them). Counted by a throwaway probe over
      `states_for`, not by hand; probe removed.
- [x] The overlay still claims the keyboard — no handler was touched; the
      diff is `view::rows` (which context), `graph_screen::hints` (what that
      context draws) and the sweep. Tests, all green and unchanged:
      `update::journal::tests::an_open_browser_swallows_the_graphs_own_keys`,
      `update::stash::tests::an_open_overlay_swallows_the_keys_it_does_not_use`,
      and, in the exact state this ticket is about, the last assertion of
      `tests/it/hint_bar.rs::the_bar_behind_an_overlay_draws_what_the_pane_still_answers`
      (`c` with the journal open and the Detail tab focused opens no editor
      and leaves the browser up).
- [x] Captures: **nothing changed.** Captures assert on every run since
      ticket 40, and the whole suite is green with no capture blessed —
      including `hint_bar::capture_hint_bar_for_docs_shots`. No committed
      capture is of an overlay with the Detail tab focused, so no grid
      moved; `docs/shots/` is untouched by this ticket.
- [x] Whole suite green: **before 2436 passed / 1 skipped** (docs/log/m5.md
      line 1167), **after 2438 passed / 1 skipped** — the two new tests.
      `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`
      and `cargo test --doc` all pass, workspace-wide, run after `git add -A`.

Rule 6: you do not tick "looks right". The user does.

## NOTES (for the conductor)

**§2.1 could be honoured, but not by drawing the overlay's verbs — and the
difference is load-bearing.** The bar behind an overlay exists in exactly one
state: the sub-100-column `Tab` layout with the **Detail tab shown**
(`update::detail::control::tab_hint_rows` returns a row only for
`DetailLayout::Tab { shown: true }`; every other layout draws the overlay's
own footer instead). In that state the overlay does **not** have focus — the
detail pane does — and `update::keys::pane_and_overlay_key:179` sends the key
to `detail_key` *before* the overlay whenever the pane has focus. Measured
with a throwaway probe (not committed), with journal/stash/map open and the
pane focused:

- `⏎` moves focus to the diff (`detail::keys:96`); with the diff focused it
  is consumed doing nothing. The stash's `⏎ apply` and the journal's
  `⏎ revert` never run.
- `Esc` closes the **pane** (`detail::keys:110`), leaving the overlay open.
  `esc close` would name the wrong thing.

So drawing the overlay's list there would have replaced nine dead keys with
two keys that do something other than what they say — the failure mode the
ticket's own "a hint that lies is worse than no hint" rules out. §2.1 says
the hint bar shows *what's active*, and its modal rule is conditioned on the
modal pane **having focus**; here the pane has it. What the bar now draws is
what that state actually answers: `⏎ open` (unless the diff has focus),
`j/k move`, `u undo`, `/ search` — the last two because `key_event`
dispatches undo and search above `pane_and_overlay_key`. No new binding, no
key taken from anything, no handler changed.

Rows for the conductor, none of them touched here:

1. **A Map hint context now exists** (`HintContext::Map`, from keymap §2.3a:
   `⏎ seek`, `j/k move`, `esc close`) so the classifier has an answer for
   the Map overlay, the way `Journal` and `Stash` already had one. It added
   one `bound_actions` row, `("⏎", "seek")` — the binding is m2's
   `map::seek_selected`, not new. Like `Journal`/`Stash` it is not drawn on
   screen today (the Map draws its own footer, which carries no keys); it is
   what the sweep presses and what the M8 help overlay will read.
2. **`docs/design/02-keymap.md` §2.1 lists five modal panes and the three
   overlays are not among them**, though their `shown_key`s claim the
   alphabet exactly like a modal pane. The doc's own rule says a future
   overlay that needs letters "becomes a sixth modal pane by amending the
   list above, not by quietly claiming them". That amendment has never been
   made. A doc row, not a code defect — the conductor's call.
3. **Under-draw, knowingly:** with the **changes** pane focused under an
   overlay, `space stage`, `d discard`, `⌃A all` and `e edit` are live
   (`detail_key` answers them first) and the new bar does not draw them —
   `PaneOverOverlay` is one list for any focused pane. Nothing dead is
   drawn; this is the "which live keys deserve a slot" question the ticket
   assigns to M8.
4. **The commit editor** is the fourth `GraphOverlay` and is deliberately not
   classified: it claims every key but `Ctrl+C` before the panes, and it
   draws no hint bar to be wrong on (`view::frame::draw_commit_editor`
   refuses the `Tab` layout; the split layout draws no hint row). If a bar
   is ever drawn under it, it is the same defect again — named in
   `view::rows::overlay_context`'s doc so the next reader sees it.
5. **Unverified, not broken:** colour of the new bar (rule 5 — the pty
   captures carry no attributes), and how the short bar *looks* at 40
   columns (rule 6 — no capture of this state is committed, and this ticket
   adds none; the four hints measure 34 cells, so they fit, but "looks
   right" is the user's tick).

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
