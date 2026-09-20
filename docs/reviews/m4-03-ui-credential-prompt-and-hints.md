# Review: M4 group 03 — ui-credential-prompt-and-hints

Ticket: `.scratch/m4-remotes/issues/03-ui-credential-prompt-and-hints.md`
Range: `git diff 0ba9203^1 0aa5839`, graft-ui only (the range also carries
group 04's ops work; not reviewed here). Reviewed 2026-09-20, late — this
group's round was missed at the time.

Read: CLAUDE.md, the ticket and both NOTES, `docs/design-notes/m4-remotes.md`
"New UI" and hard choice 2, mockup §1.14, `crates/graft-ui/MAP.md` (the
`text_input`/`credential_prompt`/`confirm`/`graph_screen` lines), M3 review 06
Minor 11, the graft-ui diff per module, the committed capture. Opened whole:
`draw_line` in `text_input.rs`, `Confirm::render`, `key_hints` in
`graph_screen/footer.rs`, `fit.rs` header — each to confirm a finding.

## Gates (graft-ui, `CARGO_TARGET_DIR=target-review03`, deleted after)

| Check | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy -p graft-ui --all-targets -- -D warnings` | ok |
| `cargo nextest run -p graft-ui` | 251 passed, 1 skipped (the `#[ignore]` capture printer), **1.31 s** |
| `cargo test --doc -p graft-ui` | ok (0 doctests) |

Slowest 5 (all under a third of a second): `syntax::tests::properties::highlight_never_panics_or_overlaps` 0.32 s, `lanes_without_hue crowded_shapes_at_every_depth_below_truecolor` 0.19 s, `syntax::tests::spans_never_overlap_or_run_past_the_line` 0.18 s, `ascii_tier crowded_shapes_draw_only_the_ascii_glyph_table` 0.17 s, `graph_column every_shape_draws_as_snapshotted_and_the_same_in_dark_and_light` 0.15 s.
Peak RSS of the nextest process 91 MB. `target-review03` reached 1.2 GB (removed); the shared `target/` is 6.2 GB.

**Not run:** workspace clippy with `--features heavy-tests`, the heavy tier, the release perf run, fuzzing. Reason: the conductor scoped this round to graft-ui; the diff touches no heavy code and no frame-time path (widgets and a footer string). Those numbers are in the M4 acceptance review, not here.

## Tick spot-check

6 ticks. All 6 checked. All 6 name a test (or MAP.md) and every named test exists and passes — 12/12 tests found in the run, MAP lines 10/14/24 present. Two ticks hold in letter but not in substance:

- Tick 4 (`buttons_stay_above_footer_at_120`): the assertion cannot fail at 120×40 — see Major 4.
- Tick 5 ("surviving from 156 columns"): true but not first; the three hints already fit at 144 — see Major 3.

Neither is a false tick (the named test asserts what the tick says and passes), so no Critical.

## The masked field — can it show or print the secret?

Verdict: **no path found inside graft-ui.** Paths traced:

- `FieldWidget::render` → `display_text(view, ascii)` → `draw_line(&shown, …)`; the guide is dropped and `counter_column` returns `None` while masked. `text_input.rs:487-560`.
- `CredentialPrompt::draw_field_row` → `display_text(&field, false)` → `draw_line`. `credential_prompt.rs:126-146`.
- `draw_line` only ever sees the masked string; its scroll and `fit` operate on that; indexing is clamped (`index.min(chars.len())`), so no panic text can carry the secret. `text_input.rs:693-738`.
- `Debug for FieldView` prints `"<redacted>"` for `text` while masked; `CredentialView` derives `Clone` only, so there is no accidental `{:?}` on the whole view. No logging in graft-ui.
- Length leaks by design: the bullet count (unavoidable), and `Debug` printing `cursor: Some(n)` (Minor 3).

`FieldView.text` is `pub` and readable by any caller — necessary, the app has to send the answer; the guard is that the only draw gate is `display_text`, which holds today. Not zeroed after use: filed by the group 06 review, not re-examined.

## Critical

None.

## Major

1. **The too-small fallback never fits where it fires, and is not centred.**
   `credential_prompt.rs:41` `TOO_SMALL` is 45 cells; the width branch triggers below `MIN_COLUMNS = 40`, so in that branch the message is always `…`-cut — the committed capture shows `terminal too small for the cr…` at 30 columns. `credential_area` (`:77-85`) returns a full-width one-row rect and `render` (`:162-169`) draws at `area.x`, left-aligned; the ticket says "centred". Only when the *height* is the trigger (frame ≥ 45 wide, < 7 tall) does the sentence appear whole. `too_small_draws_one_line` checks only `contains("too small")`, so it passes on the cut text.
   Fix: a message ≤ 30 cells (`too small for the prompt`, 24), offset `x` by `(width - cells)/2`, and assert the whole message and its centring in the test.

2. **The ASCII tier cannot reach the credential prompt.** `draw_field_row` hard-codes `display_text(&self.view.field, false)` (`credential_prompt.rs:131`) and `CredentialPrompt` has no `ascii` field (`:59-64`). `FieldWidget.ascii` exists, but its only callers (commit editor, stash prompt) pass `false` and are unmasked — so the `*` path proven by `masked_field_at_the_ascii_tier_draws_stars` is one no production widget takes, and the ticket's "ASCII tier: `*`" is unmet where it matters. The box chrome (`╭ │ ▏ ─`) is Unicode at every tier too.
   Fix: `CredentialPrompt { view, theme, ascii }`, the app passing the renderer tier, and the snapshot at the ASCII tier; or drop the flag and record in KNOWN_GAPS that the ASCII tier draws `•`.

3. **The recorded threshold is wrong: 144, not 156.** By arithmetic on `HINTS` (`footer.rs:179-208`) joined by `key_hints` (`:243-261`), ` q quit … P push` is 144 cells, and the committed "156 cols (first width the remote hints survive)" capture line is 155 cells *including* `  J journal` (11). So all three remote hints first fit at **144** columns; 155 is where `J journal` joins. "First surviving at 156" is stated in MAP.md line 14, the test doc (`graph_screen/tests.rs:782-789`), the capture heading, and the ticket NOTES/tick 5. The ticket's scope asked to "record which widths keep them"; the record is off by twelve.
   Fix: correct all four places; make `remote_hints_at_wide_and_medium` pin the boundary (143 lacks `P push`, 144 has it) instead of the arbitrary 156.

4. **`buttons_stay_above_footer_at_120` cannot fail.** At 120×40 the box is 9 rows (4 capped message lines + 5) in a 40-row frame; `area.bottom() < frame.bottom()` held before the footer reservation existed. The reservation in `confirm_area` (`confirm.rs:158-165`) only acts when `confirm_rows ≥ usable_height`, and no test drives that. M3 Minor 11 is therefore fixed by code that no test exercises.
   Fix: a frame whose height equals or is below `confirm_rows` (e.g. 120×9 and 120×6) asserting `area.bottom() < frame.bottom()` and `area.height == frame.height - 1`.

5. **Stale comment describing the reverted placement, and two tests weakened for it.** `graph_screen/tests.rs:663-666` and `:679` say "the remote-write hints sit ahead of `j/k move`… 60 columns is too narrow to keep both" — that is the first, wrong placement the fix round removed. After the fix the 80/120 footers equal pre-ticket content, and ` q quit … j/k move` is 59 cells, so 60 columns holds `j/k move` again. The widening 60 → 90 and the comment are leftovers; the reveal test lost its narrow case.
   Fix: restore 60, delete the comment.

6. **No confirm-box capture and no rendered confirm test at 80/160.** Scope item 5 asked for "push confirm with a two-line consequence at 80 cols and one line at 160"; what exists is `long_message_wraps` calling the private `wrap_message` at width 20 and `message_caps_at_four_lines_with_ellipsis` at width 10. Nothing renders `Confirm` with a wrapping message, and the shots file's "confirm box" section is a prose description of an assertion, not a grid. CLAUDE.md rule 6 (a visual ticket ends with a capture) is unmet for the push box. Note also the design mockup at 80 columns wraps the *command* line; `Confirm::render` still `fit`-truncates it (`confirm.rs:207`) — pre-existing, but the mockup was this ticket's spec, and a lease push command exceeds 64 cells.
   Fix: two `TestBackend` renders at 80×24 and 160×48 through `confirm_area` + `Confirm`, snapshotted and pasted into the shots file; decide (conductor) whether the command line wraps like the message.

7. **For the conductor — the daily footer never shows the M4 keys.** With the mockup's position honoured, `f fetch  p pull  P push` need 144 columns; the user's terminals (Zed/VS Code panes, 80–140) will never show them, and the drop order between 144 and 155 shows `p pull` without `J journal` etc. The drop order itself is coherent (leaving, undo, search, movement, graph shape, stash, remotes, journal, paging) and the threshold follows from the fixed-slot list, not an accident — but nobody has decided that undiscoverable-below-144 is acceptable. The alternative tried (ahead of search/movement) broke the daily footer, so this is a spec question: a contextual `extra` slot when a remote exists, a shorter spelling (`f/p/P remote`), or a KNOWN_GAPS row.

## Minor

1. `masked_80x24` accepts 7 *or* 8 bullets (`credential_prompt/tests.rs:88-91`); "hunter2" is 7. Assert `count('•') == 7`.
2. `border_row` doc (`credential_prompt.rs:103-105`) promises "a space, label, dashes, a space, right"; the code emits neither space (capture: `8443─────`, `push────╯`; mockup: `8443 ─────`). Fix code to match the mockup, and the doc.
3. `Debug for FieldView` (`text_input.rs:439-455`) redacts `text` but prints `cursor: Some(n)`; at end-of-field `n` is the length, which the same impl's comment says a counter must not leak. Redact `cursor` while masked, or say why a length in a debug line is fine.
4. `CredentialPrompt` ignores `field.focused`; the cursor draws whenever `cursor` is `Some` (`credential_prompt.rs:132-142`). Harmless while the prompt is modal; document it or honour the `FieldView` contract.
5. "Draws nothing outside that rect at any size" (`credential_prompt.rs:57`) holds only for rects from `credential_area`: `draw_field_row` writes the 3-cell `│ ▏` at `area.x` *before* the `width <= 4` guard (`:129-130`), so a caller-supplied 1–2-wide, ≥5-tall rect overflows. Guard first, or narrow the claim.
6. Four copies of the cells-wide helper: `cells` in `text_input.rs:34`, `confirm.rs:29`, `credential_prompt.rs:98`; `text_width` in `graph_screen/layout.rs:219`. One `pub(crate)` fn in `fit.rs`.
7. Shots file "Judge by eye" claims "cursor after the last bullet"; the cursor is a reversed cell, invisible to a TestBackend grid (rule 5). Say so — the reader will look for the mockup's `_` and not find it.
8. `wrap_line` on `split(' ')` turns a run of spaces into empty words (`confirm.rs:36-52`), giving lines with trailing blanks; harmless, worth a `split_whitespace` or a comment.
9. `Ctrl+R refresh` now drops at 200 columns (`footer-hints-200.snap`); the complete list needs 227+ columns and `the_footer_lists_every_graph_key…` had to widen to 230. Design rule, but the widest-ever hint bar is now past any real terminal.
10. `confirm_area` centres on `usable_height` now, so even-height frames place the box one row higher than before; "existing snapshots for one-line messages unchanged" held because no graft-ui snapshot renders the confirm box, not because the geometry is unchanged. The NOTES' workspace-wide nextest covers the app-crate snapshots, so not a bug — the claim is by absence.

## Verdict on the captures

`docs/shots/m4-credential-prompt-and-hints.txt` (4.1 KB): a user **can** judge the credential prompt from it — box structure, title in the border, `▏` then seven bullets, the dimmed note's text, footer in the border, plaintext `alice` for the username ask. They **cannot** judge the too-small fallback as designed (what they see is the cut text), the cursor (not in the grid), colour (none recorded, stated), or the push confirm box (no grid at all).

## Did the missed round matter?

Yes. Majors 1–6 are inside graft-ui rendering and test substance; group 06's review (app wiring, the un-zeroed secret) and the acceptance walk (full-app captures at the daily widths) would not have looked at the ASCII flag, the too-small sentence, the 144/156 arithmetic, or the vacuous confirm test.
