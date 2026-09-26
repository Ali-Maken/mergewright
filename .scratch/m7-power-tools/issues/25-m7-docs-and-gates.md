# m7-docs-and-gates — fix the stale docs and captions M7 inherited; walk-gate the two push journeys

Status: ready
Blocked by: none
Blocked by: none
Crates: graft (tests only: `tests/` walks), docs (`TESTING.md`, `docs/shots/*.tsv`, `docs/design-notes/m6-conflicts.md`,
`crates/*/MAP.md`, `.scratch/m6-conflicts/issues/*`). No production source, except comments named below.
Source: `docs/design-notes/m7-power-tools.md` "KNOWN_GAPS rows naming M7" (Stale docs, captions → G-accept; Test gates
→ G-accept).
Build with: **Sonnet**. It is text fixes, each named with its file, plus two walks in an existing harness.
Scout: none. Every item names its file.
Needs: none.

## The items (from KNOWN_GAPS; builders don't read it)
Before changing each one:
- open the file and confirm the stale text is still there;
- if it isn't, write `already fixed: <item>` in NOTES and move on;
- find each by its quoted text, not only its line (lines drift).

Where an item says "match the capture", the capture is the truth; change the words, never the capture.

| # | Where | What's stale | Fix |
|---|---|---|---|
| 1 | `docs/shots/m6-contact-sheet-captions.tsv:19` | the m5-hints-ladder caption's widths | name the widths at which hints raise as the capture shows (80/120/160) |
| 2 | `crates/graft-ops/src/journal/file.rs:202-203` (comment) and graft-ops MAP `read_state_now` | describe an older behaviour | describe what the function does now, read from its body |
| 3 | M6 ticket 04, `:94` | says what ticket 05 superseded | append `superseded by 05` with the line of 05 that did it |
| 4 | M6 ticket 08, `:3` (Status) | still reads open, but the seam closed in `1aadc4d` | status: done, citing `1aadc4d` |
| 5 | `TESTING.md:2245`, `docs/shots/m6-contact-sheet-captions.tsv:1,3,9`, `docs/design-notes/m6-conflicts.md:152` | the old resolver title `file 12 of 13 · 2 of 3 conflicts open` | the title as the current m6 resolver captures show it |
| 6 | `.scratch/m6-conflicts/issues/13-resolver-file-list.md:125` | says three panes at 200 columns | what shipped, as its capture shows |
| 7 | `docs/shots/m5-contact-sheet-captions.tsv:10-12` | disagree with `TESTING.md:2019` | match the capture; then make TESTING.md and the TSV agree |
| 8 | graft MAP: `land_in_resolver`, `operated`, `resumed` | entries describe pre-M6 behaviour | describe what each does now, read from its body |
| 9 | M6 ticket 17, a NOTES bullet the review flagged as stale | the builder finds it by the review's quote in `docs/reviews/m6-*.md` | correct it, or mark it `superseded:` with the reason |
| 10 | the `fix-file-keys-before-120x40` row of the M6 captions TSV | the caption | "`O/T file` full, `j/k` and `⏎` bare at 120" |
| 11 | `docs/design-notes/m6-conflicts.md:373-374` | the two lines contradict each other | keep the one the shipped code matches (cite its line); strike the other with `(superseded)` |

The M6 tickets live in `.scratch/m6-conflicts/issues/`; find each by its number prefix.

**Gates:**

| # | What | Fix |
|---|---|---|
| 12 | The push journeys `P y` (push) and `p r r P y` (pull-rebase, then push) are in the M6 note's counts but no walk gates them | add them to the walk harness that parses the notes' `journey-counts` fences, using the M6 note's ids and counts; each asserts `len(keys) <= noted_keys(id)` and the landing the M6 note gives |
| 13 | pty captures are compared only when someone runs the walks | not here: `m7-acceptance`'s script runs every m3–m7 walk in compare mode. Leave a NOTES line pointing there |

## Read first
- The note's "KNOWN_GAPS rows naming M7" table (Stale docs; Test gates).
- `docs/design-notes/m6-conflicts.md`: its journey-counts fence (the push ids) and the lines named in items 5 and 11.
- The walk harness's MAP entry (`crates/graft/MAP.md`: `m5_walk`/`journeys`, `parse_counts`, `noted_keys`) before any
  test source.

## Acceptance (name the verifying test or the diff line at tick time)
- [ ] Items 1–11: each ticked with the file:line changed, or `already fixed` with the line that shows it.
- [ ] `push_journey_is_walk_gated` and `pull_rebase_push_journey_is_walk_gated`: green; red first by lowering the noted
      count by 1 in a scratch copy (say so in NOTES; don't commit the scratch).
- [ ] No capture file changed (`git diff --stat docs/shots/*.txt` is empty).

## Touches (check each, report how)
- Captions TSVs: the M5/M6 contact sheets are assembled from them. Check: the sheet-assembly script (if one exists)
  still runs.
- MAP.md: text only. Check: `git diff` shows only the named entries.
- `m6-conflicts.md`: edits only at items 5 and 11. Check: `git diff --stat`.
- The walk harness: two new walks. Check: `cargo nextest run` workspace-wide.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md` (the items are restated
above). Change words, never behaviour.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~35k.
