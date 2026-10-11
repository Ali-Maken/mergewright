# Mergewright 5: messages written into git say `mergewright`; J's column says Mergewright

Status: ready
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
pub fn source_word(kind: RowKind) -> &'static str; // Graft → CURRENT.title(), Outside → "outside", Lost* → "" (drawn as today)
```
If the scout shows the word is built in `graft::view::journal` instead, the function lives there with the same shape;
say so in NOTES.

## Acceptance (each box: the test named at tick time)
- [ ] **Set-aside across generations.** A repository holding a set-aside stash written as `graft: your edits, set
      aside for …` (made by hand with `git stash push -m`): after relaunch the app offers it and puts it back; `git stash
      list` is empty after. Same with `tributary:` (never written, but read). Truth from `git stash list` and the file
      bytes. Test: `graft-ops tests/it/set_aside_generations.rs::an_older_tag_is_still_ours`.
- [ ] **New writes carry the tag.** A set-aside and a backup made now: `git log -1 --format=%s` / `git stash list`
      start with `mergewright` (quoted). Test: `…::new_writes_say_mergewright`.
- [ ] **No reader left behind.** `rename_names::no_old_message_literal`: no string literal `"graft:` / `"graft ` /
      `"tributary:` outside `on_disk.rs` in non-test source. Scout's reader table complete in NOTES.
- [ ] **J column.** TestBackend: a `●` row reads `●  Mergewright`, a `$` row `$  outside`, at 80 and 187 columns;
      the legend's sentence names Mergewright. Test: `journal_screen::tests::source_column_words`. Captures
      regenerated (m7 journal walks included), drift summary quoted
      (`.scratch/m8-polish-and-ship/mwr-messages-shot-drift.txt`).
- [ ] `no_old_name_in_user_visible_strings` allow-list lost the J-column and git-message entries (diff quoted).
- [ ] Visual (rule 6): `docs/shots/m8-mergewright-journal-187x30.txt` and `…-80x24.txt` (TestBackend, Unicode), for
      the user's eye. Colour unverified (rule 5).

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
