# G-palette 1 — `?` and `⌃P` / `:` open one palette; actions, theme rows, your config

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: the screen is specified (DESIGN §1.6, §1.16,
keymap §2.12); the table it reads is fixed by G-keymap.
Blocked by: 08, 09, 09b, 10
Needs: `keymap-table-and-translate` (`Keymap::in_context`, `spell`, `KeyWarning::words`, `context_of`,
`AppState::config_problems`), `keymap-spell-from-the-table` (hints over `ActionId`), `look-outside-repo-screen`
(empty state answers `?`).

## Read first
- `docs/design/01-view-mockups.md` §1.6, §1.16 (whole); `docs/design/02-keymap.md` §2.2 (`?`, `⌃P` rows), §2.12
- `docs/adr/0056-help-is-the-palette-on-this-place-and-the-tour-is-a-practice-repo.md` §1–2
- `docs/design-notes/m8-polish-and-ship.md`: Journeys J4–J7, J9; Interfaces `graft-ui::palette`; New UI last bullet
- `crates/graft-ui/MAP.md`: `picker` (the nearest modal list box), `text_input` (`TextField`, `FieldView`), hints
  (doorway, `every_context`); `crates/graft/MAP.md`: `update::picker` (modal claim, mouse), theme change path

## Scope
In:
- `graft_ui::palette` widget per §1.16: box ≤ 96 columns centred over the focused pane, full width under 100 columns;
  query row `› ▏`; groups; row = key (accent, 6 cells) · label · config id (muted, right-aligned, dropped < 100
  columns); unbound key `—`; scrolls; no match `nothing matches · esc close`.
- `?` → `PaletteMode::Here(ctx)`: `YOUR CONFIG` (when `config_problems` is non-empty, one row each) · `IN PROGRESS`
  (operation live; its keys not repeated under HERE) · `HERE` · `EVERYWHERE`. `⌃P` / `:` → `Everything`: an `ACTIONS`
  group (here + everywhere + app rows), other sources come in `palette-find-anything`.
- App rows: `Theme: dark`, `Theme: light`, `Theme: high contrast` — switch this run only, through the existing theme
  change path (caches flushed as on an OSC 11 change). `Practice tour` row is G-tour's (not drawn until built, §1.15).
- Filter: typed text matches labels and `Binding::search` words; a typed chord bound in this place ranks its row first
  (J5 `? x`). One matcher (see Scout).
- `⏎` runs the row "as its key would, in the place `?` was opened from": close, then dispatch the action's chord
  through the normal path (translate included). A refusal reads as it would from the key.
- Modal (§2.1): claims every key but `Ctrl+C`; `↑↓` move, `⏎` run, `esc` close; mouse: click a row = select, double =
  run, wheel scrolls, click outside closes.
- `?` answers on every screen: graph, every pane and place, overlays, resolver, banner, empty state. Inside a text
  field `?` types `?`.
- Doorway: the bar's `. more` becomes `? keys` (last, never dropped); `.` stays bound and becomes an ordinary candidate
  `. actions`, ranked last before the doorway (§2.1: everything bound is a candidate).
- Rows (decided here): 163 — `space`, `1`, `H` reachable at 80 columns through `? keys`; 538 — `1 first-parent` not
  offered on the Detail tab's bar while the graph is hidden (its effect is off-screen, ADR-0041 carve-out); 539 —
  `a amend` hinted where it acts (if still in `NEVER_HINTED`, remove it there); 553 — overlay up + changes pane
  focused: the bar draws the pane's live keys.
Out:
- Branches, tags, files, commits, recent commands (`palette-find-anything`). Tour row (G-tour). First-launch line
  (G-first-run).

## Signatures
```rust
// graft-ui::palette
pub enum PaletteMode { Here(ContextId), Everything }
pub enum RowKind { Action, App, Branch, Tag, File, Commit, Recent }
pub struct PaletteRow { pub label: SanitizedText, pub key: Option<KeyLabel>, pub config: Option<ActionId>,
    pub kind: RowKind }
pub struct PaletteView { pub mode: PaletteMode, pub query: FieldView, pub groups: Vec<(SanitizedText, Vec<PaletteRow>)>,
    pub cursor: usize, pub warnings: Vec<SanitizedText> }
pub struct Palette<'a> { pub view: &'a PaletteView, pub theme: &'a Theme }      // impl Widget
pub fn palette_area(over: Rect, frame: Rect) -> Rect;
pub fn palette_row_at(area: Rect, view: &PaletteView, column: u16, row: u16) -> Option<usize>;
// graft-core (or where the scout finds the matcher)
pub fn fuzzy_score(query: &str, text: &str) -> Option<u32>;
// graft (pub(crate)): update::palette — PaletteState { mode, query: TextField, cursor, opened_from: ContextId }
```

## Acceptance
- [ ] Widget tests 80×24, 120×36, 200×60: groups and order as §1.16; config id dropped < 100 cols; `—` for unbound.
- [ ] J4 `find-key-by-name` = 2 (`?` `merge` shows `m merge …`); J5 `what-does-key` = 2 (`?` `x` ranks `x` first);
      J6 `run-unrecalled` = 3 (`⌃P` `stash` `⏎` → the stash prompt opens); J7 `stopped-how-go-on` = 1 (rebase stopped,
      `?` → `IN PROGRESS` first with `c continue`); J9 `light-theme` = 3 (`⌃P` `theme light` `⏎` → theme kind light).
      Each a walk test counting keys.
- [ ] `?` opens on every `ContextId` with a screen (sweep test over `every_context()` and the empty state).
- [ ] Scout's finding on `?` doing nothing today recorded with its line before the fix (rule 7).
- [ ] With 2 config problems: status `config: 2 problems · ? shows them`; `?` lists `YOUR CONFIG` first (test).
- [ ] `every_key_the_bar_draws_is_answered` covers `? keys` and `. actions`.
- [ ] Captures: `docs/shots/m8-help-graph-120x36.txt`, `…-rebase-stop-120x36.txt`, `…-80x24.txt`,
      `m8-palette-everything-120x36.txt`, beside §1.16's drawing. **Colour unverified** (rule 5).

## Touches
- Hint bar doorway in every non-modal context (all captures with `. more` re-shot — list in NOTES); `every_context`
  sweeps; `NEVER_HINTED`; context menu (`.` unchanged); theme change path and every cache it flushes (strips, rows);
  mouse routing order (palette before picker/menu); the resolver and banner (modal: `?` must be theirs to give —
  check they don't bind `?` already); empty state bar (`? keys` added).
- `docs/design/02-keymap.md` §2.12 (written); MAP.md graft-ui, graft.

## Scout questions
- What does each dispatcher do with `?`, `:` and `⌃P` today (file:line)? (Rule 7: the capture shows nothing opened;
  find the line before calling it a defect.)
- Is `graft_ops::switch_list::filter_switch_rows`' matcher reusable as a pure `fuzzy_score`? Where should it live so
  graft-ui and graft-ops both reach it (graft-core)?
- How does a theme change flow today (OSC 11 re-query on focus — built?) and what does it flush (file:line)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~75k tokens. If the doorway re-shoot list alone is large, captures go to a Sonnet follow-up.

> Conductor (2026-10-04, from the 09/09b split by planner-fable): this ticket must call `dispatch_key` (09) rather than matching keys itself.
