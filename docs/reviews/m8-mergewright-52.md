# Review: m8 ticket 52 — git messages say `mergewright`; J's column says Mergewright / outside

Commit `1faf4791` (merged `ee526ca2`) against base `f2809cf0`. Ticket
`.scratch/m8-polish-and-ship/issues/52-mergewright-git-messages-and-journal-column.md`, scout
`.scratch/m8-polish-and-ship/scout-52.md`. Reviewer: read-only, **no cargo run** (conductor's instruction):
every claim below comes from reading the source, the diff and the captures, not from running anything.

**Counts: Critical 0 · Major 0 · Minor 7.**

## The focus question: can an older-generation set-aside or backup go missing?

**No path found where it does.** No production code reads one of our messages to decide anything, so the
tag change cannot make a reader miss one:

- `Generation::of_message` has **no production caller** (`git grep of_message` → only `on_disk.rs:159`, its
  tests, and the two test files). `MESSAGE_TAG` is used only at the six writers.
- The set-aside stash is found **by commit id** everywhere I looked:
  `graft-ops/src/apply/set_aside.rs:60` `stash_of` (from the journal entry's `PutBack { stash }`),
  `set_aside.rs:348-353` `listed_at` (`*commit == stash`), `apply/recover.rs:178-180` `left_at` (same),
  `graft/src/update/undo/notice.rs:238,266` (same). The S list (`graft/src/view/stash.rs:60`) only draws the
  message.
- Backups are found **by ref**, mapped across generations at read (`graft-ops/src/rename/map.rs:13-24`
  `map_ref`, `map_entry`; `rename/journals.rs:15-38`). The backup commit's subject and author
  (`snapshot/plumbing.rs:298-311`) have no reader (`git grep BACKUP_AUTHOR` → definitions only).
- History, search, radar: the radar walks an allowlist of refs (`mistake_radar/probes.rs:397-405`), fetch
  skips internal refs by name (`apply/fetched.rs:60-66`). I found nothing that skips commits by message.

The builder's reader table in NOTES agrees with the code at every row I checked (6 of 6).

## Does test 1 catch a regression?

`graft-ops/tests/it/set_aside_generations.rs::an_older_tag_is_still_ours` gives the stash an older tag, then
runs recovery and `u`. The regression that would actually hurt is a reader that starts filtering the stash list
by `MESSAGE_TAG`, for example inside `graft_git::stash::commits`, or a put-back that looks the stash up by its
message. The test **would catch that**: `recover_interrupted` would return no `EditsAt::Stash { n: 0 }`
(`:76-80`), or `run_undo` would leave the stash listed (`:86-91`). It does not cover
the `graft` crate's undo notice (`notice.rs`) or the S overlay. Both match by id today, so this is not a gap
in what the test proves. It is a limit on where a future regression would show up. Weaknesses are in Minor 2.

## Acceptance spot-check

Checked **6 of 6** boxes; **6 held**, judged by reading the named tests (none were run):

1. `set_aside_generations::an_older_tag_is_still_ours`: exists. It loops over `Generation::CURRENT.older()`,
   and git truth comes from `git stash list`, `whole()` and the file bytes. The test does not use the box's
   "made by hand with `git stash push -m`". The deviation is stated in the box and in NOTES, with the reason:
   a stash with no journal entry is no one's to offer. **Held.**
2. `new_writes_say_mergewright`: checks that the stash `%gs` and the backup `%s` start with the tag. **Held.**
3. `rename_names::no_old_message_literal`: exists (`rename_names.rs:229`), and the reader table is in NOTES.
   I could not verify the "seen red" claim without running it. By reading, a `"graft: put-back forecast"`
   literal would match `of_message` → `Graft ≠ CURRENT` (`:242-243`). **Held.**
4. `journal_screen::tests::source_column::source_column_words`: covers `●  Mergewright` and
   `$  outside        5m ago` at 187, the legend at 80, and no `Graft` at either width. **Held.**
5. The allow-list dropped from 24 to 18 entries. I counted the tuples in `ALLOWED` at `f2809cf0` (24) and at
   `1faf4791` (18), and the six removals in the diff match the box. **Held.**
6. `capture_source_column_for_docs_shots`: exists, and both files are in `docs/shots/`. The box does not
   claim they look right. **Held.**

## Critical

None.

## Major

None.

## Minor

1. **`crates/graft-ui/MAP.md:454`: `source_word` is listed under `## palette`.** It is a duplicate of the
   correct line at `:366` (`## journal_screen`), and it sits next to the palette's own, unrelated `RowKind`.
   *Why:* MAP is how the next builder finds things. A palette `source_word` that does not exist sends them to
   the wrong module. *Fix:* delete `:454`.

2. **Test 1 pins existing behaviour but was never seen red, and its loop can pass vacuously.**
   `set_aside_generations.rs:55` has `for older in Generation::CURRENT.older()` with no check that the loop
   ran. If `older()` ever returns nothing (a wrong filter, a reordered `GENERATIONS`), the test passes with
   zero assertions. NOTES says it "passed as soon as it compiled", and no mutation is recorded. *Why:* a test
   that has never failed is a hypothesis about what it catches (rule 1). *Fix:* assert that the loop ran
   exactly `Generation::GENERATIONS.len() - 1` times. Record one seen-red, for example a temporary
   `starts_with(MESSAGE_TAG)` filter in `listed_at` (`apply/set_aside.rs:350`) failing it at `:86-91`.

3. **`no_old_message_literal` does not enforce the ticket's "no literal `mergewright` at a writer".**
   `rename_names.rs:242-243` flags earlier generations only. A writer that hard-codes
   `"mergewright: index"` passes, and the next rename would miss it. That is exactly the silent drift the
   `MESSAGE_TAG` const exists to prevent. *Fix:* in `graft-ops/src`, also flag literals where
   `of_message(..) == Some(CURRENT)`, outside `on_disk.rs`.

4. **(textual) At 80 columns the legend no longer explains `✗`, yet `✗` rows are on screen.**
   `docs/shots/m8-mergewright-journal-80x24.txt:34` ends `$ done outside Mergewright: u never…`, while `:15`
   and `:17` draw `└ ✗ lost` rows. Before this ticket it reached `✗ a…`, which was barely more use. The
   builder followed "shedding order unchanged" and flagged it, so this is a design question, not a build
   defect. *Why:* visibility (ux-principles): a glyph whose meaning is cut off. *Fix (design, via the
   conductor):* a narrow wording, e.g. `● Mergewright: u undoes · $ outside: u never touches · ✗ no branch
   holds it`, or shed words inside each clause before cutting whole clauses.

5. **(textual) From 120 to 137 columns, `⇢` rows are drawn but the legend's `⇢` clause is not.** The whole
   legend plus the clause is 138 cells (`m8-mergewright-journal-187x30.txt:40` measures 138). `legend_at`
   (`legend.rs:37-45`) drops the clause below that, and the source column starts at 120
   (`journal_screen.rs:105`). The design mock now shows this state: `01-view-mockups.md` §1.10 draws a
   `└ ⇢ replaced` row with no clause, and its prose says so. *Fix:* route with 4. The same shorter wording
   gets the clause back at 130.

6. **(textual) Captions and captures left disagreeing.** `docs/shots/m7/m7-contact-sheet-captions.tsv:1,2,7,12`
   still quote `●  Graft`, `outside Graft` and `no trace here`, but the m7 shots they caption were regenerated
   by this ticket and now read `Mergewright` / `outside`. A rebuilt m7 sheet would put the old words over the
   new pictures. `docs/shots/m3-08-journal-after-unstage-{80x24,160x48}.txt` still draw
   `outside Graft … no trace here`. The builder names them stale, but neither 55 (docs sweep) nor 56 (M8's
   pty captures) lists them (grep finds `m3-08` only in 11f and 52). *Fix:* add the m7 captions TSV to 55, and
   the m3-08 re-shot to 56, or record them as a gap.

7. **(textual, question) The footnote now overclaims.** It reads `a branch deleted outside Mergewright and
   never checked out leaves no trace` (`journal_screen.rs:61`). Dropping `here` (review 50's suggestion) made
   it fit in 80 columns, but it turned "not in this list" into "nowhere". The commits still exist,
   unreferenced, until gc. Optional: `… leaves no trace in this list` measures 89 cells with the 2-cell
   indent, so it does not fit 80. If the absolute wording is a concern, a shorter rewrite is the user's or
   the designer's call.

## For the conductor (not findings)

- KNOWN_GAPS rows `M8/27-git-messages-graft` (`:1321`) and `M8/27-j-column-graft` (`:1320`) are still open.
  The builder does not edit KNOWN_GAPS, and this review finds both closed by `1faf4791`.
- Rule 3: the diff adds no `let _`, swallowed `Err` or `unwrap_or_default`. The new `unwrap_or`/`expect` are
  in tests only. Rule 5/6: both new captures say colour is unverified and leave the look to the user.
- Architecture: `graft-ui` reads `graft_core::on_disk::Generation` (`view.rs:74`, `legend.rs:11`). That is
  core → ui, which is allowed. No new crate edges.
- Gates: **not run** (instruction). The heavy tier and the release perf run were not run either.
