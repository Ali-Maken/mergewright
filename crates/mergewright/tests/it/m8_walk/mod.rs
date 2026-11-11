//! M8 acceptance (m8-polish-and-ship/28): every journey of the M8 note's
//! `journey-counts` fence walked through the app and counted against the
//! note, plus the contact sheet's coverage.
//!
//! [`WALKS`] maps each fence id to the file under `tests/it/` that walks it
//! and the `#[test]` that runs the walk; [`every_m8_journey_id_has_a_walk`]
//! holds the map to the fence. Every walk counts its inputs through
//! [`within_the_note`], which fails naming both numbers when a walk takes
//! one key more than the note allows ([`one_key_over_the_note_fails`]).
//!
//! Also the real binary over a pty for what only a terminal shows
//! ([`clone`]).

mod clone;
mod contact_sheet;
mod journeys;
mod look;
mod mergewright;
mod teaches;

use std::path::Path;

/// The M8 note, from the crate's own directory.
const NOTE: &str = "../../docs/design-notes/m8-polish-and-ship.md";

/// The keys the M8 note's fence allows journey `id`, read through
/// [`crate::m5_walk::parse_counts`].
pub fn noted_keys(id: &str) -> usize {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(NOTE);
    let note =
        std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    crate::m5_walk::parse_counts(&note)
        .unwrap_or_else(|why| panic!("the M8 fence does not parse: {why}"))
        .iter()
        .find(|row| row.id == id)
        .and_then(|row| row.keys)
        .unwrap_or_else(|| panic!("the M8 fence has no count for {id}"))
}

/// Fails when a walk of `id` took `taken` inputs and the note allows
/// fewer; the message names both numbers and what was taken.
pub fn within_the_note_taking(id: &str, taken: usize, what: &str) {
    let noted = noted_keys(id);
    assert!(
        taken <= noted,
        "{id}: took {taken} inputs {what}, the note allows {noted}"
    );
}

/// [`within_the_note_taking`] for a walk's route, one entry per input as
/// the fence counts them (a typed name is one input, as its `<name>`).
pub fn within_the_note<T: std::fmt::Debug>(id: &str, route: &[T]) {
    within_the_note_taking(id, route.len(), &format!("{route:?}"));
}

/// The id → walk map: each id of the M8 note's fence, the file under
/// `tests/it/` that walks it, and the `#[test]` running the walk. The file
/// must hold the id as a string literal and call [`within_the_note`] (or a
/// helper that does).
pub const WALKS: &[(&str, &str, &str)] = &[
    (
        "hold-j-stop",
        "held_key.rs",
        "held_j_stops_on_release_unicode",
    ),
    ("wheel-flick", "m8_walk/journeys.rs", "wheel_flick"),
    (
        "jump-oldest",
        "idle_advance.rs",
        "g_after_laid_out_lands_without_laying_out",
    ),
    (
        "find-key-by-name",
        "m8_walk/journeys.rs",
        "find_key_by_name",
    ),
    ("what-does-key", "m8_walk/journeys.rs", "what_does_key"),
    ("run-unrecalled", "m8_walk/journeys.rs", "run_unrecalled"),
    (
        "stopped-how-go-on",
        "palette_in_progress.rs",
        "j7_help_mid_rebase_puts_continue_first_and_enter_asks_what_c_asks",
    ),
    ("light-theme", "m8_walk/journeys.rs", "light_theme"),
    (
        "first-launch",
        "first_launch.rs",
        "a_terminal_launch_counts_and_shows_the_line_until_the_palette",
    ),
    (
        "start-tour",
        "tour.rs",
        "the_palette_starts_the_tour_and_esc_returns_to_his_commit",
    ),
    (
        "start-tour-empty-state",
        "tour.rs",
        "t_on_the_empty_state_starts_the_tour_and_esc_comes_back",
    ),
    (
        "leave-tour",
        "tour.rs",
        "the_palette_starts_the_tour_and_esc_returns_to_his_commit",
    ),
    (
        "open-recent-outside",
        "outside_repo.rs",
        "open_recent_outside_opens_the_newest_at_head_on_one_terminal",
    ),
    (
        "open-second-recent",
        "outside_repo.rs",
        "open_second_recent_is_j_then_enter",
    ),
    ("init-here", "outside_repo.rs", "init_here_is_i_then_y"),
    ("clone", "m8_walk/clone.rs", "clone_from_the_empty_state"),
    (
        "which-repo-branch",
        "look_chrome.rs",
        "capture_the_chrome_at_every_designed_size",
    ),
    (
        "cursor-row-branch",
        "look_chrome.rs",
        "capture_the_cursor_row_in_dark_and_light",
    ),
    (
        "pull-nothing-new",
        "daily_landing.rs",
        "j18_a_pull_with_nothing_new_leaves_the_cursor",
    ),
    (
        "reword-head",
        "m7_walk/amend_into.rs",
        "reword_head_with_a_line_built_on_it",
    ),
    (
        "undo-see-result",
        "daily_landing.rs",
        "j20_u_and_capital_u_land_on_the_result",
    ),
    (
        "quit-while-write",
        "quit_waits/stuck.rs",
        "quit_while_write_is_two_keys",
    ),
    (
        "quit-stuck-write",
        "quit_waits/stuck.rs",
        "stop_now_mid_merge_leaves_it_paused",
    ),
    ("seek-branch", "palette_find.rs", "walk_seek_branch"),
    ("seek-tag", "palette_find.rs", "walk_seek_tag"),
    ("seek-commit-sha", "palette_find.rs", "walk_seek_commit_sha"),
    (
        "seek-commit-words",
        "palette_find.rs",
        "walk_seek_commit_words",
    ),
    ("file-history", "palette_find.rs", "walk_file_history"),
    ("rerun-recent", "palette_find.rs", "walk_rerun_recent"),
    (
        "fold-merge-count",
        "m8_walk/journeys.rs",
        "fold_merge_count",
    ),
    (
        "whitespace-mode-shown",
        "m8_walk/journeys.rs",
        "whitespace_mode_shown",
    ),
    (
        "journal-replaced",
        "m8_walk/mergewright.rs",
        "journal_replaced",
    ),
    (
        "picker-enter-current",
        "m8_walk/mergewright.rs",
        "picker_enter_current",
    ),
];

/// Every id in the M8 fence has a walk in [`WALKS`], and every entry names
/// a real `#[test]` in a file that names the id and counts through
/// [`within_the_note`]. An id with no walk, a stale entry, a file lacking
/// the id literal or the test fn, each fails by name. No count is
/// hard-coded: the fence decides how many ids there are.
#[test]
fn every_m8_journey_id_has_a_walk() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let note = std::fs::read_to_string(root.join(NOTE)).expect("the M8 note");
    let rows = crate::m5_walk::parse_counts(&note).expect("the M8 fence parses");
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    let wrong = walk_problems(&ids, WALKS, root);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// What [`every_m8_journey_id_has_a_walk`] reports for `walks` against the
/// fence's `ids`, one line per problem.
fn walk_problems(ids: &[&str], walks: &[(&str, &str, &str)], root: &Path) -> Vec<String> {
    let mut wrong: Vec<String> = Vec::new();
    for fenced in ids {
        if !walks.iter().any(|(id, ..)| id == fenced) {
            wrong.push(format!("{fenced}: no walk"));
        }
    }
    for (id, file, test) in walks {
        if !ids.contains(id) {
            wrong.push(format!("{id}: not a row of the M8 fence"));
        }
        let path = root.join("tests/it").join(file);
        let Ok(source) = std::fs::read_to_string(&path) else {
            wrong.push(format!("{id}: {file} does not exist"));
            continue;
        };
        if !source.contains(&format!("\"{id}\"")) {
            wrong.push(format!("{id}: {file} never names it"));
        }
        if !source.contains(&format!("#[test]\nfn {test}()")) {
            wrong.push(format!("{id}: {file} has no #[test] fn {test}()"));
        }
        if !source.contains("within_the_note") {
            wrong.push(format!("{id}: {file} never counts through within_the_note"));
        }
    }
    wrong
}

/// The map's check is not vacuous: an id dropped from [`WALKS`], an entry
/// the fence has no row for, and an entry naming a test that is not there
/// each come back by name.
#[test]
fn a_missing_or_stale_walk_is_named() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let note = std::fs::read_to_string(root.join(NOTE)).expect("the M8 note");
    let rows = crate::m5_walk::parse_counts(&note).expect("the M8 fence parses");
    let mut walks: Vec<(&str, &str, &str)> = WALKS
        .iter()
        .copied()
        .filter(|(id, ..)| *id != "wheel-flick")
        .collect();
    walks.push(("no-such-journey", "m8_walk/journeys.rs", "wheel_flick"));
    walks.push(("leave-tour", "tour.rs", "no_such_test"));
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    let wrong = walk_problems(&ids, &walks, root);
    assert!(
        wrong.contains(&"wheel-flick: no walk".to_owned()),
        "{wrong:#?}"
    );
    assert!(
        wrong.contains(&"no-such-journey: not a row of the M8 fence".to_owned()),
        "{wrong:#?}"
    );
    assert!(
        wrong.contains(&"leave-tour: tour.rs has no #[test] fn no_such_test()".to_owned()),
        "{wrong:#?}"
    );
}

/// One key over the note fails, naming both numbers: `leave-tour` is 1
/// (`Esc`), so a two-key route is refused.
#[test]
fn one_key_over_the_note_fails() {
    let allowed = noted_keys("leave-tour");
    let route: Vec<&str> = std::iter::repeat_n("Esc", allowed + 1).collect();
    let refused = std::panic::catch_unwind(|| within_the_note("leave-tour", &route))
        .expect_err("one key over the note must fail");
    let message = refused
        .downcast_ref::<String>()
        .cloned()
        .expect("within_the_note panics with a formatted message");
    assert!(
        message.contains(&format!("took {} inputs", allowed + 1))
            && message.contains(&format!("the note allows {allowed}")),
        "{message}"
    );
    within_the_note("leave-tour", &route[..allowed]);
}

/// The corpus octopus plus `topic-4`: one commit on `main`'s tip that no
/// merge brought in, dated an hour before [`mergewright_fixtures::CORPUS_NOW`].
///
/// The octopus's own topics fold by default (ease-pass 06), so a walk that
/// needs a branch row that is not `HEAD` — where merging it is an action —
/// clicks `topic-4`'s instead (ease-pass 06 D3, route 2: a new ref, the
/// corpus refs untouched). No journey's step count changes: the click is
/// setup, not part of any route.
pub fn octopus_with_an_unmerged_branch() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::OctopusMerge);
    let date = format!("@{} +0000", mergewright_fixtures::CORPUS_NOW - 3600);
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(scratch.path())
        .args([
            "-c",
            "user.name=Test Maker",
            "-c",
            "user.email=test@example.invalid",
            "commit-tree",
            "main^{tree}",
            "-p",
            "main",
            "-m",
            "topic-4: start",
        ])
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .unwrap_or_else(|why| panic!("git commit-tree: {why}"));
    assert!(
        out.status.success(),
        "git commit-tree: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tip = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    crate::app_harness::git(scratch.path(), &["branch", "topic-4", &tip]);
    scratch
}
