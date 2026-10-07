# G-palette 1d — `? keys` on overlays, places and marked rows: the remainder of 11c, with its two conflicts ruled

Status: partial (builder 2026-10-08: R1, R3, captures done; R2 built as ruled but two of its numbers do not hold — NOTES) — was ready (conductor 2026-10-07; 10k + 22c merged) — was draft (planner spec-only pass 2026-10-07; replaces the unbuilt part of
`11c-doorway-on-overlays-places-and-marked-rows.md`, blocked on NOTES conflicts 1–2)
Build with: **Sonnet** — both conflicts ruled below; the patch exists; the one new piece (Marked's rank-then-move) is
a few lines in `whole_bar`'s Marked arm with a monotone test. Not Fable (no signature, judgeable from tests).
Blocked by: **10k merged** (10k may touch `update/tests.rs`, which this edits). **Serial with 15** (shares
`graph_screen/hints*`). **Capture run serial** with 22c's and 12d's (all rewrite `docs/shots`) — never concurrent.
Needs: as 11c (`DOORWAY`, the `?`→`⌃P`→nothing fallback in `spelled_bar`, `palette::open_key` before overlays/places,
09e's `ChangesPane { over_overlay }`). Scout: `scouts/11c-doorway.md` (reuse; no new scout).
**Crate set:** graft-ui (`graph_screen/hints.rs`, `hints/tests.rs`, `hints/catalogue.rs` only if a coverage slot
moves, MAP.md) + graft (`tests/it/hint_bar.rs`, `update/tests.rs`, capture tests that re-shoot, MAP.md) +
`docs/design/02-keymap.md` §2.1 + `docs/design/01-view-mockups.md` §1.0 (one sentence each) + captures.

## Read first
- 11c: Decisions 1–5 and NOTES (whole ticket is short); this file's Rulings override 11c decisions 1, 3 and 5
- `scouts/11c-doorway.md`; `scouts/11c-wip.patch` (apply it first: `git apply`)
- `docs/adr/0039-…md` "Where it sits" (:57–66) and the modal-pane rule (:110–112)
- `docs/adr/0058-…md` §4 bar bullet (:46–48)
- `crates/graft-ui/MAP.md` `graph_screen::hints` (:25, :53); then `hints/fitting.rs` `place_doorway` (:184)

## Rulings (planner; not the builder's)
**R1 — ADR-0039 stands; 11c decision 1 amended.** Overlays and places (`Journal`, `Stash`, `Map`, `PaneOverOverlay`,
`Blame` every form, `History` every form, `BackToBlame`, `BackToHistory`) take `DOORWAY` as the **trailer**, placed by
the one existing rule (`place_doorway`): after every hint drawn with words, **leading** the bare keys; last when none
are drawn. Blame{lines} at 80 = `⏎ that commit  ^ before it  L history of these lines  ? keys  /  space  w  esc` is
**right**. Why: one placement rule for every bar (ux row 4, consistency); ADR-0039's reading — what follows the doorway
is "keys the menu spells out" — is true here, since `?` lists the place's keys (§2.12); these bars keep their bare-key
ladder by 11c's own decision 1 ("they keep their own ladder"). DESIGN §1.0's "always last" was written for bars with
no bare keys (ADR-0058 §4); it gets the clause below, no behaviour change for any bar already built. **No new ADR, no
user approval needed** — the accepted ADR governs; the bar row is still his to judge by eye in the captures (rule 6).
`one_rule_puts_the_doorway_before_a_tail_and_last_without_one` now covers these contexts, assertion unchanged.

**R2 — Marked: `? keys` ranks right after `/ search`, then draws last.** Not "ranked last" (unreachable: the fitter
stops at the first hint that does not fit, so a last candidate draws only at ~230+). In the Marked arm, insert
`DOORWAY` into the candidate list immediately after the later of `u undo` and `/ search` that the list holds (neither
→ end of list); fit as today; then move the doorway, if drawn, to the end of the drawn hints. Width-neutral: a marked
bar is words-only with uniform ` · ` joins, so the move cannot overflow — assert it. Effects: `u undo` and `/ search`
are never displaced (ranked ahead of it, 10c R1 kept by construction); hints ranked after it draw only once it does
(that is the doorway's job — they are what `?` recovers); monotone in width. Accepted cost: on a branch-tip marked row
(`b r t m` present) the doorway first draws near ~129 cells, so at 120 that row shows 12 hints and no `? keys`; `?`
still answers. Why not the trailer: it cost `u undo` at 83 (11b NOTES). Not a user question — 09e already ruled that the
doorway yields on marked rows; this only makes it reachable.

**R3 — docs (replaces 11c decision 5).** 02-keymap §2.1, after "…`?` lists every key of the place, §2.12).":
"*The doorway ends every bar's words but on the modal ones ADR-0058 §4 names; where a place keeps bare keys it leads
them (ADR-0039). On a marked row it ranks right after `u undo` and `/ search` and never displaces them.*"
01-view-mockups §1.0 :51 "is always last" → "is always last among the words, and leads any bare keys a place still
draws (ADR-0039)".

11c decisions 2 and 4 stand unchanged.

## Scope
In: apply the patch; R1–R3; fix the two tests NOTES names; update expected strings; re-shoot; three new captures; docs.
Out: bar ranking beyond R2's one insertion (15); whether places should drop their bare keys (not ruled here — if he
dislikes R1's look in the capture, that is his finding, new ticket); what the palette lists (report only, as 11c).

## Signatures
No new `pub` items, no signature changes. R2's insert-then-move is private to `graph_screen/hints.rs` (Marked arm or a
private helper it calls).

## Acceptance (each tick names its test, at tick time)
- [x] **Check first** — held: no text-field sub-state draws a doorway. `graft::update::tests::a_text_field_inside_a_place_draws_no_keys_doorway` (journal `/`, blame `/` on lines, stash `s`; 120x36, 80x24, 60x20; pane shut and shown; all three fields reached). Not via `modal_bars_draw_no_doorway`: the contexts stay `Journal`/`Blame`/`Stash`, but the frame draws the prompt in place of the bar (`journal_screen.rs:379`, `view/frame/overlays.rs:182`, `stash_screen.rs:195`). Original check text:
  ~ **Check first, before the patch:** journal find, blame find while typing, stash save prompt — which `HintContext`
  each draws. If any draws `Journal`/`Blame`/`Stash` (so R1 would put `? keys` on a bar where `?` types), **stop**,
  report with the `view/rows.rs` line. Else held by `…::hints::tests::modal_bars_draw_no_doorway` (extended to those
  states' contexts).
- [x] R1 — `graft_ui::graph_screen::hints::tests::overlays_and_places_draw_the_keys_doorway` (red first on the patch, green after), `…::one_rule_puts_the_doorway_before_a_tail_and_last_without_one`, `…::every_bar_ends_with_the_keys_doorway_and_never_more` (amended: `assert_doorway_placed`). Original:
  ~ R1: each listed context at 80, 120, 200 draws `? keys`; it follows every worded hint and precedes every bare key;
  `help = "none"` → `⌃P keys`; both none → no doorway, no fragment. Red first.
  (`graft_ui::graph_screen::hints::tests::overlays_and_places_draw_the_keys_doorway` — the patch's
  `…end_with…` renamed and its `ends_with` replaced by this) and `one_rule_puts_the_doorway_before_a_tail_and_last_without_one`
- [ ] R2 — **partial.** Built as ruled (`hints.rs` `marked_list`, `doorway_last`). Held: monotone 40..=240, last whenever drawn, no overflow, never drawn without `u undo` and `/ search`, branch tip likewise (`…::a_marked_row_keeps_undo_and_search_before_the_doorway`). **Not held, by measurement:** "at 120 `? keys` drawn" — first drawn at 129 (both dividers; the 12 hints ahead of it take 119 cells); "at 83 and 92 `u undo` and `/ search` drawn" — false with or without the doorway (`u undo` is 9th; at 83 the bar ends `m merge`). Test pins 129 instead. Original:
  ~ R2: plain marked row (`RowVerbs::NONE`), both dividers: at 120 `? keys` drawn and last; at 83 and 92 `u undo`
  and `/ search` drawn; for 40..=240 monotone (once drawn, drawn at every wider width) and last whenever drawn; drawn
  width ≤ bar width after the move. Branch-tip marked row: `? keys` never displaces `u undo`/`/ search`; NOTES records
  its first-drawn width. (`…::hints::tests::a_marked_row_keeps_undo_and_search_before_the_doorway`, amended)
- [x] Tests identify the doorway by kind, assertions unchanged — `a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` (strips it only where `whole_bar` returns a trailer); `resize_is_drawn_only_when_nothing_else_was_dropped` needed no change (green). Original:
  ~ `resize_is_drawn_only_when_nothing_else_was_dropped` and `a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn`
  green: change only **how each identifies the doorway** (trailer for R1 contexts; an ordinary, droppable list hint on
  Marked) — never what it asserts. If an assertion itself must change, **stop** and report. (those tests; grep the names)
- [x] Decision 4 — `the_floor_fits_at_forty_eight_cells_of_bar` green unchanged. Measured floor change: History{rows} 36 cells (was 28), doorway alone below (`history_place::history_bar_narrows_by_its_ladder`). Original:
  ~ Decision 4: `the_floor_fits_at_forty_eight_cells_of_bar` green or amended with each affected context's measured
  floor in NOTES; no bar draws a fragment. (that test)
- [x] `?` over Map, journal, stash (graph and pane), blame, history, marked — `graft::update::tests::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn` green (first run on record). Caveat: its Marked states are `states_for`'s plain row (`update/tests.rs:4990` — the marker is not in the state). Original:
  ~ `?` over Map, journal, stash (graph and pane focus), blame, history, marked row draws the palette's `keys here`;
  a place where it does not is a **stop**. (`graft::update::tests::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn`
  — lives in `update/tests.rs` for `states_for`; first run on record)
- [x] `graft::update::tests::every_key_the_bar_draws_is_answered` green. Original:
  ~ `graft::update::tests::every_key_the_bar_draws_is_answered` green with the new contexts.
- [x] Report only — see NOTES "Palette HERE". Original:
  ~ Report only (11c's box, unchanged): per place, does `?`'s HERE list the place's keys — NOTES with the
  `update/palette.rs` line; a wrong one is a finding for 11's file.
- [x] Captures — `GRAFT_UPDATE_SHOTS=1` once, then unset green; 17 re-shot (list in NOTES), 3 new. `update::tests::capture_the_keys_doorway_for_docs_shots` (journal, blame), `it::hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots` (marked). Bar rows the user's to judge; colour unverified. Original:
  ~ Captures: `GRAFT_UPDATE_SHOTS=1` once, unset, green. Allowed diffs: a listed bar gaining `? keys` (R1 position
  or R2 last), and the hints the ladder drops for it — each listed in NOTES. Frozen history (11b NOTES list) untouched.
  New: `docs/shots/m8/doorway-journal-120x36.txt`, `docs/shots/m8/doorway-blame-80x24.txt` (shows R1's
  `? keys  /  space  w  esc`), `docs/shots/m8/doorway-marked-120x36.txt` (plain marked row). Each ≤200 KB. Bar row
  judged by the user (rule 6). **Colour unverified** (rule 5).
- [x] Docs — R3 sentences in 02-keymap §2.1 and 01-view-mockups §1.0; graft-ui MAP `graph_screen::hints` and `Blame`/`History` lines; graft MAP m8/11b row gains 11d. (checked by reading) Original:
  ~ R3 sentences in 02-keymap §2.1 and 01-view-mockups §1.0 (the patch's 02-keymap sentence replaced); MAP.md graft-ui
  `graph_screen::hints` :25 (which contexts take the trailer; Marked's ranked doorway) and :53; graft MAP.md m8/11b row
  gains 11d. (doc, checked by reading)

## Touches
- Every overlay/place bar and its tests (`hints/tests.rs` `doorway_of` :1100–1140 — places → `DOORWAY`; Marked stays
  `EXPLAIN` there; `every_context`, `coverage_slot`; `catalogue.rs` :556–557 only if a slot moves).
- `tests/it/hint_bar.rs` expected strings; `history_place` unit lists (`["esc back","? keys"]`, NOTES).
- ~20 failing capture tests (NOTES: blame_place, history_place, hint_bar, look_polish, offer_and_blame) → re-shot;
  m8 contact sheet re-assembled by the conductor.
- 09e's `ChangesPane { over_overlay: true }` already draws `DOORWAY` (scout contradiction 1): unchanged, assert only.
- Palette, keymap bindings, journal/undo: none. `view/rows.rs`: read only (the check-first box).
- Ticket 15 (bar ranking) reads the Marked list: R2's insertion is the one rank it must keep; say so in its NOTES.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k: patch + R1 test edit 6k; R2 insert-move + test 8k; two test fixes 6k; expected lists 8k; re-shoots + three
captures 14k; docs/MAP 3k. Re-shoots over ~30 files → captures split to a follow-up (conductor).


## NOTES (builder 11d, 2026-10-08)

Scout: none new — reused `scouts/11c-doorway.md` and 11c's patch (applied cleanly). Did not compact.

**R2 does not hold as written (stop-class, reported, not designed around).** Measured on the plain marked
row (`Marked(PlainRow{RowVerbs::NONE})`, which still draws `b r t c y m`): full list `. explain · ⏎ open ·
b branch · r rebase · t tag · c commit · y sha · m merge · u undo · S stash · J journal · / search · q quit …`.
`/ search` is the 12th hint at 119 cells, so the doorway after it first fits at **129** (both dividers);
branch tip (`BranchRow{current:false}`) **137**. At 83 the bar ends `m merge` — `u undo` does not fit even
without a doorway, so "at 83 and 92 `u undo`/`/ search` drawn" was never true of this context (11b's 83/92
figures must have come from another bar). What R2 guarantees does hold. Needs a ruling: accept 129, or rank
`u undo`/`/ search` earlier (ticket 15's ranking).

**Finding (rule 7, with lines): R1 does not reach the journal, stash or Map screens as the user sees them.**
`HintContext::Journal`/`Stash`/`Map` are drawn only by `draw_tab_hints` (`view/frame/bars.rs:137`), which draws
only when `tab_hint_rows` > 0, i.e. `DetailLayout::Tab { shown: true }` (`update/detail/control.rs:103`) —
and with the graph unfocused there `overlay_context` returns `PaneOverOverlay`/`ChangesPane` instead
(`view/rows.rs:699–717`). The screens draw their own footers: journal `journal_screen.rs:243` `footer`,
stash `stash_screen.rs:189`, so `docs/shots/m8/doorway-journal-120x36.txt` shows `⏎ undo this  / find  space
command log  J/esc close` — **no `? keys`**; `help-at-map-120x36.txt` did not change on re-shoot. Unverified:
whether any reachable layout draws those three contexts at all. Blame, history, `BackTo*` and the pane over
an overlay do draw R1 (captures). Fix belongs to a follow-up (journal/stash/Map footers are graft-ui
screens outside this crate set's files).

**Palette HERE (report only).** `?` opens with `opened_from = edge::context_of(state)` (`update/palette.rs:336`);
`context_of` (`update/edge.rs:63–98`) gives `Blame` (:78), `History` (:79), `Journal`/`Stash`/`Map` with the
graph focused (:84–86) — each the place's own context; pane focus over an overlay → `Changes`/`Diff`
(:91–96), the pane's keys, which are what answer there; marked row → `Graph`. None looked wrong; whether
each `ContextId`'s HERE list names the place's keys (e.g. Map `⏎ seek`) was not read — unverified.

**Captures.** Re-shot (each: `? keys` gained; R1 position = after the words, before bare keys / last):
m5-hints-ladder (Journal rows; at narrow widths the floor now gives `? keys` alone or `⏎ rev  esc close  ? keys`),
m5-hints-marked-160x48 (`? keys` last at 160), m7 blame-100x30, -140x40, -187x50, -find-187x50, -long-line-187x50,
-shallow-187x50, diff-pane-b-with-offer-187x44, esc-back-to-blame/-history-from-the-graph-120x40, history-100x30,
-binary-140x40, -file-140x40, -lines-187x50, m8 pane-over-map-bar-80x24, polish-blame-whitespace-counted-120x36.
No hint dropped by the doorway in any re-shot except the m5 ladder rows named. Frozen history untouched.
New: doorway-journal-120x36 (shows the finding above), doorway-blame-80x24 (R1's
`? keys  /  space  w  esc`), doorway-marked-120x36 (real EvilMerge fixture, a current-branch marked tip; no
`? keys` at 120 per R2's measured cost). Journal/blame shots are drawn from `states_for` states, not a real repo.

**Assertions changed outside the two named tests** (each contradicted a ruling): `marked_leads_with_explain_and_draws_no_doorway` → `…_draws_no_actions` (no-doorway assert dropped, R2; order compared without the doorway); `history_place::*` expected lists gain `? keys` (R1); `history_bar_narrows_by_its_ladder` re-measured (decision 4).

**Ticket 15:** R2's insertion (doorway right after the later of `u undo`/`/ search` in `marked_list`) is the
one rank it must keep.

Touches: graft-ui `graph_screen/hints.rs` (Journal/Stash/Map/PaneOverOverlay/Blame/History trailer, `marked_list`,
`doorway_last` — words-only bars only; checked by the full graft-ui suite), `hints/tests.rs`; graft
`update/tests.rs`, `update/place/tests/hints.rs`, `tests/it/{hint_bar,history_place}.rs`; 17 shots + 3 new;
02-keymap §2.1, 01-view-mockups §1.0; both MAPs. `catalogue.rs` untouched (no slot moved). Palette, keymap,
journal/undo, `view/rows.rs`: unchanged (read only).
