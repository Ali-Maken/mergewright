# resolver-words-and-marks: `no more conflicts this way`; a stop line that keeps a short origin; no tint on a hand-edited both-take

Status: done (2026-09-25; see NOTES, fourth pass)
From: user picks 2026-09-24 items 7 (batch 6, "shorten both") and 8 (batch 2, `no more conflicts this way`); ADR-0044
§2 row "took both, then a hand edit"; KNOWN_GAPS M6/15 :693, M6/21 :769-770; `docs/log/m6.md:78`.
Build with: builder-opus. Why: brownfield in two crates; three independent word/flag changes, one moving a status to a
notice with a new lifetime, and a flag threaded through the pane and card draws. Not Fable: each is pinned by a test
and a capture.
Scout: `.scratch/m6-conflicts/scouts/61.md` (main @ 3c480347). Read it before any source.
Needs: `conflict-band-and-state-colours` merged (the took-both split); **60 merged** (`DocView.finish`, the finish row,
the same three `DocView` literals — rebase on 60's); ticket 51 merged (crate order).
Crates: graft (`update/resolver/walk.rs`, `update/banner/stop.rs`, `view/resolver.rs`, `view/resolver/whole_file.rs`,
tests), graft-ui (`conflict_resolver.rs`, `panes/band.rs`, `panes/draw.rs`, `draw/block.rs`, tests).
Lane: wave 2, sixth (last build before acceptance).

## Read first
- `.scratch/m6-conflicts/user-batch.md` items 2 and 6 (:8, :14-15).
- `docs/design/02-keymap.md` §2.6 the `n` / `N` row (edited 2026-09-24).
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §2 (:60-74, the hand-edit row and the card).
- 60's spec, Scope "The finish row" and Signatures (`DocView.finish`, drawn instead of `notice`; a warning notice
  outranks it).
- `crates/graft/MAP.md` §update::resolver (the cross rule :343; `Choices` :355), §update::banner stop.rs (:307-314).
- `crates/graft-ui/MAP.md` lines 12 (`DocView`), 15 (panes), 129-131 (`GraphView::stopped`, `StopWording`,
  `stopped_row`).

## Find and pin first (before any change; stop if the finding differs)
1. **A hand-edited both-take after the re-read.** Scout 61 could not confirm what a block reads after the `handed_back`
   re-read of an edited file (`update/mod.rs:299`). Don't assume it.
   - Write `a_hand_edited_both_take_still_reads_as_took_both`: a file with one block, `b` (took both), then a hand edit
     through the path that calls `record_edited` (`update/mod.rs:294/309/342`), then drive the re-read to landing.
   - Pin: the block's state; `state.choices[path].edited`; the result rows the view draws.
   - **Spec expects:** the block is still `Took(Both)` and `edited` is true. **If the block reads anything else** (open,
     gone, another side) or `edited` is false: **stop**, report `blocked: after a hand edit the block reads <x>`. The
     tint rule below needs a spec decision then.
2. **Today's notice lifetime.** Name in NOTES where `open.notice` is set and cleared (file:line). The rule-3 words take
   that lifetime. Required: after `j`/`k`, a take landing, or `⏎`, the words are gone. **If today's notice survives any
   of those:** stop and report; a new clearing rule is a scope change.
3. **Who reads the stop ladder.** `grep -rn "stop_wordings\|stop_reason\|stop_line_wordings" crates` → NOTES, one line
   each. Scout 61: `stopped_line` (`view/rows.rs:343`) reads the ladder; `operate.rs:2166/2202/2216` read `stop_reason`
   in tests. Any other draw reading rungs ≥ 1 changes words with this ticket: list it and regenerate its captures.

## Decisions
- **Notice lifetime (conductor, 2026-09-25): (a), today's lifetime.** `no more conflicts this way` stays up while `j`/`k`
  go nowhere: it stays true, the Out list forbids a new clearing rule, and (b) would change every notice. `⏎`
  unchanged: the notice stands while the stage is in flight and is replaced when it lands; that landing is the visible
  answer. Pin: `the_notice_lifetime_today` (`update/resolver/tests/cross.rs`).
- **Rung 1 at 187 with the pane open (conductor, 2026-09-25): (a), accept the fall-through.** Measured (a temporary
  print in `stopped_row`, reverted; shell merge, 13 conflicted, pane open): 187x50 → footer 108 columns, bar 61,
  **room 44**; the new rung is **50** cells, so the footer draws `merge in progress · 13 conflicted`, the next whole
  rung, as the ladder's rule gives. 120x40 (pane below) → 120 columns, room 56: rung 1 shows whole. 160x48 (pane
  beside) → 92 columns, bar 61, **room 28**: the footer draws `merge in progress` (the old `… · 13 files conflicted`,
  39 cells, did not fit there either). The fitting is not changed (Out); the banner's full rung is unchanged.
- **Hand-edit tint dropped (conductor, 2026-09-25):** ADR-0044 amended at 6bad98dc; pin
  `a_hand_edited_both_take_reads_as_no_block_after_the_reread`.

## Scope
**In:**
- **Batch 2 — rule 3 is a notice, not a sticky status** (decided here):
  - Today `no_more_conflicts` (`walk.rs:88-98`) sets `Status::Sticky { reason: "no more conflicts", keys }`; a sticky
    status replaces the bar (`view/frame/bars.rs:33-44`), so the words and the keys are one row, and the bar's own
    hints (`⏎ done`) vanish while it stands. That is the double wording in `docs/log/m6.md:78`.
  - New: `open.notice = NO_MORE_THIS_WAY` (info, **not** a warning), and `state.status` is not touched. The bar keeps
    its hints, `⏎ done` included — the next key stays visible (UX §3.1, §2 #6).
  - The card form: the same notice; no side keys in it. The side keys stay in the bar.
  - The done-file `n`/`N` with nothing to cross (`walk.rs:110`, `Status::sticky("no more conflicts")`): the same notice.
  - **With 60's finish row:** `DocView.finish` is drawn instead of `notice`, and only a warning outranks it. So when
    every file is done, the finish row stands and `n` at the edge shows nothing new: the finish row already names the
    next key. Rule 3 on a file not yet done has no finish row (not every file done), so its notice shows.
  - The doc comment `walk.rs:8` says the new words.
  - `names::side_keys`: if walk was its last caller (MAP.md:330), delete it (clippy dead code); else leave it.
- **Batch 6 — the stop ladder, "shorten both"** (decided here: one rule for every rung):
  - **Rung 0, the full rung: unchanged** (`begun outside Graft`, `from a previous run`, `N files conflicted`; the
    missing `·` before `from a previous run`, `stop.rs:257`, stays).
  - **Every shorter rung says the count short: `N conflicted`** (`1 conflicted` singular). So the full rung spells it
    out, and every rung that has had to shorten is short in both parts. This changes the words of the existing drop
    rungs (`… · 13 files conflicted` → `… · 13 conflicted`); their order and what they drop are unchanged.
  - **New rung 1**, right after rung 0, before any rung that drops the origin:
    - shell: `merge in progress · from the shell · 13 conflicted`
    - previous run: `rebase in progress · from last run · step 3 of 7 · 13 conflicted` (the step only where rung 0 has
      one; `still` dropped, `from last run` carries it)
    - this run (no origin clause): the rung is rung 0 with the short count; duplicates are skipped as today (:211).
  - Shell, 13, the full ladder: `merge in progress · begun outside Graft · 13 files conflicted` →
    `merge in progress · from the shell · 13 conflicted` → `merge in progress · 13 conflicted` → `merge in progress`.
  - Fitting is `stopped_row` (`graft-ui hints/fitting.rs:246`; caller `graph_screen/footer.rs:104` at the graph side's
    width), **not** `fit_status_line`. It is not changed: first whole rung that fits, else the last cut with `…`.
  - **If rung 1 does not fit whole at 187 with the changes pane open:** record the room `stopped_row` had, and stop.
    Changing the fitting is a scope change.
- **Took both, then a hand edit: dropped (conductor, 2026-09-25).** Pin `a_hand_edited_both_take_reads_as_no_block_after_the_reread` shows the block is gone after the re-read; ADR-0044 amended at 6bad98dc: the row never draws, nothing is built.
~~- **Took both, then a hand edit:** `DocView.hand_edited` from `Choices.edited` of the open file. When true, the result~~
~~  rows of a `Took(Both)` block get no tint (ADR-0044 §2); separators and the yours/theirs panes as took both.~~
~~  - Panes (≥ 180): `row_tint` (`panes/band.rs:125`, called `panes/draw.rs:123`) gets `hand_edited`.~~
~~  - Card (< 180): `draw/block.rs:124-125` and `:163-165` tint half/half. Rows showing the **result** text of a~~
~~    hand-edited took-both block: no tint. Rows showing a **side**: that side's tint, unchanged. NOTES says which of the~~
~~    two each of those lines draws.~~
~~  - Only `Took(Both)` changes; took yours / took theirs after a hand edit are unchanged (ADR-0044 §2 names only both).~~

**Out:** the 80x24 relaunch stop line (KNOWN_GAPS :772); short forms (`short-words-from-the-draft`); when `n`/`N` wrap
or cross (P2, ticket 15); a new notice-clearing rule; the fitting; the banner's full-rung line.

## The failing tests first
- `n_past_the_last_block_says_no_more_conflicts_this_way` (graft `update::resolver` walk tests): every block taken, no
  other file to cross to: `n` → `open.notice == NO_MORE_THIS_WAY`; `state.status` unchanged from before `n`; the bar is
  the resolver's hints. Red today: sticky `no more conflicts · ⏎ marks <file> done`.
- `a_shell_stop_keeps_a_short_origin_before_dropping_it` (graft `update::banner` stop tests):
  `stop_wordings(shell merge, 13)[1] == "merge in progress · from the shell · 13 conflicted"`. Red today: rung 1 drops
  the origin.

## Signatures
```rust
// graft/src/update/resolver/walk.rs
pub(crate) const NO_MORE_THIS_WAY: &str = "no more conflicts this way";
// graft/src/update/banner/stop.rs: `stop_reason`, `stop_wordings`, `stop_line_wordings` signatures unchanged.
//   `stop_wordings` returns one more rung; `Wording::words` (private, :242-274) gains short forms for origin and count
//   (the builder's shape; NOTES names it).
// (dropped 2026-09-25: DocView.hand_edited and row_tint(hand_edited) — ADR-0044 @ 6bad98dc)
```

## Touches (each entry, and how the builder checks it)
- **Every `no more conflicts` string:** `walk.rs:8, :88-98, :110`; tests `resolver/tests.rs:91, :126`,
  `tests/cross.rs:134, :142`, `tests/file_keys.rs:323`, `update/tests.rs:3121` (comment),
  `tests/it/m5_walk/resolving.rs:718, :722, :751`. Check: `grep -rn "no more conflicts" crates` — every hit uses the
  const or asserts it; the tests now assert the notice, not the status.
- **Not twice, and the bar keeps `⏎ done`:** `no_more_conflicts_is_not_repeated_in_the_bar` (bottom row has no
  `no more conflicts`; it contains `⏎ done`).
- **The card:** `a_card_says_no_more_this_way_and_its_bar_keeps_the_side_keys`.
- **Lifetime:** `the_no_more_notice_is_gone_after_the_next_move` (`j`, a take, `⏎`: one row each).
- **60's finish row:** `the_finish_row_outranks_no_more_this_way` (every file done, `n` on a done file: the finish
  row is drawn, the notice is not); `no_more_this_way_shows_before_the_file_is_done`.
- **Stop ladder:**
  - `a_previous_run_stop_keeps_a_short_origin`; `a_this_run_stop_has_no_duplicate_rung`.
  - Monotonic: the existing `stop_wordings` ordering test, extended over origin × step × count (1, 13): each rung
    strictly narrower than the one before; the count is whole or absent.
  - `stop_line_keeps_the_short_origin_at_187_with_the_pane_open` (a `view` test at the graph side's real width).
  - NOTES: which rung 187 (pane open), 160 and 120 draw, for shell and previous-run; KNOWN_GAPS M6/21 :769-770 is the
    conductor's to close or re-word from that.
  - Find-and-pin 3's readers; the banner row tests green.
~~- **Hand edit:** `a_hand_edited_both_take_has_no_result_tint` (graft-ui buffer, panes), `…_on_the_card` (graft-ui~~
~~  buffer, card), `doc_view_carries_hand_edited_from_choices` (graft), `a_plain_both_take_keeps_its_split` (no edit:~~
~~  today's split, unchanged).~~
~~- **`DocView` literals:** the three in graft, on 60's result, and every graft-ui test literal. Check: the build.~~ (dropped with the tint)
- **Walk:** `every_scenario::journey`; any scenario asserting the old words (60 moved `two-files-out-of-order` to the
  summary). No journey count changes.
- **Captures:** `grep -rln "no more conflicts" docs/shots` (scout 61: `m6/m6-08-no-more-conflicts-{120x40,80x24}`),
  `m6-18-launch-shell-begun-*`, and every capture whose stop line shows a rung ≥ 1 (`grep -rln "files conflicted"
  docs/shots`, then keep the graph-footer hits). Each regenerated by its own test; NOTES lists file → test. The
  contact sheet and `m6-contact-sheet-captions.tsv` are the conductor's.
- **Docs:** `crates/graft/MAP.md` (the cross rule's words and that they are a notice; `stop_wordings`' rungs and the
  short-count rule; :312 and :330). ~~`crates/graft-ui/MAP.md` line 12 (`hand_edited`) and 15 (`row_tint`)~~ (dropped with the tint).

## Acceptance (name the verifying test at tick time)
- [x] Find-and-pin 1: the ticket stopped; pinned as observed by `a_hand_edited_both_take_reads_as_no_block_after_the_reread`
      (`tests/it/resolver/handover.rs`); the tint part dropped (ADR-0044 @ 6bad98dc).
- [x] Find-and-pin 2 and 3: the lines in NOTES, and (conductor, 2026-09-25) as tests: `the_notice_lifetime_today`
      (`update/resolver/tests/cross.rs`), `only_the_graph_footer_reads_the_short_stop_rungs` (`tests/it/stop_ladder_readers.rs`).
- [x] `n_past_the_last_block_says_no_more_conflicts_this_way`: red first, then green (`update/resolver/tests/cross.rs`).
- [x] `a_shell_stop_keeps_a_short_origin_before_dropping_it`: red first, then green (`update/banner/tests.rs`).
- [x] every test named under Touches — `no_more_conflicts_is_not_repeated_in_the_bar`,
      `the_no_more_notice_is_gone_after_the_next_move`, `no_more_this_way_shows_before_the_file_is_done` (cross.rs);
      `a_card_says_no_more_this_way_and_its_bar_keeps_the_side_keys` (`update/resolver/tests.rs`);
      `the_finish_row_outranks_no_more_this_way` (file_keys.rs); `a_previous_run_stop_keeps_a_short_origin`,
      `a_this_run_stop_has_no_duplicate_rung`, `every_stop_ladder_narrows_and_keeps_its_count_whole` (banner tests);
      walk `m5_walk::every_scenario::n_past_the_last_block_says_no_more`. **Substituted:**
      `stop_line_keeps_the_short_origin_at_187_with_the_pane_open` was not written — the 187 Decision makes rung 2 the
      right draw there; `capture_a_launch_into_a_shell_merge` (`tests/it/conflict_interim/landing_shots.rs`) asserts it
      at 187 and rung 1 at 120. Tint tests dropped with the tint.
- [x] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-stop-line-short-origin-187x50.txt` (a shell
      merge, 13 files conflicted, the changes pane open) and `fix-no-more-this-way-120x40.txt`.
  - **What right looks like:** the graph footer's stop part reads `merge in progress · from the shell · 13 conflicted`,
    whole, before the bar. The resolver's notice row (under the identity row) reads `no more conflicts this way`; the
    bottom row has no `no more conflicts` and contains `⏎ done`.
~~  - Tint (hand edit): colour, so **unverified** by capture; the buffer tests are the evidence.~~
  - **Substituted (builder):** no new files under those two names; the existing captures of the same frames were
    retaken instead. Shell merge, 13 conflicted, pane open: `fix-landing-b-launch-187x50.txt` (rung 2, per the 187
    Decision) and `…-120x40.txt` (rung 1), by `capture_a_launch_into_a_shell_merge`. The notice:
    `m6-08-no-more-conflicts-120x40.txt` / `-80x24.txt`, pty walk `n-past-the-last-block-says-no-more`. At 80x24 the
    bar says `⏎` bare, not `⏎ done` (the 80-column bar is keys only).
- [x] MAP.md updated (doc, no test): `crates/graft/MAP.md` `stop_wordings`, the cross rule (rule 3), `side_keys`, done-file `n`/`N`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: three independent changes sharing the resolver's view files, three find-and-pins. One context. If it grows,
hand off at the seam: the stop ladder (graft `stop.rs` only) is separable.

## NOTES

**2026-09-25, builder (opus). Blocked at find-and-pin 1, as the ticket and the conductor say. No product code changed.**
Scout 61 used; its lines re-checked against this tree (after 60, 52, 50, 51, 64).

- **Find-and-pin 1 — differs from the spec.** `blocked: after a hand edit the block reads gone`.
  - Test (pinned as observed, green): `a_hand_edited_both_take_reads_as_no_block_after_the_reread`,
    `crates/graft/tests/it/resolver/handover.rs`. Renamed from the spec's `…_still_reads_as_took_both`: that name would
    assert what is not true. Real repo (`Shape::StoppedMerge`, `both.txt`, one block), `b` landed, the file edited on
    disk, `e`'s `HandedOver(Ok)` fed back, the queued `OpenConflict` run for real to landing.
  - Observed: before the edit the block is `Took(Both)`. After the re-read the document has **no blocks**; the notice
    is `no conflict markers · ⏎ marks it resolved`; the frame at 120x40 shows the edited text and that notice.
    `state.choices["both.txt"]` = `takes: [Some(Both)], edited: true`.
  - Why (code, not inference): a take is written to disk with its markers removed (`o_takes_ours_on_disk`,
    `t_and_b_take`), and the re-read parses markers only; `Region::Resolved` exists only for an in-memory take
    (`graft-git/src/conflict/document.rs:112`), and `update/resolver.rs:11-15` says "re-reading collapses every block
    already taken". `conflict_arrived` (`update/resolver/answers.rs:92-94`) says the same.
  - So ADR-0044 §2's row "took both, then a hand edit" has no `Took(Both)` block to draw after `e`/`M`: the tint rule
    as specced never fires in that order. It could fire only for `edited` set by an earlier hand-over and a `b` taken
    after it (edit, re-read, then `b` on another block): then `edited` is true while that block is an in-memory
    `Took(Both)` the user took on purpose. Not tested. **Spec decision needed** (planner/conductor): drop the tint
    rule, or re-word it for that order.
- **Find-and-pin 2 — the notice's lifetime (read only, not tested; stopped before the change).** `open.notice` set:
  `walk.rs:59, :133, :161`; `take.rs:80, :122, :130, :212`; `answers/taken.rs:44, :148`; `answers.rs:87, :115, :204`;
  `settle.rs:50`; `resolved.rs:134, :195, :300`; `base.rs:53, :107`; `status_read.rs:69`. Cleared: a take landing that
  advances `take.rs:210`; a click on a block `scroll.rs:110`; `restart.rs:53`; `status_read.rs:96`; a read landing
  keeps only a non-problem notice (`answers.rs:132 landing_notice`). `j`/`k` and `⏎` clearing: **not confirmed**
  (`walk.rs:133` overwrites it on a walk; `⏎` not traced).
- **Find-and-pin 3 — who reads the stop ladder** (`grep -rn "stop_wordings\|stop_reason\|stop_line_wordings" crates`):
  - `view/rows.rs:349` `stopped_line` via `stop_line_wordings` (the only draw reading rungs ≥ 1).
  - `banner/stop.rs:284` `stop_line` (rung 0 only).
  - Tests: `banner/tests.rs:1362, :1467-1507`; `operate.rs:2166, :2202, :2216` (rung 0);
    `tests/it/m5_walk/conflict_journeys.rs:233, :391` (rung 0; its comment `:378-383` describes the 120-col rung and
    will need re-wording with the new rung 1); `tests/it/m5_walk/resolved_operations.rs:586` (comment).
- Remaining (all of Scope, untouched): batch 2 (rule 3 as a notice), batch 6 (the stop ladder, separable: graft
  `stop.rs` only), the hand-edit tint (blocked above), captures, sheet, MAP.md. No journey rows lowered. No KNOWN_GAPS
  row closed.
- Did not compact.

**2026-09-25, builder (opus), second pass. Blocked at find-and-pin 2, as the conductor said. No product code changed.**
Merged main (ADR-0044 amendment 6bad98dc). Tint scope, its signatures, Touches and acceptance rows struck above.

- **Find-and-pin 2 — differs from the spec.** `blocked: j/k with no row to step to leave the notice up`.
  - Test (pinned as observed, green): `the_notice_lifetime_today`, `crates/graft/src/update/resolver/tests/cross.rs`.
    Fixture: every block of `three.txt` taken, an info notice on the notice row.
  - `j`/`k` with **no row to step to** (a one-file list, tested; `k` on the first row or `j` on the last take the same no-target path, read in code, not tested): the file stays and **the
    notice stands**. Cause in code: `update/resolver/files.rs:200-216 step_file` computes no target and does
    nothing. Rule 3 fires exactly when no other file is left to do, so a one-file merge (the common case) is
    precisely where `j`/`k` would leave `no more conflicts this way` up.
  - `j`/`k` **onto another row**: fresh resolver, notice gone.
  - A take landing: replaced by `all blocks taken · ⏎ marks three.txt done` (`take.rs:212`).
  - `⏎`: **still up while the stage is in flight**; replaced once it lands (`resolved.rs:64-95`). Whether "gone after
    `⏎`" means at the press or at the landing is for the conductor; today it is at the landing.
  - The ticket's Out list forbids "a new notice-clearing rule", so making `j`/`k` at the edge clear it is a scope
    change: conductor's call. Options: (a) accept that the words stay (they stay true: nothing changed); (b) clear
    the notice on a `j`/`k` that goes nowhere (one line in `step_file`, touches every notice, not only rule 3).
- **Find-and-pin 3:** not made a test yet (stopped). The readers are listed in the first pass above.
- Helper made `pub(super)`: `staged` in `tests/words_and_c.rs` (used by the pin test).
- Ticks: find-and-pin 1 only. Batches 2 and 6, captures, sheet, MAP.md untouched. No journey rows lowered. No
  KNOWN_GAPS row closed. Did not compact.

**2026-09-25, builder (opus), third pass. Blocked: rung 1 does not fit whole at 187 with the pane open.**
Decision above written in; pin 3 made a test; batch 6 built; stopped at Scope's "record the room, and stop".

- **Pin 3, as a test:** `only_the_graph_footer_reads_the_short_stop_rungs` (`crates/graft/tests/it/stop_ladder_readers.rs`)
  scans non-test sources of graft and graft-ui for `stop_wordings(` / `stop_line_wordings(`: exactly
  `graft/src/update/banner/stop.rs` (the definitions) and `graft/src/view/rows.rs` (`stopped_line`, the graph footer).
- **Batch 6, built** (`update/banner/stop.rs`): `Wording::words(origin: Origin, step: bool, count: Count)`, with
  private enums `Origin { Full, Short, Gone }` and `Count { Full, Short, Gone }`. Rungs: (Full, step, Full) →
  (Short, step, Short) → (Gone, step, Short) → (Gone, no step, Short) → (Gone, no step, Gone); repeats skipped.
  Tests, red first then green: `a_shell_stop_keeps_a_short_origin_before_dropping_it`,
  `a_previous_run_stop_keeps_a_short_origin`, `a_this_run_stop_has_no_duplicate_rung`,
  `stop_wordings_drop_whole_parts_lowest_value_first` (re-worded), and the new monotonic
  `every_stop_ladder_narrows_and_keeps_its_count_whole` (origin × merge/rebase-with-step × count 0, 1, 13: each rung
  strictly narrower, the count whole or absent).
- **The stop — measured, not inferred.** The room `stopped_row` computed (a temporary `eprintln!` in
  `graft-ui/src/graph_screen/hints/fitting.rs:246`, reverted; driven by `capture_a_launch_into_a_shell_merge`, a
  shell merge with 13 files conflicted):
  - 120x40 (pane below): **columns 120, bar 61, room 56**: rung 1 (50 cells) fits; the footer reads
    `merge in progress · from the shell · 13 conflicted · ⏎ resolve …`.
  - **187x50 (pane beside the graph): columns 108, bar 61, room 44.** Rung 1 is 50 cells, **6 short**; the footer
    reads rung 2, `merge in progress · 13 conflicted · ⏎ resolve …`.
  - 160: not measured.
  - Changing the fitting is out of scope. Options for the conductor: (a) accept rung 2 at 187 (the origin shows
    from 120 below and in the banner's full rung); (b) a shorter bar beside the stop; (c) a shorter rung-1 wording
    (e.g. `merge · from the shell · 13 conflicted`, 38 cells) — a words change, the user's.
- **Pinned as observed:** `capture_a_launch_into_a_shell_merge` (`tests/it/conflict_interim/landing_shots.rs`) asserts
  rung 1 at 120 and rung 2 at 187, with the room in its comment. `relaunch_into_a_live_conflict`
  (`tests/it/conflict_interim.rs:896`, a previous-run merge) now expects `merge in progress · from last run` at 120.
  Its two rebase siblings (`:1050`, `:1098`) are unchanged (`rebase still in progress`, a substring of rung 0 and of
  the no-origin rungs; which rung they draw was not checked).
- **Captures retaken** (`GRAFT_UPDATE_SHOTS=1`, each by its own test): `docs/shots/m6/fix-landing-b-launch-120x40.txt`
  and `…-187x50.txt` ← `capture_a_launch_into_a_shell_merge`. m6 sheet rebuilt: 3 parts, 176 181 / 178 148 /
  141 772 B. No earlier milestone's captures retaken.
- **Stale, not retaken** (pty walk captures, not checked by the fast tier; they show old rung-≥1 words):
  `m6/m6-18-launch-shell-begun-{120x40,187x50,200x50}.txt` (`m5_walk/conflict_journeys.rs:40`) and
  `m6/m6-06-relaunch-sticky-120x40.txt` (`m5_walk/resolved_operations.rs:580`). Also the comment at
  `m5_walk/conflict_journeys.rs:378-383` describes the old 120-col rung. For the resume.
- MAP.md: `crates/graft/MAP.md` `stop_wordings` line updated.
- **Not started:** batch 2 (rule 3 as a notice) and its tests/captures (`fix-no-more-this-way-120x40`, m6-08); the
  187 capture `fix-stop-line-short-origin-187x50`. No journey rows lowered (none of this pass changes a step count).
  No KNOWN_GAPS row closed. Did not compact.

**2026-09-25, builder (opus), fourth pass. Done.** Scout 61 used throughout.
- Decisions: the 187 fall-through and the 160 measurement written in (160x48, pane beside: 92 columns, bar 61, room 28
  → `merge in progress`; the old `… · 13 files conflicted` did not fit there either).
- **Batch 2 built** (`update/resolver/walk.rs`): `NO_MORE_THIS_WAY` (`pub(crate) const`); rule 3 and the done-file
  `n`/`N` with nothing to cross set it as an info notice and leave `state.status` alone; the card's side keys stay in
  the bar (`names::side_keys` still has three other callers, so kept). Walk doc comment rewritten. Red first:
  `n_past_the_last_block_says_no_more_conflicts_this_way` and three siblings failed before the change, green after.
- **Pty captures retaken** (`.scratch/m6-conflicts/pty-walk-m6.py` with `GRAFT_UPDATE_SHOTS=1`, then a plain run each,
  all `settled`; m6-06 is the SIGKILL relaunch, `died on signal 9`):
  - `m6-08-no-more-conflicts-120x40`, `-80x24` ← `n-past-the-last-block-says-no-more`;
  - `m6-18-launch-shell-begun-120x40`, `-187x50`, `-200x50` ← `b-launch-to-first-conflict`;
  - `m6-06-relaunch-sticky-120x40` ← `kill-9-mid-rebase-then-relaunch`.
  - Plain-run unchanged: `m6-18-…-160x48`, `m6-18-…-80x24`, `m6-06-…-80x24`.
- Captions updated (`m6-contact-sheet-captions.tsv`): m6-06 120x40, m6-08 both, fix-landing-b-launch both, m6-18
  120/187/200. Sheet rebuilt: 176 539 / 178 148 / 141 905 B. No earlier milestone's captures retaken.
- Comment `m5_walk/conflict_journeys.rs:378-383` re-worded to the new 120 rung.
- Journey rows: none lowered — no step count changes (rule 3 adds no key; the ladder is words).
- KNOWN_GAPS rows (the conductor's to close or re-word): M6/21 `:769-770` (the stop line's origin) — the origin now
  shows short at 120 and 200, not at 160 or 187 with the pane open; M6/15 `:693` (the double wording) — words said once.
- Colour unverified: nothing in this ticket changes colour.
- **Compacted once in this pass**, after batch 2, the captures, captions, sheet and MAP.md, before these ticks and
  NOTES. What remained then was only these ticks, the NOTES, gates and commit, and all of it is done here.
