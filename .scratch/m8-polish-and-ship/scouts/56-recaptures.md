# Scout 56 — mwr recaptures

## Writers
- TestBackend/check_shot tests: `graft-fixtures` capture mod, env `MWR_UPDATE_SHOTS` (crates/graft-fixtures/MAP.md:45-49). Examples: crates/graft-ui/tests/resolver_layout.rs:24, crates/graft-ui/src/graph_column/tests.rs:718, crates/graft-ui/src/journal_screen/tests/source_column.rs:46. Out of scope per ticket.
- pty python: docs/shots/37-pty-drive.py, 37-pty-frames.py, 39-capture.py (`<repo> <cols> <rows> <keys> <out> <note>`), 40-capture.py (`... <out.png>`); 37/39 import scripts/mwr_sandbox.py `sandbox_env`.
- Shell: scripts/m8-accept.sh (fast tier with `env -u MWR_UPDATE_SHOTS`, :52; sheet :75).
- Drift: scripts/shot-drift.py `--base <rev> --out <file>`; name-only = old line matches `(?i)graft` only (header :1-15, OLD regex). Tributary/trib are NOT counted name-only.
- Contact sheet: scripts/contact-sheet.sh `[--captions-order] <milestone> [<shots-dir>]`.

## Lists
- 27-shots-left.txt: 450 paths; 41 contain "m8".
- 27d-classified.txt: 443 rows — 244 reshoot, 199 code-name; 36 rows mention m8.

## Old names in docs/shots now (grep -rliE 'graft|tributary|\btrib\b')
- 436 of 434+subdir files match any case; 94 match capitalised `Graft|Tributary`.
- docs/shots/m8/: 30 files, 21 match; only 2 capitalised (pane-over-map-bar-80x24, polish-whitespace-120x36).
- 83 docs/shots files added since 2026-10-02 (git log --diff-filter=A --since).
- README live-doc link: README.md:15 -> docs/shots/m8/fork-follows-banner-187x44.txt.

## m8/ matches -> writer (by basename grep)
Writer is a Rust test (TestBackend/it walk) for 18: look_polish.rs (4), hint_bar.rs (3), update/tests/doorway.rs (5), search_hint.rs, detail_pane/tests.rs, m7_walk/switch.rs, m7_walk/amend_into.rs, partial_fetch.rs, view/tests/pixel_preview.rs.
No writer found: 21c-journal-partial-fetch-120x40, 21c-journal-partial-fetch-80x24, doorway-marked-140x40.

## Contradictions
- Ticket cites `docs/log/m8-polish-and-ship.md`: does not exist; log is docs/log/m8.md, "M8 start f5080f50" at :27.
- Most m8/ writers are Rust tests, which ticket Scope puts out ("TestBackend captures").
- shot-drift name-only only knows `graft`, not Tributary/trib.
- Could not confirm: which matches are code-name vs product name per file; per-file add commit vs f5080f50.
