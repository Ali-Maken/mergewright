# G-theme — change one colour in config; a colour that misses a floor is named

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**.
Needs: `look-cursor-row-and-dark-b` (`contrast`, `ContrastRule`, `CONTRAST_RULES`), `keymap-table-and-translate`
(`AppState::config_problems`, the `config: N problems` status).

## Read first
- `PROMPT.md` §5 "Colour tokens" bullet and its four sub-bullets (lanes are not a theme's to change)
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows "Change one colour", "Contrast floor"; Journeys J10;
  Interfaces `graft-ui::theme`
- `docs/research/themes.md` — the gitui / helix `inherits` / VS Code `colorCustomizations` rows only
- `crates/graft-ui/MAP.md` `## theme`; `crates/graft/MAP.md` `## config` (`ThemeChoice`, `ConfigWarning`)

## Scope
In:
- Config: `[theme.dark]`, `[theme.light]`, `[theme.high_contrast]`; keys are `Token::name` spellings (`selection`,
  `text-muted`, …); values `"#RRGGBB"`. Applied over the built-in theme of that kind, before depth reduction (256/16
  use the same nearest mapping; `NO_COLOR` ignores overrides).
- Warnings (non-fatal, into `config_problems`): unknown token; `lane-N` (`lanes are fixed — PROMPT §5`); bad colour;
  a resulting pair under a `CONTRAST_RULES` floor (`selection on bg 2.1:1 — under 3.0`). The override is still
  applied: his choice, named.
- Palette theme rows (G-palette) and OSC 11 re-query keep his overrides for the kind they switch to.
Out: per-widget attributes, theme files, palette-inheriting default (all R in Prior art). Lane hues.

## Signatures
```rust
// graft-ui::theme
pub struct ContrastMiss { pub rule: ContrastRule, pub ratio: f32 }
impl ContrastMiss { pub fn words(&self) -> String; }
impl Theme { pub fn with_overrides(self, overrides: &[(Token, Rgb)]) -> (Theme, Vec<ContrastMiss>); }
impl Token { pub fn from_name(name: &str) -> Option<Token>; }
// graft::config
pub struct ThemeOverrides { pub dark: Vec<(Token, Rgb)>, pub light: Vec<(Token, Rgb)>, pub high_contrast: Vec<(Token, Rgb)> }
// Config gains `theme: ThemeOverrides`
```

## Acceptance
- [ ] J10 (person step, TESTING.md row): `[theme.dark] selection = "#3A3F4B"` → restart → the cursor row is that
      colour (buffer test reads the cell's bg).
- [ ] `selection = "#15171C"` → status `config: 1 problem · ? shows them`; `?` lists `selection on bg 1.0:1 — under
      3.0` (test).
- [ ] `lane-3 = "#FF0000"` → warning, lanes unchanged (test).
- [ ] Override survives a theme switch to its kind and back (test).
- [ ] `NO_COLOR` set → overrides ignored, no warning (test).

## Touches
- `Theme::new` callers (startup, OSC 11 re-query, palette rows) — all must pass overrides; strip/row caches keyed by
  theme (an override is part of the key); config loader's unknown-key warnings (`[theme.*]` known now); MAP.md
  graft-ui, graft; README config section is G-docs'.

## Scout questions
- Every call site of `Theme::new` (file:line); is the theme part of each cache key as a value or as the kind only?
- Does anything paint `bg` itself, or is the terminal's background left showing (PROMPT §9 translucency)? An override
  of `bg` must not start painting it where it was transparent — say what happens.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~45k tokens.
