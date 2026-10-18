//! `Ctrl+T`: theirs in every open block, one write, then `⏎`
//! (m6-conflicts/16), against the stopped-merge corpus. A merge stages
//! nothing until the review's `y` (ease-pass/05b).

use super::*;

/// What taking theirs in every block of `path` gives, block by block — the
/// parser's own answer, not `git checkout --theirs` (research §1.5: the two
/// differ where git merged a hunk cleanly beside a conflicted one).
fn per_block_theirs(repo: &Path, path: &str) -> Vec<u8> {
    let bytes = std::fs::read(repo.join(path)).expect("the conflicted file");
    let mut file = mergewright_git::conflict::parse(&bytes).expect("it parses");
    file.take_all(mergewright_git::conflict::Take::Theirs);
    file.render()
}

#[test]
fn ctrl_t_takes_theirs_everywhere_and_a_merge_stages_nothing() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let want = per_block_theirs(&repo, "three.txt");
    let state = resolver_on(&app, state, "three.txt");
    let state = app.press(&state, KeyPress::control(KeyCode::Char('t')));
    let state = settled(&app, state);
    assert_eq!(
        std::fs::read(repo.join("three.txt")).expect("it reads"),
        want,
        "one write, every block theirs"
    );
    assert_eq!(open(&state).file.as_ref().expect("a document").open(), 0);

    // `⏎` on the solved file: on to the next open clash, nothing staged —
    // the review's `y` stages it (ease-pass/05b).
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.operating.is_none(), "⏎ in a merge queued a write");
    let state = settled(&app, state);
    assert_ne!(open(&state).path, Path::new("three.txt"), "on to the next");
    let unmerged = crate::app_harness::git(&repo, &["ls-files", "-u", "--", "three.txt"]);
    assert!(!unmerged.trim().is_empty(), "three.txt was staged");

    drop(state);
    app.close();
    drop(scratch);
}

/// `three.txt` after `Ctrl+T`: every header took theirs, the notice counts
/// the blocks — the capture the user judges by eye (m6-conflicts/16).
#[test]
fn take_all_capture() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    let state = app.press(&state, KeyPress::control(KeyCode::Char('t')));
    let state = settled(&app, state);
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(
        body.contains("took theirs · feature in 3 blocks · n goes to the next"),
        "{body}"
    );
    // Solved in a merge: the bar points on to the next file (ease-pass/05c).
    assert!(body.contains("n next file:"), "{body}");
    crate::captures::check_shot("m6/fix-take-all-120x40.txt", &format!("{body}\n"));
    drop(state);
    app.close();
    drop(scratch);
}
