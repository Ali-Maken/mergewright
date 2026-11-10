# pre-publish: the last codename spellings 11c left on purpose (ghost seed, fork label, internal-ref fn, scratch prefix, cited test names), and the guard's `11c` exceptions removed

Status: ready
Blocked by: 11a, 11b4, 11c (merged)
Runs: alone, before 12 (it changes ghost ids and one capture's paths; 12 then regenerates on top)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: each rename is decided below, and each has a test written first that pins what must not change.
Needs: `mergewright_core::on_disk::COMMAND` (exists)
Scout: none needed. Sites come from 11c's NOTES (`git show main:.scratch/pre-publish-cleanup/issues/11c-codename-other-crates.md`, § NOTES) and `crates/mergewright/tests/it/codename_allowed.txt`. Line numbers are from 11c's NOTES; re-find each one by grep.

## Closes
The `11c` rows of `codename_allowed.txt` (5) and 11c's "left on purpose" list. After this, the only codename exceptions left are `guard` and `git-word` rows, plus 12's identity pending.

## Decisions
**D1. Ghost-id seed: `graft-ghost\0` → `preview-ghost\0`** (ops `preview/ghost.rs:23`, `effects.rs:28`, ops MAP :309).
- Neutral, with no product name, so a later rename never moves ghost ids again.
- **Verify first** that nothing persisted holds a ghost id. Grep the journal writer, saved rewrites (`rewrite/saved.rs`), `apply/record.rs` and anything else under `.git/<DIR>/` for the ghost-id fn and its callers. Quote the grep in NOTES. If anything stores one, **stop**: the change would orphan stored ids.
- Test-first: `ghost::tests::ghost_id_seed_is_neutral` pins `ghost_id(subject, n) == sha1("preview-ghost\0<subject>\0<n>")` for one known input (red, then green).
- Captures that change: every ghost-preview capture shows ghost sha7s (m5 ghost preview; the m7 rewrite previews, if they draw ghosts).
  - Regenerate them. Each changed line must differ **only in hex runs**: run `scripts/pp-12-identity-diff.py` if 12 has added it, otherwise a one-off check whose command is in NOTES. List the files.
  - Unnamed ghost lanes take their colour from the oid (`BranchIdentity::of_short_sha`), so their colour may shift. Captures can't show colour (rule 5): **colour unverified**; say so in NOTES.
- `docs/capabilities/m5-history-and-ghost-preview.md` cites the seed: update it in the same commit.

**D2. Fork label: `graft-fork-<n>` → `format!("{}-fork-{n}", on_disk::COMMAND)`** (`mwr-fork-<n>`), spelled once.
- Sites: ops `rewrite.rs:188`, `rewrite/todo.rs:317`, `rewrite/progress.rs:35`, `saved.rs:55`, and their readers and tests.
- Writers and readers share one fn, so a label can't be written one way and read another.
- The label is written into git's rebase todo **while a rebase runs**.
  - A rebase that an old build started and stopped mid-way won't be recognised by this build. Under the user's rule (code holds only current state), there's no compatibility reader.
  - Cost for 00-index: finish or abort any rebase the app started before upgrading.
- Test-first: `rewrite_fork_label_round_trips`, written before the change. It starts a rewrite that uses fork labels, stops it at a conflict, re-reads the progress from disk the way reopening does, then continues and finishes. Assert the progress read names the forks, and the result matches the same rewrite run without a stop. Green before the change; green, unchanged, after.
- App test `m7_walk/fork_rows.rs` cites `graft-fork-1` → the new label (allow row goes).

**D3. `RefName::is_graft_internal` → `is_mergewright_internal`** (core).
- The core MAP already names it that way. Update every caller and `docs/capabilities/m3-working-tree-undo-and-stash.md`.

**D4. Test fn names cited in docs/capabilities.**
- `grafts_own_refs_are_not_drawn_but_are_still_readable` (git `repo/tests.rs`) → `own_refs_are_not_drawn_but_are_still_readable`.
- `a_shell_merge_is_not_adopted_as_grafts_own_entry` (ops `history_ops.rs`) → `a_shell_merge_is_not_adopted_as_an_own_entry`.
- Update the citations in the same commit. If `mergewright-capabilities` checks that cited tests exist, its test proves it; name that test.

**D5. Fixture scratch prefix: `graft-` → `mwrfx-`** (fixtures `scratch.rs`).
- The same length (6), so `docs/shots/m8/21c-journal-partial-fetch-120x40.txt` keeps its truncation. 11c's `mwr-` moved it, and that was the stop.
- Regenerate that capture; the diff must be the word only.

**D6. Old-name guard inputs: keep only those the guard needs.** For each, rewrite to a neutral foreign spelling (e.g. `refs/other/`, `other:`) **only if the test still pins the same property**. Otherwise keep it with a `guard` row and a one-line reason.
- Rewrite:
  - `core/src/on_disk/tests.rs` and `core/src/ref_name/tests.rs`: "a foreign ref/message isn't ours". Recognition is current-only, so any foreign prefix proves it.
  - `ops/tests/it/conflict_set.rs:353,360` and `ops/src/journal_list/tests/words.rs:148`: the same argument.
  - `ui/.../source_column.rs:25,34` ("old name never drawn") → assert the column word is in the exact current set. That's stronger, and needs no old name.
- Keep, with `guard` rows: `ops/tests/it/goldens/m5-journal.jsonl`, but **only if** it pins reading lines exactly as stored by an earlier build. If its old names are incidental (a ref prefix any string could fill), change them to `refs/mergewright/` and require the test's expectations to change by the word only.
- App `rename_names.rs` `ALLOWED` prefixes that no production literal uses any more (11c NOTES: `graft-input`, `graft-askpass`, `graft-marker-`, `.graft-tmp`; after D1/D2 also `graft-ghost`, `graft-fork-`; check `graft-`): remove them, with a grep per entry showing no production use. Their allow rows go too. Removing an exception only tightens the guard.
- Unchanged, still `guard`: `codename_absent.rs`, `old_names.rs`, `rename_names.rs`'s word lists and word-finder tests, `rename_names/scripts.rs`, `palette/names_tests.rs`, `shot-drift.py`, `shot-drift-test.py`.

**D7. Allow-list and pending.**
- Remove the five `11c` rows and the `11c` reason from the file's header comment. The header then names only `guard` and `git-word`.
- If `codename_absent.rs`'s `PENDING` still lists the other crates under `11c`: relabel it `12` with the comment "fixture identities and `graft.git` only, cleared by 12". That's all that's left there after this ticket.

## Stop conditions
- Anything persisted holds a ghost id (D1).
- `rewrite_fork_label_round_trips` can't be green before the change on today's code, or its expectations have to change after.
- A capture diff goes beyond hex (D1) or the word (D5).
- A D6 rewrite weakens what a test pins: keep it with a `guard` row instead and say why.

## Scope
In: ops (`preview`, `rewrite*`, tests), core (`ref_name`, `on_disk` tests), git (one test name), fixtures (`scratch.rs`), ui (`source_column` test), app (`m7_walk/fork_rows.rs`, `rename_names.rs` `ALLOWED`, `codename_absent.rs` `PENDING` label, `codename_allowed.txt`), `docs/capabilities` (3 citations), the regenerated captures, the MAP lines (ops `ghost`/`rewrite`, core `ref_name`).
Out: `graft.git` fixture URL, fixture identities, commit-id changes (12).

## Signatures
```rust
// mergewright-ops (private or today's visibility)
const GHOST_SEED: &[u8] = b"preview-ghost\0";
fn fork_label(n: usize) -> String;   // format!("{}-fork-{n}", on_disk::COMMAND); every writer and reader calls it
// mergewright-core
impl RefName { pub fn is_mergewright_internal(&self) -> bool; } // was is_graft_internal
// tests
#[test] fn ghost_id_seed_is_neutral();          // ops preview/ghost tests
#[test] fn rewrite_fork_label_round_trips();    // ops tests/it
```

## Acceptance (tick each with its test, inline)
- [ ] No persisted ghost id ← grep quoted in NOTES
- [ ] Ghost seed pinned ← `ghost_id_seed_is_neutral`
- [ ] A stopped rewrite round-trips under the new label ← `rewrite_fork_label_round_trips` (green before and after; file diff since its first commit empty)
- [ ] Ghost captures differ only in hex; scratch capture only in the word ← check output in NOTES, files listed; colour marked unverified
- [ ] Capability citations resolve ← capabilities test by name
- [ ] Allow-list: zero `11c` rows; every row `guard` or `git-word` ← `codename_absent::every_allowed_codename_occurrence_still_exists`, `no_shipped_file_names_the_codename`; `cut -f3 codename_allowed.txt | sort | uniq -c` in NOTES
- [ ] Remaining codename lines in crates are identity only, plus `guard` rows ← `git grep -n -I -i -E 'graft|tributary|trib_' -- crates` tagged in NOTES
- [ ] Fast and walk tiers green; no `over_400.txt` row raised ← counts, `file_size`

## Touches (check each, say how)
- The ghost preview (m5) and rewrite previews (m7): captures, and contact sheets (list; don't rebuild).
- The rewrite editor's progress read and the fork rows walk.
- Undo/journal: whether ghost ids or fork labels are journaled (D1 grep; D2 test).
- `published_repo` and the fixtures cache key (`scratch.rs`: is it in a key? if so, caches rebuild once).
- Capabilities parser. Keymap: none.

## Read first
This ticket. 11c's NOTES (`git show main:…/11c-codename-other-crates.md`). `codename_allowed.txt`. `crates/mergewright-ops/MAP.md` § preview (ghost) and § rewrite lines; `crates/mergewright-core/MAP.md` § ref_name; `crates/mergewright-fixtures/MAP.md` scratch line.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~50k tokens.
