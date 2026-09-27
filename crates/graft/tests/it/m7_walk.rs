//! M7 journeys walked through the app, each counted against the M7 note's
//! `journey-counts` fence (`docs/design-notes/m7-power-tools.md`).

pub mod amend_into;
pub mod blame;
pub mod history;
pub mod picker;
pub mod pr_contents;
pub mod pull;
pub mod rebase_line;
pub mod switch;

use std::path::Path;

use graft::update::resolver::RowState;
use graft_core::event::KeyCode;

use crate::resolver::file_list::{click_file, on_gone_card, rows, shared_done};

/// The M7 note, from the crate's own directory.
const NOTE: &str = "../../docs/design-notes/m7-power-tools.md";

/// The keys the M7 note's fence allows journey `id`.
fn noted_keys(id: &str) -> usize {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(NOTE);
    let note =
        std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    let rows = crate::m5_walk::parse_counts(&note)
        .unwrap_or_else(|why| panic!("the M7 fence does not parse: {why}"));
    rows.iter()
        .find(|row| row.id == id)
        .and_then(|row| row.keys)
        .unwrap_or_else(|| panic!("the M7 fence has no count for {id}"))
}

/// `git ls-files -u -- <name>`: each stage's mode, oid and number.
fn stages(repo: &Path, name: &str) -> Vec<String> {
    crate::app_harness::git(repo, &["ls-files", "-u", "--", name])
        .lines()
        .map(|line| line.split('\t').next().unwrap_or(line).to_owned())
        .collect()
}

/// `restart-after-done` (Times I left :109): after `⏎` marked `shared.rs`
/// done, the user clicks its row, presses `d`, then `y` — within the note's
/// count. The index then holds the stop's stages (mode and oid) and the
/// markers are back on disk. The fixture's file is `shared.rs` with one
/// conflict block, so one `<<<<<<< ` line, not the ticket's `three.txt`.
#[test]
fn restart_after_done() {
    let scratch = crate::resolver::restart::app_merge_with_a_whole_file_conflict();
    let repo = scratch.path().to_owned();
    let (app, state) = on_gone_card(&scratch);
    let at_stop = stages(&repo, "shared.rs");
    assert_eq!(at_stop.len(), 3, "the stop's stages: {at_stop:?}");
    let state = shared_done(&app, &state);
    assert!(stages(&repo, "shared.rs").is_empty(), "marked done: staged");

    let mut route: Vec<&str> = Vec::new();
    route.push("click shared.rs");
    let state = click_file(&app, &state, "shared.rs");
    let state = app.pump(state, "the done file's read", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == Path::new("shared.rs") && open.job.is_none() && open.done.is_some()
        })
    });
    route.push("d");
    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_some(), "d's box: {:?}", state.status);
    route.push("y");
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "y launched: {:?}", state.status);
    let allowed = noted_keys("restart-after-done");
    assert!(
        route.len() <= allowed,
        "restart-after-done took {} inputs {route:?}, the note allows {allowed}",
        route.len()
    );

    let state = app.pump(state, "the restart, and shared.rs read again", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && rows(state)
                .iter()
                .any(|(path, row)| path == "shared.rs" && *row != RowState::Done)
    });
    assert_eq!(stages(&repo, "shared.rs"), at_stop, "the stop's stages");
    let disk = std::fs::read_to_string(repo.join("shared.rs")).expect("shared.rs on disk");
    assert_eq!(
        disk.lines()
            .filter(|line| line.starts_with("<<<<<<< "))
            .count(),
        1,
        "{disk}"
    );
    drop(state);
    app.close();
    drop(scratch);
}
