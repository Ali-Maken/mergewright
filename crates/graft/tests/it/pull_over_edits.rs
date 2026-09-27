//! A fast-forward pull over an unsaved edit to a file the pull changes
//! (m7-power-tools/03b; before it, m6-conflicts/85 D's refusal): `p` sets
//! the edit aside, fast-forwards and puts it back — one key, no decision —
//! the cursor on the new `HEAD`, the line `pulled origin/main ✓ · your edits
//! are back`. When the edit and the pull change the same line, the put-back
//! clashes and he lands on the resolver.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::{AppState, Status};
use graft_core::journal::PutBackWhy;
use graft_fixtures::remote::LocalRemote;

use crate::app_harness::{App, git};
use crate::captures::frame;
use crate::remote_by_key::{finish, local, plain, ready, rev, run};
use crate::stash_clash::stash_list;

pub(crate) const SHARED: &str = "shared.txt";
/// Ten lines: an edit at the top and one at the bottom merge.
const TEN: &str = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
/// The teammate's change: the first line.
const THEIRS: &str = "one\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
/// His unsaved edit: the last line.
pub(crate) const MINE: &str = "1\n2\n3\n4\n5\n6\n7\n8\n9\nten\n";
/// Both, after a clean put-back.
pub(crate) const BOTH: &str = "one\n2\n3\n4\n5\n6\n7\n8\n9\nten\n";
/// His unsaved edit on the teammate's line: the put-back clashes.
const SAME_LINE: &str = "my one\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";

/// Pushed `shared.txt` ([`TEN`]), then the teammate's change on top (pushed
/// from here and stepped back, as a teammate's would be): origin is one
/// ahead, touching the file he then edits to `edit` without saving. Returns
/// origin's tip, 40 hex.
pub(crate) fn origin_ahead_over(remote: &LocalRemote, edit: &str) -> String {
    let work = remote.work();
    std::fs::write(work.join(SHARED), TEN).expect("the working clone is writable");
    run(remote, work, &["add", "--", SHARED]);
    run(remote, work, &["commit", "-q", "-m", "add shared"]);
    std::fs::write(work.join(SHARED), THEIRS).expect("the working clone is writable");
    run(remote, work, &["commit", "-q", "-am", "their change"]);
    run(remote, work, &["push", "-q", "origin", "HEAD"]);
    let theirs = rev(work, "HEAD");
    run(remote, work, &["reset", "-q", "--hard", "HEAD~1"]);
    std::fs::write(work.join(SHARED), edit).expect("the working clone is writable");
    theirs
}

/// The changed lines of his unstaged diff, headers and context left out:
/// what he wrote, byte for byte, whatever the pull did around it.
pub(crate) fn his_lines(repo: &Path) -> Vec<String> {
    git(repo, &["diff", "-U0", "--no-color", "--", SHARED])
        .lines()
        .filter(|line| {
            (line.starts_with('+') || line.starts_with('-'))
                && !line.starts_with("+++")
                && !line.starts_with("---")
        })
        .map(str::to_owned)
        .collect()
}

/// The id of the commit the graph cursor is on.
pub(crate) fn selected(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string())
}

/// `p` pressed from the graph, run to its answer, the owed landing spent.
pub(crate) fn pulled(app: &App, state: &AppState) -> AppState {
    let state = app.press(state, plain('p'));
    let state = finish(app, state, "the pull");
    app.pump(state, "the landing", |state| {
        !state.land_on_head && selected(state).is_some()
    })
}

/// `p`, one key: HEAD at origin's tip, his edit back unstaged over theirs,
/// no stash left, the cursor on the new HEAD, `pulled origin/main ✓ · your
/// edits are back`. Was `a_ff_pull_over_overlapping_edits_names_keys_that_work`
/// (m6-conflicts/85 D: the refusal, then `⏎ S s ⏎ esc p`, 7 keys),
/// rewritten to the journey 03b makes of it.
#[test]
fn a_ff_pull_over_overlapping_edits_takes_one_key() {
    let remote = local();
    let work = remote.work();
    let theirs = origin_ahead_over(&remote, MINE);
    let mine = his_lines(work);
    assert_eq!(mine, ["-10", "+ten"], "the fixture's edit");
    let (app, state) = App::open(work);
    let state = app.resize(&state, 120, 40);
    let state = ready(&app, state);

    // One key, `p`: `pulled` presses it and nothing else.
    let state = pulled(&app, &state);

    assert_eq!(rev(work, "HEAD"), theirs, "HEAD at origin's tip");
    assert_eq!(
        std::fs::read_to_string(work.join(SHARED)).expect("the file"),
        BOTH,
        "theirs and his, both"
    );
    assert_eq!(his_lines(work), mine, "his edit byte-equal");
    assert_eq!(
        git(work, &["diff", "--cached", "--name-only"]).trim(),
        "",
        "unstaged, as it was"
    );
    assert!(stash_list(work).is_empty(), "the set-aside stash dropped");
    assert_eq!(
        selected(&state).as_deref(),
        Some(theirs.as_str()),
        "the cursor on the new HEAD"
    );
    let Status::Done { what, also, .. } = &state.status else {
        panic!("a done line: {:?}", state.status);
    };
    assert_eq!(what, "pulled origin/main");
    assert_eq!(also.as_deref(), Some("your edits are back"));
    drop(state);
    app.close();
}

/// Both clauses on the footer at 80 columns, `your edits are back` whole.
#[test]
fn a_ff_pull_over_edits_says_they_are_back() {
    let remote = local();
    let work = remote.work();
    origin_ahead_over(&remote, MINE);
    let (app, state) = App::open(work);
    let state = app.resize(&state, 80, 24);
    let state = ready(&app, state);
    let state = pulled(&app, &state);

    let drawn = frame(&state, 80, 24);
    let line = drawn
        .iter()
        .find(|row| row.contains("pulled origin/main"))
        .unwrap_or_else(|| panic!("no pulled line at 80:\n{}", drawn.join("\n")));
    assert!(
        line.contains("pulled origin/main \u{2713} \u{b7} your edits are back"),
        "{line}"
    );
    drop(state);
    app.close();
}

/// His edit on the teammate's line: the pull is done, the put-back clashes,
/// he lands on the resolver on `shared.txt`, the set-aside stash kept.
#[test]
fn a_ff_pull_clash_lands_on_the_resolver() {
    let remote = local();
    let work = remote.work();
    let theirs = origin_ahead_over(&remote, SAME_LINE);
    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.press(&state, plain('p'));
    let state = finish(&app, state, "the pull");
    let state = app.pump(state, "the resolver", |state| state.resolver.is_some());

    assert_eq!(rev(work, "HEAD"), theirs, "the pull is done");
    let open = state.resolver.as_ref().expect("the resolver is open");
    assert_eq!(open.path, Path::new(SHARED));
    let clash = state.clash.as_ref().expect("the clash is recorded");
    assert_eq!(clash.clash.why, PutBackWhy::SetAside);
    assert_eq!(stash_list(work).len(), 1, "his edit kept in the stash");
    drop(state);
    app.close();
}
