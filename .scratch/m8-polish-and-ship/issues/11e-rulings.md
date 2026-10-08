# 11e — rulings on the scout's open points (planner spec-only pass, 2026-10-08)

Status: draft. Amends `11e-doorway-footers-and-widths.md` F2 where it names these points; everything else in 11e
stands. Read with: `scouts/11e-footers.md`, 11e F2, 11d R1, ADR-0039 ("never a fragment"; doorway last thing dropped;
modal exception), ADR-0058 §4 (prompts are modal).

## Rulings

**E1 — S and Map get J's shedding rule; truncation is the defect.** "Their existing rule" in F2 assumed one; S
(`stash_screen.rs:189`, no width) and Map (`render.rs:416`, `set_line` truncates) have none, so today a narrow footer
draws a cut clause — and with `? keys` last, truncation would clip the doorway first. ADR-0039 forbids both. One rule
for all three footers (ux H4 consistency): J's existing cut (`journal_screen.rs:273-280`: drop whole clauses from the
right of the middle; first clause and the close clause kept longest), lifted into one crate-private function that J,
S and Map all call. J's output at every width is unchanged except for the doorway's reserved cells.
Narrow end, stated to the bottom: all clauses → middle shed → first + close → **doorway alone** → bare `?` → nothing.
Never a fragment.

**E2 — Map: keys, then the doorway, then the notes.** ` · +N more` and ` · partial` are not hints; they describe the
view (rows not shown; data incomplete). The doorway ends the **key** clauses (DESIGN §1.0 "last among the words" —
the hints' words), the notes follow with their ` · ` joins as today:
`j/k select  ⏎ zoom to commits  M/esc back  ? keys · +3 more · partial`.
Width: the reserve is doorway + notes; key clauses shed into the rest by E1. Below the reserve, drop order: `+N more`,
then `partial`, then the doorway's ladder (`?`, nothing). `partial` outlives `+N more` because it is the one that says
the data is wrong to trust; the doorway outlives both (ADR-0039: last thing a bar drops). Design completeness — at 48
cells `j/k select` + reserve fits (~37 cells); NOTES gives the measured floors.

**E3 — sub-states: one rule.** The doorway is drawn on every footer a place draws from **its own keys**, in every
sub-state; not where the footer is replaced by a prompt's line or blanked. Applied:

| Place, sub-state | Footer today (scout Q4) | Doorway |
|---|---|---|
| J ready, every row kind | own keys | yes |
| J not ready | `close` only | yes (`close  ? keys`) |
| J `/` find | `/typed` field | no — text field; `?` types |
| J `box_open` | blank | no |
| S ready / Loading / Empty / Failed | own keys | yes |
| S `s` save prompt | `⏎ save  ⇥ scope  esc back` | no — a prompt (ADR-0058 §4 modal; complete list; `?` would type) |
| S `box_open` | blank | no |
| Map Ready / Empty / Failed | ready footer | yes |
| Map Loading | `esc cancels` | **check-first** (below) |

Map Loading is a cancellable job, not a prompt; whether `?` answers there is not readable from the docs. §2.1 decides
it: a drawn key must work. So the builder presses `?` at Map Loading (`dispatch_key`) **before** writing the footer:
opens the palette → `esc cancels  ? keys`; swallowed or refused → no doorway, NOTES quotes the line that swallows it.
Do **not** change what `?` does at Loading in this ticket.

**E4 — the m4-05, m4-remote-keys and m6 shot hits are not Map footers.** Confirmed by reading the captures:
`m4-05-progress-footer-{80x24,160x48}` (:26, :50) and `m4-remote-keys.txt` (:79) are the **operation progress
footer** (`fetch all remotes · … · esc cancels`); `m6/fix-box-owns-the-bar-*` :12 and `m4-remote-keys.txt` :11 are
caption comments. None is in 11e's allowed-diff list. If any of them changes on re-shoot, that is an unexpected diff:
stop and report it.

**E5 — Map Failed / Empty and S Loading / Empty / Failed footers name keys that may have nothing to act on.** A
question, not a defect (rule 7: no line read). The coverage sweep below includes those states, so it answers it. A key
those footers draw that does nothing there → **stop**, report key + state + line; footer words are out of 11e's scope.

## Signatures (adds to 11e's)
```rust
// graft_ui::graph_screen::hints::fitting (beside `place_doorway`) — crate-private
/// Indexes of the clauses a place footer keeps in `room` cells (doorway and notes already reserved by the caller).
/// Drops whole clauses from the right of the middle; first and last kept longest; empty when first + last
/// do not fit. Never cuts a clause.
pub(crate) fn shed_footer(clause_widths: &[usize], sep: usize, room: usize) -> Vec<usize>;

// graft_ui::stash_screen — gains the width it is drawn at (one caller: its `Widget::render`, `:411`)
pub fn footer(&self, width: u16) -> String;
// graft_ui::map_screen::render — same (one caller: `render.rs:416`)
fn footer_line(&self, width: u16) -> Line<'static>;
```
`JournalScreen::footer` keeps its signature; its cut body becomes a call to `shed_footer`. Each screen already holds
`keys: &Keymap` (graft-ui MAP :265, :276) — `doorway_words(self.keys)`, no new parameter. NOTES quotes the final
signatures and their callers (`rg`).

## Acceptance (adds to 11e's; each tick names its test, at tick time)
- [x] (`graft_ui::graph_screen::hints::tests::shed_footer_drops_the_middle_first`, green) E1 one cut: `shed_footer` — keeps all when they fit; sheds right-of-middle first; first + last last; empty
  below them; monotone in `room`. (`graft_ui::graph_screen::hints::tests::shed_footer_drops_the_middle_first`)
- [x] (`graft_ui::stash_screen::tests::footer_never_draws_a_fragment`, `graft_ui::map_screen::tests::footer_never_draws_a_fragment`, green; red before per builder 1 NOTES) E1 no fragment, red first (fails on today's S and Map): S (rows, empty) and Map (ready) footers at every width
  20..=200 — every drawn clause is whole (each is one of the full clauses), the doorway is whole or bare `?` or absent,
  never cut. (`graft_ui::stash_screen::tests::footer_never_draws_a_fragment`,
  `graft_ui::map_screen::tests::footer_never_draws_a_fragment`)
- [x] (J tests listed in ticket NOTES builder 1 "the_journal_screen exact-bar pins", green) E1 J unchanged: J's existing footer tests green, edited only for the doorway's cells; NOTES lists each edit.
  (those tests, by name in NOTES)
- [x] (`graft_ui::map_screen::tests::the_doorway_ends_the_keys_and_the_notes_follow`, green) E2 order and drop: Map ready with `more > 0` and `partial`: footer reads keys, `? keys`, ` · +N more`,
  ` · partial` in that order at 80, 120, 200; at widths below the reserve, `+N more` goes before `partial`, `partial`
  before the doorway. `partial` keeps Warning ink (span style asserted; colour on screen unverified, rule 5).
  (`graft_ui::map_screen::tests::the_doorway_ends_the_keys_and_the_notes_follow`)
- [x] (`graft_ui::{journal_screen,stash_screen,map_screen}::tests::footer_ends_with_the_keys_doorway` at 48/80/120/200; `graft::update::tests::a_text_field_inside_a_place_draws_no_keys_doorway` setup-only edit per rulings-2 G5; `graft::update::tests::place_footers::a_box_over_a_place_draws_no_keys_doorway`, renamed) E3 table: every "yes" row draws the doorway at 48, 80, 120, 200 (extends 11e's F2 footer tests to J not-ready,
  S Loading/Empty/Failed, Map Empty/Failed); every "no" row draws none
  (`graft::update::tests::a_text_field_inside_a_place_draws_no_keys_doorway` green unedited;
  `…::a_preview_over_the_journal_draws_no_keys_doorway` gains the S `box_open` case — rename to
  `a_box_over_a_place_draws_no_keys_doorway` if it now covers both, NOTES says so).
- [x] (`graft::update::tests::place_footers::the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers`: opens the palette, footer `esc cancels  ? keys`) E3 check-first, Map Loading: `?` pressed at Loading via `dispatch_key` — outcome and line in NOTES; footer
  follows it. (`graft::update::tests::the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers`)
- [x] (`graft::update::tests::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn` over `place_footer_states`; Loading by the test above) E3 every drawn doorway answers: `…::the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn` covers
  J not-ready, S Loading/Empty/Failed, Map Empty/Failed (and Loading if drawn). (that test)
- [x] (`graft::update::tests::place_footers::every_key_a_place_footer_draws_is_answered`; stop raised by builder 1, ruled in rulings-2 G2) E5: `graft::update::tests::every_key_a_place_footer_draws_is_answered` (11e F2) includes S Loading/Empty/Failed
  and Map Empty/Failed; any no-op key → stop per E5.
- [x] (the diff: `git status` after the re-shoot lists none of them; ticket NOTES builder 2) E4: re-shoot diff lists no `m4-05-*`, `m4-remote-keys`, `m6/fix-box-owns-the-bar-*` change. (the diff, in NOTES)
- [x] (`graft::update::tests::capture_the_keys_doorway_for_docs_shots`: `doorway-map-120x36` shows `· +3 more · partial`; `doorway-stash-60x20`) Captures (adds to 11e's): `docs/shots/m8/doorway-map-120x36.txt` uses a fixture with `+N more` showing (E2's
  order visible); if none of the fixtures has one at 36 rows, shoot `doorway-map-more-120x24.txt` too. Narrow S:
  `docs/shots/m8/doorway-stash-60x20.txt` (E1's shed, no fragment). Bar rows his to judge (rule 6); colour unverified.

## Touches (adds to 11e's)
- `journal_screen.rs` cut body → `shed_footer`: J footer tests, J captures (already re-shot by 11e).
- `stash_screen.rs` `footer` callers (`rg 'footer\('` in graft-ui and graft — NOTES); `render.rs` `footer_line` callers.
- Operation progress footer (`esc cancels` in m4 shots): not touched — E4.
- 01-view-mockups §1.11 bar line: `? keys` last (11e docs box); no Map mock-up line exists to edit (checked: §1 has no
  Map section).
- Ticket 15: `shed_footer` is not the graph bar's ladder; 15 does not rank place footers. Say so in NOTES.
