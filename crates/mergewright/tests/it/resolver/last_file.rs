//! The last file, and the other refusals (m6-conflicts/04).

use super::*;

#[test]
fn last_file_closes_to_where_it_opened() {
    last_file_with_a_banner();
    last_file_without_one();
}

/// Opened from the banner, walked through both files of a merge: nothing
/// is staged (ease-pass/05b), the resolver stays on the last file, its
/// notice offering `y`, `y` opens the review over it, `esc` goes back, and `c` opens
/// the same review (m6-conflicts/60's summary no longer opens on its own).
fn last_file_with_a_banner() {
    let scratch = two_file_merge();
    let (app, state) = opened_on(&scratch);
    let state = both_files_taken(&app, state);
    assert_eq!(open(&state).path, Path::new("b.txt"), "stays on the last");
    assert_eq!(
        still_unmerged(scratch.path()).trim(),
        "a.txt\nb.txt",
        "a merge stages nothing before the review"
    );
    let state = captured_both_moments(&app, &state);

    // `c` opens the same review, the resolver under it.
    let state = app.key(&state, KeyCode::Char('c'));
    let state = review_landed(&app, state);
    assert!(state.resolver.is_some(), "the resolver stays under it");
    drop(state);
    app.close();
    drop(scratch);
}

/// The review's read landed over the resolver.
fn review_landed(app: &App, state: AppState) -> AppState {
    let state = app.pump(state, "the review's read", |state| {
        state
            .finish
            .as_ref()
            .is_some_and(|finish| finish.job.is_none())
    });
    let finish = state.finish.as_ref().expect("the review");
    assert_eq!(finish.return_to, mergewright::update::finish::ReturnTo::Resolver);
    state
}

/// From launch: the resolver opened from the banner, `a.txt` done, every
/// block of `b.txt` taken, and `⏎` pressed on it — its `Stage` launched (a
/// cherry-pick; a merge stages at its review, ease-pass/05b).
fn last_file_marked(app: &App, state: AppState) -> AppState {
    let state = both_files_taken(app, state);
    app.key(&state, KeyCode::Enter)
}

/// From launch: the resolver opened from the banner, `a.txt` taken and
/// marked, every block of `b.txt` taken.
fn both_files_taken(app: &App, state: AppState) -> AppState {
    let state = app.pump(state, "the merge banner", |state| {
        state.in_progress.is_some()
    });
    let state = on_the_banner(app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = settled(app, state);
    assert_eq!(open(&state).path, Path::new("a.txt"));

    let state = take_all_and_mark(app, state);
    assert_eq!(open(&state).path, Path::new("b.txt"), "on to the second");
    let mut state = state;
    while open(&state).file.as_ref().expect("a document").open() > 0 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(app, state);
    }
    state
}

/// Premise 1: the stage has landed, the done read has not.
fn stage_landed(app: &App, state: AppState) -> AppState {
    let between = app.pump(state, "the last stage", |state| state.operating.is_none());
    let open_now = open(&between);
    assert_eq!(open_now.path, Path::new("b.txt"), "the same file, reopened");
    assert!(open_now.job.is_some(), "its read is out");
    assert!(open_now.file.is_none() && open_now.done.is_none());
    assert_eq!(
        open_now.notice,
        Some(("checking the files".to_owned(), false)),
        "the status the stage asked for is still out"
    );
    // Observed (2026-09-24): nothing under the header while the read is out
    // — `view::resolver::resolver_view` answers `None` for any resolver
    // waiting on its opening read, the next-file reopen's included. The
    // notice is held, not drawn, until the read lands.
    let frame = crate::captures::frame(&between, 120, 40);
    assert!(
        frame[1..].iter().all(|row| row.trim().is_empty()),
        "what shows while the read is out:\n{}",
        frame.join("\n")
    );
    between
}

/// Two moments, captured: the review `y` opened over the solved merge,
/// and after `Esc` from it — the resolver on the last file, its notice
/// offering `y` again.
fn captured_both_moments(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('y'));
    assert!(state.operating.is_none(), "y wrote: {:?}", state.status);
    let state = review_landed(app, state);
    let landed = crate::captures::frame(&state, 120, 40);
    assert!(
        landed
            .iter()
            .any(|row| row.contains("Nothing committed yet.")),
        "{}",
        landed.join("\n")
    );
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.finish.is_none(), "Esc closed the review");
    assert_eq!(
        state.graph_overlay(),
        Some(GraphOverlay::Resolver),
        "the resolver has the keyboard again"
    );
    let after_esc = crate::captures::frame(&state, 120, 40);
    let body = after_esc.join("\n");
    assert!(
        after_esc
            .iter()
            .any(|line| line.contains("· y reviews the merge")),
        "{body}"
    );
    let shot = format!(
        "{}\n--- after Esc ---\n{}",
        landed.join("\n"),
        after_esc.join("\n")
    );
    crate::captures::check_shot("m6/fix-finish-from-the-resolver-120x40.txt", &shot);
    state
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
        matches!(&state.status, Status::Done { what: line, also: None, keys: None } if line == "every file done"),
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
    assert_eq!(
        still_unmerged(scratch.path()).trim(),
        "a.txt\nb.txt",
        "a merge stages nothing before the review (ease-pass/05b)"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// **The footer draws one tick** (ticket 41). The last file staged from
/// the resolver, the resolver closed back to the banner, and the banner's
/// `c` pressed while the status that stage asked for is still out: the
/// line says it is checking, and the read landing settles it to
/// `✓ every file done  c finishes the merge`. Read from the drawn footer,
/// not from `Status`: the renderer owns the tick, so only the frame shows
/// where it lands.
#[test]
fn the_last_file_done_draws_one_tick_on_the_footer() {
    // A cherry-pick: a merge stages nothing before its review (ease-pass/05b).
    let scratch = two_file_cherry_pick();
    let (app, state) = opened_on(&scratch);
    let state = last_file_marked(&app, state);
    let between = stage_landed(&app, state);
    let state = app.key(&between, KeyCode::Escape);
    assert!(state.resolver.is_none(), "Esc closed the resolver");
    assert!(state.banner_focus, "back on the banner");
    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.pump(state, "the status the stage asked for", |state| {
        state.status_read.is_none()
    });
    assert!(state.banner_focus, "still on the banner");
    assert_eq!(still_unmerged(scratch.path()), "", "and git agrees");

    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    let footer = frame
        .iter()
        .find(|row| row.contains("every file done"))
        .unwrap_or_else(|| panic!("no row names it:\n{body}"));
    assert!(
        footer.contains("✓ every file done  c continues the cherry-pick"),
        "{footer:?}"
    );
    assert_eq!(footer.matches('✓').count(), 1, "one tick: {footer:?}");
    assert_eq!(
        footer.trim_end(),
        " ✓ every file done  c continues the cherry-pick",
        "nothing after `cherry-pick`"
    );
    // Trimmed to the two rows the check is about: the header with the
    // banner's `MERGING`, and the footer. The graph rows between them are
    // commits the fixture makes now, so their ids, dates and the order of
    // two made in the same second change every run.
    assert!(frame[0].trim_end().ends_with("CHERRY-PICKING"), "{body}");
    let shot = format!("{}\n⋮\n{}\n", frame[0].trim_end(), footer.trim_end());
    crate::captures::check_shot("m6/fix-every-file-done-footer-120x40.txt", &shot);
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
    let spent = mergewright::update::resolver::open_resolver(
        &mut next,
        &mut commands,
        "both.txt".into(),
        OpenedFrom::Banner,
        &mut || {
            raw += 1;
            mergewright_core::job::JobId::new(raw)
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
    // Begun in the shell, so nothing kept the stages: no `d` offered
    // (m6-conflicts/28).
    let frame = crate::captures::frame(&state, 120, 40);
    assert!(
        crate::captures::doc_rows(&frame)
            .iter()
            .any(|row| row.trim_end() == "▸ added-by-them.txt · done ✓"),
        "{}",
        frame.join("\n")
    );

    drop(state);
    app.close();
    drop(scratch);
}
