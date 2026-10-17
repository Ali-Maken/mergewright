# resolver-picks — take, advance in the file, undo a pick, stage once at the review's `y`

Status: ready (ids revised 2026-10-17 after ease-vocabulary's scout; `resolver.review` row and the merge all-solved bar moved here from 03 on 2026-10-17 — 03 Handoff blocker 1; this group's own scout not yet run)
Blocked by: 02-clash-reads, 03-ease-vocabulary, 03b-ease-vocabulary-rest
Build with: (blank — brownfield; conductor sets after the scout). **Planner recommends Fable**: passes all three tests — approach decided here, a wrong staging model is structural, index states can't be judged from the diff.
Needs: `ConflictFile::reopen(&mut self, block: usize) -> Result<(), NoSuchBlock>` (clash-reads; `NoSuchBlock(pub usize)` is the existing type, `graft-git/src/conflict.rs:329`, already returned by `take` and handled at `graft-ops/src/conflict.rs:125,160`); existing ids, unchanged, whose handlers this group changes: `resolver.finish` (`c`, `keymap/table.rs:325`), `resolver.next_block` (`n`, `:323`), `app.back` (`esc`, Global `:77`, handled `graft/src/update/resolver.rs:363` → `escape`), `app.undo` (`u`, Global `:78`, answered `resolver.rs:379` via `asks_undo`); `confirm.yes` (`y`, borrowed by `Finish`, `keymap.rs:204`). From 03b: `HintContext::Clash(ClashBar { at: ClashAt, op: ClashOp })` (its `u`/`n`/`esc` words already drawn — this group makes them true), the live merge banner's `⏎ back to the clashes` (`Banner { merge: true }`); hint verbs are `Cow<'static, str>`. **This group adds the `resolver.review` row itself** (below).

## Read first
- `docs/design-notes/ease-pass.md`: Journeys (merge-*, undo-a-pick, pause-and-resume) and landing points, Mock behaviours M5, M8–M10, M12, M14–M16, Hard-to-reverse choice 3, Questions 7 and 12.
- `docs/design/02-keymap.md` §2.6, §2.6a, §2.6b.
- `crates/graft-ops/MAP.md`: `Resume`, `resume`, `finish_summary`, `SummaryError`, `FinishMergeParams`, `apply_take`, `TakeError`, `Opened`. `crates/graft/MAP.md`: `update::resolver`, `update::finish`, `update::banner`, `update::relaunch`, `update::clash`. `crates/graft-ui/MAP.md`: `keymap`, `graph_screen::hints`.
- `.scratch/ease-pass/scouts/03-ease-vocabulary.md` ("Who reads the ids today"); `.scratch/ease-pass/issues/03-ease-vocabulary.md` Handoff blocker 1 (the exact row line); `03b-ease-vocabulary-rest.md` Signatures.
- The scout's file.

## Scout questions
- Where a stop lands today and what P11 auto-open does; what `⏎` stages in the resolver and how "file done" is tracked.
- How `finish_summary` reads (index vs disk) and how relaunch finds a live merge's unmerged files.
- Every `Resume` / `FinishMergeParams` construction site, and the existing types of `Resume::ContinueWith`'s and `FinishMergeParams`'s message fields (kept as they are; only `solved` is added).
- How a stash clash (`update::clash`) enters and leaves the resolver.
- **Key routing (same kind of mismatch as V's scout):** the resolver matches `KeyCode`s directly (`resolver.rs:363-380`) — how will `y` reach it as `resolver.review` (translated chord, or a new match arm)? Does the finish summary answer `app.undo` today, or swallow it (`stands_down`)? Does `escape` (`:363`) already land on the banner, focused (§2.6a says "`Esc` there lands on the banner")? What `N` (`:373`) does today.
- `finish_preview`'s scratch index: the snapshot code's `GIT_INDEX_FILE` route (`graft-ops/src/snapshot/plumbing.rs:117`) or commit-what-goes-in's `IndexFile::Scratch` (an executor feature for writes) — which fits a read.

## Scope
In (graft-ops; graft app `update::resolver`, `update::finish`, `update::banner`, `update::relaunch` and their tests; graft-ui `keymap/table.rs` (one row), `tests/keymap_ids.txt`, `keymap/tests/ease.rs`, `graph_screen/hints/lists/resolver.rs` (the `ClashOp::Merge` + `ClashAt::AllSolved` arm only)):
- **`resolver.review` row, with its handler, in one commit** (03 blocker 1): `b("resolver.review", Resolver, &[k('y')], "Review the merge (a merge only)")` after `resolver.finish`; the id added to `tests/keymap_ids.txt`; the `None` assert for `y` in `keymap::tests::ease::the_clash_screen_keys_resolve_to_their_ids` flipped to `resolver.review`. In a rebase/cherry-pick/revert/stash clash `y` is not drawn and answers as an unbound key does there today.
- **Merge all-solved bar:** the `ClashBar { at: AllSolved, op: Merge }` arm's words become `y review the merge · u undo pick · esc pause, come back later` (03 Handoff, exact); `y` spelled from `resolver.review`. Other arms untouched (03b owns them).
- Stop lands on the resolver, cursor on the first open block of the first file (0 keys). P11 auto-open of the summary dropped for merges.
- `o`/`t`/`b`: `apply_take`, push a `Pick`, cursor to `next_open_in_file`; file solved → stays, cursor stays; **nothing staged in a merge**. On a taken block: notice `already taken · u undoes it`, no write.
- `n` (`resolver.next_block`): `next_open` across files (list order); none open → notice `no open clashes left · y reviews the merge` (merge) / today's finish words (others). `N` keeps today's meaning.
- `u` (`app.undo`, answered in the resolver where `asks_undo` answers it today): pop the last pick, `reopen` + `write_back` against the fingerprint, cursor on that block (switching file if needed). File changed outside → notice in words, stack kept. Empty stack → notice `nothing to undo here`. A `NoSuchBlock` from `reopen` (stack and file out of step) → notice in words, stack kept, nothing written — never swallowed.
- `y` (`resolver.review`) and `c` (`resolver.finish`) in a merge when every file is solved → the review (Fr draws it); with open clashes → notice `N clashes still open · n goes to the next`. In a rebase/cherry-pick/revert/stash clash `y` does what an unbound key does there today; `c` unchanged.
- Review (`ContextId::Finish`): `y` (`confirm.yes`) commits via `Resume::Continue{solved}` / `ContinueWith{message, solved}` or `FinishMerge` with `solved`, staging the solved files **once**; `u` (`app.undo`) → back to the resolver with the last pick reopened; `esc` (`app.back`) → resolver unchanged.
- `⏎` in a merge: on a solved file → next file; on an open file → refused as today.
- Relaunch mid-merge: an unmerged file with no markers counts as solved.
- **Q7 a — pause:** `esc` (`app.back`) on the clash screen → the graph with the operation banner drawn on top and **focused** (today's §2.6a banner); its bar leads `⏎ back to the clashes` (03b's words on `Banner { merge: true }`). Banner `⏎` → the resolver, cursor on the **first open clash** (`next_open`), picks kept. Journey `pause-and-resume` = `esc ⏎`, 2 keys.
- **Q12 a — rebase, cherry-pick, revert, stash put-back:** the same advance-in-file, `n` to the next open clash, and `u` pick undo as a merge. Finishing stays as today: `⏎` marks the file done **and stages it**; `c` goes on (rebase/cherry-pick/revert); a stash clash's last `⏎` closes to the changes pane as today. `u` on a file already marked done refuses `<file> is marked done · d starts it over`. Picks in a file not yet marked done undo as in a merge.
Out: drawing (L, Fr), the look screen content (Fr), every other hint arm (03b), the review and look bars (Fr).

## Signatures
graft-ui keymap (new row, `defaults()`): `resolver.review  Resolver  [KeyChord::key('y')]`.

graft-ops (message field types as they are today — scout names them):
```rust
pub enum Resume { Continue { solved: Vec<PathBuf> }, ContinueWith { message: /* today's */, solved: Vec<PathBuf> }, Skip, Abort }
pub struct FinishMergeParams { pub merge_head: Oid, pub message: /* today's */, pub solved: Vec<PathBuf> }
/// The review's facts as if `solved` were staged: reads a scratch index, never the real one.
pub fn finish_preview(repo: &RepoHandle, solved: &[PathBuf], stop: &dyn Stop) -> Result<FinishSummary, SummaryError>;
```
graft app (`update::resolver`):
```rust
pub struct Pick { pub path: PathBuf, pub block: usize, pub take: Take }
#[derive(Default)]
pub struct Picks { /* stack */ }
impl Picks {
    pub fn push(&mut self, pick: Pick);
    pub fn pop(&mut self) -> Option<Pick>;
    pub fn last(&self) -> Option<&Pick>;
    pub fn is_empty(&self) -> bool;
    pub fn in_file(&self, path: &Path) -> usize;
    /// Drops the picks in `path` (a non-merge file marked done with `⏎`).
    pub fn seal(&mut self, path: &Path);
}
pub struct BlockAt { pub index: usize, pub open: bool }
/// First open block, top-down.
pub fn next_open_in_file(blocks: &[BlockAt]) -> Option<usize>;
/// (file index, block index) of the first open block in list order.
pub fn next_open(files: &[(PathBuf, Vec<BlockAt>)]) -> Option<(usize, usize)>;
```
`Pick.block` indexes as `ConflictFile::take`/`reopen` do. `Picks` lives for one operation; cleared on finish or abort; kept across pause (`esc`) and relaunch-free resume; not journaled (a pick is a disk write inside the live operation, which the operation's backup already covers — scout confirms).

## Touches
Every `Resume` and `FinishMergeParams` site; `resume` for shell-begun merges; journal entries for finish/abort; `u` routing (§2.6a refusal must stay for every other screen mid-operation — test it from the graph); `asks_undo` (`resolver.rs:379`) and every other place that answers `app.undo` mid-operation; banner focus and its bar; finish summary for non-merge operations (unchanged); stash clash close path (`update::clash`); relaunch path; resolver and finish captures (recaptured by L, Fr); existing `NoSuchBlock` handling in `graft-ops/src/conflict.rs:125,160` (unchanged); key table golden and clash check (`keymap/clash.rs:21`); `?`/`⌃P` listing (`y` now listed in `Resolver`); graft dead-key sweeps (`sweep.rs:185`, `palette_live.rs:222`); man page key table (regenerate and diff: one row).

## Acceptance
- [ ] `y` → `resolver.review` in `Resolver`; no key bound twice in one scope (`keymap::tests::table::every_action_has_a_default_and_no_context_binds_one_chord_twice`); `keymap::tests::ease::the_clash_screen_keys_resolve_to_their_ids` asserts `resolver.review` (was `None`). Closes 03's open boxes for `y`.
- [ ] `tests/keymap_ids.txt` gains exactly `resolver.review`, nothing renamed or removed (`keymap::tests::table::action_ids_match_the_golden_list`).
- [ ] graft sweeps green with the row live: `every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `dead_rows_help_lists_elsewhere_match_the_known_table`.
- [ ] Merge all solved → bar reads `y review the merge · u undo pick · esc pause, come back later`; a rebase all solved keeps 03b's `c` words and draws no `y`.
- [ ] Fixture merge, one file, 2 clashes: `o` → cursor on clash 2, file still listed `?`; `o` → file `✓`, cursor stays; index still unmerged for that path (`git ls-files -u` non-empty).
- [ ] `u` → clash 2 open again on disk (bytes equal pre-pick), cursor on it; `u` again → clash 1.
- [ ] Changed-outside: edit file between pick and `u` → notice, disk untouched, stack kept.
- [ ] `y` all solved → review state; `c` all solved → the same review state; review `y` → merge commit with both parents, index clean; review `u` → resolver, last pick open.
- [ ] Journey `m y o y y` lands on the graph at the merge commit (state test; the pty walk is A's).
- [ ] Q7: `esc` on the resolver → graph, banner focused, bar leads `⏎ back to the clashes`; `⏎` → resolver, cursor on the first open clash, earlier picks still taken on disk and `u` still undoes them.
- [ ] Shell-begun merge (`git merge` outside, then relaunch): same keys, finishes through `FinishMerge` with `solved`.
- [ ] Relaunch after solving on disk but before `y`: files read as solved.
- [ ] Q12: rebase stop, one file with 2 clashes: `o` advances within the file; `u` undoes it; `⏎` stages the file (`git ls-files -u` empty for it); `u` then refuses with `d starts it over`; `y` changes nothing; `c` continues the rebase.
- [ ] Q12: stash put-back clash: `o` then `u` restores the block; last `⏎` closes to the changes pane as today.
- [ ] From the graph mid-merge, `u` still refuses with today's §2.6a words. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~78k). If the scout finds the shell-begun path separate from `Pending`, split: picks + advance + `u` + Q12 + the `y` row and bar here, review `y` staging and Q7 pause as a follow-on.
