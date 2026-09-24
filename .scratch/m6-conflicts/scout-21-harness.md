# Scout 21 — walk harness (tests/it/m5_walk) for key-counted journeys

Paths under `crates/graft/tests/it/` unless stated.

## 1. run.rs
- `run_scenario` m5_walk/run.rs:23 — `Fixture::build(shape, remote)` :24 → `(scenario.setup)(&fixture)` :27 → `marks.before = truth(..)` :28 → `drive_app` :29 → `(scenario.expect)` :41 → `fsck_clean` :42 → secret checks :47-59.
- `drive_app` :234 splits steps at `Relaunch`/`Kill` (`chunks` :288); each chunk `App::open(repo)` :252, `app.resize(&state, size.0, size.1)` :254, builds `Walk` :256, `walk.drive` :262.
- `Walk` :328 holds `app: &App`, `fixture`, `columns: u16`, `seen: Seen` — **no AppState field, no rows**; state is threaded through `drive(state, steps, marks)` :337 and returned. Size lives in `state.size` (set by resize).
- `Step::Key(key) => self.app.press(&state, *key)` :345. `Type` folds `app.key(Char)` :346-350. After every step `self.note(&state)` :394 records status lines.
- No frame-to-text in run.rs. Helper: `crate::captures::frame(state: &AppState, columns: u16, rows: u16) -> Vec<String>` captures.rs:61; `frame_at(.., clock: Option<Clock>)` :96 renders `graft::view::view` into `ratatui TestBackend` :107-121, rows trim_end'd, repo path rewritten to `CAPTURE_REPO_PATH`.

## 2. every_scenario.rs
- `walk!` macro :19-31 emits one `#[test] fn $test() { passes($name) }` per pair, plus `every_scenario_has_a_test` asserting `scenarios()` names == macro list **in order** :25-29. List :33-103 (M6 entries :83-102). `passes` → `named` → `run_scenario` :6-17.
- `captures_are_named_keyed_and_listed_in_the_matrix` :108 — capture stems distinct, sorted stems == `M5_STEMS ++ M6_STEMS` :121-122, stem prefix m5-/m6-, keys and on_screen non-empty, stem appears in `history_matrix + conflicts_matrix` :124-141, `stems_line` has 5 tab fields.
- `contact_sheet_captions_cover_every_m6_capture` :250 → `captions_cover_every_capture(docs/shots/m6, "m6")` :262: every TSV line `name\tcaption`, non-empty caption, file exists, no dup; every `m6-*.txt` in docs/shots/m6 (except `m6-contact-sheet.txt`) captioned. Only `m6-` prefixed files are checked — `fix-*`, `prototype-journeys.txt` are not.
- coverage.rs: `M6_STEMS: [&str; 9]` :70-80 (fixed-length array — adding a stem means bumping 9); `M6_SIZES = ["120x40","80x24"]` :26; `conflicts_matrix` :137 → `table(..)` :184 over stems × sizes, dir docs/shots/m6; `M6_MATRIX_PATH` .scratch/m6-conflicts/matrix.txt :29. `stems_are_sorted_and_distinct` :356. Test near :339-349 checks M6 matrix row count against `M6_STEMS.len()`.
- Adding a captured scenario: macro line; stem in `M6_STEMS` (sorted, len); files `docs/shots/m6/<stem>-120x40.txt` and `-80x24.txt` (matrix); caption lines in TSV.

## 3. fixture.rs (m5_walk/fixture.rs)
- M6 setups :303-483: `as_built` :311 (StoppedMerge untouched = shell-begun stopped merge, no journal entry); `merge_aborted` :320 (`git merge --abort` → clean `main`, `feature` beside; walk begins the merge in-app, only kind banner `c` finishes, cites graft-ops/src/operation.rs:574-575); `with_an_editor` :354; `with_a_mergetool` :376 (both.txt only); `twice_conflicting_in_two_places` :409 (shared.txt, 2 blocks, rebase); `stash_to_pop_over_a_conflict` :447 / `stash_pop_conflict` :458 (file `p`); `diverged` :477 (Remote::Local, `remote()?.commit_on_remote`).
- M5 single-file/one-block: `conflicting` :201 (shared.txt, one line, `main` checked out, merge in app); `twice_conflicting` :238.
- StoppedMerge/StoppedRebase shape (graft-fixtures/src/shapes/conflicts.rs:14-28): 13 paths — `both.txt` one block, `three.txt` three blocks, crlf, nofinal, md.md, zdiff3, pic.bin, `theirs-deleted.txt` UD, `ours-deleted.txt` DU, AA, DD/AU/UA. So "modify/delete + a second file" exists only inside the 13-file shape; **no small fixture with just one modify/delete + one other file** — could not find one.
- `TwoStopRebase` shape (shape.rs:88-94): a.txt step 1, b.txt step 2.
- Remote: `Remote::{None,Local,Http}` :28; Local = `LocalRemote::new(shape)` :65, app opens work clone. No setup here runs `git pull` itself; `pull_diverged_offers_m_and_r` scenario uses `diverged`.

## 4. Mouse
- `App::mouse_down(state, MouseButton, column, row)` app_harness.rs:~151, `App::mouse(state, MouseKind, col, row)` ~161, `right_click`. All go through `update`.
- file_list.rs:176 `click_file`: renders `captures::frame(state, size.columns, size.rows)`, finds name on frame row 1 (`gone_like_column` :184), `mouse_down(Left, column, 1)`.
- **No `Step` variant for a click**; only `Step::DragBadgeOnto` (gestures.rs:68-120). A journey Step clicking a file row would be new.

## 5. Keys
- `KeyPress::plain(code)` / `KeyPress::control(code)` graft-core/src/event.rs:70,79. Ctrl+T = `KeyPress::control(KeyCode::Char('t'))` (precedent regressions.rs ctrl-r: `Step::Key(KeyPress::control(ch('r')))`). `KeyCode::Tab`, `BackTab`, `Escape`, `Enter` event.rs:98-108.

## 6. m5_walk example / pty script
- crates/graft/examples/m5_walk.rs: `--only`, `--list`, `--stems` (prints `stems_line` per captured scenario :102-109), `--capture <scenario>` (`capture` → `show` → `prepare_capture`, prelude = capture keys; `GRAFT_CAPTURE_REPO_OUT` env) :110,168-200, `--matrix` → `write_matrix` :111.
- .scratch/m6-conflicts/pty-walk-m6.py: size = `sys.argv[1]` (e.g. `120x40`), extra args filter scenarios :36,46. `CAPTURES` from `m5_walk --stems`, filtered `m6-` :42-45. Main loop :246-266: `Run(..)`, `frame_of` :209, `check_capture` :185 writes `docs/shots/m6/{stem}-{SIZE}.txt` through `graft-fixtures capture`, caps 8 KB soft/200 KB hard. Special case `RELAUNCH = "m6-06-relaunch-sticky"` → `relaunch` :221. New stem = new `Capture` on a scenario; script picks it up automatically.
- Contact sheet is **not** built by the py script: `scripts/contact-sheet.sh m6 docs/shots/m6` reads the TSV, `find m6-*.txt | LC_ALL=C sort`, missing caption prints NOT RECORDED (:36-73).

## Contradictions / unconfirmed
- Brief says "how `Walk` holds the AppState and size": it holds neither (run.rs:328-334); state is passed, size is in `state.size`.
- Brief implies py assembles the sheet: it does not (scripts/contact-sheet.sh does).
- No existing click Step; no small modify/delete + second-file fixture (could not confirm one exists outside the 13-file shape).
