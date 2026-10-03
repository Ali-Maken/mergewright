//! m6-conflicts/70 on a real repository: every `⇥` word on the way out of a
//! rebase stop is where `⇥` lands, the refusal counts its tabs, and the
//! abort from the changes pane is `⇥ ⇥ a y`. Real `git`, real job pool,
//! real keys.
//!
//! Colour is not in a character grid: **unverified** (CLAUDE.md rule 5).

use graft::session::Clock;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;
use graft_ui::detail::DetailFocus;

use super::{CONFLICT_NOW, git, killed_mid_rebase, landed};
use crate::app_harness::{App, changes_pane, cursor_to};
use crate::captures::{check_shot, frame_at, settled_frame};

/// The relaunched rebase stop, with the keyboard on the changes pane's
/// file list: from the banner, `⇥` to the graph and `⇥` to the files.
fn on_the_files(app: &App, state: AppState) -> AppState {
    let state = landed(app, state);
    assert!(state.banner_focus, "{:?}", state.status);
    let state = app.key(&state, KeyCode::Tab);
    let state = app.key(&state, KeyCode::Tab);
    assert!(!state.banner_focus);
    assert_eq!(
        state.detail.focus,
        DetailFocus::Files,
        "the pane's file list"
    );
    settled_frame(app, state)
}

fn row_with<'a>(frame: &'a [String], needle: &str) -> Option<&'a String> {
    frame.iter().rev().find(|row| row.contains(needle))
}

/// The journey "rebase stop, from the changes pane, abort": `⇥ ⇥ a y`, four
/// keys (design note, journey count 4), and git has no rebase left.
#[test]
fn abort_a_rebase_from_the_changes_pane_is_four_keys() {
    let scratch = killed_mid_rebase();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let mut state = on_the_files(&app, state);
    let mut keys = 0;
    for code in [
        KeyCode::Tab,
        KeyCode::Tab,
        KeyCode::Char('a'),
        KeyCode::Char('y'),
    ] {
        state = app.key(&state, code);
        keys += 1;
        if state.operating.is_some() {
            break;
        }
    }
    assert!(keys <= 4, "{keys} keys");
    assert!(state.operating.is_some(), "the abort was sent");
    let state = app.pump(state, "the abort", |state| {
        state.operating.is_none() && state.in_progress.is_none()
    });
    assert!(
        matches!(state.status, Status::Done { .. }),
        "{:?}",
        state.status
    );
    let status = git(repo, &["status"]);
    assert!(!status.contains("rebase"), "{status}");
    drop(state);
    app.close();
}

/// A shell-begun merge, the banner focused: frame 5.
fn merge_banner_frame(columns: u16, rows: u16) -> String {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::StoppedMerge);
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the merge's banner", |state| {
        graft::update::banner::banner_view(state).is_some() && state.working_tree.is_some()
    });
    let mut state = state;
    for _ in 0..3 {
        if state.banner_focus {
            break;
        }
        state = app.key(&state, KeyCode::Tab);
    }
    assert!(state.banner_focus, "`⇥` reaches the banner");
    let state = settled_frame(&app, state);
    let frame = frame_at(&state, columns, rows, Some(Clock::Fixed(CONFLICT_NOW)));
    // The banner's bar is the graph screen's footer, which with the changes
    // pane open below it is not the frame's last row.
    let body = frame.join("\n");
    assert!(row_with(&frame, "c finish").is_some(), "{body}");
    assert!(!body.contains("c continue"), "{body}");
    drop(state);
    app.close();
    frame.join("\n")
}

/// An untracked `new.txt` under the cursor: frame 6. Its row reads `new`.
fn untracked_frame(columns: u16, rows: u16) -> String {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    std::fs::write(scratch.path().join("new.txt"), "hello from new\n")
        .expect("the scratch repo is writable");
    let (app, state) = App::open(scratch.path());
    let state = changes_pane(&app, state);
    let state = cursor_to(&app, state, "new.txt");
    let state = app.pump(state, "the untracked file's read", |state| {
        !matches!(
            state.detail.diff,
            graft::update::detail::Loading::Pending(_)
        )
    });
    let state = settled_frame(&app, state);
    let frame = frame_at(&state, columns, rows, Some(Clock::Fixed(CONFLICT_NOW)));
    let row = row_with(&frame, "new.txt").cloned().unwrap_or_default();
    assert!(row.trim_end().ends_with("new"), "{row:?}");
    assert!(!row.contains("bin"), "{row:?}");
    drop(state);
    app.close();
    frame.join("\n")
}

/// Frames 1–4: the files bar ends `⇥ diff`; `a` names `⇥ ⇥ banner`; one
/// `⇥` and the diff's bar names the banner; the second and the banner has
/// the keyboard.
fn rebase_frames(columns: u16, rows: u16) -> Vec<(String, String)> {
    let scratch = killed_mid_rebase();
    let (app, state) = App::open(scratch.path());
    let state = on_the_files(&app, state);
    let shot = |state: &AppState| frame_at(state, columns, rows, Some(Clock::Fixed(CONFLICT_NOW)));
    let mut frames = Vec::new();

    let one = shot(&state);
    // The pane's bar: the row that says `c continue` (`c cont` at 80, where the
    // ` · ` joins cost the cells: its short rung) (at 120x40 the pane
    // sits below the graph, so it is not the frame's last row).
    let bar = row_with(&one, "c cont").cloned().unwrap_or_default();
    assert!(bar.contains("\u{21e5} diff"), "{bar:?}");
    assert!(!bar.contains("\u{21e5} graph"), "{bar:?}");
    frames.push((
        "files focused: the bar ends `⇥ diff`".to_owned(),
        one.join("\n"),
    ));

    let state = app.key(&state, KeyCode::Char('a'));
    let two = shot(&state);
    let words = "\u{21e5} \u{21e5} banner, then c continue  a abort";
    assert!(row_with(&two, words).is_some(), "{}", two.join("\n"));
    frames.push((
        format!("`a`: the bottom row names `{words}`"),
        two.join("\n"),
    ));

    let state = app.key(&state, KeyCode::Tab);
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    // The refusal is `Sticky`, which only `Esc` dismisses, so it still
    // stands over the bar here, still counting from the files. Recorded in
    // the ticket's NOTES as a question; the bar under it is
    // `the_diff_bar_names_the_banner_during_a_stop`'s.
    let three = shot(&state);
    assert!(row_with(&three, words).is_some(), "{}", three.join("\n"));
    frames.push((
        "one `⇥` (the diff): the sticky refusal still stands over the bar".to_owned(),
        three.join("\n"),
    ));

    let state = app.key(&state, KeyCode::Tab);
    assert!(state.banner_focus, "the second `⇥` is the banner");
    frames.push((
        "a second `⇥`: the banner has the keyboard".to_owned(),
        shot(&state).join("\n"),
    ));
    drop(state);
    app.close();
    frames
}

fn capture(columns: u16, rows: u16) {
    let mut frames = rebase_frames(columns, rows);
    frames.push((
        "a merge stop, the banner focused: `c finish`, not `c continue`".to_owned(),
        merge_banner_frame(columns, rows),
    ));
    frames.push((
        "an untracked `new.txt`: the row reads `new`, not `bin` (its body is n1's open half)"
            .to_owned(),
        untracked_frame(columns, rows),
    ));
    let mut body = String::new();
    for (index, (caption, frame)) in frames.iter().enumerate() {
        body.push_str(&format!("--- frame {}: {caption}\n{frame}\n", index + 1));
    }
    check_shot(&format!("m6/fix-tab-words-{columns}x{rows}.txt"), &body);
}

#[test]
fn capture_fix_tab_words_120x40() {
    capture(120, 40);
}

#[test]
fn capture_fix_tab_words_80x24() {
    capture(80, 24);
}
