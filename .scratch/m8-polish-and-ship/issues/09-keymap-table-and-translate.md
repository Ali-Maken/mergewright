# G-keymap 1a — one key table and its translate (graft-ui)

Status: done (2026-10-04; re-specced 2026-10-04 after scout `scouts/09-keymap-table.md`; split: the app edge is `09b`)
Build with: **Fable (signature)** — `ActionId`s become his config's API (never renamed after release), the table's
shape and the scope/resolution rule are what bar, `?`, palette, tour and man page read; wrong here is structural
(ADR-0055). Scout used: `scouts/09-keymap-table.md`.
Blocked by: 06, 07
Needs: none in code. `09b-keymap-edge-and-config` needs every signature below.

## Read first
- `docs/adr/0055-keys-are-one-table-and-remaps-translate-at-the-edge.md` (whole)
- `docs/design/02-keymap.md` §2.1 "modal panes" list (:100–157) and "Every binding is remappable" (:174), then every
  table §2.2–§2.18 (tables only; the prose under one only where a row points at it)
- `docs/design-notes/m8-polish-and-ship.md`: Interfaces `graft-ui::keymap` (this spec supersedes it where they differ,
  listed under "Differs from the note")
- `.scratch/m8-polish-and-ship/scouts/09-keymap-table.md` (whole, one page)
- `crates/graft-core/MAP.md`: `KeyPress`, `KeyCode`, `Modifiers`
- `crates/graft/MAP.md`: `update::keys` files, `bound_actions`; then source `update/keys/bound.rs` and the claim chain
  in `update/keys.rs` `key_event` and `update/mod.rs:1101` — **read only; this ticket changes no graft dispatch**

## Scope
In (crate set: `graft-ui`, plus one new `#[cfg(test)]` file in `graft`):
1. **Doc ↔ code walk, first, into NOTES.** For every row of §2.2–§2.18: does the code answer it? Two lists: **doc-only**
   (doc row, code doesn't answer) and **code-only** (code answers, no doc row), each with its § or file:line. They are
   doc questions for the conductor, not build choices. What enters the table:
   - doc row the code answers → a row;
   - code-only key → a row (else it can't be remapped and a collision with it is silent), id from the code's verb;
   - doc-only row an M8 ticket builds (`?`, `Ctrl+P`/`:` → 11; empty state `c` → 18, `t` → 19) → a row, so ids are
     decided once, here; listed in NOTES as 09b's `NOT_YET_ANSWERED` with the ticket;
   - doc-only row no ticket builds (e.g. §2.9 `b`, "planned, not yet built") → **no row**; NOTES only.
2. **`defaults()`**, the table:
   - id `<context>.<verb>`, snake_case, from the doc's words; the prefix is `ContextId::config_name`.
   - **One action per verb.** Opposite verbs on one doc row (`g`/`G`, `u`/`U`, `n`/`N`, `[`/`]`, `<`/`>`,
     `Tab`/`Shift+Tab`) are two actions. Synonyms (`Ctrl+P` and `:`; `Ctrl+Z` and `u`; `↓` and `j`) are one action.
   - **A key the doc repeats with the same verb in a nested place is one row**, in the outermost context holding it
     (§2.4 `c`/`a` "same verbs, same keys" → `graph.commit`/`graph.amend`; blame `w` "§2.5's meaning" →
     `diff.ignore_whitespace`; banner `q` → `app.quit`; empty state `↑↓ jk`, `⏎`, `?`, `q` → `app.*`). Nested places
     reach it through `scope()`. Same key, different verb (resolver `o`, graph `o`) → separate rows.
   - `label`: the doc's action words as a palette row reads them (`Merge selected branch into current`). `search`:
     hidden words only where the doc names them (§2.12 `aside` → stash); else empty.
   - `app.quit_now` (`Ctrl+C`) is a row. It can't be moved, and nothing can take `Ctrl+C`.
3. **`KeyChord`** reuses `graft_core::event::KeyCode` (no second key enum; scout: it has `BackTab`, `Insert`, space as
   `Char(' ')`). Normal form: a `Char` carries its case with `shift: false`; `Shift+Tab` is `BackTab`. A press with
   `command` held is never a chord.
   - `parse`: `m`, `M`, `shift+m` (= `M`), `ctrl+p`, `alt+up`, `shift+tab`, `enter`, `esc`, `tab`, `space`, `up`,
     `down`, `left`, `right`, `pgup`/`pageup`, `pgdn`/`pagedown`, `home`, `end`, `backspace`, `delete`, `insert`,
     `f1`–`f12`; names case-insensitive, a single character case-sensitive. It also parses every `label()` (`⌃P`, `⏎`,
     `⇥`): he may copy a key from `?` into his config. `none` is not a chord (the override reader handles it).
   - `label()`: **identical to how `bound_actions` and the hint tables spell the key today** (`⌃A`, `⏎`, `esc`,
     `space`, `↑`, …) — take the spellings from the code, don't invent. Normalise `Ctrl+Shift+<letter>` the way the
     input decoder delivers it (check graft-core's decoding; say which in NOTES).
4. **`ContextId`** fixed to the doc and the scout: adds `Confirm` (§2.3c), `CommitEditor` (§2.10), `Credential`
   (§2.14). `Tour` has no rows (§2.18: the tour's `Esc` is `app.back`); kept for 11/19.
   - `config_name`: `app` (Global), `graph`, `map`, `preview`, `confirm`, `changes` (§2.4), `diff`, `resolver`,
     `operation` (Banner — §2.1's own example `operation.continue`), `finish`, `rewrite`, `picker`, `blame`, `stash`,
     `commit` (CommitEditor), `journal`, `history`, `palette`, `empty_state`, `tour`, `credential`, `menu`.
   - `scope()`: where a key pressed in this place may still land, **innermost first, `Global` last**. It may
     over-include (a translated default the code ignores there does what pressing the default does: nothing); it must
     never under-include (a remap would fail there). Not for help: `?` lists `in_context`.

     | ctx | scope |
     |---|---|
     | Graph | Graph, Diff, Global |
     | Changes | Changes, Diff, Graph, Global |
     | Diff | Diff, Graph, Global |
     | Map, Journal, Stash | self, Graph, Global |
     | Menu | Menu, Changes, Diff, Graph, Global |
     | Blame, History | self, Diff, Global |
     | Preview, Confirm, Resolver, Banner, Finish, Rewrite, Picker, CommitEditor, Credential, Palette, EmptyState | self, Global |
     | Tour, Global | Global |

     A key the walk shows live outside this table → widen the scope and say so in NOTES.
   - `is_text_field()`: `Picker`, `Palette`, `CommitEditor`, `Credential`. Their rows are **fixed**: in the table (for
     `?`, the man page), never remapped.
5. **`Keymap`** — ADR-0055 §3 made exact:
   - **Effective chords**: an override replaces the action's whole default list (`app.down = "n"` drops `↓`; to keep it,
     `["n", "down"]`). `"none"` alone empties it; `"none"` inside a list → `BadKey`.
   - **Resolve a chord in a scope**: walk the scope inner → outer; the first context with an action holding the chord
     wins. Inside one context an override beats a default; two overrides on one chord → the earlier line keeps it.
   - **`translate(ctx, pressed)`**: `Ctrl+C` → `Pass`. Resolve `pressed` over effective chords in `ctx.scope()` → A.
     - No A: `Unbound` if `pressed` is a default chord of any action in the scope, else `Pass`.
     - A found: D = `pressed` if it is one of A's defaults and resolves **over defaults** to A in this scope, else A's
       first default that does (D is what the dispatcher will act on as A). No D (A is shadowed here) → `Unbound`;
       D == `pressed` → `Pass`; else `Default(D)`.
   - **Warnings, never fatal, in `lines` order**: unknown action; bad chord; override of a fixed row → `Fixed`; `Ctrl+C`
     given to anything, or `app.quit_now` moved → `Reserved`; **collision** when one chord ends up on two actions in
     one context, or between `Global` and any context (Global is live everywhere): `kept` resolves it there, `lost`
     loses that chord in that context. Shadowing between two non-Global contexts that only over-inclusion brings
     together is **not** warned — J8 must read 1 problem: `[keys.graph] merge = "M"` also meets `changes.mergetool`
     and `map.close` on `M`, where the inner one keeps it, silently.
   - `KeyWarning::words` (lowercase, the app's key spelling), e.g. `M given to graph.merge — map view has no key`;
     `u given to graph.merge — undo keeps ⌃Z on the graph`; `o given to app.undo — in the resolver o stays take ours`;
     `"ctrl+" is not a key (graph.merge)`; `grpah.merge is not an action`; `⌃C always quits — not given to
     graph.merge`; `picker.switch can't be remapped — keys in a text field stay as they are`.
   - `spell`: the first effective chord's label, `—` when none. `chords`: effective chords. `in_context`: that
     context's own rows only, table order, with effective chords.
6. **Tests** (graft-ui unless said):
   - golden id list `crates/graft-ui/tests/keymap_ids.txt`, one id per line, sorted; fails on any difference; the
     failure says *a rename breaks his config — keep the old id as an alias (ADR-0055 §2)*. An addition edits the file
     in the same commit.
   - ids unique, match `^[a-z_]+\.[a-z0-9_]+$`, prefix == the row's `config_name`.
   - every default round-trips both ways: `parse(&c.label().0) == Ok(c)` and `parse` of its canonical text; bad text →
     `BadKey`.
   - each warning kind produced and worded (one test each).
   - **pure translate sweep**: one keymap remapping every non-fixed action except `app.quit_now` to its own free chord
     (no default uses it: `alt+<letter>`, `alt+f<n>`, …). For every `ContextId` and every action with a default that
     resolves to it in that scope: `translate(ctx, free) == Default(D)` (or `Pass` is impossible here, every default
     having moved), and every default chord in the scope → `Unbound`. Default keymap: every default chord → `Pass`
     in every context.
   - J8, table level: `[("graph.merge", ["M"])]` → exactly one warning, `Collision { key: M, context: Graph, kept:
     graph.merge, lost: graph.map }`; `translate(Graph, M) == Default(m)`; `translate(Graph, m) == Unbound`;
     `translate(Changes, M) == Pass` (mergetool keeps it).
   - **in graft**, new `crates/graft/src/update/keys/table_check.rs` (`#[cfg(test)]`, the only graft edit):
     `every_bound_key_has_a_row` — each key `bound_actions()` spells (`j/k` split on `/`) equals `label()` of a default
     chord of some row. No context check (`bound_actions` has none, scout); 09b checks contexts by pressing.
Out:
- Graft's dispatch, config, `AppState`, status row (09b). Hint/label spelling (10). `?` and the palette (11).

## Signatures
```rust
// graft-ui::keymap (new module, pub in lib.rs)
pub use graft_core::event::KeyCode;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextId { Global, Graph, Map, Preview, Confirm, Changes, Diff, Resolver, Banner, Finish, Rewrite, Picker,
    Blame, Stash, CommitEditor, Journal, History, Palette, EmptyState, Tour, Credential, Menu }
impl ContextId {
    pub const ALL: &'static [ContextId];
    pub fn config_name(self) -> &'static str;
    pub fn scope(self) -> &'static [ContextId];
    pub fn is_text_field(self) -> bool;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActionId(pub &'static str);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyChord { pub code: KeyCode, pub ctrl: bool, pub alt: bool, pub shift: bool }
impl KeyChord {
    pub fn parse(text: &str) -> Result<KeyChord, ChordError>;
    pub fn label(&self) -> KeyLabel;
    pub fn from_press(press: &graft_core::event::KeyPress) -> Option<KeyChord>;   // None: command held
    pub fn to_press(&self) -> graft_core::event::KeyPress;
}
#[derive(Debug, PartialEq, Eq)]
pub struct ChordError { pub text: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyLabel(pub String);                       // "⌃P", "m", "—" when unbound
pub struct Binding { pub action: ActionId, pub context: ContextId, pub default: &'static [KeyChord],
    pub label: &'static str, pub search: &'static [&'static str] }
pub fn defaults() -> &'static [Binding];
#[derive(Debug, PartialEq, Eq)]
pub enum Translated { Default(KeyChord), Unbound, Pass }
#[derive(Debug, PartialEq, Eq)]
pub enum KeyWarning {
    UnknownAction(String),
    BadKey { action: String, text: String },
    Collision { key: KeyChord, context: ContextId, kept: ActionId, lost: ActionId },
    Reserved { action: String, key: KeyChord },
    Fixed(ActionId),
}
impl KeyWarning { pub fn words(&self) -> String; }
pub struct Keymap { /* defaults + overrides, resolved per context */ }
impl Default for Keymap { /* defaults(), no overrides */ }
impl Keymap {
    pub fn with_overrides(lines: &[(String, Vec<String>)]) -> (Keymap, Vec<KeyWarning>); // ("graph.merge", ["M"])
    pub fn translate(&self, ctx: ContextId, pressed: KeyChord) -> Translated;
    pub fn spell(&self, action: &ActionId) -> KeyLabel;
    pub fn chords(&self, action: &ActionId) -> &[KeyChord];
    pub fn in_context(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])>;
}
```
No other `pub` items or `Binding` fields without the conductor.

**Differs from the note's Interfaces** (signature fixes to the code and the doc, no new scope): `KeyName` dropped for
core's `KeyCode`; `ContextId` gains `Confirm`, `CommitEditor`, `Credential` (doc §2.3c, §2.10, §2.14 had no context);
`Collision` fields `yours/displaced` → `kept/lost` (an inner default can keep a chord a Global override asked for);
`Reserved` names the action; `Fixed` is new (typed text is never remapped, so an override there must say so rather
than silently do nothing); `scope`, `is_text_field`, `from_press`/`to_press`, `chords` added.

## Acceptance
- [x] NOTES: doc-only and code-only lists with §/file:line; `NOT_YET_ANSWERED` ids with their ticket. — NOTES below (a doc artefact; no test).
- [x] Golden id list exists; its test fails on a rename (shown by a temporary edit, reverted; test named at tick). — graft-ui `keymap::tests::table::action_ids_match_the_golden_list`; `graph.rebase`→`graph.rebase_onto` made it fail with the alias message, reverted.
- [x] Id shape and uniqueness (test). — graft-ui `keymap::tests::table::ids_are_unique_well_formed_and_prefixed_by_their_context`.
- [x] Chord round-trip over every default, both directions; bad text → `BadKey` (tests). — graft-ui `keymap::tests::chords::every_default_round_trips_through_its_label_and_its_words`, `…_through_the_press_the_decoder_delivers`, `chords::text_that_names_no_key_is_an_error_carrying_the_text`, `warnings::a_bad_key_leaves_the_action_on_its_defaults`.
- [x] Each warning kind produced and worded; `Ctrl+C` → `Reserved`; fixed row → `Fixed` (tests). — graft-ui `keymap::tests::warnings::an_unknown_action_is_named`, `a_bad_key_leaves_the_action_on_its_defaults`, `ctrl_c_is_reserved_both_ways`, `a_text_field_row_is_fixed`, `a_collision_his_override_wins_names_what_lost_it`, `a_collision_his_override_loses_says_what_stays`.
- [x] Pure translate sweep green over every `ContextId`, remapped and default keymaps (test named at tick). — graft-ui `keymap::tests::translate::the_sweep_moves_every_action_and_every_context_follows` (>200 context×action pairs), `translate::the_default_keymap_passes_every_default_everywhere`.
- [x] J8 table-level test as in Scope 6. — graft-ui `keymap::tests::translate::j8_merge_on_capital_m_is_one_problem_and_acts_as_m`.
- [x] `every_bound_key_has_a_row` green with no exemptions, or each exemption a NOTES doc question. — graft `update::keys::table_check::every_bound_key_has_a_row`; 3 exemptions (`release`, `type`, `tab`) in NOTES; removing `app.page_up` made it fail (`PgUp (page)`), reverted.
- [x] `crates/graft-ui/MAP.md` gains `keymap` (every pub item); `crates/graft/MAP.md` names `table_check`. — by reading (docs; no test).

## Touches
- New module; no existing graft-ui item changes. graft: one new `#[cfg(test)]` file reading `bound_actions` (re-exported
  from `update::keys`, MAP.md :491). No drawn change, so no capture.
- Later readers: 09b (edge, config), 10 (`spell`), 11 (`in_context`, `words`, runs actions by default chord), 23
  (man page from `defaults()`), 18/19 (empty state `c`/`t` rows). Their specs name `KeyName` or `yours/displaced`
  nowhere (grep of `.scratch/m8-polish-and-ship/issues/`, 2026-10-04).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~110k: doc tables ~25k, chain read ~20k, rows ~15k written, module + tests ~35k. If the walk alone passes
~50k: land the types and the walk's NOTES, commit at the seam, report `blocked: table rows`; the conductor gives the
rows (from NOTES) to a Sonnet follow-up.

## NOTES (builder, 2026-10-04)

Scouts: `scouts/09-keymap-table.md` (given) and `scouts/09-key-walk.md` (mine: every chord the dispatch matches, per
place, file:line; paths below are `crates/graft/src/update/` unless they say otherwise). Did not compact.

### Doc-only (doc row, code doesn't answer) — doc questions, not build choices
| Row | Code | In the table? |
|---|---|---|
| §2.2 `?` | no handler; `place/keys.rs:27` passes it on, nothing catches it | **yes**, `app.help` → 11 |
| §2.2 `Ctrl+P`, `:` | none | **yes**, `app.palette` → 11 |
| §2.12 `Enter` run the row | no palette yet | **yes**, `palette.run` → 11 |
| §2.13 `c` Clone | `../empty_state.rs:87-117` has `q o i ⏎ j k` only | **yes**, `empty_state.clone` → 18 |
| §2.13 `t` Practice tour | same | **yes**, `empty_state.tour` → 19 |
| §2.13 `?` | same | via `app.help` (11) |
| §2.2 `h`/`l`, `←`/`→` move left/right | `keys/predicates.rs:35-46` `motion_for` has neither | no row |
| §2.2 `Ctrl+Shift+Z` redo | `keys/predicates.rs:140-150` unbound on purpose (§2.2's own note) | no row (`U` is `app.redo`) |
| §2.3 `i` / §2.4 `i` add to `.gitignore` | no arm anywhere | no row |
| §2.3 `e`, `d`, `s` on the graph | doc says "only once the detail pane is open"; code agrees (`detail/keys.rs:247,256,262`) | as `changes.*` |
| §2.2 `Space` (generic toggle) | answered per place, never globally | no `app.*` row: `graph.fold`, `changes.stage`, `diff.fold_dir`, `blame.heatmap`, `stash.preview`, `finish.toggle_diff`, `journal.expand` |

**09b's `NOT_YET_ANSWERED`:** `app.help` (11), `app.palette` (11), `palette.run` (11), `empty_state.clone` (18),
`empty_state.tour` (19).

### Code-only (code answers, no doc row) — each is a row; doc questions for the conductor
| Key | Where | Row |
|---|---|---|
| `PgUp`/`PgDn` | `keys/predicates.rs:41-42` (§2.6/§2.6b cite "§2.2" for them; §2.2 has no row) | `app.page_up`/`app.page_down` |
| `Home`/`End` as `g`/`G` | `keys/predicates.rs:43-44` | in `app.top`/`app.bottom` |
| `n`/`N` step a kept search | `search.rs:43-44` (§2.6 calls them graph globals; §2.2 has no row) | `app.next_match`/`app.prev_match` |
| `J` opens the journal | `journal/keys.rs:29` (§2.11 lists `J` only as close) | `graph.journal` |
| `F2` status, `F3` renderer | `keys/predicates.rs:63,82` | `graph.status`, `graph.renderer` |
| `B` write / `X` decline the commit-graph offer | `commit_graph.rs:89,93` | `graph.write_commit_graph`, `graph.decline_commit_graph` |
| `←`/`→`/`⇥`/`⇧⇥` move a preview's cursor | `preview/keys.rs:60-61` (§2.3b has none; §2.3c does for the box) | `preview.left`/`right`/`cycle` |
| `⇧⇥` in a confirm box | `confirm.rs:170-191` | in `confirm.cycle` |
| `S` closes the stash list | `stash.rs:204` | `stash.close` |
| `Space` expands a journal row | `journal/keys.rs:99-107` | `journal.expand` |
| `⇧⇥` previous field in the commit editor | `commit_editor/keys.rs:62` | `commit.prev_field` |
| `y` / `n` on the empty state's init box | `../empty_state.rs:74-75` | `empty_state.confirm`/`cancel` |
| `b` new branch from a stash | `stash.rs:223` — **§2.9 says "planned, not yet built"; the code built it** | `stash.branch` (doc stale) |

Not rows (doc questions): the **name prompt** (`name_prompt.rs:255-286`: `⏎ create`, `⇥ toggle`, `⌃U`) has no
`ContextId` and no doc table; its keys are text-field keys by state (09b passes them). The search, J-find, blame-find,
stash-save and rewrite-field prompts, and the finish summary's subject/body, are text fields by state, not by context:
their `⏎`/`Esc`/`⌃E`/`⌃←→` are not rows. `Ctrl+Z` suspends instead of undoing when `suspend_on_ctrl_z` is set
(`keys.rs:217-223`); the table keeps `⌃Z` on `app.undo`.

### Scout correction
`09-key-walk.md` (B) says §2.6a `q` and `e` are unanswered; that row describes the finish summary. The banner answers
both: `banner/focus_keys.rs:135` (`e`), `:166` (`q`). Checked by grep.

### Decisions where the spec was silent (say if wrong)
1. **`translate` passes every key in a text-field context** (`ContextId::is_text_field`), remapped or not. The spec's
   Scope 5 rule would otherwise turn a letter typed in the picker into a remapped Global action (`app.back = "x"` →
   typing `x` closes the picker). §2.12/§2.14/§2.16: "claims every key but `Ctrl+C`". The sweep asserts it.
2. **Labels the code never spells** (none of these is in `bound_actions` or a hint): `home`, `end`, `backspace`,
   `delete`, `insert`, `⇧⇥` (BackTab; the spelling in `finish.rs:75` and MAP.md), `⇧` before a shifted non-char key,
   `alt+⌃X` for ctrl+alt. Everything `bound_actions` spells is taken from it (`⌃A`, `⏎`, `esc`, `space`, `PgUp`,
   `F2`, `alt+↑`, `⇥`).
3. **Ctrl+Shift+letter** normalises to `Char(uppercase)` + ctrl, labelled `⌃⇧C`; `⌃C` is Ctrl+c. Kitty delivers
   `Char('C')` + control + shift (`crates/graft-term/src/input/decode/csi.rs:141-142`), which `from_press` folds and
   `to_press` restores. **Legacy bytes can't carry it**: `0x03` is `Char('c')` + control
   (`crates/graft-term/src/input/decode/mod.rs:170`), so §2.10's `Ctrl+Shift+C` co-author trailer quits instead
   outside kitty (kitty is opt-in, `crates/graft/src/terminal.rs:159`). A doc/code question, not this ticket's.
   Consequence for his config: `ctrl+P` (capital) means Ctrl+Shift+P, as a single character is case-sensitive.
4. **A line with any problem is not applied** (bad chord, `Ctrl+C` in a list, fixed row): the action keeps its
   defaults. Collisions are applied and resolved by the rule. An empty list is `BadKey` with text `""`.
5. **Collisions are warned only where an override is involved**; Global vs. context default overlaps (`⏎`, `esc`…)
   are the table's design. Text-field contexts are not checked (nothing there is remapped).
6. **Ids from the code's verb where the doc's is a phrase**: `graph.switch` (C), `graph.take` (o), `confirm.yes`,
   `commit.commit`. `map.close` is its own row (the ticket's J8 names it), not `graph.map` reached through scope.
   `preview.confirm` is `[y, m, r]` (synonyms: §2.3b's `m m`/`r r`). Banner `⏎` is `app.open` (§2.4: "§2.2's global
   open … not a new letter"); `Esc` is `app.back` in every place.
7. No scope widened: no key in the walk is live outside the Scope 4 table.

### `every_bound_key_has_a_row` exemptions (each a doc question)
`release` (the mouse button ending a drag), `type` (picker typing), `tab` — the rewrite editor's bar spells Tab
`tab` (`keys/bound.rs` "tab after") while everything else spells `⇥`: spelling drift for ticket 10.
