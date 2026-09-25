# 84 — review fix for 74 (Major M1, M2, M3, M4; Minor m1, m2, m3, m5, m6, m7, m8)

Status: draft
Blocked by: 75 (graft tests, scripts, docs). 84 runs after 75 lands: it rewords surfaces 75 walks and captures
(Touches, last rows), so 75's command is re-walked after 84.
Crates: graft-ops, graft-ui, graft (and `docs/shots/m6`, `TESTING.md` rows 75 wrote, the m6 captions TSV)
Source: `docs/reviews/m6-final-fixes-74.md`. Every finding names its lines; re-check each against main after 75.
Build with: Opus — M1 moves one fact (where the edits are) from an app-side guess into graft-ops, where it is known;
M2 is one read added to `plan` with a real-repo table. Both are judged by tests that read the tree, the index, the
refs and the stash list, so not Fable. The conductor may revise after the scout.
Scout: `scout` first (ADR-0035, brownfield), ≤ 1 page, answering only the four questions under "Scout brief".
Needs: 73, 74, 82, 83 (merged); 75 merged.

## Decided (planner, 2026-09-26) — not open here
- **M1** (ADR-0045 §5 amended): graft-ops decides where the edits are and says so as `EditsAt` on the error; the app
  reads no stash list to guess. `edits_kept_in` and the job's second list read go. Words: the ADR's table; the
  operation's own failure first, the clause kept whole.
- **M2** (ADR-0045 §8 new): `plan` forecasts the put-back when `set_aside` is `Some`: `stash create` on a temp index
  copy, `landed` (checkout: target; rebase: one `merge-tree` of `HEAD` and upstream), then `merge-tree --merge-base=HEAD
  <landed> <S'>`. `PreviewPlan::put_back`. The conflict line folds it (design note amendment table).
- **M3** (ADR-0043 §4 amended): a set-aside clash's panes are `branch · <HEAD's name>` | `yours · edits set aside`,
  bar `o branch  t yours`; keys stay positional. The stash pop keeps `yours · main` | `stashed · stash@{n}`. The close
  reads `every file done · set-aside copy dropped ✓ · u brings it back` (ADR-0045 §3 amended); the drop still runs when
  every block took the branch side (`u` brings the copy back).
- **M4:** `is_set_aside_entry`, `is_set_aside_clash`, `has_edits_aside`; `Status::Done { what, also, keys }`; one
  rendering `<what> ✓ · <also> · <keys>` (each part only when present).
- **m3** is closed by M3 + M4: both put-back ends are `<what> ✓ · <after>`.
- **m4** is the conductor's (gap row). m6 goes with M1.

## Read first
- The review: M1–M4, m1, m2, m3, m5–m8, and "Landings".
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` "Amendment 2026-09-26" (whole), §4, §5.
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §4 (table and its 2026-09-26 amendment).
- `docs/design-notes/m6-stash-put-back.md` "Amendment 2026-09-26" (whole) and the `[A]` lines.
- `docs/design-notes/m5-history-and-ghost-preview.md` :164-169 (forecast is per commit, `may`), :217-218
  (`predicting…`, `NotChecked` words).
- `crates/graft-ops/MAP.md` `## preview` (:216-230), `ApplyError` (:185), `apply` set-aside (:179), `index_state`
  (:133).
- `crates/graft-ui/MAP.md` :12 (`SideRoles`), :23 (resolver `roles`, `short_form`), :35-36 (`set_aside_row`,
  `fitting_line`).
- `crates/graft/MAP.md` :246 (`edits_in_status`), :468-472 (`stash_name`, `dropped_status`, `edits_back`,
  `set_aside_entry`/`set_aside`), :716 (`resolver_context`), :719 (`done_line`).
- 74's NOTES (walks, captures, "Findings"); 75's NOTES (captures and TESTING.md rows it wrote).

## Scout brief (code, not intent; ≤ 1 page)
1. The rebase replay forecast in `graft-ops/src/preview/kinds.rs` (:166) and the checkout plan (:351-376): how
   `merge-tree` is spawned today (flags, `--merge-base`, how git's version is gated), and where a new forecast slots.
   Does `merge-tree --merge-base` accept a **tree** for `<branch1>` on the minimum git the forecast gates on, or must
   `landed` be a commit (`commit-tree`)?
2. `git stash create` under `GIT_INDEX_FILE=<copy>`: how `snapshot::index_state` makes its temp copy (reuse it).
3. Every constructor and reader of `Status::Done` (`grep -rn "Status::Done" crates/graft/src`), and what
   `update/undo.rs:229` does to `keys`.
4. The error variants a set-aside can end in on both paths (`apply` → `update/operate.rs:987-990`; `resume` →
   `update/banner.rs:746-750`), and whether 83's `Journal { kept }` / `AfterState::SetAsideKept { words }` reach
   `edits_in_status`.

## Signatures
```rust
// graft-ops, apply/error.rs — M1 (ADR-0045 §5 amended). Re-exported from the crate root.
/// Where a set-aside's edits are when the entry did not end in a clean put-back or a clash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditsAt {
    /// Not in the tree; the copy is listed at `n`. `S p` puts them back.
    Stash { n: usize },
    /// Back in the tree; a copy stays listed at `n`, or was not found to drop (`None`).
    Back { copy: Option<usize> },
    /// Stored, then the reset (ADR §4 step 4) failed or stopped: the entry's backup has them.
    Backup { copy: Option<usize> },
}
impl ApplyError {
    /// `Some` only for a set-aside that ended other than clean or `Clashed`. The one place the app asks.
    #[must_use]
    pub fn edits_at(&self) -> Option<EditsAt>;
}
// Each variant the scout's Q4 lists gains `edits: Option<EditsAt>`, set from `Kept` where `back` is known
// (`apply/set_aside.rs:288`, `:444-445`) and from `abandon` (`:122-163`). No other variant changes.

// graft-ops, preview — M2 (ADR-0045 §8)
pub struct PreviewPlan { /* … */ pub put_back: Option<ConflictForecast> } // Some exactly when `set_aside` is Some
// private: fn forecast_put_back(/* repo, git, op, facts, stop, replay: &ConflictForecast */)
//   -> Result<ConflictForecast, PreviewError>;   // Err only for Stopped; a failed spawn is NotChecked(reason)

// graft-ui, conflict_resolver/names.rs — M3
// SideRoles::SetAside: ours() == "branch", theirs() == "yours"

// graft — M1
// jobs/operate.rs: `Operated::Failed { edits_in: Option<usize> }` → `{ edits: Option<graft_ops::EditsAt> }`,
// from `error.edits_at()`; `edits_kept_in` and its stash-list read removed.
pub(crate) fn edits_status(failure: &str, at: graft_ops::EditsAt) -> Status; // replaces `edits_in_status`
// graft — M2 (update/preview.rs, pure)
pub(crate) fn conflict_line(op: &graft_ops::ConflictForecast,
                            put_back: Option<&graft_ops::ConflictForecast>) -> String;
// graft — M4 (update/state.rs, view/status.rs)
// Status::Done { what, also, keys } — `also` is `Option<_>` of `what`'s type.
pub(crate) fn done_line(what: &str, also: Option<&str>, keys: &str) -> String; // `<what> ✓[ · <also>][ · <keys>]`
pub(crate) fn is_set_aside_entry(entry: &JournalEntry) -> bool;   // was set_aside_entry
pub(crate) fn is_set_aside_clash(state: &AppState) -> bool;       // was set_aside
pub(crate) fn has_edits_aside(state: &AppState) -> bool;          // was edits_aside
```

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **M1, graft-ops** (`tests/it/set_aside.rs`), each asserting `edits_at()` **and** reading the tree and
      `git stash list`: `a_drop_that_fails_after_a_clean_put_back_is_edits_back` (`Back { copy: Some(0) }`; edits in
      the tree; S listed; red first), `a_stop_after_the_store_is_edits_in_the_backup` (`Backup { copy: Some(0) }`),
      `an_unfound_copy_after_a_failed_operation_is_edits_back` (the `Kept::Unlisted` arm; builder names the fault it
      injects), `a_put_back_that_fails_without_a_clash_is_edits_in_stash` (`Stash { n: 0 }`; tree lacks the edits).
- [ ] **M1, app:** `operate::tests::edits_words::each_place_the_edits_can_be_is_worded` (the ADR's four rows, each
      `<failure> · <clause>`; `S p` only in `Stash`; red first on `Back`),
      `operate::tests::edits_words::the_clause_is_kept_whole_at_80` (failure cut with `…`, clause whole),
      `banner::tests::resume_words::a_continue_whose_drop_fails_says_the_edits_are_back` (the `resume` path). NOTES:
      `grep -n "edits_kept_in\|stash list\|list_stashes" crates/graft/src/jobs/operate.rs` → empty.
- [ ] **M2, graft-ops** (real repos, builder names the file): `a_switch_whose_put_back_clashes_forecasts_the_clash`
      (the fixture `set_aside.rs` :288-301 uses → `Likely { paths: [base.txt] }`; red first),
      `a_switch_whose_put_back_is_clean_forecasts_clean`, `a_rebase_whose_put_back_clashes_forecasts_the_clash`,
      `a_rebase_that_may_stop_leaves_the_put_back_not_checked`, `a_clean_tree_has_no_put_back_forecast` (`None`),
      `the_put_back_forecast_writes_no_ref_index_or_file` (`.git/index` bytes, `for-each-ref`, `git stash list`,
      `git status --porcelain=v2` equal before and after `plan`), `the_put_back_forecast_honours_stop`.
- [ ] **M2, app:** `update::preview::tests::the_conflict_line_folds_the_put_back` (the note's four rows, red first on
      `Likely`); walk `set_aside::the_preview_warns_before_a_switch_that_clashes` (`. ⏎` on the clash fixture: the box
      shows `your edits may clash when put back: base.txt` and not `no conflicts expected`).
- [ ] **M2 capture** (rule 6; colour **unverified**, rule 5): `docs/shots/m6/set-aside-clash-preview-120x40.txt`,
      written by that walk through `check_shot`. **What right looks like:** the checkout preview box with
      `[ Confirm ]`; under `working tree:` the row `your edits: 1 file set aside, put back after`; the conflict line
      `your edits may clash when put back: base.txt`; no `no conflicts expected` anywhere in the box.
- [ ] **M3:** graft-ui `conflict_resolver::names::tests::set_aside_roles_name_the_panes_branch_and_yours` (replaces
      `…_yours_and_set_aside`; red first); graft `update::clash::tests::a_put_back_clash_lands_on_the_resolver_with_set_aside_names`
      (`branch · main` | `yours · edits set aside`), `update::clash::tests::a_put_back_clash_done_names_the_dropped_copy`
      (renamed from `…_says_your_edits_are_back`; `every file done · set-aside copy dropped ✓ · u brings it back`);
      `every_key_the_bar_draws_is_answered` and `every_planned_hint_is_still_unbound` green with `o branch`,
      `t yours`; NOTES say which rung `short_form` gives `branch` at 80 (ADR-0043 §4 amended: no new row).
- [ ] **M3 walks** (`tests/it/m6_stash_put_back/set_aside.rs`): new `t_in_a_set_aside_clash_keeps_your_edits`
      (`. ⏎ y t ⏎`, ≤ `noted_keys("switch-dirty-clash")`: on the branch; file holds his edit; stash list empty;
      status as above); `a_dirty_switch_that_clashes_is_five_keys` keeps `o` and gains: after `⏎`, `u` → `git stash
      list` shows the copy again (what `u brings it back` promises); its assertion messages say `branch`. Pop walks
      and `stash-clash-landing-*`, `stash-clash-done-120x40` unchanged (`check_shot`).
- [ ] **M3 capture:** `docs/shots/m6/set-aside-clash-landing-120x40.txt` regenerated. **What right looks like:**
      pane or card names `branch · main` and `yours · edits set aside`; notice `your edits clashed in 1 file · esc, u
      undoes the switch too`; bar `o branch` `t yours`; the word `set aside ·` nowhere as a pane name.
- [ ] **M4:** the three renames (`grep -rn "fn set_aside_entry\|fn set_aside(\|fn edits_aside" crates/graft/src` →
      empty, in NOTES); `view::status::tests::done_line_places_also_before_keys` (four shapes: what; what+also;
      what+keys; all three); `update::clash::tests::a_clean_put_back_says_your_edits_are_back` unchanged in words
      (the clause now in `also`); `set-aside-done-120x40.txt` byte-identical (`check_shot`). NOTES: what
      `update/undo.rs:229` does with `also`, and the test that pins it.
- [ ] **m1:** `set_aside::esc_u_undoes_a_clashed_switch` — `. ⏎ y` into the clash, then `esc u` (2 keys, no take so no
      drift box): old branch, index and tree as before (staged split included), stash list as before.
- [ ] **m2:** `set_aside::aborting_a_stopped_rebase_puts_the_edits_back` — `p r r` to the stop, then abort (keys as
      the banner takes them; NOTES say which): original tip; edits in the tree; stash list empty; status
      `aborted: rebase … ✓ · your edits are back`.
- [ ] **m5:** the 80×24 preview test (`set_aside.rs` :544) asserts the exact wording `fitting_line` picks at 80, not
      `contains`. Builder names the test and the rung.
- [ ] **m7:** tests 74 added inline to `update/operate.rs` and `update/undo.rs`, and every test this ticket adds
      there, live in `update/operate/tests/` and `update/undo/tests/`. NOTES: `wc -l` of both files before and after;
      the nextest count unchanged by the move alone.
- [ ] **m8:** `kept_open`'s `Clashed` arm (`update/banner.rs` ~:910) keeps its code; its comment says why it is
      unreachable (`finish::refusal` → `None` for `Clashed`, `update/finish/answers.rs:175`; a set-aside never wraps
      a merge). Doc, no test.
- [ ] Contact sheet rebuilt (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`), captions TSV row for the
      new capture; each part ≤ 200 000 B; no machine paths.
- [ ] MAP.md: graft-ops (`EditsAt`, `edits_at`, `PreviewPlan::put_back`, the forecast); graft-ui (`SideRoles::SetAside`
      words); graft (`edits_status`, `conflict_line`, `Done::also`, `done_line`, renames, `dropped_status`'s set-aside
      words). Doc, no test.
- [ ] TESTING.md: the rows 75 wrote for `switch-dirty-clash`, `pull-rebase-stop-then-clash`, the set-aside clash
      landing capture and the put-back failure line, reworded to the words above; a row for the new capture. NOTES
      list each row changed.

## Touches (check each, report how)
- **Every `ApplyError` match** (app: `operate::operation_failed`, `banner::resumed`, `undo`; graft-ops tests): the new
  field. Check: the build; `grep -rn "ApplyError::Git {" crates` in NOTES.
- **83's paths** (`Journal { kept }`, `AfterState::SetAsideKept`): unchanged unless the scout finds they reach
  `edits_in_status`. Check: 83's tests green.
- **The job pool:** one fewer git read per failed set-aside (`jobs/operate.rs`). Check: the build; M1 app tests.
- **`plan` for every other kind and for a clean tree:** no new spawn (`put_back == None`). Check:
  `a_clean_tree_has_no_put_back_forecast`; every existing preview test and capture green (`check_shot`).
- **The preview's `predicting…` window and perf:** 2–3 spawns on the preview job, only with edits aside. Check: the
  stop test; release perf is the review's (ADR-0021), not the builder's.
- **Preview captures** of a rebase or checkout with a clean put-back (`set-aside-preview-120x40`, `-80x24`): the line
  stays `no conflicts expected`. Check: `check_shot` unchanged; if one changes, NOTES say why.
- **Resolver hint bar and `resolver_context`:** `o branch  t yours` at 80 and 120; the rebase's `o upstream  t yours`
  and the pop's `o yours  t stashed` unchanged. Check: bar tests; `every_key_the_bar_draws_is_answered`.
- **`keys/bound.rs:102`** (`t set aside`): replaced by the new role words. Check: `every_planned_hint_is_still_unbound`.
- **`dropped_status(index, why)`:** Pop words unchanged; SetAside words change. Check: 72b's pop close tests green.
- **Every `Status::Done` reader** (graph footer, stash overlay, resolver notice row, undo status): output unchanged
  except the set-aside close. Check: the full nextest, every `check_shot` green; `git status docs/shots` lists only
  the two captures named above plus the sheet parts.
- **Journeys:** counts unchanged. Check: `journey_counts_match_the_note` and the six set-aside walks green.
- **README.md** :133-134 (74 reworded it): re-read for `set aside ·` / `your edits are back ✓`. Check: grep in NOTES.
- **`docs/design/`:** no edit (keymap §2.6 `Enter` already says "drops the stash … `u` brings it back"; roles are not
  in the keymap). `docs/design/01` §1.5 mockups show merge names only. Check: `grep -rn "set aside" docs/design`.
- **75's walks and captures** (re-walk after 84 lands):
  - `m5_walk::every_scenario::journey` `switch-dirty-clash` and `pull-rebase-stop-then-clash`: counts unchanged; any
    assertion on pane names, bar or end status changes to the words above.
  - `m6-23-set-aside-clash-landing` (120x40, 187x50, 80x24): changes (names, bar).
  - `m6-22-set-aside-preview`: expected unchanged (clean put-back fixture); re-walk confirms.
  - `m6-24-set-aside-done`: expected byte-identical (`also` renders as `keys` did).
  - `m6-20`, `m6-21`, `m6-25`: unchanged (pop, menu).
  - `edits_survive_a_kill_at_every_step`: a stop before step 4 now words `u restores the backup · a copy is in
    stash@{n}` if the walk reads the status line there; the relaunch line is unchanged.
  - TESTING.md "Conflict journeys" rows for the two journeys and "Captures" row for m6-23; the contact sheet.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Large (~80k). If it compacts, the split is: **84a** M1 + M2 (graft-ops and their app words, the preview capture) /
**84b** M3, M4, m1, m2, m3, m5, m7, m8 (names, `Done`, walks, the landing capture).

## NOTES (builder)
