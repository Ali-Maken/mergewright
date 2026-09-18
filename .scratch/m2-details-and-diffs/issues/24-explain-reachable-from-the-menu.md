# explain-reachable-from-the-menu: the `.` menu never offers Explain, so the Explain panel is unreachable

Status: ready
Blocked by: 12, 17, 23
Build with: Sonnet — a wiring defect against specs that already exist (tickets 12 and 17); no new decision.

Filed by the conductor, 2026-09-18, from ticket 23's pty walk. **Not new scope:** tickets 12 and 17 already specify this behaviour; the acceptance walk found it doesn't happen.

## The finding, verbatim from ticket 23's report

> **`.` menu never lists Explain** (`context_menu.rs` `marked = false`) — Explain panel unreachable by keys; "and explained" half of the promise fails.

M2's promise is *what a commit changed is readable without leaving the graph, and a bad merge is flagged and explained.* Ticket 17 built the Explain panel and its `open_explain`/`close_explain`/`escape_priority`; ticket 12 built the context menu that is supposed to reach it. The two never met: the menu is built with the commit's "marked" state hardcoded false, so the Explain entry is filtered out on every commit, including ones the radar has flagged.

## Scope

In:
- Find the actual cause in `crates/graft/src/view/context_menu.rs` (and whatever feeds it) — why the marked/finding state is not reaching the menu. Fix the cause, not the symptom: the menu must ask the same source of truth the marker column asks (`findings_for` / `marker_for`), not a second hand-maintained flag.
- The Explain entry appears when the commit under the cursor has at least one finding, and does not appear when it has none.
- Tests: menu on a flagged commit lists Explain; menu on a clean commit does not; choosing it opens the panel ticket 17 built.
- Refresh whichever `docs/shots/` capture shows the menu (`m2-context-menu.txt`, and ticket 23's gallery stop if its stem covers the menu).
- Read `.scratch/m2-details-and-diffs/issues/12-context-actions-menu.md` and `17-explain-panel.md` Acceptance sections only, and ticket 23's TESTING.md entry for the Explain stop.

Out:
- Any change to what Explain renders (ticket 17's, done).
- The other findings ticket 23 recorded (no 2-file commit in the corpus, compare header counts, stop 1's opening row) — those are KNOWN_GAPS rows, not this ticket.

## Acceptance

- `.` on a foxtrot/evil merge lists Explain; pressing it opens the Explain panel; `Esc` closes it per ticket 17's priority order.
- `.` on an unflagged commit does not list Explain.
- Gates green, capture refreshed, KNOWN_GAPS row for this defect removed if ticket 23 added one.
