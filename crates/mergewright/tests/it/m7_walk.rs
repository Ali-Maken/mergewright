//! M7 journeys walked through the app, each counted against the M7 note's
//! `journey-counts` fence (`docs/design-notes/m7-power-tools.md`).

pub mod amend_into;
pub mod blame;
pub mod fork_rows;
pub mod history;
pub mod journal;
pub mod picker;
pub mod pr_contents;
pub mod promise;
pub mod pull;
pub mod rebase_line;
pub mod rewrite_editor;
pub mod rewrite_stops;
pub mod switch;

use std::path::Path;

use mergewright::update::resolver::RowState;
use mergewright_core::event::KeyCode;

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

/// The harness's id → walk mapping (m7-acceptance): each journey id of the
/// M7 note's two `journey-counts` fences, the file under `tests/it/` that
/// walks it, and the `#[test]` that runs the walk. The file must hold the id
/// as a string literal (the walk reads its count through [`noted_keys`] by
/// that id) and the test fn.
pub const WALKS: &[(&str, &str, &str)] = &[
    (
        "squash-last-three",
        "m7_walk/rewrite_editor.rs",
        "squash_last_three",
    ),
    (
        "swap-adjacent",
        "m7_walk/rewrite_editor.rs",
        "swap_adjacent",
    ),
    ("fix-into-older", "m7_walk/amend_into.rs", "fix_into_older"),
    ("absorb-staged", "m7_walk/amend_into.rs", "absorb_staged"),
    (
        "amend-with-dependents",
        "m7_walk/amend_into.rs",
        "amend_with_dependents",
    ),
    (
        "rebase-line-onto-main",
        "m7_walk/rebase_line.rs",
        "r_y_moves_the_line",
    ),
    (
        "squash-merged-bottom",
        "m7_walk/rebase_line.rs",
        "r_y_drops_the_squash_merged_branch",
    ),
    (
        "keep-a-suggested-drop",
        "m7_walk/rebase_line.rs",
        "keep_a_suggested_drop",
    ),
    (
        "rewrite-stop-to-first-file",
        "m7_walk/rewrite_stops.rs",
        "rewrite_stop_to_first_file",
    ),
    (
        "edit-stop-lands",
        "m7_walk/rewrite_stops.rs",
        "edit_stop_lands",
    ),
    (
        "edit-stop-amend-continue",
        "m7_walk/rewrite_stops.rs",
        "edit_stop_amend_continue",
    ),
    (
        "abort-from-a-chain-stop",
        "m7_walk/rewrite_stops.rs",
        "abort_from_a_chain_stop",
    ),
    ("restart-after-done", "m7_walk.rs", "restart_after_done"),
    (
        "switch-named-branch",
        "m7_walk/picker.rs",
        "switch_named_branch",
    ),
    ("switch-back", "m7_walk/picker.rs", "switch_back"),
    (
        "switch-with-edits",
        "m7_walk/picker.rs",
        "switch_with_edits",
    ),
    (
        "cherry-pick-in-view",
        "m7_walk/picker/take.rs",
        "cherry_pick_in_view",
    ),
    (
        "cherry-pick-other-branch",
        "m7_walk/picker/take.rs",
        "cherry_pick_other_branch",
    ),
    ("pr-contents", "m7_walk/pr_contents.rs", "pr_contents"),
    (
        "pull-current-branch",
        "m7_walk/pull.rs",
        "pull_current_branch",
    ),
    (
        "blame-line-to-commit",
        "m7_walk/blame.rs",
        "blame_line_to_commit",
    ),
    (
        "commit-back-to-blame",
        "m7_walk/blame.rs",
        "commit_back_to_blame",
    ),
    ("blame-who-at-line", "m7_walk/blame.rs", "blame_who_at_line"),
    ("blame-line-before", "m7_walk/blame.rs", "blame_line_before"),
    ("function-history", "m7_walk/history.rs", "function_history"),
    ("file-history", "m7_walk/history.rs", "file_history"),
    (
        "lost-to-shell-reset",
        "m7_walk/journal.rs",
        "the_three_lost_work_journeys",
    ),
    (
        "deleted-branch-back",
        "m7_walk/journal.rs",
        "the_three_lost_work_journeys",
    ),
    (
        "edit-in-undo-backup",
        "m7_walk/journal.rs",
        "the_three_lost_work_journeys",
    ),
];

/// Every journey id in both M7 fences has a walk in [`WALKS`], and every
/// entry names a real `#[test]` in a file that counts that id. An id with no
/// walk, a stale entry, or a renamed test fails, each named.
#[test]
fn every_m7_journey_id_has_a_walk() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let note = std::fs::read_to_string(root.join(NOTE)).expect("the M7 note");
    let rows = crate::m5_walk::parse_counts(&note).expect("the M7 fences parse");
    let mut wrong: Vec<String> = Vec::new();
    for row in &rows {
        if !WALKS.iter().any(|(id, ..)| *id == row.id) {
            wrong.push(format!("{}: no walk", row.id));
        }
    }
    for (id, file, test) in WALKS {
        if !rows.iter().any(|row| row.id == *id) {
            wrong.push(format!("{id}: not a row of either M7 fence"));
        }
        let path = root.join("tests/it").join(file);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
        if !source.contains(&format!("\"{id}\"")) {
            wrong.push(format!("{id}: {file} never names it"));
        }
        if !source.contains(&format!("#[test]\nfn {test}()")) {
            wrong.push(format!("{id}: {file} has no #[test] fn {test}()"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
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
