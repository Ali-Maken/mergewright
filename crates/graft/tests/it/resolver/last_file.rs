//! The last file, and the other refusals (m6-conflicts/04).

use super::*;

#[test]
fn last_file_closes_to_where_it_opened() {
    last_file_with_a_banner();
    last_file_without_one();
}

/// Opened from the banner, walked through both files, closed back onto it.
fn last_file_with_a_banner() {
    let scratch = two_file_merge();
    let (app, state) = opened_on(&scratch);
    let state = app.pump(state, "the merge banner", |state| {
        state.in_progress.is_some()
    });
    let state = on_the_banner(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = settled(&app, state);
    assert_eq!(open(&state).path, Path::new("a.txt"));

    let state = take_all_and_mark(&app, state);
    assert_eq!(open(&state).path, Path::new("b.txt"), "on to the second");
    let state = take_all_and_mark(&app, state);

    assert!(
        state.resolver.is_none(),
        "the last file closes the resolver"
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "every file done ✓ · c finishes the merge"),
        "{:?}",
        state.status
    );
    assert!(state.banner_focus, "back on the banner it opened from");
    assert_eq!(still_unmerged(scratch.path()), "", "and git agrees");
    drop(state);
    app.close();
    drop(scratch);
}

/// A conflicted stash pop, opened from the pane: no banner, so no `c`.
fn last_file_without_one() {
    let scratch = stash_pop_conflict();
    let (app, state) = opened_on(&scratch);
    assert!(state.in_progress.is_none(), "a stash pop is no operation");
    let state = resolver_on(&app, state, "sample.txt");
    let state = take_all_and_mark(&app, state);

    assert!(
        state.resolver.is_none(),
        "the only file closes the resolver"
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "every file done ✓"),
        "no banner, so no `c continue`: {:?}",
        state.status
    );
    assert!(!state.banner_focus, "there is no banner to hand back to");
    assert_eq!(still_unmerged(scratch.path()), "", "and git agrees");
    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn resolving_the_last_listed_file_first_goes_back_for_the_rest() {
    // `b.txt` is the last CONFLICTS row, but not the last conflict:
    // "all conflicts resolved" here would send the user to `c` with
    // `a.txt` still unmerged.
    let scratch = two_file_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "b.txt");
    let state = take_all_and_mark(&app, state);

    let Some(open) = state.resolver.as_ref() else {
        panic!(
            "the resolver closed with a.txt still unmerged: {:?}",
            state.status
        )
    };
    assert_eq!(open.path, Path::new("a.txt"), "back to the one left");
    assert_eq!(still_unmerged(scratch.path()).trim(), "a.txt");

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn open_refused_while_a_box_is_open() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = on_the_banner(&app, state);
    // `a` asks "Abort the merge?" — a real box, raised by the real key.
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(
        state.confirm.is_some(),
        "`a` raised no box: {:?}",
        state.status
    );

    // `⏎` now answers the box, so no key reaches `open_resolver` while it
    // is up; the guard is for a caller that does — which is what this is.
    let mut next = state.clone();
    let mut commands = Vec::new();
    let mut raw = 9_001;
    let spent = graft::update::resolver::open_resolver(
        &mut next,
        &mut commands,
        "both.txt".into(),
        OpenedFrom::Banner,
        &mut || {
            raw += 1;
            graft_core::job::JobId::new(raw)
        },
    );
    assert!(spent, "the refusal is still an answer");
    assert!(commands.is_empty(), "and it queued nothing");
    assert!(next.resolver.is_none(), "no resolver over the box");
    assert!(next.confirm.is_some(), "the box keeps its question");
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason == "finish the box first"),
        "{:?}",
        next.status
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn open_on_a_path_git_no_longer_calls_unmerged_shows_it_done() {
    // The list on screen says `added-by-them.txt` is conflicted; git, by
    // the time the read runs, does not — it was staged behind the app's
    // back. The job reads the status itself and routes the path to the done
    // side (m6-conflicts/25 D2): the resolver opens on it read-only, where
    // before this ticket `Refusal::NotUnmerged` closed it with
    // `… is not conflicted`. That refusal now answers only the race between
    // the job's status read and its open.
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    crate::app_harness::git(scratch.path(), &["add", "added-by-them.txt"]);
    let state = on_the_banner(&app, state);

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    assert!(
        matches!(commands.as_slice(), [Command::OpenConflict(_, path), Command::ReadConflictSet(_)] if path == Path::new("added-by-them.txt")),
        "the stale list still names it: {commands:?}"
    );
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the done read", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.job.is_none())
    });

    let open = state.resolver.as_ref().expect("the resolver stays open");
    assert_eq!(open.path, Path::new("added-by-them.txt"));
    assert!(open.done.is_some(), "{:?}", open.done);
    let frame = crate::captures::frame(&state, 120, 40);
    assert!(
        frame
            .iter()
            .any(|row| row.starts_with("▸ added-by-them.txt · done ✓ · d starts it over")),
        "{}",
        frame.join("\n")
    );

    drop(state);
    app.close();
    drop(scratch);
}
