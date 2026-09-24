# conflict-colour-tokens: six colour roles for the conflict resolver, with their depth fallbacks

Status: done
From: user picks 2026-09-24 item 1 and P13 (`.scratch/m6-conflicts/user-picks-2026-09-24.md`); ADR-0044 §1.
Amended 2026-09-24 (spec-only pass) after scout `.scratch/m6-conflicts/scouts/54-56.md`. The planner re-read
`theme/mod.rs:141-196`, `theme/chrome.rs:25-133`, `theme/depth.rs:130-138` and `theme/tests.rs:26-49`.
Build with: builder-sonnet — one module, and every value and path is decided here.
Needs: none.
Crates: graft-ui only.
Lane: wave 1, lane U, first. Runs in parallel with lane O (`graft-ops` + `graft`).

## Read first
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §1 (the table, α, the 256/16/`NO_COLOR`/light rules).
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24, "Colour roles: six new tokens (P13)" (:937-966).
- `.scratch/m6-conflicts/scouts/54-56.md` "Theme and tokens" (the code as it stands, with lines).
- `crates/graft-ui/MAP.md` §theme (:269-287). **Stale**: it says "dark or light", but `ThemeKind::HighContrast` exists
  (`theme/mod.rs:153`, override only).
- PROMPT.md §5 "Colour tokens" and "Colour depth" (:371-382), as ADR-0044 §5 edited them.

## Decisions (made in this pass)
1. **Three themes, not two.** `HighContrast` gets hues and no tints. Its hues are the dark ones, `#56B4E9`,
   `#CC79A7` and `#E69F00`, because all three are well above 4.5:1 on its black `bg`. The builder measures them in a
   test and records the ratios in NOTES. Its three tint slots hold `bg` (`#000000`). They are never drawn, because
   `tint` answers `None` in this theme.
2. **The tables keep one named argument per colour.** `chrome::build` stays at its 12 arguments. A second
   `const fn conflict(hues, ours, theirs, open, ours_tint, theirs_tint, open_tint) -> ChromeHues` sets the six new slots
   on a built table, with 7 arguments, so no new `too_many_arguments` expect is needed. `DARK`, `LIGHT` and
   `HIGH_CONTRAST` each become `conflict(build(…), …)`. Tints are written as literals: the dark ones from ADR-0044, the
   light ones computed once and pasted in. A test ties each literal to `mix`. There is no `const` float maths.
3. **Fixed palette entries are their own path.** `reduce` (`mod.rs:351`) stays exactly as it is for the twelve
   existing tokens, so no existing colour moves. For the six new tokens, a token-aware step runs first:
   - `pinned(kind, token, depth) -> Option<u8>`. At 256 colours: Dark and HighContrast pin 74, 175, 214 (the hues);
     Dark also pins 23, 53, 58 (the tints). At 16 colours: Dark and HighContrast pin 6, 5, 3 (the hues).
   - With no pin, 256 colours uses the new `depth::nearest_256_cube` (16..=231, never the grey ramp). This is how the
     light theme's six are reduced. 16 colours uses the existing `nearest_16`, for the light theme's hues. That
     answer is unverified and goes in NOTES.
4. **A tint where tints are dropped paints nothing.** `Theme::token(tint)` returns the no-colour `TokenColor`
   variant, the one `reduce` gives under `ColorDepth::None` (the builder names it), and not `bg`. Painting `bg` at 16
   colours would itself be an ANSI block.

## Scope
**In:**
- `Token` gains, **at the end of `Token::ALL`**: `Ours`, `Theirs`, `Open`, `OursTint`, `TheirsTint`, `OpenTint`.
  `Token::name` spells them `ours`, `theirs`, `open`, `ours-tint`, `theirs-tint`, `open-tint`. `TOKEN_COUNT` = 18.
- Dark values exactly as ADR-0044 §1: truecolor `#56B4E9`, `#CC79A7`, `#E69F00`; tints `#284559`, `#4B3445`,
  `#533F13`; 256 pins 74, 175, 214, 23, 53, 58; 16 pins 6, 5, 3; no tints at 16.
- Tint rule: `mix(hue, bg, TINT_ALPHA)`, `TINT_ALPHA = 0.30`, one α. Each tint literal equals `mix` of its hue over its
  theme's `bg` to within one step per channel. That is a test, so the table and the rule cannot drift.
- Light: hues `#0072B2`, `#A8467F`, `#B36B00`; tints are `mix` over the light `bg` (`#FBFBFC`), pasted in as
  literals; 256 through `nearest_256_cube`. Record the resulting light values in NOTES (unverified: he runs dark).
- High contrast: decision 1.
- `Theme::tint(Token) -> Option<TokenColor>`: `Some` only for the three tint tokens, at truecolor and 256, in Dark or
  Light. `None` at 16 colours, under `NO_COLOR`, and in HighContrast. Widgets ask this, never the depth.
- `NO_COLOR`: the three hues resolve to no colour. The widget's glyphs and modifiers carry the state (ticket 55's
  job).

**Out:**
- Drawing anything with the tokens (`conflict-band-and-state-colours`).
- PROMPT.md §5 text (already applied, commit 7f2d7b3d).
- Config overrides. The scout found no config reader of `Token::ALL` and no parsing of token names, so nothing new is
  built.

## The failing test first
`graft-ui/src/theme` tests: `conflict_tokens_resolve_per_depth`.
- A table of the six tokens × {truecolor, 256, 16, `NO_COLOR`} × {Dark, HighContrast} → the exact `TokenColor` of
  ADR-0044 §1 and decision 1, with `tint` answering `None` where they say none.
- Red today: the variants do not exist. The red is a compile failure; say so in NOTES.

## Signatures
```rust
// graft-ui/src/theme/mod.rs
pub enum Token { /* existing 12, */ Ours, Theirs, Open, OursTint, TheirsTint, OpenTint }
pub const TOKEN_COUNT: usize = 18;
impl Token { #[must_use] pub const fn is_tint(self) -> bool; }
pub const TINT_ALPHA: f32 = 0.30;
/// `fg` over `bg` at `alpha` (0 = bg, 1 = fg), per channel, rounded. Pure.
#[must_use] pub fn mix(fg: Rgb, bg: Rgb, alpha: f32) -> Rgb;               // graft_core::capabilities::Rgb, as the tables use
const fn pinned(kind: ThemeKind, token: Token, depth: ColorDepth) -> Option<u8>;   // private, decision 3
impl Theme {
    /// A tint token's background, or `None` where tints are dropped (16 colours, NO_COLOR, high contrast).
    /// A non-tint token → `None`, with a debug_assert naming the misuse.
    #[must_use] pub fn tint(&self, token: Token) -> Option<TokenColor>;
}
// graft-ui/src/theme/chrome.rs (private)
const fn conflict(hues: ChromeHues, ours: Rgb, theirs: Rgb, open: Rgb,
                  ours_tint: Rgb, theirs_tint: Rgb, open_tint: Rgb) -> ChromeHues;
// graft-ui/src/theme/depth.rs
pub(super) fn nearest_256_cube(color: Rgb) -> u8;                          // 16..=231 only
```

## Touches (each entry, and how the builder checks it)
- **Readers of `Token::ALL`** (the scout's list): `Theme::new` (`mod.rs:245`); `paint.rs:63`
  (`no_color_paints_nothing_at_all`); `theme/tests.rs:28` (`every_token_prompt_md_names_is_present_exactly_once`,
  which pins **12 names**), `:53`, `:88`, `:315`, `:578`. The status screen F2 and the config reader do **not** read it.
  - Check: `grep -rn "Token::ALL" crates` before and after, with the same hits.
  - `:28` is edited to the 18 names in PROMPT.md §5's order (ADR-0044 §5). This is the one intended edit to an
    existing assertion.
  - The theme sweeps (`:88`, `:315`, `:578`): the builder reads each. The three tints are **backgrounds**. Where a sweep
    asserts foreground contrast or legibility against `bg`, the tints are left out **by name, with a comment citing
    ADR-0044 §1's 1.5–2.5:1 target**. That exclusion is decided here; it does not disable a test. Every hue token
    stays in every sweep. If a hue token fails a sweep (CVD, the grey axis, contrast), stop and report it as a finding:
    the values are the user's pick, so don't retune them.
- **Exhaustive matches on `Token`**, anywhere in the workspace (a missed one fails to compile).
  - Check: `cargo clippy --all-targets` workspace-wide.
- **Contrast targets** (ADR-0044 §1): dark tints 1.5:1–2.5:1 against `bg`; `text` on each tint ≥ 4.5:1; each tint
  below `selection`'s 3.04:1.
  - Check: `dark_tints_meet_their_contrast_targets` (WCAG relative luminance, the formula ticket 44 used);
    `high_contrast_hues_clear_4_5_on_bg`.
- **The 256-colour mapping of the twelve existing tokens** is unchanged, and no conflict token lands on the grey ramp.
  - Check: `existing_tokens_reduce_as_before` (all twelve × three themes × 256/16, equal to the old `reduce`);
    `no_conflict_token_maps_to_the_grey_ramp` (all three themes).
- **Other widgets**: none draw these tokens yet.
  - Check: `grep -rn "Token::Ours\|Token::Open\|Token::Theirs" crates` lists only `theme`.
- **Lane O** (57, 58): no shared crate. Check: `git diff --stat` names only `crates/graft-ui`.
- **Docs**: `crates/graft-ui/MAP.md` §theme: three themes, not "dark or light"; `tint`, `mix`, `TINT_ALPHA`,
  `pinned`, `nearest_256_cube`; the six tokens.

## Acceptance (name the verifying test at tick time)
- [x] `conflict_tokens_resolve_per_depth`: red first (compile failure: variants and `tint` missing), then green.
- [x] `dark_tints_equal_the_mix_rule`, `light_tints_equal_the_mix_rule`
- [x] `dark_tints_meet_their_contrast_targets`, `high_contrast_hues_clear_4_5_on_bg`
- [x] `no_conflict_token_maps_to_the_grey_ramp`
- [x] `existing_tokens_reduce_as_before` (also: `resolved_values` snapshot diff is 18 added rows, zero changed)
- [x] `tint_is_none_at_sixteen_colours_under_no_color_and_in_high_contrast`
- [x] `every_token_prompt_md_names_is_present_exactly_once`, with 18 names.
- [x] No capture: nothing is drawn with the tokens yet (grep `Token::Ours|Open|Theirs` → theme only).
- [x] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small. One context.

## NOTES
- Built 2026-09-24, lane U. Scout used: `.scratch/m6-conflicts/scouts/54-56.md`. No compaction.
- Red: compile failure (no `Token::Ours`…, no `TINT_ALPHA`, no `Theme::tint`), as expected.
- No-colour variant is `TokenColor::Default`: `Theme::token(tint)` gives it where tints are dropped.
- High contrast hues on `#000000` (WCAG): ours 9.10, theirs 6.86, open 9.32.
- Dark tints vs bg / text on tint: ours 1.80 / 7.24, theirs 1.62 / 8.05, open 1.80 / 7.23. All under selection's 3.04.
- Light (unverified, he runs dark): hues `#0072B2` `#A8467F` `#B36B00` → 256: 25, 132, 130; 16: 6, 5, 3 (nearest_16, unverified).
  Tints `#B0D2E6` `#E2C5D7` `#E5D0B0` → 256: 152, 224, 187 (cube). theirs-tint blue is 214.5 before rounding; `#D7` pasted, test allows ±1.
- Sweep exclusion: only `near_neutral_chrome_does_not_pick_up_a_hue_at_sixteen_colours` skips the tints (by `is_tint`, comment cites ADR-0044 §1). `readable_tokens_…` uses its own list, untouched. `no_color_empties_every_token…` and `paint.rs:63` pass with 18 tokens. No hue failed any sweep.
- `TOKEN_COUNT` made `pub` per signature.
- Colour is **unverified** by capture (rule 5); values checked only by the unit tests above.
- **Touches:** `Token::ALL` readers — grep before/after same hits (`theme/mod.rs`, `theme/tests.rs`, `paint.rs:63`); `resolved_values` snapshot re-accepted, diff is additions only; exhaustive `Token` matches — workspace clippy clean; other widgets — grep shows no draw of new tokens; lane O — `git diff --stat` only `crates/graft-ui`; MAP.md §theme updated.
