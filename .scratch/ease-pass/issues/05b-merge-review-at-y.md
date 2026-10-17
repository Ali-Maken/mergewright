# merge-review-at-y — a merge stages once, at the review's `y`; the stop lands on the clash; `esc` pauses

Status: ready
Blocked by: 05-resolver-picks, 03-ease-vocabulary, 03b-ease-vocabulary-rest
Build with: Fable — passes all three tests: the approach is decided here (Hard choice 3), a wrong staging model is structural (every finish path, both merge origins, relaunch), and index states can't be judged from the diff.
Needs: from 05: `Picks`, `Pick`, `Restore`, `apply_untake`, `next_open`, `next_open_in_file`. Existing: `Resume` (`graft-ops/src/apply/resume.rs:38-52`: `Continue` unit, `ContinueWith { message: Vec<u8> }`, `Skip`, `Abort`), `FinishMergeParams { merge_head: Oid, message: Option<Vec<u8>> }` (`graft-ops/src/operation/finish_merge.rs:15`), `finish_summary` (`graft-ops/src/finish.rs:139`), `FinishSummary`, `SummaryError`, `Opened` (graft-ops MAP :73). Ids: `resolver.finish` (`c`, `keymap/table.rs:325`), `app.back` (`esc`, :77), `app.undo` (`u`, :78), `confirm.yes` (`y`, **`keymap/table.rs:235`**, borrowed by `Finish` — borrows comment `graft-ui/src/keymap.rs:202-204`). From 03b: `ClashBar { at: AllSolved, op: Merge }`, `Banner { merge: true }` (`⏎ back to the clashes`). **This group adds the `resolver.review` row itself.**

## Read first
- `docs/design-notes/ease-pass.md`: Journeys (merge-one-clash, merge-three-clashes, pause-and-resume and the landing points), Mock behaviours M5, M9–M12, M14–M16, Hard-to-reverse choice 3, Questions 7, 12, 17.
- `docs/design/02-keymap.md` §2.6 (both tables), §2.6a (banner table and the `u` paragraph), §2.6b.
- `crates/graft-ops/MAP.md`: `Resume`, `resume`, `finish_summary`, `FinishMergeParams`, `conflict` (:73-79). `crates/graft/MAP.md`: `update::resolver`, `update::finish`, `update::banner`, `update::relaunch`, `jobs::finish`. `crates/graft-ui/MAP.md`: `keymap`, `graph_screen::hints`.
- `.scratch/ease-pass/issues/03-ease-vocabulary.md` Handoff blocker 1 (the row line); `03b-ease-vocabulary-rest.md` Signatures.
- The scout's file, and 05's "Decided here".

## Decided here (scout contradictions resolved)
- **One finish path for both merge origins.** A shell-begun merge has no `Pending` entry; `resume` routes it to `Target::ShellMerge` → `apply(FinishMerge)` (`resume.rs:100-105,134`), built at `apply/resume/shell.rs:59` from the same `Resume`. So `solved` rides on `Resume` and is copied into `FinishMergeParams` there; no second path.
- **Staged once, inside the finishing operation**: the Graft-begun continue stages `solved` then commits, inside its `Pending` span; `FinishMerge` stages `solved` then commits, as its own journaled entry. `u` after landing (M19) undoes the whole finish, staging included (undo restores HEAD and index).
- **"Solved" is the resolver's classification, never "no markers"** — a binary or delete/modify conflict has no markers and is not solved:
  `FileState::Open` = unmerged and (a marker document with open blocks, or a whole-file card, `Opened.file == None`); `Solved` = unmerged, a marker document, 0 open blocks (picks or a hand edit); `Done` = git no longer calls it unmerged (staged — a whole-file choice, or a stage from the shell). Relaunch reads the same rule, so files solved before a quit read as solved.
- **The review reads a scratch index, by the read-only env route**: copy the real index to a temp file, `git add -- <solved>` and the summary's reads with `GIT_INDEX_FILE` set — the snapshot plumbing's route (`graft-ops/src/snapshot/plumbing.rs:15-39`, `:145-154`; env at :24, :39, :154). `IndexFile::Scratch` is the executor's, for writes — not used. Temp file removed on every exit path.
- **Q7 `esc`** lands on the banner, focused, only when the resolver was opened from the banner (`leave.rs:55-58`). The merge stop now opens the resolver itself, so it opens it **as `OpenedFrom::Banner`** (the stop *is* the banner's operation). If `Banner` drives anything besides `leave.rs:55`, add `OpenedFrom::Stop`, treated as `Banner` at `leave.rs:55`, and say which in NOTES. Opened from the changes pane (`OpenedFrom::Pane`): today's `esc`.
- **Stop landing is merges only** (rebase, cherry-pick, revert keep today's: graph, banner focused, `relaunch.rs:49 land_on_live_operation`). §2.6a already says every stop opens the resolver; the code doesn't, and he has used the code — **Question 17**. Build (b) today's for non-merges; (a) is one branch at the landing site.
- **A relaunch mid-merge is a pause**: it lands as today (graph, banner focused, `relaunch.rs:49,85,105`); `⏎` → first open clash. Picks don't survive it (05).
- **Review `u`** acts where review `y` acts (Files focus, `finish/keys.rs:100`); in a text field (Subject/Body, `:117`, `body_key`) it is typed text, as `y` is. If 12 (Fr) removes the review's text fields, `u` acts everywhere in the review. The silent `_ => {}` at `finish/keys.rs:106` no longer catches `u`.
- **Banner `c` in a merge** (knock-on: it refuses "while any path is still unmerged", and solved files now stay unmerged until `y`): every unmerged path `Solved` → the review (as resolver `y`); else today's refusal, counting `Open` files.

## Scope
In (graft-ops `apply/resume*`, `operation/finish_merge.rs`, `finish.rs`, `conflict` (`file_state`); graft app `update::resolver`, `update::finish` (`keys.rs` `y`/`u`/`esc` only), `update::banner` (`⏎`, `c`), `update::relaunch`, the merge-stop landing; graft-ui `keymap/table.rs` (one row), `tests/keymap_ids.txt`, `keymap/tests/ease.rs`, `graph_screen/hints/lists/resolver.rs` (`ClashOp::Merge` + `ClashAt::AllSolved` arm only)). Owns `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` in its wave.
- **`resolver.review` row with its handler, one commit**: `b("resolver.review", Resolver, &[k('y')], "Review the merge (a merge only)")` after `resolver.finish`; id added to `tests/keymap_ids.txt`; the `None` assert for `y` (`keymap/tests/ease.rs:26`) flipped to `resolver.review`; `bound.rs:127,130` resolver list updated. `y` reaches the resolver by a new arm in `act` (`resolver.rs:343-385`, today `_ if plain => scroll_key` :382) that matches the translated id, as `asks_undo` (:394) does. In a rebase/cherry-pick/revert/stash clash `y` is not drawn and answers as an unbound key does there today.
- **Merge all-solved bar**: the `ClashBar { at: AllSolved, op: Merge }` arm reads `y review the merge · u undo pick · esc pause, come back later` (03 Handoff, exact); `y` spelled from `resolver.review`. Other arms untouched.
- **Merge staging moves to `y`**: in a merge `o/t/b`, `⌃O/⌃T` and `O/T` stage nothing. `O`/`T` in a merge = every open block of the file, one pick, then the cursor to `next_open` (the "mark done" half has no meaning without a `⏎` step); `⌃O`/`⌃T` = the same, cursor stays. Rebase/cherry-pick/revert/stash keep 05's (today's) staging.
- **`⏎` in a merge**: `Solved` file → cursor to `next_open`; `Open` file → today's refusal (`mark_resolved_refusal`, `resolved.rs:39`); nothing open anywhere → notice `every clash is solved · y reviews the merge`. Nothing staged. P11 dropped for merges: `stay_after_last`/`finish_owed`/`finish_if_owed` (`resolved.rs:211,220,286,316`) no longer fire for a merge.
- **`n` with none open, in a merge** → notice `no open clashes left · y reviews the merge` (05 left today's words).
- **`y` (`resolver.review`) and `c` (`resolver.finish`) in a merge**: no `Open` file → the review; else notice `N clashes still open · n goes to the next`. `c` elsewhere unchanged.
- **Review** (`ContextId::Finish`), facts from `finish_preview(repo, solved)`: `y` (`confirm.yes`) → `Resume::Continue { solved }` / `ContinueWith { message, solved }` → (shell-begun) `FinishMerge { solved }`; `u` → back to the resolver, last pick undone (05's `u`), cursor on it; empty stack → notice `nothing to undo here`, review stays; `esc` (`app.back`, `finish::escape` :44) → the resolver unchanged. A failed finish (a hook refuses) → the review stays with today's failure words; `y` again works.
- **Merge stop lands on the resolver**, cursor on `next_open` (0 keys), opened as Decided.
- **Q7 a**: `esc` → graph, banner focused, bar leads `⏎ back to the clashes`; banner `⏎` (`banner/pressed.rs:71`) in a merge → the resolver, cursor on `next_open`, picks kept. Journey `pause-and-resume` = `esc ⏎`.
- **Relaunch**: file states by `file_state`; lands as Decided.
Out: drawing the review and look (12, Fr); the clash screen's look (11, L); 05's pick mechanics.

## Signatures
graft-ui keymap (new row, `defaults()`): `resolver.review  Resolver  [KeyChord::key('y')]`.

graft-ops:
```rust
pub enum Resume { Continue { solved: Vec<PathBuf> }, ContinueWith { message: Vec<u8>, solved: Vec<PathBuf> }, Skip, Abort }
pub struct FinishMergeParams { pub merge_head: Oid, pub message: Option<Vec<u8>>, pub solved: Vec<PathBuf> }
/// The review's facts as if `solved` were staged: a scratch index, never the real one.
pub fn finish_preview(repo: &RepoHandle, solved: &[PathBuf], stop: &dyn Stop) -> Result<FinishSummary, SummaryError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileState { Open { open: usize }, Solved, Done }
/// `open` is 0 for a whole-file card (`file == None`) — still `Open`.
pub fn file_state(opened: &Opened, unmerged: bool) -> FileState;
```
`solved` is empty for every non-merge `Resume` and for every site that isn't the merge review (construction sites: `u/finish/keys.rs:236,251,253`, `u/banner/pressed.rs:91`; matches `u/banner/resume.rs:32`, `banner/resume/answers.rs:150,165`, `jobs/runner/operations.rs:36`; tests `finish/tests.rs`, `banner/tests.rs`, `quit/tests.rs:304,753`, `keys/live_c_tests.rs:254`, `banner/rewrite_stop_tests.rs`, `operation/tests.rs:1266`).

## Touches
Every `Resume`/`FinishMergeParams` site above; `resume` (`resume.rs:100-105,134`) and `shell.rs:59`; `finish_summary` (kept for any non-merge caller; `StillUnmerged` at :147-150 no longer reached from a merge review); journal entries for finish/abort (unchanged count: one per finish); `u` routing (§2.6a refusal from the graph and banner — test); banner `⏎`/`c`; the finish summary for non-merges (unchanged); the stash clash close path (unchanged); relaunch; key table golden and clash check (`keymap/clash.rs:21`); `?`/`⌃P` listing (`y` in `Resolver`); graft sweeps (`update/tests/sweep.rs`, `tests/palette_live.rs`, `tests/keymap_edge/sweep.rs`; ui `keymap/sweep.rs`); man page key table (regenerate, diff one row); `git status` from the shell mid-merge now shows solved files unmerged until `y` (NOTES says so — Hard choice 3's known cost).

## Acceptance
- [ ] `y` → `resolver.review` in `Resolver`; no chord bound twice (`keymap::tests::table::every_action_has_a_default_and_no_context_binds_one_chord_twice`); `keymap::tests::ease::the_clash_screen_keys_resolve_to_their_ids` asserts `resolver.review`.
- [ ] `tests/keymap_ids.txt` gains exactly `resolver.review` (`action_ids_match_the_golden_list`); graft sweeps green (`every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `dead_rows_help_lists_elsewhere_match_the_known_table`).
- [ ] Merge all solved → bar reads `y review the merge · u undo pick · esc pause, come back later`; a rebase all solved draws no `y`.
- [ ] Fixture merge, one file, 2 clashes: stop lands on the resolver, cursor on clash 1 (0 keys); `o o` → file `Solved`, `git ls-files -u` still lists it.
- [ ] `y` → review (Files focus), facts from `finish_preview`; the real index's bytes are unchanged by the preview (hash before/after); no temp index left behind.
- [ ] Review `y` → merge commit with both parents, index clean, one journal entry; `u` from the graph then puts the merge back.
- [ ] Review `u` → resolver, last pick open, cursor on it. Review `esc` → resolver unchanged. `c` all solved → the same review.
- [ ] Journey `m y o y y` lands on the graph at the merge commit (state test; the pty walk is 14's).
- [ ] Q7: `esc` → graph, banner focused, bar leads `⏎ back to the clashes`; `⏎` → resolver on the first open clash; earlier picks still taken on disk and `u` still undoes them.
- [ ] Shell-begun merge (`git merge` outside, relaunch): lands on the banner; `⏎ o y y` finishes through `FinishMerge` with `solved`.
- [ ] Relaunch after solving on disk but before `y`: those files `Solved`; a binary conflict and a delete/modify conflict in the same fixture read `Open` (`file_state` table test covers each `UnmergedHow`).
- [ ] Banner `c` mid-merge: all solved → review; one open → refusal naming 1 open file.
- [ ] Hook-fail fixture: the hook today's merge finish runs (name it in NOTES — `git commit` concluding a merge runs `pre-commit`/`commit-msg`, not `pre-merge-commit`) exits 1 → review stays with the failure words; fix the hook, `y` commits.
- [ ] Rebase stop still lands on the graph, banner focused (Q17 b); `⏎` `c` unchanged. From the graph mid-merge `u` refuses with §2.6a's words. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~65k. Split point if it overruns: row + bar + review keys + staging here; Q7 pause, banner `c` and relaunch as a follow-on.
