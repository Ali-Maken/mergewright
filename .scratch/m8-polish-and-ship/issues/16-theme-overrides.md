# G-theme — change one colour in config; a colour that misses a floor is named

Status: done
Build with: **Opus** — brownfield across two crates (graft-ui theme, graft config + session); the theme gains a
pre-reduction colour table and two readers move onto it; judgement on every path that builds a theme (scout:
`.scratch/m8-polish-and-ship/scouts/16-theme-overrides.md`).
Blocked by: 07, 09, 09b, 10, 10c (10 shares graft `config/keys.rs`, `config/load.rs` and `keymap_from`'s problem list —
build serially, 10 first)
Needs: `look-cursor-row-and-dark-b` (`contrast`, `ContrastRule`, `CONTRAST_RULES`, `Theme::on_selection`),
`keymap-edge-and-config` (`AppState::config_problems`, `config_notice`, `keymap_from`).

## Read first
- The scout file above — whole (1 page). Its file:line refs are main @ a6f9f12e; re-check after 10 lands.
- `PROMPT.md` §5 "Colour tokens" bullet and its four sub-bullets (lanes are not a theme's to change)
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows "Change one colour", "Contrast floor"; Journeys J10;
  "Where he lands" row "Startup with `[keys]`/`[theme]` problems"; Interfaces `graft-ui::theme`
- `docs/adr/0003-transparent-graph-rendering.md` (lines 1–12: never paint a background into the graph)
- `docs/research/themes.md` — the gitui / helix `inherits` / VS Code `colorCustomizations` rows only
- `crates/graft-ui/MAP.md` `## theme`; `crates/graft/MAP.md` `## config`, `Session` entries (:79–82),
  `AppState::config_problems` (:205)

## Decisions (spec-only pass, 2026-10-04, from the scout)
1. **Overrides apply inside construction, before reduction.** `Theme` holds only reduced colours (theme/mod.rs:256–261),
   so `with_overrides(self, …)` cannot work. It becomes a constructor: `Theme::with_overrides(kind, caps, overrides)`;
   `Theme::new(kind, caps)` is that call with `&[]` (test/bench/example call sites unchanged). `Theme` keeps a private
   pre-reduction table `rgb: [Rgb; TOKEN_COUNT]` = `kind.chrome()` with overrides written over it; every token is reduced
   from that table (256/16 by the same nearest mapping as today).
2. **Every reader of built-in colours moves onto the table.** `on_selection` (contrast.rs:130) and `raster_muted`
   (mod.rs:353) read `self.rgb`, not `kind.chrome()`. After this ticket, `kind.chrome()` is read only by the constructor
   and `contrast_misses`. So an overridden `selection` drives text lifting on the cursor row, and an overridden
   `text-muted` reaches the pixel `+N` marker.
3. **Conflict tokens:** an overridden conflict token skips the private `pinned` 256/16 table and goes through
   `nearest_256_cube`; an un-overridden one is unchanged. Overriding `ours` does not re-derive `ours-tint` — each key is
   independent (VS Code `colorCustomizations`, helix). `Theme::tint` rules (none at 16 / `NO_COLOR` / high contrast) are
   unchanged; a tint override there is stored and not drawn, no warning.
4. **Contrast misses are a property of the file, not the terminal.** `contrast_misses(kind, overrides)` is pure, on
   pre-reduction colours; startup checks **all three** `[theme.*]` tables, so a miss in `[theme.light]` is named while
   he runs dark (he edits one file for every terminal). Built-ins pass every floor (existing tests), so only his
   overrides can miss. Ratio words round **down** to one decimal, so a miss never reads as meeting its floor (2.96 →
   `2.9:1`).
5. **`NO_COLOR`:** overrides change nothing on screen (every token is already `Default`); contrast misses are **not**
   reported (nothing visible is wrong — CLAUDE.md status rule). File errors (unknown token, `lane-N`, bad colour) **are**
   reported: the file is wrong in every terminal.
6. **`bg` (scout's open question):** allowed, and checked. `bg` means "the ground your terminal shows"; overriding it
   is how he tells Graft his terminal's real background, so floors measured against it are measured against what he
   sees. It paints only where `Token::Bg` already paints (finish_summary.rs:174, commit_editor.rs:205,
   rewrite_editor/draw.rs:28; fg at detail_pane/line_render.rs:277). **No new painter**: graph rows keep the terminal's
   ground (ADR-0003). Not inventing an OSC 11-derived `bg` here.
7. **One rebuild path.** `Session` keeps the overrides (from `settle`'s `&Loaded`) and gains `retheme(kind)` — the
   only way a theme is rebuilt mid-run. `apply`'s `TerminalChange::Theme` arm (session.rs:198) calls it. Palette rows
   (ticket 11, not merged at scout time) must call it too: if 11 has merged when you build, route its `Theme: …` rows
   through `retheme` and test it; if not, write in this ticket's handoff "11: theme rows call `Session::retheme`" for the
   conductor.
8. **Cache keys need no change.** Overrides are fixed for a run, so the theme kind stays a sufficient key; strips are
   dropped on a theme change as today (turn.rs:254). `StripPaint::of` (view/strips.rs:45–52) picks up the override via
   `raster_muted` (decision 2).
9. **Config parse.** `theme` joins `SCHEMA` (load.rs:217–224) as a table parsed by its own code, like `keys` (skipped at
   :233). `[theme.<kind>]` kinds: `dark`, `light`, `high_contrast` (`ThemeKind::name` spellings for the first two; check
   what `name` gives high contrast and accept that spelling too if it differs). Problems:
   unknown kind `theme.solarized` / unknown token `theme.dark.selction` → `UnknownKey { path }`; `lane-N` →
   `LaneColour { path }`; a value that isn't `"#RRGGBB"` (hex, either case) → `BadColour { path, value }`; `theme`
   not a table → `UnknownKey { path: "theme" }` as today. Entries in sorted order (toml, as `Keys::remaps`).

## Scope
In: decisions 1–9; words for the three new problems; the capture below; MAP.md rows for every new/changed item.
Out: per-widget attributes, theme files, palette-inheriting default (all R in Prior art). Lane hues. A `bg` read from
OSC 11. The palette's `Theme: …` rows themselves (ticket 11). README config section (G-docs).

## Signatures
```rust
// graft-ui::theme
#[derive(Clone, Copy, Debug, PartialEq)] pub struct ContrastRule { pub fg: Token, pub bg: Token, pub min: f32 } // derives added if missing
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContrastMiss { pub rule: ContrastRule, pub ratio: f32 }
impl ContrastMiss { pub fn words(&self) -> String; }               // "selection on bg 1.0:1 — under 3.0" (ratio floored to 0.1)
impl Token { pub fn from_name(name: &str) -> Option<Token>; }      // inverse of Token::name over Token::ALL; lanes → None
impl Theme {
    pub fn with_overrides(kind: ThemeKind, caps: &Capabilities, overrides: &[(Token, Rgb)]) -> Theme;
    // Theme::new(kind, caps) keeps its signature; body = Theme::with_overrides(kind, caps, &[])
}
pub fn contrast_misses(kind: ThemeKind, overrides: &[(Token, Rgb)]) -> Vec<ContrastMiss>; // CONTRAST_RULES order

// graft::config
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeOverrides { pub dark: Vec<(Token, Rgb)>, pub light: Vec<(Token, Rgb)>, pub high_contrast: Vec<(Token, Rgb)> }
impl ThemeOverrides { pub fn for_kind(&self, kind: ThemeKind) -> &[(Token, Rgb)]; }
// Config gains `pub theme: ThemeOverrides` (serde(skip), filled by load's own parse, as Keys::remaps)
// ConfigWarning gains `LaneColour { path: String }`, `BadColour { path: String, value: String }`
pub(crate) fn theme_problems(overrides: &ThemeOverrides, no_color: bool) -> Vec<String>;
//   each miss as "[theme.<kind>] " + ContrastMiss::words; empty when no_color

// graft::Session
// gains a private `overrides: ThemeOverrides`, set in `settle` from `&Loaded`
pub(crate) fn retheme(&mut self, kind: ThemeKind) -> bool;        // rebuild with this kind's overrides; true if changed
```
`Rgb` and `Capabilities` are the types `contrast` and `Theme::new` take today. Startup (startup.rs:243) appends
`theme_problems(&loaded.config.theme, <the settled caps' no-colour flag>)` after `keymap_from`'s list into
`AppState::config_problems`.

Words for the new problems: `theme.dark.lane-3: lane colours are fixed, not themeable`; `theme.dark.selection: "red"
is not a #RRGGBB colour`.

## Acceptance
- [x] J10 (person step stays a TESTING.md row, accept ticket's): `[theme.dark] selection = "#3A3F4B"` loads as
      `theme.dark == [(Selection, Rgb(0x3A,0x3F,0x4B))]` (config test); a settled truecolor dark session's cursor-row
      cell bg is that colour (buffer test). — `config::theme_tests::j10_one_selection_line_loads_as_one_override`, `session::theme_tests::j10_the_cursor_row_is_filled_with_his_selection`
- [x] `selection` = the built-in dark `bg` (test reads it from `ThemeKind::Dark`, no literal) → `config_problems ==
      ["[theme.dark] selection on bg 1.0:1 — under 3.0"]`, bottom row `config: 1 problem · ? shows them`, F2's
      status notes list it (test). `?`'s `YOUR CONFIG` row is ticket 11's test. — `event_loop::tests::theme_problems::selection_on_bg_is_one_problem_on_the_bottom_row_and_on_f2` (+ `config::theme_tests::selection_on_the_built_in_bg_is_one_contrast_problem`)
- [x] Ratio floored: a pair at 2.96:1 words as `2.9:1` (unit test on `words`). — `theme::overrides::tests::a_ratio_is_floored_so_a_miss_never_reads_as_its_floor`
- [x] A miss in `[theme.light]` is named while the session runs dark (test). — `config::theme_tests::a_light_miss_is_named_whatever_theme_runs` (`theme_problems` takes no kind: all three checked)
- [x] `lane-3 = "#FF0000"` → `LaneColour` warning, `Theme::lane(3)` unchanged (test). — `config::theme_tests::a_lane_colour_is_refused_and_lanes_stay_put`
- [x] `"red"` → `BadColour`; `[theme.solarized]` and `theme.dark.selction` → `UnknownKey`; `[theme.dark]` with valid
      keys → no warning (config tests). — `config::theme_tests::{a_colour_that_is_not_hex_is_a_bad_colour, an_unknown_kind_or_token_is_an_unknown_key, valid_keys_in_every_kind_say_nothing_and_hex_takes_either_case}`
- [x] Overridden `selection`: `on_selection(Text)` resolved colour is ≥ 4.5:1 against the override, not the built-in
      (test). Overridden `text-muted`: `raster_muted()` returns it (test). — `theme::overrides::tests::{on_selection_lifts_against_the_override_not_the_built_in, an_overridden_text_muted_reaches_the_pixel_marker}`
- [x] Overridden conflict token at 256 colours: `token(Ours)` is the cube-nearest of the override, not the pin (test). — `theme::overrides::tests::an_overridden_conflict_token_at_256_is_cube_nearest_not_the_pin`
- [x] Survives a switch: `retheme(Light)` then `retheme(Dark)` → `token(Selection)` is the dark override again; light
      gets light's overrides (test). `apply(TerminalChange::Theme)` goes through `retheme` (test). — `session::theme_tests::{retheme_keeps_each_kinds_overrides_across_a_switch, a_re_detected_background_rebuilds_through_retheme}`
- [x] `bg` overridden: a graph row's cells keep `Color::Reset` bg; the commit editor's fill is the override (buffer
      tests). — `session::theme_tests::an_overridden_bg_fills_the_commit_editor_and_never_the_graph`
- [x] `NO_COLOR`: tokens equal the no-override `NO_COLOR` theme; no contrast miss in `config_problems`; a bad colour
      still listed (test). — `theme::overrides::tests::no_color_ignores_overrides`, `session::theme_tests::no_color_draws_the_same_tokens_with_or_without_overrides`, `event_loop::tests::theme_problems::no_color_drops_the_miss_and_keeps_the_bad_colour`, `config::theme_tests::no_color_reports_no_contrast_miss`
- [x] Capture `docs/shots/m8-16-theme-problems.txt`: graph with the bottom row `config: 2 problems · ? shows them`,
      then F2 notes showing one contrast miss and one `lane-3` problem. Colour of an applied override is **unverified**
      by capture (rule 5): say so in the handoff. — `event_loop::tests::theme_problems::capture_theme_problems`

## Touches
- `Theme::new` callers: session.rs:171, :198 → `with_overrides`/`retheme`; tests/examples/benches keep `new` (list in
  scout). Check with `rg 'Theme::new' crates` — production hits must be zero after.
- Built-in readers: `rg '\.chrome\(\)' crates/graft-ui/src/theme` — only the constructor and `contrast_misses` after.
- Caches: strips (view/strips.rs:45–52, turn.rs:254) — no key change (decision 8); state the check.
- Config: `SCHEMA`, `unknown_keys`, `ConfigWarning` words in `keymap_from` (config/keys.rs:71), config tests'
  unknown-key cases; ticket 10's edits in the same files.
- `config_problems` count and notice (view/frame/bars.rs) — order: file warnings, key warnings, theme misses.
- Paint sites of `Token::Bg` (decision 6) — unchanged code, new buffer test.
- Snapshots: graft-ui theme/snapshots may move if `Theme`'s Debug changes — review each, don't bulk-accept.
- MAP.md: graft-ui `## theme` (`with_overrides`, `contrast_misses`, `ContrastMiss`, `Token::from_name`, `rgb` table
  note, `on_selection`/`raster_muted` source); graft `## config` (`ThemeOverrides`, new warnings, `theme_problems`),
  `Session::retheme`.
- Keys: none claimed. Journal/undo: none.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~55k tokens.

## NOTES (builder, 2026-10-07)
- Scout used; verified against code after 09/09b/10/10c/09c — line refs drifted by a few lines (SCHEMA load.rs:208, not :217), substance held. It sufficed.
- **Colour unverified (rule 5):** the capture is a character grid. Applied override colours are proven only by buffer tests (cell `bg == Color::Rgb(..)`), never by eye.
- **11: theme rows call `Session::retheme`.** Ticket 11 (palette) not merged at build time; no palette code exists.
- Deviations: `ThemeOverrides` also derives `Eq` (Config/Loaded derive Eq; Token, Rgb are Eq). `[theme] dark = 3` (kind not a table) → `UnknownKey { theme.dark }`, as `[keys]` does for `keys.graph = 3`. `BadColour.value` for a non-string is its TOML scalar (`3`) or kind (`array`). High-contrast tables accept `high-contrast` and `high_contrast`; problems are labelled `[theme.high-contrast]` (`ThemeKind::name`, the `ui.theme` spelling).
- `with_overrides`/`contrast_misses`/`Token::from_name` live in `theme/overrides.rs` to keep `theme/mod.rs` under 400 lines (380). `session.rs` was 473 lines before this ticket, 494 after (inline tests); new session tests went to `session/theme_tests.rs`. Splitting session.rs's inline tests is out of scope.
- `Session::retheme` also sets `rendering.theme`, so the F2 Theme row follows a palette switch.
- Tests were written in the same pass as the code, not strictly red first.
