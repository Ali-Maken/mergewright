# 30c put-back-review-fixes — review m7-30-06d: Major 1, Minor 1, Question 1, Minors 2, 3, 6, 7

Status: partial — all built and green; two checks open (K4 at 80 columns, the Minor 3 doc check), see NOTES
Build with: Opus. This is the set-aside protocol (ADR-0045): undo must hold at every kill point, and one of the
fixes is a git rule (which staged paths a plain `stash apply` loses) that has to be probed before it is built on.
Needs: none. 30, 30b and 06d are on main.
Blocked by: **20i** (it is building in graft now). Queue **before 03b**: both hold graft-ops `operation.rs`
(30c removes `SetAside.staged`) and the put-back, and 03b's pull row (see its note from review 30-06d) should be
built on 30c's rule. 33 shares no file with 30c; the conductor picks that order.
Crates: graft-ops (`apply/set_aside/{put_back,answer,returned}.rs`, `apply/recover.rs`, `apply/error.rs` `EditsAt`
derives only, `operation.rs` `SetAside`, `objects.rs` doc, tests, MAP.md), graft (`interrupted_notice`, the
`AppState` field and the launch path that fills it, `update/banner.rs` Minor 7, the hand-built `SetAside` in
`update/preview/tests.rs`, one app test, MAP.md). Docs: ADR-0045, TESTING.md, `docs/shots/m7/`. Needs another crate
→ commit at the seam, report `blocked: needs <crate>` with the signature, stop.
Scout: **the review is the scout** for Major 1 and Minors 1, 2, 3, 6, 7 (its lines are at `08f08d5c`; re-read each
site first, because 06e merged after it). **One short scout first** (`scout`, ≤1 page,
`.scratch/m7-power-tools/scouts/30c.md`) for the kill seam and the relaunch words:
(a) how graft-ops spawns `git`: through `PATH`, or through a seam that exists today (a program path on `Context`, an
env var)? (b) Where the launch receives `Recovered` and how it reaches `AppState`. (c) Where `interrupted_notice`
lives, and which tests and captures pin ` · edits in stash@{n}`. (d) Every construction of `SetAside { .. }` and of
`Recovered { .. }` in both crates.
Split by: planner, 2026-09-27, from KNOWN_GAPS M7/30-am, M7/30-kill, M7/30-review-minors
(`docs/reviews/m7-30-06d.md`).

## Read first
- `docs/reviews/m7-30-06d.md`: Major 1, Minors 1, 2, 3, 6, 7, and Question 1. Not the rest.
- 30's ticket: "Decided" 2, "Signatures", NOTES "Mechanism" and T5, T6, T8, T10
  (`.scratch/m7-power-tools/issues/30-put-back-keeps-the-index.md`).
- ADR-0045: "Amendment 2026-09-26" §5 (the `EditsAt` table, :141-156) and "Amendment 2026-09-26 (b)" (:185-239).
- graft-ops MAP.md: `SetAside` (:32), `apply` layout (:194), `apply` set-aside (:200), `AfterState::StagingNotKept`
  (:209), `recover_interrupted` / `Recovered` (:217-218), `EditsAt` (:242), `objects.rs` (:280).
- graft MAP.md: `interrupted_notice` (:321), `AppState::live_entries` (:188).
- The scout's report.
- `docs/research/ux-principles.md` §2 rows 1, 4, 9.

## Decided (planner, 2026-09-27)
1. **Major 1: an added path is lost when its staged content moved on.** A plain `stash apply` re-adds a staged new
   file **from the working tree** (review's probe: `AM n.txt` → `A  n.txt`, index v2, v1 gone). So `staging_lost`
   names an `A` path when `S^2:path` ≠ `S:path`, a missing `S:path` included (`AD`: staged new, then deleted).
   - The read is one `git diff-tree -r --no-renames --name-only -z S^2 S`, intersected **in memory** with the added
     paths. No pathspec: there is no argv limit and no pathspec-magic escaping to get wrong.
   - It runs only when the first read found an `A` path, so no new spawn on the usual fallback.
   - The result is in path order with no duplicates, the order git prints.
   - A plain `A ` (content unchanged since staging) stays unnamed. Git re-adds it exactly, and naming it would be
     false.
   - Words unchanged: a staged new file whose staged content went is "staging not kept".
   - Why not rebuild the lost index entries ourselves: that is Graft writing index content where git has a verb.
     30 Decided 2 rejected it; `u` restores the split exactly.
2. **Minor 1: append the clause to git's words, as `naming` does** (the review's second option). One formatter for
   ` · staging not kept: <first>[ +N more]`, shared with `StagingNotKept`'s Display, `SetAsideKept.words` and the
   `Aborted` reason. Use the one that exists; don't add a second.
   - **Rejected:** an `unstaged` field on `EditsAt::Back`. On the operation-failed path `S` is dropped, so there is
     no `copy`, and `Back { copy: None }` already means "the copy was not found" (ADR-0045 :152). That would need a
     third state in a public enum for a path the review calls near-unreachable.
   - **The clause carries no key of its own.** An operation that failed leaves an `Aborted` entry, and whether `u`
     reaches its backup there is not verified (30 NOTES: an `Aborted` entry gives `NothingToUndo`). The only key on
     the line is `explain`'s existing ` · u restores the backup`, shown when `recoverable`. B3 checks that key does
     what it says.
3. **Question 1: yes to the kill tests, and the relaunch line says where the edits are, by reading.** The notice
   today reads the stash list and guesses. That is the guess ADR-0045's §5 amendment removed from the status line
   ("only `apply` knows whether the put-back ran … the app reads no list to guess"). Recovery is the one place that
   reads the repository after a kill, so it decides:
   - `S` listed and no tracked file differs from `HEAD` → `EditsAt::Stash { n }`:
     `your edits are in stash@{n} · S p puts them back`;
   - `S` listed and the tree changed → `EditsAt::Back { copy: Some(n) }`:
     `your edits are back · a copy stays in stash@{n}`;
   - `S` not listed → nothing (as today).
   - "Tree changed" is the put-back's own judge ("every tracked file equals `HEAD`", 30 NOTES "Mechanism"). Use that
     code; don't write a second.
   - The notice adds ` · <EditsAt::words()>`. Same state, same words as the status line (heuristic 4).
   - **Launch-time fact:** `Recovered` is read once, when the launch closes the entry. On a later launch the entry is
     already closed and the fact is gone, so the notice keeps today's ` · edits in stash@{n}`. That wording is true
     in both states (a copy is in the stash), but less exact. No journal field is added: that would be serde,
     which is hard to reverse, for a window of milliseconds.
   - **The relaunch line does not name staging that was not kept.** No read at recovery can tell a kept index from
     a lost one: `--index` merges the staged patch onto the landed tree, so a kept index need not equal `S^2`. The
     plain apply having run is not journaled either. `u` restores the split exactly, and K2b proves it. This goes
     to the conductor as a narrowed gap.
   - **The kill is real.** A `git` shim first in the child's `PATH` sends `SIGKILL` to its parent (the child running
     `apply`) on a chosen argv, **before** exec'ing the real git. There is no fail-point in production code. The
     child is the test binary itself (`current_exe()` running one `#[ignore]`d helper test gated on an env var), so
     the parent's env is never changed. `#[cfg(unix)]`.
   - If the scout finds graft-ops does not resolve `git` through `PATH` and has no existing seam for it: **stop,
     report `blocked: kill seam`**. A production hook is a design change.
4. **Minor 2: remove `SetAside.staged`.** It is always 0, and nothing words it since 30b. The T11 test
   (`set_aside_staged_is_zero_for_every_kind`) is deleted: the type now can't express the case it guarded. Name
   the test in NOTES.
5. **Minor 7: fold the `Status::Sticky` arm into `other =>`** with a one-line comment, rather than testing it. It
   composes words for a case its own comment calls unreachable, and nothing calls it (PROMPT.md 6.2).
6. **Not folded:** Minors 4 and 5 are corrections to 06d's and 30's NOTES, the conductor's. Minor 8 is in 03b's
   note. Question 2 (a `Journal` failure after a fallback, where `u` may undo the entry before) needs an injected
   journal failure; the conductor files it. M7/30b-resize changes every `Status` match in graft; it stays a gap.

## Journeys
| Journey | From | Keys | Count | Best tool | Lands |
|---|---|---|---|---|---|
| `switch-staging-not-kept` with an `AM` file (changed) | as 30b's row, plus `n.txt` staged v1 then edited to v2 | `. ⏎ y` | 3 (unchanged) | shell: `stash`, `switch`, `stash pop` = 3 commands, and v1 is lost with no word | on `fix`, sticky `… staging not kept: f.txt +1 more · u undoes it, staging included`; `n.txt` `A ` at v2 |
| …then `u` | the line above | `u` | 1 | — | on `main`; `:n.txt` = v1, worktree v2; everything byte-equal to before |
| `relaunch-after-kill` (new) | Graft killed between the put-back and the drop; he launches it again | none | 0 to see where the edits are | shell: `git status` + `git stash list`, then work out whether the stash is a duplicate = 2 commands and a judgement | graph, footer `… · your edits are back · a copy stays in stash@{0}` |
| …then `u` | the footer above | `u` | 1 | — | back as before the switch, staging included; stash list as before (no duplicate) |

## Scope
**In:** Decided 1–5; tests A1–A4, B1–B3, K1–K4, M6; the relaunch capture; the docs under Touches.
**Out:** Decided 6. A preview forecast (M7/30forecast). The staging lost on a clash (ADR-0045 :129). Any new
journal field.

## Signatures
```rust
// crates/graft-ops/src/apply/set_aside/put_back.rs (crate-private)
/// Staged paths a plain `stash apply` loses: every staged change in `S^1..S^2`, except an added path
/// whose staged blob equals its working-tree blob (git re-adds that one from the tree, unchanged).
/// An added path edited on (`AM`) or deleted since (`AD`) is lost (review m7-30-06d Major 1).
fn staging_lost(ctx: &Context, stash: Oid) -> Result<Vec<PathBuf>, ApplyError>; // signature unchanged
/// The rule, pure and unit-tested. `staged`: `diff-tree --name-status S^1 S^2` as the existing parse
/// holds it (keep its status type); `moved_on`: the paths of `diff-tree --name-only S^2 S`.
fn lost(staged: &[(/* existing status type */, PathBuf)], moved_on: &BTreeSet<PathBuf>) -> Vec<PathBuf>;

// crates/graft-ops/src/apply/recover.rs
pub struct Recovered {
    // …existing fields unchanged…
    /// Interrupted set-aside entries whose stash is still listed, and where their edits were when
    /// recovery read the repository (ADR-0045 §5; 30c).
    pub set_aside: Vec<(EntryId, EditsAt)>,
}
/// Where an interrupted set-aside's edits are. `tracked_changed`: the put-back's own judge, false
/// when every tracked file equals `HEAD`. `None` when `stash` is not listed.
pub(crate) fn left_at(stash: Oid, stashes: &[(Oid, usize)], tracked_changed: bool) -> Option<EditsAt>;

// crates/graft-ops/src/apply/error.rs — derive `Clone, Debug, PartialEq, Eq` on `EditsAt` if missing.

// crates/graft-ops/src/operation.rs
pub struct SetAside { pub files: usize } // `staged` removed (Minor 2)

// crates/graft — the AppState struct (graft MAP.md names its file)
/// From the launch's `Recovered::set_aside`: where each interrupted set-aside's edits were at
/// recovery. Lives as long as the notice (until `J`); empty after any later launch.
pub interrupted_edits: Vec<(EntryId, EditsAt)>,
// `interrupted_notice(&AppState) -> Option<String>`: signature unchanged. An entry in
// `interrupted_edits` adds ` · {EditsAt::words()}`; one not in it keeps ` · edits in stash@{n}`.
```

## Tests (each named at tick time; red run quoted in NOTES where marked red first)
Fixtures: 30's P3 (`put_back_index.rs`, 20-line `f.txt`, line 5 staged, `fix` changes line 7) and T2's (staged v2,
worktree v3).
- **A1** `put_back_names_a_staged_new_file_edited_on` (`put_back_index.rs`): P3 plus `n.txt` staged v1, worktree
  v2. `StagingNotKept { paths: [f.txt, n.txt] }`, Display `… staging not kept: f.txt +1 more …`. Git after: `n.txt`
  `A ` at v2 (record what you observe). `u` → `:n.txt` v1, worktree v2, whole state = before. Redo → whole state =
  after. **Red first** (today `[f.txt]`, the review's repro).
- **A2** `put_back_names_a_staged_new_file_deleted_since`: P3 plus `n.txt` staged, then removed from the tree (`AD`).
  Named; `u` exact. Record git's status after in NOTES (expected: `n.txt` gone from index and tree; unverified).
  **Red first.**
- **A3** `put_back_leaves_an_unchanged_staged_new_file_unnamed`: P3 plus `n.txt` `A ` only → `paths == [f.txt]`.
  And T5 (`put_back_falls_back_when_the_index_cannot_go_back`, no `A` path) gains an assert: its Command Log has
  **no** `diff-tree … S^2 S` line. Guards; not red first.
- **A4** the unit test at `put_back.rs:259-267` rewritten on `lost` and renamed
  `lost_names_an_added_path_only_when_its_content_moved_on`: `M`, `D`, `T` named; `A` named iff in `moved_on`;
  output sorted and deduped. **Red first** against the old rule.
- **B1** `answer::tests::a_failed_operation_after_the_fallback_names_the_staging` (unit, `answer.rs`): an operation
  failure plus a clean fallback over `[f.txt]` → `ApplyError::Git` whose `explain()` contains
  `staging not kept: f.txt`. **Red first.** End to end this path has no fixture (a failed operation leaves `HEAD` at
  `S^1`, where `--index` is exact). Say so; do not invent one.
- **B2** `answer::tests::a_failed_drop_after_the_fallback_names_the_staging` (unit): `Kept::Git { back: true }` with
  `unstaged: [f.txt]` → `explain()` contains `staging not kept: f.txt` and
  `your edits are back · a copy stays in stash@{0}`. **Red first.**
- **B3** `a_drop_that_fails_after_the_fallback_names_the_staging` (`put_back_index.rs`, the shim making
  `stash drop` exit 1): P3. The `explain()` line quoted in NOTES contains both clauses. If the line names `u`, `u`
  gives the whole state = before **and** the stash list = before. **Red first** on the words.
- **K1** `set_aside_kill::killed_between_the_two_applies_loses_nothing` (new `tests/it/set_aside_kill.rs`): P3; the
  shim kills on `stash apply` **without** `--index`. Then `recover_interrupted`:
  - the entry `is_interrupted`;
  - `Recovered.set_aside == [(id, Stash { n: 0 })]`;
  - tracked files equal `HEAD`, and `S:f.txt` holds the edit;
  - `u` → whole state = before, stash list = before.
- **K2** `…::killed_before_the_drop_loses_nothing`: T2's fixture (the `--index` apply lands whole); the shim kills
  on `stash drop`. `Recovered.set_aside == [(id, Back { copy: Some(0) })]`; index v2, worktree v3; `u` → before,
  stash list = before.
- **K2b** `…::killed_before_the_drop_after_the_fallback_loses_nothing`: P3; the shim kills on `stash drop`.
  `Back { copy: Some(0) }`; ` M f.txt`; `u` → before (line 5 staged again), stash list = before.
  - K1, K2 and K2b are red first on the `Recovered.set_aside` assert (field absent or empty). Their loss and undo
    asserts verify rather than change: say which were green first.
  - **If `u` refuses, or leaves `S` listed:** the cause is the undo protocol (ADR-0045 §6), not the put-back. Stop,
    report the line, and don't fix it here (rule 2).
- **K3** `recover::tests::left_at_reads_the_tree_not_the_list` (unit): not listed → `None`; listed and unchanged →
  `Stash { n }`; listed and changed → `Back { copy: Some(n) }`. **Red first.**
- **K4** `m6_stash_put_back::set_aside::a_relaunch_after_a_kill_before_the_drop_says_the_edits_are_back` (graft
  `tests/it`): K2b's state, made the same way from graft's test binary (one shim script: share the file, or keep two
  copies with a comment naming the other). Then launch the harness:
  - the footer has `your edits are back · a copy stays in stash@{0}` and lacks `your edits are in stash`, at 120x40
    and 80x24;
  - `u`, 1 key counted → whole state = before, stash list = before;
  - captures `docs/shots/m7/30c-relaunch-edits-back-{120x40,80x24}.txt`, each with a `What right looks like:`
    header;
  - **red first** (today: ` · edits in stash@{0}`).
  - Plus one unit test beside the existing notice tests: an entry **not** in `interrupted_edits` keeps
    ` · edits in stash@{n}`. The test that pins that pattern (`update/undo/tests/set_aside.rs:52`) stays green
    unchanged; name it.
- **M6** (Minor 6) `undo_after_a_fallback_restores_the_split` gains `run_redo`, then whole state = after
  (`index_after` recorded after the plain apply). Expected green first. Red → stop and report (the redo protocol).

## Acceptance (each tick names its test inline; a box with no test is not ticked)
- [ ] Scout report `.scratch/m7-power-tools/scouts/30c.md` answers (a)–(d). **Not ticked:** it answers (a) only
      (PATH, `graft-git/src/run.rs:374`) and could not find the drop (it is `returned.rs` `drop_by_id`). (b)–(d) I
      found by grep while building (NOTES "Construction sites"). No test names a scout; nothing to tick against.
- [x] Major 1: A1 `put_back_index::put_back_names_a_staged_new_file_edited_on`, A4
      `put_back::tests::lost_names_an_added_path_only_when_its_content_moved_on` red then green (quoted in NOTES);
      A2 red on a wrong expectation, rewritten on the measured behaviour as
      `put_back_index::put_back_leaves_a_staged_new_file_deleted_since_unnamed` (NOTES "A2"); A3
      `put_back_index::put_back_leaves_an_unchanged_staged_new_file_unnamed` and T5's no-`diff-tree` Log assert in
      `put_back_index::put_back_falls_back_when_the_index_cannot_go_back` green first. `M7/30-am` closable for `AM`.
- [x] Minor 1: B1 `answer::tests::a_failed_operation_after_the_fallback_names_the_staging`, B2
      `answer::tests::a_failed_drop_after_the_fallback_names_the_staging`, B3
      `put_back_index::a_drop_that_fails_after_the_fallback_names_the_staging` red then green; B3 line in NOTES.
- [ ] Question 1: K1 `set_aside_kill::killed_between_the_two_applies_loses_nothing`, K2
      `set_aside_kill::killed_before_the_drop_loses_nothing`, K2b
      `set_aside_kill::killed_before_the_drop_after_the_fallback_loses_nothing`, K3
      `recover::tests::left_at_reads_the_tree_not_the_list` red then green; K4
      `m6_stash_put_back::set_aside::a_relaunch_after_a_kill_before_the_drop_says_the_edits_are_back` red then green,
      captures written and re-checked green. **Not ticked: at 80x24 the footer is cut after `a copy stay…`**, so
      `stash@{0}` and `J to inspect` are lost (NOTES "80 columns"); K4 asserts the whole clause at 120x40 only.
      Colour unverified (rule 5).
- [x] Minor 2: `SetAside.staged` gone; T11 `set_aside_staged_is_zero_for_every_kind` deleted; sites in NOTES.
      `operation::history::tests::sets_aside_table` and
      `update::preview::tests::the_set_aside_row_sits_under_the_working_tree_line` green.
- [x] Minor 3: `put_back.rs`'s link resolved (now `[from_index_base](super::rebuild::from_index_base)`); ADR-0045:227
      names `apply/set_aside/rebuild.rs`; `objects.rs:1-4` and graft-ops MAP say absorb uses `resolve` only.
      Ticked by 30e: `RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc --no-deps -p graft-ops`
      exits 0 after 30e fixed the 5 older links.
- [x] Minor 6: M6 `put_back_index::undo_after_a_fallback_restores_the_split` (redo added) green first.
- [x] Minor 7: the arm folded. `update::banner::tests::resume_words::a_resume_whose_put_back_lost_staging_is_done_and_says_so`
      green; clippy clean.
- [x] Every set-aside test in both crates green:
      `cargo nextest run --workspace -E 'test(/set_aside|put_back|stash|absorb|recover|interrupt/)'`: 408/408.
- [x] Docs under Touches done: ADR-0045 (:227 path, both dated lines, the absorb item 6 T11 reference), graft-ops
      MAP (`SetAside`, `StagingNotKept` + the A rule, `Recovered.set_aside`, `left_at`, `EditsAt` derives,
      `objects.rs`), graft MAP (`interrupted_notice`, `AppState::interrupted_edits`,
      `JournalRead::interrupted_edits`, `edits_clause`; banner MAP names nothing of the folded arm), TESTING.md
      (the `AM` row, the `relaunch-after-kill` row).
- [x] Gates, workspace-wide: `cargo fmt --check` ok; `cargo clippy --all-targets -- -D warnings` ok;
      `cargo nextest run` 4141/4141 (3 skipped); `cargo test --doc` 3/3.

## Touches (check each, say how)
- `staging_lost` callers: `run_put_back` only (grep). The resume path (`close_resume`) goes through it too:
  re-run `absorb_stopped_then_continued_elsewhere_says_the_staging_went` (it asserts `[b.txt, old.txt]`). If its
  paths change, that is the fix showing; explain it in NOTES.
- Command Log: the fallback gains `diff-tree … S^2 S` when an `A` path exists. T8 `both_applies_refused_keeps_the_stash`
  asserts the Log: re-run it. Also `grep -rn "diff-tree" crates/*/tests docs/shots` for exact-line asserts.
- `ApplyError::Git` text: `grep -rn "ApplyError::Git\|explain()" crates/graft/src` to confirm graft words it through
  `explain`. `grep -rn "a copy stays" crates/graft` for exact asserts that now gain the clause.
- `SetAside` construction sites (scout (d)): `operation.rs` `sets_aside`, graft-ops preview, graft
  `update/preview/tests.rs` (drop `staged: 1` and its "on purpose" comment). 03b's pull kind is not built yet.
- `Recovered` construction sites (scout (d)) and the launch path; `plans_left`, the sibling field, as the pattern.
- `EditsAt` derives: `grep -rn "EditsAt" crates/graft/src` for matches that the derives or the new field touch.
- The notice's other readers: the journal browser (`J`) row for an interrupted entry, and the graph's `interrupted`
  `RowMark`. Both unchanged (no code path touched). Say how you checked.
- `AppState::live_entries` still wins over the notice (a live conflict is never "interrupted"): its test stays green.
  Name it.
- Keymap: no new key. `u` and `S p` keep their meanings (`docs/design/02-keymap.md`). Nothing edited.
- Journal/undo: no new journal field (`grep -n Serialize` on `Recovered`/`EditsAt`: none expected). Undo and redo
  are exercised by A1, A2, B3, K1–K4 and M6.
- Captures: new `docs/shots/m7/30c-relaunch-edits-back-*`. Existing ones: `grep -rln "edits in stash" docs/shots`.
  Any capture taken on the launch that recovered an interrupted set-aside now shows the `EditsAt` words: regenerate
  it (`GRAFT_UPDATE_SHOTS=1`, then a green re-run) and list it. No change to `docs/shots/m7/30b-staging-not-kept-*`
  (its fixture has no `A` path).
- Docs:
  - ADR-0045: fix the :227 path. Append a dated line under "Amendment 2026-09-26 (b)":
    `(2026-09-27, 30c) An added path is named when its staged content differs from the tree's (AM, AD). SetAside.staged removed.`
    Append to the §5 amendment:
    `(2026-09-27, 30c) Recovery says where an interrupted set-aside's edits are (Recovered.set_aside); the relaunch notice words it. A later launch keeps "edits in stash@{n}".`
  - graft-ops MAP: :32 (no `staged`), :200, :209 (the `A` rule), :217-218 (`Recovered.set_aside`, `left_at`), :242
    (derives), :280.
  - graft MAP: :321 `interrupted_notice`, the new `AppState` field, and `banner` if it names the folded arm.
  - TESTING.md: under 30b's `switch-staging-not-kept` row, an `AM` line (`f.txt +1 more`; `u` gives `:n.txt` v1).
    A new `relaunch-after-kill` row from the Journeys table, stating what right looks like concretely.

## For the conductor (not the builder)
- On merge: close `M7/30-am`; amend `M6/83` (the `AM` part closed; type-change and copy still unverified). Replace
  `M7/30-kill` with a narrowed row: "after a kill between the plain apply and the drop, the relaunch line doesn't
  name the files whose staging went (not readable at recovery); `u` restores it exactly (K2b)". Status **broken**,
  rare, M8. Strike Minors 1, 2, 3, 6, 7 from `M7/30-review-minors`, leaving 4, 5 (NOTES edits, yours) and 8 (03b).
- File review Q2 (a `Journal` failure after a fallback: where does `u` go?) as a question row.
- `M7/30b-resize` unchanged.

## Gates
Workspace-wide, quiet: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Own worktree, own `CARGO_TARGET_DIR`, one cargo command at a time.

## Size
About 60k: scout read 3k; Major 1 and its tests 12k; Minor 1 and B1–B3 (the shim) 12k; K1–K3 and recovery 14k; K4,
the notice and captures 10k; Minors 2, 3, 6, 7 6k; docs 4k. Over budget → hand off after the graft-ops-only part
(A, B, K1–K3, Minors 3, 6), leaving K4, the notice, Minor 7 and all of Minor 2 (a field removal can't be split
across crates without breaking the build).

## NOTES (builder, 2026-09-27)
**COMPACTED once**, after K1–K3, the notice and its unit test. What remained then: K4 + captures, docs, the
Minor 3 check, gates, this ticket. All of it done in this session after the compaction.

**Red runs.**
- A4: left `["a.txt","gone.txt","link"]`, right `[…,"deleted.txt","edited.txt",…]` (old rule dropped every `A`).
- A1: left `["f.txt"]`, right `["f.txt","n.txt"]`.
- B1: `Checkout at 1: error: your local changes would be overwritten · u restores the backup` (no staging clause).
- B2: `Checkout at 1: error: could not drop · your edits are back · a copy stays in stash@{0} · u restores the backup`.
- B3 red: `checkout other: error: the test shim refuses this drop · your edits are back · a copy stays in stash@{0} · u restores the backup`.
  **B3 green:** `checkout other: error: the test shim refuses this drop · staging not kept: f.txt · your edits are back · a copy stays in stash@{0} · u restores the backup`.
  It names `u`; `u` gives the whole state = before and the stash list = before (asserted).
- K1/K2/K2b: left `[]`, right `[(id, Stash { n: 0 })]` / `[(id, Back { copy: Some(0) })]`. Their loss and undo asserts
  were green first (undo restores the stash list too): ADR-0045 §6 holds at both kill points.
- K3: left `None`, right `Some(Stash { n: 1 })`.
- Notice unit test (`update::undo::tests::set_aside::the_notice_words_where_recovery_found_the_edits`): red
  `…checkout other · edits in stash@{0} · J to inspect`. The pattern test
  `update::undo::tests::set_aside::relaunch_after_a_kill_names_the_stash_the_edits_are_in` stays green unchanged.
- K4: red (new branch disabled) `an operation was interrupted: checkout fix · edits in stash@{0} · J to inspect`.
- M6, A3, T5's Log assert: green first.

**A2 (finding, measured).** The ticket expected `AD` (staged new, then deleted) to be named. It isn't, and naming it
would be false: `git stash create` stores the **staged** blob in `S`'s tree for a path deleted from disk, so
`S^2:n.txt == S:n.txt` and the plain apply re-adds it. Git after: ` M f.txt`, `A  n.txt`, `:n.txt` = v1, `n.txt` on
disk = v1. Staging kept; the **deletion** is lost, by the stash itself (the `--index` path has the same `S`; not
probed there: unverified). `u` exact. Test renamed `put_back_leaves_a_staged_new_file_deleted_since_unnamed`.
ADR-0045's dated line records it. Gap for the conductor: a set-aside brings back a staged-new file he deleted from
disk (unverified on the `--index` path; `u` restores).

**80 columns (open, needs a decision).** The relaunch notice has no ladder. At 80x24 it is drawn
`an operation was interrupted: checkout fix · your edits are back · a copy stay…`: `stash@{0}` and `J to inspect`
are cut (`docs/shots/m7/30c-relaunch-edits-back-80x24.txt`, last row). The old line (77 chars) fit. The notice is one
string in `crates/graft/src/view/frame/bars.rs:307-308` → `sanitize_line`, cut at the width; the fallback line uses
rungs (`update/operate.rs` `staging_not_kept_rungs`). A ladder for the notice is a design choice (which part goes
first), not mine: filed here. K4 asserts `checkout fix · your edits are back` at 80 and the whole clause at 120.

**Minor 3 check.** `RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc --no-deps -p graft-ops` fails with 5
errors, none in `put_back.rs`: `apply/error.rs:95` and `:120` (`resume`, 2026-09-23), `apply.rs:80`
(`inside_live_operation`), `apply.rs:93` (`snapshot::chain_backup`), `rewrite/dependents.rs:66` (`plan`). Not fixed:
out of this ticket's files.

**Construction sites (Minor 2; scout (d) was missing, found by grep).** `SetAside { staged }` removed at:
graft-ops `operation.rs` `sets_aside`, `operation/history/tests.rs`, `tests/it/history_ops.rs` (2),
`tests/it/absorb_staged.rs`, `tests/it/set_aside.rs` (round trip; now asserts `staging_lost == 0`); graft
`update/preview/tests.rs` (2, the "on purpose" comment gone), `update/preview.rs` comment. `JournalRead` gained
`interrupted_edits`: every literal given `Vec::new()` (≈17, graft src and tests), `update/tests.rs`
`every_field_spelled_out` given a value.

**Other decisions.**
- Recovery's reads for `edits_left` failing → no row, commented: recovery must still close the entry, and the notice
  falls back to its older, still-true words. Not an error swallowed silently: the choice is written at the site.
- K4 lives in `m6_stash_put_back/set_aside.rs` as named (the fixture is there); its child is
  `child_switches_until_the_drop` (`#[ignore]`d). One shim script, shared: graft includes
  `crates/graft-ops/tests/it/git_shim.sh` by `include_str!`.
- K4 waits for the radar (`captures::radar_answered`) before drawing: under a full run the `ⓘ` on `fix` was missing
  once (the radar is its own job). Cause, not a retry.
- `interrupted_notice` split (`interrupted_words`) to stay under clippy's line limit; no behaviour change.

**Outside the crate set's tickets, but in the crates.** Base `be0a7ca3` did not compile: `graft/src/jobs/detail.rs`
matched `BlameError` without `TooLarge`/`Changed` (the 18e × 20i merge). I added both to the `Failed` arm so the
gates could run. Main's `6ed55d2f` changes the same file (`jobs/detail.rs`, per its stat): expect a conflict on
merge there; take main's (not compared line by line).

**Touches.** `staging_lost` callers: `run_put_back` only (grep); `absorb_stopped_then_continued_elsewhere_says_the_staging_went`
green, paths unchanged. Command Log: T8 `both_applies_refused_keeps_the_stash` green; `grep -rln diff-tree
crates/*/tests docs/shots` hits only unrelated reads and `put_back_index.rs`. `ApplyError::Git` words reach graft
through `explain()`; `grep -rln "a copy stays" crates/graft` exact asserts all green. `EditsAt`/`Recovered`: no
`Serialize` (grep). Journal browser `J` row and the graph's `interrupted` `RowMark`: no code path touched (they don't
read `interrupted_edits`; grep). `live_entries` beats the notice:
`conflict_interim::the_footer_does_not_call_a_live_merge_interrupted` green. `m6_stash_put_back::kill::edits_survive_a_kill_at_every_step`
now asserts the new clause, green. Keymap: no key. Captures: `grep -rln "edits in stash" docs/shots` → only the two
new 30c files' headers; `docs/shots/m7/30b-staging-not-kept-*` unchanged (green).
