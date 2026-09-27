# 30b put-back-words — the preview and the done line after "every put-back keeps the index"

Status: done
Build with: Opus. The words are decided here; the work is small but crosses the operate answer, the resume banner
and captures in two crates, and every match on `Operated` has to be judged, not pattern-filled.
Needs: 30's `graft_ops::apply::AfterState::StagingNotKept { paths: Vec<PathBuf> }` (on 30's branch) and
`SetAside.staged == 0` for every kind (30 Decided 5).
Base: **30's branch** (`worktree-agent-a424cf7145815a3bc`, head 900118bb), with main merged in by the conductor
before dispatch: the C1 test needs 31's `patch.rs` fix. Check it is there (`grep -n "deleted file mode"
crates/graft-git/src/diff/patch.rs`). Not there → leave the C1 box open, say so. 30 and 30b merge to main together.
Crates: graft, graft-ui. Plus **test-only** `crates/graft-ops/tests/it/absorb_staged.rs` (one test, the C1 carry).
Docs: the ones under Touches. Anything else → commit at the seam, `blocked: needs <crate>`, stop.
Scout: none needed. 30's NOTES "Blocked" and "Gaps for the conductor" list every site (grep-checked by the planner,
2026-09-27). Re-read each site before editing.

## Read first
- 30's ticket on its branch: "Decided" 1, 2, 5, and NOTES "Blocked", "Gaps for the conductor", "Touches".
- ADR-0045, "Amendment 2026-09-26 (b)" only.
- `docs/research/ux-principles.md` §2 rows 1, 5, 8, 9, and §3.5.
- graft-ui MAP.md: `set_aside_row` (:35). graft MAP.md: `Operated` and the operate answer, if listed.

## Decided (planner, 2026-09-27)
1. **Normal case: the preview says nothing about staging.** The row is
   `your edits: 3 files set aside, put back after`, cut to `your edits: 3 files set aside` when narrow.
   - The staging now comes back as he left it, as `git switch` carries it. That is what he expects, so it is not
     news: heuristic 8, "status only when there's news". A `· staging kept` clause would spend cells saying nothing.
   - `SetAside.staged` is always 0 (30), so there is no count to word anyway.
2. **No fallback warning in the preview.**
   - A "staging may not come back" clause would show on every preview where he has staged anything, and almost never
     come true. §3.5 (NN/g): "if you warn people too much, they stop paying attention".
   - Nothing is lost when it does happen. The done line names the files, and `u` restores the split exactly
     (ADR-0045 §6). §3.5: undo permits fewer warnings.
   - An **exact** forecast would meet heuristic 5 (show the result before confirming). It is feasible with git's own
     check: `git apply --cached --check` of the staged patch on a temporary index of the landed tree. That is how
     `stash apply --index` fails (30 P2/P3: `conflicts in index. Try without --index.`). But it belongs in graft-ops
     `preview/put_back.rs`, and it makes the read-only preview write a temp index. So it is **not in 30b**; it goes
     to the conductor as a gap (below).
3. **Fallback done line: the op finished, plus one piece of news. Not "could not confirm".**
   - Today it renders through `Operated::Incomplete` as
     `checkout fix — could not confirm it fully: your edits are back · staging not kept: …`
     (`crates/graft/src/update/operate.rs:845-848`, via `jobs/operate.rs:333-339`). That misreads a write that was
     read back in full (CLAUDE.md status rule).
   - New line, longest first, the first that fits the status line:
     1. `checkout fix ✓ · your edits are back · staging not kept: base.txt · u undoes it, staging included`
     2. `checkout fix ✓ · staging not kept: base.txt · u undoes it, staging included`
     3. `staging not kept: base.txt · u undoes it, staging included`
   - `checkout fix` is the entry's description, as `Done` uses it (a resume uses what `resumed_done` puts first).
     `✓ ·` is exactly as `Done` draws it. Paths are `<first>[ +N more]`, as graft-ops' Display. Rung 1's text after
     `✓ · ` equals `AfterState::StagingNotKept { paths }.to_string()` (pinned by test, so the two cannot drift).
   - The cut drops `your edits are back` first. The files and the recovery key are what he must read (heuristic 9:
     what, and the next key).
   - **Sticky**, not `Done`. `Done` clears on the next key (`update/keys.rs:134`), and this line is the only place
     that tells him. It stays until the next status replaces it.
   - Words are CONTEXT.md-clean: "staging" and "unstaged" are git's own words, which he uses. `u undoes it` matches
     `u brings it back` and `u undoes the switch too` elsewhere.
4. **Only `[StagingNotKept]` alone maps to the new outcome.** A drop that failed after a fallback
   (`SetAsideKept`, whose words end `· staging not kept: …`) really is unconfirmed, and stays `Incomplete` as today.
   So does any other `unread` set.

## Journeys (counts unchanged except the new one)
| Journey | From | Keys | Count | Best tool | Lands |
|---|---|---|---|---|---|
| `pull-rebase-dirty` | graph, diverged, 3 edits, `one.txt` staged | `p r r` | 3 | JetBrains 3–4 | preview row `your edits: 3 files set aside, put back after`; then `rebase main onto origin/main ✓ · your edits are back`; `git status` `M  one.txt`, ` M three.txt`, ` M two.txt` |
| `switch-dirty` | graph on `main`, `base.txt` staged, `notes.txt` edited; cursor on `fix` | `. ⏎ y` | 3 | lazygit 2, JetBrains 3 | on `fix`, `… ✓ · your edits are back`; `M  base.txt`, ` M notes.txt` |
| `switch-staging-not-kept` (new) | graph on `main`, 20-line `f.txt`, line 5 edited and staged; `fix` changed line 7; cursor on `fix` | `. ⏎ y` | 3 | shell: `git switch fix` refuses (30 P2's shape); by hand `stash`, `switch`, `stash pop` = 3 commands, and the staging is lost with no word | preview row `your edits: 1 file set aside, put back after`, no staging words; after `y`, on `fix`, rung 1 sticky; `git status` ` M f.txt` with lines 5 and 7 both in; stash list empty |
| …then `u` | the line above | `u` | 1 | — | back on `main`; `git status` `M  f.txt`, index and worktree byte-equal to before; stash list empty |

## Scope
**In:**
- `set_aside_row` loses `staged`, and the preview stops passing it.
- `Operated::StagingNotKept`, mapped in `answer`, handled at every `Operated` match, and the sticky ladder.
- The five red graft tests and the stale unit tests, changed as below.
- New app test and captures for the fallback. Docs and captures under Touches.
- The C1 carry test (graft-ops, test-only).

**Out:**
- A preview forecast of the fallback (Decided 2 → gap).
- Removing the `SetAside.staged` field (graft-ops). It is always 0 and graft stops reading it; its removal is a
  graft-ops follow-up (06d is the natural home).
- The staging lost on a clash (ADR-0045 Consequences :129).
- `Incomplete`'s wording for any other `AfterState`.

## Signatures
```rust
// crates/graft-ui/src/preview_box.rs — was (files: usize, staged: usize)
/// The set-aside row under `working tree:` (ADR-0045 §4, amended 2026-09-26 b): longest first,
/// `your edits: N files set aside, put back after`, then `your edits: N files set aside`. No staging
/// words: the put-back keeps the staging, as `git switch` does.
#[must_use]
pub fn set_aside_row(files: usize) -> Vec<String>;

// crates/graft/src/jobs/operate.rs
pub enum Operated {
    // …existing variants unchanged…
    /// The write ran and was read back, and the edits it set aside are back, but git could not
    /// reinstate the staging of `paths` (graft-ops `AfterState::StagingNotKept`). Everything a `Done`
    /// does (refresh, commit editor, landing), plus a sticky line naming the files.
    StagingNotKept { applied: Applied, paths: Vec<PathBuf> },
}

// crates/graft/src/update/operate.rs (beside `done_status`)
/// The fallback's status wordings, longest first (planner 30b, Decided 3). `what` is what `Done` would
/// put before its tick.
pub(crate) fn staging_not_kept_rungs(what: &str, paths: &[PathBuf]) -> Vec<String>;
```
- `answer`: `Err(ApplyError::AfterState { applied, unread })` whose `unread` is exactly
  `[AfterState::StagingNotKept { paths }]` → `Operated::StagingNotKept`. Anything else, as today.
- Pick the rung with the status line's own width fit if there is one. Otherwise use `graft_ui::preview_box::fitting_line`'s
  rule against the status line's text width. Say in NOTES which.

## Tests (red first where marked; each named at tick time)
Red today on 30's branch, changed to:
- `m6_stash_put_back::set_aside::a_dirty_pull_rebase_is_three_keys` (`set_aside.rs:744,753`): the row is
  `your edits: 3 files set aside, put back after` at both 120 and 80. `PREVIEW_HEADER` (:815-820) is reworded to match
  (no staged clause; the cut rule stays).
- `m5_walk` `rebase_preview_with_edits` (`stash_journeys.rs:520`): `Sees::Has("your edits: 3 files set aside, put
  back after")`, plus an absence check on `comes back unstaged` (the harness's own absence `Sees`).
  `three_edits_back` (:541-548): `["M  one.txt", " M three.txt", " M two.txt"]`. Its doc comment says the staging
  is kept, as `git switch` keeps it (ADR-0045 amendment b). This fixes `journey_pull_rebase_dirty`, `…_preview` and
  `…_rejected_push_dirty_to_pushed`.
- `journey_switch_dirty` (`stash_journeys.rs:775`): `["M  base.txt", " M notes.txt"]`. Doc comment at :754-755
  reworded to "the staging of `base.txt` kept".

Stale (green today, but asserting the old clause):
- graft-ui `set_aside_row_words` (`preview_box/tests.rs:422`): `set_aside_row(1)` / `(3)` rung 0 only. Drop the
  `staged` asserts.
- graft-ui `set_aside_row_cuts_the_staged_clause_first` (:444) → rename `set_aside_row_cuts_to_the_file_count`. Two
  rungs, each a prefix of the one before. `fitting_line` gives rung 0 at 80 and 60, and rung 1 at a width where
  rung 0 doesn't fit (40), and at 20.
- graft-ui `the_set_aside_row_draws_whole_at_80_and_120` (:470): `set_aside_row(3)`.
- graft `the_set_aside_row_sits_under_the_working_tree_line` (`update/preview/tests.rs:1106`): keep `staged: 1` in
  the hand-built plan, **on purpose**. The row is `…put back after` at 120, and `your edits: 3 files set aside` at a
  width that cuts it. A comment says why: graft never words `staged`.

New:
- `staging_not_kept_rungs_cut_the_edits_back_clause_first`: the three rungs of Decided 3, one path and three paths
  (`+2 more`).
- `staging_not_kept_words_match_graft_ops`: rung 1's tail equals `AfterState::StagingNotKept { paths }.to_string()`,
  for one and for three paths.
- `answer_maps_staging_not_kept_alone_to_its_own_outcome` (unit, `jobs/operate.rs`): `[StagingNotKept]` →
  `Operated::StagingNotKept`; `[SetAsideKept{…}]` and a two-item `unread` → `Incomplete`. Red first.
- `a_switch_whose_staging_cannot_come_back_says_so` (`tests/it/m6_stash_put_back/set_aside.rs`), the
  `switch-staging-not-kept` fixture (30 P3's shape):
  - `. ⏎ y` counted, 3 keys. The preview row is `your edits: 1 file set aside, put back after`, with no
    `staged`/`staging` words.
  - After `y`: the sticky line at 187 is rung 1, and at 80 rung 2 or 3, whole. It never says `could not confirm`.
  - After one more key (`j`), the line is still there.
  - Git: ` M f.txt`, lines 5 and 7 in, stash list empty.
  - Then `u`, 1 key: `M  f.txt`, index and worktree byte-equal to before, on `main`.
  - Red first (today's line says `could not confirm it fully`).
  - Captures `docs/shots/m7/30b-staging-not-kept-{187x50,80x24}.txt`, each with a `What right looks like:` header.
- `a_resume_whose_put_back_lost_staging_is_done_and_says_so` (unit, `update/banner` tests, as `finish_here.rs:309`
  builds `Product::Resumed`): the banner closes as on `Done`, the status is the sticky rung, and no
  `could not confirm`. Red first.
- **C1 carry:** `a_staged_file_deletion_drops_the_commit_that_made_it` in `crates/graft-ops/tests/it/absorb_staged.rs`.
  - Fixture: review 06b probe q8 (`docs/reviews/m7-06b-absorb-staged.md:100-103`). `f2` made all of `b.txt`; he
    stages `git rm b.txt`; absorb.
  - Assert: `Committed`; `f2` is dropped from history (the M3 drop); `ls-tree -r HEAD` has neither `b.txt` nor
    `dev/null`; `f3` is still there; status as before minus the placed deletion; `u` gives the state equal to before.
  - Red first only if you revert 31's `write_git_headers` for one uncommitted run. Otherwise say "not seen red; the
    red evidence is review q8".

## Acceptance (each tick names its test inline)
- [x] The five red graft tests green, each named, with the new expectation quoted:
  `m6_stash_put_back::set_aside::a_dirty_pull_rebase_is_three_keys` (row `your edits: 3 files set aside, put back
  after` at 120 and 80); `m5_walk::every_scenario::journey_pull_rebase_dirty`, `…_pull_rebase_dirty_preview`,
  `…_rejected_push_dirty_to_pushed` (`Sees::Has("your edits: 3 files set aside, put back after")`,
  `Sees::Lacks("comes back unstaged")`, `three_edits_back` = `["M  one.txt", " M three.txt", " M two.txt"]`);
  `m5_walk::every_scenario::journey_switch_dirty` (`["M  base.txt", " M notes.txt"]`).
- [x] The four stale unit tests changed as above: graft-ui `preview_box::tests::set_aside_row_words`,
  `set_aside_row_cuts_to_the_file_count` (renamed), `the_set_aside_row_draws_whole_at_80_and_120`; graft
  `update::preview::tests::the_set_aside_row_sits_under_the_working_tree_line` (`staged: 1` kept on purpose; 120 →
  `…put back after`, 40 → `your edits: 3 files set aside`).
- [x] The five new tests (plus two), each named, red runs quoted in NOTES:
  `update::operate::tests::staging_not_kept::staging_not_kept_rungs_cut_the_edits_back_clause_first`,
  `…::staging_not_kept_words_match_graft_ops`, `jobs::operate::tests::answer_maps_staging_not_kept_alone_to_its_own_outcome`
  (red), `m6_stash_put_back::set_aside::a_switch_whose_staging_cannot_come_back_says_so` (red),
  `update::banner::tests::resume_words::a_resume_whose_put_back_lost_staging_is_done_and_says_so` (red), C1
  `absorb_staged::a_staged_file_deletion_drops_the_commit_that_made_it` (red with 31 reverted). Extra:
  `…::staging_not_kept_status_picks_the_rung_that_fits`, `…::operated_takes_staging_not_kept_as_done_with_its_line`.
- [x] Captures `docs/shots/m7/30b-staging-not-kept-187x50.txt` (2.1 KB) and `-80x24.txt` (1.4 KB), each with a
  `What right looks like:` header, written and re-checked green by
  `a_switch_whose_staging_cannot_come_back_says_so`. Colour unverified (rule 5): a Sticky draws in the Warning
  token; not seen.
- [x] Changed m6 captures regenerated (`GRAFT_UPDATE_SHOTS=1`, then a green re-run), listed in NOTES "Captures":
  in-process by `a_dirty_pull_rebase_is_three_keys`; pty by `.scratch/m6-conflicts/pty-walk-m6.py <WxH>
  pull-rebase-dirty@preview pull-rebase-dirty` at 120x40, 80x24, 187x50 (second run without the env var: all six
  written/settled, exit 0).
- [x] 30's open boxes ticked in 30's file, with test names and "(closed in 30b)": set-aside filter 407/407; Gates;
  Carry C1.
- [x] Gates, workspace-wide: `cargo fmt --check` pass, `cargo clippy --all-targets -- -D warnings` pass,
  `cargo nextest run` 4051 passed / 4051 run (1 skipped), `cargo test --doc` pass.

## Touches (check each, say how)
- `set_aside_row` callers: `grep -rn set_aside_row crates` shows only `update/preview.rs:956` and the tests.
- **Every `Operated` match**; each must take the new variant as `Done` does, except for the status:
  - `update/mod.rs:545` (`staged`: it ran, so the resolver moves on);
  - `update/operate.rs:925` (`operated`, the `lands_on_head` landing as `Done`);
  - `update/banner.rs:722` and `:926`;
  - `update/finish/answers.rs:173` (`None`);
  - `update/operate/tests/lands_on_head.rs:135`.
  - Grep `Operated::` for any others.
- Other screens drawing the same state:
  - The checkout preview draws the same row (`switch-dirty`'s `. ⏎` preview). It is covered by the new app test's
    preview assert.
  - The journal browser (`J`) shows the entry, which has no staging words. Unchanged; say so.
- Keymap: no new key. `u` keeps its meaning (`docs/design/02-keymap.md`); nothing to edit.
- Journal/undo: graft only reads. Undo is 30's, pinned there (T6) and by the new app test's `u`.
- Captures:
  - regenerated by test: `docs/shots/m6/set-aside-preview-{120x40,80x24}.txt`,
    `m6/m6-22-set-aside-preview-{80x24,120x40,187x50}.txt`, and any `m6/*set-aside-done*` whose status pane changes
    (`one.txt` now staged);
  - the captions in `docs/shots/m6/m6-contact-sheet-captions.tsv:153,164` reworded;
  - the sheet reassembled with `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`.
- Docs:
  - `TESTING.md:2451` (`pull-rebase-dirty`: row words; `M  one.txt` "still staged, as `git switch` keeps it");
  - `:2454` (`switch-dirty`: `M  base.txt` "(the staging kept)");
  - `:2582` (`m6-22` row words);
  - a new row for `switch-staging-not-kept`, from the Journeys table;
  - `README.md:134-136` (drop the clause; add "if git can't keep your staging, the line names the files and `u`
    restores it");
  - `docs/design-notes/m6-stash-put-back.md:33,118`: append `(superseded 2026-09-26: ADR-0045 amendment b, tickets
    30/30b)`;
  - graft-ui MAP.md :35;
  - graft MAP.md, the `Operated` entry if it has one.

## For the conductor (not the builder)
- Close KNOWN_GAPS `M7/30words` on merge.
- Open `M7/30forecast` (**unverified**, M8): "the preview does not forecast that the staging can't come back.
  Exact: `git apply --cached --check` of the staged patch on a temp index of the landed tree, in graft-ops
  `preview/put_back.rs`. Needs a ruling on a temp index in a read-only preview."
- The `SetAside.staged` field removal → 06d (graft-ops).

## Gates
Workspace-wide, quiet: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`.

## Size
About 40k: the row and its tests about 6k, the new outcome and its match sites about 12k, the app test, fixture and
captures about 14k, the C1 test about 4k, docs about 4k.

## NOTES (builder)

Scout: none, as the ticket says (30's NOTES list every site); each site re-read before editing, and `grep -rn
"Operated::" crates/graft/src` re-run on the merged branch (line numbers had moved). No compaction.

### What was built
- `graft_ui::preview_box::set_aside_row(files)`: two rungs, no staging words. Only caller `update/preview.rs:plan_lines`.
- `Operated::StagingNotKept { applied, paths }` (`jobs/operate.rs`); `answer` maps an `AfterState` whose `unread`
  is exactly `[StagingNotKept]` to it, anything else stays `Incomplete` (Decided 4).
- `update/operate.rs`: `staging_not_kept_rungs(what, paths)` (Decided 3's three rungs, built with
  `view::status::done_line`, so `✓ ·` is `Done`'s own), `staging_not_kept_status(what, paths, columns)` →
  `Status::Sticky { reason, keys: Some("u undoes it, staging included") }`. `done_what` (a remote op's words, else
  the description) and `landed` (editor closed, refresh, landing) factored out of the `Done` arm so the new arm
  shares them.
- **Rung pick:** the status line has no ladder of its own (`fit_status_line` only cuts `reason` with `…`), so the
  rung is picked by `fitting_line`'s rule against the status row's text width (`columns - 1`: the row indents one
  cell), at answer time from `AppState::size`. The key half rides `keys`, so `reason · keys` is exactly the rung and
  the footer's fit keeps `u undoes it, staging included` whole. **Cost:** a resize after the answer does not re-pick;
  a narrower window cuts the reason with `…` (keys stay whole). A Status variant carrying the ladder would fix it;
  not done (every `Status` match would change).
- `banner.rs`: `resumed_staging_not_kept` = `resumed_done` (banner closed, refresh, landing), then the sticky led by
  `resumed_done`'s `Done.what` (a finish-summary merge's `merged X into Y` included). An `Unfinished` close keeps its
  reason plus the shortest rung (not reachable today: a put-back runs only on a close that finished). `kept_open`
  and `finish::answers::refusal` take it as `Done`. `update/mod.rs` `staged`: the resolver moves on. To stay under
  clippy's 60-line limit, `resumed`'s `Clashed` arm moved into `ended_in_clash` (same two calls).

### Red runs
- `answer_maps_…` (arm disabled for one run): `Incomplete { … error: "your edits are back · staging not kept: f.txt
  · u undoes it, staging included" }`.
- `a_switch_whose_…` (same): `left: "checkout fix — could not confirm it fully: your edits are back · staging not
  kept: f.txt · u undoes it, staging included · esc dismiss"`.
- `a_resume_whose_…` (routed to `resumed_incomplete` for one run): failed on `land_on_head` (`left: false, right:
  true`) — the old path did not close as `Done`.
- C1: see 30's ticket (red with 31's header reverted, `graft-git` restored byte-identical after).
- The rung tests were written with the function; not seen red.

### Captures (one-line diffs)
- `docs/shots/m6/set-aside-preview-{120x40,80x24}.txt`, `m6-22-set-aside-preview-{120x40,80x24,187x50}.txt`: the
  row `… put back after · 1 staged comes back unstaged` → `… put back after`.
- `m6-22-*` also: `git rebase --no-update-refs --no-autostash origin/main` → `git rebase -i --no-update-refs
  --no-autostash --empty=drop origin/main`. **Not 30b's change**: the captures were stale from earlier work
  (the rebase command changed before this branch); regenerated with the rest.
- `set-aside-done-120x40.txt`, `m6-24-set-aside-done-{120x40,80x24,187x50}.txt`: `Uncommitted changes (3
  unstaged)` → `(1 staged, 2 unstaged)`.
- `m6-24-…-187x50.txt` also: header `192 rows so far` → `202 commits` — timing of the walk's count, not 30b; the
  re-run without `GRAFT_UPDATE_SHOTS` matched. It may flake between the two.
- Captions `m6-contact-sheet-captions.tsv` (set-aside-preview ×2, m6-22 ×2, m6-24 ×2) reworded; sheet reassembled
  (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`; sheets 1, 3, 4, 5 changed, all under 200 KB).

### Touches
- `set_aside_row` callers: `grep -rn set_aside_row crates` → `update/preview.rs` and tests only.
- Every `Operated` match (`grep -rn "Operated::" crates/graft/src`): `update/mod.rs` `staged`; `update/operate.rs`
  `operated`; `banner.rs` `resumed` and `kept_open`; `finish/answers.rs` `refusal`; `resume_answered` uses `if let
  Operated::Done` (a merge finish never sets aside, so `StagingNotKept` → `Closed`, as `Incomplete`);
  `lands_on_head.rs` unchanged (`only_done_lands` asserts `Incomplete` lands nothing; the new
  `operated_takes_staging_not_kept_as_done_with_its_line` asserts this one lands). The compiler checked
  exhaustiveness.
- Checkout preview draws the same row: the new app test asserts it at 187 and 80.
- Journal browser (`J`): draws the entry's description, no staging words; unchanged (no code path touched).
- Keymap: no new key; `u` unchanged, pinned by the new test's `u`.
- Journal/undo: graft reads only; the new app test's `u` checks index and tree byte-equal.
- Docs: TESTING.md (`pull-rebase-dirty`, `switch-dirty`, new `switch-staging-not-kept` row, `m6-22`), README.md,
  `docs/design-notes/m6-stash-put-back.md` :33 and :118 (superseded note), graft-ui MAP.md `set_aside_row`, graft
  MAP.md (`Operated`, `operated`, `resumed`, the two new fns).
- Out of crate set: only the C1 test file in graft-ops. `graft-git/src/diff/patch.rs` was edited for one red run
  and restored (`git diff --stat crates/graft-git` empty).

### Gaps for the conductor
- `M7/30forecast` as the ticket says (not built).
- The rung is chosen at answer time; a resize afterwards cuts with `…` rather than re-picking (above).
- A Sticky draws in the Warning colour and, with its own `keys`, has no `esc dismiss`; `Esc` still clears it
  (`update/keys.rs`). Colour unverified.

