# conflict-colour-tokens: six colour roles for the conflict resolver, with their depth fallbacks

Status: ready
From: user picks 2026-09-24 item 1 and P13 (`.scratch/m6-conflicts/user-picks-2026-09-24.md`); ADR-0044 §1.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: in `graft-ui::theme`, how `Token`, `Token::ALL`, `Token::name`, `TokenColor` and `Theme::token` resolve a
value per `ThemeKind` and per colour depth (truecolor, 256, 16, `NO_COLOR`); whether a high-contrast theme exists (MAP
says dark and light only); every reader of `Token::ALL` in the workspace (status screen F2 `RenderingView`, config
round-trips, tests); how the existing nearest-colour mapping picks a 256 entry. ≤ 1 page, code not intent.
Needs: none.
Crates: graft-ui only.
Lane: wave 1, lane U, first. Runs in parallel with lane O (`graft-ops` + `graft`).

## Read first
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §1 (the table, α, 256/16/`NO_COLOR`/light rules).
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24, "Colour roles: six new tokens (P13)" (:937-966).
- `crates/graft-ui/MAP.md` §theme (:269-287); `crates/graft-ui/MAP.md` line 12-15 (who will read the tokens next).
- `docs/design/04-*.md` §4.6 (colour depth), PROMPT.md §5 "Colour tokens" and "Colour depth" (:371-382).
- Source, only after the MAP: `graft-ui/src/theme*`.

## Scope
**In:**
- `Token` gains, **at the end of `Token::ALL`**: `Ours`, `Theirs`, `Open`, `OursTint`, `TheirsTint`, `OpenTint`;
  `Token::name` spells them `ours`, `theirs`, `open`, `ours-tint`, `theirs-tint`, `open-tint`.
- Dark values exactly as ADR-0044 §1: truecolor `#56B4E9`, `#CC79A7`, `#E69F00`; tints `#284559`, `#4B3445`,
  `#533F13`; **fixed** 256 entries 74, 175, 214, 23, 53, 58; 16 colours cyan (6), magenta (5), yellow (3), tints none.
- Tint rule: `mix(hue, bg, TINT_ALPHA)`, `TINT_ALPHA = 0.30`, one α. The dark tint constants must equal what `mix`
  gives from the dark `bg` (`#14161B`) to within one step per channel — a test, so the table and the rule cannot drift.
- Light theme: hues `#0072B2`, `#A8467F`, `#B36B00`, tints by `mix` over the light `bg`; 256 by nearest entry **in the
  6×6×6 cube (16-231), never the grey ramp (232-255)**. Record the resulting light values in NOTES (unverified: he
  runs dark).
- `Theme::tint(Token) -> Option<TokenColor>`: `Some` only for the three tint tokens at truecolor and 256; `None` at 16
  colours, under `NO_COLOR`, and in a high-contrast theme if one exists. Widgets ask this, never the depth.
- `NO_COLOR`: the three hues resolve to no colour (the widget's glyphs and modifiers carry the state; U2's job).

**Out:**
- Drawing anything with them (`conflict-band-and-state-colours`).
- PROMPT.md §5 text (ADR-0044 §5, the conductor applies it).
- User-config overrides of these tokens beyond what every token already gets (if config reads `Token::ALL`, they come
  for free; nothing new is built).

## The failing test first
`graft-ui/src/theme` tests: `conflict_tokens_resolve_per_depth`.
- A table: each of the six tokens × {truecolor, 256, 16, `NO_COLOR`} for the dark theme → the exact `TokenColor` of
  ADR-0044 §1 (`None` from `tint` where the ADR says none).
- Red today: the variants do not exist (a compile failure is the red; say so in NOTES).

## Signatures
```rust
// graft-ui/src/theme.rs (or its child the scout names)
pub enum Token { /* existing …, */ Ours, Theirs, Open, OursTint, TheirsTint, OpenTint }
pub const TINT_ALPHA: f32 = 0.30;
/// `fg` over `bg` at `alpha` (0 = bg, 1 = fg), per channel, rounded. Pure.
#[must_use] pub fn mix(fg: (u8, u8, u8), bg: (u8, u8, u8), alpha: f32) -> (u8, u8, u8);
impl Theme {
    /// A tint token's background, or `None` where tints are dropped (16 colours, NO_COLOR, high contrast).
    /// Any non-tint token → `None` (debug_assert names the misuse).
    #[must_use] pub fn tint(&self, token: Token) -> Option<TokenColor>;
}
```
`Theme::token(OursTint | TheirsTint | OpenTint)` returns what `tint` returns, or the theme's `bg` where `tint` is
`None`, so a widget that calls the wrong one paints no tint rather than a full-strength ANSI block.

## Touches (each entry, and how the builder checks it)
- **Every reader of `Token::ALL`** (the scout's list): status screen F2, config round-trip, name tests.
  - Check: `grep -rn "Token::ALL" crates` before and after; each reader's test runs green; if F2 draws the token list,
    its capture changes and is regenerated (`GRAFT_UPDATE_SHOTS=1`, a red run then green), named in NOTES.
- **Exhaustive matches on `Token`** anywhere in the workspace (they fail to compile if missed).
  - Check: `cargo clippy --all-targets` workspace-wide.
- **Contrast targets** (ADR-0044 §1): dark tints 1.5:1-2.5:1 against `bg`, `text` on each tint ≥ 4.5:1, each tint
  below `selection`'s 3.04:1.
  - Check: `dark_tints_meet_their_contrast_targets` (WCAG relative luminance, the same formula ticket 44 used).
- **256 mapping:** no tint ever lands on the grey ramp.
  - Check: `no_conflict_token_maps_to_the_grey_ramp`, both themes.
- **Other widgets:** none draw these yet. Check: `grep -rn "Token::Ours\|Token::Open" crates` lists only theme.
- **Docs:** `crates/graft-ui/MAP.md` §theme (`tint`, `mix`, `TINT_ALPHA`, the six tokens).
- **Other crates:** none. Check: `git diff --stat` names only `crates/graft-ui` and `docs/shots` (if F2 changed).

## Acceptance (name the verifying test at tick time)
- [ ] `conflict_tokens_resolve_per_depth`: red first, then green.
- [ ] `dark_tints_equal_the_mix_rule`
- [ ] `dark_tints_meet_their_contrast_targets`
- [ ] `no_conflict_token_maps_to_the_grey_ramp`
- [ ] `tint_is_none_at_sixteen_colours_and_under_no_color`
- [ ] `token_names_round_trip` (existing or new) includes the six names.
- [ ] No capture: nothing is drawn with the tokens yet (U2 ends with the capture). If F2's capture changed, it is
      named in NOTES with its "what right looks like" line updated.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small. One context.

## NOTES
