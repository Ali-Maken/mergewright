# M7 fix: scroll on a big repo — contact sheet (m7-power-tools/47)

Its own file, not the next section of `m7-contact-sheet-*.txt`: that sheet is
text assembled from `m7-contact-sheet-captions.tsv`, and the Ghostty-path
capture is a PNG. Both captures come from `scripts/47-scroll-acceptance.sh`
step 5, on `TeamSpec::MONOREPO` (17 700 commits, his repo's shape) at 187×50,
after `j` held ×188 at 16 ms, before `J`. Numbers: `docs/perf/47-scroll-acceptance.txt`.

## 1. The Ghostty path: `fix-scroll-ghostty.png`

*Re-shot 2026-10-22 (ease-pass-22): the old picture drew the strips bare; this one is laid out as the real frame places them.*

**Right looks like:** the lit thread (the selection's lanes and dots) at full
strength, the off-thread lanes at half strength, no line wider than another,
the same lines as the Unicode grid below, one for one. **Colour unverified;
Ghostty's own rendering unverified until the user looks** (CLAUDE.md rules 5 and 6).

![Kitty+deflate frame at release](fix-scroll-ghostty.png)

What it is: every strip on screen as Mergewright sends it to a Kitty terminal
with deflate on — the strip backdrop with the selection's emphasis image over
it, built through `StripCache::composited` (ADR-0053) — laid out by
`event_loop::placements`, the call the real frame places its strips with:
cropped to the graph column (2 cells, 34 px) and to the pane's lines 34–45
(12 lines of 37 px, 444 px): history rows 178–188, the cursor's row 188 on
line 44, and line 45, the cursor row's second line, drawn as its continuation
band (a vertical stroke, no dot). Over a stand-in dark background
(`#1E1E2E`), not his theme's. Drawn in-process by
`crates/mergewright/tests/it/ghostty_shot.rs`; Ghostty itself never ran.

**Why the cursor was "44 vs 45" and the old PNG had 189:** the old shot
skipped the tall-row step, so it kept history row 189 where the real frame
puts the second line, and the cursor looked one line from the end. The
"45" was a miscount of the text grid: the pane starts on screen row 2 (row 0
the title bar, row 1 blank), not 1, so the grid's cursor on screen row 46 is
pane line 44, the PNG's.

## 2. The Unicode tier: `fix-scroll-release-unicode.txt`

**Right looks like:** the same 12 pane lines as the PNG (screen rows 36–47, pane lines 34..46 counted from 0),
the graph column only (first 4 columns: the graph is as wide as the lanes in view, 2 here, so there is no `⋮`
divider to cut at); a `◉` merge dot in the lane column on each history row, the cursor `▸` on line 44 (screen row 46),
and a `┃` on line 45 (screen row 47), the cursor row's second line. **The two line up row for row:** 11 history
rows 178–188 and the second line. The `▴` on row 185's line is a text-tier marker the PNG has no counterpart for. Characters only:
the lit thread's colour and the cursor's fill are not recorded, so this grid cannot show which row is lit.
Colour unverified. Re-taken 2026-10-22 (ease-pass-22) by the pty text walk of
`scripts/47-scroll-acceptance.sh` step 5, run on its own (not the whole perf script).

```
36|  ◉
37|  ◉
38|  ◉
39|  ◉
40|  ◉
41|  ◉
42|  ◉
43|  ◉▴
44|  ◉
45|  ◉
46|▸ ◉
47|  ┃
```

The PNG is drawn by the Pixel tier and this grid by the Unicode tier, from
two different runs, and their graph columns are not the same width, so compare
lines, not columns.

## 3. Before and after

In-process (`held_key::`, a terminal draining at 500 KB/s Unicode, 40 MB/s
Pixel). Before: `fix-scroll-held-key.txt`'s first block (rev `e0052eac`,
red); it had no deflate or iTerm2 variant, so its raw-Kitty Pixel row is the
"before" of all three Pixel rows. `j_ms` from scenario B, the rest from A.

| Row | tail_ms | j_ms | frames_after_release | bytes_per_frame_held | strip_transmits_held | fold_changes_held |
|---|---|---|---|---|---|---|
| Unicode | 3260 → 69 | 3210 → 23 | 109 → 2 | 10 667 → 14 074 | 0 → 0 | n/a |
| Pixel, Kitty + deflate | 215 976 → 47 | 225 288 → 2 | 188 → 2 | 13 113 157 → 152 255 | 4 → 81 (unjudged) | unread |
| Pixel, iTerm2 | 215 976 → 37 | n/a (no B) | 188 → 2 | 13 113 157 → 132 424 | 4 → 0 (unjudged) | unread |
| Pixel, Kitty raw (**known gap**) | 215 976 → 2 223 | 225 288 → 605 | 188 → 2 | 13 113 157 → 0 | 4 → 2 (unjudged) | unread |

Budget: `tail_ms` and `j_ms` ≤ 100, `frames_after_release` ≤ 2,
`bytes_per_frame_held` ≤ 512 KiB. The raw-Kitty row misses `tail_ms` (A and
B) and `j_ms` by known gap `M7/46-kitty-no-deflate`.
