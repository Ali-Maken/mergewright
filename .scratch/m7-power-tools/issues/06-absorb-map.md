# absorb-map — each staged hunk goes to the commit that last changed its lines; one that fits none stays staged

Status: done (m7-power-tools-06). See NOTES.
Blocked by: 02-rewrite-engine, 02b-rewrite-engine-b
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
- [x] `crates/graft-ops/tests/it/absorb.rs` — each test below, by that name, green; plus
      `an_insertion_at_the_top_of_a_file_stays` and unit tests `absorb::tests::{runs_join_neighbouring_lines,
      porcelain_blame_reads_each_final_line}`:
  - `a_hunk_goes_to_the_commit_that_last_changed_its_lines` (red first — seen red against a stub returning an empty
    map: `left: []`);
  - `an_insertion_between_one_commits_lines_goes_there`;
  - `a_hunk_over_two_commits_stays`;
  - `a_hunk_on_lines_older_than_the_branch_stays`;
  - `a_new_file_stays`;
  - `a_hunk_that_does_not_commute_stays`;
  - `absorb_with_nothing_placed_is_refused`;
  - `absorb_map_writes_no_ref_index_or_file`;
  - `absorb_map_honours_stop`.
- [x] **Oracle** `absorb_agrees_with_the_trees` (`tests/it/absorb.rs`), one test per row:
      `absorb_agrees_with_the_trees_{one_target, two_targets, mixed_placed_and_stays, an_insertion,
      an_unsaved_edit_alongside}`. Expected HEAD tree built independently (porcelain `git diff --cached -U0`, each
      placed hunk applied alone with `--unidiff-zero` on a test-owned temporary index). On 5 rows (one target; two targets; mixed placed and stays; an
      insertion; an unsaved edit alongside). Plan and apply, then:
  - the new HEAD tree = old HEAD tree + placed hunks;
  - the index = new HEAD + stays hunks (or, when the put-back loses the split, the worktree has them and the returned
    flag says so);
  - each target's new commit diff contains its hunks.
- [x] MAP.md (graft-ops `absorb`) — `crates/graft-ops/MAP.md` `## absorb (m7/06)`, plus `rewrite` rows (no test; doc).

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

## NOTES (builder, 2026-09-26)

No compaction. No scout: the conductor waived it (rewrite built minutes before); the scout brief's three questions,
answered from the code:
1. Hunk parser: `graft_git::diff::parse_unified` (+ `hunk_patch`), used here. No `-U0`-specific parser exists.
2. Blame: none existed. The spawn is private to `absorb` (G-blame is pass 2b).
3. Temporary index: `snapshot::plumbing::TempIndex` (84's forecast uses it). Raised to `pub(crate)` with `path_of`.

Verified, and how:
- Gates, workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo nextest run` (3601
  passed, 1 skipped), `cargo test --doc`. git 2.54.0.
- Behaviour: the tests named in Acceptance, on real repositories. Not verified: anything in the app (graft wires it in
  `amend-into-and-absorb-keys`); a repo with no default branch and no upstream (the first-commit fallback, no test).

Deviations, for the conductor to accept or send back:
1. **The commutation check carries one line of `HEAD` context either side**, not the bare `-U0` hunk. Still `git apply
   --cached --check` on a temporary index, as decided. Why: with zero context, `apply` finds a changed line anywhere it
   matches and places an insertion by line number alone, so it passes exactly the case that conflicts on replay — a
   later commit changed the *neighbouring* line (git's merge conflicts on adjacent changes). With the context line,
   that case fails on C's tree. Proof: `a_hunk_that_does_not_commute_stays` (`d1` is f1's, f2 changed `d2` below it).
   Measured on that repository by a throwaway script (git 2.54): the bare `-U0` hunk `apply --cached --check
   --unidiff-zero` on f1's tree **applies**, and `git commit` + `rebase -i --autosquash main` **conflicts**.
   The same context patches build the made commits, so `--unidiff-zero` is never used in the code.
2. **Staged read is plumbing `diff-index --cached -p -U0 --no-renames <HEAD>`**, not porcelain `git diff --cached`:
   porcelain refreshes `.git/index` (a read must not write it; M2 review round 2) and follows `diff.*` config.
   `--no-renames`: a rename's new side is a new file (stays), never placed into a commit that had the old path.
3. **`AbsorbMap` gains `stays_unstaged: bool`** — the "return that fact" the ticket asks for. True when any hunk stays
   on a file `HEAD` has: the set-aside put-back has no `--index`, so those come back unstaged; a new file comes back
   staged. Proven both ways by the oracle (`mixed_placed_and_stays`: only `new.txt` staged after, the working tree holds
   every staged line; the other rows: the index tree equals the old index tree).
4. **Mixed in/out of range:** lines blamed to one commit in the range *and* one below it → `TwoCommitsChangedIt`
   (only all-below is `BeforeYourBranch`). The ticket's two rules overlap there; this is the order I chose.
5. **Insertion at the top or bottom of a file** (a neighbour missing) → `NoCommitChangedIt`. Test:
   `an_insertion_at_the_top_of_a_file_stays`. Binary and mode-only staged changes also stay `NoCommitChangedIt`, with
   `old`/`new` = `0..0` (no hunk to name). No test for binary/mode-only.
6. **Rows start at the oldest target**, `onto` = its parent (as AmendInto). Base = Open's base (upstream merge-base,
   else the default branch); none → the first commit.

Gaps (for KNOWN_GAPS, by the conductor):
- *Unverified:* `blame.ignoreRevsFile` in his config would change which commit blame names. Not overridden; the
  commutation check still guards a wrong placement from a stopped replay, not from a wrong-but-clean one.
- *Unverified:* binary / mode-only / whole-file-deletion staged changes: no rows.
- *Unverified:* the first-commit fallback when there is no upstream and no default branch.
- *Known, inherited:* hunks that stay come back unstaged (KNOWN_GAPS :804's put-back); flagged by `stays_unstaged` for
  the preview to say.

Touches:
- `rewrite::plan`: a new `Absorb` arm and `root_of`; Open/AmendInto untouched — `rewrite_plan`, `rewrite_oracle`,
  `rewrite_run` green in the workspace nextest.
- The index: read only — `absorb_map_writes_no_ref_index_or_file` (refs, `ls-files -s`, every working-tree file, and
  every file under `.git` but `objects/`, before = after, across `absorb_map` and `plan`).
- `snapshot::TempIndex`/`path_of` visibility `pub(super)` → `pub(crate)`: no behaviour change; snapshot tests green.
- `Refusal::NothingToAbsorb`, `RewriteRequest::Absorb`: new variants. graft matches `Refusal` with `other =>`
  (`graft/src/update/operate.rs:1097`, per 02 NOTES); `grep -rn RewriteRequest crates/graft crates/graft-ui` → none.
  Workspace clippy + nextest green.
- Journal / undo: through the engine's entry — `undo_after_absorb_restores_index_and_commits` (refs, index, files,
  HEAD all equal to before).
- Captures / keymap: none.
