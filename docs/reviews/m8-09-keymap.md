# Review — m8-polish-and-ship keymap group: 09 (table + translate) and 09b (edge, config, status row)

Reviewer, one round, at main `4c9a3eda` (no crate change since `a327c1d5`). Diffs: `0b1f1099^1..0b1f1099` (09) and
`a327c1d5^1..a327c1d5` (09b). I built in a detached worktree with its own target. Both are removed. Ticket 10 was not
reviewed. No release perf run, on the conductor's instruction.

**Counts: 0 Critical · 5 Major · 12 Minor.** Rules 5/6: I judge no colour and no "looks right" here.

## What I ran and what it showed

| Check | Result | How |
|---|---|---|
| `cargo fmt --check` | ok | review worktree |
| `cargo clippy --all-targets -- -D warnings` | ok (33.5 s, peak RSS 1.07 GB) | same |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok | same |
| `cargo nextest run` (workspace) | **5497 passed, 0 failed**, 10 skipped. 613.9 s in nextest, 671.8 s wall; peak RSS 1.26 GB | load average 37–39 (a builder was running alongside) |
| `cargo test --doc` (workspace) | ok (3 doctests) | same |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | **red**: the graft `it` binary had 743 passed and 8 failed. cargo stopped there, so the later binaries **never ran**. 1035 s wall, peak RSS 1.48 GB | Failures: 4 × `held_key` (`frames_after_release=3`, budget ≤2), 2 × `many_refs` budget (debug build: 3.8 s and 2.8 s against 1.5 s), 2 × capture drift (`m7/fix-scroll-ghostty.png`, last re-shot in M7 before 07's cursor-row change; `m7/pull-over-edits` reads `140 rows so far` where the shot has `202 commits`, an async count). **I did not attribute any of them to 09/09b.** I ran them only under load 37. Pace, timing and stale-shot causes fit, but that is not shown. They need a quiet re-run. |
| Release perf run | **not run** | conductor's instruction |
| Reviewer probes | 5 throwaway tests in the review worktree, reverted | cited per finding |
| `target/` | review target 9.4 GB (removed). Shared `target/` 54 GB | `du -sh` |

5 slowest fast-suite tests, under load: `graft-ops undo_cancel::cancel_at_every_step_boundary_is_clean` 33.4 s,
`graft-ui hints::the_bar_never_draws_a_fragment_of_itself` 28.9 s, `…the_bare_keys_are_a_tail_not_a_scatter` 24.2 s,
`…a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` 23.9 s, `…no_hint_loses_ground_as_the_room_for_it_grows`
22.2 s. None of them is a keymap test.

## Pipeline

- The design note has Need, Prior art and Journeys (`docs/design-notes/m8-polish-and-ship.md:19,32,83`). The study
  exists: `docs/research/keymap-help-remap.md`. Remapping is a take (`:57`).
- J8 "Rebind an action" is 5+1 through `?` (`:93`). `?` is ticket 11, so the journey cannot be walked yet.
  09b's J8 tests cover the config half only, and the tick says the person step stays with accept-m8. That holds.

## Tick spot-check: 14 checked, 14 held, 0 false

- **09 (8 ticks): 6 checked, 6 held.**
  - Golden list, id shape, round-trip, warnings and both sweeps: each named test exists and ran green.
    `checked > 200` is asserted.
  - `every_bound_key_has_a_row`: 3 exemptions, each in NOTES.
  - graft-ui MAP.md `## keymap` exists.
  - 145 ids in the table equal the 145 lines in `keymap_ids.txt`.
- **09b (9 ticks): 8 checked, 8 held as written.**
  - Sweep (`checked > 300` asserted at `keymap_edge.rs:278`).
  - Exemption counts: 5 `NOT_YET_ANSWERED` and 18 `NO_EFFECT_IN_FIXTURE`, each with a reason. Counted.
  - Typed text: 10 fields (`every_field`), plus the outside prompt.
  - J8 unit test and pty test; `keys_suspend_on_ctrl_z_reaches_app_state`.
  - Shots: the diff adds only the one new file, 4.7 KB.
  - MAP entries.
  - Tick 1 holds only because its own text concedes the EmptyState part is partial: see Major 4.

## Critical

None.

## Major

### 1. A graph verb moved onto a Global key takes that key from the changes pane, the diff and the `.` menu; the warning names only the graph

- **What happens** (probe, `[keys.graph] merge = "j"`):
  - The only warning is `j given to graph.merge — move down keeps ↓ on the graph`.
  - `translate` gives `Default(m)` for `j` in Changes, Diff, Map, Journal, Stash and Menu.
  - Through `update`, `j` in the Changes and Diff fixtures no longer moves. It does what `m` does there.
  - In the Menu fixture, `j` opens the merge preview instead of moving the menu's cursor.
  - Map, Journal and Stash could not be told apart: their fixtures don't move on `j` at all (Major 3).
- **Why.** `scope()` puts Graph ahead of Global for every nested place (`crates/graft-ui/src/keymap.rs:145-150`), and
  `resolve` takes the first context in scope order (`keymap/resolve.rs:156-158`). In the code, though, those places
  answer the Global movement keys themselves, before the graph does:
  - `context_menu_key` claims `j`/`k` at `crates/graft/src/update/keys.rs:216`;
  - `pane_and_overlay_key` claims them at `:274`;
  - both run before `graph_key` at `:281`.
- **Why the warning is too narrow.** `collisions()` only compares a context's *own* rows with Global's
  (`resolve.rs:197-207`). A Graph row shadowing a Global row inside Changes, Diff or Menu is never seen.
- **Why it matters.** One line in his config silently breaks movement in up to six places. The one problem he is shown
  says "on the graph". The `.` menu's whole vocabulary is movement plus confirm (§2.1, 02-keymap.md:114-121), and he
  loses it.
- **Fix (needs a spec ruling).** Two options:
  - Rank the Global rows an inner place answers itself (movement, `⏎`, `esc`) above Graph in those scopes.
  - Or run the collision check over each context's effective resolution across its whole scope, and name every place
    that loses the key.
- **Missing test.** One line per nested context: `app.down` still moves there after `graph.merge = "j"`.

### 2. `suspend_on_ctrl_z = true` is silently undone by moving `app.undo` off `⌃Z`

- **What happens.** Probe with `suspend_on_ctrl_z = true` and `[keys.app] undo = "u"`:
  - default keymap: `⌃Z` emits `Command::Suspend`;
  - with the remap: no suspend;
  - warnings: none.
- **Why.** Suspend is not a row. The edge sees `⌃Z` only as `app.undo`'s default, so moving undo makes `⌃Z`
  `Unbound` (`resolve.rs:103-109`, `table.rs:77`). The press never reaches the suspend check at
  `crates/graft/src/update/keys.rs:230`.
- **Why it matters.** This is exactly the config a job-control user writes: turn suspend on, take `⌃Z` off undo.
  Suspend stops working with no word.
- **Fix.** Make suspend a row (e.g. `app.suspend`, live only when the switch is on). Or have the edge pass `⌃Z` when
  `suspend_on_ctrl_z` is set. Add a test for that pair of settings.

### 3. The behavioural sweep is green on "nothing equals nothing" for over half its cases, and never sweeps a row reached through scope

- **Half the cases prove nothing.** Probe over 09b's own sweep: **210 of 406** non-text cases have a default press that
  changes nothing in the fixture (`seen(base, d) == seen(base, F20)`). So "the free chord does what the default
  does" passes there because both do nothing. Examples:
  - every `app.*` row in Map: `j`, `k`, `⏎`, `q`, …;
  - Changes and Diff `app.up`, `app.top`, `app.page_up` (the cursor is already at the top);
  - Map, Journal and Stash `j`: "default j acts? false" for all three overlays.
  - Some of these are correct swallows (Preview, Confirm, Banner). The overlays and pane movement are not.
- **Scope-reached rows are never swept.** The sweep checks only a context's own rows and Global's
  (`crates/graft/src/update/tests/keymap_edge.rs:174-178`). Never checked:
  - Diff rows in Blame and History (`w`, `v`, `L`, `B`);
  - Graph rows in Changes, Diff, Menu and the overlays.

  The one property 09 says must never fail, that `scope()` never under-includes (`keymap.rs:137-140`), therefore has
  no behavioural test. Major 1 slipped through for the same reason.
- **Mirrored helpers.** Both sweeps work out the expected default with a copy of `resolve` over defaults:
  `acting_default` (`keymap_edge.rs:160-171`) and `default_holder` (`keymap/tests/translate.rs:11-17`). A wrong scope
  order is agreed with, not caught.
- **Fix.**
  - Give the overlays rows to move over and put the pane cursors mid-list.
  - Sweep every row in `ctx.scope()`, not just own rows and Global.
  - Assert a floor on non-trivial cases rather than `checked > 300`.

### 4. Partial or unverified work ticked with no KNOWN_GAPS row (rule 4)

- **The EmptyState sweep.** 09b tick 1 (`09b-keymap-edge-and-config.md:101-103`) is `[x]` but says the EmptyState is
  "one remap and the default identity, not a full sweep". NOTES (`:189-190`) leave it as "Gap for the conductor if
  wanted". `grep` of `KNOWN_GAPS.md` finds no row.
- **Two more unverified items, also with no row:**
  - F2 sanitising of config problems (NOTES `:170-171`);
  - `diff.word_diff` / `blame.heatmap` drawing at all (NOTES `:186-188`).
- **Fix.** Either untick 09b tick 1, or open a gap row marked **broken (test missing)** for the outside sweep, plus
  **unverified** rows for the other two.
- **Minor 2 settles one of them.** My probe shows the F2 sanitising holds.

### 5. Question (hard to reverse): "one confirm key" is five action ids

- §2.3b makes `y` the one confirm key across previews, boxes, the finish list and the rewrite editor
  (02-keymap.md:282; ADR-0044 §3).
- The table makes it five independent ids:
  - `preview.confirm` (`table.rs:199`);
  - `confirm.yes` (`:220`);
  - `finish.commit` (`:347`);
  - `rewrite.start` (`:383`);
  - `empty_state.confirm` (`:489`).
- A user who moves one gets `y` confirming in four places and his key in the fifth. That is "same key, same meaning"
  broken by his own config, with no warning.
- Ids are frozen at release (ADR-0055 §2), so this is the cheap moment to decide.
- **Option.** One `app.confirm` row (`y`). Graph's `graph.copy_sha` `y` already shadows it on the graph through
  scope. `preview.confirm` keeps only `m`/`r`.
- Filed as a question: the 09 spec's rules ("nested place → one row") don't cover sibling modals.

## Minor

1. **The doc comment says the opposite of the code.** `crates/graft/src/config/keys.rs:20-23` says the line "whose id
   sorts later wins". The code keeps the **earlier** line (`resolve.rs:181-185`). Probe:
   `graph.fetch = "z"` with `graph.merge = "z"` → `z given to graph.merge — on the graph z stays fetch`.

   File order is also not honoured, against Scope 5's "earlier line keeps it". This is documented, but it is his file.
   Consider enabling `toml`'s `preserve_order` (`crates/graft/Cargo.toml:55`).
2. **F2 sanitising has no test.** The old assertion was removed (`session.rs:459`); the code is at
   `view/status.rs:191`. My probe holds: a TOML key `"\u001b[31mmerge"` gave a problem drawn as
   `graph.␛[31mmerge is not an action` on F2. Commit that probe as a test.
3. **The pixel prefetch's what-if pages go through the edge.** At `crates/graft/src/event_loop/running.rs:179` they
   are fed to `update` as raw `PageDown`/`PageUp`.
   - With `app.page_down` moved, the what-if is `Unbound`: nothing is prefetched for his real page key.
   - With another action on `PgDn`, that action runs in a throwaway clone.

   This file is not in 09b's Touches. Use `dispatch_key` (default chord), or translate his chord.
4. **09's decision that text fields pass every key: keep it, but say so.**
   - It is right for typed characters and editing keys (ADR-0055 Consequences).
   - The cost: a moved Global key (`app.back = "ctrl+g"`) is silently ignored in the picker, palette, commit editor,
     credential prompt and every state field. Probe: Picker `esc` Pass, `ctrl+g` Pass, warnings `[]`. Esc still works
     there, so it is safe.
   - Suggest one line in `?` / the man page ("text fields keep esc ↑↓ ⏎"). Or, later, translate only non-character
     Global chords in list-style fields (picker, palette).
5. **The `⌃⇧C` gap row understates the harm** (gap M8/ctrl-shift-c-legacy; 09 NOTES 3).
   - In a legacy terminal, `⌃⇧C` arrives as `⌃C` (`graft-term/src/input/decode/mod.rs:170`).
   - `⌃C` quits at once (`update/commit_editor/keys.rs:17`).
   - `commit_draft` lives only in memory, so **the typed commit message is lost**.
   - The table now spells `⌃⇧C` (`table.rs:453`), and `?` and the man page will advertise it.
   - Re-rate the row and give it an owner.
6. **Doc drift.**
   - §2.9 stash `b` still says "planned, not yet built" (02-keymap.md:471; gap M8/keymap-doc-stash-b open).
   - 09 NOTES' other doc questions have no tracking: 12 code-only rows (`PgUp`/`PgDn`, `n`/`N`, `J`, `F2`/`F3`, `B`/`X`,
     preview `←→⇥`, …) and doc-only rows (`h`/`l`, `⌃⇧Z`, `i`).
   - §2.1 "Every binding is remappable" (02-keymap.md:174) is false for the Fixed text-field rows (`⌃S`, `⌃E`, `⌃⏎`
     in the commit editor).
7. **Extra pub items.** `KeyChord::key`, `plain`, `ctrl`, `ctrl_code`, `alt` and `QUIT_NOW`
   (`keymap/chord.rs:40-89`) are pub. The ticket said "No other `pub` items … without the conductor", and NOTES don't
   mention them.
8. **Readability (PROMPT 6.2).**
   - `keymap/table.rs` is 514 lines and `update/tests/keymap_edge.rs` 549, both over 400.
   - `collisions()` nests 4 deep (`resolve.rs:193-222`).
9. **Question, by reading, not run.** One Global override can count as several problems: `[keys.app] undo = "o"` looks
   to yield three (graph `take`, resolver `take ours`, start screen `open`) through the per-context loop at
   `resolve.rs:193-207`. So `config: 3 problems` would be shown for one line. Is that wanted?
10. **The `app.quit_now = "none"` words read wrong.** `resolve.rs:81` falls back to `QUIT_NOW`, so the warning reads
    `⌃C always quits — app.quit_now can't move to ⌃C`. Say "can't be unbound" instead.
11. **The unbound path skips the blame/history `keep_in_view` fall-through** (`update/edge.rs:92-100`). It is documented
    in NOTES with its cause, and the fixtures are pre-settled to hide it. Accept, but it ties the edge to a dispatcher
    detail. Add a test that a not-yet-scrolled place still settles on its next answered key.
12. **The empty state's confirm box under a remap is untested.** `outside_edge` (`empty_state.rs:72-91`) has no test for
    `y`/`n`/`esc` with the init confirm open under a remap. This is part of Major 4.

## Asked about by the conductor

- **Every row answered per context.** Real for own rows: actual presses, 23 exemptions, each with a reason, and the
  test fails on an exempt row that answers. Weak for the sweep (Major 3).
- **Same key, same meaning.** Broken by config in Majors 1, 2 and 5. In the shipped table it holds.
- **Text fields get every key.** Judged in Minor 4: keep, add words.
- **Outside-repo keys go through translate.** Yes: `event_loop/places.rs:289` → `outside_edge`. Every other `Event::Key`
  entry goes through `update` → `edge::key` (`update/mod.rs:1150`), the prefetch included (Minor 3).
- **Config remaps and `[keys.*]` warnings.** Correct by test and pty. The order quirk is Minor 1.
- **EmptyState sweep partial.** No tick claims a full sweep, but the box is ticked (Major 4).
- **F2 sanitising.** It holds by probe; it has no test (Minor 2).
- **`⌃⇧C` legacy.** Minor 5.
- **Doc drift.** Minor 6.
- **Errors swallowed.** No `unwrap`/`expect`/`let _ =` outside tests in the new code.
- **Git on the UI thread.** None.
