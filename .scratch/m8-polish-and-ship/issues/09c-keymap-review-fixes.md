# G-keymap 1c — review fixes: a remap never takes a place's own keys; suspend is a row; one confirm id; sweeps that prove something

Status: draft (planner spec-only pass, 2026-10-04, from `docs/reviews/m8-09-keymap.md` Majors 1–5)
Build with: **Opus** — every rule below is decided here; correctness is held by named behavioural tests, not by eye.
Not Fable: the shape is fixed (fails "a wrong approach becomes structural").
Blocked by: 10, 10c (holds graft-ui hints, `Keymap` derives, `update/edge.rs`)
Needs: from 09 — `ContextId::{scope, is_text_field}`, `Keymap::{with_overrides, translate, in_context}`, `KeyWarning`;
from 09b — `keymap_from`, `context_of`, `dispatch_key`, `outside_edge`, `edge::unanswered`; from 10 — `Arc<Keymap>` in
`AppState`, hints keyed by `ActionId`.
Crate set: graft-ui (`keymap/*`, `tests/keymap_ids.txt`, hint lists naming the removed ids) + graft (`config/keys.rs`,
`update/tests/keymap_edge*`, `empty_state` tests, `view/status` test).

## Read first
- `docs/reviews/m8-09-keymap.md` Majors 1–5 (:58–148), Minors 2, 9, 12 (:158, :192, :200)
- This ticket's Decisions (whole). `09-keymap-table-and-translate.md` Scope 4–5 and NOTES "Decisions" 1–7
- `docs/design/02-keymap.md` §2.1 bullet `.` menu (:114–121), "Every binding is remappable" (:174–186), §2.3b `y`
  row (:282), §2.3c (:289)
- `docs/research/ux-principles.md` row 4 (:54)
- `crates/graft-ui/MAP.md` `## keymap`; `crates/graft/MAP.md` `update::edge`, `context_of`, `dispatch_key`,
  `AppState::suspend_on_ctrl_z`, `config_problems`
- Source after the maps: the claim chain `crates/graft/src/update/keys.rs:210–290` (read only), `keymap/resolve.rs`,
  `keymap/table.rs`, `update/tests/keymap_edge.rs`

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

**D4 — The behavioural sweep's oracle is behaviour, not a copy of `resolve`** (Major 3). Delete `acting_default`
(`keymap_edge.rs:160–171`). Expected = what the row's default chord does under the default keymap. Which rows a
place shadows comes from a hand-written `SHADOWED: &[(ContextId, &str /*id*/, &str /*chord*/, &str /*by id*/, &str
/*why: § or file:line*/)]`, never computed. graft-ui's `default_holder` (`keymap/tests/translate.rs:11–17`) may stay
for the pure sweep; the pinned D1 cases below are what catch an order bug there.

## Scope
In: D1–D4 as above, Minors 2, 9, 12. Out: Minors 1, 3–8, 10, 11 (conductor's); `?`/palette (11); docs (below).

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
// graft::config — keymap_from passes KeySwitches { suspend_on_ctrl_z: loaded.keys.suspend_on_ctrl_z }
```
`Keymap::default()` = switches off. No other new `pub` items.

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
  `confirm.yes = "c"` → `c` confirms in Preview, Confirm, Finish, Rewrite and the init box fixtures, `y` acts as F20
  there and still copies the SHA on the graph. `graph.merge = "M"` → `M` opens the preview, `M` again confirms.
  Override on `preview.confirm_merge` → `Follows`.
- [ ] **Major 3.** Sweep over `reachable(ctx)` for every context with a fixture (all rows, not own + Global). Overlays
  given rows to move over; pane cursors mid-list. Every case either **acts** (default press ≠ F20) or is listed with
  a reason (`SWALLOWED_BY_DESIGN` — Preview/Confirm/Banner swallows, §2.3c; `NO_EFFECT_IN_FIXTURE`; `SHADOWED`). A
  listed entry that acts, or a shadow that doesn't hold, fails the test. Test prints acting/listed counts; asserts
  acting ≥ the count at landing (written in the test with the date).
- [ ] **Major 4.** EmptyState swept like Major 3 through `outside_edge`, three fixtures: plain, init box open (closes
  Minor 12: `y`/`n`/`esc` under a remap), prompt open (every key typed). F2 sanitising pinned: the reviewer's probe
  `"\u001b[31mmerge"` → F2 draws `graph.␛[31mmerge is not an action` (Minor 2).
- [ ] `keymap_edge.rs` split under 400 lines per file (Minor 8, since it is rewritten).
- [ ] MAP.md: graft-ui `borrows`, `KeySwitches`, `reachable`, `Clash`, new variants, removed ids; graft `keymap_from`.
- [ ] Captures: default config → no capture changes. `docs/shots/m8-keymap-config-problem-120x36.txt` re-shot only if
  J8's words are drawn in it; NOTES names it. Colour unverified (rule 5).

## Touches
- graft-ui `keymap/{resolve,table,warning}.rs`, `keymap.rs`, tests, golden list. 10's hint lists and `NEVER_HINTED` /
  `bound_actions` naming `finish.commit`, `rewrite.start`, `empty_state.confirm`, `preview.confirm` → `confirm.yes` /
  the split rows (grep each id; NOTES lists hits). `table_check`.
- graft `keymap_from`, every `with_overrides` caller (grep), the edge (logic unchanged; check `Unbound` path still
  matches), 09b's sweeps and J8 tests, `empty_state` tests.
- Later tickets: **11** must show borrowed rows in `?` (Preview/Finish/Rewrite list `y confirm`) — conductor amends
  11. **18** (`empty_state.clone` `c`) meets `confirm.yes` in EmptyState's order: warned, not blocked. **23** man page:
  `app.suspend` "only with `suspend_on_ctrl_z = true`".
- Journal/undo: untouched (keys only). Mouse: not translated (unchanged).

## Conductor (not the builder's)
- **Doc amendments** to apply (planner text): 02-keymap.md §2.1 "Every binding is remappable", append: "A line acts
  where its section is live and never over a place's own keys: a place answers its own rows and the keys it handles
  itself (movement in panes and the `.` menu) before anything it falls through to. Each line with a problem is one
  problem, naming every place it changes. `confirm.yes` is the one confirm key (§2.3b). With `suspend_on_ctrl_z`,
  `⌃Z` is `app.suspend` and undo keeps `u`." ADR-0055: "Amended 2026-10-04 (09c): resolution order, grouped
  warnings, `app.suspend`, one confirm id."
- **KNOWN_GAPS**: `diff.word_diff` / `blame.heatmap` draw only as colour → **unverified** row (rule 5; no test can
  close it). 09b tick 1's EmptyState part → **broken (test missing)** row until this ticket's Major 4 box is green.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~90k: graft-ui rules + warnings + rows ~35k; graft sweeps rewrite ~40k; EmptyState + F2 ~10k. Over the line: land
graft-ui, `keymap_from` and the Major 1/2/5 pinned tests, commit at the seam, report `blocked: sweeps`; the conductor
gives Majors 3–4 to an Opus 09d.
