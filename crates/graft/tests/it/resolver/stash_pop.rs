//! A conflicted `git stash pop`: no operation, no banner, the same resolver (m6-conflicts/05 scope 4).

use super::*;

// -------------------------------------------------- stash-pop conflict path

/// `KNOWN_GAPS.md` row 229: a conflicted `git stash pop` leaves an unmerged
/// index with no operation and so no banner — the resolver still opens on
/// it, names the sides in its own words, and the last file closes it with
/// no `c continue` offered (m6-conflicts/05 scope 4).
#[test]
fn stash_pop_conflict_resolves_without_a_banner() {
    let scratch = stash_pop_conflict();
    let (app, state) = opened_on(&scratch);
    assert!(state.in_progress.is_none(), "a stash pop is no operation");

    let state = resolver_on(&app, state, "sample.txt");
    let rows = crate::captures::frame(&state, 120, 40);
    // ADR-0043 §4: theirs is `stashed`, the role alone, and the bar's
    // `t` says so.
    assert!(
        rows.last()
            .is_some_and(|bar| bar.contains("o yours  t stashed")),
        "the stash pop's role words are not on the bar:\n{}",
        rows.join("\n")
    );

    // The take journals nothing; `⏎` is an ordinary journaled `Stage` —
    // no live operation to fold into (planner ruling, 2026-09-23).
    let before = journal_entries(scratch.path()).len();
    let mut state = state;
    while open(&state).file.as_ref().expect("a document").open() > 0 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(&app, state);
    }
    assert_eq!(
        journal_entries(scratch.path()).len(),
        before,
        "the take adds no journal entry"
    );
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage and the close", |state| {
        state.operating.is_none() && state.resolver.is_none()
    });
    let after = journal_entries(scratch.path());
    assert_eq!(after.len(), before + 1, "`⏎` adds exactly one entry");
    assert_eq!(
        after.last().map(|entry| entry.description.clone()),
        Some("stage sample.txt".to_owned()),
        "the one entry is the stage"
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "all conflicts resolved"),
        "no banner, so no `c continue`: {:?}",
        state.status
    );
    assert!(!state.banner_focus, "there is no banner to hand back to");
    assert_eq!(
        crate::app_harness::git(scratch.path(), &["ls-files", "-u"]).trim(),
        "",
        "the index holds no unmerged entry"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// The whole-file half of the stash-pop box: the stash deleted a file the
/// branch then changed, so `sample.txt` is `deleted by them`. `o` is its own
/// journal entry, `resolved sample.txt (ours)`, and resolving the only file
/// closes the resolver with no `⏎` (m6-conflicts/05 scope 4).
#[test]
fn stash_pop_whole_file_journals_its_own_row() {
    let scratch = stash_pop_delete_conflict();
    let (app, state) = opened_on(&scratch);
    assert!(state.in_progress.is_none(), "a stash pop is no operation");
    let before = journal_entries(scratch.path()).len();

    let state = resolver_on_whole_file(&app, state, "sample.txt");
    let rows = crate::captures::frame(&state, 120, 40);
    // ADR-0043 §4: theirs is `stashed`, the role alone, and the bar's
    // `t` says so.
    assert!(
        rows.last()
            .is_some_and(|bar| bar.contains("o yours  t stashed")),
        "the stash pop's role words are not on the bar:\n{}",
        rows.join("\n")
    );

    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the whole-file resolve and the close", |state| {
        state.operating.is_none() && state.resolver.is_none()
    });
    let after = journal_entries(scratch.path());
    assert_eq!(after.len(), before + 1, "`o` is exactly one entry");
    assert_eq!(
        after.last().map(|entry| entry.description.clone()),
        Some("resolved sample.txt (ours)".to_owned())
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "all conflicts resolved"),
        "no banner, so no `c continue`: {:?}",
        state.status
    );
    assert_eq!(
        crate::app_harness::git(scratch.path(), &["ls-files", "-u"]).trim(),
        "",
        "the index holds no unmerged entry"
    );
    assert!(scratch.path().join("sample.txt").exists(), "ours kept it");

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn enter_elsewhere_unchanged() {
    // `both.txt` is staged before the app ever opens, so git stops calling
    // it unmerged: the same row, in the same pane, with none of the
    // resolver's claim on it.
    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedMerge);
    crate::app_harness::git(scratch.path(), &["add", "both.txt"]);
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped operation", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    let state = on_the_row(&app, state, "both.txt");
    assert_eq!(
        state.detail.focus,
        graft_ui::detail::DetailFocus::Files,
        "the cursor is in the file list"
    );

    let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    assert!(after.resolver.is_none(), "no resolver on a resolved row");
    assert!(commands.is_empty(), "and nothing was read for one");
    assert_eq!(
        after.detail.focus,
        graft_ui::detail::DetailFocus::Diff,
        "`⏎` keeps the meaning it has everywhere else: focus the diff"
    );

    drop(state);
    app.close();
    drop(scratch);
}
