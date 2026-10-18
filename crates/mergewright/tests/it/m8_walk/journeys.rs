//! The M8 journeys no ticket walked through the app (m8-polish-and-ship/28):
//! real repositories, the real job pool, real keys and wheel; each route
//! counted against the note through [`within_the_note`], the outcome read
//! from the drawn frame (or, for the loop's own theme command, from the
//! command the key asks for).
#![allow(
    clippy::expect_used,
    reason = "a walk step that cannot proceed has nothing left to prove; the panic names why"
)]

use mergewright::session::Clock;
use mergewright::update::{AppState, Command};
use mergewright_core::event::{KeyCode, KeyPress, MouseButton, MouseKind};
use mergewright_ui::detail::DetailFocus;
use mergewright_ui::theme::ThemeKind;

use super::within_the_note;
use crate::app_harness::{App, press_for_commands};
use crate::captures::{frame_at, settled_frame};
use crate::look_polish::opened;
use crate::m7_walk::blame::{blame, blame_ready, drawn as blame_drawn, on_line_22, session};

const COLUMNS: u16 = 120;
const ROWS: u16 = 36;
/// The cursor glyph, on the graph and in the palette.
const CURSOR: char = '\u{25b8}';

fn ctrl_p() -> KeyPress {
    KeyPress::control(KeyCode::Char('p'))
}

/// `text` typed one character at a time: one input as the fence counts it.
fn typed(app: &App, state: &AppState, text: &str) -> AppState {
    let mut state = state.clone();
    for character in text.chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    state
}

fn drawn(state: &AppState) -> Vec<String> {
    frame_at(
        state,
        COLUMNS,
        ROWS,
        Some(Clock::Fixed(mergewright_fixtures::CORPUS_NOW)),
    )
}

/// The corpus octopus opened at 120×36, laid out and settled.
fn octopus() -> (mergewright_fixtures::Scratch, App, AppState) {
    laid_out(mergewright_fixtures::copy_into_temp(
        mergewright_fixtures::Shape::OctopusMerge,
    ))
}

/// `scratch` opened at 120×36, laid out and settled.
fn laid_out(
    scratch: mergewright_fixtures::Scratch,
) -> (mergewright_fixtures::Scratch, App, AppState) {
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.resize(&state, COLUMNS, ROWS);
    let state = settled_frame(&app, state);
    (scratch, app, state)
}

/// The corpus octopus with the cursor clicked onto the unmerged `topic-4`'s
/// row: a branch that is not `HEAD`, where merging it is an action. (The
/// octopus's own topics fold by default; see
/// [`super::octopus_with_an_unmerged_branch`].)
fn on_a_branch_row() -> (mergewright_fixtures::Scratch, App, AppState) {
    let (scratch, app, state) = laid_out(super::octopus_with_an_unmerged_branch());
    let frame = drawn(&state);
    let y = frame
        .iter()
        .position(|line| line.contains("topic-4: start"))
        .unwrap_or_else(|| panic!("topic-4's row is drawn\n{}", frame.join("\n")));
    let row = u16::try_from(y).expect("short screen");
    let state = app.mouse_down(&state, MouseButton::Left, 40, row);
    let frame = drawn(&state);
    assert!(
        frame[y].starts_with(CURSOR),
        "the cursor on topic-4: {:?}",
        frame[y]
    );
    (scratch, app, state)
}

/// No palette on screen: its footer's `esc close` is gone.
fn palette_closed(state: &AppState) -> bool {
    !drawn(state).iter().any(|line| line.contains("esc close"))
}

/// The palette row under its cursor, as drawn.
fn palette_cursor_row(state: &AppState) -> String {
    let frame = drawn(state);
    frame
        .iter()
        .find(|line| {
            line.char_indices()
                .any(|(at, glyph)| glyph == CURSOR && at > 0)
        })
        .cloned()
        .unwrap_or_else(|| panic!("no palette cursor row\n{}", frame.join("\n")))
}

/// J4 `find-key-by-name` = 2: `?` then `merge` — the cursor lands on HERE's
/// merge row, and the row spells its key `m`.
#[test]
fn find_key_by_name() {
    let (_scratch, app, state) = on_a_branch_row();
    let route = ["?", "merge"];
    let state = app.key(&state, KeyCode::Char('?'));
    let state = typed(&app, &state, "merge");
    within_the_note("find-key-by-name", &route);
    let row = palette_cursor_row(&state);
    assert!(
        row.contains("Merge selected branch into current") && row.contains(&format!("{CURSOR}m")),
        "{row:?}"
    );
    drop(state);
    app.close();
}

/// J5 `what-does-key` = 2: `?` then `x` — what `x` does here heads the
/// list, under the cursor.
#[test]
fn what_does_key() {
    let (_scratch, app, state) = on_a_branch_row();
    let route = ["?", "x"];
    let state = app.key(&state, KeyCode::Char('?'));
    let state = app.key(&state, KeyCode::Char('x'));
    within_the_note("what-does-key", &route);
    let row = palette_cursor_row(&state);
    assert!(
        row.contains("Compare against a marked base") && row.contains(&format!("{CURSOR}x")),
        "{row:?}"
    );
    drop(state);
    app.close();
}

/// J6 `run-unrecalled` = 3: `⌃P` `stash` `⏎` — the palette closes and the
/// stash overlay is open, as `S` opens it.
#[test]
fn run_unrecalled() {
    let (_scratch, app, state) = on_a_branch_row();
    let route = ["⌃P", "stash", "⏎"];
    let state = app.press(&state, ctrl_p());
    let state = typed(&app, &state, "stash");
    let state = app.key(&state, KeyCode::Enter);
    within_the_note("run-unrecalled", &route);
    assert!(state.stash.shown, "the stash overlay, as `S` opens it");
    let frame = drawn(&state);
    assert!(
        frame
            .last()
            .is_some_and(|bar| bar.contains("s save") && bar.contains("S/esc close")),
        "the stash overlay's own keys on the bar, the palette gone\n{}",
        frame.join("\n")
    );
    drop(state);
    app.close();
}

/// J9 `light-theme` = 3: `⌃P` `theme light` `⏎` asks the loop for the light
/// theme. The re-theme is the event loop's (it re-reads nothing from
/// git), so the harness reads the command `⏎` asks for; the loop's side
/// is `event_loop::tests`'s.
#[test]
fn light_theme() {
    let (_scratch, app, state) = on_a_branch_row();
    let route = ["⌃P", "theme light", "⏎"];
    let state = app.press(&state, ctrl_p());
    let state = typed(&app, &state, "theme light");
    let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    within_the_note("light-theme", &route);
    assert!(palette_closed(&after), "the palette closed");
    assert!(
        commands.contains(&Command::Retheme(ThemeKind::Light)),
        "{commands:?}"
    );
    drop(state);
    app.close();
}

/// The graph's first drawn commit row: where the list stands.
fn top_row(state: &AppState) -> String {
    drawn(state)
        .into_iter()
        .find(|line| line.contains('\u{25cf}') || line.contains('\u{25cb}'))
        .expect("a commit row")
}

/// J2 `wheel-flick` = 1 (the gesture): a burst of wheel notches over the
/// graph moves it, and once the wheel stops nothing moves after the frame
/// that drew the last notch — the settled frame is that frame.
#[test]
fn wheel_flick() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    crate::idle_advance::long_line(scratch.path());
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, COLUMNS, ROWS);
    let state = crate::idle_advance::laid_out_to_the_end(&app, state);
    let state = settled_frame(&app, state);
    let before = top_row(&state);
    let route = ["wheel"];
    let mut state = state;
    for _ in 0..20 {
        state = app.mouse(&state, MouseKind::ScrollDown, 40, 10);
    }
    within_the_note("wheel-flick", &route);
    // The rows, not the header: its right end reports the window being
    // read (`reading… 500 rows` → `3000 commits`), which is not movement.
    let last_notch = drawn(&state).split_off(1);
    assert_ne!(top_row(&state), before, "the wheel moved the list");
    let state = settled_frame(&app, state);
    let settled = drawn(&state).split_off(1);
    // A row the last notch drew blank may fill in: the harness pumps no
    // job between notches, so the window's read lands only after the
    // burst (the real loop takes it between notches). A drawn row that
    // changes is movement.
    let moved: Vec<String> = settled
        .iter()
        .zip(&last_notch)
        .filter(|(after, at)| !at.trim().is_empty() && after != at)
        .map(|(after, at)| format!("{at:?}\n  → {after:?}"))
        .collect();
    assert!(
        moved.is_empty(),
        "nothing moved after the wheel stopped:\n{}",
        moved.join("\n")
    );
    drop(state);
    app.close();
}

/// J33 `fold-merge-count` = 1, as ease-pass 06 redrew it: the corpus
/// octopus opens folded, none of its topics' rows drawn, its row's tail
/// reading `▸ 3 commits` once the count lands (the three topic commits it
/// brought in; an octopus names no one branch, so no ` from …`). `Space` on
/// it opens it: `▾ 3 commits`, and the three topics' rows appear. `Ctrl+R`
/// does not count it again. (The tail's pending and failed words are
/// `view::rows::fold::tests`'.)
#[test]
fn fold_merge_count() {
    let (_scratch, app, state) = octopus();
    let merge = "Merge branches 'topic-1', 'topic-2' and 'topic-3'";
    let topics = [" topic 1 ", " topic 2 ", " topic 3 "]; // subjects (ease-pass G7)
    let folded = app.pump(state, "the fold's count", |state| {
        drawn(state)
            .iter()
            .any(|line| line.contains(&format!("{merge}  \u{25b8} 3 commits")))
    });
    let frame = drawn(&folded);
    assert!(
        topics
            .iter()
            .all(|topic| frame.iter().all(|line| !line.contains(topic))),
        "folded: no topic's row drawn\n{}",
        frame.join("\n")
    );
    let merge_row = frame
        .iter()
        .find(|line| line.contains(merge))
        .expect("the merge's row");
    assert!(
        merge_row.starts_with(CURSOR),
        "cursor on HEAD's merge: {merge_row:?}"
    );
    let route = ["Space"];
    let opened = app.key(&folded, KeyCode::Char(' '));
    within_the_note("fold-merge-count", &route);
    let opened = app.pump(opened, "the topics' rows", |state| {
        let frame = drawn(state);
        topics
            .iter()
            .all(|topic| frame.iter().any(|line| line.contains(topic)))
            && frame.iter().any(|line| opened_tail(line, merge))
    });
    let opened = settled_frame(&app, opened);
    // An octopus names no one branch: its rows' branch cells stay blank
    // (ease-pass G5), so nothing but lanes comes before the subject.
    for line in drawn(&opened)
        .iter()
        .filter(|line| line.contains(" topic "))
    {
        let before = &line[..line.find(" topic ").expect("filtered on it")];
        assert!(!before.chars().any(char::is_alphabetic), "{line:?}");
    }
    let refreshed = app.press(&opened, KeyPress::control(KeyCode::Char('r')));
    let refreshed = settled_frame(&app, refreshed);
    assert!(
        drawn(&refreshed)
            .iter()
            .any(|line| opened_tail(line, merge)),
        "the count stays over Ctrl+R\n{}",
        drawn(&refreshed).join("\n")
    );
    drop(refreshed);
    app.close();
}

/// The opened octopus's row: its tail reads `▾ 3 commits`. Opened, the
/// lanes widen and the subject is cut (`…'to…`): its start is enough.
fn opened_tail(line: &str, merge: &str) -> bool {
    line.contains(&merge[..24]) && line.contains("\u{25be} 3 commits")
}

/// The detail pane's title row: the first drawn row holding `title`.
fn line_with(state: &AppState, needle: &str) -> Option<String> {
    drawn(state).into_iter().find(|line| line.contains(needle))
}

/// J34 `whitespace-mode-shown` = 1: in the detail pane `w` ends the title
/// `· whitespace ignored`, `w` again takes the words away; in blame (whose
/// default ignores it) the title says nothing until `w`, then
/// `· whitespace counted`.
#[test]
fn whitespace_mode_shown() {
    let scratch = crate::look_polish::repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.resize(&state, COLUMNS, ROWS);
    let tip = app.key(&state, KeyCode::Enter);
    let mut tip = app.pump(tip, "the tip's detail", |state| state.detail.shown);
    for _ in 0..3 {
        if tip.detail.focus == DetailFocus::Files {
            break;
        }
        tip = app.key(&tip, KeyCode::Tab);
    }
    let tip = settled_frame(&app, tip);
    assert_eq!(line_with(&tip, "whitespace"), None, "nothing said at first");
    let route = ["w"];
    let on = app.key(&tip, KeyCode::Char('w'));
    within_the_note("whitespace-mode-shown", &route);
    let on = settled_frame(&app, on);
    let title = line_with(&on, "\u{b7} whitespace ignored").expect("the title names the mode");
    assert!(
        title.trim_start().starts_with("COMMIT ")
            && title.trim_end().ends_with("\u{b7} whitespace ignored"),
        "the pane's title row ends with it: {title:?}"
    );
    let off = settled_frame(&app, app.key(&on, KeyCode::Char('w')));
    assert_eq!(
        line_with(&off, "whitespace"),
        None,
        "`w` again: the words go"
    );
    drop(off);
    app.close();

    let scratch = session();
    let (app, state) = on_line_22(scratch.path(), COLUMNS, ROWS);
    let state = blame(&app, &state);
    let before = blame_drawn(&state, COLUMNS, ROWS);
    assert!(
        before[0].contains(" BLAME \u{b7} ") && !before[0].contains("whitespace"),
        "{:?}",
        before[0]
    );
    let route = ["w"];
    let counted = blame_ready(&app, app.key(&state, KeyCode::Char('w')));
    within_the_note("whitespace-mode-shown", &route);
    let after = blame_drawn(&counted, COLUMNS, ROWS);
    assert!(
        after[0].trim_end().ends_with("\u{b7} whitespace counted"),
        "{:?}",
        after[0]
    );
    drop(counted);
    app.close();
}
