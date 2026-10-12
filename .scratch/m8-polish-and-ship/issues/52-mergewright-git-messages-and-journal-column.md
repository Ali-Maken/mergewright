# Mergewright 5: messages written into git say `mergewright`; J's column says Mergewright

Status: done
Blocked by: 48, 51
Build with: **Opus**. Git writes and the readers that find his set-aside edits again: a reader missed means a put-back
that silently does nothing. Judged by git-truth tests, so not Fable.
Brownfield: a **scout first** (ADR-0035), one page: every writer and every reader of a message the app writes into git
(stash, backup commit subject, index/absorb/forecast commits), file:line, and what each reader decides from it.
Needs: `mergewright-env-names` merged; `graft_core::on_disk::{Generation, generation_of_message}` (ticket 1).
Beside: `mergewright-release-and-formula` (disjoint files; two cargo builders, the cap).

## Read first
- ADR-0057 Amendment (2026-10-11) §1 (Tag row, Backup identity), §10
- KNOWN_GAPS rows `M8/27-git-messages-graft`, `M8/27-j-column-graft` (the cited lines)
- 27a NOTES "Kept, allow-listed" (the writer list)
- `crates/graft-ops/MAP.md` `## snapshot`, `## apply` (set aside), `## journal_list`; `crates/graft-ui/MAP.md`
  `## journal_screen` (`legend`, `RowKind`); `crates/graft/MAP.md` `## view::journal`, `## update::stash`
- `docs/design/01-view-mockups.md` §1.10 (the journal mock, lines 366-380); `docs/design/02-keymap.md` §2.11 (read only)
- The scout's file. Then source, only the files it names.

## Decisions
- **Writers write `Generation::CURRENT.dir()`** as the tag: `mergewright: your edits, set aside for …` (stash),
  `mergewright {id} {label}` (backup commits), `mergewright: index`, `mergewright: absorb…`,
  `mergewright: put-back forecast`. One helper builds the tag; no literal `mergewright` at a writer.
- **Every reader accepts every generation** through `generation_of_message`. In particular: the set-aside stash found
  again after a stop, a kill or a relaunch; the S list's marking of a set-aside entry; anything skipping our own
  backup commits in history, search or the radar. A reader the scout finds that compares a message any other way moves
  to `generation_of_message`. NOTES has a table: `reader · file:line · before · after`.
- **Old messages are never rewritten** (stashes, commits are his history).
- **J column** (§10): `●` rows `Mergewright` from `Generation::CURRENT.title()`, `$` rows `outside`; the 14-cell
  column unchanged. Legend: `● done in Mergewright: u can undo it · $ done outside Mergewright: u never touches it ·
  ✗ a commit no branch holds` (plus Q1's clause if it has landed), the product word from the table. Shedding order
  unchanged.
- **Mock-up** `docs/design/01-view-mockups.md:368-374`: the drawn words follow (`Mergewright`, `outside`); prose around
  it keeps the codename (§12).
- The allow-list entries in `no_old_name_in_user_visible_strings` for the J column and the git messages are removed.

## Scope
In: the writers and readers above; J's column and legend; the mock-up lines; TestBackend captures of J (m7 walks
included: tests compare them); the allow-list. Out: the `Migrate` row's words (ticket 1); `✗`/replaced rows
(`usability-q1-*`); fixture identities (`Graft Fixture`: code name).

## Signatures
```rust
// graft-core::on_disk
pub fn message_tag() -> &'static str; // Generation::CURRENT.dir(); the one tag every writer uses
// graft-ui::journal_screen — RowKind unchanged; the source word for `Graft`-kind rows comes from the caller:
pub fn source_word(kind: RowKind) -> &'static str; // Graft → CURRENT.title(), Outside → "outside", Lost*, Replaced, Deleted → "" (drawn as today; ticket 46)
```
If the scout shows the word is built in `graft::view::journal` instead, the function lives there with the same shape;
say so in NOTES.

## Acceptance (each box: the test named at tick time)
- [x] **Set-aside across generations.** A repository holding a set-aside stash written as `graft: your edits, set
      aside for …` (made by hand with `git stash push -m`): after relaunch the app offers it and puts it back; `git stash
      list` is empty after. Same with `tributary:` (never written, but read). Truth from `git stash list` and the file
      bytes. Test: `graft-ops tests/it/set_aside_generations.rs::an_older_tag_is_still_ours` (loops over
      `Generation::CURRENT.older()`: `graft`, `tributary`). See NOTES: the stash is the app's own, re-listed
      under the older tag, not a bare `git stash push`.
- [x] **New writes carry the tag.** A set-aside and a backup made now: `git log -1 --format=%s` / `git stash list`
      start with `mergewright` (quoted). Test: `graft-ops set_aside_generations::new_writes_say_mergewright`
      (stash `%gs` starts `mergewright: your edits, set aside for `; backup `%s` starts `mergewright `).
- [x] **No reader left behind.** `rename_names::no_old_message_literal`: no string literal `"graft:` / `"graft ` /
      `"tributary:` outside `on_disk.rs` in non-test source. Scout's reader table complete in NOTES.
      Test: `mergewright::it rename_names::no_old_message_literal` (seen red: a `"graft: put-back forecast"`
      literal put back in `snapshot/commit.rs` fails it at `:100`; restored).
- [x] **J column.** TestBackend: a `●` row reads `●  Mergewright`, a `$` row `$  outside`, at 80 and 187 columns;
      the legend's sentence names Mergewright. Test: `journal_screen::tests::source_column_words`. Captures
      regenerated (m7 journal walks included), drift summary quoted
      (`.scratch/m8-polish-and-ship/mwr-messages-shot-drift.txt`).
      Test: `graft-ui journal_screen::tests::source_column::source_column_words` (moved to
      `tests/source_column.rs`: `tests.rs` is over the file-size ratchet). At 80 there is no source column
      (J1: 120 and wider), so at 80 the test checks the legend and that no `Graft` is drawn.
- [x] `no_old_name_in_user_visible_strings` allow-list lost the J-column and git-message entries (diff quoted).
      Test: `mergewright::it rename_names::no_old_name_in_user_visible_strings`, passing with the 6 entries
      removed (24 → 18): `journal_screen/pieces.rs`/`Graft`, `journal_screen/legend.rs`/`done in Graft: {can}`,
      `apply/set_aside.rs`/`graft: your edits`, `apply/set_aside/rebuild.rs`/`graft: `,
      `rewrite/absorb.rs`/`graft: absorb`, `snapshot/commit.rs`/`graft`.
- [x] Visual (rule 6): `docs/shots/m8-mergewright-journal-187x30.txt` and `…-80x24.txt` (TestBackend, Unicode), for
      the user's eye. Colour unverified (rule 5).
      Test: `graft-ui journal_screen::tests::source_column::capture_source_column_for_docs_shots`. Looks-right
      is the user's, not ticked here.

## Touches
- **Callers:** set-aside put-back (relaunch, stop, quit-while-write's set-aside), stash overlay rows, rewrite/absorb,
  put-back forecast, history/search/radar filters (per the scout).
- **Screens:** J (column, legend), S (stash rows' message), the relaunch banner's set-aside line if it quotes the
  message.
- **Journal / undo:** undo of a set-aside put-back must still find the stash: covered by the first test. No journal
  line rewritten.
- **Captures:** J captures (m7 and m8), S captures showing a set-aside message.
- **Docs:** `01-view-mockups.md` §1.10 drawn words; MAP lines (graft-ops snapshot/apply, graft-ui journal_screen);
  TESTING.md lines quoting `graft: your edits` (instruction lines only; records stay) go to `mergewright-docs-sweep`.
- **Keymap:** none.
- **Crates:** graft-core (helper), graft-ops, graft-ui, graft.
- Each is checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide. Capture runs to files; summary lines only.

## Size
One context, ~45k tokens after the scout.

## Added by the conductor (review 50, 2026-10-12)
- `docs/reviews/m8-mergewright-50.md` minors in J: the 80-col footnote ends `leaves no trace…`, one cell over
  (`journal_screen.rs:60`; the review suggests dropping `here`); comments at `journal_screen.rs:82,95`, `view.rs:51`;
  `pieces.rs:30,32` draws `Graft`. Fix with this ticket's J column work; capture for his eye.

## NOTES (builder, 2026-10-12)

Scout used: `.scratch/m8-polish-and-ship/scout-52.md`. No compaction.

**Deviations from the spec (say so, not silent):**
- `message_tag()` is a **const** `graft_core::on_disk::MESSAGE_TAG`, not a fn: `core_has_no_free_functions`
  (ADR-0007) fails on a free fn in graft-core. Same value, `Generation::CURRENT.dir()`.
- `generation_of_message` is `Generation::of_message` (already there, `on_disk.rs:159`).
- `source_word` lives in `graft-ui::journal_screen` (`view.rs`), as the spec's first choice: the scout found
  the word built at `pieces.rs:30,32`, not in `graft::view::journal`.
- **The first test's stash is not a bare `git stash push -m`.** Readers find the set-aside by the stash id the
  journal entry records, never by message (table below), so a stash with no entry is not "ours" to any
  generation and nothing offers it. The test makes the realistic older-build state instead: the app's own
  set-aside, interrupted by a SIGKILL (K1's shim), then `git stash drop` + `git stash store -m "<older>: your
  edits, set aside for switch to other" <same id>`. Then recovery (`recover_interrupted`, as relaunch runs it)
  says `EditsAt::Stash { n: 0 }`, `u` puts it back, `whole()` equals before, `git stash list` is empty.
  It passed as soon as it compiled: it pins existing behaviour, it did not drive a code change.
- One writer the spec did not list: `rebuild.rs` `your edits, put back with the index`. Renamed too.

**Reader table** (`grep` of `crates/*/src` for `starts_with|strip_prefix|contains` on a message/subject/stash,
and for `"graft[: ]`; checked by me, not only the scout):

| reader | file:line | before | after |
|---|---|---|---|
| set-aside of an entry (put-back, resume, recovery) | `graft-ops/src/apply/set_aside.rs:60` `stash_of` | journal entry's `PutBack { stash }` oid | unchanged; no message read |
| undo notice finds the stash in the list | `graft/src/update/undo/notice.rs:238,266` | `*commit == put_back.stash` (oid) | unchanged |
| S list rows | `graft-git/src/stash.rs:117` | message stored for display only | unchanged; draws whatever tag |
| backups | `graft-ops/src/rename/journals.rs:31` and the ref walks | by ref (`refs/<gen>/…`), trailer by `of_trailer` | unchanged |
| history skips backups (test only) | `graft/tests/it/m3_walk.rs:230` | `subject.starts_with("graft ")` | `Generation::of_message(subject).is_some()` |
| reflog words | `graft-ops/src/journal_list/{steps,words}.rs` | git's own reflog wording | not ours; unchanged |

No production reader compared one of our messages; nothing else moved to `of_message`. History/search/radar:
no message-based skip found by grep (`mistake_radar/rules.rs:55` reads `Merge `, git's).

**J column** (§10): `●  Mergewright`, `$  outside`, column width unchanged (15 cells incl. gap, `SOURCE`).
Legend from `Generation::CURRENT.title()`. Review-50 minors fixed: `NO_TRACE` drops `here` (74 cells, whole at
80); comments `journal_screen.rs:82,95` now name `source_word`; `pieces.rs:30,32` no longer draws `Graft`.

**For the user's eye** (drift: `.scratch/m8-polish-and-ship/mwr-messages-shot-drift.txt`, 22 shots, every changed
line one of: source word, legend, footnote `here`, stash message + its sha):
- At 80 columns the legend now cuts at `$ done outside Mergewright: u never…` (with `Graft` it reached `✗ a…`).
- At 130–137 columns the legend's `⇢` clause no longer fits (whole legend + clause is 138 cells). The design
  mock (130 wide) is redrawn without it and says so.
- New: `docs/shots/m8-mergewright-journal-187x30.txt`, `…-80x24.txt`, captioned in `docs/shots/m8-contact-sheet-captions.tsv` (sheet not rebuilt).
- Stale, not regenerated (not compared by the fast tier): `docs/shots/m3-08-journal-after-unstage-*.txt`
  (heavy-tier pty walk; already stale since ticket 50), `docs/shots/m8-contact-sheet-{2,3}.txt` (assembled at
  acceptance).

**Not done here (routed):** TESTING.md instruction lines quoting `graft: your edits` → `mergewright-docs-sweep`.
KNOWN_GAPS rows `M8/27-git-messages-graft`, `M8/27-j-column-graft` can close (builder does not edit KNOWN_GAPS).
