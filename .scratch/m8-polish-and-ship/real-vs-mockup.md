# M8: real app vs mock-up, same renderer

Real Graft (dev build of main at 64a14709) run in a pty at 120x36, drawn with pyte + Pillow, same colour table as `../m8-shots/shoot.py`.
No graphics protocol in the pty, so Graft draws its Unicode tier: the daily view in Zed / VS Code terminals.
Test repos made with plain git (3 branches, 2 merges, 1 staged + 1 unstaged file; a clone stopped in a merge with README conflicted).
Colour is the renderer's guess of each terminal colour, not a real terminal (rule 5).

**Rev 2 (fair font).** Real *and* mock-up shots are re-rendered with one font, JetBrains Mono Nerd Font Regular/Bold 14 (`~/Library/Fonts`), which has `│ ─ ┴ ╭ ╮ ╯ ╰ ┼ ●`. Mock-ups re-shot into `mock/` (same shoot.py, font swapped; the old Menlo set in `../m8-shots/` is untouched). Glyph check: a 2x crop of r01's lane area showed lanes joining across rows with rounded corners. Only `ⓘ` (U+24D8, one cell in r07) still falls back to Arial Unicode.

Renderer caveat: pyte has no alternate screen: `r08` keeps the last frame after `q`; a real terminal restores the shell.

### Keys sent in the main run (one process, r01–r05, r08)

Times from process start; "rows changed" = screen rows that differed before vs after the wait.

| t (s) | key | wait | rows changed |
|---|---|---|---|
| 4.48 | `jjjj` | 0.4 | 3 (cursor moved, r02) |
| 5.36 | `⏎` | 1.0 | 7 (detail opened, r03) |
| 6.86 | `Esc` | 0.4 | 7 (detail closed) |
| 7.32 | `?` | 1.5 | **0** (r04) |
| 9.33 | `Esc` | 0.4 | 0 |
| 9.79 | `g` | 0.4 | 3 |
| 10.21 | `k` | 0.4 | 3 |
| 10.62 | `⏎` | 1.0 | 8 (staging opened, r05) |
| 12.13 | `Esc` | 0.4 | 8 |
| 12.54 | `q` | 0.8 | 0 |

So pty input works (j, ⏎, g, k, Esc all changed the screen in the same process); `?` alone changed nothing. A separate fresh run (`?` as the first key, 2.0 s wait, full-screen text dump) also showed no overlay.

## Files

Real app, alone:
- `r01-graph-open.png` graph on open
- `r02-cursor-moved.png` cursor down 4 rows
- `r03-commit-detail.png` Enter on a commit: detail + diff
- `r04-help.png` after `?` on the graph, 2 s wait: nothing visible opened
- `r05-working-tree.png` uncommitted row opened: staging screen
- `r06-outside-repo.png` started in a non-repo folder
- `r07-conflict.png` opened on a repo stopped in a merge (README conflicted)
- `r08-quit.png` after `q` (see caveat)

Pairs (real left, mock-up right):
- `p1-graph-vs-scene1.png` r01 vs mock/01-scroll-start
- `p2-help-vs-scene2.png` r04 vs mock/04-keys-help-list
- `p3-outside-vs-scene3.png` r06 vs mock/07-firstrun-outside-repo
- `p4-colours-vs-scene4.png` r01 vs mock/12-themes-dark-a-b

All files under 100 KB.

## Visual differences seen in the pictures

1. **Header.** Real: one line, `graft` + full absolute path, cut off with `…` (r01). Mock-up: a bold repo name `◆ billing-api` then a short `~/code/billing-api · main`, with a blank line and a rule before the list (04-keys-help-list, 12-themes-dark-a-b).
2. **Bottom bar.** Real: one row of ~35 bare letters (`o b t P c p f u S / R q F2 F3 x + …`) running off the right edge; only the first four carry words (r01, r08). Mock-up: a thin rule, then a short line of words with ` · ` separators that fits the width (07-firstrun-outside-repo, 01-scroll-start).
3. **`?`.** Real: nothing visible opens (r04). Mock-up: a framed box over the graph, title `keys here`, `esc close`, sections HERE / EVERYWHERE, one key per line with a plain-words action (04-keys-help-list).
4. **Outside a repo.** Real: two plain lines of error text at the top-left, rest of screen empty, no keys offered (r06). Mock-up: `graft · no repository here`, four keyed actions (Open, Init, Clone, practice tour), a RECENT list with a highlighted row, and a hint bar (07-firstrun-outside-repo).
5. **Margins and spacing.** Real: content starts in column 0 with no left gutter, no blank rows or rules between header, list and hint bar (r01, r05). Mock-up: 1–2 column left gutter, blank rows and horizontal rules separate the regions (07-firstrun-outside-repo, 12-themes-dark-a-b).
6. **Colour use in the list.** Real: every row has a yellow `DE` author tag and a grey age; the branch label on the cursor row (`[feature/refunds]`) is dim magenta on the grey highlight and hard to read (r01). Mock-up: author names and dates in grey, branch labels in bold blue, legible on the cursor row (01-scroll-start, 12-themes-dark-a-b). Graph lanes: real uses magenta/teal/light blue dots; mock-up blue main lane with orange/salmon side lanes.
