# Review: m8-polish-and-ship ticket 50, the app's words say Mergewright

Commit `27ac35c0` against base `3dadcbb0` (merged at `049d5f07`). Reviewer: read-only, 2026-10-12.
Ticket: `.scratch/m8-polish-and-ship/issues/50-mergewright-app-words.md`.

**Counts: Critical 0 · Major 1 · Minor 6 · Questions 1.**

## What was verified, and how

- **Gates: not run.** The conductor told me not to run cargo, because gates and a builder were already running. I have
  no gate, heavy-tier or perf result of my own. Everything below comes from reading the diff, the source at `27ac35c0`
  and the committed captures.
- **Source diff:** I read every non-test `crates/*/src` hunk (35 files). Each is a literal or doc-comment swap
  `Tributary` → `Mergewright`, apart from two changes. `APP_NAME` now comes from the names table
  (`graft-ui/src/graph_screen/header.rs:20`), and the status screen title uses it instead of its own literal
  (`status_screen/mod.rs:173`). The `Mergewright-Link:` doc comment in `graft-ops/src/snapshot/chain.rs:18` matches
  the writer, which uses `Generation::CURRENT.link_trailer()` (`chain.rs:38`).
- **Old names left in the source:** I ran `git grep` at `27ac35c0` over non-test literals in `crates/*/src` and
  `crates/*/examples`.
  - `\b(Tributary|TRIBUTARY|Trib|trib|GRAFT)\b` (env-var names excluded) found nothing.
  - Lowercase `tributary` in a literal is found only at `graft-ops/src/tour.rs:62` (see Major 1).
  - The `TRIB_*` env names still show in `cli/man.rs:93`. Ticket 51 owns them, so they are out of scope.
- **The 40 captures that changed:** I read `git diff --word-diff` over all of `docs/shots/`. 37 captures change only
  the word, and the padding absorbs the 2 extra cells: confirm boxes, the credential box (80, 120 and 160 columns),
  the refusal footers and the status snapshots all keep their width. Three captures shed a clause at 80 columns (Minor
  1). One capture has the tour header's stray letters (Minor 2, Question 1). I found no wrapped word and no misaligned
  column.

## Acceptance spot-check: 5 of 5 boxes checked, all 5 hold

| Box | Checked against | Holds? |
|---|---|---|
| Grep test | `crates/graft/tests/it/rename_names.rs:228-262`: reads `Generation::CURRENT.older()` and checks title and command as whole words over `src` and `examples`. `the_word_test_reads_words_only` (`:285-298`) gains `Tributary` and `trib:` cases. | yes. The allow-list count in NOTES is wrong (Minor 3), but the ticked behaviour holds. |
| Empty state | `Mergewright · no repository here` found in all of `docs/shots/m8-look-outside-repo-{120x36,187x50,no-recent-80x24,open-prompt-80x24}.txt` and `docs/shots/m8/first-launch-empty-state-120x36.txt` | yes |
| Captures and drift | `.scratch/m8-polish-and-ship/mwr-words-shot-drift.txt` summary line `changed 41 · name-only 1 · drift-files 40 · png 0` matches NOTES. NOTES names the 3 captures that shed a clause, and my word-diff confirms no others. | yes |
| `?` and `⌃P` | `crates/graft/src/update/palette/names_tests.rs:28`: every `?` row over a stopped merge, plus every default binding's label and search words | yes |
| `27-shots-left.txt` | `wc -l` = 450, matching NOTES | yes |

The ticket is a word swap governed by ADR-0057's Amendment, not a new feature. It has no journey, so the
pipeline's step counts do not apply.

---

## Critical

None.

## Major

### 1. The tour header still shows the old name `tributary-practice-…`, no ticket owns it, and the generalised grep test cannot see it

- **Where:** `crates/graft-ops/src/tour.rs:62` defines `pub const PRACTICE_PREFIX: &str = "tributary-practice-";`.
  `crates/graft-ops/src/tour/build.rs:53,58` uses it to name the folder. `crates/graft/src/view/tour.rs:126` draws
  the folder path in the header. The capture shows it at `docs/shots/m8-tour-187x50.txt:3`:
  `… practice-tmp/tributary-practice-1609459200/work · a throwaway repo, not yours`. At 80 columns the path is cut from
  the left, so the old name is visible only at wide sizes, which are his everyday sizes.
- **Why it matters:** the ticket's promise is "every word it draws". This one is drawn on the screen a new user sees
  first.
  - The builder flagged it honestly. But the KNOWN_GAPS row (`KNOWN_GAPS.md:1350`, uncommitted working tree) sends it
    to "51/55", and neither ticket mentions it: `51-mergewright-env-names.md` is about env vars and
    `55-mergewright-docs-sweep.md` is about docs. As things stand it would ship.
  - The test is case-sensitive by design (ticket §Decisions), so the lowercase on-disk word `tributary` passes it.
- **Knock-on:** renaming the constant is not a pure word swap. `tour.rs:215` refuses to delete a practice folder whose
  name does not start with `PRACTICE_PREFIX`. Whoever renames it must decide what happens to a `tributary-practice-*`
  folder left by an older build: read both prefixes, or let it age out of `$TMPDIR`.
- **Suggested fix:**
  - The conductor gives it an owner: ticket 52 or 54, or a one-line ticket.
  - Change the prefix to `Generation::CURRENT.dir()` plus `-practice-`. The deletion guard accepts any generation's
    prefix.
  - Widen `says_old_name` (`rename_names.rs:230`) to check `old.dir()` as well. That is the table's lowercase word.
    I grepped at `27ac35c0`: the only lowercase `tributary` literal outside `on_disk.rs` is `tour.rs:62`, so the
    wider check flags exactly this site and nothing new.

## Minor

### 1. The journal's footnote sheds its last word at 80 columns. For the user's eye.

- **Where:** `crates/graft-ui/src/journal_screen.rs:60`, the `NO_TRACE` string. It is 79 cells plus a 2-cell indent,
  which is 81 cells, one more than an 80-column screen holds. So it is cut to `… leaves no trace…` in
  `docs/shots/m3-journal-80x24.txt:33`, `docs/shots/m8-q1-journal-replaced-ui-80x24.txt:16` and
  `docs/shots/m8/21c-journal-partial-fetch-80x24.txt:21,53`.
- **Why it matters:** `here` is the word that makes the sentence about this list. The ticket correctly routes this to
  the user (§Width). No layout rule changed.
- **Suggestion:** drop `here` everywhere: `a branch deleted outside Mergewright and never checked out leaves no trace`
  is 74 cells and fits at 80. This is a wording decision, so it goes to the user, not the builder.

### 2. The tour capture contradicts its own caption: `◇ workri…` where the caption promises `◇ work` and a gap

- **Where:** `docs/shots/m8-tour-80x24.txt:3` and `docs/shots/m8-tour-187x50.txt:3`. Line 1 of each capture is the
  caption, which says "the header `◇ work` with a gap". The drawn row reads `◇ workri…`. Before ticket 50 it read
  `◇ workar…`.
- **Why it matters:** the caption and the picture disagree, and that stands on the capture alone (rule 7, textual).
  This is a pty capture of the real binary (`crates/graft/tests/it/tour_walk.rs:1,32`), not a TestBackend frame. So
  he would see the same thing in his terminal. The letters line up with columns 7 and 8 of the empty state's row 0,
  ` Mergewright · …`: `r` and `i` now, `a` and `r` in ` Tributary`.
- **Fix:** none until the cause is known. See Question 1.

### 3. NOTES gives the wrong allow-list count

- **Where:** the ticket NOTES say "23 entries (21 kept … 2 new)". `ALLOWED` in `crates/graft/tests/it/rename_names.rs`
  has 22 tuples at `3dadcbb0` and 24 at `27ac35c0`, counted by parsing the tuples.
- **Why it matters:** ticket 52 starts from that count when it removes the journal entries.
- **Fix:** correct the NOTES to 24 (22 kept, 2 new).

### 4. Doc comments describe a journal column that is not drawn yet

- **Where:**
  - `crates/graft-ui/src/journal_screen.rs:82,95` say the source column reads `` `Graft` / `outside Mergewright` ``.
  - `crates/graft-ui/src/journal_screen/view.rs:51` says "done in Mergewright".
  - What the code draws: `crates/graft-ui/src/journal_screen/pieces.rs:30,32` draw `Graft` and `outside Graft`, and
    `legend.rs:25` says `done in Graft`. Captures confirm it, for example `docs/shots/m7/journal-187x50.txt:3,49`.
  - `crates/graft-ops/src/apply/fold.rs:206` now names both products in one sentence ("outside Mergewright … entry
    of Graft's").
- **Why it matters:** these comments now describe ticket 52's result, not the code (PROMPT.md 6.2). A reader who
  trusts them gets the screen wrong.
- **Fix:** restore `outside Graft` in those three comments, or leave them for ticket 52 to change together with the
  code. Reword `fold.rs:206` to name one product.

### 5. Two copies of the whole-word matcher

- **Where:** `crates/graft/src/update/palette/names_tests.rs:14` (`names_word`) and
  `crates/graft/tests/it/rename_names.rs:217` (`says_word`) are the same function, byte for byte.
- **Why it matters:** if one is fixed later (for example, Unicode boundaries), the other will be missed. It is small,
  but it is the clever-ish code PROMPT.md 6.2 asks to keep in one place.
- **Fix:** move it to `graft_core::on_disk` as `Generation::named_in(text) -> Option<Generation>` (tested there), and
  have both tests call it.

### 6. The file-size ratchet was raised for a test-module declaration

- **Where:** `crates/graft/tests/it/over_400.txt` goes from 404 to 406 for `crates/graft/src/update/palette/mod.rs:403-404`
  (`#[cfg(test)] mod names_tests;`).
- **Why it matters:** the ratchet exists to shrink files over 400 lines. Raising it for a test line sets a precedent.
- **Fix:** declare the module from inside `palette/tests.rs` (`mod names;`, with the file at `tests/names.rs`), or
  add the test to `tests.rs`. Then return the ratchet to 404.

## Questions (rule 7: no line found)

### Q1. Why does the tour header show stray letters from the empty state?

`docs/shots/m8-tour-*.txt:3` is a pty capture of the real binary. Two cells of the header's gap still hold the empty
state's previous frame. Possible causes:

- the header leaves those cells marked as skip;
- ratatui's previous buffer and the real screen fell out of step when the tour opened (a terminal swap or a missing
  clear);
- the vt100 parser in `tests/it/pty`.

I did not locate the line. It existed before ticket 50, and it is already recorded in `KNOWN_GAPS.md:1349`
(uncommitted) for 56. Because this is a pty capture, it is probably visible in a real terminal. It is worth a
`debugger` pass before 56's recaptures, since a recapture would only reproduce it.

## Rule checks

- **Rule 3 (errors):** the new code is test-only (`names_tests.rs`, `rename_names.rs`). The non-test changes are
  literals only. I found nothing swallowed, and no new `unwrap` or `expect` outside tests.
- **Rule 7:** the builder filed the tour letters as a question with no line, which is correct. Every behavioural
  finding above names its line.
- **Colour:** unverified, as the ticket and rule 5 say.
- **Architecture:** `graft-ui` → `graft-core` already exists (`crates/graft-ui/Cargo.toml:10`). No new crate edge.
- **Not committed by the reviewer:** the worktree was in use (gates and a builder running), and the conductor commits
  between helpers.
