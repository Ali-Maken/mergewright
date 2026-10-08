# G-palette 1e — `? keys` on the journal, stash and Map footers (one bar per place); R2's measured widths become the spec

Status: **done (builder 2, 2026-10-08; rulings-2 G1–G5 applied — see "NOTES (builder 2)")**. Was: partial — stopped on two stop rules. Was: ready (conductor 2026-10-08; rulings in 11e-rulings.md) — was draft (planner spec-only pass 2026-10-08; replaces 11d's open R2 box and its NOTES finding "R1 does not reach
the journal, stash or Map screens")
Build with: **Opus** (conductor 2026-10-08 after scout `scouts/11e-footers.md`: three variants enumerated in 4 files, uneven footer rules). Planner expects **Opus**: deletes three
`HintContext` variants every sweep enumerates, and moves bar coverage onto the footers he actually sees — a wrong cut
leaves a bar no test reads, which is how 11d's miss passed green. Not Fable: no signature other tickets build on;
every change has a test oracle.
Scout: **yes, first** (`scout`, Opus low, ≤1 page, `scouts/11e-footers.md`). Questions below.
Blocked by:
- **11d merged** (done, `5b0a112e`).
- **Serial with 15** (both edit `graph_screen/hints*`; 15 not started — whichever starts second rebases).
- **Capture run serial** with 12d's and 22c's (each rewrites `docs/shots`); never concurrent. 12d re-shoots captures:
  if 12d is unmerged when this reaches its capture box, wait, rebase, re-shoot after it.
- **May run beside 12f** (disjoint crates/files: 12f holds graft-core, graft-ops, graft `update/palette/*`,
  `jobs/palette.rs`, `update/tests/keymap_edge/palette_find.rs`; this holds none of them). Only shared file:
  `crates/graft/MAP.md`, different rows — conductor merges.
- **May run beside 10l** (disjoint: 10l holds `update/banner*`, `status_read.rs`, `keymap_words/stops.rs`,
  `update/palette/tests.rs`). **Two cargo helpers at a time** (memory): 12f + 11e, or 11e + 10l, never three.
Needs: graft-ui `graph_screen::hints::{hints_for, place_doorway (fitting.rs), DOORWAY}`, the `?`→`⌃P`→nothing fallback
(today inside `spelled_bar`), `Keymap`; graft `view::frame::overlays::draw_journal`, `view::stash`, `map_view`,
`view/rows.rs` `overlay_context`; `update::tests::states_for`.
**Crate set:** graft-ui (`journal_screen*`, `stash_screen*`, `map_screen*`, `graph_screen/hints.rs`, `hints/*.rs`,
`MAP.md`) + graft (`view/frame/*`, `view/rows.rs`, `view/stash*`, the journal/map view builders the scout names,
`update/tests.rs`, `tests/it/*` capture tests that re-shoot, `MAP.md`) + `docs/design/01-view-mockups.md` §1.10/§1.11
bar lines + captures.
**Not:** `update/palette*`, `update/banner*`, `update/edge.rs` (`ContextId` is untouched), graft-core, graft-ops,
keymap table / `keymap_ids.txt`.

## Read first
- 11d: Rulings R1, R2 and NOTES (the two findings) — this ticket's cause
- `crates/graft-ui/MAP.md` `graph_screen::hints` (:25 — the `Journal`/`Stash`/`Map` trailer sentence, the R2 widths),
  `journal_screen` (:26), `map_screen` (:28), `stash_screen` (:32)
- `crates/graft/MAP.md` `journal_view` (:928), `draw_journal` (:929), `stash_view` (:930), `map_view` (:939),
  `resolver_context` (:948 — **the precedent**: one `HintContext` source replaced a hard-coded footer copy, m6-05 M1)
- `docs/design/01-view-mockups.md` §1.0 :50–51, §1.10 :366–377 ("bar is per row"), §1.11
- `docs/design/02-keymap.md` §2.1 :38–48
- The scout file, then source.

## Scout questions (each answer with file:line)
1. Journal, stash, Map: where each draws its bottom bar (journal `journal_screen.rs:243`, stash `stash_screen.rs:189`
   per 11d; **Map: find it** — 11d gave no line), its type (`String`, spans, `Vec<Hint>`), every clause per state
   (journal row kinds' `enter_words`; stash rows / empty; Map), how it cuts for width, whether any clause is a bare key.
2. Every constructor of `HintContext::{Journal, Stash, Map}` (`rg`), and every path that **draws** one: is any
   reachable (`draw_tab_hints` `view/frame/bars.rs:137` with `Tab{shown:true}` and the graph focused? anywhere else)?
   Say reachable / unreachable with the line that decides it.
3. Where the doorway's `?`→`⌃P`→nothing fallback lives (`spelled_bar` — crate, file:line, signature).
4. Sub-states that must draw no doorway (journal `/` find, journal `box_open`, stash `s` prompt, Map loading/failed):
   which line blanks or replaces the footer in each.
5. Which `docs/shots` files draw a J, S or Map footer (`rg -l` on each footer's fixed words).

## Rulings (planner)
**F1 — 11d's R2 widths: the measured numbers are the spec.** Cause of the failed box: the 120 / 83 / 92 figures were the
planner's estimate from 11b's m2 captures — another row's list — not a measurement of `Marked(PlainRow)`. The rule R2
built holds everything it promised (never displaces `u undo` / `/ search`, monotone, last when drawn, reachable). Pins:
plain marked row first draws `? keys` at **129** cells, branch tip (`BranchRow{current:false}`) at **137**, both
dividers. Not re-ranked: `u undo` is 9th on every commit row, marked or not; raising it on marked rows alone would give
one row two orders (ux consistency, H4). Whether `u undo` should rank higher on every row is 15's ranking (10c NOTES
:156), not ruled here. At 120 a marked row shows no `? keys`; `?` still answers — accepted in 11d R2, no user question.

**F2 — one bar per place: the footer he sees is the bar; it gets the doorway from the one rule.** Cause of the miss:
J, S and Map draw hand-built footers, while R1 (and every bar sweep) was applied to `HintContext::{Journal,Stash,Map}`
lists those screens never draw — two sources for one bar, and the tests read the one he does not see. Same defect class
as the resolver's footer (m6-05 M1), same fix direction. The footers cannot become `HintContext` lists: J's `⏎` words
carry a branch name (`⏎ put main back to it`), and `Hint` verbs are `&'static str`. So:
- The doorway's spelling is **one function** (`doorway_words`, below); `spelled_bar` calls it, and so does each of the
  three footers. No second copy of the fallback.
- Each footer ends its words with it, placed by the one rule (after every worded clause, leading any bare key — scout
  Q1 says whether there are any; none → last). Two-space join, as those footers join today.
- Width: the doorway's cells are reserved first, as on every bar (the doorway is the last thing a bar drops); the
  footer's words shed into what remains by **their existing rule**, unchanged. Below the width where the footer's own
  narrowest form plus the doorway fit: the doorway alone; never a fragment. Measured floors → NOTES.
- No doorway where the footer is replaced: J find, J `box_open`, S save prompt, Map loading/failed if the footer is
  blank there (scout Q4).
- **`HintContext::{Journal, Stash, Map}` are deleted.** Unreachable → delete (with `every_context`, coverage slots,
  `marked_form`/`pane_focused_form` arms, `fitting`'s trailer list, catalogue lists, their tests). Reachable (Q2) → that
  drawer calls the place's footer builder instead, then delete. `PaneOverOverlay` is untouched (already draws `? keys`).
- Coverage moves with them: a new sweep asserts every key each footer draws is answered at that place (below), replacing
  what `every_key_the_bar_draws_is_answered` checked on the dead lists.
No user question: `? keys` on every bar is approved (ADR-0058, ADR-0039, DESIGN §1.0); the footers' words are unchanged.

## Scope
In: F1 (pins + one capture), F2, docs, re-shoots. Out: re-ranking any bar (15); `Hint` dynamic verbs; what the palette
lists per place (11d NOTES: unverified, a separate finding); stash/J/Map footer words.

## Signatures
```rust
// graft_ui::graph_screen::hints (or the module the scout finds `spelled_bar` in — NOTES says which)
/// The bar doorway as his keymap spells it: `? keys`, else `⌃P keys` when `help` is unbound, else None.
/// The one source: `spelled_bar` and the journal / stash / Map footers all call it.
pub fn doorway_words(keys: &Keymap) -> Option<String>;
```
Footer builders: if one gains a `keys: &Keymap` parameter, it goes last; NOTES quotes each final signature and its
callers (`rg`). Deleted: `HintContext::Journal`, `HintContext::Stash`, `HintContext::Map` (and their coverage slots;
`COVERAGE_SLOTS` renumbered — MAP says the new count).

## Acceptance (each tick names its test, at tick time)
- [x] **Check first** (re-read at base; no test — a read: graph footer only with no overlay, `view/frame.rs` ~:185; `draw_tab_hints` needs pane focus): scout Q2's answer re-read at base; if a reachable drawer of the three contexts draws something
  other than the place's footer **and** the user can see both at once, stop and report the layout with the line.
- [x] F1 (`graft_ui::graph_screen::hints::tests::a_marked_row_keeps_undo_and_search_before_the_doorway`, green): `graft_ui::graph_screen::hints::tests::a_marked_row_keeps_undo_and_search_before_the_doorway` pins first-drawn
  width 129 (plain, both dividers) and 137 (branch tip, both dividers) as exact values, monotone 40..=240, last when
  drawn. Amend only the pins; the rest unchanged.
- [x] (`it::hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots`, green after re-shoot; bar ends `/ search · ? keys` at 140) F1 capture: `docs/shots/m8/doorway-marked-140x40.txt` (EvilMerge fixture as 11d's 120 shot; bar ends `? keys`).
  If the fixture row's first-drawn width is over 140, shoot at 160x48 and name it so. `doorway-marked-120x36.txt` stays:
  NOTES gives its contact-sheet caption "at 120 a marked row draws no `? keys` (R2's measured cost)".
  (`it::hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots`, extended)
- [x] F2 one source (`graft_ui::graph_screen::hints::tests::doorway_words_follows_his_keymap`, green): `doorway_words` — `help` bound → `? keys`; `help = "none"` → `⌃P keys`; both none → `None`;
  remapped `help` → his label. `spelled_bar`'s existing doorway tests green unedited.
  (`graft_ui::graph_screen::hints::tests::doorway_words_follows_his_keymap`)
- [x] F2 footers, red first (`graft_ui::journal_screen::tests::footer_ends_with_the_keys_doorway`, `graft_ui::stash_screen::tests::{footer_ends_with_the_keys_doorway, footer_never_draws_a_fragment}`, `graft_ui::map_screen::tests::{footer_ends_with_the_keys_doorway, footer_never_draws_a_fragment, the_doorway_ends_the_keys_and_the_notes_follow}`; red 6/9 before the fix, green after): J (every row kind `journal_view` makes: `⏎` worded, none, a live row), S (rows, empty),
  Map (ready) at 48, 80, 120, 200 cells: the footer ends with `? keys` (or leads its bare keys); `⌃P keys` when `help`
  unbound; nothing when both unbound — no fragment, no stray separator; once drawn, drawn at every wider width.
  (`graft_ui::journal_screen::tests::footer_ends_with_the_keys_doorway`, `…stash_screen::tests::…`,
  `…map_screen::tests::…` — or one table test in graft if the footers are built there; NOTES names it)
- [x] (`graft::update::tests::a_text_field_inside_a_place_draws_no_keys_doorway` — setup-only edit, G5; `graft::update::tests::place_footers::a_box_over_a_place_draws_no_keys_doorway`, renamed per E3; Map Loading draws the doorway per G3) F2 no doorway in sub-states: J find, J `box_open`, S save prompt (and Map loading/failed per Q4).
  `graft::update::tests::a_text_field_inside_a_place_draws_no_keys_doorway` green unedited, plus `box_open` case
  (`…::a_preview_over_the_journal_draws_no_keys_doorway`).
- [x] F2 deletion (the `rg` is empty; `graft_ui::graph_screen::tests::every_context_reaches_every_shape` green, `COVERAGE_SLOTS` 113): `rg -n 'HintContext::(Journal|Stash|Map)\b' crates` empty; `every_context_reaches_every_shape`,
  `coverage_slot` exhaustive, green. (those tests; the `rg`)
- [x] (`graft::update::tests::place_footers::every_key_a_place_footer_draws_is_answered`; red at the merged base on Map Empty/Failed `j`/`k` — quoted in NOTES builder 2 — and by mutation (`stash.branch` clause removed → red), green after) F2 coverage moved, red first (a footer clause with no answering key fails it — show by mutation, revert):
  every key each J / S / Map footer draws, pressed at that place via `dispatch_key`, acts (not a no-op, not a refusal
  for a reason the footer did not show). (`graft::update::tests::every_key_a_place_footer_draws_is_answered`)
- [x] (`graft::update::tests::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn`, green, loops `place_footers::place_footer_states`) `?` from each footer opens the palette: `graft::update::tests::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn`
  green (its J/S/Map states now the footers').
- [x] (`graft::update::tests::every_key_the_bar_draws_is_answered`, `graft::update::tests::every_bound_action_is_hinted_live_or_is_named_here`, both green; the S footer is a hint source and draws `b`, G1) `graft::update::tests::every_key_the_bar_draws_is_answered`, `…::every_bound_action_is_hinted_live_or_is_named_here`
  green. If the second fails because a J/S/Map key was hinted only by a deleted list: the footer draws it → add the
  footer as a hint source in that test; it does not → **stop** (a §2.1 spec change), report the key.
- [x] (the capture tests listed in NOTES builder 2, green on the unset run; diff list there) Captures: `GRAFT_UPDATE_SHOTS=1` once, unset, green. Allowed diffs: a J/S/Map footer gaining `? keys` and the
  words its existing rule sheds for it; `m5-hints-ladder`'s Journal rows (generator loses the variant — NOTES says what
  replaces them). Each listed in NOTES. `docs/shots/m8/doorway-journal-120x36.txt` now shows `? keys`; new
  `doorway-stash-120x36.txt`, `doorway-map-120x36.txt`; `help-at-map-120x36.txt` re-shot. ≤200 KB each. Bar rows the
  user's to judge (rule 6); **colour unverified** (rule 5).
- [x] (checked by reading: `git diff` of both MAP.md files and 01-view-mockups) Docs: 01-view-mockups §1.10 :376 bar and §1.11's bar line (if it draws one) gain `? keys` last; graft-ui MAP
  (:25 trailer sentence drops `Journal`/`Stash`/`Map`, `doorway_words`, the three screens' footer lines, new
  `COVERAGE_SLOTS`); graft MAP (`draw_journal`, `stash_view`, `map_view` footer source). (checked by reading)

## Touches
- Every bar sweep that enumerates `every_context` (`hints/tests.rs`, `update/tests.rs`), `catalogue.rs` coverage slots,
  `fitting.rs` trailer list: grep each variant name before/after (NOTES).
- `view/rows.rs` `overlay_context` (returns the deleted variants with the graph focused per 11d NOTES :699–717).
- `view/frame/bars.rs` `draw_tab_hints`; `view/frame/overlays.rs` `draw_journal`; the status row J fills
  (`journal_status_row`) — unchanged, assert a problem line still wins the row.
- Keymap / `keymap_ids.txt`, journal/undo, palette, `edge::context_of`: none (empty diff shown).
- Captures with J/S/Map footers (scout Q5) and m5-hints-ladder; m8 contact sheet re-assembled by the conductor.
- Ticket 15: one fewer context to rank; say so in NOTES.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~55k: F1 pins + capture 5k; `doorway_words` + three footers + tests 14k; variant deletion + sweeps 14k; footer-keys
sweep 8k; re-shoots + three captures 10k; docs/MAP 4k. Past ~65k after the deletion: stop, captures to a split.

## NOTES (builder, 2026-10-08)

**Compacted once mid-ticket.** Scout used (Q2 re-read at base). Stopped, not done.

### Stops (binding)
1. **§2.1 stop — `stash.branch`.** `every_bound_action_is_hinted_live_or_is_named_here` (graft `update/tests.rs` :6020)
   fails: `bound, and drawn on no bar in any context: ["stash.branch"]`. It was "live" only in the deleted `stash_list`
   (`hints/lists.rs` at base :401–409, `hint(STASH_BRANCH, "branch")`); the stash footer never drew it
   (`graft-ui/src/stash_screen.rs` :214–218 lists apply/pop/drop/files only), yet `b` is bound and answered
   (`graft/src/update/stash.rs` :229). Mockup §1.11 draws `b branch from stash`. Needs a ruling: footer draws `b branch`,
   or `stash.branch` goes on `NEVER_HINTED` with a reason.
2. **E5 stop — Map `j/k` at Empty and Failed.** `place_footers::every_key_a_place_footer_draws_is_answered` fails:
   `map, empty`/`map, failed`: `j` and `k` of `j/k select` do nothing. The footer draws them in every non-Loading
   status: `graft-ui/src/map_screen/render.rs` :145 (`key_verb(&[app.down, app.up], "select")`), likewise `⏎ zoom`
   :146 (not reported dead — `⏎` presumably answered or idle-equal; unchecked). The same sweep also flags `k` on the
   Ready Map: a test-state artifact (cursor on row 0, `k` cannot move) — the sweep should press from a middle row;
   not a product defect.

### Other findings
- **Map Loading (E3 check-first): `?` opens the palette there** —
  `place_footers::the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers` fails with
  `left: "esc cancels"` vs `"esc cancels  ? keys"`. Next step: `map_screen/render.rs` :136 Loading arm → build
  `esc cancels` as a clause through `fit_place_footer` (no notes) so it ends `? keys`.
- `place_footers::a_preview_over_the_journal_draws_no_keys_doorway` fails: no journal footer key opens a preview in
  these states (stash `⏎`/`p` do). Next step: open J's preview the way `update::journal` does (find the key/command
  that sets `state.preview` on a journal row) and add it; S half already reached.
- `a_text_field_inside_a_place_draws_no_keys_doorway` could **not** stay unedited: it matched the deleted variants.
  Its J and S states now come from `place_footers` (journal row 0, stash list); assertion unchanged; green.
- `the_journal_screen` exact-bar pins updated to end `? keys`: `update::journal::tests::minors::no_b_hint_on_a_step_that_names_no_commit`,
  `it::journal_screen::esc_while_typing_closes_the_prompt_only` (+ three in `graft_ui::journal_screen::tests`).
- J change: when first + close don't fit, J now draws the doorway alone (E1's shed rule), not `close` alone.
- `m5-hints-ladder`: the Journal case and its `rev` section removed (the J footer is no hint context; nothing
  replaces them). Ticket 15: one fewer context to rank; `shed_footer` is not the graph ladder.
- S save-prompt line still truncates (unchanged, out of scope).

### Signatures
- `graft_ui::graph_screen::hints::doorway_words(keys: &Keymap) -> Option<String>` (pub); `doorway_key` (crate).
- `hints::fitting::shed_footer(clause_widths: &[usize], sep: usize, room: usize) -> Vec<usize>`;
  `fit_place_footer(clauses: &[String], keys: &Keymap, notes: &[String], width: usize) -> PlaceFooter`;
  `place_footer_text(clauses: &[String], keys: &Keymap, width: usize) -> String` (crate).
- J `footer(&self, width: usize)`; S `footer(&self, width: u16)`; Map `footer_line(&self, width: u16)`.
- graft `view::rows::overlay_context`: Map/Journal/Stash with graph focus → `None`.

### Remaining (exact next steps)
1. Rulings on stops 1 and 2. 2. Map Loading doorway (above). 3. J preview case (above). 4. Sweep: press Map `k` from a
non-top row. 5. `GRAFT_UPDATE_SHOTS=1 cargo nextest run --test-threads 4` once, unset, green; check E4 (m4-05,
m4-remote-keys, m6/fix-box-owns-the-bar unchanged); capture drifts seen so far, all J/S/Map footers: `capture_m2_map_view`,
`capture_m3_journal`, `capture_m3_stash`, `capture_journal_after_squash`, `journal_expanded_capture`,
`journal_find_capture`, `the_three_lost_work_journeys`, `capture_the_ladder_for_docs_shots`, the doorway captures
(test code for `doorway-{journal,stash,map}-120x36`, `doorway-stash-60x20`, `doorway-marked-140x40` written, not shot).
`help-at-map` not yet checked. 6. graft-ui MAP :25 trailer sentence (m8/11e history line added; :25 itself unread).
7. Gates; final commit `m8-polish-and-ship-11e: …`.

### Touches (checked)
`rg 'HintContext::(Journal|Stash|Map)\b' crates` empty; every_context sweeps (graft-ui green); `overlay_context`
(rows.rs); doorway palette test (green); keymap/journal/undo: no diff (`git diff --stat` shows no keymap or
journal-op files). Docs: 01-view-mockups §1.10 :376, §1.11 bar; both MAP.md files.

## NOTES (builder 2, 2026-10-08)

Merged `worktree-agent-aaabcad8cbbc98e96` cleanly (no conflicts with 10l/12f/20/18a). No scout: the previous
builder's NOTES and scout file named every line; reads were the handlers below. Not compacted.

### Rulings-2 applied
- **G1** S Ready footer: `⏎ apply  p pop  s save  d drop  b branch  space files  esc close  ? keys`. Word `branch`:
  `Keymap::key_verbs` takes the caller's verb (`keymap/spell.rs:53`); `branch` is the deleted `stash_list`'s word
  (`hints/lists.rs` at base :406) — no new word. (Mockup §1.11 says `b branch from stash`; it also says `space
  preview` where the code says `space files` — pre-existing drift, not touched.) Loading/Empty/Failed: `s save  esc
  close  ? keys`. Tests: `graft_ui::stash_screen::tests::the_branch_key_is_drawn_only_with_a_stash_selected`;
  `every_key_a_place_footer_draws_is_answered` asserts `b` opens `name_prompt`. Mutation (clause removed): those two +
  `every_bound_action_is_hinted_live_or_is_named_here` red; restored green. `NEVER_HINTED`:
  `git diff f800e8ce -- crates/graft/src/update/tests.rs | grep '^[+-]' | grep -c NEVER_HINTED` → 0.
  02-keymap §2.9 `b` status cell → "built (m3/m5 name prompt, `update/stash.rs`)"; only edit there.
- **G2** red at the merged base (quoted from the run): `map, empty: \`j\` of \`j/k select\` in \`j/k select  ⏎ zoom to
  commits  M/esc back  ? keys\`` (and `k`, and both at `map, failed`). Fix: `map_screen/render.rs` `footer_line` draws
  `j/k select` only at `MapStatus::Ready`. `⏎` kept: at Empty/Failed it is not a no-op — `seek_selected`
  (`update/map.rs:191-193`) closes the Map when no row is selected; the sweep passes it. **Question (not a defect, no
  ruling asked):** its words there say `zoom to commits` while it only closes. Test:
  `graft_ui::map_screen::tests::empty_and_failed_draw_no_keys_that_do_nothing`. Snapshots `empty_snapshot`,
  `failed_snapshot` re-accepted (footer loses `j/k select`). Sweep's Map Ready states now use three rows, cursor on
  row 1 (`place_footers::three_branch_map`), so `k` is a real press.
- **G3** Map Loading: `esc cancels  ? keys` (built through `fit_place_footer`). Test
  `the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers` now also presses `esc` after `?`: palette
  closed, `map.data` still `Pending(704)`, no `CancelJob`. `loading_snapshot` re-accepted.
- **G4** `a_preview_over_the_journal_draws_no_keys_doorway` renamed `a_box_over_a_place_draws_no_keys_doorway`. S via
  `⏎` (apply preview); J via state (`state.preview` set to that same box; J stays shown). `rg` answer: J's box is
  reachable — `update/journal/answers.rs:141` `recover_answered` opens the put-back preview with J left shown; so
  `journal_screen.rs:389` is **not** a dead branch.
- **G5** text-field test: setup-only. Diff vs base (`f800e8ce`): the `every_context()` loop with its
  `match context { Journal|Blame{lines:true}→'/', Stash→'s' }` became a `places` list (J/S states from
  `place_footers`, Blame = every `Blame{lines:true,..}` of `every_context()` as at base); `reached.push`/`drawn.push`
  format the name, not `{context:?}`. **No assert line changed** (diffed the function body against base).

### Other fixes this round
- `place_footers::map_with_more` set only a row's `complete`; the footer reads `MapData::complete`
  (`view/map.rs:84`), so `· partial` never showed. Now sets both; `doorway-map-120x36` shows `· +3 more · partial`.
- `doorway-stash-120x36` caption said `(Z)` / `Z/esc close`; the drawn bar is `esc close` and the key is `S`. Fixed.
- Floors (computed from `shed_footer`, shipped keymap, not separately measured): stash `⏎ apply  esc close  ? keys` 26
  cells; Map ready first+last+doorway 30, plus notes 50; below, the doorway alone.

### Captures (re-shot once with `GRAFT_UPDATE_SHOTS=1`, then unset)
Changed, all J/S/Map footers gaining `? keys` (S list also `b branch`): `m2-map-view.txt`, `m2-map-view-loading.txt`
(`esc cancels  ? keys`), `m3-journal-{80x24,160x48}`, `m3-journal-expanded`, `m3-stash-list`, `m3-stash-empty`,
`m7/journal-187x50`, `m7/journal-after-squash-187x44`, `m7/journal-{100x30,140x40,187x24}`,
`m7/journal-after-put-back-120x40` (the pinned-clock walk stops at its first drift, so these took four update runs
of `the_three_lost_work_journeys`), `m7/journal-expanded-120x40`, `m7/journal-find-187x50`,
`m5-hints-ladder` (Journal section removed — nothing replaces it: J is no hint context), `m8/doorway-journal-120x36`
(fixture now the merged journal; ends `J/esc close  ? keys`), `m8/doorway-blame-80x24` (header comment only; bar row
unchanged), `m8/doorway-marked-120x36` (caption only). New: `m8/doorway-stash-120x36`, `m8/doorway-stash-60x20`,
`m8/doorway-map-120x36`, `m8/doorway-marked-140x40`. E4: no `m4-05-*`, `m4-remote-keys`, `m6/fix-box-owns-the-bar-*`
change (git status). Raster PNGs regenerated byte-identical (no diff). No Map Empty/Failed capture exists. All ≤8 KB.
Bar rows the user's to judge (rule 6); colour unverified (rule 5).

### Touches (builder 2)
`stash_screen.rs` footer (one caller, its `render`); `map_screen/render.rs` `footer_line` (one caller); graft-ui
snapshots (3 Map); `update/tests.rs` (text-field setup, stash caption); `update/tests/place_footers.rs`; 02-keymap §2.9
one cell; both MAP.md. Keymap table / `keymap_ids.txt`, journal/undo, palette, `edge::context_of`, graft-ops: no diff
(`git diff --stat f800e8ce` over those paths lists only `update/journal/tests/minors.rs`, builder 1's exact-bar pin
gaining `? keys`). Ticket 15: one fewer context to rank; `shed_footer` is not the graph ladder.
