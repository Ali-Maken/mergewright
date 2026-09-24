# singular-and-m6-09-keys: `1 conflicts`; m6-09's capture shows a stray refusal

Status: done
Blocked by: none (45 merged `e6d3d43b`).
Was: filed by the conductor 2026-09-24 from ticket 45 NOTES findings 2 and 3 (line checked by the conductor).
Build with: **Opus** (low).
Needs: none.
Crates: graft-ops (1), graft tests + scripts + docs (2).

## Scope
In:
1. **Singular.** `graft-ops/src/operation.rs:647-649`, `ConflictsOpen` → `"{open} conflicts still open in …"`, so the bottom row reads `1 conflicts still open in both.txt`. Failing test first (`open == 1`). Fix it, then grep the workspace for every other drawn `{n} conflicts` / `{n} files` / `{n} blocks` phrase with no singular. Fix the ones the user sees, same rule, each with a test. List them in NOTES.
2. **m6-09 keys.** The capture's keys `k ⏎ j j j j ⏎ d` carry a `⏎` that has been refused since ticket 17 (`j` moves files in the resolver), so the frame's bottom row shows a refusal, not the box's keys (`←/→ move  ⏎ choose  esc cancel`). Change the capture keys, tests only, so the journey reaches the box the way a user would today. Retake at 120x40 and 80x24, fix the captions, and re-run the m6 contact sheet (parts ≤ 200 000 B).
3. Any other capture whose words change because of 1: regenerate it by its own test and list it in NOTES.

Out: app behaviour other than the wording.

## Acceptance (each tick names its test, at tick time)
- [x] 1: failing test first (`open == 1` → `1 conflict still open in …`) — test: `graft-ops operation::tests::conflict_refusals_in_words` (seen red: `left: "1 conflicts still open in both.txt"`)
- [x] 1: other singular sites found and fixed, each tested; the list is in NOTES — tests: `graft update::resolver::tests::base_shown_words_are_singular_at_one`, `graft update::operate::tests::untracked_size_is_singular_at_one`
- [x] 2: m6-09 bottom row shows the box's keys at both sizes — test: `pty-walk-m6.py 120x40|80x24 start-over-brings-the-markers-back` (frame check against `docs/shots/m6/m6-09-start-over-box-*.txt`; green re-run without `GRAFT_UPDATE_SHOTS`, exit 0 both sizes)
- [x] Sheet re-run: 0 uncaptioned — test: `every_scenario::contact_sheet_captions_cover_every_m6_capture` (in the nextest run) + script output `120 captures, 0 without a recorded caption`
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest (3049 passed, 1 skipped), doctests.

## NOTES (builder, 2026-09-24) — not compacted

Scout: none — three one-line format sites and one key list; ticket 45 NOTES carried the harness map.

**Singular sites** (grep of `{n} <plural noun>` in `crates/`):
- Fixed (graft-ops, graft): `operation.rs` `ConflictsOpen`; `update/resolver/base.rs` `base shown for N conflicts` (now `base_shown_words`); `update/operate.rs` discard box untracked size ` — N lines` (now `untracked_size`).
- Already guarded: `stash_screen::files_label`, `push_box.rs` commits, `commit_editor.rs` files.
- **Not fixed, outside crate set (graft-ui), for the conductor:** `graft-ui/src/detail_pane/files.rs:36` `✉ message ({lines} lines)` → `(1 lines)` on a one-line message; `graft-ui/src/graph_screen/layout.rs:429` `{rows} commits` → `1 commits` on a one-commit repo. `map_screen/render.rs:368` `{done}/{total} branches` reads fine as a fraction.
- Error texts (`graft-graph`, `graft-fixtures` `thiserror`) left: not drawn in normal use.

**m6-09:** keys `k ⏎ j j j j ⏎ d` → `k ⏎ j j j j d` (`tests/it/m5_walk/resolving.rs`, the capture only; `open_nth` untouched). Diff of both frames: header keys line and bottom row only (`1 conflicts still open…` → `←/→ move  ⏎ choose  esc cancel`). Captions (`m6-contact-sheet-captions.tsv` rows 17–18) and TESTING.md "Conflicts walk" paragraph updated.

**Item 3:** no other capture held `1 conflicts` / `— 1 lines` / `base shown for 1` (grep of `docs/shots`, sheets excepted).

**Sheet:** `m6-contact-sheet-1.txt` 198 253 B, `-2.txt` 95 902 B; 120 captures, 0 uncaptioned.

**Not verified:** colour (rule 5); look (rule 6).
