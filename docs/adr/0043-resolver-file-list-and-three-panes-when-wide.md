# ADR-0043 — The resolver: a file list always, three panes when wide, the card below

Date: 2026-09-23
Status: accepted (the user's proposal on the prototype walk, 2026-09-23; audited in
`docs/research/conflict-layout-audit.md`; walked as prototype v2). Supersedes ADR-0042.

## Context

ADR-0042 made the resolver one file drawn as a document, each conflict a card with
OURS and THEIRS inside it, and rejected three columns. On the prototype walk the user
said the resolver was *"confusing at which file I was talking with"* and asked for
JetBrains' layout: yours | result | theirs, the chosen side flowing into the middle,
plus a list of files (field notes, 2026-09-23).

The audit (`conflict-layout-audit.md`) found:
- The rejection leaned on VS Code #160806, which changed only the **default**; it is
  not evidence against three panes for a user who asks for them (§0).
- We dropped DESIGN's `CONFLICTED FILES` strip, so the file you are in lived in one
  header row that changed silently between files (§C).
- Each side had 3–4 names in 3 rows (`ours = main …`, `OURS · HEAD`, `main`), and
  line numbers belonged to no side: they skipped every marker line (§C).
- Three panes save no keys. They answer "which is mine, theirs, the result" by
  **position**. Below about 50 characters a pane, code gets cut off (§B).

## Decision

1. **A file list is always on screen.** It shows every file this operation conflicted
   on, with its state (`2 of 3 open`, `⏎ marks done`, `done ✓`, `keep or delete`), and
   the cursor file marked `▸`. Below 180 columns it is a **strip** above the document:
   one row when it fits, otherwise a grid of up to 3 rows ending `… N more`. At 180
   columns and wider it is a **column** on the left. Files stay listed as `done ✓`
   until the operation ends. Where the list comes from: git's unmerged paths, plus the
   `Conflicts:` list git writes into `MERGE_MSG`, plus the live entry's backup chain.
   This way a relaunch or an operation begun in the shell lists the same files.
2. **Three panes at `THREE_PANE_MIN = 180` columns and wider:** yours | result |
   theirs. **Rows line up by git's own regions**, so no 3-way diff is needed. A plain
   region (git has already merged it) is drawn on the same rows in all three panes. An
   open block takes `max(yours, theirs)` rows, and a taken block takes enough rows for
   the most lines any pane shows for it. Shorter panes are padded with blank rows that
   have no line number. In the result pane an open block is a placeholder
   (`? open · o t b`, or its BASE lines when `B` is on). A take fills that placeholder
   in: the chosen side goes into the middle. This removes ADR-0042's reason 1. That
   reason was about aligning three *independent* files, and this design never does.
   ~~At 180 columns each pane is 46 characters; at 200 columns, 52. The number is a
   constant that the user's eye tunes.~~ **Amended 2026-09-24** (ticket
   `14-resolver-three-panes` S2; 46 did not follow from the list column): Each pane is a
   third of what the list column and two `│` separators leave, the remainder to the
   result, its line-number gutter included: 48–53 columns at 180, 55–59 at 200. The list
   column's clamp (22–32) is the knob the user's eye tunes.
3. **Below 180 columns, the card** (ADR-0042's document, unchanged in shape: sides
   paired at 100 columns and wider, stacked below 100), with the fixes from §C:
4. **One name per side, everywhere:** pane titles, card labels, headers, hint verbs,
   list states and the finish summary. The name is `<role> · <name>`. The role word
   comes first and is also the hint's short form, so the hint ladder's prefix rule
   holds:

   | operation | left (stage 2) | right (stage 3) |
   |---|---|---|
   | merge, pull | `yours · main` | `theirs · feature` |
   | rebase | `upstream · main` | `yours · 3f2a1b Add OAuth provider` |
   | cherry-pick | `yours · main` | `picked · 3f2a1b …` |
   | revert | `yours · main` | `reverting · 3f2a1b …` |
   | stash pop | `yours · main` | `stashed · stash@{0}` |

   The prototype's `main (yours)` is **not** used. Keymap §2.1 says trailing
   parentheses carry a key, never a word. The rebase explainer line is gone, because
   the names now say who owns which side (ADR-0041's carve-out is still met). Git's
   raw marker labels (`HEAD`, `Stashed changes`) are no longer shown.
5. **Every pane numbers its own lines, with no gaps.** Each side is numbered as the
   file would read if that side were taken in every block (`Projection::Ours`,
   `::Theirs`). The result pane counts a taken block as its taken lines, and an open
   block as its *yours* lines. The open-block placeholder draws no number, so rows stay
   level across the panes until he chooses, and a take renumbers the lines below it.
   In the card, plain and taken lines carry the result's numbers, and the card's two
   sides carry their own. Rows that stand for markers never draw a number. `e` still
   opens `$EDITOR` at the block's line on disk.
6. **Carried over from ADR-0042, unchanged:** a take writes the file immediately
   (temp file + rename, fingerprint-checked), and nothing is buffered. No inline editor
   is built (`e` is `$EDITOR`). `B` shows base, `d` starts the file over, `Esc` closes
   the resolver and never aborts, and `⏎` marks the file done.

   **Amended 2026-09-24** (ticket `14-resolver-three-panes`, D1–D6): whole-file shapes
   at ≥180 columns draw their card row and note full width across the panes; a side
   with no file shows `(not on <side>)` (added-by) or `(deleted on <side>)` (deleted),
   carried as `RegionView::WholeFile { ours_absent, theirs_absent: Option<SanitizedText> }`.
   With `B` on, the §2 placeholder reads `base` over muted base lines, not `? open · o t b`;
   a binary result reads `? open · o t`. A take does not yet advance the cursor (`n`
   follows `t`) until ticket 15 (P2).

## Consequences

- **The yours and theirs panes are projections of the conflicted file, not git's
  stage files.** Where git merged a change from the other side cleanly, the yours pane
  shows that change too, because it is on a plain row. That is honest: a plain row is
  identical in all three panes, and the finish summary's *combined by git on its own*
  section is where those changes are named. Showing the true stage files would need
  the three-way alignment this ADR still refuses.
- `graft-ui::conflict_resolver::layout` gains an `Arrangement` argument, and the app
  scrolls and hit-tests through the same rows it draws, as before.
- Keys this layout invites are **not** decided here. `O`/`T` (take the whole file
  and mark it done), `j`/`k` (move between files) and `c` (finish, from inside the
  resolver) are Proposed changes P6–P8 in `docs/design-notes/m6-conflicts.md`
  Amendment 2. Until approved, the list is moved with `n`/`N` and the mouse.
- `docs/design/01` §1.5 is to be rewritten from this ADR, and ADR-0042's §1.5
  replacement goes with it.
- Rule 6 still applies: the user judges the look on the captures at 120, 160 and 200
  columns. Colour and reverse video are **unverified** in captures (rule 5).

## Related

ADR-0042 (superseded), ADR-0038 §3 (`$EDITOR`), ADR-0039 (the hint ladder's prefix
rule), ADR-0041 (carve-out), `docs/research/conflict-layout-audit.md`.
