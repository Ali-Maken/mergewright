# explain-reachable-from-the-menu: the `.` menu never offers Explain, so the Explain panel is unreachable

Status: done
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

## Build notes (Sonnet, 2026-09-18)

Cause fix, not a workaround: `update::context_menu::open_for_history_row`
was hardcoding `let marked = false;`. Now it looks the cursor row up in
`next.history`, then calls `crate::view::radar::findings_for` (the same
function `view/entries.rs`'s marker column and `view/strip.rs`'s strip
call) with that row's `(id, parents, subject)`, and lists `Explain` iff
it returns anything non-empty. No second flag, no `GraphEntry::marker`
duplication.

Also fixed: the gallery's own banner text for stop 7
(`crates/graft/tests/it/gallery.rs`, the `foxtrot-merge · .` entry) still
said Explain was absent — updated to say it leads, and reran the ticket
23 pty script for stop 7 at all four of its sizes (`docs/shots/m2-07-
foxtrot-merge-dot-{80x24-ascii,80x24-unicode,100x30-unicode,160x40-
unicode}.txt`), all now showing `Explain` first in the menu on the
flagged foxtrot merge. `docs/shots/m2-context-menu.txt`'s header comment
updated to say the caller now computes `marked` for real rather than
faking it (its two rendered variants are unchanged — that capture drives
`ContextMenu::for_row` directly to show both renderings side by side).

Removed the now-stale KNOWN_GAPS row (M2 acceptance walk table) and
TESTING.md's stop-7 gap note; TESTING.md's stop 7 entry rewritten to
describe the fixed menu.

Tests added: `open_for_history_row_lists_explain_when_the_row_has_a_finding`,
`open_for_history_row_omits_explain_on_an_unflagged_row`,
`open_for_history_row_omits_explain_without_a_ready_radar_context`
(replacing the old test that asserted the bug). Fixture:
`crate::view::tests::history_of(3, 0)` + a `duplicate_findings` entry,
same shape `update::detail`'s explain tests already use.

Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` all green (workspace). No compaction, no force-summarise.
