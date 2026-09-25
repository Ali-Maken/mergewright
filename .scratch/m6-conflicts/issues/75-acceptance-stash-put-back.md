# acceptance-stash-put-back: walk picks 15–17, count every journey against the note, captures, TESTING.md

Status: done
Blocked by: 67, 71, 72, 72b, 73, 74 (and 68–70 for the bottom row and tab words)
From: the acceptance update for picks 15–17 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24-26). Promises: M6
"a real conflict is resolvable without leaving the app" (now a stash put back by hand or by Graft); M3 "you cannot lose
work" (edits set aside are in the tree or the stash list at every instant).
Build with: **Fable** — the acceptance ticket (judges the pass against the note; does not count against the Fable build
expectation).
Needs: 67, 71, 72, 72b, 73, 74, all merged; **and** the conductor's doc rows (design note "Where I'd differ"): the ten
journey rows in `docs/design-notes/m6-conflicts.md`'s `journey-counts` fence, keymap §2.1/§2.6/§2.9, the `DirtyTree`
doc line, KNOWN_GAPS rows (staged-after-clash, fast-forward pull over overlapping edits, row 50, 71's cherry-pick
question).
Crates: graft (tests only), scripts, docs (TESTING.md, captions TSV). A behaviour defect found here is filed with its
line (rule 7), not fixed.
Scout: none; ticket 43's NOTES and 63's NOTES describe the harness and the sheet script.

## Read first
- `docs/design-notes/m6-stash-put-back.md` "Journeys", "Where each event lands him", "New UI".
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §5 (the survival table), §6.
- Ticket 63 "Scope" and NOTES (the harness: `m5_walk::every_scenario`, `journey_counts_match_the_note`, the pty walk,
  `scripts/contact-sheet.sh`).
- The amendment blocks of 71, 72, 73, 74 (what changed after their scouts), and NOTES of every ticket in Needs
  (captures and TESTING.md rows each listed for this ticket).
- TESTING.md "Times I left" (:87-117) and "Conflict journeys", "Captures" (:2284-2386, lines as they stand).

## Scope
**In:**
1. **Every journey as a scenario** in `m5_walk::every_scenario::journey`, keyed by the note's ids:
   `stash-pop-clash-to-done` 4, `stash-apply-clash-to-done` 4, `stash-clash-put-back` 2, `pull-rebase-dirty` 3,
   `rejected-push-dirty-to-pushed` 5, `switch-dirty` 3, `switch-dirty-clash` 5, `pull-rebase-stop-then-clash` 8,
   `menu-checkout` 3, `menu-cherry-pick` 4. Each asserts git's truth at the end (stash list, `git status`, branch,
   `git log origin/main` where it pushes), not the screen alone. The group walks in `tests/it/m6_stash_put_back.rs` stay;
   these are the counted ones.
2. **The survival check, end to end:** `m6_stash_put_back::edits_survive_a_kill_at_every_step` — the app driven through
   `. ⏎ y` on a dirty tree, killed (the harness's stop) before each of ADR §4's steps 1–6, relaunched: the edits are in
   the tree or in `git stash list`, and the relaunch line names `stash@{n}` when they are in the list.
3. **One command:** `cargo nextest run -E 'test(m5_walk::every_scenario::journey) | test(journey_counts_match_the_note) |
   test(m6_stash_put_back::)'` green, every journey at or under its row. The route table to
   `.scratch/m6-conflicts/acceptance-stash-put-back-routes.txt`; read its summary line only (context guard).
4. **Captures** through the pty walk (`.scratch/m6-conflicts/pty-walk-m6.py`), 120x40, 187x50 and 80x24:
   `m6-20-stash-clash-landing` (`S p`), `m6-21-stash-clash-done` (`S p o ⏎`), `m6-22-set-aside-preview` (`p r` on a
   dirty tree), `m6-23-set-aside-clash-landing` (`. ⏎ y` into a clash), `m6-24-set-aside-done` (`p r r`),
   `m6-25-menu-fixed-slots` (`.` on a branch row). Matrix and captions TSV updated
   (`captures_are_named_keyed_and_listed_in_the_matrix`).
5. **Contact sheet:** `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`, parts ≤ 200 000 B, this pass's
   captures in spec order (67's `fix-rebase-refused-before-preview` is gone: 74 deleted it).
6. **TESTING.md**, every line concrete:
   - "Conflict journeys": one row per journey above — start, keys, count, landing words, git truth. Landing words are the
     note's, e.g. `stash-pop-clash-to-done`: resolver on `notes.md`, notice `stash@{0} clashed in 1 file · esc, u puts
     it all back`; after `o ⏎`, bottom row `every file done · stash@{0} dropped ✓ · u brings it back`, `git stash list`
     empty.
   - "Captures": what right looks like for m6-20 … m6-25 (the words each spec's capture line gives).
   - "Times I left": the stash-pop-clash row (N3, 11 keys) → closed, naming `stash-pop-clash-to-done`; the dirty-pull row
     (N4, 16 keys) → closed, naming `rejected-push-dirty-to-pushed`.
   - The colour paragraph: the dim `Checkout — already on <b>` slot and the resolver roles are asserted in buffer tests
     (named); a capture carries no colour; **unverified** until he looks.
   - Every row the Needs tickets' NOTES listed (`every file done` for a pop, `later milestone`, menu positions,
     `preview failed:` for a refusal) rewritten.
7. **Perf:** the release perf run from its script to `docs/perf/acceptance-stash-put-back.txt`; summary lines only. The
   preview gains a row and `check`: the preview frame and the menu open stay inside budget (ADR-0021).

**Out:** any behaviour fix; M7's checkout/cherry-pick keys and picker; the whole-app usability walk (the reviewer's,
after this).

## Acceptance (each tick names its test, at tick time)
- [x] `m5_walk::every_scenario::journey` (the ten new scenarios among all) and `journey_counts_match_the_note` green;
      route table path; none over.
      **Tests:** `m5_walk::every_scenario::journey_{stash_pop_clash_landing, stash_pop_clash_to_done,
      stash_apply_clash_to_done, stash_clash_put_back, pull_rebase_dirty_preview, pull_rebase_dirty,
      rejected_push_dirty_to_pushed, switch_dirty, switch_dirty_clash, pull_rebase_stop_then_clash, menu_checkout,
      menu_cherry_pick}` (`crates/graft/tests/it/m5_walk/stash_journeys.rs`) and
      `m5_walk::conflict_journeys::tests::journey_counts_match_the_note`. The one command: 56/56 passed. Routes:
      `.scratch/m6-conflicts/acceptance-stash-put-back-routes.txt` — every one of the ten equal to its note row
      (4, 4, 2, 3, 5, 3, 5, 8, 3, 4); none over. The fence gained `menu-checkout 3` and `menu-cherry-pick 4`.
- [x] `m6_stash_put_back::edits_survive_a_kill_at_every_step` green; every step 1–6 covered (listed).
      **Test:** `m6_stash_put_back::kill::edits_survive_a_kill_at_every_step`. Steps 1 stash create, 2 begin,
      3 stash store, 4 reset --hard, 5 the checkout, 6 the put-back — each killed, relaunched, asserted. Seen:
      1–2 edit in tree, no line; 3–4 tree and `stash@{0}`; 5–6 `stash@{0}` only; the relaunch line for 3–6
      `an operation was interrupted: checkout fix · edits in stash@{0} · J to inspect`. How the kill is made: NOTES.
- [x] Captures m6-20 … m6-25 at three sizes, each ≤ 200 KB, listed in the matrix.
      **Test:** `m5_walk::every_scenario::captures_are_named_keyed_and_listed_in_the_matrix` (stems in
      `coverage::M6_STEMS`, matrix regenerated). 18 files under `docs/shots/m6/`, largest 9 771 B
      (`m6-22-set-aside-preview-187x50.txt`); a second pty pass without `GRAFT_UPDATE_SHOTS` reported every one
      `settled` at all three sizes.
- [x] Contact sheet part paths and sizes, each ≤ 200 000 B.
      **Check:** `bash scripts/contact-sheet.sh --captions-order m6 docs/shots/m6` → 175 captures, 0 uncaptioned:
      `m6-contact-sheet-1.txt` 179 772 B, `-2` 177 281 B, `-3` 176 526 B, `-4` 164 416 B. m6-20…25 last, in spec
      order (sheet 4). (A script, not a test.)
- [x] TESTING.md rows as above; the build's counts equal the note's (the parser holds them).
      **Test:** `journey_counts_match_the_note` holds the counts; the rows are doc (no test): "Conflict journeys" →
      "The stash put-back journeys" (ten rows + kill paragraph), "Captures" rows m6-20…25 + the Rust captures
      paragraph (82's stop-line row), the colour paragraph, "Times I left" N3/N4 rows (closed), row 12 and the
      history "Proves" line rewritten.
- [x] Perf summary lines, inside budget, with the file path.
      **Test:** `m6_stash_put_back::frame_budget::the_menu_and_the_set_aside_preview_keep_input_to_frame_under_16_ms`
      (new, heavy tier) + `heavy::budgets`, release, via `.scratch/m6-conflicts/perf-m6-75.sh` →
      `docs/perf/acceptance-stash-put-back.txt`: `stash_frames: 4 frame sets, max 195 us, 0 at/over the 16000 us
      budget`; `first_paint_us: 5 runs, max 40816 us, 0 over the 150000 us budget`; `peak_rss: max 97 MiB`; 6/6 passed.
- [x] NOTES: defects found, each with its line or as a question (rule 7); what was not verified (colour, rule 5).
      (Doc, no test.) Below.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide. Context guard: captures, route tables and perf output go to files; the report names paths only.

## Size
Medium (~60k): ten scenarios on the existing harness, one kill walk, six captures at three sizes, TESTING.md.

## NOTES

**Compacted once** mid-ticket (at the captures step). Nothing remains: everything after it was done from the summary
and re-read files. Scouts: **yes, two** (walk harness; kill/pty/perf), before the compaction.

### Deviations (say so, not silently)
- **Kill test, how:** the app has no stop hook a test can reach (`jobs::Cancel` is `pub(super)`,
  `crates/graft/src/jobs/types.rs:40-58`; no env var or feature — scout). So `. ⏎ y` runs through `update`, `y`'s
  `Command::Operate` is taken before any job runs it, and the test runs that operation the way
  `jobs/operate.rs::in_context` does, with a `Stop` that **panics** (unwinds, nothing closed, nothing put back).
  The kill lands at the first `Stop` poll after the previous step's mark, which is inside a running git: that git
  runs on (as after a real `kill -9`) and the test waits for this process's children (`pgrep -P`) before relaunching
  — a bounded poll on a condition, in the test only. Consequence: "before step 3" came with `stash store` done, so
  the "`S` objects only, not listed" state is not reached at app level; graft-ops'
  `a_stop_before_each_step_loses_nothing` covers it.
- **Journeys walked twice:** the eight stash ids are walked by `m6_stash_put_back` (72b/74) and now by m5_walk too;
  `walked` is a set, so no failure. The menu tests `menu_checkout_is_three_keys` / `menu_cherry_pick_is_four_keys`
  still hard-code `<= 3` / `<= 4` (`crates/graft/tests/it/m6_stash_put_back.rs:148`, `:192`) though the
  fence now has their rows — could read `noted_keys`. Not changed (out of scope).
- **Times I left N3/N4:** the rows did not exist; added as closed `review N3/N4` rows, marked so in the table's note.
- **m6-20…25 at three sizes** (the ticket's), not five: the matrix shows `—` at 160x48 and 200x50.
- **Dim/roles colour:** no buffer test asserted the dim slot's `DIM`; added
  `m6_stash_put_back::the_checkout_slot_on_head_s_row_is_drawn_dim_and_only_it` (red when the `Cherry-pick` half is
  inverted — checked, restored). Roles are words only (`graft-ui/src/conflict_resolver/draw/block.rs:35`); no buffer
  test runs the band under `StashPop`/`SetAside` roles.

### Defects and questions (rule 7)
- **Question (with line):** `j` from the uncommitted row lands on `HEAD`'s row, not the next row drawn —
  `crates/graft/src/update/selection.rs:46-52` (`Down(n)` → `Down(n-1)` from HEAD). With `fix` checked out and `main`
  newer, the uncommitted row is drawn at the top, `main` second, `fix` third, and the cursor opens on the uncommitted
  row: `j` skips `main` to land on `fix` (m6-23's second take, keys `j . ⏎ y`: `already on fix`); reaching `main` took
  `j k`. Intended (the row belongs to HEAD) or a spatial jump? Not a claim of wrong logic — the comment at :49 says
  it is deliberate for the case where HEAD is the next row.
- **Question, carried from 74's NOTES:** the stop line's ` · edits aside` is drawn at wide terminals only — is 120
  wide enough for it? Not re-measured here.
- **Question, carried from 82:** relaunch at 80 draws one of two lines (`stash@{0} clash from last run · …` or
  `stash@{0} clashed · 1 conflicted · …`); which one was not recorded. Not walked here.
- TESTING.md row 12 (`stash-pop-conflict-resolves-without-a-banner`) said `all conflicts resolved`; its scenario
  asserts `every file done` (`crates/graft/tests/it/m5_walk/resolved_operations.rs` "resolved, with nothing to
  continue"). Textual; rewritten. The history "Proves" line said a dirty tree refuses a rebase; rewritten.

### Not verified
- Colour and dim (rule 5): every capture is a character grid. The dim slot is proven in the buffer only.
- By eye (rule 6): m6-20…25 at three sizes — the user judges them on sheet 4.
- The kill test's state "`S` objects only, not listed" at app level (see above).
- Perf: an in-memory sink; the terminal's own parse and paint are not in the number.

### Touches (how checked)
- m5_walk list and `walk!` order: `every_scenario_has_a_test` green. Fence: `journey_counts_match_the_note` green
  (both directions). `coverage::M6_STEMS` + matrix: `captures_are_named_keyed_and_listed_in_the_matrix` green.
- Captions TSV / sheet: script reports 0 uncaptioned; `contact_sheet_captions_cover_every_m6_capture` green in the full run.
- Machine paths: `nothing_a_commit_would_carry_names_this_machine` green (new files are un-ignored, so read).
- No product code touched (`git diff --stat` shows `crates/graft/tests`, docs, `.scratch` only).
- Gates, workspace-wide after the last edit: fmt ok; clippy `--all-targets -D warnings` ok (and `-p graft
  --features heavy-tests`); nextest 3462/3462 passed (1 skipped); `cargo test --doc` ok.
