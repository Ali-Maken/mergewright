# Review: M6 look-and-finish, wave 2 (tickets 59, 64, 60, 52, 50, 51, 61)

Range `63effe89..368afe3a`, read-only. Reviewer, 2026-09-25. Built and tested in a throwaway worktree at `368afe3a`
with its own `CARGO_TARGET_DIR` (removed afterwards); the main tree was not built.

**Counts: 0 Critical · 3 Major · 9 Minor.** Ticks: 22 checked, 22 held. No data-loss path found. No false tick.
**Heavy tier is red**: one capture test, M3.

## What I verified, and how

**The three key-changing tickets (the focus).**

- **Can `y` confirm the force (lease) box anywhere? No.** `ConfirmThen::takes_y` (`crates/graft/src/update/confirm.rs:53-67`)
  is the only gate, and `Operate(Push(params))` returns `params.lease.is_none()`. Every lease box goes through
  `Operate(Push)` with `lease: Some`: `push_box.rs:30-33` and `after_push.rs:247-250`. A grep for `Operation::Push` in
  non-test `crates/graft/src` finds no other construction site. The `y` arm (`confirm.rs:170-172`) needs
  `pending.yes().is_some()`, and `yes()` returns `None` before it reads any label when `takes_y` is false
  (`:37-42`). The confirm box claims keys before the finish summary, the resolver and the banner (`keys.rs:128-133`),
  so no other `y` handler sees a key while a box is up. Tests: `y_is_swallowed_where_the_box_does_not_take_it`
  covers both lease sites, `repush_chosen`'s own box, undo-after-push and revert. `remote_by_key::lease_box_keeps_on_y`
  (`tests/it/remote_by_key.rs:1121-1160`) runs a real rejected push and checks the frame is unchanged, nothing is
  sent, no write starts and origin does not move.
- **Can `⏎` or `c` commit or finish while an operation should refuse? No new path found.**
  - `⏎` in the commit editor's subject calls the same `commit` that `⌃⏎` calls (`commit_editor.rs:682-688`), so it
    gets the same `refusal`. While a job runs, `editor_key` eats the key first. Pinned by `a_refused_enter_sends_nothing`
    (three rows, each compared with `⌃⏎`) and `enter_while_a_job_runs_is_eaten_like_ctrl_enter`.
  - `c` in the pane or the graph is the banner's `continue_pressed` (`keys.rs:270-285`), and only where
    `pane_c_acts` holds (`commit_editor.rs:81-89`). It gets the banner's refusals: `status_read::owed` gives CHECKING,
    and `continue_refusal` refuses while files are unmerged. A merge only opens the summary, which writes nothing.
    Rebase, cherry-pick and revert go to `resume`, which refuses while a write runs (`banner.rs:571`). A shell-begun
    non-merge and a bisect keep M3a's words with nothing sent (`live_c_tests.rs:140-170`).
  - The summary's `y` (`finish/keys.rs:220-267`) refuses while the summary is still reading, while the subject is
    empty, and while a write runs. After that, ops' gate decides.
  - The owed auto-open (`resolved.rs:227-258`) waits for every take, the stage, the resolver's read and the status
    read. It spends nothing while a file is still unmerged, and it goes through the same `continue_pressed`.
- **Can a stale read make a key act on the wrong state? No new case found; the known one stands.**
  - `live_operation` reads Graft-begun only once the journal read has landed (`commit_editor.rs:58-66`). Until then,
    pane `c` on a Graft-begun rebase refuses in words, which is the safe direction.
  - A stale `in_progress` gives the banner's own path, and ops refuses `NotInProgress` in words.
  - The owed summary is gated on the status read the Stage asked for (`the_read_after_the_last_stage_is_owed_when_it_lands`).
  - The discard box's `y` acts on `then`, the same thing `⏎` acts on. Only the word shown comes from the label.
  - KNOWN_GAPS M6/51 (`space` then a fast `c` opens the editor on a stale `nothing staged`) is unchanged. `⏎` in the
    subject now makes it one key more likely to be met, because the refusal is what a fast `⏎` hits.
- **Rule 3.** I scanned the added production lines for `unwrap()`, `expect(`, `let _`, `unwrap_or_default` and
  `Err(_) =>`.
  - Only `finish_summary.rs:303` hits (`split(' ').next().unwrap_or_default()`, on an iterator that always yields).
    It swallows no error.
  - `page()`'s `unwrap_or(isize::MAX)` (`finish/keys.rs:115`) saturates a `usize` to `isize` conversion. It swallows
    nothing.
  - `status_read::abandon` now clears the owed open, and every caller puts the failure's words on the status line
    (`a_failed_status_read_cancels_the_open_and_says_so`). One silent path is new; see m1.
- **Colour:** not judged. It stays unverified (rule 5). The buffer tests assert tokens only.
- **Contact-sheet parts at `368afe3a`**, all at or under 200 000 B:
  - m6-1 176 539 B, m6-2 178 148 B, m6-3 141 905 B;
  - m3 124 826 B;
  - m5-1 171 317 B, m5-2 111 541 B.

  No file under `docs/shots` is over 200 000 B.
- **Machine paths:** the added lines in the whole diff contain none of `/Users/`, `/private/var`, `/var/folders` or
  the user name. The two `<os-tmp>/graft-remote-…` mentions are the redacted placeholder, in NOTES and in KNOWN_GAPS.
- **Architecture:**
  - No `Cargo.toml` changed.
  - The new graft-ui types (`ConfirmYes`, `LiveC`, `FinishRow`) are view vocabulary.
  - `graft_ops::undo::operation_live_words` sits in ops, which both callers already use.
  - The MAP.md rows match the code at the lines I read.

## Tick spot-checks (22 checked, 22 held)

| Ticket | Checked | Held | Which |
|---|---|---|---|
| 50 | 4 | 4 | `y_is_swallowed_where_the_box_does_not_take_it` (5 rows, asserts box, status, commands, operating); `y_and_enter_emit_the_same_commands` (10 rows, state equality); `a_narrow_bar_keeps_the_y_hint` (real discard opener at 80×24); journey rows 3/3/2 in the fence (`m6-conflicts.md:504-511`) plus `capture_fix_box_takes_y` asserting origin unmoved |
| 51 | 3 | 3 | `a_refused_enter_sends_nothing` (three rows vs `⌃⏎`); `stage_one_file_and_commit_is_six_steps` (`steps <= 6` and `== 6`); `the_footer_follows_focus` (every `Field`) |
| 52 | 3 | 3 | `the_bar_offers_c_exactly_where_c_acts` (5 rows × 3 frames, bar ⇔ act); `c_in_the_changes_pane_opens_the_summary`; `finishing_from_the_pane_journals_once_and_u_lands_on_the_banner` |
| 59 | 3 | 3 | `preview_title_shortens_then_cuts`; `preview_frame_styles` (accent border cells, `y` accent+BOLD); `hit_round_trips_inside_the_frame` (80/120/187) |
| 60 | 3 | 3 | `a_take_still_landing_holds_the_summary` (held, then once, never twice); `switching_file_cancels_the_open`; `a_failed_status_read_cancels_the_open_and_says_so` (deviation stated in the tick) |
| 61 | 3 | 3 | `no_more_conflicts_is_not_repeated_in_the_bar`; `the_no_more_notice_is_gone_after_the_next_move`; `the_notice_lifetime_today` |
| 64 | 3 | 3 | `resolver_bar_shows_every_key_at_80`; `done_list_marks_start_over`; m2's one wording (`preview.rs`, `execute.rs` both call `operation_live_words`) |

60's tick naming `space_on_the_last_conflicts_row_lands_on_the_banner` is disclosed ("see NOTES Q3"), so I count it
as held. Its name is M2 below.

## Critical

None.

## Major

### M1. Four source files crossed or grew past PROMPT 6.2's ~400-line bound in this range; `finish_summary.rs` grew by 182 lines

- **Where:**
  - `crates/graft-ui/src/finish_summary.rs`: 428 → 610 lines. `impl FinishScreen` alone is `:258-566`, about 300 lines.
  - `crates/graft-ui/src/conflict_resolver.rs`: 387 → 449.
  - `crates/graft-ui/src/graph_screen/hints/lists.rs`: 388 → 434.
  - `crates/graft/src/update/resolver.rs`: 388 → 415.

  The counts are `wc -l` at `63effe89` and at `368afe3a`, with the tests already in separate files for the first
  three.
- **Why it matters:** PROMPT 6.2 (files under about 400 lines) is a hard rule, and CLAUDE.md makes readability
  findings Major. The last review filed the same class as its M2 (`theme/mod.rs`, `files.rs`), and ticket 64 fixed
  those. This wave then added four more. `finish_summary.rs` is the one that matters: the frame, the border's styled
  keys, the list, the message and the notice are one 300-line `impl`.
- **Fix (pure moves, like 64's):**
  - `finish_summary.rs`: move the frame and border drawing (`preview_title`, `preview_line`, `footer_hints`, the
    border part of `render`) to `finish_summary/frame.rs`, and the list rows and `tints` to `finish_summary/list.rs`.
  - `conflict_resolver.rs`: move `FinishRow` and `finish_row_at` (`:154-200`) to `conflict_resolver/finish_row.rs`.
  - `lists.rs`: move the finish and done lists to `lists/resolver.rs`.
  - `resolver.rs`: move `cancel_owed_finish` beside `finish_if_owed` in `resolved.rs`.
  - The existing tests stay green unedited.

### M2. The landing after `space` on the last `CONFLICTS` row contradicts the design note; the test pinning it is named for the opposite, and the question was never filed

- **Where:**
  - `crates/graft/tests/it/resolver/opens_summary.rs:142`: the test is `space_on_the_last_conflicts_row_lands_on_the_banner`,
    but `:159` asserts `!state.banner_focus, "the pane keeps the keyboard"`.
  - `docs/design-notes/m6-conflicts.md:951-952` says: "The last file done outside the resolver (`space` on a
    `CONFLICTS` row): → the banner, `every file done ✓ · c finishes the merge`, unchanged."
  - Ticket 60's NOTES Q3 reports the mismatch. KNOWN_GAPS has no row for it: the log's 60 line files only M6/60
    (Q1), and `grep -n "space_on_the_last\|Q3" KNOWN_GAPS.md` finds nothing.
- **Why it matters:**
  - Where the app lands him after an event is part of the journey (pipeline step 4). The note, the code and the test
    name now say three different things.
  - A test whose name states the opposite of its assertion misleads the next reader (PROMPT 6.2, names). It is also
    the kind of disagreement rule 7 asks to be filed, not left in NOTES.
  - Since 52, the pane he is left in draws `c finish`, and `c y` finishes. So the built landing may be the better
    one, but nobody has decided that.
- **Fix:**
  - Rename the test to what it pins, e.g. `space_on_the_last_conflicts_row_opens_no_summary_and_keeps_the_pane`.
  - File a KNOWN_GAPS question: banner or pane, after `space` on the last row.
  - Correct the note's line to the decided landing.

### M3. `capture_fix_c_live_frames` depends on timing: it fails every time it runs alone, and it failed the heavy tier

- **Where:**
  - `crates/graft/tests/it/resolver/shell_begun.rs:1327-1359` (`bisect_frame`).
  - It settles with `settled_frame` (`:578-584`), which waits only until no row reads `reading…` or `sorting…`.
  - The committed frame 1 (`docs/shots/m6/fix-c-live-frames-187x50.txt:17`) has the radar's info mark
    (`│ ●  ⓘ [feature] …`). The radar context is an async answer, and nothing in the settle waits for it.
- **Observed, at `368afe3a` in a clean worktree:**
  - The full `cargo nextest run` passed, twice (3232/3232).
  - Run alone, it failed 5 of 5 under `cargo nextest run -E 'test(capture_fix_c_live_frames)'` and 3 of 3 under
    `cargo test --features heavy-tests`: `drawn │ ●    [feature] …` (no `ⓘ`).
  - It failed in both heavy-tier runs (`--test-threads=2`), and in the first one it stopped the other binaries from
    running.
  - So the result depends on whether the radar lands before the settle. Under full-suite load it does; alone, it
    does not.
- **Why it matters:** the heavy tier, which every milestone review must pass, is red. The fast gate is green only by
  timing, so a quieter machine turns it red with no code change. The capture the user judges shows a mark that the
  test does not control.
- **Fix (cause):** make `bisect_frame`'s settle also wait until the radar's answer has landed. I did not trace which
  state field that is; the builder pins it. Then retake the capture and run it alone five times. Do not make the check
  tolerant.

## Minor

### m1. Pane or graph `c` mid-rebase does nothing, and says nothing new, while a write runs; the bar still says `c continue`

- **Where:** `crates/graft/src/update/keys.rs:270-285` → `banner::continue_pressed` → `resume`, which returns `false`
  with no words when `operation_in_flight` (`crates/graft/src/update/banner.rs:571-573`).
- **The cause:** before 52 the same key went to `commit_editor::open`, which answered in M3a's words. The banner's own
  `c` has the same silence; that is pre-existing.
- **Why it's Minor:** the status line already shows the running write's `Busy` words, so he can tell something is in
  progress. `the_bar_offers_c_exactly_where_c_acts` does not cover `state.operating` (`live_c_tests.rs` has no
  `operating`).
- **Fix:** give `continue_pressed` one refusal for "a write is already running", in the words the summary uses
  (`finish/keys.rs:252`), and add a row with `operating: Some(_)` to the bar-vs-key table.

### m2. `every_box` copies nine sites' labels by hand, so a relabelled button turns `y` off with every test green

- **Where:** `crates/graft/src/update/confirm.rs:484-600`. Only the push rows use the site's own builder
  (`push_box`). The stash-drop (`:512`), undo and redo drift (`:551-554`), skip, abort and start-over rows restate
  `with_labels` literals.
- **How it fails:** `ConfirmYes::from_label` (`graft-ui/src/graph_screen/hints/context.rs:312`) returns `None` for an
  unknown label, so `y` is quietly swallowed.
- **Coverage:** real-site coverage exists for discard (`a_narrow_bar_keeps_the_y_hint`), start-over and abort (the
  walks `j d y`, `a y`). There is none for stash drop, undo or redo drift, or skip.
- **Fix:** build each row by calling the site's opener on a fixture state, or add one assertion per opener's own test
  that `yes().is_some()`.

### m3. The push journeys are not step-counted by a failing check

- **Where:**
  - `docs/design-notes/m6-usability-fixes.md:58` sets `P y` at **2**, and `:62` sets `p r r P y` at **5**.
  - `tests/it/m4_walk/push.rs:127-165` `push_plain_by_y` has two keys by construction. Ticket 50's tick itself says
    "the m4 walk has no count check".
- **Why it matters:** pipeline step 7 says the walk "fails if it exceeds the note's count". The conflict journeys
  have this check (`journey_counts_match_the_note`); the push journeys do not.
- **Fix:** a `journey-counts`-style row for `push` and `rejected-push-to-pushed`, with the m4 walk counting keys the
  way `conflict_journeys.rs` does.

### m4. The note still says `editor-to-finish` is 5

- **Where:** `docs/design-notes/m6-conflicts.md:916` reads "Unchanged: `editor-to-finish` `Esc ⇥ ⇥ c y` **5**". The
  fence at `:509` says **2** (`c y`), lowered by 52.
- **Fix:** reword `:916` to the fence's value, or strike it.

### m5. The status words `<file> done ✓ · every file done` are set but never drawn while the summary is open

- **Where:** `crates/graft/src/update/resolver/resolved.rs:204-217` (`last_done`). Ticket 60 NOTES Q4 says the summary
  takes the whole frame, bottom row included, so the line is not on screen. It is not in KNOWN_GAPS.
- **Why it's Minor:** it is words that no screen shows. The acceptance line for `fix-last-file-opens-summary`
  promised them, and they cannot appear.
- **Filed as a question:** should the summary leave the status row, or should the words go? It needs a decision, not
  a fix.

### m6. The two subject fields that ADR-0044 §3.4 pairs word the same keys differently

- **Where:**
  - The commit editor's subject (`crates/graft-ui/src/commit_editor.rs:172`):
    `⏎ commit  ⇥ body  ⌃E $EDITOR  ⌃S sign-off  esc cancel`.
  - The finish summary's subject (`crates/graft-ui/src/graph_screen/hints/lists.rs:421-426`):
    `⏎ commit  ⇥ body  ⌃E editor  esc back`.
- **Why it matters:** the rule is "same key, same meaning, same words" (`docs/research/ux-principles.md` §2 #4). Both
  `esc`s keep the draft, and both `⌃E`s hand the message to `$EDITOR`. It is a textual finding, and the words are the
  user's to pick.
- **Fix:** one word for `⌃E` in both. `esc` may stay apart if "cancel" versus "back" is meant; say so in the note.

### m7. KNOWN_GAPS M6/48 is half stale

- **Where:** `KNOWN_GAPS.md:783` says "The hint bar offers `c commit` and `a amend` while an operation is live, and
  both refuse".
- **What changed:** since 52, no graph-row or pane bar says `c commit` while live (`the_bar_offers_c_exactly_where_c_acts`,
  `no_graph_row_context_is_unwrapped_while_live`). `a amend` was never drawn (M5/33, `:543`).
- **Fix (conductor):** close the `c` half, and keep or merge the `a` half with M5/33.

### m8. Keymap §2.3c says the box's footer leads with `y`, but the box has no footer

- **Where:** `docs/design/02-keymap.md:230` says "the box's footer leads with `y <verb>`". The box has no footer; the
  `y` is drawn on the hint bar. The usability note records this (`m6-usability-fixes.md:146-147`), but the keymap,
  the shared reference, does not.
- **Fix:** "the hint bar leads with `y <verb>`".

### m9. The summary's footer styles its first key by splitting rendered text

- **Where:** `crates/graft-ui/src/finish_summary.rs:300-304` renders the hints to a string, then takes up to the first
  space as the key to paint `accent` BOLD.
- **Why it matters:** it is correct for today's lists (`y`, `⏎`, `⌃⏎`). A leading key with a space in it, or a change
  to `hints_text`'s spacing, would silently paint the wrong span. That is clever code where plain code would do
  (PROMPT 6.2).
- **Fix:** build the spans from `footer_hints(..)`: the first `Hint`'s `key` strong, the rest muted.

## Gates (worktree at `368afe3a`, run one at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo nextest run` | **3232 passed, 1 skipped**. 195 s the first time and 143 s the second (the machine is shared with the acceptance build) |
| `cargo test --doc` | 3 passed |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | **FAILED**. First run: stopped at `it` (437 passed, 1 failed). Re-run with `--no-fail-fast`: **3266 passed, 1 failed** (`resolver::shell_begun::capture_fix_c_live_frames`, M3), 369 s |
| Release perf: `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture` | **13 passed**. Frame p99: Pixel 8.2 ms / Unicode 8.4–8.8 ms (50k refs and fixture; max 10.7 ms, all under the 16 ms budget). First paint: 15.5–38.4 ms on the 100k fixtures. Peak RSS: ≤ 102 MB |
| Fuzzing, benchmarks | not run in this wave review (7.1 time limits; the milestone review runs them) |

- **The 5 slowest fast-tier tests** (nextest, second run):
  - `graft-git diff_fixture::parse_unified_matches_numstat_for_every_shape` 9.0 s;
  - `graft m5_walk::every_scenario::merge_resolved_with_the_keys` 8.6 s;
  - `graft-ops conflict_ops::whole_file_shapes_resolve_like_git` 8.2 s;
  - `graft-ops undo_cancel::cancel_at_every_step_boundary_is_clean` 7.5 s;
  - `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.7 s.
- **Peak memory:** the nextest runner's own maximum RSS was 272 MB. The app's peak RSS in the release budgets was
  ≤ 102 MB.
- **Size of `target/`:** 7.3 GB in the throwaway worktree, after the debug, heavy and release builds. The worktree and
  its target are removed.

## Not verified

- **Colour** in any of the seven tickets (rule 5). The frames' look is the user's to judge (rule 6).
- **The pty captures** (`m6-06`, `m6-08`, `m6-18`, `m3-04/05`): I did not re-run them. Only their headers and the
  asserted lines were read.
- **`c` from a commit's focused diff pane** (52's third path): it is swept, but not walked against real git. The
  builder says the same.
- **Fuzzing and benchmarks:** not run for this wave-level review.
