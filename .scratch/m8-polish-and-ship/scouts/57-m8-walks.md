# Scout: m8 journey walks — closest existing tests

Paths below are relative to crates/graft/. "App" means the headless harness `tests/it/app_harness.rs`, "pty" means `tests/it/pty.rs`, and "unit" means `update()` tests in src/.

## Harness API (tests/it/app_harness.rs)
- `App::open(path) -> (App, AppState)` :125. Variants: `open_with_clock(path, Clock::Fixed(..))` :151, `open_with_workers` :140, `open_with_askpass` :134.
- `app.key(&s, KeyCode) -> AppState` :194 (plain key). `app.press(&s, KeyPress)` :186. Ctrl+P is `app.press(&s, KeyPress::control(KeyCode::Char('p')))`, the same call as `src/update/tests/keymap_edge/palette.rs:134`. The harness has no type-text helper: send one `Char` per key with `app.key`.
- `app.mouse(&s, MouseKind::ScrollDown, col, row)` :215. `app.resize(&s, c, r)` :251.
- `app.pump(s, "what", |s| done) -> AppState` :260. `app.settle_write(s, what)` :274 asserts `Status::Done`. `app.close()` :290 asserts a clean shutdown.
- Screen text: `crate::captures::frame_at(&s, cols, rows, Some(Clock::Fixed(..))) -> Vec<String>` (captures.rs:113) and `frame(..)` :63. Settle with `captures::settled_frame(&app, s)` :201.
- Fixtures: `graft_fixtures::copy_into_temp(Shape::OctopusMerge)`. The pty uses `Pty::spawn(&Isolated, arg, ceiling, (120,36))` :140 with `send(bytes)`, `wait_for(what, |Screen|)` :303 and `Screen::lines()` :506.

## Per journey
1. **hold-j-stop**: `held_key.rs:88` `held_j_stops_on_release_*`. These replay a timed key script (:170) through their own runner (`run` :367), not App or pty. Real key events, timed. **wheel-flick**: the closest are unit tests `src/update/tests/divider_and_wheel.rs:94,128` (real `ScrollDown` event). The only App wheel test is `tests/it/resolver/modal.rs:88` (the resolver, not the graph). No graph wheel test in tests/it. **jump-oldest**: `idle_advance.rs:105` `g_after_laid_out_lands_without_laying_out` is App and presses `G` (:112).
2. **find-key-by-name / what-does-key**: no test found that types a name into `?`, or presses `x` in help, and asserts the key or meaning shown. *Could not confirm one exists.* Nearest: unit `bar_keys.rs:161` (`?` opens the palette), `keymap_edge/palette.rs:356` (`?` opens and is typed in every place) and `palette_live.rs:130`. App `look_polish.rs:140` presses `M` then `?` and asserts HERE/EVERYWHERE. **stopped-how-go-on**: App `palette_in_progress.rs:48` `j7_help_mid_rebase_puts_continue_first...` presses `?` (:56) at a real rebase stop. Also :139 (edit stop).
3. **run-unrecalled**: unit `keymap_edge/palette.rs:180` (j6, palette stash then Enter). Pty `palette_find.rs:172` `walk_rerun_recent` sends `CTRL_P` (`\x10`, :20). **light-theme**: unit `palette.rs:197` `j9_...` presses ctrl_p, types "theme light", Enter, and asserts `Command::Retheme(Light)` only. It does not see the redraw. Loop side: `src/event_loop/tests.rs:438`.
4. **first-launch**: pty `first_launch.rs:38` and :95 (`Isolated::first_launch()`, pty.rs:83). `:14` is headless and never counts.
5. **which-repo-branch**: App `look_chrome.rs:250` asserts the header starts ` ◆ api  ` (:255). Drawn by `frame_at`, 0 keys. **cursor-row-branch**: `look_chrome.rs:196` asserts the cursor line starts `▸ │ │ ●` (:212). Whether it asserts the branch badge text: *could not confirm* (only the doc comment at :189 names `[feature/two]`).
6. **pull-nothing-new**: App `daily_landing.rs:50` `j18_...` presses `j`, then presses the pull key (pull call not read past :60). **undo-see-result**: App `daily_landing.rs:118` `j20_u_and_capital_u_land_on_the_result`, and `land_on_result.rs:270`.
7. **fold-merge-count**: `look_polish.rs:103`. App. Real `Space` via `app.key` (:110), then pumps until the frame shows `... 'topic-3' +3 ` (:111-115). It does **not** assert the interim ` +` state. The header shows `1 merge folded`.
8. **whitespace**: `look_polish.rs:46`. Real Enter and Tab, then `app.key('w')` (:57). It asserts state `options.ignore_whitespace` and the frame line `· whitespace ignored`. It does not assert that the line is the *title* row. Blame: `look_polish.rs:174` presses `w` (:186) via `m7_walk::blame` helpers and asserts `after[0]` ends `· whitespace counted`.

## Contradictions
- Journey 1 assumes the hold-j test drives the app. It uses a custom paced replay runner (held_key.rs:367), not App or pty.
- Journey 3 light-theme: the unit test proves only the emitted command (palette.rs:208). No end-to-end theme switch test was found in tests/it.
