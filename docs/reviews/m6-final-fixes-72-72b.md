# Review: m6-conflicts 72 + 72b — a stash clash lands on the resolver

Range `f5478def..d572438f` (72 merged `7986e5f1`, 72b `d572438f`). Reviewer, read-only, detached worktree at `d572438f`,
own `CARGO_TARGET_DIR`. Tickets: `.scratch/m6-conflicts/issues/72-…md`, `72b-…md`. Design: ADR-0045,
`docs/design-notes/m6-stash-put-back.md`.

**Counts:** Critical 0 · Major 4 · Minor 8.

## Gates (run one at a time, workspace-wide)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (26 s) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | **3387 passed, 1 skipped**, 153.4 s. The machine was loaded (another agent's gates); the log's baseline is ~50 s |
| `cargo test --doc --workspace` | pass (3 doctests) |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | **3423 passed, 0 failed**, 414 s wall, peak RSS 1.0 GB |
| Release perf `… --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold` | **13/13 passed**, 137 s. Input-to-frame p99: Unicode 4.5–5.4 ms, Pixel 6.4–6.5 ms (max 8.2 ms; budget 16 ms). First paint 100k: 15.8 ms (commit-graph), 17.9 ms (none), 46.6 ms (50k refs). Bottom: 53–61 ms with a commit-graph, 487–547 ms without one. Peak RSS 31–104 MB |
| Fuzz | not run: none of the 4 targets (`conflict_parse`, `parse_file_list`, `parse_porcelain_v2`, `parse_unified`) parses anything this diff touches |
| Benches | not run: `graft-ui` and `graft-graph` benches cover code this diff does not touch |

Fast-suite 5 slowest: `diff_fixture::parse_unified_matches_numstat_for_every_shape` 8.1 s,
`m5_walk::every_scenario::merge_resolved_with_the_keys` 8.1 s, `conflict_ops::whole_file_shapes_resolve_like_git` 8.0 s,
`undo_cancel::cancel_at_every_step_boundary_is_clean` 7.4 s,
`layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.6 s. None is new in this range.
Peak memory for nextest was 478 MB max RSS. `target/` is 6.5 GB (dev + release + heavy).

## What I verified, and how

- **Ticks:** I spot-checked 3 of 72's 4 ticks and all 3 held. The named tests exist and assert what the tick says.
  The `index_after` pin reads the journal **file** (`graft-ops/tests/it/stash_clash.rs:77-93`). The MAP lines are
  present. I spot-checked 6 of 72b's 6 ticks and all 6 held. Every named test exists. The three walks count 4, 4 and
  2(+1) keys, which matches the note's `Journeys` rows :37-39. The cadence pin asserts `ReadJournal` for both
  scopes. The captures are written by `check_shot`. The builder disclosed that the done capture's caption departs
  from the ticket's (m4 below), so that tick is not false. I could **not** re-observe the red runs claimed by
  temporary mutation. They are the builder's word.
- **ADR-0045 §3, drop by commit id and never by position: holds.** `operation/stash.rs` `position_of` finds the
  stash by `commit`, and `plan_drop_by_commit` builds `stash drop stash@{n}` from the facts `check` just passed. The
  fallback `stash drop <hex>` is proven to be refused by git
  (`git_refuses_a_stash_drop_by_bare_commit_id`). `drop_put_back_drops_by_commit_id_not_position` pushes a second
  stash on top and asserts that `stash@{1}` was dropped and the top one survived.
- **ADR-0045 §6, undo of a clashed pop keeps the stash: holds.** The entry is `Committed`, so it is in force
  (`apply.rs` `closed`). `refs/stash` does not move on a clash, so the inverse stores nothing. Two tests read git back
  to prove it. `undo_of_a_clashed_pop_restores_the_tree_and_keeps_the_stash` checks the tree and the stash list. The
  walk `m6_stash_put_back::esc_u_puts_a_clashed_pop_back` checks the tree, `status --porcelain`, `ls-files -u`, the
  stash list, and `clash == None`.
- **ADR-0045 §2 rule matches `live_clash`.** I checked `graft-ops/src/clash.rs:59-92` against the ADR bullet by
  bullet. The table test has one row per ADR row, and I traced each by hand.
- **Rule 3 on the new failure branch: holds.** `clashed_put_back` maps a status-read failure to `ApplyError::Facts`.
  `close_unreadable_put_back` (`apply.rs:431-460`) closes the entry `Aborted` with both reasons and returns git's
  failure. It discards `finish`'s `Ok(unread)` (`apply.rs:444`), but that is safe: `finish` has already folded those
  failures into the `Aborted` reason (`apply/record.rs:190-193`). `clashed` (`apply.rs:469-491`) routes unread
  after-state fields to `AfterState` rather than dropping them. In the close, `Err(busy)`
  (`resolver/resolved.rs:170-175`) is worded, not dropped. No new `unwrap`/`expect` outside tests. The one new
  `Err(_)` (`operation/stash.rs` `plan_drop_by_commit`) is documented as safe, and a test proves the refusal.
- **Builder's open point 3, the interim arm: confirmed.** 72 put it in `jobs/operate.rs` `refresh_after_failure`.
  72b added `answer`'s `Clashed → Operated::Clashed` (`jobs/operate.rs:353-358`) ahead of the generic failure path.
  The `Clashed` name left in `refresh_after_failure` (:220) cannot be reached, by construction. It is documented
  there as a guard, like `Conflicted`. There is no stale Sticky path.
- **Pipeline:** the note has Need, Prior art (citing `docs/research/commit-stash.md` A1–A4, 41 links) and Journeys.
  The keymap rows the note says the conductor owes (§2.6 `Enter` :279, §2.9 `p` :382) are already written.
- **Terminal sanitising:** the new words reach the screen through `stop_line_wordings` → `view::rows::stopped_line`,
  which sanitises (unchanged). Paths reach the notice via the resolver's existing sanitised notice row. I found no
  new raw path.
- **UI thread:** no git work in `update`. `refresh_clash` is pure over state. The drop is an `Operation` launched as a
  job.

## Critical

None.

## Major

**M1. The clash stop line is never drawn at 80×24, and no gap row says so.**
`crates/graft-ui/src/graph_screen/hints/fitting.rs:248-257`. `stopped_row` returns the bare hint bar when fewer than
12 cells are left, and at 80 columns the graph bar leaves fewer (the builder reported this; I did not render the
frame). The design note's "New UI" (:114-115) and "Where each event lands him" (:60) promise
`stash@{0} clashed · 1 file conflicted`, with a ladder down to `stash clashed`.
- **Why it matters:** a clash has no banner by design. After `esc` and a move to the graph at 80 columns, nothing on
  screen names the stash or says a put-back is half done. The same holds for the relaunch line `… clash from last
  run`. This is UX 1.1, visibility of system status, at a width his Zed/VS Code split panes often have.
- **The test:** `the_stop_line_words_a_clash_at_80_and_120` (`graft/tests/it/stash_clash.rs:315-343`) is named for 80
  but asserts only the ladder there. The code comment (:330-334) discloses this, so the tick is not false. The test is
  weak, though, and the gap is not in `KNOWN_GAPS.md`.
- **Fix:** a graft-ui ticket (outside 72b's crate set). For a clash, let the stop row cut hint keys before it drops
  the reason, or give the clash the 12 cells ahead of the tail keys. Until it lands, add a KNOWN_GAPS row marked
  **broken** and put the 80×24 graph frame after `esc` on the contact sheet for his eye.

**M2. A shell-made conflict that follows an unfinished Graft pop is named as that pop's stash, and finishing it
drops that stash.**
- **The code:** `crates/graft-ops/src/clash.rs:69-86` takes the newest in-force entry that is not a step inside the
  clash. It then checks only that entry's own before/after index shapes and that its stash is still listed. Nothing
  ties the conflict now in the tree to that entry. `crates/graft/src/update/resolver/resolved.rs:164-172` then
  launches `DropPutBack { stash }` for a `Pop`.
- **Git premise, reproduced** (scratch repo, git 2.x):
  1. Pop stash A. It clashes, and git keeps A.
  2. Resolve in the shell with `git checkout --ours a.txt && git add a.txt`, which is not a Graft write.
  3. Run `git stash pop stash@{1}` (B). It clashes too, and git keeps B.
  4. Both stashes are still listed, and `b.txt` is unmerged.
- **What Graft does after that:** if the step-1 pop was Graft's, `live_clash` names **A**. The resolver shows theirs
  as `stashed · stash@{0}`, which is A, while the conflict is B's. The last `⏎` drops A, a stash he kept on purpose
  (he took ours for it). I reasoned this from the code; I did not run it end to end in Graft.
- **Why it matters:** ADR-0045 §2's residual clause says the misnaming is "words only … Apply never drops, so nothing
  is lost". That is true only for apply, and the pop variant drops a stash. `u` brings it back (pinned,
  `StashStore` inverse), and the status names the drop, so this is not Critical. But it is an unasked-for destructive
  step recovered only if he notices, against the M3 promise.
- **Fix (planner/ADR amendment):** tie the live clash to the tree now. For example, `live_clash` could also require
  that every unmerged path now was unmerged in the entry's `index_after` stage tree (the app can pass the paths). Or
  record the clash's paths on the outcome line and compare them. Add the pop row to the §2 table test.

**M3. Readability: `remote_op` first holds any launched op, then is shadowed as the remote one**
(`crates/graft/src/update/operate.rs:862-869`).
- **What's wrong:** `let remote_op = …running.op.clone(); let launched_op = remote_op.clone(); let remote_op =
  remote_op.filter(Operation::is_remote);` A reader of line 862 is told this is a remote op when it is any op.
- **Why it matters:** `operated` is the app's central answer handler, and PROMPT 6.2 counts names that mislead.
- **Fix:** bind `launched_op` first, then `let remote_op = launched_op.clone().filter(Operation::is_remote);`.

**M4. Readability: an impossible state carries a fabricated success.**
`crates/graft-ops/src/apply.rs:431-460` `close_unreadable_put_back` takes `failure: Option<PlanFailure>`, and its
`None` arm (:453-459) returns `Ok(Applied { refresh: WorkingTree, … })`. The only comment, "`clashed_put_back` reads
nothing unless git failed", says this cannot happen.
- **Why it matters:** if a later change reached it, a status-read failure would be answered as a clean success, the
  exact thing rule 3 forbids. The type should make it unrepresentable.
- **Fix:** call `clashed_put_back` only inside `if let Some(failed) = &failure`, and pass `PlanFailure` by value to
  `close_unreadable_put_back`, whose return then becomes `ApplyError`.

## Minor

**m1. `clash_pane_owed` is cleared only when paid, and it pays without a clash or under a modal.**
- **The code:** `crates/graft/src/update/resolver/open.rs:67` sets it. `crates/graft/src/update/clash.rs:68-72` is the
  only site that clears it: it pays on any later dirty working-tree read, whether or not `state.clash` is still set,
  and before the `modal_owns_keys` check (:79) that the relaunch landing respects.
- **The risk:** if the first read after the landing shows a clean tree, the owe stays. That could happen from a quick
  `esc u`, or from a working-tree read that failed. Hours later an unrelated edit would then set
  `on_working_tree = true` and open the changes pane (`relaunch.rs:158-176`): a cursor jump nobody asked for. The
  trigger is **unverified**.
- **Fix:** clear the owe when `refresh_clash` leaves `clash == None`, and skip it under a modal.

**m2. The `Clashed` answer says nothing when the landing is refused.**
`crates/graft/src/update/clash.rs:124-134` sets `Status::Ready`, and the notice is set only if
`land_on_clash` returns `true`. That call refuses with a resolver or a box open (`resolver/open.rs:56`). He then gets
no words at all, and at 80 columns no stop line either (M1).
- **Fix:** when the landing is refused, put the notice words on the status line.

**m3. A clash with an unread after-state field is never landed on.**
`apply.rs:469-491` turns it into `AfterState` → `Operated::Incomplete`. The journal read then finds the clash as
`ThisRun`, but `refresh_and_land` lands only a `PreviousRun` clash (`update/clash.rs:79`). The builder flagged the
first half.
- **Fix:** land a clash that is newly found while `operating` is none, or add a KNOWN_GAPS row (**unverified**,
  rare).

**m4. The done capture's "what right looks like" was rewritten to match what is drawn.**
- **The mismatch:** the ticket said the changes pane, with the words on the bottom row. The capture's caption
  (`graft/tests/it/m6_stash_put_back.rs` `DONE_HEADER`) says HEAD's commit, with the status under the graph. That
  follows from the fixture: yours was taken, so the tree is clean. A stash that also changed files without a clash
  would leave changes to show.
- **Status:** disclosed in the NOTES, so it is not a false tick. It is for his eye (rule 6), not a defect.
- **Fix:** the conductor puts the capture on the contact sheet with the question "the status under the graph, not
  the last row — OK?".

**m5. The journey ids have drifted, and the rows are not in the parsed fence.**
- **The ids:** the walk doc comments say `stash-pop-clash`, `stash-apply-clash` and `stash-pop-clash-back`
  (`m6_stash_put_back.rs:319,377,421`). The note says `stash-pop-clash-to-done`, `stash-apply-clash-to-done` and
  `stash-clash-put-back` (:37-39).
- **The fence:** none of the three rows is in `docs/design-notes/m6-conflicts.md`'s `journey-counts` fence
  (:490-514), which the note (:105-107) says the conductor owes. The counts are enforced only by hard-coded `<= 4`.
- **Fix:** use the note's ids in the walks, and add the rows to the fence.

**m6. `DropPutBack` is the only place a stash is named by its short sha.**
`crates/graft-ops/src/operation.rs:875` describes it as `drop stash 0707070`, so `J` and the undo words read that way.
Everywhere else he sees `stash@{n}` and the message. This is consistency, UX 1.4.
- **Fix:** carry the index looked up at launch for the description, or word the row as `drop stash@{n} (<message>)`
  from the facts.

**m7. A stash past the 50-entry read cap is treated as "gone".**
`graft-ops/MAP.md` (`DropPutBack`) says `check` refuses `NoSuchStash` past `read`'s 50-entry cap. `live_clash` then
also finds no clash, because its stash list comes from `refs.stashes()`. With a pop of `stash@{50}` or deeper, the
resolver names no stash and the close drops nothing: safe, but the words are wrong.
- **Fix:** a KNOWN_GAPS row (**unverified**, rare), or look the commit up without the cap.

**m8. The `Operated::Clashed` arms in `banner::resumed` and `kept_open` are unreachable until 74, and untested.**
They sit at `crates/graft/src/update/banner.rs:745-748` and `:884-886`. The first also calls `operation_ended`.
- **Fix:** 74's tests must reach both arms. Say so in 74's Touches.

## Builder's open points

1. The 80×24 stop line: **M1**.
2. The done landing shows HEAD's commit: **m4**. The behaviour follows from a clean tree, so it is a question for his
   eye, not a defect.
3. The interim arm: confirmed removed. See "What I verified".
