# right-click-acts-on-the-wrong-row: the menu names one commit and the verb runs on another

Status: done
Blocked by: 17 (same crate — `graft`)
Build with: **Opus.** A verb running on a row the user did not pick, and one
path of it silently, is the failure class this project ranks above all others.
Crates: graft (say so if `graft-ui` is forced)
Scout: **not needed.** `docs/reviews/m5-12-13-context-menu-overlay.md` (`96edfd7`)
reproduced it through real `update` and names the path. Read it, not the tree.

## F1 — Critical (escalated by the conductor from the review's Major)

**Right-click opens the context menu for the clicked row without moving the
cursor.** Group 13's fall-through then runs the verb on the **cursor** row.

Reproduced by the reviewer through real `update`: right-click `main`, press
`m` → `already on it`, no preview. The menu named one commit; the verb acted
on another.

**Why I escalated it.** Two different failures hide here and the second is the
bad one:

- `m`, `r`, `x` are loud — they go through the preview box, so an attentive
  user sees a plan for a commit they did not pick, and M5's confirm step is
  the backstop. Bad, recoverable.
- **`y` copies the wrong sha with no feedback at all.** Nothing is drawn,
  nothing is confirmed, and the user pastes that sha into a shell command
  minutes later. A silent wrong answer that the user then acts on elsewhere is
  worse than a refused operation or a visibly wrong preview, and this project
  has no other defect of that shape open.

**Decide the cause, do not patch the symptom.** Two honest fixes exist and they
are not equivalent — say which you chose and why:

1. **Right-click moves the cursor**, then opens the menu. The menu and every
   verb then agree by construction, and the rule "a verb acts on the cursor" is
   preserved everywhere.
2. **The menu carries its target row**, and every verb reads the menu's target
   rather than the cursor while the menu is open.

(1) is smaller and matches how the rest of the app already works. (2) is more
faithful to what a right-click means elsewhere in the world but introduces a
second notion of "the current row", which is exactly the kind of duplicate
state this milestone keeps being bitten by. **I recommend (1)**; take (2) only
if you find something that forbids (1), and then say what.

Whichever you take, the test must press through real `update` from a
right-click, not from a constructed state — the review found this by doing
that, and a state-level test is what missed it.

## F2 — the review's remaining Majors and Minors

In `docs/reviews/m5-12-13-context-menu-overlay.md`. Work them, except the ones
already assigned:

- **Already in ticket 17, do not touch:** the `Fix: revert the merge (-m 1)`
  label, its MAP.md claim, and the tests pinning it.
- **Already filed, do not re-report:** `.` closing the menu (ticket 15); the
  §2.1/§2.6 `M` contradiction; the confirm box below 2 usable rows; the
  confirm and credential boxes being centred rather than band-cleared, which
  is an open question for the user by eye.

**`GRAFT_UPDATE_SHOTS=1` masks regressions** (the review's Minor m1): a normal
run must fail on drift and only an explicit regeneration may rewrite a
capture. Check it cannot be set by accident in a test run, and that a
regenerated capture is visible in `git status` rather than silently identical.

## Acceptance (every tick names its verifying test inline, at tick time)

Every test below is in `crates/graft/tests/it/context_menu.rs` unless said
otherwise, and every one of them presses a real `Event::Mouse` through the
real `update` (`app_harness::App::right_click`).

- [x] Right-click a row that is not the cursor, then press each of the menu's
      verbs: the verb acts on the row the menu names — driven through real
      `update` from the click — test:
      `m_after_a_right_click_merges_the_branch_the_menu_named` (the merge's
      `source_tip`), `r_after_a_right_click_rebases_onto_the_branch_the_menu_named`
      (the rebase's `onto_tip`), `x_after_a_right_click_marks_the_commit_the_menu_named`
      (the compare base), `y_after_a_right_click_copies_the_sha_the_menu_named`.
      Each is measured against `git rev-parse main`, not against the menu.
      `f`/`p`/`P` are not covered here: pressing them starts a real network
      job, which is not a fast-tier test. They go down the same
      `context_menu_key` fall-through as the four above and act on the
      cursor, which the first tick's fix moves — **unverified**, not broken.
- [x] `y` after a right-click copies the sha of the row the menu names — this
      is the silent one, so it needs its own tick — test:
      `y_after_a_right_click_copies_the_sha_the_menu_named`. It asserts the
      `Command::CopyToClipboard` itself (through
      `app_harness::press_for_commands`, because the job runner refuses that
      command) and the `Status::Copied` short sha the footer shows.
- [x] The menu's title and its target cannot disagree — a test that goes red
      if they drift apart, not one that checks them against each other —
      test: `a_right_click_moves_the_cursor_to_the_row_it_opens_the_menu_on`
      pins the invariant the fix rests on (menu row == cursor row), and the
      four verb tests above check the labels (`Merge main into feature (m)`
      and so on) and the verb's target **separately, each against git**. The
      menu has no title field; its labels are what names the row.
- [x] A normal test run fails on capture drift; `GRAFT_UPDATE_SHOTS=1` is the
      only thing that rewrites one — test:
      `only_an_explicit_regeneration_rewrites_a_capture`. Only the literal
      `1` now regenerates (`updating_shots`), and a regeneration writes the
      file **and still fails** (`check_capture`), so it is red until the new
      capture is committed and `git status` must show it. The policy is
      tested over a temporary file rather than by setting the variable:
      `cargo test` runs a binary's tests in one process.

## Mutation table (each tick, red tests named)

| mutation | red |
|---|---|
| **F1, the bug itself:** the `Down(Right)` arm opens the menu without `select_graph_row` | 5, and only these 5 in the whole `graft` suite: `a_right_click_moves_the_cursor_to_the_row_it_opens_the_menu_on`, `x_…`, `y_…` (wrong sha: `8d6cd28` for `45fa51d`), `m_…`, `r_…` |
| `check_capture` returns `Ok` after rewriting (the old behaviour) | `only_an_explicit_regeneration_rewrites_a_capture` |
| `updating_shots` accepts any value (`var_os(..).is_some()`, the old behaviour) | `only_an_explicit_regeneration_rewrites_a_capture` |
| `primary_pane_area` takes `split_area(..).1` (band clears across the detail pane) | `the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it`, and nothing else — as the review found |
| `context_menu_key`'s fall-through sets a generic `key not bound` status instead of handing the key on | `every_verb_the_menu_advertises_is_live_while_it_is_open` (it stayed green on this before m3's hardening) |

## What was done

**F1 — cause (1), the right-click moves the cursor.** `update::mouse`'s
`Down(Right)` arm now runs the same `select_graph_row` the left click runs
before `open_for_history_row`, and sets `on_working_tree` for the
uncommitted row. Nothing forbade (1). (2) — the menu carrying its own target
— would have added a second notion of "the current row" for every verb to
read, which is the duplicate state this milestone keeps being bitten by. The
cursor cannot move while the menu is open (the menu claims `j`/`k`, and any
mouse event while it is open either dispatches or closes it), so menu row and
cursor row agree from the click to the keystroke by construction.

**No test was pinning the old behaviour** — because **nothing tested the
right-click at all.** `MouseButton::Right` appeared exactly once in the whole
crate before this ticket, in `update::mouse` itself. Every context-menu test
opened the menu with `.`, which starts from the cursor, so the gap between
the click and the cursor was invisible to the suite. The harness had no mouse
helper; it has `mouse_down`/`right_click` now.

**F2 — the review's Majors and Minors.** M2 (file sizes), M3 (band vs the
split), m1 (`GRAFT_UPDATE_SHOTS`), m2 (the capture's header artifact and the
mid-sort frame), m3 (`every_verb_…` proving "the verb ran", not "something
changed"). m5's items are C1's, in ticket 17.

## NOTES — for the conductor

- **Not done, and why.** m4 asks for four KNOWN_GAPS rows to be moved to
  `docs/gaps/closed.md`. CLAUDE.md says a builder never reads
  `KNOWN_GAPS.md`, so this is the conductor's. The rows the review names are
  511–514, plus its own four new rows; of those, "right-click menu + verb
  letter acts on the cursor row" is now **closed** (the five tests above),
  "`GRAFT_UPDATE_SHOTS` exported in a shell turns regressions green" is now
  **closed** (`only_an_explicit_regeneration_rewrites_a_capture`),
  "band-with-split untested through a frame" is now **closed**
  (`the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it`), and
  "the golden snapshot depends on the temp-path length of the machine" is
  **closed by construction** — the header is now fitted to `/code/api`, not
  to a temporary path, so there is nothing left to be Linux-dependent. Still
  **unverified**: no Linux run.
- **Captures regenerated, by eye.** `docs/shots/m5-ui-context-menu-verbs-80x24.txt`
  and `-160x48.txt`. Two things changed in the picture: the 80-column
  header reads ` graft /code/api  8 commits` instead of ` graft /code/api  8…`
  (the truncation was an artifact of the temporary path), and `feature`'s
  tip now carries the radar's muted `ⓘ`. The marker was **a race** before —
  the capture was taken before the radar's context job answered — and the
  frame now waits for it. The caption says so. Rule 6: the user judges both.
- **M2 is only half closed.** `view/frame.rs` is 731 lines and
  `it/context_menu.rs` is 961, both still over PROMPT.md §6.2's ~400. The
  review's prescribed split is done (`view/frame/tests.rs`;
  `it/preview.rs` 1711 → 790 + `it/context_menu.rs`); cutting the remainder
  further is a module-boundary decision, not a test move, and wants
  `codebase-design`. Named, not half-fixed.
- **A slow failure mode.** `m_after_a_right_click_…` and `r_after_a_right_click_…`
  fail by *timing out* the harness's 120 s pump when the bug is present,
  because the refused verb queues no job. They pass in ~0.2 s. Green runs are
  unaffected; a red one on these two costs two minutes.
- **`#[ignore]`d capture writers untouched.** `capture_preview_for_docs_shots`,
  `capture_time_machine_for_docs_shots` and `capture_scrolled_preview_for_docs_shots`
  still write without asserting. They never run in a normal suite, so they
  cannot mask a regression — but they are not golden snapshots either.
- **No compaction.** No scout: the ticket said not to, and the review named
  the file and line.
