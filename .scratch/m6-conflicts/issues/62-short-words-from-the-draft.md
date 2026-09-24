# short-words-from-the-draft: the hint bar's short words — batch 4

Status: blocked (conductor question; a product preference, so possibly his)
From: user picks 2026-09-24 item 6 ("use the 2026-09-21 draft"); `.scratch/m6-conflicts/user-batch.md` item 4;
KNOWN_GAPS :523.
Build with: *(blank until unblocked)*
Needs: `hint-words-keep-the-way-out` merged (same ladder).
Crates: graft-ui only.

## The question (one line, for the conductor)
His pick "use the 2026-09-21 draft" collides with his own later ruling: m5/27 cut that draft to **prefixes only** (nine
rows kept, thirteen squeezes and substitutions deleted — `cmt`, `stg`, `tool`, `find`, `stop`, …), and the build
follows it (`short_form`, graft-ui MAP line 22; test `every_short_form_is_a_prefix_of_its_verb`). The batch item's own
example (`space stage` → `space stg`) is one of the rows he deleted, and KNOWN_GAPS :523 still says "unbuilt", which is
stale: the ladder and the nine rows shipped in m5/24 + m5/27.

Options:
- **(a) As built — recommended.** "The draft" = the draft as he amended it on m5/27. Nothing to build; the conductor
  closes KNOWN_GAPS :523 naming `every_short_form_is_a_prefix_of_its_verb` and
  `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`. Why: his m5/27 reason ("a substitution gives one verb two
  names") still holds, and his batch-9 pick (A over B, which offered `tool`) points the same way.
- **(b) Restore the whole 22-row draft.** Reverses m5/27 and ADR-0039's prefix rule; needs his explicit word, since it
  was his ruling.

## If (b): scope
- `short_form` gets the thirteen deleted rows back, verbatim from
  `.scratch/m5-history-and-ghost-preview/issues/24-the-ladder-and-the-stale-banner.md:24-36`.
- `every_short_form_is_a_prefix_of_its_verb` is deleted with a comment naming his ruling of 2026-09-24.
- Touches: every capture that shows a short rung (`grep -rln "cont\b\|disc\b" docs/shots` as a start; the builder's
  full list in NOTES); `every_key_the_bar_draws_is_answered` (verbs unchanged); keymap §2.1 "one meaning per verb"
  (the conductor decides whether a note is needed).
- Acceptance: `short_form_matches_the_draft` (22 rows); capture `docs/shots/m6/fix-short-words-80x24.txt`, the changes
  pane's bar at 80 columns, **what right looks like:** the bar's short words are exactly the draft's.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
(a) none. (b) Small.

## NOTES
