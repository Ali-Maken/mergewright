//! The first-launch line on the graph's status row (m8-polish-and-ship/18d,
//! rulings R7): the row's fallback for his first five launches, until he
//! first opens the palette. Every state here is given a temp config dir;
//! the user's own is never touched.

use super::place_footers::footer_row;
use super::*;
use crate::first_launch::{FirstLaunch, for_this_launch, test_config, test_read};

const LINE: &str = "first time? ? keys here · ⌃P anything";

fn config(text: Option<&str>) -> tempfile::TempDir {
    test_config(text)
}

/// The graph after a walk, on this launch's line.
fn graph(first_launch: FirstLaunch) -> AppState {
    AppState {
        first_launch,
        ..merge_on_screen(JobId::new(801), 0).0
    }
}

/// The status row as drawn at 120×36 — drawn as is, not settled by a
/// key first (a key would acknowledge the notice under test).
fn footer(state: &AppState) -> String {
    let sized = AppState {
        size: Size {
            columns: 120,
            rows: 36,
        },
        ..state.clone()
    };
    footer_row(&crate::view::tests::draw(&sized, 120, 36))
}

fn press(state: &AppState, key: KeyPress) -> AppState {
    update(state, AppEvent::Core(Event::Key(key))).0
}

/// Launch 1 shows the line; `?`, `⌃P` and `:` each open the palette, write
/// `done`, and the line is gone the same frame.
#[test]
fn shown_until_the_palette_opens() {
    for opener in [
        KeyPress::plain(KeyCode::Char('?')),
        KeyPress::control(KeyCode::Char('p')),
        KeyPress::plain(KeyCode::Char(':')),
    ] {
        let dir = config(None);
        let (line, warning) = for_this_launch(Some(dir.path()));
        assert_eq!(warning, None);
        let state = graph(line);
        assert_eq!(footer(&state), LINE);

        let (opened, commands) = update(&state, AppEvent::Core(Event::Key(opener)));
        assert!(opened.palette.is_some(), "{opener:?}");
        assert!(!opened.first_launch.is_shown(), "{opener:?}");
        assert!(!footer(&opened).contains("first time?"), "{opener:?}");
        // `done` is the loop's to write (`running::carry`), named here.
        assert!(
            commands.contains(&Command::FirstLaunchDone(dir.path().to_path_buf())),
            "{opener:?}: {commands:?}"
        );
        assert_eq!(
            test_read(dir.path()).as_deref(),
            Some("1\n"),
            "update writes nothing itself"
        );

        // Closed again: the hints are back, not the line.
        let closed = press(&opened, KeyPress::plain(KeyCode::Escape));
        assert!(closed.palette.is_none());
        assert!(
            !footer(&closed).contains("first time?"),
            "{}",
            footer(&closed)
        );
    }
}

/// Launch 6 with no palette open: not shown, and the hints are drawn.
#[test]
fn gone_after_five_launches() {
    let dir = config(Some("5\n"));
    let (line, warning) = for_this_launch(Some(dir.path()));
    assert_eq!(warning, None);
    assert!(!line.is_shown());
    let row = footer(&graph(line));
    assert!(!row.contains("first time?"), "{row}");
    assert_eq!(test_read(dir.path()).as_deref(), Some("6\n"));
}

/// A problem and a notice each take the row; the line returns when they
/// clear.
#[test]
fn a_problem_or_notice_wins_the_row() {
    let dir = config(None);
    let state = graph(FirstLaunch::shown(dir.path().to_path_buf()));
    for status in [
        Status::Failed("could not read the refs".to_owned()),
        Status::Waiting("still reading the refs".to_owned()),
    ] {
        let covered = AppState {
            status: status.clone(),
            ..state.clone()
        };
        let row = footer(&covered);
        assert!(!row.contains("first time?"), "{status:?}: {row}");
        let cleared = AppState {
            status: Status::Ready,
            ..covered
        };
        assert_eq!(footer(&cleared), LINE, "{status:?}");
    }
    // The file is untouched by either: only a palette open writes it.
    assert_eq!(test_read(dir.path()), None);
}

/// An unreadable or garbled file: no line, one warning in `words()`, and
/// the app runs on that warning as its first status.
#[test]
fn a_broken_file_warns_once_and_never_stops_the_app() {
    // Unreadable: `launches` is a folder.
    let unreadable = config(Some("/"));
    let garbled = config(Some("x7\n"));
    for dir in [&unreadable, &garbled] {
        let (line, warning) = for_this_launch(Some(dir.path()));
        assert!(!line.is_shown());
        let warning = warning.expect("one warning");
        assert!(warning.starts_with("first-launch: "), "{warning}");
        // `drive` hands it to the first place as its status (`Arrival`).
        let state = AppState {
            status: Status::Sticky {
                reason: warning.clone(),
                keys: None,
            },
            ..graph(line)
        };
        let row = footer(&state);
        assert!(row.starts_with("first-launch: "), "{row}");
        // Keys still work (the cursor moves), and no line is drawn.
        let after = press(&state, KeyPress::plain(KeyCode::Char('j')));
        assert!(after != state, "j moved the cursor");
        assert!(!footer(&after).contains("first time?"));
    }
}
