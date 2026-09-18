# M2 design pass: details and diffs (2026-09-15) — New UI

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Previous: [Where I'd differ from DESIGN.md (Proposed changes)](04-where-id-differ-from-designmd-proposed-changes.md) · Next: [§8 coverage map](06-8-coverage-map.md)

---

## New UI

**Detail pane** (positions per DESIGN §1.1–1.3; content here). Regions:
*header* (fixed), *files* (list, focusable), *body* (message or diff, focusable).
- **Wide 160+:** right column, 42 % of width (min 48 cols), divider `│`.
  Header 4–5 lines: `COMMIT a1b2c3d` (accent when focused) · `Rui Mehdi <r@x> · 2026-09-13 14:02 +0500 · 2 min ago`
  (+ `committed by …` when different; `with: <co-authors>`) · `parents 9f8e7d6, 3c2b1a0 · diff vs first parent` ·
  `Contained in: main, release/2.1 +3 more` (`checking…` while the job runs) · radar line when any.
  Files below the header, up to 40 % of the column: first entry `✉ message (14 lines)` when the body has a
  non-trailer line, then a tree split on `/` (dirs collapsible with `Space`; `M/A/D/R/C/T` letter + `+12 −4` right-aligned,
  binary shows `bin`). Body takes the rest: the message when the cursor is on `✉`, else the selected file's diff. Cursor starts on the first file.
- **Medium 100–159:** pane below the graph, 40 % of height (min 8 rows), divider `─`. Header 2 lines
  (`COMMIT a1b2c3d · Rui M · 2 min ago · parents 9f8e7d6 · Contained in: main` / subject + radar). Files left (30 cols) | body right.
- **Narrow <100:** tab strip `[Graph] Detail` (DESIGN §1.3); Detail tab = header 3 lines, files ≤ 40 % height, body below.
- **Eye lands on:** the tinted diff lines; then the short SHA + subject. Chrome is rules, not boxes; the focused region's title is accent, others `text-muted`.
- **Keys:** `Enter` on a graph row → focus Files; `Tab`/`Shift+Tab` cycle Graph→Files→Diff; `Esc` → back to Graph (closes Explain first);
  `j/k ↑↓` move (files: loads that file's diff; body: scroll), `PgUp/PgDn/g/G`, wheel in each region; `Enter` on a file → focus Diff;
  `Space` folds a dir; `w` ignore-whitespace, `W` word tint, `v` unified/side-by-side, `[`/`]` context 0–20 (each `w`/`[`/`]` re-reads the file, options are per session and in `[diff]` config);
  `L` loads the rest past `max_lines` (5,000); `y` copies the SHA anywhere; divider drag, `<`/`>` shrink/grow the pane in 4-cell steps on the **focused divider** — the layout's one divider: columns in `Right`, rows in `Below`; a no-op in the `Tab` layout, with the pane hidden or the Map shown (ADR-0028) — remembered per orientation in `<common_dir>/graft/layout`; hints list `<> resize` at ≥ 100 columns.
- **States:** no selection → `select a commit` muted; loading → header from the row at once, files `reading…`; error → `git diff-tree: <first stderr line>` in `warning`, hints stay;
  binary → `binary · 12 KiB → 13 KiB`; truncated → `… 4,120 more lines · L load all`; untracked file under the working-tree spec → `untracked · no diff yet (M3)`.
- **Diff lines:** gutter `old new` numbers (width from the last line number, min 3), sign cell, text. `added`/`removed` soft bg, changed words a stronger bg (`W`), syntax roles as fg, hunk heading `@@ … @@ fn refresh()` in `text-muted`
  pinned on the top line while its hunk is scrolled through. CRLF shown as `␍` at the end. At 16 colours/`NO_COLOR`: sign + bold/dim only (m1/12c rule).
  Side-by-side: two halves, own gutters, `pairs()` alignment, unpaired lines blank on the other side, `│` between.
- **Compare (`x`):** `x` on row A marks it (accent `▏` left rule on that row; footer `compare base a1b2c3d — move, x again clears`); the pane follows the cursor:
  `COMPARE a1b2c3d → 7f6e5d4 · 14 files +120 −33`, files = `git diff-tree base head`. `Shift`/`Ctrl`+click a second row does the same. `Esc`/`x` on the base clears.
- **Working tree row selected:** `UNCOMMITTED · 2 staged 1 unstaged 1 untracked` (a zero count omitted) / `diff vs HEAD a1b2c3d`, no author, date, contained or radar lines; one diff vs HEAD (read-only; staging is M3).
  **Reaching it** (`working-tree-row-selectable`): the row stays where m1/12b put it — above HEAD, only while HEAD is the top visible history row (or unborn) and the tree is dirty. The selection is a flag beside `History`'s cursor (`AppState::on_working_tree`), never a history row:
  `k`/`↑` from the top visible row (cursor on HEAD) lands on it, the highlight moves up, HEAD's row un-highlights; `j`/`↓` returns to HEAD; `PgDn`/`G` leave it and move as usual; `k`/`PgUp`/`g` on it do nothing at the very top, else scroll up with the row leaving alongside HEAD;
  a click on its screen row selects it. It clears itself whenever the row is no longer shown (tree clean after `Ctrl+R`, HEAD moved, wheel scrolled HEAD off the top) or the history cursor moved (search hit, click, `G`), and the pane follows to HEAD's commit.
  On it: `Enter` opens the working-tree diff; `y` copies nothing; `x` is a no-op and a live compare shows the working tree with the base kept; `.` lists `Open details` only; Focus mode never fades it.

**Marker column** (graph screen): 2 cells between the graph column and badges, present only while the repo has any finding
(stable within a session, never shifts on scroll). `⚠` in `warning`, `ⓘ` in `info`; ASCII `!`/`i`; a marked row's footer hint gains `. explain`.

**Explain panel** (`.` → Explain, or right-click; only on a marked row): replaces files+body while open. Title `⚠ FOXTROT MERGE`
(severity colour, caps), paragraphs wrapped to width, then the evidence sketch (three lines like DESIGN §1.8: labelled short SHAs joined by `──╮`/`──╯`, `(this merge)`),
footer `Esc close`. The main graph fades every row not in `Explanation::highlight` (Focus-mode vocabulary; `Esc` restores). No Fix… button.

**Context actions (`.`, right-click):** popup list anchored under the row (bottom-left of the graph when no room): `Explain` (marked rows), `Copy SHA`,
`Compare with this (x)`, `Open details`. `j/k/Enter/Esc`, click. Width = longest item + 4, `surface-raised` bg, accent cursor.

**Map view (`M`)** — as DESIGN §5.1; fixed here: rows = local branches (current first, then default, then by tip date; cap 200, `+N more` in the footer), one column per bucket, `buckets = max(10, width − 24)`, time span = oldest tip… now, clamped to 1 day…366 days;
Unicode `▁▂▃▄▅▆▇█` by count/max (a non-zero count never blank), ASCII ` .:-=+*#%@`, colour = branch identity; the default row counts every commit in the span, any other row only commits off the default first-parent chain (a merged branch keeps its bars);
`●╮` (ASCII `o\`) at the tip's bucket when the tip is reachable from the default tip; tag marks `┴` + `[tag]` and radar marks `⚠`/`ⓘ` over the bucket; `▸` + ` ●` and the `NOW` header for the current branch; ` (active)` on a row whose tip is in the newest bucket; ` · partial` in `warning` when a cap was hit.
Header `MAP · <branch> · last N days|months` / `BRANCH … NOW`; footer `↑↓ select  ⏎ zoom to commits  M/Esc back`. `↑↓` select, `Enter` → `History::seek(tip)` in the normal graph, `M`/`Esc` back (data discarded). Empty: `no local branches`. Loading: `building map… n/N branches`, `Esc cancels`. Failed: the line in `warning`. Resize re-requests. No mouse in M2 (gap row).
Text glyphs at every tier in M2 (Proposed above).

**Overview strip (`O`)** — as DESIGN §5.3; fixed here: one column, the graph area's rightmost column in every layout (narrow: the screen edge; wide/medium with the pane shown: just left of the detail divider); bucket = `ceil(rows_total / strip_rows)` with `rows_total = Window::laid_out` (honestly incomplete while streaming); priority when a bucket holds several: `⚠` > `ⓘ` > `◆` search > `●` HEAD > `◉` merge > `█` viewport (only where nothing else drew) > `┆`; ASCII `! i * o + # :`; blank past the last bucket;
merges and radar known for resident rows only; not drawn while Map view is shown (no restore state needed); off by default, `O` flips it; click on the strip selects its bucket's first row.

**`?` help overlay:** not in M2 (spec fix 2026-09-18, below). Nothing in M2 draws one or hints at one.
