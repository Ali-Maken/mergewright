# 47-scroll-acceptance — held `j` stops when he lets go; `J` opens at once, at his scale, both tiers

Status: ready (amended 2026-10-01, spec-only pass after 46 / 46c merged; was stale at old `:37-39`, `:59`)
Build with: Fable. Acceptance ticket: it judges whether the promise holds end to end, which no single diff shows.
Needs: every other 41–46c ticket: `41-scroll-team-fixture`, `42-scroll-held-key-gate`, `43-scroll-coalesce-input`,
`44-scroll-emphasis-layer`, `45-scroll-kitty-layer-deflate`, `46-scroll-strips-wiring` (`_pixel_deflate` /
`_pixel_iterm2` variants, `KittyTransmit::{width_px, deflated}`), `46b-scroll-old-raster-deletion` (composite is the
only raster path), `46c-scroll-strips-fold-churn` (`fold_changes_held=unread`, the `unjudged` strip line,
`pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes`).
Blocked by: 41–46, 46b, 46c

## Promise
On a repository his size, scrolling stops when he lets go of the key, and the next key acts at once — on both tiers.
**Scope of "both tiers" after 46:** Unicode; Pixel on Kitty **with deflate** (Ghostty's path) and on iTerm2. Kitty
without deflate misses by known gap `M7/46-kitty-no-deflate` (**broken**, user chose ship-as-gap 2026-10-01; fix
parked as `46d-scroll-kitty-without-deflate`). This ticket reports it red; it does not hide or fix it.

## Read first
- `42-scroll-held-key-gate.md` (whole: Need, Prior art, Journeys, Decided, Planner ruling).
- `46-scroll-strips-wiring.md` §Decided 7; "Planner ruling 2026-10-01 (46 return)" (a), (b).
- `46c-scroll-strips-fold-churn.md` §NOTES (Step 0 outcome, Decided 4 case "unread", release gate).
- `docs/adr/0053-selection-paint-is-its-own-layer.md`; `docs/adr/0021-*.md` §Decision.
- `docs/perf/fix-scroll-held-key.txt`: the first block (red, rev HEAD) and `# rev fold-churn` (`:238-276`),
  SUMMARY/VERDICT lines only.
- `TESTING.md` §"The heavy tier" (`:145-233`), in particular "Held keys", "The strip bound", "Deflate off is red by
  known gap" (`:169-218`).
- `crates/graft/MAP.md` the `held_key` line (`:907`), `## view::strips`.

## Scope
In: one command that walks both journeys on every tier above, in-process and over a real pty; the TESTING.md
checklist; the contact-sheet section with the Ghostty-path capture; DESIGN.md §3.8's added paragraph; the perf
record. Out: any product change; any verdict edit in `held_key.rs` (46c owns the strip line; 46d owns the raw pair);
deleting or `#[ignore]`-ing the raw pair (rule 2; needs the user's OK, not given). A failing item is reported with its
line, not fixed here.

## The command
`scripts/47-scroll-acceptance.sh` (new) runs, in order, each writing to `docs/perf/47-scroll-acceptance.txt` and
printing only `SUMMARY` lines and the file path (**context guard**: the helper reads summaries and paths, never raw
output):
1. **release `held_key::`, two invocations** (`cargo test --release --features heavy-tests --test it --
   --test-threads=1 --nocapture`):
   - (a) everything under `held_key::` with `--skip` on exactly the two raw tests
     (`held_j_stops_on_release_pixel`, `j_then_journal_opens_at_once_pixel`, `--exact`-matched so the `_deflate` /
     `_iterm2` twins still run). Must exit 0.
   - (b) the raw pair alone. Expected to exit non-zero; the script judges it by its VERDICT lines (Verdict reading
     below), never by exit status.
2. release `frames::` and `heavy::budgets_hold` — nothing else regressed (ADR-0021 run).
3. **pty walk** with `scripts/scroll-lag-pty.py` (the debugger's; extend, do not fork) on `TeamSpec::MONOREPO` (never
   his repo), 187×50, `j` at 16 ms for 3 s, then `J` after 50 ms; reader rates free and 500 KB/s (text) / 40 MB/s
   (kitty). Profiles: `PROFILE=text`; `PROFILE=kitty` answering the `o=z` probe **yes** (judged); `PROFILE=kitty`
   answering **no** (known gap, printed, not judged). Each kitty run prints `deflate=yes|no` **observed** from the
   transmits Graft actually wrote (`o=z` in the APC headers), not from what the profile was told to answer; a judged
   run that observed `no` is a miss. Reads the Graft process's own RSS (`ps -o rss` on its pid at the end).
4. the journey step counts (below).
5. the Ghostty-path capture (Contact sheet box).

Exit 1 on any miss; every miss named in a `SUMMARY miss` line. Known-gap lines print `SUMMARY known_gap
M7/46-kitty-no-deflate <item>=<value>` and are not misses.

### Verdict reading (what the script counts)
- `ok` → pass. `FAILED` → miss, **except** the known-gap list below. `unjudged (fold_changes_held unread)` → neither:
  collected into one `SUMMARY unjudged` line with the counts; the guard for it is the fast-tier test (box 3).
- **Known-gap list, exact** (from `# rev fold-churn` `:248`, `:267-268`): raw `tier=Pixel scenario=A tail_ms<=100`;
  raw `tier=Pixel scenario=B tail_ms<=100` and `j_ms<=100`. Any **other** raw-pair item `FAILED` is a miss. A listed
  item that comes back `ok` prints `SUMMARY note known-gap item passed: <line>` for the conductor (not a miss, not a
  closure; the gap closes through 46d).
- If any line prints a judged `strip_transmits_held<=…` verdict (the harness learned to read the fold), it counts
  like any other item.

## Acceptance (each box names its test)
- [ ] **Gate green, release**, every judged VERDICT item `ok` in: `held_key::{held_j_stops_on_release_unicode,
      j_then_journal_opens_at_once_unicode, held_j_stops_on_release_pixel_deflate,
      j_then_journal_opens_at_once_pixel_deflate}` and the iTerm2 run (scenario `A-iterm2`; name per `held_key.rs`,
      re-locate by the `_pixel_iterm2` suffix). Strip lines read `unjudged`. — step 1(a), the script's `SUMMARY
      held_key` / `SUMMARY unjudged` lines.
- [ ] **Deflate off red by known gap, and only there:** the raw pair fails exactly the known-gap list; every other
      raw item `ok` (`frames_after_release`, `frames_before_J`, `bytes_per_frame_held`). Quote the three FAILED lines
      in NOTES with their values. — step 1(b), `SUMMARY known_gap` lines.
- [ ] **Strip invariant guarded** (replaces the old `ceil(rows_scrolled/32) + 2` row): fast tier green —
      `pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes` and
      `pixel_scroll::j_inside_resident_strips_sends_no_strip`.
- [ ] Before/after table in `docs/perf/47-scroll-acceptance.txt`, from the red run (`fix-scroll-held-key.txt` first
      block, rev HEAD) and this run. Rows: Unicode, Pixel-deflate, Pixel-iTerm2, Pixel-raw (marked known gap); per row
      `tail_ms`, `j_ms`, `frames_after_release`, `bytes_per_frame_held`, `strip_transmits_held`, `fold_changes_held`.
      The red run had no deflate or iTerm2 variant: its Pixel row is the "before" for all three Pixel rows, said so in
      the table. — the script's `SUMMARY before_after` lines.
- [ ] pty walk: `frames_after_release ≤ 2` and `tail_ms ≤ 100` for `text` and judged `kitty` (deflate=yes), every
      rate; kitty `bytes_per_key` ≤ 512 KiB (was 7.5 MB, `scroll-lag-big-repo.txt:51`); **Graft-process RSS ≤ 300
      MB** at both judged profiles (PROMPT.md budget; was 662–703 MB at Pixel, `:49-52`). Kitty deflate=no: printed as
      known gap with the same fields. — `scripts/scroll-lag-pty.py` `SUMMARY pty` lines, checked by the script.
- [ ] No regression: `frames::` p99 ≤ 16 ms on every scroll, `heavy::budgets_hold` green, `g_after_idle` green.
      — those tests by name.
- [ ] Journey step counts, asserted by the walk: journey 1 = 1 key (`j` held), journey 2 = 1 key (`J`); every
      `held_key` test's key list (Unicode, raw, `_deflate`, `_iterm2`) is `j`×188, then `J` in B, nothing else.
      — `held_key::*` assert the key-list shape (42 Decided adds it; add it if missing; it is the walk's count).
- [ ] Fast-tier guard present and green: `event_loop::tests::queued_keys_draw_once`.
- [ ] `TESTING.md`: section "M7 fix: scroll on a big repo", one row per scenario, each naming its test and saying
      concretely what right looks like:
      - held `j`, Unicode: `frames_after_release ≤ 2`, `tail_ms ≤ 100` at a 500 KB/s drain; cursor on the row shown at
        release.
      - held `j`, Pixel (Kitty+deflate, iTerm2): the same at 40 MB/s, plus `bytes_per_frame_held ≤ 512 KiB`; the strip
        count printed `unjudged`, guarded by the capped fast-tier test (point at "The strip bound", don't restate it).
      - `J` after held `j`: `frames_before_J ≤ 1`, `j_ms ≤ 100`, Unicode and Kitty+deflate.
      - Kitty without deflate: red by known gap `M7/46-kitty-no-deflate`, the three items and their last values;
        "right" is exactly those three red and nothing else.
      - Ghostty-path look: the capture below shows the lit thread at full strength, off-thread lanes at half
        strength, no line wider than another, cursor row matching the Unicode grid's. Colour **unverified** (rule 5);
        what Ghostty draws is the user's check.
      - his daily check (below).
- [ ] Contact sheet (ADR-0037): the milestone sheet's next section if one exists, else
      `docs/shots/m7/fix-scroll-contact.md` (say which), each capture under its "right looks like" line, plus the
      before/after table as text:
      - `docs/shots/m7/fix-scroll-ghostty.png`: the Kitty+deflate frame at release, MONOREPO 187×50, as Graft sends it
        to Ghostty — the strip backdrops with the emphasis layer over them, built through `composite` (ADR-0053: the
        same picture the two Kitty layers blend to). Cropped to the graph column, cursor row ±10 rows; if still over
        200 KB, fewer rows, never resampled. Caption: "colour unverified; Ghostty's own rendering unverified until the
        user looks".
      - `docs/shots/m7/fix-scroll-release-unicode.txt`: the pty `text` walk's grid at release, trimmed to the graph
        column, same rows.
      - (The old `fix-scroll-thread-legacy.png` is gone: 46b deleted the legacy raster. 46b's re-taken snapshots pin
        the composite.)
- [ ] `docs/design/03-graph-layout-algorithm.md` §3.8: one paragraph after "The key": the backdrop alpha joins the
      key; the selection's paint is a second image above the strip, keyed on the strip plus the selection (ADR-0053);
      on a graph wider than the lane cap the fold follows the selection (ADR-0025), so a `j` can re-send strips — a
      layout change, not paint; Kitty transmits deflated when probed, and without deflate the held-key budget is not
      met (known gap). Nothing else in DESIGN.md changes.
- [ ] NOTES list, for the conductor's `KNOWN_GAPS.md` (builders do not edit it):
      - "scroll fix not yet tried in his terminal on his repo" — **unverified**, not broken.
      - "his Ghostty answers the `o=z` probe yes" — **unverified** (46 ruling (b)); settled by his first Ghostty run.
      - `M7/46-kitty-no-deflate`: this run's three values, for the row (it stays open; this ticket doesn't close it).
      - any verdict item that missed, with its line.

## For the user (the conductor asks; not this ticket's call)
- Daily check, his words to `docs/field-notes.md` the same day: open his work repo, hold `j` ~3 s, let go — does it
  stop at once? Then `J` — does it open at once? In Zed (Unicode) and in Ghostty (Pixel).
- In Ghostty: does the graph look like `fix-scroll-ghostty.png` (lit thread, dimmed lanes)? If it scrolls slowly
  there, say so the same day: that would mean his Ghostty fell back to raw (the known gap), not that the fix failed.

## Files (exact)
- `scripts/47-scroll-acceptance.sh` (new), `scripts/scroll-lag-pty.py` (extend: fixture path, rates, RSS, `J`, the
  deflate yes/no profile answer, the observed `deflate=` field, release-grid dump), `docs/perf/47-scroll-acceptance.txt`
  (new), `TESTING.md`, the contact sheet file, `docs/shots/m7/fix-scroll-ghostty.png`,
  `docs/shots/m7/fix-scroll-release-unicode.txt`, `docs/design/03-graph-layout-algorithm.md` (§3.8 paragraph only).
- `crates/graft/tests/it/held_key.rs` only for the key-list assertion if missing (no verdict edits).
- The PNG writer: a test under `crates/graft/tests/it/` gated on `heavy-tests`, run by step 5; it uses the existing
  composite path, adds no product API.

## Touches
- `scripts/fix-scroll-held-key.sh` and `docs/perf/fix-scroll-held-key.txt`: read, never written (47 writes its own
  file).
- `46d-scroll-kitty-without-deflate`: when it lands, the script's known-gap list and the pty `deflate=no` row become
  judged. Name it in the script's comment beside the list.
- `scripts/scroll-lag-big-repo.sh` (the debugger's, takes a copy of his repo): untouched; the acceptance never reads
  his repo.
- `scripts/m7-perf-key.sh`: must still select no `held_key::` (grep, quote).
- Raw terminal streams: never committed; the pty script dumps under ignored `target/`.
- Redaction guard (`capture::machine`): the perf file and the `.txt` capture carry no home or scratch path; check
  with the guard test.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; also `cargo clippy --all-targets --features heavy-tests -- -D warnings`. Release runs only through
the script.

## Size
Medium (~50k).
