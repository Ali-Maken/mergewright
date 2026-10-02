# G-keymap 1 — one key table, remaps translated at the edge

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Fable (signature)**: `ActionId` becomes his config's
API (never renamed after release) and every later surface (bar, `?`, palette, tour, man page) reads this table; a
wrong table shape or a leaky translate is structural (ADR-0055).
Blocked by: 06, 07
Needs: none in code. Sequenced after `look-chrome-header-gutter-bar` and `look-cursor-row-and-dark-b` (lane 6).

## Read first
- `docs/adr/0055-keys-are-one-table-and-remaps-translate-at-the-edge.md` (whole)
- `docs/design/02-keymap.md` §2.1 last bullet ("Every binding is remappable"), §2.2, then every table §2.3–§2.18 as
  the table's source (read the tables, skip the prose under them)
- `docs/design-notes/m8-polish-and-ship.md`: Interfaces `graft-ui::keymap`, Journeys J8
- `crates/graft/MAP.md`: `config` (`Config::keys`, `Keys { suspend_on_ctrl_z }`, `ConfigWarning`), `update` entry,
  `update::keys::bound_actions`, `update::tests::NEVER_HINTED`
- `crates/graft-ui/MAP.md`: `graph_screen::hints` (`HintContext`, `every_context`)

## Scope
In:
- `graft_ui::keymap`: `defaults()` — every binding of 02-keymap.md as data (action id, context, default chords,
  label). Ids `<context>.<verb>`, snake_case, from the doc's words (`graph.merge`, `app.undo`, `operation.continue`).
  The table's test: every key `bound_actions()` answers today has exactly one row, and every row is answered.
- `KeyChord` parse/print: `"m"`, `"M"`, `"ctrl+p"`, `"shift+tab"`, `"alt+up"`, `"enter"`, `"esc"`, `"space"`,
  `"pgdn"`; print as the app spells keys (`⌃P`, `⇥`, `⏎`).
- `Keymap::with_overrides`: `[keys.<context>] <verb> = "<chord>" | ["<chord>", …]`; `"none"` unbinds. Warnings, never
  fatal: unknown action, bad chord, collision (both named, defaults included; the displaced action loses that chord
  in that context), reserved (`Ctrl+C`). `KeyWarning::words` spells each (`m given to graph.fetch — merge has no key`).
- `translate(ctx, pressed)`: looks in `ctx`, then `Global`. Remapped chord → `Default(action's default chord)`; a
  default chord whose action moved and nothing else claims → `Unbound`; anything not in the table → `Pass`. Text-field
  contexts (palette, picker, commit editor, prompts, search, credential) always `Pass`: typed text is never remapped.
- graft `update`: one call to `translate` before any `*_key` dispatcher; `context_of(&AppState) -> ContextId`.
  `Unbound` → the key does nothing, no status.
- graft `config`: `[keys]` keeps `suspend_on_ctrl_z` (a bool at the table's top level); sub-tables are contexts.
- Startup with problems: graph opens as normal; status row `config: N problems · ? shows them` (problem rows are
  `AppState::config_problems: Vec<String>`, which G-theme appends to and G-palette lists). Zero problems → nothing
  shown (CLAUDE.md status rule).
- **Translate sweep test** (ADR-0055 consequence): remap every action to a free chord, walk every context, assert each
  remapped chord acts as the default did and each displaced default does nothing.
Out:
- Spelling keys from the table on the bar, `.` labels, boxes, banner (`keymap-spell-from-the-table`). `?` and the
  palette (G-palette). Man page (G-docs).

## Signatures
```rust
// graft-ui::keymap
pub enum ContextId { Global, Graph, Changes, Diff, Resolver, Banner, Finish, Rewrite, Picker, Blame,
    History, Stash, Journal, Map, Preview, Menu, Palette, EmptyState, Tour }
pub struct ActionId(pub &'static str);
pub enum KeyName { Char(char), Enter, Esc, Tab, BackTab, Space, Up, Down, Left, Right, PageUp, PageDown, Home, End,
    Backspace, Delete, F(u8) }
pub struct KeyChord { pub code: KeyName, pub ctrl: bool, pub alt: bool, pub shift: bool }
impl KeyChord { pub fn parse(text: &str) -> Result<KeyChord, ChordError>; pub fn label(&self) -> KeyLabel; }
pub struct KeyLabel(pub String);                       // "⌃P", "m", "—" when unbound
pub struct Binding { pub action: ActionId, pub context: ContextId, pub default: &'static [KeyChord],
    pub label: &'static str, pub search: &'static [&'static str] }   // search: hidden words ("aside" → stash)
pub fn defaults() -> &'static [Binding];
pub enum Translated { Default(KeyChord), Unbound, Pass }
pub enum KeyWarning { UnknownAction(String), BadKey { action: String, text: String },
    Collision { key: KeyChord, context: ContextId, yours: ActionId, displaced: ActionId }, Reserved(KeyChord) }
impl KeyWarning { pub fn words(&self) -> String; }
pub struct Keymap { /* defaults + overrides, per context */ }
impl Keymap {
    pub fn with_overrides(lines: &[(String, Vec<String>)]) -> (Keymap, Vec<KeyWarning>);
    pub fn translate(&self, ctx: ContextId, pressed: KeyChord) -> Translated;
    pub fn spell(&self, action: &ActionId) -> KeyLabel;
    pub fn in_context(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])>;
}
// graft (pub(crate))
pub(crate) fn context_of(state: &AppState) -> ContextId;
// AppState gains `keymap: Keymap`, `config_problems: Vec<String>`
```
`search` on `Binding` is an addition over the note's shape (palette hidden words, §2.12); the builder may not add
other fields without the conductor.

## Acceptance
- [ ] Table ↔ dispatch: every `bound_actions()` key has one row; every row is answered (test named at tick).
- [ ] Ids unique, `<context>.<verb>` snake_case; a golden list `crates/graft-ui/tests/keymap_ids.txt` that fails on
      any rename (the release contract).
- [ ] Chord parse/print round-trips every default; bad text → `BadKey` (tests).
- [ ] Each warning kind produced and worded (tests); `Ctrl+C` refused as `Reserved`.
- [ ] Translate sweep green over every `ContextId` (test name at tick).
- [ ] Text fields: with `graph.merge = "x"`, typing `x` in the commit editor, search and picker types `x` (tests).
- [ ] J8 (person step, TESTING.md row): `[keys.graph] merge = "M"` → startup status `config: 1 problem · ? shows them`
      (collides with Map), `M` merges, Map has no key. Test drives the same through `load_from`.
- [ ] `suspend_on_ctrl_z` still read (existing test green).

## Touches
- Every `*_key` dispatcher's caller (one entry point; the scout names it); `bound_actions`; config loader and its
  unknown-key warnings (`[keys.graph]` must not warn as unknown); status row; journal/undo unaffected (keys only).
- `HintContext` ↔ `ContextId` (the scout maps them; hints keep their own enum until keymap-spell).
- `docs/design/02-keymap.md` (no edit: the doc is the source); MAP.md graft-ui, graft; README config section is G-docs'.

## Scout questions
- The single entry point before the `*_key` dispatchers (file:line); any dispatcher reached another way (mouse,
  replayed keys, harness `hand_back`)?
- How `HintContext` and the focused place map to a dispatch context today; which states have no clean context.
- TOML crate in use and how `[keys]` parses today (file:line); does an unknown sub-table warn?
- Keys bound in code but absent from 02-keymap.md, and the reverse: list both (those are table questions for the
  conductor, not build choices).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~75k tokens. The table is long but mechanical; if it alone passes ~30k, the conductor splits the table's
rows by context into a Sonnet follow-up after the types land.
