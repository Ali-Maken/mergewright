# G-keymap 1a — one key table and its translate (graft-ui)

Status: ready (re-specced 2026-10-04 after scout `scouts/09-keymap-table.md`; split: the app edge is `09b`)
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
- [ ] NOTES: doc-only and code-only lists with §/file:line; `NOT_YET_ANSWERED` ids with their ticket.
- [ ] Golden id list exists; its test fails on a rename (shown by a temporary edit, reverted; test named at tick).
- [ ] Id shape and uniqueness (test).
- [ ] Chord round-trip over every default, both directions; bad text → `BadKey` (tests).
- [ ] Each warning kind produced and worded; `Ctrl+C` → `Reserved`; fixed row → `Fixed` (tests).
- [ ] Pure translate sweep green over every `ContextId`, remapped and default keymaps (test named at tick).
- [ ] J8 table-level test as in Scope 6.
- [ ] `every_bound_key_has_a_row` green with no exemptions, or each exemption a NOTES doc question.
- [ ] `crates/graft-ui/MAP.md` gains `keymap` (every pub item); `crates/graft/MAP.md` names `table_check`.

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
