# 47-scroll-acceptance — held `j` stops when he lets go; `J` opens at once, at his scale, both tiers

Status: ready
Build with: Fable. Acceptance ticket: it judges whether the promise holds end to end, which no single diff shows.
Needs: every other 41–46 ticket: `41-scroll-team-fixture`, `42-scroll-held-key-gate`,
Blocked by: 41–46, 46b, 46c
`43-scroll-coalesce-input`, `44-scroll-emphasis-layer`, `45-scroll-kitty-layer-deflate`, `46-scroll-strips-wiring`.

## Promise
On a repository his size, scrolling stops when he lets go of the key, and the next key acts at once — on both tiers.

## Read first
- `42-scroll-held-key-gate.md` (whole: Need, Prior art, Journeys, Decided, Planner ruling).
- `docs/adr/0053-selection-paint-is-its-own-layer.md`; `docs/adr/0021-*.md` §Decision.
- `docs/perf/fix-scroll-held-key.txt` (SUMMARY lines only).
- `TESTING.md` §"The heavy tier" (`:145-233`).

## Scope
In: one command that walks both journeys on both tiers, in-process and over a real pty; the TESTING.md checklist;
the contact-sheet section; DESIGN.md §3.8's added paragraph; the perf record. Out: any product change. A failing
item is reported with its line, not fixed here.

## The command
`scripts/47-scroll-acceptance.sh` (new) runs, in order, each writing to `docs/perf/47-scroll-acceptance.txt` and
printing only `SUMMARY` lines and the file path (**context guard**: the helper reads summaries and paths, never raw
output):
1. release `held_key::` (the four tests; a root module of the `it` test crate, outside `frames::`, per 42's planner
   ruling 2026-10-01) — the gate.
2. release `frames::` and `heavy::budgets_hold` — nothing else regressed (ADR-0021 run).
3. **pty walk** with `scripts/scroll-lag-pty.py` (the debugger's; extend, do not fork) on `TeamSpec::MONOREPO` (never
   his repo), 187×50, `PROFILE=text` and `PROFILE=kitty`, `j` at 16 ms for 3 s, then `J` after 50 ms; reader rates
   free and 500 KB/s (text) / 40 MB/s (kitty). Reads the Graft process's own RSS (`ps -o rss` on its pid at the end).
4. the journey step counts (below).
Exit 1 on any miss; every miss named in a `SUMMARY miss` line.

## Acceptance (each box names its test)
- [ ] Gate green, both tiers, release: every verdict item `ok` in the four `SUMMARY held_key` lines.
      — `held_key::{held_j_stops_on_release_unicode, held_j_stops_on_release_pixel,
      j_then_journal_opens_at_once_unicode, j_then_journal_opens_at_once_pixel}`.
- [ ] Before/after table in `docs/perf/47-scroll-acceptance.txt`, from the red run (`fix-scroll-held-key.txt`, rev
      HEAD) and this run: per tier `tail_ms`, `j_ms`, `frames_after_release`, `bytes_per_frame_held`,
      `strip_transmits_held`. — the script's `SUMMARY before_after` lines.
- [ ] pty walk: `frames_after_release ≤ 2` and `tail_ms ≤ 100` for every profile and rate; Pixel `bytes_per_key`
      ≤ 512 KiB (was 7.5 MB, `scroll-lag-big-repo.txt:51`); **Graft-process RSS ≤ 300 MB** at both profiles (PROMPT.md
      budget; was 662–703 MB at Pixel, `:49-52`). — `scripts/scroll-lag-pty.py` `SUMMARY pty` lines, checked by the
      acceptance script.
- [ ] No regression: `frames::` p99 ≤ 16 ms on every scroll, `heavy::budgets_hold` green, `g_after_idle` green.
      — those tests by name.
- [ ] Journey step counts, asserted by the walk: journey 1 = 1 key (`j` held), journey 2 = 1 key (`J`); the held-key
      test's key list is `j`×188 then `J`, nothing else. — `held_key::*` assert the key-list shape (42 Decided adds
      it; add it if missing; it is the walk's count).
- [ ] Fast-tier guards present and green: `pixel_scroll::j_inside_resident_strips_sends_no_strip`,
      `event_loop::tests::queued_keys_draw_once`.
- [ ] `TESTING.md`: section "M7 fix: scroll on a big repo", one row per scenario, each naming its test and saying
      concretely what right looks like:
      - held `j`, Unicode: `frames_after_release ≤ 2`, `tail_ms ≤ 100` at a 500 KB/s drain; cursor on the row shown at
        release.
      - held `j`, Pixel: the same at 40 MB/s, plus `bytes_per_frame_held ≤ 512 KiB` and
        `strip_transmits_held ≤ ceil(rows_scrolled/32) + 2`.
      - `J` after held `j`: `frames_before_J ≤ 1`, `j_ms ≤ 100`, both tiers.
      - Pixel look: the two PNGs below show the same lit thread; off-thread lanes at half strength; no line wider than
        another. Colour **unverified** (rule 5).
      - his daily check (below).
- [ ] Contact sheet (ADR-0037): the milestone sheet's next section if one exists, else
      `docs/shots/m7/fix-scroll-contact.md` (say which): `fix-scroll-thread-legacy.png`,
      `fix-scroll-thread-composite.png`, each under its "right looks like" line, plus the before/after table as text.
      Each file ≤ 200 KB.
- [ ] `docs/design/03-graph-layout-algorithm.md` §3.8: one paragraph after "The key": the backdrop alpha joins the
      key; the selection's paint is a second image above the strip, keyed on the strip plus the selection (ADR-0053);
      Kitty transmits deflated when probed. Nothing else in DESIGN.md changes.
- [ ] NOTES list, for the conductor's `KNOWN_GAPS.md` (builders do not edit it): "scroll fix not yet tried in his
      terminal on his repo" — **unverified**, not broken; the terminal question if still open; any verdict item that
      missed, with its line.

## For the user (the conductor asks; not this ticket's call)
- Which terminal and size was it (Ghostty → Pixel; Zed / VS Code → Unicode)? The diagnosis' RSS points to Pixel
  (`debug-scroll-lag.md` §Question). Rule 7: unconfirmed.
- Daily check, his words to `docs/field-notes.md` the same day: open his work repo, hold `j` ~3 s, let go — does it
  stop at once? Then `J` — does it open at once?

## Files (exact)
- `scripts/47-scroll-acceptance.sh` (new), `scripts/scroll-lag-pty.py` (extend: fixture path, rates, RSS, `J`),
  `docs/perf/47-scroll-acceptance.txt` (new), `TESTING.md`, the contact sheet file,
  `docs/design/03-graph-layout-algorithm.md` (§3.8 paragraph only).
- `crates/graft/tests/it/held_key.rs` only for the key-list assertion if missing.

## Touches
- `scripts/scroll-lag-big-repo.sh` (the debugger's, takes a copy of his repo): untouched; the acceptance never reads
  his repo.
- Raw terminal streams: never committed; the pty script dumps under ignored `target/`.
- Redaction guard (`capture::machine`): the perf file carries no home or scratch path; check with the guard test.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; the release runs only through the script.

## Size
Medium (~45k).
