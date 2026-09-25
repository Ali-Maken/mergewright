# 82 — review fix for 72, 72b (Major M1, M2, M3, M4; Minor m1, m2, m5, m6)

Status: done
Blocked by: 73 (graft-ops, graft tests). Runs after 73 and **before 74**: it shares files with 74 (Touches, last row).
Crates: graft-ui, graft-ops, graft
Source: `docs/reviews/m6-final-fixes-72-72b.md`. Every finding names its lines; re-check each against main after 73
(73 edits `graft-ops` `apply.rs` and may add a `SetAside` path through `clash.rs`).
Build with: Opus — a stash-drop guard (ADR-0045 §2, amended 2026-09-25) as a pure rule with a table test and a
real-repo replay, then routing and words. Correctness shows in the tests, so not Fable.
Scout: none. The review names every line; M2's design is the ADR amendment.
Needs: 72, 72b (`live_clash`, `StashClash`, `ClashState`, `refresh_clash`, `stop_wordings`, `DropPutBack`); 73's NOTES.

## Decided (planner, 2026-09-25)
- **M1:** a clash's stop reason outranks the bar's tail keys; every other stop fits as today (it has a banner to say
  it, and its captures stay). One flag on `StopWording`, set only where the clash ladder is built.
- **M2:** ADR-0045 §2 amended: the candidate the journal walk finds is live only when the newest in-force entry that
  recorded an `index_after` records **exactly the index now** (`IndexState`, read by the recorder's own
  `index_state`). Any index change outside the journal → no clash → names no stash, drops nothing. No journal format
  change. Read only while a candidate exists.
- **m6:** the drop reads `drop stash@{n}`, as `StashDrop` does (`Operation::describe`, graft-ops MAP :58); `n` is the
  position `check` found **by commit** in the facts, never the launch's guess. No message, to match `StashDrop`.
- m3, m4, m7: the conductor's (gap rows, a by-eye question). Not here.

## Read first
- The review: M1–M4, m1, m2, m5, m6.
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §2, §3, §7, Consequences' last row (all amended today).
- `crates/graft-ui/MAP.md` :17 (`hints` files), :130-132 (`GraphView::stopped`, `StopWording`, `stopped_row`).
- `crates/graft-ops/MAP.md` `## clash` (:164-167), `index_state` (:133), `ApplyError::Clashed` and the put-back
  failure branch (:181-182), `Operation::describe` (:58).
- `crates/graft/MAP.md` :121-122 (`AppState::clash`), :146 (`RefsRead`: the pattern for the new read), :336-337
  (`stop_line`), :348 (`land_on_clash`), :460-466 (`update::clash`), :791 (a read job's shape).
- 72b's NOTES ("Question … 80×24", "Assertions that moved"); 73's NOTES.
- `docs/design-notes/m6-stash-put-back.md` Journeys :37-39, "Where each event lands him" :50-60, New UI :114-115.
- `docs/design-notes/m6-conflicts.md` :490-517 (the `journey-counts` fence and how the walk parses it).

## Signatures
```rust
// graft-ui, graph_screen::hints (fitting.rs) — M1
impl StopWording {
    /// The reason outranks the bar's tail keys: a stop with no banner to say it (a stash clash).
    /// `stopped_row` keeps room for the reason's shortest wording first, gives the bar the rest (the bar's own
    /// ladder, tail keys first), then draws the fullest wording that fits. Off by default: every other stop fits
    /// as today (bar whole, bare bar under 12 cells).
    pub fn before_keys(self) -> Self;
}

// graft-ops, clash.rs — M2 (ADR-0045 §2 as amended)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashCandidate { pub clash: StashClash, pub expect: IndexState }
/// §2's walk (today's `live_clash` body) plus `expect`: the newest in-force entry's recorded `index_after`.
/// `None` also when `expect` is merged or none is recorded (the tie cannot hold).
pub fn clash_candidate(entries: &[JournalEntry], in_progress: Option<InProgress>, any_unmerged: bool,
                       stashes: &[(Oid, usize)]) -> Option<ClashCandidate>;
/// §2 whole, pure: the candidate whose `expect == *index_now`. "Anything unmerged" is `index_now.is_unmerged()`.
pub fn live_clash(entries: &[JournalEntry], in_progress: Option<InProgress>, index_now: &IndexState,
                  stashes: &[(Oid, usize)]) -> Option<StashClash>;
// The read: `snapshot::index_state` made reachable from the app (a `pub use`, or a thin `pub fn` in `clash` taking
// what `index_state` takes today). One encoding, the recorder's. No second stage-tree reader.

// graft — M2's read (names follow `RefsRead` / `Command::ReadStashes`; builder reports final names in NOTES)
pub(crate) enum IndexRead { Unknown, Pending(JobId), Read(Result<graft_ops::IndexState, String>) }
// AppState gains `pub(crate) clash_index: IndexRead`; `Command::ReadIndexState { id: JobId }`; its answer event
// carries `(JobId, Result<IndexState, String>)`; a stale id is dropped.

// graft-ops, apply.rs — M4 (private; arguments as today, `failure` by value, the `None` arm gone)
fn close_unreadable_put_back(/* … */ failure: PlanFailure) -> ApplyError;
```

## Acceptance (name the verifying test at tick time)
- [x] **M1:** a one-file pop clash, resolver shut (`esc`, then the graph), at 80×24: the stop row draws the clash
      reason before the bar's tail keys. The row must contain **at least** `stash@{0} clashed` (name and state, UX
      1.1); `stash@{0} clashed · 1 file conflicted` is the target (note :114). The test asserts the **exact wording
      drawn** at 80 and at 120, not the ladder alone; NOTES say which rung is drawn at 80 and which bar keys remain.
      Relaunch at 80: the origin drops first, the name never. Tests:
      `stash_clash::the_stop_line_words_a_clash_at_80_and_120` (strengthened; red first at 80),
      graft-ui `hints::fitting::tests::a_clash_reason_outranks_the_tail_keys_at_80` (red first),
      `hints::fitting::tests::a_stop_without_before_keys_fits_as_today` (pin), and a relaunch-at-80 assertion
      (builder names it). **Ticked:** all three green; relaunch = `stash_clash::the_relaunch_stop_line_at_80_keeps_the_name`.
- [x] **M1 capture** (rule 6; colour **unverified**, rule 5): `docs/shots/m6/stash-clash-stop-line-80x24.txt`, written
      by the strengthened test through `check_shot`. **What right looks like:** the graph screen, no banner, no
      resolver; the bottom hint row begins ` stash@{0} clashed · 1 file conflicted ·` (or the rung NOTES name), then
      the keys that fit; nothing cut mid-word.
- [x] **M2 table:** `clash::tests::live_clash_table` takes `index_now` on every row and gains the five rows of ADR-0045
      §7 (amended): shell pop after an unfinished Graft pop → `None`; same path, other blobs → `None`; a Graft step
      inside the clash, index = its `index_after` → `Some`; newest index-touching entry with `index_after` unrecorded
      → `None`; Graft apply clash finished, then a shell pop → `None`. Each row's comment names its ADR bullet. Red
      first for the first row. **Ticked:** `clash::tests::live_clash_table` (§2 walk rows) +
      `clash::tests::live_clash_table_tie_rows` (the five §7 rows; split out for clippy `too_many_lines`).
- [x] **M2 real repo, graft-ops:** `stash_clash::a_shell_pop_after_an_unfinished_graft_pop_is_no_clash` (the review's
      four steps with real git; the index read + `live_clash` → `None`) and
      `stash_clash::a_graft_pop_clash_is_tied_to_the_index_it_left` (recorder's and reader's encodings agree →
      `Some`).
- [x] **M2 real repo, app:** `stash_clash::a_shell_pop_after_an_unfinished_graft_pop_names_no_stash_and_drops_nothing`
      (graft): `S p` clashes on A; shell `git checkout --ours a.txt && git add a.txt`, `git stash pop stash@{1}`; a
      working-tree read lands; the resolver's theirs names no stash; `⏎` on `b.txt` → `git stash list` still shows A
      and B; `clash == None`. Red first (it drops A today).
- [x] **M2 read cadence:** `update::clash::tests::the_index_is_read_only_while_a_candidate_exists` (no
      `ReadIndexState` with nothing unmerged, or with no `put_back` entry; one out at a time; `refresh_clash` keeps
      its value while it is out, as for the other reads);
      `update::clash::tests::an_index_read_failure_is_no_clash_and_says_so` (`clash == None`; status
      `can't tell which stash clashed · <error>`, sanitised).
- [x] **M2 journeys unchanged:** `m6_stash_put_back::a_stash_pop_that_clashes_is_four_keys`,
      `…::a_stash_apply_that_clashes_is_four_keys`, `…::esc_u_puts_a_clashed_pop_back` green: 4, 4, 2(+1); pop drops,
      apply keeps.
- [x] **M3:** `update/operate.rs:862-869` binds `launched_op` first, then
      `let remote_op = launched_op.clone().filter(Operation::is_remote);`; no shadowing. Test: the build and
      `update::operate` tests green (a rename; no behaviour change).
- [x] **M4:** `clashed_put_back` is called only inside `if let Some(failed) = &failure`;
      `close_unreadable_put_back` takes `PlanFailure` by value and returns `ApplyError`; no `Ok(Applied)` arm is left.
      Test: the build (the arm cannot be written), and 72's unreadable-status test green (builder names it).
      **Ticked on the build only:** no unreadable-put-back-status test exists in graft-ops (grep in NOTES); the type
      change is the check. Not covered by a behaviour test — see NOTES.
- [x] **m1:** the pane owe is cleared when `refresh_clash` leaves `clash == None`, and is not paid while a modal owns
      the keys. Tests: `update::clash::tests::the_pane_owe_is_dropped_when_the_clash_ends` (clean first read after the
      landing, then a dirty read: the changes pane does **not** open) and
      `update::clash::tests::the_pane_owe_waits_under_a_modal` (red first, both).
- [x] **m2:** a `Clashed` answer whose landing is refused (a resolver or box open) puts the notice words on the status
      line. Test: `update::clash::tests::a_refused_landing_puts_the_notice_on_the_status_line`
      (`stash@{0} clashed in 2 files · esc, u puts it all back`; red first).
- [x] **m5:** the three walks' doc comments use the note's ids `stash-pop-clash-to-done`, `stash-apply-clash-to-done`,
      `stash-clash-put-back`; three rows added to `docs/design-notes/m6-conflicts.md`'s `journey-counts` fence
      (`4`, `4`, `2` with `# +1 y when the drift box asks · m6-stash-put-back.md :37-39 · lazygit ~10 (A2)`);
      the walks read their counts from the fence through the helper the m5 walks use, not a hard-coded `<= 4`.
      Test: the three walks, and the fence parser's own test green.
- [x] **m6:** a `DropPutBack` entry's description is `drop stash@{n}`, `n` from the facts `check` passed. Tests:
      graft-ops `drop_put_back_is_described_by_its_position_in_the_facts` (a second stash on top → `drop stash@{1}`;
      red first); `view::journal::tests::a_clashed_pop_reads_as_a_normal_row_and_its_drop_as_a_stash_drop` updated
      to `drop stash@{0}` (NOTES: the assertion that moved); an undo-words test reads `undo: drop stash@{0}`.
      **Ticked:** undo words = `m6_stash_put_back::a_stash_pop_that_clashes_is_four_keys` (asserts the `Done` status
      `undo: drop stash@{0}` after `u`; added green, not run red first).
- [x] Contact sheet rebuilt with the new capture, each part ≤ 200 000 B. No machine paths. **Ticked:**
      `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6` → 150 captures, 4 parts, largest 179 772 B;
      `grep /Users/` on the parts: 0.
- [x] MAP.md: graft-ui `StopWording::before_keys`; graft-ops `clash` (candidate, `live_clash`'s new signature, the
      read); graft (`IndexRead`, `clash_index`, `ReadIndexState`, `refresh_clash`'s wait, m1/m2 behaviour). Doc, no
      test. **Ticked:** doc, no test (as specified).
- [x] (see NOTES below) NOTES: the TESTING.md row for `stash-clash-stop-line-80x24` (for 75); `grep -rn "live_clash" crates` output.

## Touches (check each, report how)
- **`stopped_row` for every other stop** (merge, rebase, cherry-pick, revert; banner focused or not): unchanged,
  flag off. Check: every capture with a stop row passes `check_shot` in the full nextest; `git status docs/shots`
  shows only the new capture.
- **The clash ladder's readers** (`tests/it/stop_ladder_readers.rs` pins who reads `stop_wordings`): the flag is set
  where `update::clash` builds the ladder, not in `banner::stop`. Check: that test green.
- **`live_clash` callers:** `update::clash::refresh_clash`, and any 73 added. Check: `grep -rn "live_clash" crates` in
  NOTES; each moved to the new signature.
- **`refresh_clash`'s call sites** (`journal::journal_event`, `update::working_tree_read`, `took_fact`'s `Refs(Ok)`):
  now also the index answer. Check: `refresh_clash_follows_the_reads` green plus the cadence test.
- **The `Clashed` answer** (`update::clash::clashed`): still sets `clash` at once (`ThisRun`), no wait for the read.
  Check: `a_clashing_pop_lands_on_the_resolver_on_the_first_clashed_file` green.
- **`close_after_last`** (`resolver/resolved.rs:164-172`): code unchanged; its drop now follows the tied clash.
  Check: the app replay test.
- **Job pool and stale answers:** the new read is cancellable and id-checked like `RefsRead`. Check: a stale-id unit
  test (builder names it).
- **Perf:** no read with no candidate; nothing on a clean launch. Check: the cadence test. Release perf is the
  review's, not the builder's.
- **`apply.rs` put-back branch** (M4) and 73's set-aside put-back, if it reaches `close_unreadable_put_back`. Check:
  `grep -n "close_unreadable_put_back\|clashed_put_back" crates/graft-ops/src` after 73, in NOTES.
- **`DropPutBack`'s words** everywhere they show: `J`, undo/redo status, Command Log, 72b's done status
  (`clash_drop` index, unchanged). Check: the journal view and undo-words tests.
- **`operated`** (M3): behaviour unchanged. Check: `update::operate` tests.
- **Status line** (m2) with a resolver open: the resolver's own notice row is not doubled. Check: the m2 test asserts
  the resolver's notice row is unchanged.
- **Captures:** new `stash-clash-stop-line-80x24.txt`; `stash-clash-landing-*`, `stash-clash-done-120x40` unchanged
  (`check_shot`); the m6 contact sheet.
- **`docs/design-notes/m6-conflicts.md`:** the fence only (m5). `docs/design/` and KNOWN_GAPS are the conductor's
  (M1's interim row closes when this lands).
- **74 shares files with this ticket:** graft `update/clash.rs` (74 adds the `SetAside` words and ladder),
  `update/operate.rs` (74 words the done lines), `tests/it/stash_clash.rs`, `tests/it/m6_stash_put_back.rs` (74 adds
  six walks), and graft-ui `hints/fitting.rs` (74's ladder test at 80 reads the clash row). So 82 lands first; 74's
  builder reads 82's NOTES and sets `before_keys` on the `SetAside` ladder too.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–large (~70k). If it compacts, the split is: M2 + M4 + m6 (graft-ops and the read) / M1, M3, m1, m2, m5.

## NOTES (builder, 2026-09-25)
**Compacted once** mid-ticket (after M1–M4, m1, m2, m6 code; before m5, MAP, sheet, gates). Nothing left undone.
Scout: none (ticket: the review names every line; M2's design is the ADR amendment).

**TESTING.md row for `stash-clash-stop-line-80x24` (for 75):**
`| stash clash, stop line at 80 | docs/shots/m6/stash-clash-stop-line-80x24.txt | S p clashed in notes.md, esc shut the resolver, graph focused, 80x24: graph screen, no banner, no resolver; bottom hint row begins " stash@{0} clashed · 1 file conflicted · " then the keys that fit; nothing cut mid-word. Colour unverified. |`

**Rung drawn at 80:** the full rung, `stash@{0} clashed · 1 file conflicted`. Keys that remain:
`⏎ changes  c commit  S stash  . more  p` (bar refitted to the room left; the last key before `. more` is `p`).
At 120 some row starts with the same full rung. Relaunch at 80: the test accepts either
`stash@{0} clash from last run · 1 file conflicted · ` or `stash@{0} clashed · 1 conflicted · ` (both keep the name);
which of the two is drawn was not recorded.

**`grep -rn "live_clash" crates` (code callers):** `graft/src/update/clash.rs:83` (refresh_clash, new signature);
`graft-ops/tests/it/stash_clash.rs:333,353`; `graft-ops/tests/it/set_aside.rs:370` (73's; moved to the real index
via `snapshot::index_state`); `graft-ops/src/clash.rs` tests. The rest are doc comments (`graft-core/src/journal.rs:302`,
`graft/src/update/state.rs:293`, `graft-ops/src/apply.rs:494`, `graft-ops/src/apply/set_aside.rs:184,264`).

**`grep -n "close_unreadable_put_back\|clashed_put_back" crates/graft-ops/src`:** `apply.rs:301` (comment), `:304`
(`clashed_put_back`, only inside `Some(failed)`), `:307` (`close_unreadable_put_back`), `:437`, `:467` (defs). 73's
set-aside path does not reach either. No test drives the unreadable-put-back branch (none existed in 72); M4 is
checked by the type only — **unverified by behaviour**.

**Assertions that moved:**
- `view::journal::tests::a_clashed_pop_reads_as_a_normal_row_and_its_drop_as_a_stash_drop`: `drop stash 0707070` →
  `drop stash@{0}`.
- `update::tests` search-lines test: `[none, cancelled, failed]` → `[&none, &cancelled, &failed]` (the new
  `clash_index` field pushed three `AppState`s over clippy's `large_stack_arrays`; no behaviour change).
- `m6_stash_put_back` walks: `<= 4` / `2` → `noted_keys(<id>)` from the note's fence; the m5 fence test
  (`journey_counts_match_the_note`) unions `m6_stash_put_back::JOURNEYS` (red first on the three new rows).
- `update::clash::tests::the_index_is_read_only_while_a_candidate_exists` split; the stale/foreign-id half is
  `a_stale_or_foreign_index_answer_is_not_judged` (Touches: the stale-id unit test).

**Deviations (say so, not silently):**
- `stopped_row` keeps its signature; the refit is a new `hints::stopped_row_before_keys(reason, &HintContext,
  Divider, columns)`, because refitting the bar needs the context. The footer picks it when `is_before_keys()`.
- The flag travels in a new `banner::StopLadder { wordings, before_keys }`, set in `update::clash::stop_wordings`
  (true) and `banner::stop` (false for git stops). `stop_ladder_readers` green.
- `IndexRead::Pending { id, stale }` (not `Pending(JobId)`): a read landing while the index read is out marks it
  stale, and its answer asks again rather than judging an index older than the tree read.
- Answer event: `AppEvent::Produced(id, Product::IndexState(Result<IndexState, String>))`, task
  `Task::ReadIndexState`; job `jobs::stash::read_index_state`.
- m2 words go out as `Status::Waiting(notice)` — the one status drawn neither as failure nor with a tick.
- No new graft-ops read fn: the app calls `snapshot::index_state` directly (the recorder's own encoding).
- `live_clash_table` split in two for clippy (`live_clash_table_tie_rows` holds the §7 rows).

**Verified:** gates workspace-wide (fmt, clippy -D warnings, nextest 3413/3413, doc tests). Red-first observed:
`live_clash_table` row 1, `drop_put_back_is_described_by_its_position_in_the_facts`,
`a_clash_reason_outranks_the_tail_keys_at_80`, the strengthened 80-col test, the app replay test (failed at "B's
conflict is not A's clash" with the tie disabled), m1 and m2 tests (fix toggled off), the fence test.
**Not verified:** M4 by behaviour (no test reaches it); the capture by eye (user's); colour (rule 5).
`git status docs/shots`: only the new capture plus the rebuilt sheet parts and captions row.
**Seen, not fixed:** 72's three captures (`stash-clash-landing-*`, `stash-clash-done-120x40`) have no caption row
and are not on the m6 contact sheet; only 82's capture was added. For 75/the conductor.
