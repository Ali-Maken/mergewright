# journal-list — the journal and git's reflog as one list: outside rows, lost-commit rows, what `⏎` puts back

Status: done (the fixture by 22b)
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
- [x] **Pure `build`** (`journal_list/tests/build.rs`; all 9 named below pass; red first for `a_graft_op_is_one_row_not_two`
  shown by stubbing `match_entries`, then restored. Also `a_deleted_branch_is_named_from_heads_switch`,
  `a_record_that_moved_nothing_is_no_row`, `a_graft_switch_is_one_row`):
  - `a_graft_op_is_one_row_not_two` (red first: its reflog records would otherwise draw a `$` twin).
  - `head_and_branch_records_pair_within_the_window`.
  - `a_shell_rebase_folds_into_one_row`.
  - `a_lost_run_sits_under_the_event_that_lost_it`.
  - `lost_ancestors_are_one_row_with_under`.
  - `rows_are_newest_first_and_lost_rows_follow_their_event`.
  - `a_lost_edit_sits_under_its_undo`.
  - `later_stages_keep_earlier_rows_in_order`.
  - `scan_state_is_carried`.
- [x] **`words`:** one test per table row, plus `an_unknown_subject_is_gits_own` (`journal_list/tests/words.rs`:
  `a_commit_is_committed_on_its_branch`, `an_amend_is_amended_on_its_branch`, `a_reset_back_along_the_branch_counts_commits`,
  `a_reset_elsewhere_names_the_target`, `a_checkout_is_a_switch_between_names`, `a_fast_forward_counts_commits`,
  `a_new_branch_is_made_at_its_commit`, `a_rebase_finish_names_its_upstream`, `a_cherry_pick_is_taken_onto_the_branch`,
  `an_unknown_subject_is_gits_own`).
- [x] **Oracle** (all 8 named below pass on git 2.54.0, plus `a_graft_switch_is_one_graft_row`; real git, `tests/it/journal_list_oracle.rs`), each asserting the rows' kinds, words and placement:
  - `shell_reset_hard_shows_the_lost_commit_under_it`.
  - `a_deleted_branch_once_checked_out_is_found_from_head`.
  - `a_branch_deleted_and_never_checked_out_leaves_nothing`: documents takeaway 12. It asserts no row.
  - `a_graft_reset_is_one_graft_row`.
  - `a_shell_commit_is_an_outside_row`.
  - `a_shell_fetch_is_no_row` (remote logs aren't read).
  - `a_stash_held_commit_is_not_lost`.
  - `a_graft_backup_ref_does_not_hold`.
- [x] **`recover_for`** (`journal_list/tests/recover.rs`; all 7 named below pass, plus `a_commit_held_again_is_refused`,
  `under_a_graft_row_main_is_moved_back_from_its_after_value`):
  - `checked_out_main_is_moved_back`.
  - `a_gone_branch_is_recreated`.
  - `moved_on_offers_main_restored`.
  - `an_existing_restored_name_gets_a_suffix`.
  - `a_head_only_commit_offers_restored`.
  - `a_graft_row_is_refused`.
  - `a_lost_edit_recovers_its_files`.
- [x] **Format:** `a_recover_entry_round_trips` (graft-core `journal.rs`), and the existing journal golden tests stay green
      (workspace nextest 3718/3718).
- [x] **`records`:** `records_reads_head_and_branches_newest_first`, `a_missing_log_is_empty`, `records_caps`
      (graft-git `reflog/tests.rs`; also `records_reads_what_git_wrote`, `a_remote_tracking_log_is_not_read`,
      `first_parent_distance_counts_back_along_the_first_parent`).
- [x] MAP.md: graft-git `reflog` [done]; graft-ops `journal_list` and the new `operation` types [done]; graft-core
      `OperationKind::Recover` [done]; graft-fixtures the shape [done, 22b: `lost_work` line]. NOTES: each open point as decided, with the oracle row.
- [x] **The fixture** (22b): graft-fixtures `lost_work` (`src/lost_work/tests.rs`: `graft_is_called_at_its_three_points_in_order`,
      `d_and_e_are_held_by_nothing_and_s_by_the_stash`, `heads_log_is_the_shells_steps_at_fixed_times`,
      `the_ids_are_the_same_every_build`, `start_is_10_12_and_end_is_10_44_utc`); with Graft's real ops, graft-ops
      `tests/it/lost_work.rs`: `the_lost_work_fixture_draws_every_row_in_place` (every row: `●` fetch, `●` undo with its
      `✗` edit, `●` discard, `$` reset with `✗` D, `$` switch with `✗` E spike gone, …),
      `the_lost_rows_are_the_fixtures_commits_and_his_edit` (tips D, E; S in the stash, not lost; the edit's backup holds
      his bytes), `every_row_is_timed_between_10_12_and_10_44`.

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

## NOTES (m7-power-tools-22 builder)
**Compacted once mid-ticket.** Stopped at the seam after it. **Remains:** the graft-fixtures "lost work" shape (section
"The fixture"), its graft-ops test, and its MAP line. Everything else in both halves (a and b) is built and green.
Plan for the remainder: graft-fixtures depends on no Graft crate, so the fixture builds the git side (A–D; shell reset
losing D "Add rate limiter"; spike/E "Try redis cache" deleted; a stash holding S; 10:12–10:44 through
`GIT_COMMITTER_DATE`/`GIT_AUTHOR_DATE`) and takes callbacks at the two Graft points (the `fetch` entry; the undo over drift
replacing an edit to `src/a.rs`). A graft-ops test supplies them with `support::run`/`run_undo` and asserts the list.
`tests/it/journal_list_oracle.rs`'s `git_at` and `lines` helpers are the pattern.

No scout: greenfield modules, per the conductor. Sites read myself: `OperationKind` (graft-core `journal.rs`, serde
snake_case; `Recover` added before `RevertTo`); `kind_name` (graft-ops `apply/record.rs`, exhaustive; arm added);
`Refusal`/`explain` (`operation.rs`; `MovedOnSince`, `HeldAgain`, `NoPutBack` were free, so no suffix); `FactsError`
(`facts.rs`; `Reflog` added, reusing graft-git's existing `history::ReflogError`); `RepoFacts.refs` (full name →
`Option<Oid>`, what `recover_for` reads). `graft` builds unchanged: its `_`/`other =>` arms absorb the new kind and
refusals. journal-screen must give them words there.

**Signature deviations from the spec:**
- `read_outside(repo, cap, stop)` has no `git: &Git`: gix reads the logs, nothing spawns, and graft-ops has no such type.
- `build` gains `branch_exists: &dyn Fn(&str) -> bool` after `subject_of`: `HeldBy::Branch.gone` is what the ref store
  says now, not a guess from the logs.
- `recover_for(list, row, entries, facts)` gains `entries`: a lost row under a `●` row needs its entry's after-value
  (`expected`) and description (`because`).
- Added `recover_scope(list, row) -> FactsScope`: the refs `recover_for` reads (the branch, `<b>-restored`, `-2`…`-9`).
- Added `Recover` (`Run(RecoverParams)` / `OfferBranch { at, name, why }`), `NoPutBack` and `Refusal::NoPutBack`. The spec
  had no way to say "offer a new branch" or "this row has nothing to put back". No `Operation::Recover` yet: that is
  journal-screen's.

**Open points decided (oracle: git 2.54.0, scripts run with `sh`):**
- Pair window **2 s**. Every HEAD/branch twin (reset, commit, `update-ref` on the checked-out branch, rebase finish) was in
  the same second; the extra second covers the clock ticking over. Test: `head_and_branch_records_pair_within_the_window`.
- `back`: first-parent distance ≤ 100 through gix (reset: new back from old; fast-forward: old back from new). Otherwise
  the words name the target. Oracle: `shell_reset_hard_shows_the_lost_commit_under_it` ("reset main 1 commit back").
- A record with old == new is dropped unless it is `checkout:`. `git stash push` writes HEAD `reset: moving to HEAD`, and
  so does a rebase finish's HEAD record. Tests: `a_record_that_moved_nothing_is_no_row`; oracle
  `a_stash_held_commit_is_not_lost`.
- A shell rebase folds into the branch's finish record. Its HEAD run (`rebase (start|pick|finish)`) and the branch's
  `rebase (finish): … onto <sha>` all fall in one second. `onto` is named from `rebase (start): checkout X`. Test:
  `a_shell_rebase_folds_into_one_row`.
- HEAD's branch is inferred from the logs (`checkout: moving from a to b`, twins, rebase finish). An unpaired HEAD commit
  reads `committed "E" on spike`, and a deleted branch is named from HEAD's switch. Oracle:
  `a_deleted_branch_once_checked_out_is_found_from_head`.
- A branch deleted without ever being checked out leaves nothing, because its only log goes with it. Oracle:
  `a_branch_deleted_and_never_checked_out_leaves_nothing`.
- Lost candidates come from `HEAD` and `refs/heads/*` only, not `refs/stash` or tags. A dropped stash is `u`'s to bring
  back.
- A Graft checkout entry carries no ref oids, so its `checkout:` record is matched by names within 60 s. Tests:
  `a_graft_switch_is_one_row`; oracle `a_graft_switch_is_one_graft_row`. Every other entry claims, per changed ref, the
  nearest-in-time unclaimed step with that (old, new).
- Graft's `refs/graft/**` backups never hold a commit. Oracle: `a_graft_backup_ref_does_not_hold`.

**Touches:** journal format checked by `a_recover_entry_round_trips` and the existing goldens. The graft
`OperationKind`/`Refusal` matches checked by workspace clippy and nextest. `history::previous_tips` left alone; its tests
are green. No UI, keymap or captures.

## NOTES (m7-power-tools-22b builder)
No scout: 22's builder left the plan; sites read myself (the oracle's helpers, `support.rs`, recover.rs's M6/77 shape,
`steps.rs::match_entries`). Not compacted.

**Fixture decisions (read against the build; each proved by a named test):**
- **The undo reverses a Graft discard, not a switch.** A Graft step that moves `HEAD` writes its reflog records on the
  wall clock (graft-ops passes no `GIT_COMMITTER_DATE`), and `match_entries` pairs a switch entry with its `checkout:`
  record within 60 s (`steps.rs:299-306`). With the fixed 10:12–10:44 journal times the records would stay unclaimed as
  `$` twins at the wall-clock top. A discard and its undo write no `HEAD`/`refs/heads` record. Same lost-edit shape
  (forced undo over his edit; `lost_edits` finds it). Test: `every_row_is_timed_between_10_12_and_10_44`.
- **Order follows the mock-up**, not the section's numbering: shell 10:12–10:42 (A–D, spike + E, switch back, delete,
  reset, stash), then Graft 10:43 discard, 10:43:40 undo, 10:44 fetch (the mock-up's newest row).
- **`spike` branches off C (`HEAD~1`), not D.** Off D, E's first parent is D, so `find_lost` folds D into E's run
  (`lost_ancestors_are_one_row_with_under`) and the list shows one `✗`, not the mock-up's two. Observed first (red run),
  then fixed.
- **The stash is made after the reset** (S's parent C). Made before, S's parent is D and the stash holds D.
- **`origin` is cloned bare at C**, before D, so no remote-tracking ref holds D; a teammate pushes two commits at
  10:30–10:32; Graft's fetch at 10:44 brings them.
- The graft-ops test reuses `journal_list_oracle.rs`'s `lines` and `branch_exists` (now `pub(crate)`).

**Question (not a defect, no line claims it wrong):** the initial commit reads `$ commit (initial): Start the api`,
git's own words: the J1 table has no row for `commit (initial):`, so `words` falls back (`an_unknown_subject_is_gits_own`).
The test asserts it as observed. journal-screen may want `committed "Start the api" on main`; that is a `words` table
change in graft-ops src, outside 22b.

**Touches:** graft-fixtures gains `pub mod lost_work` (new, nothing else changed; MAP line added). graft-ops tests only:
new `tests/it/lost_work.rs`, `main.rs` registers it, `journal_list_oracle.rs` two helpers made `pub(crate)` (its 9 tests
still green). No src, UI, keymap, captures or journal format. Checked by workspace clippy and nextest (3764/3764) and
`cargo test --doc`.
