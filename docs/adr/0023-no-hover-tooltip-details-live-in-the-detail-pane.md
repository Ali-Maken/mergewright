# No hover tooltip: the subject reveals in the footer, details live in the detail pane, `y` copies the SHA

Status: **Proposed** (planner-fable, 2026-09-13, evaluating the user's own
proposal; needs the user's approval at the M1 graph design checkpoint).
Supersedes **ADR-0020**. Changes PROMPT.md §1.3 ("**Hover** shows a tooltip
with the full message, parents and containing branches"), DESIGN.md §2.15's
hover line and §2.3's `Enter` (m1-the-graph/17, 34).

## Context

In Ghostty on git/git the tooltip covered two thirds of the width, hid the
subject column, had no border and no visible way to close, and a pinned one
showed the wrong commit after the promotion re-sort (ticket 36). The user
proposed removing it rather than fixing it.

## Evaluation

The proposal is right, for reasons beyond the bugs:

- PROMPT.md §1.1 and §1.3 already put the full message, parents and
  "contained in" in the **detail pane** (M2). The tooltip was a second home
  for the same facts, one milestone early, and every fix to it is work the
  pane makes redundant.
- It covers the graph — the thing M1 is about — and a TUI popup fights
  Shift+drag text selection, which is how terminal users copy anyway.
- Its one correctness bug (36) exists only because a popup has state. Delete
  the state, delete the bug.
- The cost is real and bounded: until M2, Graft shows no commit body and no
  parents. M2 is the next milestone and the spec's home for both.

## Decision

1. **No tooltip.** Hover and `Enter` popups go: `graft_ui::graph_screen::
   Tooltip`, `GraphView::tooltip`, `AppState::tooltip`/`pinned_tooltip`,
   `view::tooltip_view`, `columns::tooltip_date`, and ratatui's
   `unstable-rendered-line-info` feature (only the box height used it).
   `absolute_date` stays for M2's pane.
2. **The footer reveals a truncated subject.** While the pointer rests on a
   row whose subject is cut with `…`, the hint line shows the full subject
   (sanitised, one line, cut at the terminal's width). The pointer leaving
   the row brings the hints back. A subject that fits triggers nothing.
3. **`Enter` peeks, one-shot.** On a cursor row with a truncated subject,
   `Enter` shows the same footer line; any other key clears it. This keeps
   the reveal reachable without a mouse (PROMPT.md §5). M2 rebinds `Enter`
   to open the detail pane (DESIGN.md §2.2 "drill in") and drops the peek.
4. **`y` copies the full SHA** of the selected commit to the terminal
   clipboard through OSC 52 (`ESC ] 52 ; c ; base64 ESC \`, wrapped for
   tmux by `Transport`). The footer says `<short sha>… → clipboard (osc 52)`
   until the next key. Graft cannot observe whether the terminal honoured
   the sequence, so the notice says what Graft sent, not that it worked.
   Copy SHA is also listed under `.` context actions once that menu exists
   (M2+), per DESIGN.md §2.1. `y` is free in §2.2/§2.3; vim's yank and
   lazygit's `y` make it the expected key.
5. **Ticket 36 is wontfix**: no pin, no pinned state to re-key.

## Consequences

- PROMPT.md §1.3's hover sentence is replaced by: "Hovering a truncated
  subject reveals it in the hint bar; the detail pane holds the full
  message, parents and containing branches." DESIGN.md §2.15 hover line
  likewise; §2.3 `Enter` → "peek (M1) / open details (M2)"; §2.3 gains `y`.
- KNOWN_GAPS: the pinned-tooltip row closes; the "contained in" row points
  at the detail pane; the pty-assertion suggestion no longer reads the
  cursor from the tooltip.
- `docs/shots/17-*` and `34-*` become historical; TESTING.md rows that
  mention the tooltip are rewritten by ticket 41.
- Until M2 lands, `KNOWN_GAPS.md` carries: "no commit body or parents on
  screen; M2 detail pane."
