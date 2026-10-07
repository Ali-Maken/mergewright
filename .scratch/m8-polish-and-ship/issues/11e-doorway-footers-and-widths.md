# G-palette 1e — `? keys` on the journal, stash and Map footers (one bar per place); R2's measured widths become the spec

Status: draft (planner spec-only pass 2026-10-08; replaces 11d's open R2 box and its NOTES finding "R1 does not reach
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
- [ ] **Check first**: scout Q2's answer re-read at base; if a reachable drawer of the three contexts draws something
  other than the place's footer **and** the user can see both at once, stop and report the layout with the line.
- [ ] F1: `graft_ui::graph_screen::hints::tests::a_marked_row_keeps_undo_and_search_before_the_doorway` pins first-drawn
  width 129 (plain, both dividers) and 137 (branch tip, both dividers) as exact values, monotone 40..=240, last when
  drawn. Amend only the pins; the rest unchanged.
- [ ] F1 capture: `docs/shots/m8/doorway-marked-140x40.txt` (EvilMerge fixture as 11d's 120 shot; bar ends `? keys`).
  If the fixture row's first-drawn width is over 140, shoot at 160x48 and name it so. `doorway-marked-120x36.txt` stays:
  NOTES gives its contact-sheet caption "at 120 a marked row draws no `? keys` (R2's measured cost)".
  (`it::hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots`, extended)
- [ ] F2 one source: `doorway_words` — `help` bound → `? keys`; `help = "none"` → `⌃P keys`; both none → `None`;
  remapped `help` → his label. `spelled_bar`'s existing doorway tests green unedited.
  (`graft_ui::graph_screen::hints::tests::doorway_words_follows_his_keymap`)
- [ ] F2 footers, red first: J (every row kind `journal_view` makes: `⏎` worded, none, a live row), S (rows, empty),
  Map (ready) at 48, 80, 120, 200 cells: the footer ends with `? keys` (or leads its bare keys); `⌃P keys` when `help`
  unbound; nothing when both unbound — no fragment, no stray separator; once drawn, drawn at every wider width.
  (`graft_ui::journal_screen::tests::footer_ends_with_the_keys_doorway`, `…stash_screen::tests::…`,
  `…map_screen::tests::…` — or one table test in graft if the footers are built there; NOTES names it)
- [ ] F2 no doorway in sub-states: J find, J `box_open`, S save prompt (and Map loading/failed per Q4).
  `graft::update::tests::a_text_field_inside_a_place_draws_no_keys_doorway` green unedited, plus `box_open` case
  (`…::a_preview_over_the_journal_draws_no_keys_doorway`).
- [ ] F2 deletion: `rg -n 'HintContext::(Journal|Stash|Map)\b' crates` empty; `every_context_reaches_every_shape`,
  `coverage_slot` exhaustive, green. (those tests; the `rg`)
- [ ] F2 coverage moved, red first (a footer clause with no answering key fails it — show by mutation, revert):
  every key each J / S / Map footer draws, pressed at that place via `dispatch_key`, acts (not a no-op, not a refusal
  for a reason the footer did not show). (`graft::update::tests::every_key_a_place_footer_draws_is_answered`)
- [ ] `?` from each footer opens the palette: `graft::update::tests::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn`
  green (its J/S/Map states now the footers').
- [ ] `graft::update::tests::every_key_the_bar_draws_is_answered`, `…::every_bound_action_is_hinted_live_or_is_named_here`
  green. If the second fails because a J/S/Map key was hinted only by a deleted list: the footer draws it → add the
  footer as a hint source in that test; it does not → **stop** (a §2.1 spec change), report the key.
- [ ] Captures: `GRAFT_UPDATE_SHOTS=1` once, unset, green. Allowed diffs: a J/S/Map footer gaining `? keys` and the
  words its existing rule sheds for it; `m5-hints-ladder`'s Journal rows (generator loses the variant — NOTES says what
  replaces them). Each listed in NOTES. `docs/shots/m8/doorway-journal-120x36.txt` now shows `? keys`; new
  `doorway-stash-120x36.txt`, `doorway-map-120x36.txt`; `help-at-map-120x36.txt` re-shot. ≤200 KB each. Bar rows the
  user's to judge (rule 6); **colour unverified** (rule 5).
- [ ] Docs: 01-view-mockups §1.10 :376 bar and §1.11's bar line (if it draws one) gain `? keys` last; graft-ui MAP
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
