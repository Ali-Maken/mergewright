# m1-the-graph/10 — Row text columns and width degradation

Status: done
Build with: **Sonnet** (medium) — mechanical: columns, formatting, snapshots
Blocked by: 09
Leaves runnable: yes — rows show subject, author chip, date and short SHA.

Why: the graph column alone doesn't say what happened. This is also where repo text crosses into `ui`, so the sanitiser boundary is enforced for real.

## Read first
- PROMPT.md §1.3 (the column list only)
- DESIGN.md §1.1, §1.2, §1.3 (which columns each width keeps)
- ARCHITECTURE.md: the rule that repo text reaches `ui` only as `term::SanitizedText` (grep `SanitizedText`)
- Code: public API and existing tests of `crates/graft-term/src/sanitize/`, 09's graph screen

## Scope
In:
- Columns: graph · marker slot · subject · author initials chip (colour per author, `.mailmap` honoured) · relative date · short SHA.
- The marker slot is reserved and empty (M2 fills it) so M2 doesn't reflow the layout.
- Lower-priority columns drop at DESIGN.md's widths, in its order.
- All repo text reaches `ui` as `SanitizedText`.

Out:
- Ref badges (11), the uncommitted row and 16-colour chrome (12).

## Acceptance
- [x] Snapshots at 80×24, 120×40, 200×60, dark and light, showing which columns survive each width. `crates/graft-ui/tests/graph_screen.rs` (insta, feature-branches corpus, colour-blind); pty capture of the real binary at all six combinations: `docs/shots/10-row-columns.txt`.
- [x] A subject and an author name containing an escape sequence render inert — extend M0's sanitiser test, no new mechanism. `crates/graft-term/src/sanitize/tests.rs::an_author_name_carrying_an_escape_sequence_comes_out_inert` (subject case already covered by the existing `ansi_colour_injection_in_a_commit_message_comes_out_inert`).
- [x] `.mailmap` maps two identities to one chip and one colour (fixture test). `crates/graft-git/src/mailmap/tests.rs::two_identities_resolve_to_one_canonical_author`.
- [x] The marker slot's width is the same empty or filled (test). `crates/graft-ui/src/graph_screen/tests.rs::the_marker_slot_is_the_same_width_empty_or_filled`.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only) — all green, whole workspace.

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes
This session picked up mid-ticket after a forced compaction of an earlier session working the same ticket (summary handed off, no code lost — the summary's "pending tasks" were completed here): mailmap plumbing, `GraphEntry`/`AuthorChip`/`Columns` layout, tests, view.rs wiring, sanitiser test, MAP.md updates and pty captures were all done in this continuation. `.mailmap` resolution goes through gix's own `mailmap` feature (`gix_mailmap`), not a hand-rolled parser, per ADR-0011. Author-chip colour reuses `Theme::branch`/`BranchIdentity` rather than a new palette. Created `crates/graft-git/MAP.md` (previously missing) while touching that crate.
