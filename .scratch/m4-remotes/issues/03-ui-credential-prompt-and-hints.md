# ui-credential-prompt-and-hints: masked text field, the credential prompt widget, a wrapping confirm consequence, remote footer hints

Status: done
Build with: Sonnet — taking the planner's recommendation: plain widgets with TestBackend snapshots, no state, no I/O.
Crates: graft-ui
Needs: none

## Read first

- `docs/design-notes/m4-remotes.md` — New UI (Push box, Credential prompt, Hints); Hard choice 2 ("must never" list — the masked field is one of its rails)
- `.scratch/m4-remotes/scout.md` §Text input (lines 56–61), §Key routing (68–75)
- `crates/graft-ui/MAP.md` §lib.rs, §confirm (the `confirm` line), §text_input (whole), §graph_screen (`GraphView` footer-related fields and the hint line), §stash_screen (the shape of a small overlay with a `FieldView`)
- `docs/design/01-view-mockups.md` §1.14 (lines 273–287); `docs/design/02-keymap.md` §2.14 (lines 203–210)
- M3 review 06 Minor 11 (`docs/reviews/m3-06-undo-and-stash-ui.md` lines 324–328): the confirm box overdrawing the footer at 120 cols — do not repeat it

## Scope

In — in this order (each piece a commit):

1. **Masked field.** `text_input::FieldView += masked: bool` (default `false`; every constructor/literal site updated). `FieldWidget` draws one `•` per character when masked (ASCII tier: `*`), the cursor at the same character column as unmasked, no guide, no counter (a counter would leak the length — leave `counter` drawn only when `!masked`). Horizontal scroll logic unchanged (every glyph is width 1 when masked).
2. **`credential_prompt` module.** `CredentialView { title: SanitizedText, prompt: SanitizedText, field: FieldView, note: SanitizedText, footer: SanitizedText }`; `CredentialPrompt { view, theme }` widget: a bordered box like `confirm` (title in the top border, `▏` before the field row, the note row dimmed, the footer in the bottom border); `credential_area(frame: Rect) -> Rect` centred, width `min(frame.width - 4, 72)`, 5 rows, never exceeding the frame; below 40 columns or 7 rows draw a one-line `terminal too small for the credential prompt` centred instead (the app still holds the field, so `Enter`/`Esc` work blind — say so in the doc). Long prompt text is `fit` to the width with `…`. Nothing outside its `Rect` at any size.
3. **Confirm consequence wraps.** `ConfirmView.message` may be longer than the box: wrap it on spaces to the inner width, `confirm_rows` grows by the extra lines (cap 4 message lines, the rest `…`), buttons stay on the last row above the border. Existing snapshots for one-line messages unchanged.
4. **Footer hints.** Graph-screen hints gain `f fetch  p pull  P push` after `S stash` and before `J journal` (the drop-from-the-right rule keeps them at ≥ 120 cols; below that they may drop like the others — record which widths keep them). While the screen's `Status::Busy` text is shown, hints are already replaced by it — no change.
5. Snapshot tests (`TestBackend`, 80×24 and 160×48): credential prompt masked (`••••••••` visible, no plaintext), unmasked username prompt, too-small; push confirm with a two-line consequence at 80 cols and one line at 160; footer with the three new hints at 160 and at 120, and without them at 80 if they drop.
6. MAP.md lines.

Out: any key handling or state (the app owns `CredentialPrompt` state, G5b); colours as evidence (captures carry none — the masked glyph is text, which is the point).

## Signatures

```rust
// crates/graft-ui/src/text_input.rs
pub struct FieldView { pub label: SanitizedText, pub text: SanitizedText, pub cursor: usize, pub guide_at: Option<u16>, pub counter: Option<(usize, usize)>, pub focused: bool, pub masked: bool }
// crates/graft-ui/src/credential_prompt.rs
pub struct CredentialView { pub title: SanitizedText, pub prompt: SanitizedText, pub field: FieldView, pub note: SanitizedText, pub footer: SanitizedText }
pub struct CredentialPrompt<'a> { pub view: &'a CredentialView, pub theme: &'a Theme }
impl Widget for CredentialPrompt<'_> {}
pub fn credential_area(frame: Rect) -> Rect;
pub const MIN_COLUMNS: u16 = 40;  pub const MIN_ROWS: u16 = 7;
// crates/graft-ui/src/confirm.rs
pub fn confirm_rows(view: &ConfirmView, width: u16) -> u16;   // grows with the wrapped message; existing signature adjusted only if it took no width — say so
```

## Acceptance

Every tick names its verifying test inline, at tick time (ADR-0035).

- [x] A masked field draws `•` per char, never the text, cursor column = char index — `text_input::tests::masked_field_draws_bullets_only` (also `masked_field_at_the_ascii_tier_draws_stars`, `debug_redacts_a_masked_fields_text`)
- [x] Credential prompt at 80×24 and 160×48 matches the note's mockup in structure (title, prompt line, field, note, footer) — `credential_prompt::tests::{masked_80x24, username_160x48}`; too-small message at 30×5 — `too_small_draws_one_line`
- [x] Nothing drawn outside `credential_area` at any size — `credential_prompt::tests::stays_inside_its_rect` (proptest over sizes 1..200 × 1..60)
- [x] Confirm consequence wraps to ≤ 4 lines and the buttons never overlap the frame's footer row at 120×40 — `confirm::tests::{long_message_wraps, buttons_stay_above_footer_at_120}` (also `message_caps_at_four_lines_with_ellipsis`)
- [x] Footer shows `f fetch  p pull  P push` at 160 and 120 columns — `graph_screen::tests::remote_hints_at_wide_and_medium`
- [x] MAP.md updated (`text_input`, `credential_prompt`, `confirm`, `graph_screen`)

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo nextest run -p graft-ui` · `cargo test --doc -p graft-ui`. Snapshot regenerations named in NOTES.

## Size

One context; four small pieces in one crate. Scout `text_input.rs`, `confirm.rs`, `graph_screen/footer.rs` first (one haiku run, ≤ 1 page) and say so.

## NOTES

Continuation session (rate-limit handoff): the previous builder had left the
code uncommitted with no ticks. This session verified the existing work
(did not rewrite it), ran the gates, added the capture, ticked acceptance,
and committed.

- Scout: not re-run this session (the ticket's own note says the original
  builder scouted `text_input.rs`/`confirm.rs`/`graph_screen/footer.rs`
  before writing; this session read the diff directly instead — `git diff
  --stat` plus targeted reads — since the code was already written and the
  job was to verify it, not design it).
- Footer hints at 120/200 cols: `f fetch  p pull  P push` sit right after
  `u undo`; at 200 cols this pushes `Ctrl+R refresh` off the right edge
  (existing drop-from-the-right rule); at 80 cols the new hints bump
  `j/k move  Space fold` off — recorded in the new capture file. Placement
  is right after `u undo`, before `/ search` (not literally after `S
  stash`/before `J journal` as the ticket's scope line 23 first sketched) —
  MAP.md's `graph_screen` line explains why: it holds the hints inside the
  fixed-position part of the hint list, ahead of the dynamic `extra` splice,
  so they don't get pushed around by screen-specific extra hints. This is a
  placement deviation from the literal scope text, not from the acceptance
  criterion (which only pins the widths, not the exact neighbours) — flagging
  it since CLAUDE.md says say, don't silently deviate.
- Added `credential_prompt::tests::capture_for_docs_shots` (`#[ignore]`d,
  same convention as the M3 stash capture) and generated
  `docs/shots/m4-credential-prompt-and-hints.txt` from it plus the literal
  footer-hints snapshot files — judge by eye per that file's header.
- Colour unverified (TestBackend records no colour attributes, per CLAUDE.md
  rule 5) — masking is proven as character content, not colour.
- Out of scope, confirmed still out: key handling/state for
  `CredentialPrompt` (owned by the app layer, ticket G5b).
