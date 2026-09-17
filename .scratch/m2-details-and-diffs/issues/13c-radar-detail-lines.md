# radar-detail-lines: the detail pane's radar lines, and the marker capture

Status: done
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

## Done (2026-09-18)

- `detail_pane.rs::radar_lines_text` joins every `RadarLine` (not just the
  first) into the header's one reserved radar-line slot with " · ";
  `radar_text` covers all six variants including the exact
  `EvilNotCheckable`/`ForcePushNotCheckable` wording (`"not checkable — "` +
  reason, em-dash). Three new tests in `detail_pane/tests.rs`: one per
  variant's wording, one for the join, one for the header row.
- `view::detail::commit_header` now calls `view::radar::radar_lines_for`
  (previously dead code) — `#[allow(dead_code)]` removed.
- Capture: `docs/shots/m2-marker-column.txt` (1978 bytes), a foxtrot-merge
  fixture — marker (`⚠`) on the merge row, both radar lines joined in the
  pane header. Regenerated from a one-off `#[ignore]`d test that was
  removed after use (same precedent as `m2-compare.txt`); the header
  comment in the capture file documents the fixture for reproduction.
- Also finished the one 13b item left open: `[radar]` config now reaches
  `AppState.radar.policy` via `event_loop::startup::seed_config`. See
  `KNOWN_GAPS.md` m2-details-and-diffs/13b (now resolved) and
  `startup::tests::a_non_default_radar_setting_reaches_app_state_policy`.
- Gates green: `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test`. Session compacted mid-ticket (context ran out);
  resumed from a summary and finished the capture/cleanup/gates from there.
