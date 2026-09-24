# Edits to `docs/design-notes/m6-conflicts.md` — spec pass 2026-09-24 (for the conductor to apply)

The planner has no Edit tool, and rewriting the 1,375-line note whole risks it. Each edit gives the line, the exact
old text and the exact new text. Apply them in one commit with the specs.

**One deviation from the dispatch, and why.** The dispatch said: un-comment `last-block-to-finished` and apply the
P11/P12 counts in the journey-counts block now. Doing it in this commit turns two gates red:
- `journey_counts_match_the_note` checks ids **both ways** (TESTING.md :2299), and `last-block-to-finished` has no walk
  scenario yet, so un-commenting it fails the test.
- `within_the_note` fails a journey that sends more keys than its row (TESTING.md :2290-2292). The walk still sends
  `c ⏎` for `a-finish` until P11/P12 are built, so lowering `a-finish` to 1 now fails it.

So this commit marks the rows **approved** in their comments and leaves the `keys` column as built. Each spec carries a
"For the conductor" block with the exact rows it lowers, applied in **that spec's own merge commit**, the one that makes
the walk send the new keys (the pattern ticket 52 already used). The note's own words asked for this (:912-914, and the
Touches item at :1173-1174).

---

## E1 — :859-860, the prototype gate is met
Old:
```
docs/log/m6.md:78. No source. **Pipeline step 5 comes before specs:** no ticket is written from this until he has run
the prototype below.
```
New:
```
docs/log/m6.md:78. No source. **Pipeline step 5 comes before specs:** no ticket is written from this until he has run
the prototype below. **Met 2026-09-24:** he ran it ("demo looks better than before", field notes :62-:63) and approved
the picks (:65, `.scratch/m6-conflicts/user-picks-2026-09-24.md`). Specs: "Specs, 2026-09-24" below.
```

## E2 — :458-461, J6 and J6b superseded
Old:
```
- **J6 finish, merge begun in the app.** From `every file done`: `c` → summary, cursor on the first row git combined ·
  `⏎` → commit. **2**. lazygit 1: the extra key is his choice. Lands: graph focused, cursor on the merge commit,
  `merged feature into main ✓`.
- **J6b finish, merge begun in the shell (P4).** `c ⏎`. **2**. Same landing, and one `FinishMerge` entry.
```
New:
```
- ~~**J6 finish, merge begun in the app.** From `every file done`: `c` → summary, cursor on the first row git combined ·
  `⏎` → commit. **2**.~~ Superseded 2026-09-24 (P11, P12 approved): the summary is already open, `y`, **1**
  (Amendment 2026-09-24, J6). Landing unchanged: graph focused, cursor on the merge commit, `merged feature into main ✓`.
- ~~**J6b finish, merge begun in the shell (P4).** `c ⏎`. **2**.~~ Superseded 2026-09-24: `y` **1** when the last file
  was done in the app; `c y` **2** from the banner. Same landing, and one `FinishMerge` entry.
```

## E3 — :479, :481, :482, the summary's `⏎` lines
Old (:479):
```
  - `⏎` → `committing…`, then the graph with the cursor on the merge commit.
```
New:
```
  - `⏎` → `committing…`, then the graph with the cursor on the merge commit. Superseded 2026-09-24 (P12): `y` in the
    file list, `⏎` in the subject; `⏎` on a file row shows its diff.
```
Old (:481):
```
  - The summary could not be read → `summary unavailable: <why> · ⏎ commits with git's message`.
```
New:
```
  - The summary could not be read → `summary unavailable: <why> · ⏎ commits with git's message`. Superseded
    2026-09-24 (P12): `· y commits with git's message`.
```
Old (:482): the whole line starting `  - \`⏎\` on a merge begun in the shell → as J6;`. New:
```
  - `y` on a merge begun in the shell → as J6; one `FinishMerge` entry. `u` after it → the merge is back in progress, resolved; header `MERGING`, banner focused, status `undo: finish merge of <from> ✓ · c finishes the merge`; `c y` finishes it again, 2 (ticket 41; P12, 2026-09-24: was `c ⏎`, still 2). No summary opens by itself after `u` (P11: the trigger is the event).
```

## E4 — :486-508, the journey-counts fence (keys unchanged; comments marked approved)
Replace the whole fence with this. Only text after `#` changes; the ids and the `keys` column are byte-identical, and
every whole row's comment still **starts** with its parts list, unchanged.
````
```journey-counts
# id                          keys  once-approved   # parts (whole rows) · best tool
a-merge-to-stop               2
a-stop-to-first-conflict      0                     # JetBrains 0
b-launch-to-first-conflict    1
one-file-one-conflict         1                    # JetBrains 2 · P11 (approved 2026-09-24): lands on the summary, count unchanged
three-conflicts-all-theirs    1                    # JetBrains Accept Theirs 1 · P11 (approved): lands on the summary, count unchanged
modify-delete-keep-yours      1                     # JetBrains 1
two-files-in-order            2                    # JetBrains 2 · P11 (approved): lands on the summary, count unchanged
two-files-out-of-order        3                    # P11 (approved): lands on the summary, count unchanged
a-finish                      2                     # lazygit 1 (bare confirm, rejected) · P11+P12 approved 2026-09-24 → 1: `y` in the summary the last file opened · lowered by last-file-opens-the-summary
b-finish                      2                     # P11+P12 approved → 1 (`y`) · lowered by last-file-opens-the-summary
summary-open-git-combined     1
back-after-esc                2
open-a-done-file              1                     # Fork #1304 asks for it (CR 1.10); no count in CR
walk-back-a-done-file         4                     # Tower reset-to-unmerged (CR 1.11): count (?) · P15 approved → 3 (`j d y`) · lowered by ticket 50
a-whole-one-file              5                    # a-merge-to-stop + a-stop-to-first-conflict + one-file-one-conflict + a-finish · P11+P12 → 4 (`m m O y`) · approved; lowered by last-file-opens-the-summary
a-whole-two-files             6                    # a-merge-to-stop + a-stop-to-first-conflict + two-files-in-order + a-finish · P11+P12 → 5 (`m m o T y`) · approved; lowered by last-file-opens-the-summary
b-whole-two-files             5                    # b-launch-to-first-conflict + two-files-in-order + b-finish · P11+P12 → 4 (`⏎ o T y`) · approved; lowered by last-file-opens-the-summary
editor-to-finish  5  # M3a (usability M3): the editor never opens to refuse, so the walk starts in the changes pane: c (refused in words) + 2×⇥ + c + ⏎, still 5 · P14 → 2 (c ⏎; c y with P12) · P14 approved 2026-09-24 → 2 (`c y`); lowered by ticket 52
b-finish-again-after-undo    2                     # from the u landing: c ⏎ (ticket 41) · P12: `c y`, still 2 · approved; keys re-sent by finish-summary-is-a-preview
# last-block-to-finished      4                     # Amendment 2026-09-24 J13: `o ⏎ c ⏎` · P11+P12 → 3 (`o ⏎ y`) · JetBrains ~4 (?) · VS Code 3–4 · lazygit 2 (bare, rejected). Approved 2026-09-24; un-commented at 3 by last-file-opens-the-summary, which adds its scenario
```
````
Rows added later, each by its spec's merge commit (not now): `b-abort-shell-merge`, `b-undo-a-take-in-the-shell`
(`undo-and-abort-a-shell-begun-merge`); `walk-back-a-whole-file-choice` (`start-over-a-whole-file-choice`).

## E5 — :912-914, the counts paragraph
Old:
```
- **The journey-counts block:** the `keys` column is unchanged until P11 and P12 are approved and built. The
  once-approved numbers sit in its comments. `last-block-to-finished` is added commented out, because the walk has no
  scenario for it yet.
```
New:
```
- **The journey-counts block:** P11 and P12 approved 2026-09-24. The `keys` column still holds the as-built counts;
  each spec lowers its own rows in the commit that makes the walk send the new keys, because the walk fails a journey
  over its row and an id with no scenario. `last-block-to-finished` stays commented until
  `last-file-opens-the-summary` adds its scenario and un-comments it at 3.
```

## E6 — after :914, three journeys from the batch items (new text; insert before "**Where each event lands him")
```
- **J15 abort a merge begun in the shell** (batch 3, approved 2026-09-24). The banner focused: `a` · `→ ⏎` **3**; `a y`
  **2** once ticket 50 lands. Lands: the graph, the abort's `Done` line, no `MERGING`; one `AbortMerge` journal entry,
  and `u` puts the merge back, conflicted, his hand edits included. Best: lazygit `m` → abort → confirm, 3 (?).
- **J16 get a file back after a whole-file take, in a merge begun in the shell** (batch 3). A whole-file `t` on
  `gone.txt` deleted what he had kept by hand; the banner focused (after `Esc` from the resolver): `u` **1** (from the
  resolver: `Esc u`, 2 — the resolver does not bind `u`, keymap §2.6). Lands: the banner stays focused, status
  `undo: <the take's description> ✓`; the file on disk is his again. Only Graft's journaled steps come back this way
  (whole-file choices, `d`); a block take is an fs write inside the merge, not an entry. JetBrains: `⌃Z` in the merge
  dialog, 1.
- **J17 walk back a whole-file choice** (batch 7). The resolver on `notes.txt` after `gone.txt` was kept: `k` · `d` ·
  `→ ⏎` **4**; `k d y` **3** once ticket 50 lands. Lands: the card `deleted by them · o keeps the file  t deletes it`,
  the list `keep or delete`, `git ls-files -u` shows the file's stages again. Tower reset-to-unmerged, count (?).
```

## E7 — :950-951, α is picked
Old:
```
- **Tint = mix(fg, `bg`, α).** α is 0.30 by default; he picks 0.20, 0.30 or 0.40 in the prototype, and all three tints
  share it.
```
New:
```
- **Tint = mix(fg, `bg`, α).** α = 0.30, his pick 2026-09-24 ("medium"), shared by all three tints (ADR-0044 §1).
```

## E8 — :1103-1104, hard-to-reverse #2 decided
Old:
```
2. **Opening the summary on the last file (P11).** Options: stay in the resolver (P8), or open the summary. **Open it.**
   The code is easy to reverse, but it trains his hands, so he decides on the prototype.
```
New:
```
2. **Opening the summary on the last file (P11).** Options: stay in the resolver (P8), or open the summary. **Open it.**
   The code is easy to reverse, but it trains his hands, so he decides on the prototype. **Decided 2026-09-24: open it**
   (his pick; task-mode log: the summary opened by itself and his first keys there were `k`, `⏎`, then `y`).
```

## E9 — :1107-1108, the crate line (the batch items widen it)
Old:
```
- **Crates:** `graft-ui` and `graft` only. `graft-ops`, `graft-git`, `graft-core` and `graft-term` get no signature
  change, and `FinishSummary` is unchanged. Check: `git diff --stat` names none of them.
```
New:
```
- **Crates:** for P11–P13, `graft-ui` and `graft` only. `graft-git`, `graft-core` and `graft-term` get no signature
  change, and `FinishSummary` is unchanged. Check: `git diff --stat` names none of them. The batch items 3 and 7
  (approved 2026-09-24) touch `graft-ops`; their specs say how.
```

## E10 — :1146 and :1151-1152, the keymap and this note
Old (:1146): ``- **Keymap `02-keymap.md`** (the conductor edits it after approval):``
New: ``- **Keymap `02-keymap.md`** (edited in the spec pass, 2026-09-24; ADR-0044):``

Old (:1151-1152):
```
- **This note:** Amendment 2's summary mockup, J6, and its `⏎`-commits event lines are superseded where P11 and P12
  are approved. The conductor marks them.
```
New:
```
- **This note:** Amendment 2's summary mockup, J6, and its `⏎`-commits event lines are superseded where P11 and P12
  are approved. Marked 2026-09-24 (spec pass, E2-E3).
```

## E11 — :1177, PROMPT.md
Old: `- **PROMPT.md §5:** the token list (P13), edited by the conductor after approval.`
New: `- **PROMPT.md §5:** the token list and the colour-depth exception (P13); exact text in ADR-0044 §5, applied by the conductor.`

## E12 — :1344-1363, Proposed changes approved
Old (:1344): `### Proposed changes (not decided; specs wait for his word on the prototype)`
New: `### Proposed changes (P11–P13 approved 2026-09-24: "ok i guess", field notes :65; ADR-0044)`

Old (:1345): `- **P11: finishing the last file of a merge in the resolver opens the summary on its own.**`
New: `- **P11 — approved 2026-09-24: finishing the last file of a merge in the resolver opens the summary on its own.**`

Old (:1352): ``- **P12: the summary confirms with `y`, and `⏎` on a file row shows its diff.**``
New: ``- **P12 — approved 2026-09-24: the summary confirms with `y`, and `⏎` on a file row shows its diff.**``

Old (:1359): `- **P13: six colour tokens added to PROMPT.md §5.**`
New: `- **P13 — approved 2026-09-24: six colour tokens added to PROMPT.md §5** (exact text: ADR-0044 §5).`

## E13 — after :1374 (end of the note), the spec list
```
### Specs, 2026-09-24 (unnumbered, `.scratch/m6-conflicts/issues/`)
Wave 1, two lanes in parallel (disjoint crates, ADR-0036):
- **Lane U — `graft-ui` only, in order:** `conflict-colour-tokens` → `conflict-band-and-state-colours` →
  `hint-words-keep-the-way-out`. One builder can take all three in one worktree. If a scout finds a `graft` source test
  (not a capture) that pins what a U spec changes, that spec reports `blocked: needs graft` and runs after lane O.
  Both lanes regenerate files under `docs/shots/m6/`; after the second lane merges, the conductor re-runs the capture
  tests once and regenerates any file both touched.
- **Lane O — `graft-ops` + `graft`, in order:** `undo-and-abort-a-shell-begun-merge` →
  `start-over-a-whole-file-choice`.
Wave 2, one at a time (`graft` + `graft-ui`): `finish-summary-is-a-preview` → `last-file-opens-the-summary` → 52
(P14) → 50 (P15) → 51 (P16) → `resolver-words-and-marks`.
Blocked: `short-words-from-the-draft` (batch 4 against the m5/27 prefixes-only ruling; a question).
Last: `acceptance-look-and-finish` (Fable).
Genuinely risky: `undo-and-abort-a-shell-begun-merge` (a git-write safety path: the undo gate while a merge is live
decides what `u` may rewrite under it; correctness is not visible in a diff). `last-file-opens-the-summary`'s trigger
(one decision point; the walk judges it).
```
