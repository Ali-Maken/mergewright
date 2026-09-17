# radar-detail-lines: the detail pane's radar lines, and the marker capture

Status: ready
Blocked by: 13b
Build with: Sonnet — rendering against `RadarLine` variants already defined; no new decisions.
Needs: `radar-wiring` (`radar_lines_for`, `RadarState`), `detail-pane-frame` (`DetailPane`, `RadarLine` slot)

## Scope

Items 5 and 6 of ticket 13's handoff list:

- `crates/graft-ui/src/detail_pane.rs`: render all six `RadarLine` variants (including `EvilNotCheckable`), not just the first.
- Capture `docs/shots/m2-marker-column.txt` (≤200 KB): a fixture with findings, showing the marker column and the pane's radar lines. User judges by eye.

Out: probe results (tickets 14–16) and the Explain panel (17).

## Acceptance

Ticket 13's acceptance rows for the detail pane's radar lines. Every variant has a rendering test; the capture shows a marker in the graph and its line in the pane.
