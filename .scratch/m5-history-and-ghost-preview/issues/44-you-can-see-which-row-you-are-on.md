# you-can-see-which-row-you-are-on

Status: ready
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

- [ ] `Token::Selection` exists, resolved for both themes and all three depth
      columns — test: …
- [ ] Contrast ≥ 3:1 against `bg` in both themes at truecolor and 256, and text
      on it ≥ 4.5:1 — test: … ; **and the same test fails on `SurfaceRaised`**,
      with the number it printed quoted in NOTES.
- [ ] All five "you are here" call sites use `Selection`; `view/frame.rs`'s
      three raised-panel uses still use `SurfaceRaised` — test: …
- [ ] Ticket 35's `▸` marker is unchanged and still unconditional — test: …
- [ ] `resolved_values` snapshot regenerated, not hand-edited — test: …
- [ ] Whole suite green; report before and after. Baseline **2470 passed, 1
      skipped**.

Rule 6: you do not tick "looks right". The user does — and for this ticket he
is the only instrument that exists.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Run them **after `git add -A`**, not before.

`CARGO_TARGET_DIR=/tmp/graft-target-g44`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** Commit, never push.

## Report

≤120 words. The ratio before and after, per theme and depth. What you chose at
`Palette16` and why. What you could not verify — which is not broken.
