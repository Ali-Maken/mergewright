# m1-the-graph/36 — Pinned tooltip shows the wrong commit after a re-sort

Status: draft — **proposed wontfix** (M1 graph design pass, 2026-09-13): ADR-0023 removes the tooltip and the pin, so there is no row-keyed pin to re-key. Close as wontfix when the user approves ADR-0023 (ticket 41 removes the code and closes the KNOWN_GAPS row). If ADR-0023 is rejected, this ticket stands as written below.
Build with: **Opus** (high) — state that must survive the ADR-0008 promotion swap
Blocked by: user's decision on ADR-0023
Leaves runnable: yes — a tooltip pinned with Enter keeps showing the commit it was pinned on, through promotion, refresh and scrolling.

Why: found in 27c. The pinned tooltip (34, ADR-0020) stores a row number. When the promotion swap (ADR-0008) re-sorts history, that row holds a different commit, so the pin silently shows the wrong commit's hash, author and message. **User decision (2026-09-13): a correctness bug, fixed in M1, not carried to the review.**

## Read first
- Ticket 34 "User decisions" and its notes; ADR-0020; ADR-0008 (decision and consequences)
- Ticket 27c result notes (what else the swap re-keys: jump, seek, selection)
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`; scouts for source (the pin state, `promoted()` in `graft/src/history/scroll.rs`)

## Scope
In:
- Test first: pin a tooltip, trigger the promotion swap (and Ctrl+R refresh), assert the pin still shows the same commit id; red before the fix.
- Fix the cause: the pin is keyed by commit id, not row. Decide what happens if the pinned commit is no longer in history (e.g. after refresh drops it): unpin visibly — never show a different commit.
- Check the same class across the swap: selection and hover by row. Fix any other row-keyed state that shows a wrong commit; list what you checked.
- Remove the KNOWN_GAPS row for this bug.

Out: new tooltip content or layout.

## Acceptance
- [ ] Test red before, green after (swap and refresh).
- [ ] Pinned commit gone → pin cleared, no wrong commit shown (test).
- [ ] Other row-keyed state checked and listed.
- [ ] pty capture under `docs/shots/36-*` of a pin surviving a swap if feasible; user judges by eye.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Handoff (2026-09-13, stopped on conductor order: tooltip design pass)

No code written; nothing stashed (tree was clean). Findings only.

- **Cause confirmed:** `AppState::pinned_tooltip: Option<usize>` (`graft/src/update.rs` ~185) is a row number. `toggle_pinned_tooltip` stores `history.cursor`; `view.rs` `tooltip_view` reads it with `history.row_at(row)`. `History::promoted()` (`history/scroll.rs` ~130) re-anchors `cursor`/`top` by commit id but nothing re-keys the pin. Refresh (`AppEvent::Started(_, Task::ReadHistory)`, update.rs ~366) builds a fresh `History::started()` and also keeps the row number.
- **Other row-keyed state:** hover `AppState::tooltip` is a row under the pointer, so after a swap it shows the commit drawn there (correct by definition, not this bug). Selection survives promotion by anchor (27c). Selection is not carried across Ctrl+R (fresh `History`): resets to the top, not a wrong commit, but worth a look.
- **Open question for the fix:** `row_at` only sees the window on hand, so a pin keyed by `Oid` is found only while its row is in the window. "Commit gone" can't be told apart from "scrolled away" without a full-history membership check. Options: (a) pin stores `Oid` plus a `Commit` snapshot (renders anywhere; clear on refresh once the new walk is `Complete` without the id, or clear on every refresh); (b) `Oid` only, looked up in the window like `reanchor`.
- **Next step if the tooltip stays:** red tests in `update.rs` beside `moving_the_cursor_does_not_move_a_pinned_tooltip` (pin, feed a promoted `Window` in which row 1 holds another commit, assert the tooltip hash is unchanged; same for a refresh), then change the field to an id and choose (a) or (b). Remove KNOWN_GAPS row "A pinned tooltip points at a row number" when fixed.
