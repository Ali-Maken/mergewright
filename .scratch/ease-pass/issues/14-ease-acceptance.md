# ease-acceptance — walk the three mocks in the real app

Status: partial — handoff in NOTES, 2026-10-20 (2026-10-17: a team-topology repo and G12 assertions added for 01r; 03b added to Blocked by / Needs and Q8 extended, 2026-10-17; 05b, 06b, 06c added and Q16/Q17 and the fold checks added after the 05/06 scouts, 2026-10-17; 03r added, Q14–Q17 fixed to the user's answers (14a 15a 16a 17a), opened-merge bar check added, 2026-10-17; 2026-10-18 planner: the m3 pty waiter and the stale pty records / contact sheets from 08t's NOTES added — "Earlier milestones' records" below; 08u, 08v added to Blocked by; 2026-10-19 planner: 09c, 09p added to Blocked by)
Blocked by: 07b-graph-rows-cursor, 07c-graph-rows-rest, 09b-branch-picker-and-merged-label, 09c-branch-pane-rest, 09p-branch-pane-cost, all of 01–13, plus 01r, 03b, 03r, 05b, 06b, 06c, 06u, 08s–08v
Build with: Fable. This is the acceptance ticket: it judges the promise against the mocks.
Needs: every other ease-pass ticket: roles-and-route, roles-and-route-fixes (01r), ease-vocabulary,
ease-vocabulary-rest (03b), vocabulary-fixes (03r), commit-what-goes-in, clash-reads, resolver-picks, merge-review-at-y (05b),
graph-fold-and-lanes, lanes-on-screen-and-header (06b), fold-seek-reveal (06c), graph-rows, branch-pane,
branch-pane-rest (09c), branch-pane-cost (09p),
pixel-roles-and-tall-row, commit-screen, merge-preview, clash-screen-look, finish-review-and-look.

## Read first
- `docs/design-notes/ease-pass.md`: Promise, Journeys (including the `journeys` fence), Mock behaviours (G1–G14,
  C1–C15, M1–M19), New UI, Hard-to-reverse choices 2 and 3 (as amended 2026-10-17), Questions (1–17 answered a; the
  later answers at :254); the paragraph "**Commit secrets and done-line follow-ups settled**" (2026-10-18).
- `.scratch/ease-pass/issues/01r-roles-and-route-fixes.md`: "The rules, corrected" (R1–R3, Known limits).
- `.scratch/ease-pass/issues/08s-commit-fixes.md`: NOTES (part B1 = 08t) → Questions c, Captures "Not re-shot", Broken vs unverified.
- Each pass-2 spec's Acceptance captures list (file names only).
- `TESTING.md`: the M8 acceptance section only, for its form. `scripts/m8-accept.sh` usage line;
  `scripts/contact-sheet.sh` usage; `crates/graft/MAP.md` entry for `m8_walk`.
- ADR-0037 (contact sheet), ADR-0063 Consequences.

## Scope
In:
- `scripts/ease-accept.sh`: one command. It builds a fixture repo shaped like the mocks, walks every journey and every
  behaviour line in a pty, writes captures, and prints one summary line per scenario. The mock-shaped repo has:
  - `main ← wallet-release ← sara/fix-toc`, and teammates' branches by other authors;
  - PR-style merges `Merged PR <n>: …` with the branch deleted, plus a named merge, plus a sync merge (main into a
    feature branch);
  - a tag, and a branch 1 ahead of its upstream;
  - a new `.env` plus 3 modified files;
  - a merge where one file clashes in 1 place, and another where files clash in 3 places across 2 files;
  - a rebase that clashes.
- **A second small repo, `team`** (01r topologies; separate, so the journeys' commits never move these branches):
  - `develop` (an integration branch by ADR-0001 precedence) off `main`;
  - ordinary `wallet-release` off `develop`, with `sara/fix-toc` cut from its tip;
  - `ali/x` cut from `wallet-release` at sara's fork, then one more commit on `wallet-release`;
  - `bob/x` cut from an older sara commit. It commits `x1`, then runs `git merge sara/fix-toc`.
- `ease_walk::within_the_note`: reads the note's `journeys` fence, and fails when a walk takes more keys than its row.
- **Step counts, asserted by name.** Both fail one key over, and both fail if the end state isn't reached in that count:
  - `commit-all-subject` = **3** (`c`, typed subject, `⏎`), ending with HEAD one commit ahead and the cursor on it;
  - `merge-one-clash` = **5** (`m y o y y`), ending with a two-parent HEAD and the cursor on it.
- One assertion per behaviour ID (G1–G14, C1–C15, M1–M19), named by the ID in its test name. Text and shape checks
  only. G13 includes the opened merge: cursor on an opened merge row → bar starts `space hide the commits inside · ⏎ open
  · x compare`; cursor on a row inside it (a live branch) → the teammate bar, no `space` hint.
- **G12 on `team`**, each a named `g12_…` assertion, text only. The merged and fresh-branch states have no row the
  cursor can rest on (merged rows fold; opened rows take the merge row's branch), so 01r's unit tests cover them and
  this walk doesn't. With the cursor on a sara row:
  - `g12_sibling_never_built_on`: the `built on` line names neither `ali/x` nor `wallet-release` (wallet moved, so by
    Q15 (a) it reads `main ─◆ develop ─◆ sara/fix-toc`); `ali/x`'s lane is `┊`;
  - `g12_teammate_sync_is_not_a_merge`: the merged line reads `not merged`, never `merged into bob/x`;
  - `g12_dependent_before_the_move`: run on a copy of `team` taken before wallet-release's extra commit. The `built on`
    line reads `main ─◆ develop ─◆ wallet-release ─◆ sara/fix-toc`.
- **Fold checks** (06, 06c), each named `fold_…`, text only:
  - `fold_sync_merge_has_no_mark`: the sync merge's row carries no `▸`;
  - `fold_reveal_parent`: detail pane on a folded PR merge, follow parent 2 → the row reads `▾ …`, cursor on parent 2;
  - `fold_same_without_commit_graph`: the mock repo with `.git/objects/info/commit-graph` removed — once the view
    has promoted (wait on the promotion event, not a sleep), the rows' text equals the commit-graph run's.
- **Picks in a shell-begun merge** (05, 05b), named `picks_shell_merge`: `git merge` in the shell, launch → banner
  focused; `⏎ o u o y y` → merged; `git ls-files -u` non-empty until the last `y`.
- One assertion per answered question, named `q<N>_…`:
  - Q1: `C` on a teammate's row → picker on that branch; `⏎` switches. 2 keys.
  - Q2: the bar on a teammate's row has `x compare`; `x x` across two rows still compares.
  - Q3: `#` → no colour attributes anywhere in the frame, glyphs unchanged, still off after opening and closing the
    commit screen; `#` again restores colour.
  - Q4: `docs/perf/ease-pixel.txt` exists with `SUMMARY` lines for base / roles / roles+tall row. Quote the summary in
    the report; **don't judge it** (the results go to him).
  - Q5: ghost rows drawn above the merge preview box.
  - Q6: the merge preview has no `[ Cancel ]`/`[ Confirm ]`; `⏎` opens the incoming list.
  - Q7: `esc` on the clash screen → graph, banner focused, bar leads `⏎ back to the clashes`; `⏎` → first open clash.
    Journey `pause-and-resume`, 2 keys — on the merge and on the rebase.
  - Q8: no option row on the commit screen; `⌃S` adds a sign-off; `⌃P` from the commit screen lists amend, skip hooks
    and allow empty spelled `—`, and choosing skip hooks shows `· skipping hooks` in the header.
  - Q9: `u` after a commit → edits uncommitted, and the next `c` shows the old message.
  - Q10: opening a folded merge → its rows on their own lane joining the merge.
  - Q11: the top line keeps today's frame and ends with `lanes drawn: N of M`; the bar end is today's.
  - Q12: a rebase clash uses the marks and advance; `⏎` stages the file; `c` goes on.
  - Q13: a PR merge's folded row reads `▸ N commits from PR <n>`.
  - Q14 (a): the top line names HEAD's branch exactly once (no `on <branch>` added); its right end carries only
    `lanes drawn: …` / `… merged branches folded`.
  - Q15 (a): it is `g12_sibling_never_built_on` above.
  - Q16 (a), the departure from the mock's refusal: `o` on a clash, then `t` on the same clash → it reads `▶ took
    theirs`, no `already taken` words anywhere on screen; `u` → `◀ kept yours` back.
  - Q17 (a): the rebase that clashes lands on the clash screen, cursor on its first open clash, 0 keys after the
    rebase's confirm; a stop with no clash (an `edit` stop) still lands on the graph with the banner focused.
- Captures:
  - every file the pass-2 specs list under `docs/shots/ease-pass/`, re-taken by the walk at their widths (120, 100,
    90);
  - plus `main-pane-sibling-120.txt` and `main-pane-dependent-120.txt` from `team`;
  - each ≤200 KB.
  - Captions file `docs/shots/ease-pass/ease-pass-contact-sheet-captions.tsv`: filename, what right looks like,
    matching mock path.
  - Sheet from `scripts/contact-sheet.sh --captions-order ease-pass docs/shots/ease-pass`.
- **Earlier milestones' records** (08t NOTES; done here because this runs after every product ticket — 08u's M4 and
  06u still move these frames, so re-walking earlier would be wasted). Test tree and records only:
  - **The m3 pty waiter (08t question c):** `tests/it/m3_walk.rs:1018` waits for `nothing staged`, which the commit
    screen no longer draws (only `graft-ui/src/context_menu.rs:246` holds it). Change the waiter to the refusal words
    the in-process scenario `m3_walk::every_scenario::commit_refused_nothing_staged` asserts (read them there; the
    candidates are `graft-ui/src/commit_editor/words.rs:44-46`). Unverified until the pty walk runs: run it, and report
    `m3-05-commit-refused` timing out before the change (or say it didn't) and passing after.
  - Re-walk with `MWR_UPDATE_SHOTS=1`: the m3, m4 and m6 pty walks (`scripts/pty-walk-m{3,4,6}.py` or their wrappers),
    `scripts/m7-accept.sh`, `scripts/m8-accept.sh`. Regenerate with `scripts/contact-sheet.sh`: m3, m4, m6 (1-5 +
    captions), m7 (1-2 + captions), m8-contact-sheet-2 (+ captions), ease-pass.
  - Every re-shot record's diff against HEAD is listed in NOTES as one of: tick-first done line, 08u's landing row,
    06b/06u lanes, other. **Other → stop and file it** to the owning ticket with file and line (rule 7); never
    re-shoot over it.
  - `docs/shots/m6/prototype-journeys*` stays as recorded: it is the prototype he tried, not the app.
- `TESTING.md` "## Ease pass acceptance": per scenario, concrete words, never "looks correct". For example:
  - "cursor row has a second line starting with the author's name and ending with a 7-char hash";
  - "`lanes drawn: 5 of 8` at the top line's right end";
  - "`.env` listed under Staying out with `!`";
  - "`built on` line on sara never names `ali/x`";
  - "the sync merge's row has no `▸`";
  - "on the opened merge row the bar starts `space hide the commits inside`".

  Every line where colour carries meaning says **colour: unverified** (rule 5).

Out: fixing what fails (except the m3 waiter above, a test-tree line). File it back to the owning ticket with file and
line (rule 7).

## Signatures
```rust
pub mod ease_walk {
    pub struct JourneyRow { pub id: String, pub keys: u32, pub list: String }
    pub fn journeys_in(note: &str) -> Result<Vec<JourneyRow>, FenceError>;
    pub fn within_the_note(id: &str, walked_keys: u32, note: &str) -> Result<(), OverCount>;
}
```
Mirror `m8_walk`'s shapes. If `m8_walk` already offers these generically, reuse them and say so.

## Context guard
Captures, perf output and raw pty streams go to files. The builder reads summary lines and paths only. Never paste a
capture into the report.

## Touches
`TESTING.md`, `scripts/`, `docs/shots/ease-pass/`, `docs/shots/{m3,m4,m6,m7,m8}` records and sheets, the app's test
tree (`tests/it/m3_walk.rs` waiter). No product code.

## Acceptance
- [ ] `scripts/ease-accept.sh` exits 0 and prints 9 journey lines with walked/allowed counts, including exactly
  `commit-all-subject 3/3` and `merge-one-clash 5/5`.
- [x] A deliberately padded walk (one extra key) of each of those two fails `within_the_note` (negative test). — `ease_walk::one_key_over_fails_both_counted_journeys` (commit-all-subject 3→4 and merge-one-clash 5→6 each fail naming both numbers; at the count each passes).
- [x] Every behaviour ID G1–G14, C1–C15, M1–M19 has a passing test named with the ID. The three `g12_…`, the three
  `fold_…`, `picks_shell_merge` and every `q1`–`q17` assertion pass. NOTES lists IDs → test names. — all in `crates/mergewright/tests/it/ease_walk/`, run `cargo nextest run -p mergewright --test it ease_walk`: 87/87 pass (2026-10-20). IDs → tests in NOTES, "14b completion".
- [ ] Every capture under `docs/shots/ease-pass/` is ≤200 KB. The captions file has one line per capture with its mock
  path. The contact sheet assembles with no `NOT RECORDED`.
- [ ] The `TESTING.md` section is present, with one entry per capture and colour marked unverified.
- [ ] Pixel summary lines quoted from `docs/perf/ease-pixel.txt`, or the gap recorded in `KNOWN_GAPS.md` as
  **unverified**.
- [ ] **m3 waiter:** the m3 pty walk passes `m3-05-commit-refused` waiting on the in-process scenario's refusal words
  (name them); before/after result in NOTES. — the m3 pty walk's summary line for that scenario.
- [ ] **Earlier records current:** every m3/m4/m6/m7/m8 walk named above exits 0 with `MWR_UPDATE_SHOTS=1`; each
  regenerated sheet assembles with no `NOT RECORDED`; `grep -rn ' ✓ ·\|✓ ·' docs/shots` finds no done line in the old
  shape (resolver `done ✓ · ▸` rows and `files · all done ✓` strips excepted — they are not done lines); every diff
  classified in NOTES, none "other". — each walk's exit line; the grep in NOTES.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc`, all workspace-wide; plus `scripts/ease-accept.sh`, and the
earlier milestones' walks above.

## Size
Large (~75k), plus ~15k for the earlier records. If it overruns, split the earlier records off first (they need no
judgement against the mocks), then by screen (main / commit / merge).

## NOTES — handoff, partial (2026-10-20, builder compacted once)

**Compacted mid-ticket.** Done is the main mock and commit mock in-process walk (G1–G14, C1–C15), the team
repository with its two captures, and the fence reader. Everything below "Remains" is not started. No acceptance
box above is ticked: none is complete.

### Done (committed, all passing; full workspace nextest 7145/7145, doc tests, fmt, clippy clean)
- `crates/mergewright/tests/it/ease_walk/` (each file ≤400 lines):
  - `mod.rs`: `JourneyRow`, `FenceError`, `OverCount`, `journeys_in`, `within_the_note`, `note()`. Tests:
    `the_fence_reads_nine_rows_with_their_lists`, `one_key_over_fails_both_counted_journeys` (the negative test:
    commit-all-subject 3→4 and merge-one-clash 5→6 each fail, naming both numbers), `a_bad_fence_is_named`.
  - `fixture.rs`: `mock()` (mock-shaped repo, HEAD `wallet-release`, clean), `with_edits()` (commit mock's tree +
    `.env`), `team(moved)` (01r's topologies). Branch map in the file's doc comments.
  - `screen.rs`: frame readers (`drawn`, `bar`, `status`, `pane`, `line_with`, `cursor_on`, `buffer`, …).
  - `graph.rs` G1–G8, `bar_and_pane.rs` G9–G14, `commit.rs` C1–C15, `team.rs` G12 on `team`.
- IDs → tests so far: G1 `g1_lane_shape_is_the_role_against_the_cursor_branch`, `g1_built_on_branch_draws_thin`;
  G2 `g2_connectors_take_the_joining_branch_role`; G3 `g3_commit_dot_and_merge_ring`; G4
  `g4_merged_branches_fold_and_space_opens_and_closes`; G5 `g5_branch_column_once_per_change_with_you_on_head`; G6
  `g6_other_rows_draw_their_message_fainter`; G7 `g7_labels_after_the_message`; G8
  `g8_who_first_name_age_and_no_sha_or_badge`; G9 `g9_cursor_row_second_line`; G10/G11
  `g10_g11_header_counts_lanes_and_folds`; G12 `g12_pane_heading_marks_and_route`, `g12_sibling_never_built_on`,
  `g12_teammate_sync_is_not_a_merge`, `g12_dependent_before_the_move`, `g12_heading_says_youre_here_on_head`; G13
  `g13_bar_per_row_kind`; G14 `g14_hash_turns_colour_off_and_keeps_shapes`; C1–C15 `c1_…`–`c15_…` in `commit.rs`
  (each named by its ID).
- New captures: `docs/shots/ease-pass/main-pane-sibling-120.txt`, `main-pane-dependent-120.txt` (4.4 KB each,
  checked by the two team tests). Colour unverified.

### Remains (exact next step first)
1. `ease_walk/merge.rs` M1–M19 (split in two if over 400). The merge flow on `mock()`: `m` on `integration` (1 clash
   in report.md), `pay-307` (3 clashes in notes.md ×2 + report.md ×1, checks.md combined by git), `ali/tidy` for the
   rebase clash. Waits: preview plan not `Loading::Pending`; resolver `crate::resolver::settled`; review
   `crate::finish::review_landed`.
2. `ease_walk/journeys.rs`: 9 walks, one `#[test]` each, names in the WALKS map below; each calls
   `counted("<id>", &[keys…])`. Then restore the walk check and helpers below into `mod.rs`/`screen.rs` (change
   `counted`'s `.expect` to `unwrap_or_else(|| panic!…)`: `clippy::expect_used` is denied).
3. `ease_walk/questions.rs` q1–q17; `ease_walk/fold.rs` (`fold_sync_merge_has_no_mark`, `fold_reveal_parent`,
   `fold_same_without_commit_graph` — wait on `history.promotion.is_none()`; `picks_shell_merge` — shell-begun merge
   via `clash_stops.rs:251 relaunched_with`). Known: Q9 `commit_screen.rs:186`; Q7 `clash_stops.rs:154,180`; the
   picker `branch_pane/picker_on.rs:30`; commit options `commit_screen/shots.rs:145-148`.
4. `ease_walk/shots.rs`: each `docs/shots/ease-pass/*` ≤200 KB and in the captions TSV with a mock path. Rewrite
   `docs/shots/ease-pass/ease-pass-contact-sheet-captions.tsv` (today 45 rows, 9 ease-pass and 36 pointing at
   `../m2…m8`; 46 ease-pass captures lack a row). Add an `ease-pass` promise case to `scripts/contact-sheet.sh`
   (today the sheet says `Promise: NOT RECORDED`), regenerate with `--captions-order ease-pass docs/shots/ease-pass`.
5. `scripts/ease-accept.sh`, the `TESTING.md` "## Ease pass acceptance" section, the Q4 pixel quote (`grep SUMMARY
   docs/perf/ease-pixel.txt`, 31 lines), the m3 waiter (`tests/it/m3_walk.rs` ~:1016-1021 → `Nothing goes in. Put a
   file in with space.`; run the pty walk before and after).
6. Earlier records: the conductor said do not rebuild old milestone sheets (m3/m4/m6/m7/m8); leave that box
   unticked with that reason.

### Gotchas
- `team`: `sara/fix-toc` folds under bob's sync merge (fold rule a), so the team tests open with
  `MergedBranches::Shown` (`team::open_every_branch`).
- Leaving every file out: the cursor follows a moved file, so press `k` to the top before each `space`
  (`commit::leave_all_out`).
- At 120 columns subjects truncate (`Merged PR 16021: Fix the wall…`); G2/G3/G7/G9/G13 draw at 200.
- Frames after a write, drawn with the fixed clock, read ages "in the future"; the wall clock is better there.
- `CARGO_TARGET_DIR` must be absolute: a relative one makes fixture paths relative and 171 clone/remote tests fail.
- In the clash screen, after `o t n b` on pay-307, `y` reaches the review straight away, but
  `captures::settled_frame` then times out "waiting for a frame with nothing loading". Suspect `Combined by git ·
  reading…` never finishes. Not yet looked at: a **question**, no line.

### Findings so far (for the owners, not fixed here)
- **Defect (01r), `crates/mergewright-git/src/branch_status.rs:121`:** `(candidate.kind != CandidateKind::Other ||
  apart.base_only == 0)` lets an ordinary branch already merged into wallet-release (`dana/receipts`) be a base. On
  the mock, sara's pane reads `built on main ─◆ dana/receipts ─◆ wallet-release ─◆ sara/fix-toc`, and after his
  commit the pane reads `✓ up to date with dana/receipts`.
- **Question:** on `ops/ci`'s rows (tip and sync merge) the pane reads `<sha7> · not on a branch` while the second
  line says `made on ops/ci`. Decided at `view/branch_pane.rs:33-37`, via `focused()` at
  `update/branch_pane.rs:204-206` finding no candidate. Cause unknown (a branch level with its upstream?).
- **Question:** on `team`, sara's first commit `toc: fix the anchors` carries `bob/x` in the branch column (see
  `main-pane-sibling-120.txt`). Possibly the same as `ease/07b-made-on-lane`; no line yet.
- **Confirms** `ease/07c-wallet-no-column` and `ease/07b-made-on-lane`: on the open mock, wallet-release's rows say
  `made on sara/fix-toc` and no row reads `wallet-release (you)`.
- **Refutes** `ease/12-landing-bar`, `ease/08u-justmerged`, `ease/06-no-space-offer`: the landed bar reads `u undo
  the merge · P push · a amend · space show the 1 commit inside`, and `space` is offered on folded and opened merges.
- **Textual question:** the merge preview's footer reads `y merge  esc cancel  ⏎ see the 1 commit` with double
  spaces, where the note uses ` · `.

### Restore into `mod.rs` once `journeys.rs` exists (taken out so the commit compiles)
```rust
const WALKS: &[(&str, &str)] = &[
    ("commit-all-subject", "commit_all_subject"), ("commit-with-body", "commit_with_body"),
    ("commit-leave-first", "commit_leave_first"), ("merge-one-clash", "merge_one_clash"),
    ("merge-clean", "merge_clean"), ("merge-three-clashes", "merge_three_clashes"),
    ("undo-a-pick", "undo_a_pick"), ("pause-and-resume", "pause_and_resume_on_the_merge"),
    ("open-folded-merge", "open_folded_merge"),
];
// every_ease_journey_id_has_a_walk: for each fence row, its WALKS entry exists, journeys.rs holds
// "#[test]\nfn <test>()" and `counted("<id>"` (or the rustfmt-wrapped `counted(\n        "<id>"`); every WALKS
// id is a fence row.
pub fn allowed(id: &str) -> u32 { /* the fence row's keys, panic if none */ }
pub fn counted(id: &str, route: &[&str]) {
    // walked = route.len(); println!("journey {id} {walked}/{allowed} {route:?}"); panic on within_the_note Err
}
// screen.rs: ctrl(c) -> KeyPress (control modifier); head_parents(repo) -> usize (git log -1 --format=%P).
```

## NOTES — 14b completion (2026-10-20, continued after a killed run; no compaction)

Ticked above: the negative-test box and the behaviour-IDs box. **Not ticked, and why:** the script box (14c, `scripts/ease-accept.sh`), captures/captions/sheet (14c), `TESTING.md` (14c), pixel quote (14c; `q4_pixel_lag_summary_recorded` only checks the SUMMARY lines exist), m3 waiter (14c), earlier records (not rebuilt, user 2026-10-19), "each tick names its test" (until the rest are ticked).

IDs -> tests (all in `ease_walk/`):
- G1–G14: `graph.rs`, `bar_and_pane.rs` (names in "Done" above); G12 on `team`: `team.rs` `g12_sibling_never_built_on`, `g12_teammate_sync_is_not_a_merge`, `g12_dependent_before_the_move`.
- C1–C15: `commit.rs` `c1_…`–`c15_…`.
- M1–M19: `merge.rs` `m1_bar_offers_merge_into_yours`, `m2_preview_words_and_footer`, `m3_enter_lists_incoming_esc_back_y_merges`, `m4_esc_cancels`, `m5_stop_lands_on_the_first_clash_with_its_header`, `m6_list_column`, `m7_file_title_numbered_lines_and_the_open_clash`, `m8_taken_block_shrinks_and_says_which`, `m9_advance_within_the_file_and_the_solved_line`, `m10_n_goes_to_the_first_open_clash`; `merge_finish.rs` `m11_the_three_bars`, `m12_u_undoes_the_last_pick_from_the_screen_and_the_review`, `m13_e_edits_by_hand_at_the_cursor_block`, `m14_esc_pauses_and_enter_comes_back`, `m15_review_words_and_bar`, `m16_review_esc_back_and_y_again`, `m17_enter_looks_read_only_and_the_combined_row`, `m18_commit_lands_on_the_folded_merge`, `m19_u_undoes_the_merge`.
- Fold: `fold.rs` `fold_sync_merge_has_no_mark`, `fold_reveal_parent`, `fold_same_without_commit_graph`; `picks_shell_merge`.
- Q1–Q9 `questions.rs` (`q1_c_opens_the_list_on_his_branch_and_enter_switches`, `q2_x_compare_on_the_bar_and_x_x_compares`, `q3_hash_colour_off_until_pressed_again`, `q4_pixel_lag_summary_recorded`, `q5_ghost_rows_above_the_preview_box`, `q6_no_buttons_and_enter_lists`, `q7_esc_pauses_on_the_banner_and_enter_resumes`, `q8_options_off_screen_ctrl_s_and_ctrl_p`, `q9_u_after_commit_keeps_the_message`); Q10–Q17 `questions_late.rs` (`q10_…`, `q11_…`, `q12_rebase_clash_marks_enter_stages_c_goes_on`, `q13_…`, `q14_…`, `q15_sibling_is_never_built_on`, `q16_retake_replaces_and_u_brings_the_earlier_back`, `q17_rebase_clash_lands_on_the_clash_and_an_edit_stop_on_the_graph`).
- Journeys (`journeys.rs`, each `counted(...)`, `every_ease_journey_id_has_a_walk` ties them to the fence): commit-all-subject 3/3, commit-with-body 6/6, commit-leave-first 6/6, merge-one-clash 5/5, merge-clean 2/2, merge-three-clashes 8/8, undo-a-pick 1/1, pause-and-resume 2/2 (merge and rebase legs), open-folded-merge 1/1. **None over its count.**

Found while finishing (none is a product defect proven by a line):
- Q12 on the rebase: `o` (kept upstream) makes `ali/tidy`'s only commit identical to the base, so git drops it as empty on `c` and HEAD ends on `integration`'s tip. That is git, not Mergewright. The walk takes `t` (`▶`) so the commit survives and HEAD's subject is `Tidy the report heading`. `o` and the `◀ kept upstream` mark are covered on the merge (`m8_…`, `q16_…`).
- The earlier "Combined by git · reading… never finishes" suspicion is closed: it was the test not waiting. `flow::read_in` (pump until no `reading…`) fixes it, used in `m17_…`.
- `c` is refused with `still reading the file` while the resolver's next-file read is in flight (`update/resolver.rs:298`). A user pressing `c` the instant after `⏎` stages sees a sticky line, not a continue. Not filed as a defect: the key is answered with words, and a second press works. Mention for the usability walk.
- `fold_reveal_parent` follows the parent through `update::detail::show_line` with the app's real job pool; the detail pane has no key that follows a parent (blame `^` / history `⏎` are the keyed callers). `fold_same_without_commit_graph` relies on `ease_walk::screen::open`, which waits on `history.promotion.is_none()`; it proves the two runs' rows are equal, not that the second run exercised a promotion (no event was observed to fire).
