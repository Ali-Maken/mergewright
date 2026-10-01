# 47-scroll-acceptance — held `j` stops when he lets go; `J` opens at once, at his scale, both tiers

Status: built 2026-10-02. **Merges on the planner's rulings (2026-10-02, below)** once the conductor has filed the two
gap rows and the cursor fix ticket. Two boxes stay open by ruling, not ticked: pty RSS (gap `M7/47-pixel-rss`,
**broken**, expiry M8) and `g_after_idle` (gap `M7/47-idle-low-power`, **unverified**). Q1 is fixed before the M7 stop
by `scroll-cursor-under-uncommitted-row`.
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
- [x] **Gate green, release**, every judged VERDICT item `ok` in: `held_key::{held_j_stops_on_release_unicode,
      j_then_journal_opens_at_once_unicode, held_j_stops_on_release_pixel_deflate,
      j_then_journal_opens_at_once_pixel_deflate}` and the iTerm2 run (scenario `A-iterm2`; name per `held_key.rs`,
      re-locate by the `_pixel_iterm2` suffix). Strip lines read `unjudged`. — step 1(a), the script's `SUMMARY
      held_key` / `SUMMARY unjudged` lines.
      **Ticked:** the four tests above plus `held_key::held_j_stops_on_release_pixel_iterm2` (and
      `b_bound_stops_at_js_frame`), run 1(a): `SUMMARY held_key run=a tests=6 exit=0 verdict_ok=17 verdict_failed=0`;
      `SUMMARY unjudged strip_transmits_held lines=5`.
- [x] **Deflate off red by known gap, and only there:** the raw pair fails exactly the known-gap list; every other
      raw item `ok` (`frames_after_release`, `frames_before_J`, `bytes_per_frame_held`). Quote the three FAILED lines
      in NOTES with their values. — step 1(b), `SUMMARY known_gap` lines.
      **Ticked:** `held_key::held_j_stops_on_release_pixel`, `held_key::j_then_journal_opens_at_once_pixel`, run 1(b):
      exactly the three listed items FAILED, every other raw item `ok`, no `SUMMARY miss` from 1(b). Values in NOTES.
- [x] **Strip invariant guarded** (replaces the old `ceil(rows_scrolled/32) + 2` row): fast tier green —
      `pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes` and
      `pixel_scroll::j_inside_resident_strips_sends_no_strip`.
      **Ticked:** both PASS (`cargo nextest run` by name, `--status-level pass`, 2026-10-02).
- [x] Before/after table in `docs/perf/47-scroll-acceptance.txt`, from the red run (`fix-scroll-held-key.txt` first
      block, rev HEAD) and this run. Rows: Unicode, Pixel-deflate, Pixel-iTerm2, Pixel-raw (marked known gap); per row
      `tail_ms`, `j_ms`, `frames_after_release`, `bytes_per_frame_held`, `strip_transmits_held`, `fold_changes_held`.
      The red run had no deflate or iTerm2 variant: its Pixel row is the "before" for all three Pixel rows, said so in
      the table. — the script's `SUMMARY before_after` lines.
      **Ticked:** `SUMMARY before_after` lines (from `held_key::*` run 1(a)/(b) and the red block), also in
      `docs/shots/m7/fix-scroll-contact.md` §3.
- [ ] pty walk: `frames_after_release ≤ 2` and `tail_ms ≤ 100` for `text` and judged `kitty` (deflate=yes), every
      rate; kitty `bytes_per_key` ≤ 512 KiB (was 7.5 MB, `scroll-lag-big-repo.txt:51`); **Graft-process RSS ≤ 300
      MB** at both judged profiles (PROMPT.md budget; was 662–703 MB at Pixel, `:49-52`). Kitty deflate=no: printed as
      known gap with the same fields. — `scripts/scroll-lag-pty.py` `SUMMARY pty` lines, checked by the script.
      **Not ticked — RSS miss.** Frames, tail, bytes and deflate hold (text free/500k: fal 1, tail 92/62, rss 24;
      kitty-yes free/40m: fal 1, tail 56/58, 173/171 KB per key, `deflate=yes` observed, 525/518 transmits all `o=z`). **Graft RSS 396 / 369 MB > 300** at
      kitty-yes (`SUMMARY miss pty kitty-yes-free rss_mb=396`, `kitty-yes-40m rss_mb=369`). See NOTES M2.
      **Open by ruling 2** (gap `M7/47-pixel-rss`, broken, expiry M8; fix parked as `scroll-pixel-memory-budget`).
      The budget stays 300 MB. Closes when that ticket's run shows ≤ 300 MB here.
- [ ] No regression: `frames::` p99 ≤ 16 ms on every scroll, `heavy::budgets_hold` green, `g_after_idle` green.
      — those tests by name.
      **Not ticked — `frames::after_idle::g_after_idle` FAILED** (16.91 ms > 16). `frames::` otherwise 18/19 passed;
      `heavy::budgets_hold_*` 5/5 passed. See NOTES M1.
      **Open by ruling 1** (gap `M7/47-idle-low-power`, unverified). The budget stays 16 ms. Closes, or turns
      broken, at the M7 review's release perf run on mains with Low Power Mode off.
- [x] Journey step counts, asserted by the walk: journey 1 = 1 key (`j` held), journey 2 = 1 key (`J`); every
      `held_key` test's key list (Unicode, raw, `_deflate`, `_iterm2`) is `j`×188, then `J` in B, nothing else.
      — `held_key::*` assert the key-list shape (42 Decided adds it; add it if missing; it is the walk's count).
      **Ticked:** the assertion exists (`crates/graft/tests/it/held_key.rs:338-350`, not edited); all `held_key::*`
      ran past it (no `key list:` panic); `SUMMARY journeys … held_key_summaries_with_188/189_keys=7
      pty_runs_with_j188_then_J=6/6`.
- [x] Fast-tier guard present and green: `event_loop::tests::queued_keys_draw_once`.
      **Ticked:** PASS (`cargo nextest run` by name).
- [x] `TESTING.md`: section "M7 fix: scroll on a big repo", one row per scenario, each naming its test and saying
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
      **Ticked:** `TESTING.md` §"M7 fix: scroll on a big repo (m7-power-tools/47)" at the end of the file, one row per
      scenario naming its test (`held_key::*`, `frames::`, `heavy::budgets_hold_*`, `ghostty_shot::fix_scroll_ghostty_png`),
      plus the pty walk and journeys rows. Its own boxes stay open for the reviewer and the user.
      The "cursor on the row shown at release" row cannot hold until `scroll-cursor-under-uncommitted-row` lands; that
      ticket owns it (ruling 3).
- [x] Contact sheet (ADR-0037): the milestone sheet's next section if one exists, else
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
      **Ticked:** its own file, `docs/shots/m7/fix-scroll-contact.md`. The m7 sheet is text assembled from
      `m7-contact-sheet-captions.tsv`, so a PNG cannot join it. The PNG comes from `ghostty_shot::fix_scroll_ghostty_png`
      (9 396 B, 850×370, `capture::check_bytes`). The grid comes from the pty `text` walk (`scroll-lag-pty.py`
      `GRID_OUT`, rows 39–48). **Neither shows the cursor row.** The crop is the cursor ±10, clipped to the pane, and
      the cursor sits one row below the pane (`cursor_on_screen=false`), so "cursor row matching" cannot be shown. It is
      a finding (NOTES Q1), not a crop choice. Nobody judged "looks right" (rule 6).
      **The tick covers the files and their captions only.** The two captures are re-taken, cursor row shown, by
      `scroll-cursor-under-uncommitted-row` (ruling 3).
- [x] `docs/design/03-graph-layout-algorithm.md` §3.8: one paragraph after "The key": the backdrop alpha joins the
      key; the selection's paint is a second image above the strip, keyed on the strip plus the selection (ADR-0053);
      on a graph wider than the lane cap the fold follows the selection (ADR-0025), so a `j` can re-send strips — a
      layout change, not paint; Kitty transmits deflated when probed, and without deflate the held-key budget is not
      met (known gap). Nothing else in DESIGN.md changes.
      **Ticked:** the indented paragraph under "The key"; that is the only change in that file's diff.
- [x] NOTES list, for the conductor's `KNOWN_GAPS.md` (builders do not edit it):
      - "scroll fix not yet tried in his terminal on his repo" — **unverified**, not broken.
      - "his Ghostty answers the `o=z` probe yes" — **unverified** (46 ruling (b)); settled by his first Ghostty run.
      - `M7/46-kitty-no-deflate`: this run's three values, for the row (it stays open; this ticket doesn't close it).
      - any verdict item that missed, with its line.
      **Ticked:** NOTES below.

## For the user (the conductor asks; not this ticket's call)
- Daily check, his words to `docs/field-notes.md` the same day: open his work repo, hold `j` ~3 s, let go — does it
  stop at once? Then `J` — does it open at once? In Zed (Unicode) and in Ghostty (Pixel).
- In Ghostty: does the graph look like `fix-scroll-ghostty.png` (lit thread, dimmed lanes)? If it scrolls slowly
  there, say so the same day: that would mean his Ghostty fell back to raw (the known gap), not that the fix failed.
- Ask him only after `scroll-cursor-under-uncommitted-row` lands. Before that, a held `j` on a dirty tree loses the
  highlighted row (Q1), and he would be reporting a bug that is already known.

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

## NOTES (builder, 2026-10-02)

**Compacted once mid-ticket** (after the measurements, before TESTING.md / gates / ticket). Nothing remains undone.
Scout: yes, one `scout` before building (held_key harness, strips, composite path). Record:
`docs/perf/47-scroll-acceptance.txt` (HEAD a4d1f93b + this ticket's changes, `# exit 1`). Raw output stays under
ignored `target/47-acceptance/`.

### Known gap `M7/46-kitty-no-deflate`, this run's values (for the KNOWN_GAPS row; it stays open)
Run 1(b), `held_key::held_j_stops_on_release_pixel` / `held_key::j_then_journal_opens_at_once_pixel`:
```
VERDICT held_key tier=Pixel scenario=A tail_ms<=100: FAILED      tail_ms=2223
VERDICT held_key tier=Pixel scenario=B tail_ms<=100: FAILED      tail_ms=655
VERDICT held_key tier=Pixel scenario=B j_ms<=100: FAILED         j_ms=605
```
Every other raw item `ok` (frames_after_release 2, frames_before_J, bytes_per_frame_held 0). pty, printed not judged:
kitty-no free tail 118, rss 465 MB, 1.52 MB/key; 40m tail 152, rss 487 MB. (An earlier run of this script, before the
pty fix, printed A 2787, B 651, B 608, so the A tail varies a lot from run to run.)

### Misses (two boxes red; reported, not fixed: out of scope)
- **M1 `frames::after_idle::g_after_idle`: 16.91 ms > 16 ms** (`crates/graft/tests/it/frames/after_idle.rs:99`, the
  assert). Rerun alone: 16.6, 16.7 and 17.0 ms; another full run: 21.4 ms. At 12e it measured 8.2–8.6 ms
  (`docs/perf/m7-12-idle-advance.txt`). Cause **not bisected**; the regression lies somewhere between 12e and a4d1f93b.
  The rest of `frames::` is 18/19 green and `heavy::budgets_hold_*` is 5/5.
- **M2 Graft-process RSS at Kitty+deflate: 396 MB (free) / 369 MB (40 MB/s), over the 300 MB budget.** Measured with
  `ps -o rss` on Graft's pid at the end of the pty run (`scripts/scroll-lag-pty.py`). It was 662–703 MB before.
  Unicode is 24 MB on the same fixture, so the excess is on the Pixel path (strip and emphasis caches are the obvious
  suspects; **not measured**, so this is a question, not a cause).

### Q1 (filed as a question, rule 7): the cursor ends one row below the pane after held `j` when the uncommitted row shows
- Seen: in-process (`ghostty_shot`) `top=141`, `graph_rows=48`, uncommitted row pinned on top, cursor 188, so
  history rows 141–187 are drawn and `cursor_on_screen=false`. In the pty Unicode stream the selection-filled row is
  present at t=−10 ms and at 500 ms, and absent from about 1000 ms through release. The fixture's working tree is
  dirty (an untracked `corpus.ok`); his repo usually is too.
- Probable lines (not confirmed by a test):
  - `crates/graft/src/update/keys.rs:439`: `move_cursor` scrolls with `height = next.graph_rows()`.
  - The view draws `graph_rows() - uncommitted` history rows: `crates/graft/src/view/strips.rs:211-212`, and
    presumably the Unicode path's equivalent.
  - Taken together, these keep the cursor one row past the last drawn row.
- Effect: TESTING's "cursor on the row shown at release" cannot hold today; the captures can't show the cursor row.
  The user would see the selection vanish while holding `j`.

### For KNOWN_GAPS (conductor)
- "scroll fix not yet tried in his terminal on his repo": **unverified**.
- "his Ghostty answers the `o=z` probe yes": **unverified** (46 ruling (b)).
- `M7/46-kitty-no-deflate`: values above.
- M1, M2 and Q1 above.
- Colour of both captures and Ghostty's own rendering: **unverified** (rules 5, 6).

### Fixes made to the pty script on the way (cause, not symptom)
`scroll-lag-pty.py`'s reader computed `time.sleep(due - now)`, which went negative under a slow drain. The
ValueError killed the reader thread, Graft blocked on a full pty, and the main thread hung for 16 minutes. The fix
clamps the sleep at 0. Guards now record the failure in `broken=`; nothing retries or hides it:
- the reader is wrapped and records `reader_died_<Exc>`;
- `send()` checks that Graft is still alive (`graft_exited_N`), that the reader is alive, and that the pty is
  writable within 1 s (`input_blocked`).

### Touches
- `scripts/scroll-lag-big-repo.sh`: unchanged and still compatible. Its call (`:82`) passes the same positional
  arguments, and every new behaviour is behind a new env var (`DEFLATE`, `THEN_J`, `GRID_OUT`).
- `scripts/m7-perf-key.sh:60` selects `-- frames:: frame_trace::`. That matches no `held_key::` test (grep).
- `scripts/fix-scroll-held-key.sh`, `docs/perf/fix-scroll-held-key.txt`: read only.
- `crates/graft/tests/it/held_key.rs`: not edited; the key-list assertion was already at `:338-350`.
- `crates/graft/Cargo.toml`: dev-dependency `tiny-skia` only, at graft-graph's version and features, so no new
  package; the `Cargo.lock` change is one line. Dev-dependencies are exempt from the `dependency_edges` crate rule.
- `crates/graft/MAP.md`: `held_key` line extended; `ghostty_shot` line added. No product API was added.
- Privacy: `cargo run -p graft-fixtures -- redact` on the perf file, the `.txt` capture, the contact sheet, both
  scripts and TESTING.md reported "0 of 6 files changed". One home path had already been redacted earlier and made
  relative. No raw stream was committed.
- Gates (workspace-wide, 2026-10-02): all green.
  - `cargo fmt --check`;
  - `cargo clippy --all-targets -D warnings`, with and without `graft/heavy-tests`;
  - `cargo nextest run`: 5111/5111;
  - `cargo test --doc`.

## NOTES (debugger, 2026-10-02)
Record: `docs/perf/47-debug.txt` (scripts `scripts/47-debug-idle.sh`, `scripts/47-debug-rss.sh`; `scroll-lag-pty.py`
gained `MEM_OUT=` for a `vmmap --summary` at the end). Boxes stay unticked: neither miss is fixed.

**M1 `g_after_idle` — not a code regression.** The 12e merge's own code (8422c453) measures 16.6/16.7/17.1 ms today;
HEAD 16.6–22.9 ms. A CPU-only control in the same test (`refs: badges()`) runs 10.9–11.7 ms today against 5.1–6.5 ms
in `m7-12g-key-37.txt`/`m6-review.txt`: the machine runs ~2× slower than at 12e, and 2 × 8.3 ≈ 16.6. `pmset -g`
showed **Low Power Mode on, on battery** for every run (and the 47 run: badges() 11.4 ms). Not toggled, so that
attribution is unconfirmed. No file:line: nothing moved in code. To settle: rerun `scripts/47-debug-idle.sh . 3` on
mains with Low Power Mode off. Split: the history job's `ShowRows` window answers at ~16 ms; the rest is ~1 ms.

**M2 RSS — predates 41–46c, structural.** MONOREPO, Kitty, deflate yes, `j` 3 s: 0d38d93d **977 MB**; HEAD 367–374
MB (idle without `j`: 78 → 60 MB; text tier 23–24 MB both). At the end of a HEAD run: live large blocks 170.8 MB
(StripCache 117.8 MB = 16 backdrops + emphases at ~4.1 MB each on this wide graph; KittyImages 45.2 MB of frames the
cache had already evicted), plus 166 MB of **freed** large blocks still resident in macOS malloc's free cache. Lines:
`crates/graft/src/event_loop/graphics.rs:33` (`STRIPS_KEPT = 16`, counted in strips, sized for 12 lanes at ~1.9 MB a
strip; ×2 layers since ADR-0053), `crates/graft-term/src/kitty/mod.rs:123` (`Upload::frame: Arc<RasterFrame>` keeps
evicted pixels alive for a pointer compare). Experiment (reverted): `Weak` there cut live memory 45 MB and RSS not at
all (369 MB): the freed buffers moved into malloc's free cache. A fix needs a byte budget for strips and/or buffer reuse,
not a constant tweak. Not fixed.

**Q1 confirmed** by a test, saved red at `.scratch/m7-power-tools/47-q1-cursor-test.patch`
(`view::tests::held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row`: after 17 `j`, cursor 17, top 0,
graph_rows 18, 18 rows drawn, none the cursor's). Cause: the views draw `graph_rows() - 1` history rows under the
uncommitted row (`view/entries.rs:109-113`, `view/strips.rs:211-212`), every scroller passes `graph_rows()`
(`update/keys.rs:439` for `j`; also `keys.rs:408`, `history.rs:21,63`, `mod.rs:743,864,1039,1286`, `mouse.rs:189,209`,
`search.rs:147,178,198`, `map.rs:204`, `keys/actions.rs:22`, `keys/escape.rs:10`, `detail/landing.rs:147,177`,
`preview.rs:373`, `rewrite_editor/after.rs:69`, `banner/stop.rs:319`). Not one line: fixing `:439` alone leaves the
rest off by one. Not fixed.

## Rulings (planner, 2026-10-02)

**Merge: yes**, on the boxes ticked above. Before merging, the conductor files the gap rows for rulings 1 and 2 and
files `scroll-cursor-under-uncommitted-row` (ruling 3) to build before the M7 stop. Neither open box is ticked, and
neither budget moves. Every ticked box keeps its named test.

**Ruling 1: `g_after_idle`. A gap marked unverified, not a code defect, and no change to the 16 ms budget.**
- Evidence (debugger, `docs/perf/47-debug.txt:2,15-16`): the 12e merge's own code (8422c453) measures 16.6–17.1 ms
  today, against 8.2–8.9 ms at 12e. A CPU-only control runs 2.07× slower than at 12g. `pmset -g` showed Low Power Mode
  on, on battery, for every run. The attribution is **unconfirmed**: the setting was never toggled.
- KNOWN_GAPS row `M7/47-idle-low-power`, **unverified**: "`frames::after_idle::g_after_idle` 16.6–22.9 ms > 16 ms,
  measured only on battery with Low Power Mode on. The 12e code measures the same today. Re-measure at the M7 review's
  release perf run." Expiry: the M7 review.
- How it closes: the M7 review's release perf run, on mains with Low Power Mode off, runs `scripts/47-debug-idle.sh .
  3` and the 47 script's step 2.
  - Green → the row closes and box 6 is ticked, naming `frames::after_idle::g_after_idle` and the run's perf file.
  - Red → the row turns **broken**, and a `debugger` bisects 8422c453..HEAD. The place to start is the history job's
    `ShowRows` window read, about 15 of the 16.6 ms (`47-debug.txt:17`).
- So this cannot recur unseen, `perf-machine-state-guard` (unnumbered, Sonnet) makes every release perf script record
  the machine's power state and refuse to judge on battery or with Low Power Mode on. It runs before the M7 review.
- **Needs the user:** mains power and Low Power Mode off for the review's perf run. That is a physical step the conductor
  cannot take.

**Ruling 2: RSS. A known gap, broken, expiry M8. The fix is parked as `scroll-pixel-memory-budget`.**
- Why not now:
  - The cause is structural and sits in three places: strip capacity is counted in strips, not bytes
    (`graphics.rs:33`); evicted uploads stay pinned (`kitty/mod.rs:123`); and freed large buffers stay resident.
  - The fix changes `StripCache`'s capacity contract and `KittyImages`' upload identity across three crates. That is a
    signature ticket (Fable) that needs its own measurement first.
  - It predates the scroll work. M7 cut it from 977 MB to 367–374 MB.
  - The promise's latency items hold.
  - Unicode, his Zed tier, is at 24 MB.
- KNOWN_GAPS row `M7/47-pixel-rss`, **broken**: "Pixel (Kitty+deflate) Graft RSS 367–396 MB after held `j` on
  MONOREPO 187×50; budget 300 MB (PROMPT.md 7.x peak-memory); live 171 MB (StripCache 118, evicted Kitty frames 45) +
  166 MB freed-but-resident." Expiry M8. Box 5 stays open until that ticket's run ticks it.
- The user is told at the stop, as a gap line. He is not asked a question. If he wants it fixed in M7, the spec is
  ready to unpark.

**Ruling 3: cursor (Q1). A defect, confirmed by a red test, fixed before the M7 stop.**
- The spec is `scroll-cursor-under-uncommitted-row.md`, Build with Opus, scout first.
- Cause, fixed in one place: `AppState::graph_rows()` subtracts the pinned uncommitted row, as it already does for the
  commit-graph line, the pending block and the preview box. The views stop subtracting their own copy. Every scroller
  is then right with no change.
- That ticket re-takes this ticket's two captures with the cursor row shown, and owns the TESTING row "cursor on the
  row shown at release".
- Until it lands, the conductor does not ask the user for his daily check (see "For the user").
