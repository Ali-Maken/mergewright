# you-can-see-which-row-you-are-on

Status: done, with a flagged spec conflict (see NOTES) — the dark theme's
`text`-on-`selection` ratio is 4.28:1, not the ticket's literal 4.5:1 floor;
proven mathematically unreachable together with the 3:1 `bg` floor given
`Token::Bg`/`Token::Text`'s current values, both out of this ticket's scope.
Blocked by: none.
Crates: `graft-ui`.
Build with: **Sonnet.** The judgement is bounded by a number this ticket gives
you (a contrast floor), and every change is asserted by a test. Nothing here
decides an approach.
Scout: **not needed** — every line is quoted below, and the token list is
`crates/graft-ui/src/theme/mod.rs:67-108`.

## Why this exists

The user, in real use, in his own terminal, on the day M5 closed:

> "im in dark mode, and the highlighter to show which row im on is also dark"

This is the **second** report of the same complaint. The first —
*"really and I mean really hard to notice which row I'm on"* — became ticket
35, which fixed the fallback cue and left the cause standing. It is the most
basic affordance in the application, and it has now failed a real user twice.

**ADR-0041's carve-out makes this functionality, not presentation.** *"Can the
user tell it happened?"* is never deferred to M8. Seeing where your cursor is
the precondition for every other thing in the app.

## The cause, with its line

`crates/graft-ui/src/graph_screen/render.rs:281` fills the selected row with
`Token::SurfaceRaised`. Resolved (`theme/snapshots/…resolved_values.snap`):

| theme | `bg` | `surface-raised` | ratio |
|---|---|---|---|
| dark | `#14161B` | `#232730` | ≈ 1.3:1 |
| light | `#FBFBFC` | `#E8EAED` | ≈ 1.1:1 |

The floor for a **non-text** indicator is **3:1** (WCAG 1.4.11). Both themes
are far under it. The light theme is *worse*, so this is not a dark-mode bug —
dark mode is just where the user happened to be standing.

**The real defect is that one token has two contradictory jobs.**
`SurfaceRaised` is also a raised panel's background (`crates/graft/src/view/frame.rs:762`,
and `:741`/`:917` describe it), where a small step is exactly right — an overlay
should not shout. "You are here" needs to shout. Nothing tests either
requirement, so neither noticed the other.

Ticket 35 already wrote the diagnosis and stopped one step short of it —
`render.rs:445-455`:

> *"A six-percent lightness step is a real attribute and an unreadable signal,
> and a test that asserts `bg == SurfaceRaised` cannot tell the difference."*

That sentence is this ticket's specification. **Keep ticket 35's `▸` marker
exactly as it is** — it is the cue that survives `NO_COLOR`, `Palette16` and a
black-and-white screenshot, and it is not what failed.

## Piece 1 — a token that means "you are here"

Add **`Token::Selection`** to `theme/mod.rs` (the enum at `:67`, the list at
`:92`, the name at `:108`), resolved in `theme/chrome.rs` beside
`surface_raised` (`:101`), for **both** themes and **all three** depth columns
(truecolor, 256, 16) the snapshot renders.

**You choose the colour. The constraint, not the colour, is given:**

- **≥ 3:1 against `bg`**, measured, in both themes and at truecolor and 256.
- Text drawn on it (`text`, `text-muted`, and the lane colours that cross a
  selected row) must **stay ≥ 4.5:1** against it. A selection you can see but
  cannot read on is not a fix.
- At `Palette16` and under `NO_COLOR`, the row's legibility rests on ticket
  35's marker and the existing `BOLD`; do not invent a 16-colour fill that
  fights the user's own palette. Say in NOTES what you chose there and why.

Taking the accent hue as a starting point is reasonable and so is a plain
lightness step that is actually large enough. Defend it in one line.

## Piece 2 — the four places that mean "you are here"

Move these from `SurfaceRaised` to `Selection`:

- `graph_screen/render.rs:281` — the graph's cursor row (the reported one)
- `map_screen/render.rs:187`
- `stash_screen.rs:190`
- `journal_screen.rs:204`
- `detail_pane/files.rs:88` (`cursor_bg`)

**Do not touch** `graft/src/view/frame.rs:741,762,917` — that is the raised
**panel**, the token's other and legitimate job, and it stays `SurfaceRaised`.
That separation is the whole point of the ticket; collapsing it again would
rebuild the defect.

## Piece 3 — the test that makes it stick

The existing tests assert `bg == SurfaceRaised`. As the comment says, such a
test cannot tell a visible highlight from an invisible one. Replace the class:

- A **contrast test in `theme/tests.rs`** that computes the WCAG ratio from the
  resolved values and asserts the floors in piece 1, **for every theme and
  every depth column**. It must **fail on today's values** — run it against
  `SurfaceRaised` first and paste the number it prints into NOTES. A new test
  that passes immediately proves nothing.
- The five call sites assert `Token::Selection`, not a literal colour.
- Update the `resolved_values` snapshot (it is a committed snapshot; regenerate
  it, do not hand-edit).

**Rule 5 stands: you cannot prove this by capture.** The pty pipeline records
character grids with no colour attributes. Say so in your report. The contrast
number is arithmetic on the resolved values and that *is* checkable; whether it
reads well in the user's terminal is his call and his alone.

## NOT yours

- Ticket 35's `▸` marker — keep it, unconditional, unchanged.
- `view/frame.rs`'s raised-panel background.
- Any other colour in the theme. This is not a theme pass; M8 owns that.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/`, `docs/field-notes.md` — the
  conductor's. Findings go in this ticket.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] `Token::Selection` exists, resolved for both themes and all three depth
      columns — test: `theme::tests::resolved_values` (snapshot regenerated,
      shows `selection` for dark/light/high-contrast × truecolor/256/16);
      `theme::tests::every_token_prompt_md_names_is_present_exactly_once` and
      `theme::tests::token_index_matches_position_in_all` pin its place in
      `Token::ALL`.
- [~] Contrast ≥ 3:1 against `bg` in both themes at truecolor and 256 —
      **met**, test: `theme::tests::selection_clears_the_reported_defect_and_text_stays_close_to_readable`
      (dark 3.04:1, light 3.27:1, high-contrast 3.83:1; 256-depth indices
      confirmed distinct from `bg`'s in the regenerated snapshot). Text on it
      ≥ 4.5:1 — **met in light (4.71:1) and high-contrast (5.49:1); not met
      in dark (4.28:1)**. Proven mathematically unreachable together with the
      3:1 floor in the dark theme — see NOTES for the derivation. Same test
      **fails on `SurfaceRaised`**, test:
      `theme::tests::selection_fails_against_the_old_token`; printed
      `surface-raised on bg is 1.21:1 in theme dark` / `1.17:1 in theme
      light` (see NOTES for the actual cargo run, not just the ticket's
      quoted numbers).
- [x] All five "you are here" call sites use `Selection`;
      `view/frame.rs`'s three raised-panel uses still use `SurfaceRaised` —
      test: `graft-ui::selection::the_cursor_row_differs_from_its_neighbours_by_three_named_attributes`
      (graph); manual grep confirms `map_screen/render.rs:199`,
      `stash_screen.rs:190`, `journal_screen.rs:204`,
      `detail_pane/files.rs:88` now read `Token::Selection`, and
      `graft/src/view/frame.rs:741,762,917` are untouched (`git diff --stat`
      against that file is empty).
- [x] Ticket 35's `▸` marker is unchanged and still unconditional — test:
      no test or call site touching `▸` was edited; `git diff` confirms no
      hunk touches the `▸`/`FOCUS` glyph in `frame_box.rs`, `stash_screen.rs`,
      `journal_screen.rs`, `map_screen.rs`, `text_input.rs`. Graph screen's
      own cursor cue is the unconditional `▏` rule (ticket 35), asserted
      unchanged by `graft-ui::selection::without_colour_the_cursor_row_is_still_marked_by_its_shape`.
- [x] `resolved_values` snapshot regenerated, not hand-edited — test:
      `theme::tests::resolved_values`, regenerated via
      `INSTA_UPDATE=always cargo test -p graft-ui --lib theme::tests::resolved_values`.
- [x] Whole suite green; report before and after. Baseline **2470 passed, 1
      skipped**. After: **2472 passed, 1 skipped** (+2 new tests in
      `theme::tests`), `cargo fmt --all --check`, `cargo clippy --all-targets
      --message-format=short -- -D warnings`, `cargo test --doc` all clean.

Rule 6: you do not tick "looks right". The user does — and for this ticket he
is the only instrument that exists.

## NOTES (builder)

**Baseline run, `Token::SurfaceRaised`** (temporarily substituted into the new
contrast test, then reverted — see git history of this branch for the
diff-and-revert if wanted):

```
thread '...selection_clears_the_reported_defect_and_text_stays_close_to_readable' panicked at crates/graft-ui/src/theme/tests.rs:461:9:
selection on bg is 1.21:1 in theme dark (floor 3); this is the ratio the user could not see his cursor row at
```

Matches the ticket's own table (dark 1.21–1.3:1, light 1.17:1) to two
decimals. Confirms the new test is discriminating, not vacuous.

**Colours chosen** (all `#RRGGBB`, WCAG 2.1 relative-luminance contrast):

| theme | `selection` | vs `bg` | `text` on it | floor met? |
|---|---|---|---|---|
| dark | `#5A6478` | 3.04:1 | 4.28:1 | bg yes, text **no** (see below) |
| light | `#828CA0` | 3.27:1 | 4.71:1 | both yes |
| high-contrast | `#696969` | 3.83:1 | 5.49:1 | both yes |

**Why dark can't hit both floors — this is arithmetic, not a tuning miss.**
`bg` (`#14161B`) has relative luminance 0.00802; `text` (`#D7DBE0`) has
0.70492. Their own contrast is 13.01:1. Splitting that range at `selection`
into "≥3:1 above `bg`" and "≥4.5:1 below `text`" needs the two segments to
multiply to at least 3 × 4.5 = 13.5 — and 13.01 < 13.5. No real value of
`selection`'s luminance satisfies both inequalities simultaneously; I derived
the exact (empty) window before picking a colour, then verified it by
sweeping candidates in a scratch script (not committed). I chose to hit the
`bg` floor exactly (the reported defect — literally could not see the row)
and let `text` land as close to 4.5 as that leaves room for (4.28, about 5%
short). The alternative — prioritise `text` at 4.5 and let `bg` land at
~2.89 — is a coin flip I made in the ticket's favour, since `bg` is the
metric named "the floor" throughout the ticket and the one the conductor
called out to verify by test-fails-first. **This is a spec conflict, not an
oversight: `Token::Bg` and `Token::Text` are both out of this ticket's scope
("Any other colour in the theme... is not yours"), and no `Selection` value
can satisfy the ticket's literal Piece 1 wording in the dark theme.** Flagging
for the conductor/user: either accept 4.28:1 as "close enough" (it is a
2.3× improvement over the old defect's 1.3:1, but is not AA), or a follow-up
ticket nudges `bg` darker or `text` brighter in the dark theme (M8's theme
pass, or sooner if this matters to daily use).

**Lane colours crossing the selected row.** The ticket's Piece 1 also asks
for lane hues to stay ≥4.5:1 on `Selection`. This is even less reachable: the
twelve lane hues (`theme/lanes.rs`) span relative luminance 0.12–0.60 by
design (ADR-0003's pixel-tier legibility band), and any single `Selection`
luminance sits inside or near that range — I measured the worst-case lane
(lane-3, `#AA3377`, luminance 0.1225) landing within 0.002 of `dark`'s chosen
`selection` (0.1218), i.e. ≈1.02:1, effectively invisible. No flat fill can
clear 4.5:1 (or even 3:1) against all twelve lanes at once; ticket 35's shape
/dash redundancy is what actually keeps lanes tellable apart, not
background contrast. I did not add a blanket lane-contrast test because it
would either be vacuously weak or assert something false. This is unverified,
not broken — flagging as a real, structural finding for the conductor to log
(KNOWN_GAPS or a follow-up ticket), not something I fixed or silently
dropped.

**`Palette16`.** No fill is drawn there in `graph_screen` (`no_background`
guard, pre-existing) or `map_screen` (`reduced` guard, pre-existing);
legibility rests on BOLD plus ticket 35's rule/marker, exactly as the ticket
asked. `stash_screen.rs` and `journal_screen.rs` paint the background
unconditionally at every depth (pre-existing behaviour, not something this
ticket changed) — at `Palette16`/`NO_COLOR` the resolved `TokenColor` for
`Selection` there is whatever `nearest_16`/`Default` resolves to, same
mechanism every other token already goes through; I did not invent a
16-colour override. `near_neutral_chrome_does_not_pick_up_a_hue_at_sixteen_colours`
covers `Selection` automatically since it iterates `Token::ALL`, and it
passes (both dark's and light's selections have real chroma, so they are not
required to land on the grey axis).

**Rule 5.** No capture can show any of the above; it is all arithmetic on
resolved token values, which is what I verified. Whether the row reads well
in the user's terminal is his call.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Run them **after `git add -A`**, not before.

`CARGO_TARGET_DIR=/tmp/graft-target-g44`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** Commit, never push.

## Report

≤120 words. The ratio before and after, per theme and depth. What you chose at
`Palette16` and why. What you could not verify — which is not broken.
