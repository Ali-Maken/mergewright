# M8 acceptance — smooth on his biggest repo, looks like the mock-up, installs, teaches itself

Status: draft
Build with: **Fable** — the acceptance ticket (CLAUDE.md); judges whether the promise holds across every group, and
doesn't count against the Fable build expectation.
Needs: every other M8 ticket: `01-trace-product-frame-trace`, `pace-fence-and-sync`, `settle-work-at-rest`,
`pixel-memory-budget`, `look-chrome-header-gutter-bar`, `look-cursor-row-and-dark-b`, `look-outside-repo-screen`,
`trace-followup-per-turn-cost`, `keymap-table-and-translate`, `keymap-spell-from-the-table`,
`palette-help-and-actions`, `palette-find-anything`, `bar-content-pass`, `theme-overrides`, `look-polish-rows`,
`first-run-clone-and-first-launch`, `tour-practice-repo`, `daily-landing`, `daily-ops-fixes`, `words-sweep`,
`hygiene-code`, `hygiene-scripts-and-fixtures`, `docs-readme-and-man`, `release-dist-and-matrix`,
`fable-architecture-pass` and its G-health tickets, `rename-on-disk-migration`, `rename-names-sweep`.

**Promise (M8):** *"It stays smooth on your biggest repo, looks like the mock-up, installs with one command, and
teaches itself from inside the app."*

## Read first
- `docs/design-notes/m8-polish-and-ship.md`: Promise, Journeys (table, "Where he lands", the `journey-counts` fence)
- `docs/adr/0037-m4-checkpoint-decisions.md` — the contact-sheet decision only (CLAUDE.md: "A milestone's captures
  ship as one contact sheet")
- `crates/graft/MAP.md` lib.rs entries `tests/it/m7_walk.rs` and `tests/it/m5_walk/` M6 journeys (`parse_counts`,
  `within_the_note`, `inputs_of`) — the M8 walk reuses them
- Each M8 ticket's Acceptance section (to find the test each journey already has)

## Scope
In:
1. **Journey counts.** `tests/it/m8_walk.rs`: `noted_keys(id)` reads the note's `journey-counts` fence through
   `m5_walk::parse_counts`; `WALKS` maps each of the 21 ids to its (file, test); `every_m8_journey_id_has_a_walk`
   fails on an id with no walk, a stale entry, or a file lacking the id literal or the test fn; every walk test runs
   through `within_the_note` (one key over → fails naming both numbers). Write the walks the tickets didn't.
2. **Scroll (release, scripts → files).** `held_key` Unicode and Pixel (Kitty+deflate and without), a held-wheel burst
   (add it to `held_key` if G-pace didn't), `g_after_idle`; RSS after held `j` at Pixel (G-pixel-mem's budget).
   Summary to `docs/perf/m8-accept.txt`: tail after release ≤ 100 ms (J1, J2), `G` ≤ 16 ms (J3), RSS ≤ budget.
3. **Look.** Re-shoot the look captures at 80×24, 120×36, 187×50 beside the mock-up's (`docs/shots/m8-proto-*`):
   graph, cursor row on a branch label, empty state, `?` over the graph, the tour card. Regenerate the two capture
   headers stale since M7 (`docs/shots/m6/fix-undo-drift-way-back-120x40.txt:13`,
   `docs/shots/m6/fix-box-owns-the-bar-120x40.txt:9-10`).
4. **Teaches itself.** The full tour walked headless to `practice done ✓` with the binary named `trib` and no shell
   step; PROMPT §9's seven tasks each reachable from the bar or the palette in the place they happen (test per task:
   its action is on the bar or in `?` there).
5. **Installs.** From `docs/release/*.txt`: plan lists 4 targets and the man page; host archive built; formula styled;
   `cargo install --locked --path crates/graft` works. The real `brew install` on a clean account is his step (not
   publishable yet, ADR-0057).
6. **One command:** `scripts/m8-accept.sh` runs 1, 3, 4 (fast tier + capture writers) and 2 (release, heavy filters),
   writes every capture and perf file, assembles the contact sheet, prints only paths and pass/fail lines.
7. **TESTING.md "M8"**: one row per scenario saying concretely what right looks like (never "looks correct"), e.g.
   "hold `j` 3 s on your work repo in Ghostty full screen, let go: the list stops within a blink — no rows move after
   your finger is up"; person-step rows: rebind (J8, 5+1), fix one colour (J10, 2+1), `brew install` + `trib` on a
   clean account (J16, 2), `man trib` (J17, 1); the "Times I left" table gets an M8 row to fill in daily use.

Out: the whole-app usability walk (reviewer, after this); his eye on colour (rule 5, KNOWN_GAPS stays unverified).

## Context guard
Captures and perf output go to files; the builder reads pass/fail and summary lines, never a capture body or a raw
trace. Report: paths only, plus the one-line verdict per promise clause.

## Acceptance
- [ ] `every_m8_journey_id_has_a_walk` and each walk green; an M8 `one_key_over_the_note_fails` (tests named).
- [ ] `docs/perf/m8-accept.txt`: J1/J2 tails, J3, RSS lines quoted with their budgets, all within.
- [ ] Contact sheet `docs/shots/m8/contact-sheet.txt`: every M8 capture in ticket order under its "what right looks
      like" line; a test fails when a capture under `docs/shots/m8*` is missing from it (as M6's
      `contact_sheet_captions_cover_every_m6_capture`).
- [ ] Tour walked to done with `trib` (test named); seven §9 tasks reachable (tests named).
- [ ] Install facts quoted from `docs/release/`.
- [ ] TESTING.md "M8" section written: one row per journey id and per person step.
- [ ] Every capture ≤ 200 KB; no machine path (existing `nothing_a_commit_would_carry_names_this_machine` green).
- [ ] **Colour unverified** stated on the sheet's first line (rule 5).

## Touches
- `tests/it/` walk modules, `scripts/`, `docs/shots/m8/`, `docs/perf/`, TESTING.md; nothing in product code unless a
  walk finds a defect — then stop and report it (rule 2), never patch around it here.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide;
plus `scripts/m8-accept.sh` exit 0.

## Size
One context, ~75k tokens.
