# ADR-0044 — Conflict colours by position, the band between panes, and one confirm-key rule

Date: 2026-09-24
Status: accepted. The user tried the look-and-finish prototype (`prototypes/m6-conflict-look`), said *"demo looks
better than before"*, and approved the conductor's picks table: *"ok i guess"* (`docs/field-notes.md` 2026-09-24;
`.scratch/m6-conflicts/user-picks-2026-09-24.md` items 1, 3, 5, 13, 14, P13). Lukewarm: watch these in daily use.
Design: `docs/design-notes/m6-conflicts.md` "Amendment 2026-09-24: look and finish" (P11–P13);
`docs/design-notes/m6-usability-fixes.md` (P14–P16). Study: `docs/research/conflict-visuals-and-finish.md` (CV).

## Context

- *"there was no color and lines like there on other tools on market"* (field notes :61). The three panes had muted
  titles and a reversed cursor rule, nothing else. Whether a block is open or taken, and which side a result line came
  from, is **state**, so it falls under ADR-0041's carve-out ("can the user tell"), not M8's colour pass.
- *"after enter, the c part was confusing"* (:60). The finish summary committed on `⏎` from a file row, where `⏎`
  everywhere else acts on the row under the cursor, while `y` confirms every preview (field notes :58).
- The push box answered `y` by cancelling without a word; `⏎` in the commit editor's subject went to the body while
  the finish summary's subject committed. Three places, three confirm keys.

## Decision

### 1. Six colour roles, named by position

Colour follows **position**: the left pane is git's stage 2, the right pane stage 3. A pane's hue never moves; the role
word in its title (`yours · main`, `upstream · main`) says whose side it is (ADR-0043 §4). Named by position because
the names are config names he may set, and owner words (`yours`/`theirs`) swap sides in a rebase.

| token | used for | dark, truecolor | 256 | 16 | `NO_COLOR` |
|---|---|---|---|---|---|
| `ours` | left title, left rule words, left separator glyphs | `#56B4E9` | 74 | 6 cyan | none: glyphs carry it |
| `theirs` | right title, right rule words, right separator glyphs | `#CC79A7` | 175 | 5 magenta | none: glyphs carry it |
| `open` | `? open`, open blocks' separators, list `n of m open` / `keep or delete` / `⏎ marks done` | `#E69F00` | 214 | 3 yellow | BOLD |
| `ours-tint` | background of the left block rows, and of result rows taken from the left | `#284559` | 23 | none | none |
| `theirs-tint` | the same for the right | `#4B3445` | 53 | none | none |
| `open-tint` | background of the result placeholder rows | `#533F13` | 58 | none | none |

- **Tint = mix(hue, `bg`, α), α = 0.30** (his pick, "medium"), one α for all three. Targets: tint against `bg`
  1.5:1–2.5:1, `text` on a tint ≥ 4.5:1. On `#14161B`: 1.8, 1.6, 1.8 (WCAG formula). A tint stays below `selection`
  (3.04:1, the fill he could see, field notes :50), so "you are here" stays the strongest cue.
- **Okabe–Ito pair** (his pick): sky blue and reddish purple. Never red/green (CV §1.3). `added`/`removed` are not
  used here; neither are lane hues, which mean a branch, not a side.
- **Text keeps the theme's own colour** on a tint (VS Code, ediff, Neovim; CV §1.3).
- **256 colours:** the fixed entries above, not nearest-colour mapping. The nearest entry to a dark tint is on the grey
  ramp (232–255), and the hue is lost. A theme without pinned entries takes the nearest entry **in the 6×6×6 cube
  (16–231)**, never the grey ramp.
- **16 colours, and the high-contrast theme:** no background tints (ANSI backgrounds are full-strength blocks and text
  on them is hard to read). Hue goes on titles, rule words and separator glyphs only; the side not taken is
  `text-muted`. Precedent: `Theme::syntax` and the side-by-side diff go plain at `Palette16`.
- **`NO_COLOR`:** glyphs and modifiers only: DIM on the side not taken, BOLD on `? open`, the reversed cursor rule as
  built.
- **Light theme:** hues `#0072B2`, `#A8467F`, `#B36B00`, tints at the same α over the light `bg`. **Unverified**: he
  runs dark.
- M8's colour pass may retune the **values**; the **roles** stay.

### 2. The band: separator style A

At ≥ 180 columns the separator between yours | result and result | theirs stays **one cell** (ADR-0043 §2 holds). On a
plain row it is `│` in `border`. On a block's rows it is the link:

| block | left separator | right separator | yours rows | result rows | theirs rows |
|---|---|---|---|---|---|
| open | `?` on the rule row, `┃` below, in `open` | the same | `ours-tint` | `open-tint`; `? open · o t b` in `open`, BOLD | `theirs-tint` |
| took yours | `▶` on the rule row, `┃` below, in `ours` on `ours-tint` | `┊` in `text-muted` | `ours-tint` | `ours-tint` | no tint, `text-muted` |
| took theirs | `┊` in `text-muted` | `◀` on the rule row, `┃` below, in `theirs` on `theirs-tint` | no tint, `text-muted` | `theirs-tint` | `theirs-tint` |
| took both | `▶` / `┃` in `ours` | `◀` / `┃` in `theirs` | `ours-tint` | first `ours.len()` lines `ours-tint`, the rest `theirs-tint` (git's order) | `theirs-tint` |
| took both, then a hand edit | as took both | as took both | `ours-tint` | no tint (line origin unknown) | `theirs-tint` |
| file done | `│` | `│` | the done card, no tint | | |

- **Amended 2026-09-25 (ticket 61, find-and-pin 1):** the "took both, then a hand edit" row never draws. A hand edit
  of the result leaves no conflict markers, so the re-read finds no block, and the file shows `no conflict markers · ⏎
  marks it resolved` (`crates/graft/src/update/resolver.rs:11-15`; test
  `a_hand_edited_both_take_reads_as_no_block_after_the_reread`). The row stays as the rule, should a block ever survive
  a hand edit. Nothing is built for it.

- A tint fills its pane's **full width** on those rows, the separator cell included (ediff `:extend`), which makes one
  band across two panes.
- The cursor block's separator glyphs are BOLD; other blocks the same colours, not bold.
- ASCII tier: `?`, `|`, `>`, `<`, `:` stand for `?`, `┃`, `▶`, `◀`, `┊`.
- Card (< 180 columns): no glyph change; each half or stacked side in its own tint, labels in their hue, BOLD; a taken
  block in the taken side's tint.
- Rejected: curved bands (a cell grid cannot draw them; no terminal tool tries, CV §2); VS Code's side bracket and
  "colour only" (he picked A in the prototype).

### 3. One confirm-key rule

1. **`y` confirms** every Ghost Preview, every confirm box with **one acting choice**, and the finish summary's file
   list, whatever the cursor is on. The cursor still opens on Cancel (ADR-0038 §1). A box that takes `y` says so first
   in its footer: `y <verb>`.
2. **Except**, where `y` is swallowed like any other key and the footer shows no `y`:
   - the **force-push (lease) box**: it rewrites other people's history, and lazygit's users force by habit on the one
     key (PP §1.1, #3468). Its answer stays the cursor's (`→ ⏎`);
   - the **undo-after-push box**: two acting choices, so the answer is the cursor's;
   - the **journal revert** preview and its drift box (his ruling, 2026-09-23).
3. **`⏎` acts on what is under the cursor** (keymap §2.2, ADR-0038 §1): a button presses that button; a file row in the
   finish summary shows or hides its diff.
4. **`⏎` in a subject field commits**, in the commit editor and in the finish summary alike (lazygit; CS §1.1). The
   message has just been typed, so the key is anticipated (UX §1.1, KLM Rule 3). `⇥` reaches the body, where `⏎` is a
   newline. `⌃⏎` commits from any field (`⌃⏎` is unproven in Zed, KNOWN_GAPS :206, so it is never the only way).

### 4. The hint bar keeps the way out and the risky key in words (amends ADR-0039's third amendment)

His pick for the resolver at 120 columns (option A, `.scratch/m6-conflicts/user-facts-m632.md`): after the three most
relevant hints take their full rung, a hint marked **keeps its words** takes its full rung next, before any other hint
is raised from its bare key. The resolver marks `d start over` and `esc close`. Everyday keys (`j/k file`,
`M mergetool`) go bare first. Monotonicity in width (`no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`)
still holds: it is still the longest prefix of one fixed sequence.

### 5. PROMPT.md §5 — exact text (the conductor applies it)

Replace the **Colour tokens** line's first sentence:

> - **Colour tokens:** `bg`, `surface`, `surface-raised`, `selection`, `border`, `text`, `text-muted`, `accent`, `added`, `removed`, `warning`, `info`, `lane-1…12`.

with:

> - **Colour tokens:** `bg`, `surface`, `surface-raised`, `selection`, `border`, `text`, `text-muted`, `accent`, `added`, `removed`, `warning`, `info`, `ours`, `theirs`, `open`, `ours-tint`, `theirs-tint`, `open-tint`, `lane-1…12`.

and append, after that line's `selection` parenthesis:

> (The six conflict tokens were added in M6, ADR-0044: `ours` and `theirs` are the conflict resolver's left and right sides by position — git's stages 2 and 3 — and are never red and green; `open` marks a block still to decide; each `-tint` is a background mixed 30 % of its hue into `bg`. Whether a block is open or taken is feedback, so the roles ship now; M8 may retune the values.)

Replace the **Colour depth** line:

> - **Colour depth:** use truecolor, and fall back to 256 and then 16 colours using perceptual nearest-colour mapping (not naive rounding).

with:

> - **Colour depth:** use truecolor, and fall back to 256 and then 16 colours using perceptual nearest-colour mapping (not naive rounding). **Exception, the six conflict tokens:** each carries a fixed 256-colour entry, because the nearest entry to a dark tint lies on the grey ramp and loses its hue. At 16 colours, in the high-contrast theme and under `NO_COLOR` the three tints are dropped (ANSI backgrounds are full-strength blocks); the glyphs and the hues of titles and separators carry the state.

## Consequences

- `graft-ui::theme` gains six `Token`s at the end of `Token::ALL` and a `Theme::tint` that returns `None` where tints
  are dropped; every widget asks it rather than testing the depth itself.
- Colour is now asserted by **buffer tests** (cell fg, bg, modifier per state row, ADR-0041); captures still carry no
  colour (rule 5), so legibility stays his to judge by eye (rule 6), in Zed, dark, 187 columns.
- The finish summary, the push box and the commit editor change keys; every walk, capture and TESTING.md row naming
  `⏎` there changes (the specs list them). The journal revert and the force-push box keep `→ ⏎`.
- Keymap §2.3b, §2.3c, §2.6, §2.6b and §2.10 carry this rule (edited 2026-09-24).

## Related

ADR-0038 §1 (cursor on Cancel), §4 (undo after push); ADR-0039 (hint ladder); ADR-0041 (feedback is function);
ADR-0043 (three panes, names by position); CV §1–§3; `docs/research/push-pull.md` §1.1; `docs/research/commit-stash.md`
§1.1, §2.
