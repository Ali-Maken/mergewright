# journal-list — the journal and git's reflog as one list: outside rows, lost-commit rows, what `⏎` puts back

Status: ready
Blocked by: 19-lost-commit-cost
Blocked by: lost-commit-cost (its `unreached` signature and measured method)
Crates: graft-core (`OperationKind::Recover`), graft-git (`reflog` module), graft-ops (`journal_list` module, and the
`RecoverTarget`/`RecoverParams` types in `operation`), graft-fixtures (the "lost work" shape). Not graft or graft-ui:
if graft fails to build on a new variant, stop and report `blocked: needs graft`, naming the match site.
Source: `docs/design-notes/m7-power-tools.md`: pass 1b "Journal"; "Pass 2b" F1, F4, J1, "Hard-to-reverse choices
(2b)" 5, "Interfaces (2b)"; ADR-0046 (accepted); field notes :72.
Build with: **Fable (signature)**. It fixes the interface `J`, the put-back and the acceptance walk read, and it
decides ADR-0046's open points.
- Why Fable: a wrong dedupe key, placement or `⏎` meaning is structural. The screen, `recover-op` and M8's journal all
  build on this list. The `recover` serde tag is written into journal files for good.
- Why not the diff or tests alone: git's reflog semantics (a deleted branch's log vanishes with it; a shell rebase
  writes a storm of `HEAD` records; a Graft operation writes reflog records too) show only in oracle scenarios run
  with real git.
Scout: none. It is greenfield modules. The builder reads graft-ops' MAP `journal`, graft-core's MAP `JournalEntry`, and
graft-git's `history::previous_tips`.
Needs: `graft_git::history_reads::{unreached, Unreached, LostRun}` (lost-commit-cost).

## Decided (the note; the builder may amend only the "open" points, in NOTES, with the oracle row that proves it)
- **Reflogs read:** `HEAD` and `refs/heads/*` only, through gix (no spawn). The newest `cap` records overall (default
  2 000). A missing log is empty, not an error.
- **Dedupe (ADR-0046 §2):**
  - An outside record whose `(refname, old, new)` equals a journal entry's before/after for that ref **is** that entry.
    It is not drawn as `$`.
  - A `HEAD` record belongs to a branch record with the same old and new values within **2 s** (open: the window,
    from the oracle). One `$` row, under the branch's words.
  - A shell rebase's `rebase (start|pick|continue|…)` `HEAD` records fold into its `rebase (finish)` record: one row,
    and `folded` counts them.
- **Words (`words`)** — the table in J1, exactly:

  | git's subject | Words |
  |---|---|
  | `commit: <s>` | `committed "<s>" on <b>` |
  | `commit (amend): <s>` | `amended "<s>" on <b>` |
  | `reset: moving to …` | `reset <b> <N> commits back` when `back` is `Some(N)`, else `moved <b> to <sha7>` |
  | `checkout: moving from a to b` | `switch from a to b` |
  | `merge …: Fast-forward`, `pull: Fast-forward` | `fast-forwarded <b> <N> commits` |
  | `branch: Created from …` | `made <b> at <sha7>` |
  | `rebase (finish): …` | `rebased <b> onto <upstream name if in the subject, else sha7>` |
  | `cherry-pick: <s>` | `took "<s>" onto <b>` |
  | anything else | git's subject, unchanged |

  `<b>` is the short branch name; `HEAD` records use the branch HEAD was on (from the paired record, or `HEAD`).
- **`back`** (open: how it's computed): the first-parent distance from old to new, when new is an ancestor of old
  within 100 commits. Otherwise `None`.
- **Lost commits:**
  - Candidates are every outside record's old value, and every journal entry's `refs_before` values, newest first, capped
    at 500 → `unreached`.
  - Each `LostRun` is placed **under the newest event (Graft entry or outside step) whose old value was that commit**.
  - `HeldBy`:
    - `Branch{name, gone}`: from the branch log's refname, or from the `checkout: moving from <name> to …` subject when
      the old value was HEAD's. `gone` = the ref doesn't exist now.
    - `Head` otherwise.
- **Lost edits:** `build` takes them as input. Detection (`lost_edits`) is recover-op's. Each is placed under its
  entry's row.
- **Order:** newest first by time. Lost rows sit immediately after their event, in candidate order.
- **Stages:** `build` is pure and is called three times by the app:
  1. `outside=[]`, `lost=None`, `scan=Running`;
  2. with `outside`;
  3. with `lost`, `scan=Done|Capped|Failed`.

  The rows of an earlier stage keep their relative order in a later one.
- **`recover_for(list, row, facts)`**, per row:
  - **Lost commit, branch holder:**
    - The branch exists, and its tip is still the event's new value → `Run(MoveRef{name, to: tip, expected: now})`.
    - The branch is gone → `Run(Recreate{name, at: tip})`.
    - The branch moved on since → `OfferBranch{at: tip, name: "<b>-restored", why: "<b> moved on since (now
      <sha7>)"}`. If `<b>-restored` exists, `restored`, then `<b>-restored-2`, and so on.
  - **Lost commit, HEAD holder:** `OfferBranch{at, name: "restored", …}`.
  - **Outside step (`$`):** for a ref move whose ref still sits at the step's new value → `Run(MoveRef{name, to: old,
    expected: new})`. A `checkout` step → the app's switch (no `Recover`; `Refusal` naming it). Moved since →
    `OfferBranch`.
  - **Lost edit:** `Run(Files{entry, backup_tree, paths})`.
  - **Graft row:** refused. The app keeps today's revert-to (`●`, ADR-0044 §2).
  - `because` is the words of the event: `the shell's reset`, `the shell's branch -D`, `undo of "<description>"`.
- **The new kind:** graft-core `OperationKind::Recover`, serde `recover` (hard choice 5). The conductor records it as
  an ADR-0046 amendment. Old journal lines still read.
- **Narrowed from the note:** `Recover::Run` carries `RecoverParams`, not `Operation`. `Operation::Recover` lands with
  its check/plan/touches in recover-op, so nothing here is a stub.

## Read first
- The note: pass 1b "Journal"; "Pass 2b" F1, J1, hard choice 5, "Interfaces (2b)" (graft-git::reflog,
  graft-ops::journal_list, graft-ops::operation).
- `docs/adr/0046-journal-and-reflog-are-one-list.md`; `docs/adr/0044-…` §1–2.
- `docs/research/reflog.md` §2 (git's reflog behaviour), §4 takeaways 1–14.
- `docs/design/06-undo-journal-design.md` §6.1–6.4 (entries, `refs_before`/`refs_after`, backups).
- `crates/graft-core/MAP.md`: `JournalEntry`, `EntryId`, `OperationKind`.
- `crates/graft-ops/MAP.md`: `journal` (`Line`, `in_force`), `operation` (`Refusal`, `RepoFacts`).
- `crates/graft-git/MAP.md`: `history::previous_tips`, `ReflogError`; `history_reads::unreached` (lost-commit-cost's
  entry).

## Signatures (exact)
```rust
// graft-git::reflog (new)
pub struct ReflogRecord { pub refname: String, pub old: Option<Oid>, pub new: Option<Oid>, pub at: i64,
                          pub subject: String }                   // None = git's zero id
pub fn records(repo: &RepoHandle, cap: usize) -> Result<Vec<ReflogRecord>, ReflogError>;   // newest first
// graft-core
// OperationKind::Recover  (serde "recover")
// graft-ops::operation
pub enum RecoverTarget { MoveRef { name: String, to: Oid, expected: Oid }, Recreate { name: String, at: Oid },
                         Files { entry: EntryId, backup_tree: Oid, paths: Vec<PathBuf> } }
pub struct RecoverParams { pub target: RecoverTarget, pub because: String }
// graft-ops::journal_list (new)
pub struct OutsideRecord { pub record: ReflogRecord, pub back: Option<usize> }
pub struct OutsideStep { pub record: ReflogRecord, pub words: String, pub folded: usize }
pub enum HeldBy { Branch { name: String, gone: bool }, Head }
pub struct LostCommit { pub run: LostRun, pub subject: String, pub held_by: HeldBy }
pub struct LostEdit { pub entry: EntryId, pub backup_tree: Oid, pub paths: Vec<PathBuf> }
pub enum ListRow { Graft(EntryId), Outside(OutsideStep),
                   Lost { event: usize, commit: LostCommit }, LostEdit { event: usize, edit: LostEdit } }
pub enum LostScan { Running, Done, Capped { checked: usize }, Failed(String) }
pub struct JournalList { pub rows: Vec<ListRow>, pub scan: LostScan }
pub fn read_outside(repo: &RepoHandle, git: &Git, cap: usize, stop: &dyn Stop)
    -> Result<Vec<OutsideRecord>, FactsError>;
pub fn find_lost(repo: &RepoHandle, entries: &[JournalEntry], outside: &[OutsideRecord], cap: usize,
                 stop: &dyn Stop) -> Result<Unreached, FactsError>;
pub fn build(entries: &[JournalEntry], outside: &[OutsideRecord], lost: Option<&Unreached>,
             subject_of: &dyn Fn(Oid) -> Option<String>, edits: &[LostEdit], scan: LostScan) -> JournalList;
pub fn words(record: &ReflogRecord, back: Option<usize>) -> String;
pub enum Recover { Run(RecoverParams), OfferBranch { at: Oid, name: String, why: String } }
pub fn recover_for(list: &JournalList, row: usize, facts: &RepoFacts) -> Result<Recover, Refusal>;
// Refusal gains MovedOnSince { name: String, now: Oid } and HeldAgain { commit: Oid, by: String };
// if either name is taken, suffix it and say so in NOTES (02's NOTES item 1 had the same clash).
```
Use the repo and git handle types graft-ops' other functions take (`&Repo`/`&Git` in 2a's specs). The `event` index is
into `rows`.

## The fixture (graft-fixtures, "lost work"; journal-screen and the acceptance walk use it)
Scripted with real git, and a journal written by Graft's own ops where a `●` row is needed:
1. `main` with commits A–D. A Graft `fetch` entry (`●`).
2. In the shell: `git reset --hard HEAD~1` on main. D ("Add rate limiter") is lost.
3. In the shell: `git switch -c spike`, commit E ("Try redis cache"), `git switch main`, `git branch -D spike`. E is
   lost; spike is gone.
4. A Graft undo over drift that replaced an edit to `src/a.rs`: a lost edit (the M6/77 shape).
5. A stash entry holding commit S, which is not lost.

Times are fixed, 10:12–10:44, as the note's mock-up.

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Pure `build`** (`journal_list/tests.rs`):
  - `a_graft_op_is_one_row_not_two` (red first: its reflog records would otherwise draw a `$` twin).
  - `head_and_branch_records_pair_within_the_window`.
  - `a_shell_rebase_folds_into_one_row`.
  - `a_lost_run_sits_under_the_event_that_lost_it`.
  - `lost_ancestors_are_one_row_with_under`.
  - `rows_are_newest_first_and_lost_rows_follow_their_event`.
  - `a_lost_edit_sits_under_its_undo`.
  - `later_stages_keep_earlier_rows_in_order`.
  - `scan_state_is_carried`.
- [ ] **`words`:** one test per table row, plus `an_unknown_subject_is_gits_own`.
- [ ] **Oracle** (real git, `tests/it/journal_list_oracle.rs`), each asserting the rows' kinds, words and placement:
  - `shell_reset_hard_shows_the_lost_commit_under_it`.
  - `a_deleted_branch_once_checked_out_is_found_from_head`.
  - `a_branch_deleted_and_never_checked_out_leaves_nothing`: documents takeaway 12. It asserts no row.
  - `a_graft_reset_is_one_graft_row`.
  - `a_shell_commit_is_an_outside_row`.
  - `a_shell_fetch_is_no_row` (remote logs aren't read).
  - `a_stash_held_commit_is_not_lost`.
  - `a_graft_backup_ref_does_not_hold`.
- [ ] **`recover_for`:**
  - `checked_out_main_is_moved_back`.
  - `a_gone_branch_is_recreated`.
  - `moved_on_offers_main_restored`.
  - `an_existing_restored_name_gets_a_suffix`.
  - `a_head_only_commit_offers_restored`.
  - `a_graft_row_is_refused`.
  - `a_lost_edit_recovers_its_files`.
- [ ] **Format:** `a_recover_entry_round_trips`, and the existing journal golden tests stay green (old lines read).
- [ ] **`records`:** `records_reads_head_and_branches_newest_first`, `a_missing_log_is_empty`, `records_caps`.
- [ ] MAP.md: graft-git `reflog`; graft-ops `journal_list` and the new `operation` types; graft-core
      `OperationKind::Recover`; graft-fixtures the shape. NOTES: each open point as decided, with the oracle row.

## Touches (check each, report how)
- **Journal file format (graft-core):** a new serde tag. Check: the round trip, plus the existing goldens.
- **`OperationKind` matches in graft** (02's NOTES: `update/preview.rs:262`, `update/journal.rs:279`,
  `view/journal.rs:125` have `_` arms). Check: the workspace builds; grep for new sites since 02.
- **`Refusal` matches in graft** (`update/operate.rs:1097` has `other =>`). Check: the build.
- **`history::previous_tips`:** reused or left alone. Check: its tests stay green.
- **UI, keymap, captures:** none here (journal-screen).

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Oracle experiments go
in a script file run with `sh` (02's gotcha: inline git in temp dirs is refused in a worktree-isolated agent).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~80k. If it compacts, split:
- **journal-list-a:** types, `records`, `words`, `build` (pure), and their tests. The signatures land here.
- **journal-list-b:** `read_outside`, `find_lost`, `recover_for`, the fixture, the oracle, and `OperationKind::Recover`
  with its round trip.
