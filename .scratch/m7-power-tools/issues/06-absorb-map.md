# absorb-map — each staged hunk goes to the commit that last changed its lines; one that fits none stays staged

Status: ready
Blocked by: 02-rewrite-engine
Crates: graft-ops only (`absorb`, new; `rewrite::plan` gains the `Absorb` arm)
Source: note "Pass 2a" R2; prior art "Absorb" (git-absorb, jj; `docs/research/interactive-rebase.md` §4.4).
Build with: Opus. The algorithm is decided below. An oracle judges it (the final tree equals the old tree plus the
staged hunks, and each placed hunk lands in the commit that last changed those lines), so it is not Fable.
Scout: `scout` first (brownfield: the hunk and diff reads, and blame if any exists yet).
Needs: rewrite-engine (`RewriteRequest`, `RewriteParams`, `plan`).

## Decided
- **Staged hunks:** `git diff --cached -U0 --no-color`, parsed with the existing hunk parser if there is one (scout).
- **Placing a hunk** in `base..HEAD`:
  - Blame its old lines at HEAD, restricted to the range (`git blame --porcelain -L a,b <base>..HEAD -- <path>`).
  - Every old line blamed to one commit C in the range → C.
  - A pure insertion (empty old range): the line above and the line below must both blame to the same C in the range →
    C. Otherwise it stays.
  - Old lines blamed to 2+ commits → `TwoCommitsChangedIt`.
  - Any old line blamed outside the range → `BeforeYourBranch`.
  - A new file → `NewFile`. `NoCommitChangedIt` is for the other cases.
- **Commutation check:** a hunk placed in C must also apply cleanly on C's tree after C's own change; checked with
  `git apply --cached --check` against a temporary index (no real index written). If it fails → stays
  (`TwoCommitsChangedIt`).
- **`RewriteRequest::Absorb`:**
  - Each placed group (hunks per target) becomes one commit object (`made`), planned as a `Fixup` line after its
    target.
  - Hunks that stay remain staged, as the set-aside put-back leaves them. If the put-back loses the staged split
    (KNOWN_GAPS :804), return that fact so the preview can say so.
  - No hunk placed → `Refused(NothingToAbsorb)`, with the note's words.

## Read first
- Note "Pass 2a" R2 (the absorb mock and its words); "Interfaces" `graft-ops::absorb`.
- `docs/adr/0047-…` §1, §5.
- `crates/graft-ops/MAP.md` `history_reads`, `rewrite` (from rewrite-engine); `crates/graft-git/MAP.md` diff/hunk
  entries.

## Scout brief (≤ 1 page)
1. Is there a hunk parser for `-U0` diffs in graft-git or graft-ops? Its type and where it's used.
2. Does any blame read exist yet? (G-blame is pass 2b: if none, this ticket's blame spawn stays private to `absorb`.)
3. How a temporary index is made today (84's `put_back` forecast), so the same helper can be reused.

## Signatures
```rust
pub struct StagedHunk { pub path: PathBuf, pub old: Range<u32>, pub new: Range<u32> }
pub enum StaysWhy { NoCommitChangedIt, TwoCommitsChangedIt, NewFile, BeforeYourBranch }
pub struct AbsorbMap { pub placed: Vec<(StagedHunk, Oid)>, pub stays: Vec<(StagedHunk, StaysWhy)> }
pub fn absorb_map(repo: &Repo, git: &Git, head: &Tip, base: Oid, stop: &dyn Stop) -> Result<AbsorbMap, FactsError>;
// RewriteRequest::Absorb; Refusal::NothingToAbsorb
// Display: "none of your staged lines belong to a commit on your branch · c commits them"
```

## Acceptance
- [ ] `crates/graft-ops/tests/it/absorb.rs`:
  - `a_hunk_goes_to_the_commit_that_last_changed_its_lines` (red first);
  - `an_insertion_between_one_commits_lines_goes_there`;
  - `a_hunk_over_two_commits_stays`;
  - `a_hunk_on_lines_older_than_the_branch_stays`;
  - `a_new_file_stays`;
  - `a_hunk_that_does_not_commute_stays`;
  - `absorb_with_nothing_placed_is_refused`;
  - `absorb_map_writes_no_ref_index_or_file`;
  - `absorb_map_honours_stop`.
- [ ] **Oracle** `absorb_agrees_with_the_trees`, on 5 rows (one target; two targets; mixed placed and stays; an
      insertion; an unsaved edit alongside). Plan and apply, then:
  - the new HEAD tree = old HEAD tree + placed hunks;
  - the index = new HEAD + stays hunks (or, when the put-back loses the split, the worktree has them and the returned
    flag says so);
  - each target's new commit diff contains its hunks.
- [ ] MAP.md (graft-ops `absorb`).

## Touches (check each, report how)
- **`rewrite::plan`:** a new arm only. Check: the rewrite-engine tests green.
- **The index:** read only here, except inside the apply the engine already records. Check: the writes test above.
- **Other crates:** none (graft wires it in `amend-into-and-absorb-keys`). Check: the workspace builds.
- **Journal / undo:** through the engine's entry. Check: `undo_after_absorb_restores_index_and_commits` (add it here).
- **Captures / keymap:** none.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~50k).
