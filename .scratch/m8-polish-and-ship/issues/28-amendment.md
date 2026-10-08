# 28 amendment — contact sheet path and order, two new journey ids, what 28 now waits for

Status: draft (planner spec-only pass 2026-10-08, from 13l NOTES "m8 ruling" and the conductor's dispatch). Amends
`28-accept-m8.md` only where named; everything else in 28 stands.
Build with: unchanged — **Fable** (the acceptance ticket; doesn't count against the Fable build expectation).
Needs: unchanged, plus the names below ("Needs, replaced").

## Read first (additions to 28's list)
- This file, whole.
- `docs/design-notes/m8-polish-and-ship.md`: Journeys rows J33–J37, "Look polish rows judged", the two new fence ids,
  the four post-hoc "Where he lands" rows, the "Not walked" paragraph.
- `13l-stop-and-look-review-fixes.md` NOTES: "m8 ruling (captures in two places)" and the `M8/hangup-exemption-too-wide`
  row.
- `18-rulings.md` "Split" table; `20-words-sweep.md` NOTES "Open"; `05-trace-followup-per-turn-cost.md` Status line.

## A1 — the contact sheet (replaces 28's "Contact sheet" box and Scope item 6's "assembles the contact sheet")
- Built with `scripts/contact-sheet.sh --captions-order m8 docs/shots` (13l NOTES: it checks `"$shots_dir/$name"`
  at `contact-sheet.sh:154` and orders by the captions file at :102-106, so one run gathers both sets).
- 28 writes `docs/shots/m8-contact-sheet-captions.tsv`: one row per capture, in ticket order, each with its "what
  right looks like" line. Flat captures named `m8-<file>`; subdirectory captures named `m8/<file>`. Prototype scenes
  (`m8-proto-*`) first, as the mock-up the look clause compares against.
- The sheet lands at **`docs/shots/m8-contact-sheet.txt`** (the script's output name), not
  `docs/shots/m8/contact-sheet.txt`. If it passes the 200 KB cap, split it the way M5's was (`m5-contact-sheet-1.txt`,
  `-2.txt`); how the script splits is unconfirmed here — read `scripts/contact-sheet.sh` first and say in NOTES.
- First line of the sheet (each part, if split): **colour unverified** (rule 5).
- 28's own new captures go flat, `docs/shots/m8-*.txt`. Nothing under `docs/shots/m8/` moves (11e is re-shooting
  `m8/doorway-*`); 17's tick naming the `m8/` path stands.
- Coverage test (as M6's `contact_sheet_captions_cover_every_m6_capture`): reads **both** `docs/shots/m8-*.txt` and
  `docs/shots/m8/*.txt`, excluding the sheet itself; fails naming any capture missing from the captions file, and any
  captions row whose file doesn't exist.
- The open sheet halves of 17b's and 17c's capture boxes (and 17's) close on this sheet: NOTES lists them for the
  conductor, with the coverage test's name. 28 does not edit those tickets.

## A2 — journey ids (replaces Scope item 1's "each of the 21 ids")
- `WALKS` covers **every id in the note's fence** — 31 at this writing (29 + `fold-merge-count`,
  `whitespace-mode-shown`, added post hoc 2026-10-08). The test reads the fence; no count is hard-coded.
- `fold-merge-count` (J33, 1 key): `Space` on a merge whose side holds 3 commits → the row ends ` +` then ` +3`. Reuse
  `tests/it/look_polish.rs::capture_folded_merge_count_for_docs_shots` (17b: it pumps `Space` through the real job) if
  it can be driven through `within_the_note`; else write the walk beside it.
- `whitespace-mode-shown` (J34, 1 key): detail pane on a commit with a whitespace-only change, `w` → title ends
  `· whitespace ignored`; blame, `w` → `· whitespace counted`. Check whether
  `look_polish.rs::capture_whitespace_ignored_title_for_docs_shots` presses `w` or sets the state; if it sets it, write
  the walk.
- J19's allowance (note "Pass 2c"): if `reword_head_with_a_line_built_on_it` still asserts "3 + the preview's `y`",
  make it assert the fence's 4 through `within_the_note`. Say in NOTES which you found.

## A3 — TESTING.md "M8" additions (to Scope item 7)
One row each, concretely:
- J33: "fold a merge with `Space`: its row ends `+` for a moment, then `+N`, N the commits the merge brought in; press
  `Ctrl+R`: the number stays, it is not re-counted."
- J34: "in the detail pane press `w`: the title ends `· whitespace ignored`; press `w` again: the words go. In blame
  the title says nothing until `w`, then `· whitespace counted`."
- J35 (by eye, colour — rule 5): "cursor on a commit in your work repo: lanes off its ancestry are paler than the
  ones on it, in your theme, in Ghostty and Zed; at 16 colours they are dim, not gone."
- J36 (person step, 2+1): "add `overview = true` under `[ui]`, restart: the overview strip is there at launch; `O`
  still hides it for this run."
- J37 (by eye, Pixel terminal): "at the Pixel tier open a rewrite preview: the graph turns to text lines with the
  ghost rows dashed/hollow; header `gfx` unchanged; close it: the pixel lanes come back at once, no flicker of
  re-drawing." (17c: bytes asserted in memory only; a real Kitty/iTerm2 terminal not verified.)

## A4 — Blocked by, replaced
28 starts when **every** M8 ticket's Status reads done, or its open boxes are KNOWN_GAPS rows the conductor has
filed. By name, the ones open on 2026-10-08:
- **11e** with `11e-rulings.md` and `11e-rulings-2.md` (footers; re-shoots `docs/shots/m8/doorway-*`).
- **13l** Parts A and B (Part C is this pass). Plus its NOTES row routed "before 28":
  `M8/hangup-exemption-too-wide` (`startup.rs:531`, `:482`).
- **18a, 18b, 18c, 18d** — 18 is superseded by `18-rulings.md` (18a done 2026-10-08). The clone and first-launch walks
  (`clone`, `first-launch`) come from 18c and 18d.
- **20's remainder**: item 13 (doc comment — the conductor rules whether a doc-only fix needs a test), item 14 (the
  `u`, `J` route test, `apply/error/words.rs:18-31`), the askpass addition (`M8/askpass-refusal-wording`).
- **05 last** (deferred by the user 2026-10-08: "lets finish the product, and then deal with this"): runs after
  everything else above is built, and **before 28**, because 28's perf lines (J1–J3 tails, `G`) must measure the loop
  05 leaves. If 05 closes with no code (his numbers within budget), 28 starts on its NOTES. 28 then runs, then the
  usability walk, then the stop.
- Unchanged: 02–27 as listed in 28.

## Needs, replaced
In 28's Needs list, `first-run-clone-and-first-launch` → `clone-op` (18a), `clone-job-host` (18b),
`clone-prompt-and-walk` (18c), `first-launch-line` (18d). Add: `doorway-footers-and-widths` (11e),
`stop-and-look-review-fixes` (13l), `trace-followup-per-turn-cost` (05, already listed — now last), `look-polish-rows`'s
follow-ups `fold-count-and-pixel-ghosts` (17b) and `pixel-ghosts-draw-at-unicode` (17c).

## Acceptance (replaces 28's contact-sheet box; adds the rest)
- [ ] Sheet `docs/shots/m8-contact-sheet.txt` (or its `-N` parts): every capture under `docs/shots/m8-*.txt` and
      `docs/shots/m8/*.txt` in ticket order under its "what right looks like" line; first line says colour
      unverified; coverage test named at tick fails on a missing capture and on a stale captions row (seen red once:
      a captions row removed).
- [ ] `every_m8_journey_id_has_a_walk` green over the fence's 31 ids, including `fold-merge-count` and
      `whitespace-mode-shown` (tests named at tick).
- [ ] TESTING.md "M8" holds the J33–J37 rows above, word for word or tighter.
- [ ] NOTES: 05's outcome quoted before the perf run (file:line); the J19 allowance found/changed; the boxes this
      sheet closes on 17/17b/17c, for the conductor.

## Touches (additions)
- `scripts/contact-sheet.sh` (read; edit only if `--captions-order` can't take `m8/<file>` — then stop and report, it
  is 13l's finding that it can); `docs/shots/m8-contact-sheet-captions.tsv` (new); `tests/it/look_polish.rs` (walks
  reuse it); TESTING.md; the note's fence (read only).

## Gates and Size
Gates unchanged. Size: 28 was ~75k; this adds ~10k (captions file, two walks, five rows). If the captions file for
~60+ captures pushes past one context, split it off as its own Sonnet ticket and say so — don't compact mid-sheet.
