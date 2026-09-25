# resolver-words-and-marks: `no more conflicts this way`; a stop line that keeps a short origin; no tint on a hand-edited both-take

Status: blocked (find-and-pin 1 differs from the spec, 2026-09-25; see NOTES)
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
- **Took both, then a hand edit:** `DocView.hand_edited` from `Choices.edited` of the open file. When true, the result
  rows of a `Took(Both)` block get no tint (ADR-0044 §2); separators and the yours/theirs panes as took both.
  - Panes (≥ 180): `row_tint` (`panes/band.rs:125`, called `panes/draw.rs:123`) gets `hand_edited`.
  - Card (< 180): `draw/block.rs:124-125` and `:163-165` tint half/half. Rows showing the **result** text of a
    hand-edited took-both block: no tint. Rows showing a **side**: that side's tint, unchanged. NOTES says which of the
    two each of those lines draws.
  - Only `Took(Both)` changes; took yours / took theirs after a hand edit are unchanged (ADR-0044 §2 names only both).

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
// graft-ui/src/conflict_resolver.rs
pub struct DocView { /* existing …, 60's `finish`, */ pub hand_edited: bool }
// graft-ui panes/band.rs: existing parameters unchanged, `hand_edited: bool` added after `ours_len`
fn row_tint(/* state, pane, result_row, ours_len, */ hand_edited: bool /* , theme */) -> /* unchanged */;
// graft view: set in all three literals (view/resolver.rs:67, view/resolver/whole_file.rs:23, :67):
//   hand_edited: state.choices.get(&open.path).is_some_and(|c| c.edited),
//   // no entry = no choice recorded for this file = not edited; not a failure being hidden (CLAUDE.md rule 3)
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
- **Hand edit:** `a_hand_edited_both_take_has_no_result_tint` (graft-ui buffer, panes), `…_on_the_card` (graft-ui
  buffer, card), `doc_view_carries_hand_edited_from_choices` (graft), `a_plain_both_take_keeps_its_split` (no edit:
  today's split, unchanged).
- **`DocView` literals:** the three in graft, on 60's result, and every graft-ui test literal. Check: the build.
- **Walk:** `every_scenario::journey`; any scenario asserting the old words (60 moved `two-files-out-of-order` to the
  summary). No journey count changes.
- **Captures:** `grep -rln "no more conflicts" docs/shots` (scout 61: `m6/m6-08-no-more-conflicts-{120x40,80x24}`),
  `m6-18-launch-shell-begun-*`, and every capture whose stop line shows a rung ≥ 1 (`grep -rln "files conflicted"
  docs/shots`, then keep the graph-footer hits). Each regenerated by its own test; NOTES lists file → test. The
  contact sheet and `m6-contact-sheet-captions.tsv` are the conductor's.
- **Docs:** `crates/graft/MAP.md` (the cross rule's words and that they are a notice; `stop_wordings`' rungs and the
  short-count rule; :312 and :330); `crates/graft-ui/MAP.md` line 12 (`hand_edited`) and 15 (`row_tint`).

## Acceptance (name the verifying test at tick time)
- [ ] Find-and-pin 1: `a_hand_edited_both_take_still_reads_as_took_both`, green as expected, or the ticket stopped.
- [ ] Find-and-pin 2 and 3: the lines in NOTES.
- [ ] `n_past_the_last_block_says_no_more_conflicts_this_way`: red first, then green.
- [ ] `a_shell_stop_keeps_a_short_origin_before_dropping_it`: red first, then green.
- [ ] every test named under Touches.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-stop-line-short-origin-187x50.txt` (a shell
      merge, 13 files conflicted, the changes pane open) and `fix-no-more-this-way-120x40.txt`.
  - **What right looks like:** the graph footer's stop part reads `merge in progress · from the shell · 13 conflicted`,
    whole, before the bar. The resolver's notice row (under the identity row) reads `no more conflicts this way`; the
    bottom row has no `no more conflicts` and contains `⏎ done`.
  - Tint (hand edit): colour, so **unverified** by capture; the buffer tests are the evidence.
- [ ] MAP.md updated (doc, no test).

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

