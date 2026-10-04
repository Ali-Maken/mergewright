# G-keymap 1c — review fixes: a remap never takes a place's own keys; suspend is a row; one confirm id; sweeps that prove something

Status: draft (planner spec-only pass, 2026-10-04, from `docs/reviews/m8-09-keymap.md` Majors 1–5; amended 2026-10-04
after 10c: rulings on its R4 and Unbound deviations folded in; Majors 3–4 (D4) and 10c's five allow-listed literals
split off to `09d-keymap-sweeps-and-literals.md` — 09c alone was ~90k)
Build with: **Opus** — every rule below is decided here; correctness is held by named behavioural tests, not by eye.
Not Fable: the shape is fixed (fails "a wrong approach becomes structural").
Blocked by: none (10/10c merged)
Needs: from 09 — `ContextId::{scope, is_text_field}`, `Keymap::{with_overrides, translate, in_context}`, `KeyWarning`;
from 09b — `keymap_from`, `context_of`, `dispatch_key`, `outside_edge`, `edge::unanswered`; from 10 — `Arc<Keymap>` in
`AppState`, hints keyed by `ActionId`; from 10c — `HintKey::Fixed`, `preview_box`'s `again: Option<(ActionId, &'static
str)>` and `Again`, tests `preview_box::tests::again_names_a_confirm_chord_only_while_it_confirms`,
`update::tests::keymap_edge::the_preview_names_its_again_key_exactly_while_it_confirms`,
`graph_screen::hints::tests::a_fixed_key_is_drawn_only_where_keys_go_on_as_typed`,
`x_typed_into_every_field_is_an_x_with_merge_on_x`.
Crate set: graft-ui (`keymap/*`, `tests/keymap_ids.txt`, hint lists naming the removed ids, `preview_box`) + graft
(`config/keys.rs`, preview `again` callers, a **new** `update/tests/keymap_rules.rs` for this ticket's pinned tests —
`keymap_edge.rs` itself is 09d's to rewrite and split). Not the five literal widgets (09d).

## Read first
- `docs/reviews/m8-09-keymap.md` Majors 1, 2, 5 (:58–148), Minor 9 (:192)
- This ticket's Decisions and **Rulings on 10c** (whole). `09-keymap-table-and-translate.md` Scope 4–5 and NOTES
  "Decisions" 1–7. `10c-keymap-spell-footers.md` NOTES "Deviations" 1–2
- `docs/design/02-keymap.md` §2.1 bullet `.` menu (:114–121), "Every binding is remappable" (:174–186), §2.3b `y`
  row (:282), §2.3c (:289)
- `docs/research/ux-principles.md` row 4 (:54)
- `crates/graft-ui/MAP.md` `## keymap`, `preview_box`; `crates/graft/MAP.md` `update::edge`, `context_of`,
  `dispatch_key`, `AppState::suspend_on_ctrl_z`, `AppState::text_field_open`, `config_problems`
- Source after the maps: the claim chain `crates/graft/src/update/keys.rs:210–290` (read only), `keymap/resolve.rs`,
  `keymap/table.rs`, `update/state.rs:873–887` (read only), `update/preview/keys.rs:78–88`

## Decisions (planner; not the builder's)

**D1 — A `[keys.<ctx>]` line acts where that context is live, never over a place's own keys** (Major 1).
Options: (a) global take, warning naming every place lost; (b) scoped: each place answers its own rows and the
rows it answers itself in the chain before anything it falls through to. **Take (b), and warn as (a) would.** Why:
the section he wrote names the place; `j`/`k` mean move in every pane (ux-principles row 4); (a) would make `j` merge
inside the `.` menu, whose whole vocabulary is movement (§2.1 :114–121). Cause, not symptom: `scope()` puts Graph
ahead of Global in nested places, while the chain answers movement there first (`update/keys.rs:216`, `:274`
before `:281`).
- Resolution order in `ctx`: **own rows → `ctx.borrows()` → the rest of `ctx.scope()` in order**. `borrows()` lists
  the rows of other contexts this place answers itself ahead of what it falls through to, read off the chain, one
  `// file:line` comment per context. Expected at least: `Menu` → `app.down/up/open/back`; `Changes`, `Diff`, `Map`,
  `Journal`, `Stash`, `Blame`, `History` → the Global movement rows they claim (`down/up/top/bottom/page_*`, and
  `open`/`back` where claimed). The builder takes the list from the code, not from this line.
- `translate`'s "resolves over defaults" uses the same order (over the base chords, D2).
- **Warnings are computed, not pairwise.** For every context C and chord k, compare resolution over base vs effective:
  (i) **take** — a base holder D still listing k loses k in C to his line's action → clash `{k, C, kept: A, lost: D}`;
  (ii) **lose** — his line's action A doesn't get k in its own context, or (A Global) in any context → clash
  `{k, C, kept: holder, lost: A}`. Not warned: a non-Global line losing in a place it only falls through to (J8 stays
  1 problem). **One problem per config line**, every place named (settles Minor 9).
- He sees, `[keys.graph] merge = "j"`: graph — `j` opens the merge preview, `↓` moves; changes pane, diff, `.` menu,
  J/S/M overlays, blame, history — `j` moves. Bottom row `config: 1 problem · ? shows them`; F2 reads
  `j given to graph.merge — move down keeps ↓ on the graph`.

**D2 — Suspend is a row, `app.suspend`, live only while `suspend_on_ctrl_z` is on** (Major 2). Options: (a) the edge
passes `⌃Z` raw when the switch is on; (b) a row. **Take (b).** Why: (a) leaves suspend out of `?` and silently
beats any line he puts on `⌃Z`; (b) gets collisions, `?` and moving it for free, dispatchers unchanged
(`update/keys.rs:230` still suspends on `⌃Z` with the switch on).
- The keymap carries the switch. **Base chords** (what translate calls "defaults"): switch off — `app.suspend` none,
  `app.undo` `[⌃Z, u]`; switch on — `app.suspend` `[⌃Z]`, `app.undo` `[u]`.
- A `[keys.app] suspend` line with the switch off → `KeyWarning::SwitchOff`, line not applied.
- He sees: switch on + `undo = "u"` → `⌃Z` suspends, no problem. Switch on alone → as today, and the bar and `?`
  spell undo `u` (today they would spell `⌃Z`, which suspends). Switch on + `suspend = "ctrl+s"` → `⌃S` suspends, `⌃Z`
  is an unanswered key. Switch off + a suspend line → `app.suspend does nothing while suspend_on_ctrl_z is off`.
  Switch on + `[keys.graph] fetch = "ctrl+z"` → `⌃Z given to graph.fetch — suspend has no key on the graph`.

**D3 — One confirm key is one id: `confirm.yes`** (Major 5, the question). Options: (a) five ids; (b) one Global
`app.confirm`; (c) one `confirm.yes`, borrowed by the places that confirm. **Take (c).** Why: (a) lets one line break
§2.3b's "one confirm key" with no warning; (b) puts confirm in every context, so `app.confirm = "c"` would warn on
the graph, changes pane, diff, … where confirm means nothing. Ids freeze at release (ADR-0055 §2): decide now.
- Delete rows `finish.commit`, `rewrite.start`, `empty_state.confirm`; `preview.confirm` loses `y`. `Preview`,
  `Finish`, `Rewrite`, `EmptyState` borrow `confirm.yes` (D1's mechanism). On the graph `y` stays `graph.copy_sha`.
- **The repeat key follows the opener.** `preview.confirm` (`[y, m, r]`) splits into `preview.confirm_merge` (base `m`)
  and `preview.confirm_rebase` (base `r`); their effective chords are `graph.merge`'s / `graph.rebase`'s. An override
  on them → `KeyWarning::Follows`, not applied. Why: with `merge = "M"`, `M M` must still confirm (§2.3b :281); today
  the second `M` is swallowed.
- He sees, `[keys.confirm] yes = "c"`: `c` confirms every preview, box, finish list, rewrite editor and the init box;
  `y` there is unanswered; hints read `c <verb>` (10 spells from the row). One line, not five.
- Pre-release: removed ids get no alias; NOTES says so (the golden test's alias message applies after release).
- A borrow acts only where translate runs: inside a text field (`AppState::text_field_open`) `confirm.yes` is not
  consulted and `y`/`c` type themselves (R4 below).

**D4 — the behavioural sweep's oracle** moved to `09d-keymap-sweeps-and-literals.md` (verbatim there).

## Rulings on 10c (planner, 2026-10-04)

**R4 (10c deviation 1) — accepted; R4's second half is withdrawn.** R4's premise was wrong: it read `context_of`
(Finish) as what acts in Subject/Body, but `AppState::text_field_open()` (`update/state.rs:873–887`, 10c's line;
graft MAP `text_field_open`) passes every key as typed there before translate, pinned on main by
`x_typed_into_every_field_is_an_x_with_merge_on_x`. Spelling his remap there would name a key that types a letter.
So Subject/Body hints stay `Fixed`, spelled from the shipped keymap — the same rule as every text field (10's
decision 5). **Answer to 10's question: yes, typed text in Subject/Body passes untranslated.** What is still
missing is the forward half of the invariant — 10c tested "Fixed ⇒ text field", not "text field ⇒ only Fixed":
- **Rule:** wherever `text_field_open()` is true, the bar and footer draw only `Fixed` keys and `Word`s — never a
  `Key`/`EveryChord` (a remappable key spelled where no remap acts is a lie).
- D3 knock-on: deleting `finish.commit` must not leave a Subject/Body hint naming `confirm.yes` (`y` types there).
  Each Subject/Body hint names the row the field's own handler answers; the builder reads it off the handler and
  names the line in NOTES.

**Unbound (10c deviation 2) — the code reading is right; D3 makes the original acceptance true.** Today the box's
`m merge`/`r rebase` slot is a `preview.confirm` chord (`update/preview/keys.rs:78–88`, 10c's line), so
`graph.merge = "none"` left it correct. After D3 the slot is `preview.confirm_merge` / `preview.confirm_rebase`,
whose chords follow `graph.merge` / `graph.rebase`. So:
- `again` carries `preview.confirm_merge` / `preview.confirm_rebase` (not `preview.confirm`); `Again` draws the slot
  exactly while that id has a live chord, spelled from it.
- `graph.merge = "none"` → the merge preview (opened from the `.` menu) draws no `m merge` part and no `—`; `y merge`
  (from `confirm.yes`) stays. `graph.merge = "M"` → the box draws `M merge`, and `M` confirms.
- 10c's case `preview.confirm = ["y"]` is gone with the id: an override on `preview.confirm_merge` is `Follows`, not
  applied, and the box still draws the opener's key. Re-point 10c's two `again` tests; don't delete their names.

## Scope
In: D1–D3 as above, Minor 9, the R4 and Unbound rulings. Out: D4, Majors 3–4, Minors 2, 8, 12, 10c's five literals
(09d); Minors 1, 3–7, 10, 11 (conductor's); `?`/palette (11); docs (below).

## Signatures
```rust
// graft-ui::keymap
impl ContextId { pub fn borrows(self) -> &'static [ActionId]; }        // D1; empty for Graph, Global, text fields
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeySwitches { pub suspend_on_ctrl_z: bool }                 // D2
impl Keymap {
    pub fn with_overrides(lines: &[(String, Vec<String>)], switches: KeySwitches) -> (Keymap, Vec<KeyWarning>);
    pub fn reachable(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])>; // resolution order: own, borrows, scope
}
pub enum KeyWarning {
    UnknownAction(String), BadKey { action: String, text: String },
    Collision { line: ActionId, places: Vec<Clash> },                  // one per config line (D1)
    Reserved { action: String, key: KeyChord }, Fixed(ActionId),
    SwitchOff { action: ActionId, switch: &'static str },              // D2
    Follows { action: ActionId, leader: ActionId },                    // D3
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clash { pub key: KeyChord, pub context: ContextId, pub kept: ActionId, pub lost: ActionId }
// graft-ui::preview_box — shape unchanged from 10c; the id is now preview.confirm_merge | preview.confirm_rebase
pub again: Option<(ActionId, &'static str /* verb: "merge" | "rebase" */)>,
// graft::config — keymap_from passes KeySwitches { suspend_on_ctrl_z: loaded.keys.suspend_on_ctrl_z }
```
`Keymap::default()` = switches off. No other new `pub` items. 09d calls `reachable` and `borrows`.

## Acceptance (each tick names its test, at tick time)
- [ ] **Major 1.** graft-ui: `graph.merge = "j"` → `translate(C, j) == Pass` for C in every context with `app.down`
  in `borrows()`; `translate(Graph, j) == Default(m)`; exactly one warning, one clash, on Graph. graft, through
  `update`: `j` moves in Changes, Diff, Menu, Map, Journal, Stash, Blame, History fixtures after `graph.merge = "j"`
  (the reviewer's missing test). Cross-remap: for each context with borrows, each borrowed row B and one row of each
  context it falls through to moved onto B's first chord → B still acts there. `app.undo = "o"` → one problem naming
  graph, resolver, start screen (Minor 9). J8 still `config_problems.len() == 1`; its words pinned.
- [ ] **Major 2.** Four pairs pinned (D2 "He sees"), graft through `load_from` + `update`: switch on + `undo = "u"` →
  `⌃Z` emits `Command::Suspend`, zero problems; switch on alone → `spell(app.undo) == "u"`; switch on +
  `suspend = "ctrl+s"` → `⌃S` suspends, `⌃Z` acts as F20; switch off + suspend line → one `SwitchOff` problem.
- [ ] **Major 5.** Golden list edited (−4 ids, +`app.suspend`, `preview.confirm_merge`, `preview.confirm_rebase`).
  `confirm.yes = "c"` → `c` confirms in Preview, Confirm, Finish (Files focus), Rewrite and the init box fixtures, `y`
  acts as F20 there and still copies the SHA on the graph. `graph.merge = "M"` → `M` opens the preview, `M` again
  confirms. Override on `preview.confirm_merge` → `Follows`.
- [ ] **R4.** graft, through `update` + the view's hint path: (a) `confirm.yes = "c"`, finish summary, Subject focus:
  typing `c` then `y` puts `cy` in the subject and commits nothing; Files focus: `c` commits. (b) For every fixture
  where `text_field_open()` is true (finish Subject, finish Body, commit editor, picker, palette, credential, name
  prompt, stash save prompt, the `r` field of the rewrite editor, search and journal find prompts — the builder
  takes the list from `text_field_open`'s body and NOTES any it adds), the hints drawn hold no `HintKey::Key` /
  `EveryChord`. 10c's `a_fixed_key_is_drawn_only_where_keys_go_on_as_typed` stays green.
- [ ] **Unbound.** `graph.merge = "none"`, merge preview opened from the `.` menu → the box draws no `m merge` part and
  no `—`; `y merge` drawn; extend 10's `merge_unbound_is_drawn_nowhere_and_its_menu_row_names_no_key`.
  `graph.merge = "M"` → box draws `M merge`. 10c's two `again` tests re-pointed to `preview.confirm_merge` and green.
- [ ] MAP.md: graft-ui `borrows`, `KeySwitches`, `reachable`, `Clash`, new variants, removed ids, `again`'s id;
  graft `keymap_from`, `text_field_open` (the R4 rule).
- [ ] Captures: default config → no capture changes. `docs/shots/m8-keymap-config-problem-120x36.txt` re-shot only if
  J8's words are drawn in it; NOTES names it. Colour unverified (rule 5).

## Touches
- graft-ui `keymap/{resolve,table,warning}.rs`, `keymap.rs`, tests, golden list. 10's hint lists and `NEVER_HINTED` /
  `bound_actions` naming `finish.commit`, `rewrite.start`, `empty_state.confirm`, `preview.confirm` → `confirm.yes` /
  the split rows (grep each id; NOTES lists hits). `table_check`. `preview_box` (`Again`) and its graft callers
  (`preview/keys.rs`, `boxes.rs`, `preview.rs`, amend_into tests, put_back_box — 10c's list).
- graft `keymap_from`, every `with_overrides` caller (grep), the edge (logic unchanged; check `Unbound` path still
  matches), 09b's J8 tests, the finish summary's Subject/Body hint list (R4).
- `keymap_edge.rs`: tests there that name a removed id are edited in place (minimum); the rewrite and split are 09d's.
- Later tickets: **09d** sweeps over `reachable`. **11** must show borrowed rows in `?` (Preview/Finish/Rewrite list
  `y confirm`) — conductor amends 11. **18** (`empty_state.clone` `c`) meets `confirm.yes` in EmptyState's order:
  warned, not blocked. **23** man page: `app.suspend` "only with `suspend_on_ctrl_z = true`".
- Journal/undo: untouched (keys only). Mouse: not translated (unchanged).

## Conductor (not the builder's)
- **Doc amendments** to apply (planner text): 02-keymap.md §2.1 "Every binding is remappable", append: "A line acts
  where its section is live and never over a place's own keys: a place answers its own rows and the keys it handles
  itself (movement in panes and the `.` menu) before anything it falls through to. Each line with a problem is one
  problem, naming every place it changes. `confirm.yes` is the one confirm key (§2.3b). With `suspend_on_ctrl_z`,
  `⌃Z` is `app.suspend` and undo keeps `u`. Inside a text field no line acts: keys type themselves." ADR-0055:
  "Amended 2026-10-04 (09c): resolution order, grouped warnings, `app.suspend`, one confirm id; a text field
  (`text_field_open`) passes keys untranslated even inside a remappable place."
- **KNOWN_GAPS**: `diff.word_diff` / `blame.heatmap` draw only as colour → **unverified** row (rule 5; no test can
  close it). 09b tick 1's EmptyState part → **broken (test missing)** row until 09d's Major 4 box is green.
- 10c's `[~]` R4 and Unbound boxes close when this ticket's R4 and Unbound boxes tick.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~62k: graft-ui rules + warnings + rows ~35k; Major 1/2/5 pinned tests through `update` ~18k; R4 + Unbound ~9k.
Over the line: land graft-ui and `keymap_from`, commit at the seam, report `blocked: pinned tests`.
