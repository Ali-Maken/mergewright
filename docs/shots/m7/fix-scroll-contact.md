# M7 fix: scroll on a big repo — contact sheet (m7-power-tools/47)

Its own file, not the next section of `m7-contact-sheet-*.txt`: that sheet is
text assembled from `m7-contact-sheet-captions.tsv`, and the Ghostty-path
capture is a PNG. Both captures come from `scripts/47-scroll-acceptance.sh`
step 5, on `TeamSpec::MONOREPO` (17 700 commits, his repo's shape) at 187×50,
after `j` held ×188 at 16 ms, before `J`. Numbers: `docs/perf/47-scroll-acceptance.txt`.

## 1. The Ghostty path: `fix-scroll-ghostty.png`

*Re-shot 2026-10-22 (ease-pass-21); the old picture was 850×407 px.*

**Right looks like:** the lit thread (the selection's lanes and dots) at full
strength, the off-thread lanes at half strength, no line wider than another,
the same rows as the Unicode grid below. **Colour unverified; Ghostty's own
rendering unverified until the user looks** (CLAUDE.md rules 5 and 6).

![Kitty+deflate frame at release](fix-scroll-ghostty.png)

What it is: every strip on screen as Graft sends it to a Kitty terminal with
deflate on — the strip backdrop with the selection's emphasis image over it,
built through `StripCache::composited` (ADR-0053: the picture the two Kitty
layers blend to) — stacked, cropped to the graph column (2 cells, 34 px: the strips are as wide as
the lanes in view (ease-pass 06 chain), not the 50-lane cap) and to the pane's rows 34–45
(history rows 178–189, 12 rows of 37 px; the cursor's, 188, is drawn: `cursor_on_screen=true`), over a stand-in dark
background (`#1E1E2E`), not his theme's. Drawn in-process by
`crates/graft/tests/it/ghostty_shot.rs`; Ghostty itself never ran.

**The cursor's row is in it** (re-taken by m7-power-tools/48). The cursor is
on history row 188; with the uncommitted row on top the pane now draws rows
142–188, so row 188 is the last drawn row (`cursor_on_screen=true` in the
SUMMARY line, `docs/perf/scroll-cursor-under-uncommitted-row.txt`). Before 48
the pane drew 141–187 and the cursor sat one row below it (47 NOTES Q1).
Whether it *looks* like the cursor's row is the user's call (rule 6).

## 2. The Unicode tier: `fix-scroll-release-unicode.txt`

**Right looks like:** the pane rows the PNG covers (screen rows 35–46, the pane's rows 34..46 counted from 0),
the graph column only (first 4 columns: the graph is as wide as the lanes in view, 2 here, so there is no `⋮`
divider to cut at); a `◉` merge dot in the lane column on each row, the cursor `▸` on the last row (46).
**The two do not line up row for row:** the PNG is the strips, 12 history rows 178–189 with the cursor's row 188
not the last (why not checked: ease-pass 13's tall row is the suspect); the text grid is 12 screen lines with the cursor on
the last. Characters only: the lit thread's colour and the cursor's fill are not recorded, so this grid cannot show
which row is lit. Colour unverified. Re-taken 2026-10-22 (ease-pass-21) by the pty text walk of
`scripts/47-scroll-acceptance.sh` step 5, run on its own (not the whole perf script).

```
35|  ◉
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
```

The PNG is drawn by the Pixel tier and this grid by the Unicode tier, from
two different runs, and their graph columns are not the same width (50 cells
in the PNG, 38 here plus the divider), so compare rows, not columns.

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
