//! m6-conflicts/85 B: a take in the resolver writes the file outside any
//! operation; the changes pane must not keep serving the diff it cached
//! before the take (review `m6-usability-2` B).

use std::path::Path;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright::update::detail::Loading;
use mergewright_core::event::KeyCode;

use crate::app_harness::App;
use crate::captures::{check_shot, frame_at, settled_frame};
use crate::preview::FORK_NOW;
use crate::stash_clash::{NOTES, a_stash_that_clashes, landed, quiet};

const MARKERS: [&str; 3] = ["<<<<<<<", "=======", ">>>>>>>"];

/// The marker lines the pane's served diff holds for the cursor's file.
fn served_markers(state: &AppState) -> Vec<String> {
    let Loading::Ready(served) = &state.detail.diff else {
        panic!("the pane serves no diff: {:?}", state.detail.diff);
    };
    assert_eq!(served.path.as_str(), NOTES);
    served
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .map(|line| line.text.as_str())
        .filter(|text| MARKERS.iter().any(|marker| text.starts_with(marker)))
        .map(str::to_owned)
        .collect()
}

/// `S p y esc`: the pop clashed, and the file looked at in the changes pane —
/// which caches its diff, markers and all.
fn looked_at(app: &App, state: AppState) -> AppState {
    let state = crate::stash_clash::overlay_open(app, state);
    let state = crate::stash_clash::put_back(app, &state, KeyCode::Char('p'));
    let state = quiet(app, landed(app, state));
    let state = app.key(&state, KeyCode::Escape);
    let state = app.pump(quiet(app, state), "the pane's diff", |state| {
        matches!(&state.detail.diff, Loading::Ready(_))
    });
    assert_eq!(
        served_markers(&state).len(),
        3,
        "before the take the pane shows the markers"
    );
    state
}

/// `S p y esc ⏎ o esc`: the pop clashes (`y` confirms the pop preview,
/// m6-conflicts/86) in `notes.md`; `esc` shows the file
/// in the changes pane (its diff, markers and all, now cached); `⏎` reopens
/// the resolver, `o` takes yours, `esc` lands back on the pane with the
/// cursor on the file. The pane's body has no marker line and neither has
/// the file on disk. Writes
/// `docs/shots/m6/stash-clash-take-esc-pane-120x40.txt`.
///
/// Not 85's ticket's `S p o esc` (86's `S p y o esc`): on that path the
/// pane never read the file before the take (its diff is `Idle` under the
/// resolver), so nothing stale is cached and the walk is green without the
/// fix. The review's own walk
/// had looked at the file first; `esc ⏎` is that look.
#[test]
fn a_take_then_esc_shows_the_file_without_markers() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let main = super::rev(path, "main");
    let (app, state) = App::open(path);
    let state = looked_at(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = crate::resolver::settled(&app, state);
    assert_eq!(
        state.resolver.as_ref().map(|open| open.path.as_path()),
        Some(Path::new(NOTES))
    );
    let state = app.key(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Escape);
    let state = quiet(&app, state);
    let state = app.pump(state, "the changes pane's list", |state| {
        state.detail.kept_cursor.is_none()
    });
    let state = settled_frame(&app, state);
    assert!(state.resolver.is_none(), "esc closed the resolver");
    assert_eq!(
        mergewright::update::banner::cursor_conflict(&state).as_deref(),
        Some(Path::new(NOTES)),
        "the cursor is on the CONFLICTS row"
    );

    let on_disk = std::fs::read_to_string(path.join(NOTES)).expect("the file");
    // Yours, whole: no marker line on disk.
    assert_eq!(on_disk, "committed\n", "yours was taken");

    // The body the pane serves for the cursor's file. Yours is what HEAD
    // holds, so the right body is empty, and an empty body is also what a
    // pane too short to draw one looks like: read the served diff itself.
    let state = app.pump(state, "the pane's diff for the cursor", |state| {
        matches!(&state.detail.diff, Loading::Ready(_))
    });
    assert_eq!(
        served_markers(&state),
        Vec::<String>::new(),
        "the pane serves the pre-take diff"
    );
    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    for marker in MARKERS {
        assert!(
            drawn.iter().all(|row| !row.contains(marker)),
            "`{marker}` drawn:\n{}",
            drawn.join("\n")
        );
    }
    assert!(
        drawn.iter().any(|row| row.contains("CONFLICTS")),
        "{}",
        drawn.join("\n")
    );
    let header = format!("diff vs HEAD {}", &main[..7]);
    assert!(
        drawn.iter().any(|row| row.contains(&header)),
        "`{header}` not drawn:\n{}",
        drawn.join("\n")
    );
    check_shot(
        "m6/stash-clash-take-esc-pane-120x40.txt",
        &format!("{HEADER}\n{}\n", drawn.join("\n")),
    );
    drop(state);
    app.close();
}

const HEADER: &str = "\
# m6/stash-clash-take-esc-pane-120x40.txt: 120x40, drawn by `view::view`
# from a real AppState (crates/mergewright/tests/it/m6_stash_put_back/pop.rs,
# a_take_then_esc_shows_the_file_without_markers, asserted). Not a pty
# capture: a character grid with no colour (CLAUDE.md rule 5, colour
# unverified).
#
# What right looks like: `S p y esc ⏎ o esc` — a stash pop that clashes in
# notes.md, a look at the file in the changes pane (markers shown then),
# back into the resolver, `o` takes yours, `esc` back to the changes pane.
#  - The changes pane on notes.md under `CONFLICTS`.
#  - No `<<<<<<<`, `=======` or `>>>>>>>` line anywhere. Yours is what
#    HEAD holds, so the file's diff against HEAD is empty: no body lines is
#    right here (the test reads the pane's served diff to prove it).
#  - The header reads `diff vs HEAD <main's sha7>` (main's tip,
#    `commit over the notes`), not the stash's id.
";
