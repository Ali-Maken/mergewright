# M8 acceptance — smooth on his biggest repo, looks like the mock-up, installs, teaches itself

Status: **done except the perf box (7/8; perf is the user's terminal run — dispatch: not in a helper's context).** Builder 2026-10-11; amendment applied (`28-amendment.md`). Was: ready
Build with: **Fable** — the acceptance ticket (CLAUDE.md); judges whether the promise holds across every group, and
doesn't count against the Fable build expectation.
Blocked by: 02, 03, 04, 05, 06, 07, 08, 09, 10, 10b, 10e, 10f, 10g, 10h, 10i, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
21, 22, 23, 24, 25, 26, 27
Needs: every other M8 ticket: `01-trace-product-frame-trace`, `pace-fence-and-sync`, `settle-work-at-rest`,
`pixel-memory-budget`, `look-chrome-header-gutter-bar`, `look-cursor-row-and-dark-b`, `look-outside-repo-screen`,
`trace-followup-per-turn-cost`, `keymap-table-and-translate`, `keymap-spell-from-the-table`, `keymap-spell-in-words`,
`keymap-words-ops`, `keymap-words-stops`, `keymap-words-resolver`, `keymap-words-undo-journal`,
`keymap-words-preview-remote`, `palette-help-and-actions`, `palette-find-anything`, `bar-content-pass`,
`theme-overrides`, `look-polish-rows`, `first-run-clone-and-first-launch`, `tour-practice-repo`, `daily-landing`,
`daily-ops-fixes`, `words-sweep`, `hygiene-code`, `hygiene-scripts-and-fixtures`, `docs-readme-and-man`,
`release-dist-and-matrix`, `fable-architecture-pass` and its G-health tickets, `rename-on-disk-migration`,
`rename-names-sweep`.

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
- [x] `every_m8_journey_id_has_a_walk` and each walk green; an M8 `one_key_over_the_note_fails` (tests named).
      Tests: `m8_walk::every_m8_journey_id_has_a_walk`, `m8_walk::a_missing_or_stale_walk_is_named`,
      `m8_walk::one_key_over_the_note_fails`; each walk named per id in `WALKS` (`tests/it/m8_walk/mod.rs`) and in
      TESTING.md "M8 acceptance"; all green in `scripts/m8-accept.sh` step 1 (`held_key` is heavy-tier: compiled, not run).
- [ ] `docs/perf/m8-accept.txt`: J1/J2 tails, J3, RSS lines quoted with their budgets, all within. **Open:** not run
      here (dispatch); `scripts/m8-accept.sh --perf` from his terminal. No held-wheel heavy test exists, so no J2 tail
      line can appear (NOTES).
- [x] _Superseded by amendment A1's box (ticked there)._ Contact sheet `docs/shots/m8/contact-sheet.txt`: every M8 capture in ticket order under its "what right looks
      like" line; a test fails when a capture under `docs/shots/m8*` is missing from it (as M6's
      `contact_sheet_captions_cover_every_m6_capture`).
- [x] Tour walked to done with `trib` (test named); seven §9 tasks reachable (tests named).
      `tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` (`CARGO_BIN_EXE_trib`, `pty.rs:229`);
      `m8_walk::teaches::{stage_a_hunk_is_found_in_the_changes_pane, commit_is_on_the_bar,
      stash_and_bring_back_are_found, create_a_branch_is_found, merge_is_found_on_a_branch_row,
      resolve_is_on_the_stop_banner_bar, push_is_found}`.
- [x] Install facts quoted from `docs/release/`. `scripts/m8-accept.sh` step 4 → `docs/perf/m8-accept.txt`: 4 archives,
      `graft.1` in the host archive, `cargo install: ok in 165s` — and `brew style graft.rb: FAILED`. Quoted, not
      passing: the files predate the trib rename (NOTES).
- [x] TESTING.md "M8" section written: one row per journey id and per person step. "M8 acceptance": 31 journey
      rows (every fence id found by a grep of the section, 0 missing), seven §9 rows, J35–J37 + J8/J10/J16/J17 person
      rows; "Times I left" M8 row.
- [x] Every capture ≤ 200 KB; no machine path (existing `nothing_a_commit_would_carry_names_this_machine` green).
      `find docs/shots -size +200000c`: none; `published_repo::nothing_a_commit_would_carry_names_this_machine`
      green (it caught the raw accept log once — `.scratch/m8-polish-and-ship/accept/.gitignore` added, as M7's).
- [x] **Colour unverified** stated on the sheet's first line (rule 5).
      `m8_walk::contact_sheet::every_part_of_the_m8_sheet_opens_with_colour_unverified`.

## Touches
- `tests/it/` walk modules, `scripts/`, `docs/shots/m8/`, `docs/perf/`, TESTING.md; nothing in product code unless a
  walk finds a defect — then stop and report it (rule 2), never patch around it here.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide;
plus `scripts/m8-accept.sh` exit 0.

## Size
One context, ~75k tokens.

## NOTES (builder, 2026-10-11)

**Compacted once mid-ticket.** Everything before the compaction (walks, captures, sheet, script) was re-run green after
it; what remained then (TESTING.md section, gates, these NOTES, commit) is done. Nothing left over from the compaction.

Scout used: `scout` (harness API and the existing test per journey) before any walk was written.

### Gates (2026-10-11, workspace-wide)
`cargo fmt --check` ok; `cargo clippy --all-targets -- -D warnings` ok; `scripts/m8-accept.sh` rc=0 — step 1
`cargo nextest run --workspace`: 6349 run, 6349 passed, 15 skipped; `cargo test --doc` ok; `scripts/check-scripts.sh`
ok. Summary lines: `docs/perf/m8-accept.txt` (its header says `+uncommitted`: run before this commit, same tree).

### What 28 found and changed
- **No product code changed.** Tests, captures, scripts and docs only, so no screen behaves differently: the usability
  walk (`docs/reviews/m8-usability.md`) stands as run.
- **J19 allowance (A2):** found `reword_head_with_a_line_built_on_it` asserting `route.len() <= allowed + 1` ("the
  note allows 3 plus the preview's y"), with its own `m8_count`. Changed: it counts through
  `m8_walk::within_the_note("reword-head", &route)` against the fence's 4 (`crates/graft/tests/it/m7_walk/amend_into.rs:365`).
- **One counter.** `outside_repo.rs`, `palette_find.rs`, `quit_waits/stuck.rs`, `m8_walk/clone.rs` each read the fence
  through their own copy of `noted_keys`; all now go through `m8_walk::within_the_note_taking`.
- **Walks written that the tickets didn't:** `m8_walk::journeys::{wheel_flick, find_key_by_name, what_does_key,
  run_unrecalled, light_theme, fold_merge_count, whitespace_mode_shown}`. J33 and J34 press the real keys (`Space`,
  `w`), not set state. `wheel_flick` is headless: the harness pumps no job between notches, so the check is "no
  non-blank row changes after the last notch" — rows still blank at the last notch are excluded
  (`m8_walk/journeys.rs`, comment at the check). Its release tail is unmeasured: **no held-wheel heavy test exists**
  (grep `held_wheel` in `crates/`: none), so `--perf` cannot report a J2 tail.
- **Stale headers regenerated** (Scope 3): `docs/shots/m6/fix-undo-drift-way-back-*` and `fix-box-owns-the-bar-*`,
  with their tests' header lines (`undo_over_drift.rs`, `box_owns_the_bar.rs`). The M6 sheets
  `docs/shots/m6/m6-contact-sheet-{1..5}.txt` were rebuilt from them, and were **already stale before this run**
  (`MA`→`GT`, `Graft`→`Tributary` diffs from the rename).
- **Look re-shoots (Scope 3), new:** `m8-look-cursor-row-{80x24,187x50}`, `m8-look-help-graph-187x50`,
  `m8-look-outside-repo-187x50`, `m8-tour-187x50` (tests `m8_walk::look::*`, `outside_repo::outside_repo_captures`,
  `tour_walk::the_tour_captures_match`). 120×36 sizes already existed.
- **`scripts/contact-sheet.sh` edited** (A1 said edit only if `m8/<file>` failed — it didn't): the header's first line
  is now `# <milestone> contact sheet — colour unverified: character grids, no colour (rule 5)`, so A1's first-line
  rule holds on every part; plus an `m8)` promise case. Every milestone's sheet gets that first line on its next run;
  m3/m4/m5/m7 sheets on disk still carry the old first line.
- **How the script splits** (A1 asked): parts at 180 000 B (`max_bytes`, :46), between captures never inside one,
  named `<stem>-N.txt`; a run deletes the earlier sheet files first (:227). M8: 3 parts, 98 captures, 0 NOT RECORDED.
- **Captures keep the current name** (`trib`/Tributary): 27d (shots rename) is parked until the name is final.

### For the conductor
- **05's outcome, quoted before any perf run** (`05-trace-followup-per-turn-cost.md:3`): "Amendment 1 build B partial
  2026-10-11 — built; after-measurement draw_us p50/p99 missed in run 1 of 2 (NOTES B)." 05 is not closed; its numbers
  stay an open unverified gap. **The perf box is not run** (dispatch: not in a helper's context):
  `scripts/m8-accept.sh --perf` from his terminal fills `docs/perf/m8-accept.txt`'s step 2.
- **Sheet boxes this sheet closes** (28 did not edit those tickets), each by
  `m8_walk::contact_sheet::contact_sheet_captions_cover_every_m8_capture`:
  17 box 2 (`m8/polish-fold-120x36.txt`, captions row 78; the box names the flat path the capture never took),
  17b :113 (`m8/polish-fold-120x36.txt`, `m8/polish-blame-whitespace-counted-120x36.txt`, rows 78–79),
  17c :89 (`m8/polish-ghost-pixel-120x36.txt`, row 80).
- **Install facts are stale on names.** `docs/release/{plan,build,cargo-install}.txt` are dated 2026-10-09, before the
  27a rename (binary `graft`, archives `graft-*`); the install check needs the network with an empty `CARGO_HOME`, so
  not re-run here. `docs/release/build.txt` says `brew style graft.rb: FAILED` (3 correctable offenses) — the "formula
  styled" box cannot tick.
- **Question for the usability pass:** `?` `hunk` finds nothing on a hunk (the action reads `Stage / unstage`); `?`
  `stage` finds it (`m8_walk::teaches::stage_a_hunk_is_found_in_the_changes_pane`). Whether "hunk" should match is a
  wording call.
- `crates/graft/MAP.md` not edited: it lists no test modules (only :4's one line for `tests/it`), and no public item
  changed.

### Touches
- Walk files: each touched test re-run green (`scripts/m8-accept.sh` step 1, the whole fast tier). `held_key.rs` is
  heavy-only: checked by `cargo check --tests -p graft --features heavy-tests`, not run.
- Captures under `docs/shots/m6/` and `docs/shots/m8-*`: compared in step 1 with `TRIB_UPDATE_SHOTS` unset.
- `scripts/contact-sheet.sh`, `scripts/m8-accept.sh`: `scripts/check-scripts.sh` ok (shellcheck).
- TESTING.md: the M8 section and a "Times I left" M8 row; no other section edited.
- `crates/graft/tests/it/over_400.txt`: the size ratchet's rows moved with the edits (`file_size::*` red in the first
  run, green after): three test files grew (`held_key.rs` 754→757, `tour_walk.rs` 903→927, `undo_over_drift.rs`
  637→638), three shrank, `m8_walk/clone.rs` fell under 400 and left the list.
- `.scratch/m8-polish-and-ship/accept/.gitignore` (new, as M7's): the raw nextest log carries home paths;
  `nothing_a_commit_would_carry_names_this_machine` failed on it in the first run.
- Product code, keymap, journal/undo, other crates: none (`git diff --stat -- crates/*/src` empty).
