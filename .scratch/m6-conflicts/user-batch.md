**Triaged 2026-10-24** (`.scratch/ease-pass/scouts/user-batch-triage.md`): 1–5, 7, 10, 11 stale; 8 stale except Shift+Tab unconfirmed; 9 unclear (re-capture at 120 cols); 6 answered by the user 2026-10-24: drop the origin, keep the count (current behaviour).

# Questions for you at the end of the conflict-fixing work
Numbers (1)–(8) match the list kept during the build. Items 9–11 were filed later.

1. The conflict screen (where you pick a side for each clashing block) is built. Captures are in `docs/shots/m6/`.
   Does it look right? Look at: the whole screen, the note row on the card for a whole-file conflict, and the `┴` mark on the closing line of a block you already took.
   Options: looks right / change <what> (PROGRESS.md:11 (1); KNOWN_GAPS M6 triage row 591, docs/log/m6.md:143)
2. After the last open block, `n` says `no more conflicts · ⏎ marks <file> done`. If you went backwards with `N`, earlier blocks in the same file are still open, so the message can mislead. One capture also shows `⏎ marks zdiff3.txt done` twice, in the message and in the key row.
   How should it read? Options: `no more conflicts this way` / keep as is / your wording (PROGRESS.md:11 (2); docs/log/m6.md:78; KNOWN_GAPS M6/15)
3. A `git pull` in the shell stops on conflicts, and you fix them in Graft. If `t`/`d` overwrote a hand edit, Graft keeps a backup, but while that merge is still open, undo (`u`) and abort (`a`) both refuse until you run `git merge --abort` in the shell. (Finishing it now works: `c ⏎` on the banner, and `u` puts it back.)
   Should undo and abort work on a merge you started in the shell, before it's finished? Options: yes, both / abort only / no, keep the shell step (PROGRESS.md:11 (3); KNOWN_GAPS M6/08, M6/07)
4. On a narrow screen the key hints drop whole words. The fix is to shorten them first (`space stage` → `space stg`). Shorter words were drafted on 2026-09-21 and are waiting for your edits.
   Will you mark up the short words? Options: approve draft / edit them / drop the idea (PROGRESS.md:11 (4); KNOWN_GAPS "shrink before dropping" row, For the user)
5. Go on to the next milestone? Options: go / not yet, first fix <what> (PROGRESS.md:11 (5))
6. With the changes pane open on a 187-column screen, the stop line drops `begun outside Graft` / `from a previous run` so that `13 files conflicted` shows in full. It also drops them at 120 columns.
   Is that OK? Options: OK / the origin matters more, keep it / shorten both (PROGRESS.md:11 (6); KNOWN_GAPS M6/21)
7. `d` (start this file over) only works on files where both sides edited lines. It does not work where one side deleted the file or added it whole, so walking back there is not possible. Proposal: offer `d` there too, with the box `the choice comes back: keep or delete`. It would still not work in merges started in the shell.
   Want it? Options: yes / no (PROGRESS.md:11 (7); docs/design-notes/m6-conflicts.md:830)
8. The screen shown before finishing the merge (the message and the file list) scrolls only with its listed keys. `↑`/`↓` between files, `PgUp`/`PgDn` and `Shift+Tab` back do nothing there, although they work everywhere else.
   Add them? Options: yes, all three / some / no (docs/log/m6.md:246; .scratch/m6-conflicts/issues/19-*.md:203-207; not yet in the design note)
9. At 120 columns, the conflict screen's key row cuts the words after `d` (start over) and `esc` (close) when it also shows `B base`.
   Which fix? Options: A drop words from everyday keys first / B shorter words (`tool`, `restart`) / C leave it (.scratch/m6-conflicts/user-facts-m632.md)
10. The file list marks an unvisited file with `…` after its name. Next to `… 7 more` it can look like a cut-off name.
    Options: keep `…` / use another mark (KNOWN_GAPS M6/13)
11. After you stage the last file, the key row shows ` stage <file> ✓` until your next key, and not `c finish`. The line under the title does say `c finishes the merge`.
    Options: fine / show `c finish` right away (KNOWN_GAPS M6/31)
