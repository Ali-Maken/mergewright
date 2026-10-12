# Scout 52: git messages and the J column

## 1. Writers (non-test source, all in graft-ops)
| Literal | file:line | fn |
|---|---|---|
| `graft: your edits, set aside for {description}` | `crates/graft-ops/src/apply/set_aside.rs:55` | `words()` |
| `graft: index` | `crates/graft-ops/src/apply/set_aside/rebuild.rs:41` | rebuild `commit(...)` |
| `graft: your edits, put back with the index` | `crates/graft-ops/src/apply/set_aside/rebuild.rs:46` | same fn; **not listed in the ticket** |
| `graft: absorb's index base` | `crates/graft-ops/src/rewrite/absorb.rs:151` | |
| `graft {id} {label}` | `crates/graft-ops/src/snapshot/commit.rs:52` | `commit_tree` (backup) |
| `graft: put-back forecast` | `crates/graft-ops/src/snapshot/commit.rs:89` | |
No `"tributary` literal outside `on_disk.rs` (`:49`, `:114`). The other `"graft ` hits (`graft-fixtures/src/capture*.rs`, `graft-capabilities`) are test/fixture strings, not git writes.

## 2. Readers that decide from the message text
**None found.** Grep of crates/*/src for starts_with/strip_prefix/contains on message/subject/stash finds no comparison against a graft tag. Readers go by oid or ref:
- The set-aside stash is found by its oid from the journal entry: `apply/set_aside.rs:59-66` `stash_of(entry)` matches `PutBack { stash, why: SetAside }`.
- Undo notice finds it in the stash list by oid: `crates/graft/src/update/undo/notice.rs:238`, `:266` (`*commit == put_back.stash`).
- `graft-git/src/stash.rs:117` only reads the message into `StashEntry.message` (lossy). It does not compare it.
- Reflog subject matches (`journal_list/steps.rs:80,90,112`, `words.rs:67,103,108`) test git's own wording, not ours.
- Backups go by ref: `rename/journals.rs:31` (`backup_ref`). Practice prefix: `tour.rs:224`.
- **Could not confirm** that history, search or radar have any message-based skip of backup commits. The grep finds none.
Contradiction: the ticket assumes such readers exist (relaunch lookup, S marking, filters). In code they are oid/ref based.

## 3. on_disk (`crates/graft-core/src/on_disk.rs`)
- `enum Generation` `:20`; `CURRENT = Mergewright` `:33`; `GENERATIONS: [Self; 3]` `:36`
- `pub const fn dir(self) -> &'static str` `:46`; `title` `:56`; `older()` `:128`; `of_ref` `:141`; `of_trailer` `:149`
- **No free fn `generation_of_message`** (contradicts the ticket). It is `pub fn Generation::of_message(message: &str) -> Option<Self>` `:159`: the message starts with `dir()` followed by `':'` or `' '`.
- No `message_tag()` yet.

## 4. J source column (built in graft-ui, not graft::view)
- `crates/graft-ui/src/journal_screen/pieces.rs:28-36`: `("●", "Graft")` `:30`; `("$", "outside Graft")` `:32`. The ticket's target is `outside` alone.
- Legend `crates/graft-ui/src/journal_screen/legend.rs:25`: `format!("● done in Graft: {can} · $ done outside Graft: {never} · ✗ a commit no branch holds")`
- `journal_screen.rs:60`: `"a branch deleted outside Mergewright and never checked out leaves no trace here";`
- `journal_screen.rs:82`: `/// The source word's column: `Graft` / `outside Mergewright`, padded.`
- `journal_screen.rs:95`: `/// The source word (`Graft` / `outside Mergewright`): 120 and wider.`
- `view.rs:51` (in `journal_screen/view.rs`): `/// `●` — a journal entry: done in Mergewright, `u` can undo it.`
- The comments at :82 and :95 disagree with the code at pieces.rs:32.
- `RowKind::Graft` is used at `journal_screen.rs:272`, `view.rs:53,116`, `crates/graft/src/view/journal.rs:235`.

## 5. Allow-list: `crates/graft/tests/it/rename_names.rs`
`ALLOWED` `:41`; the test is `:241`. Entries:
- J: `pieces.rs`/`"Graft"` `:115-117`; `legend.rs`/`"done in Graft: {can}"` `:120-122`
- git: `set_aside.rs`/`"graft: your edits"` `:137-140`; `rebuild.rs`/`"graft: "` `:142-145`; `absorb.rs`/`"graft: absorb"` `:147-150`; `snapshot/commit.rs`/`"graft"` `:152-155`
- `no_old_message_literal`: does not exist yet.

## 6. Tests and captures that compare J
- Unit: `crates/graft-ui/src/journal_screen/tests.rs:331,332,353,372,373,376,403,416` (assert `●  Graft`, `outside Graft`, `done in Graft`).
- Integration: `crates/graft/tests/it/m7_walk/journal.rs:258` (`m7/journal-{187x50,100x30,187x24}`), `:277,312,337,413,441,494,532`; `journal_screen.rs:683`; `journal_put_back.rs:402,441`; `put_back_box.rs:290`.
- Regenerate with `MWR_UPDATE_SHOTS=1` (`crates/graft/tests/it/captures.rs:181,188`). The run rewrites the file and still fails.
- `graft: your edits` also appears in tests and shots: `graft/tests/it/m6_stash_put_back/set_aside.rs`, `quit_waits/hangup.rs`, `graft-ops/tests/it/set_aside.rs`, `docs/shots/m8-hangup-relaunch-120x40.txt`, `m7/30c-relaunch-edits-back-120x40.txt`.
- Other shots containing J words (older milestones): m3-*, m4-07-*, m8/21c-*, m8/doorway-journal-120x36, m8-q1-journal-replaced-ui-*.
