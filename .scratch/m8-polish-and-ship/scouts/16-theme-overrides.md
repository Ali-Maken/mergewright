# Scout 16 — theme overrides (main @ a6f9f12e)

## Theme today (graft-ui)
- `Token` enum theme/mod.rs:72; `Token::name` :147-170 already gives config spellings (`selection`, `text-muted`, `ours-tint`…). No `from_name`. `index()` :174 is `pub(super)`.
- `Theme` :256-261 holds `kind`, `depth`, `chrome: [TokenColor; N]`, `lanes` — **reduced** colours only; no truecolor `Rgb` table kept.
- `Theme::new(kind, &Capabilities)` :272-300: `no_color` → depth None; each token reduced from `kind.chrome()` (`reduce_conflict` for conflict tokens :282-283); lanes from `lanes::LANE_HUES` (not in kind table — lanes already outside any theme).
- **Three readers bypass the resolved table and read the built-in `kind.chrome()` directly:** `on_selection` contrast.rs:130 (selection fill used for lifting), `raster_muted` mod.rs:353 (pixel `+N` hue). An override applied only to `chrome` would not reach these.
- `contrast()` contrast.rs:35, `ContrastRule {fg,bg,min}` :49, `CONTRAST_RULES` :63-86 (text/bg 4.5, text/selection 4.5, selection/bg 3.0, text-muted/bg 4.5). No `ContrastMiss`.

## Theme::new call sites (production)
- graft/src/session.rs:171 (`Session::settle`, startup), session.rs:198 (`Session::apply`, OSC 11 re-query `TerminalChange::Theme`). Neither has the `Config`; `settle` gets `&Loaded` (:138) so overrides are reachable there; `apply` has only `self` — Session holds no overrides today.
- Tests/examples/benches: paint.rs:84,94,102; confirm.rs:276; stash_screen.rs:430; map_screen.rs:142,212; examples/graph_column.rs:24; benches/row_render.rs:122.
- **Palette theme rows: no palette module exists** (no `graft-ui/src/palette*`); tickets 11/12 not merged. Could not confirm what "G-palette theme rows" calls.

## Caches
- Pixel strips: `StripPaint::of` view/strips.rs:45-52 copies `raster_lanes()` + `raster_muted()` as values; turn.rs:254 drops strips on re-detected theme. Muted comes from built-in table (see above), so an override of `text-muted` would not reach pixels today.
- No other theme-keyed cache found by grep `theme` in graft/src/view.

## Bg painting (scout question 2)
- `Token::Bg` painted as a background in: finish_summary.rs:174, commit_editor.rs:205, rewrite_editor/draw.rs:28; as fg at detail_pane/line_render.rs:277. Elsewhere the terminal ground shows. An override of `bg` would change these four fills only; it would also feed `CONTRAST_RULES` checks while most screens show the terminal's own ground — **ratio warned is against the override, not what's visible**. Could not confirm intent beyond this.

## Config
- `Config` types.rs:19; `Ui.theme: ThemeChoice` :57-59, `overridden()` :241. No `[theme]` section.
- load.rs:180 `parse`: typed serde parse, then `unknown_keys` :227 via `SCHEMA` :217-224 — `theme` table absent, so `[theme.dark]` today warns `UnknownKey{path:"theme"}`. Only `keys` skipped (:233). Note SCHEMA checks one level; `[theme.dark]` is two levels.
- `ConfigWarning` load.rs:36 (UnknownKey, NoConfigHome, NotAKey). Problems become words in `config::keymap_from` (config/keys.rs:71) → startup.rs:243 → `AppState::config_problems` state.rs:416/856; notice in view/frame/bars.rs (MAP.md:205).

## Overlap with ticket 10
- Ticket 10 (key spelling from table) and 16 both touch graft/src/config (keys.rs/load.rs) and `keymap_from`'s problem list — same file set; do not build concurrently.

## Contradictions
1. Ticket "applied before depth reduction" fits `new`, but signature `with_overrides(self, …)` takes a finished `Theme` that holds only reduced colours (mod.rs:256-261) — no Rgb to override pre-reduction.
2. `on_selection` (contrast.rs:130) and `raster_muted` (mod.rs:353) read `kind.chrome()`; ticket's Touches misses them.
3. Ticket's "palette rows" caller does not exist yet.
4. `NO_COLOR` "no warning": not enforced anywhere today; loader has no capability access (load.rs:180).

## Tests/captures likely touched
graft-ui theme/tests.rs + theme/snapshots; graft/src/config/tests.rs (unknown-key cases); session.rs tests :315-455; event_loop/tests.rs (re-query). Keys: none claimed.
