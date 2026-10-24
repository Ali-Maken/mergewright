# 21q fix-scroll rows (read-only scout; nothing run)
PNG (ghostty_shot.rs):
- calls `pixel_rows(state,..)` directly (:186), never `TallRow::of` / `make_room` (real frame does: event_loop/graphics.rs:215, graphics/tall.rs:30). Real screen has the cursor's continuation band; the PNG has none.
- `graph_rows()` reserves one line for the second line (update/state/queries.rs:28-37) but the shot's pane = `pane_first..pane_first+graph_pane_rows()` (ghostty_shot.rs:210-211), i.e. one line MORE than history rows drawn. So 189 is a row the real pane does not draw as a history row; PNG crop = at-10 .. pane.end (:213), cursor 188 not last.
- pane_first = first `None` (uncommitted) when pinned (:204-205).
Grid: scripts/47-scroll-acceptance.sh:282-286 keeps screen rows pane_rows+1 (pane_rows from PNG), pty run, real tall layout. Cursor on last kept line (46).
Could not confirm (no run): why cursor is pane row 44 in PNG vs 45 in grid; in-process vs pty `history.top` not compared.
Script stale lines: scripts/47-scroll-acceptance.sh:291 "cut at the graph's divider"; scripts/scroll-lag-pty.py:34,301-314 divider cut (no `⋮` when graph is 2 lanes: falls through, no cut). ghostty_shot.rs:9 header "cursor's row ±10 rows" / :10-12 "one history row fewer" omit tall row.
Verdict: not just caption; PNG omits the tall-row model (ghostty_shot.rs:186,210) = capture-path defect (unproven by run).
