//! `e` and `M`: handing the file to another program, and reading it back (m6-conflicts/04).

use super::*;

// ------------------------------------------------- handing the file over

#[test]
fn e_hands_over_and_rereads() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let mut state = resolver_on(&app, state, "three.txt");
    // A block past the first, so the line `e` opens at is the cursor's and
    // not the file's first. No key walks down past an open block since
    // ease-pass 05 (`n` is the first open clash), so it is put there.
    state.resolver.as_mut().expect("open").cursor_block = Some(1);
    let block = open(&state).cursor_block.expect("a block under the cursor");

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    let [Command::OpenExternalEditor { path, line }] = commands.as_slice() else {
        panic!("`e` queued {commands:?}")
    };
    assert_eq!(path, Path::new("three.txt"));
    let want = open(&state)
        .file
        .as_ref()
        .expect("a document")
        .blocks()
        .get(block)
        .expect("the cursor's block")
        .first_line;
    assert_eq!(
        *line,
        Some(u32::try_from(want).expect("a line number")),
        "`e` opens at the cursor block's own marker line"
    );

    // Back from the editor: the document on screen is not what is on disk
    // any more, whoever wrote it, so it is read again.
    let (_, commands) = mergewright::update::update(
        &state,
        mergewright::update::AppEvent::Produced(
            mergewright_core::job::JobId::new(9_100),
            mergewright::update::Product::HandedOver(Ok(())),
        ),
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(_, path) if path == Path::new("three.txt"))),
        "the hand-back queued no re-read: {commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "and the CONFLICTS list behind it is as stale as the document was"
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn resolved_outside_closes() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");

    // What `$EDITOR` or a mergetool would have done while the resolver sat
    // there: the markers gone and the path staged, so git no longer calls
    // it unmerged.
    std::fs::write(
        repo.join("both.txt"),
        "same on both sides\nsomething else entirely\nand a tail\n",
    )
    .expect("writing the file");
    crate::app_harness::git(&repo, &["add", "both.txt"]);

    // Any take now finds the file changed under it, throws the document
    // away and reads it again — and the read comes back "not conflicted".
    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the re-read after the outside edit", |state| {
        state.resolver.is_none()
    });
    assert!(
        matches!(&state.status, Status::Done { what: line, .. } if line == "both.txt resolved outside"),
        "{:?}",
        state.status
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
#[allow(
    non_snake_case,
    reason = "the key is `M`, and the test is named for it"
)]
fn M_hands_over() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "both.txt");

    // No `merge.tool` in the fixture: the banner's own refusal, reused.
    let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('M')));
    assert!(commands.is_empty(), "a refusal hands nothing over");
    let Status::Sticky { reason, .. } = &after.status else {
        panic!("`M` with no tool said {:?}", after.status)
    };
    assert!(reason.contains("no merge.tool configured"), "{reason:?}");

    let mut configured = state.clone();
    configured.mergetool = Some("vimdiff".into());
    let (_, commands) = press_for_commands(&configured, KeyPress::plain(KeyCode::Char('M')));
    assert!(
        matches!(commands.as_slice(), [Command::OpenMergetool { path }] if path == Path::new("both.txt")),
        "{commands:?}"
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn changed_on_disk_rereads_once() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");

    // Something else writes the file between the read and the take.
    std::fs::write(
        repo.join("both.txt"),
        "same on both sides\n<<<<<<< HEAD\nrewritten\n=======\nby someone else\n>>>>>>> feature\nand a tail\n",
    )
    .expect("writing the file");

    let state = app.key(&state, KeyCode::Char('o'));
    // The keypress itself leaves a notice (`all blocks taken`, the one
    // block taken); the refusal's is the one marked a problem.
    let state = app.pump(state, "the refused take", |state| {
        state
            .resolver
            .as_ref()
            .and_then(|open| open.notice.as_ref())
            .is_some_and(|(_, problem)| *problem)
    });
    let (words, problem) = open(&state).notice.clone().expect("a notice");
    assert!(words.contains("changed"), "{words:?}");
    assert!(problem, "a take that did not happen is a problem");
    assert!(
        open(&state).job.is_some(),
        "and the document is read again rather than written over"
    );

    let state = settled(&app, state);
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        1,
        "the re-read shows what is on disk now"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// Find-and-pin 1 of m6-conflicts/61, pinned **as observed** (2026-09-25):
/// what a block took both ways reads as after the user edits the file in
/// `$EDITOR` and the resolver reads it back.
///
/// The spec expected the block to stay `Took(Both)` with `edited` set. It
/// does not: a take is written to disk with its markers gone, and the
/// re-read parses markers only (`update/resolver.rs:11-15`, "re-reading
/// collapses every block already taken"), so the document comes back with
/// **no blocks at all**. `edited` is set, and the recorded take survives in
/// `choices`. Ticket 61 stopped here: the hand-edit tint rule has no
/// `Took(Both)` block to draw after a re-read, and needs a spec decision.
#[test]
fn a_hand_edited_both_take_reads_as_no_block_after_the_reread() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Char('b'));
    let state = settled(&app, state);
    assert_eq!(
        block_states(&state),
        [mergewright_git::conflict::BlockState::Took(
            mergewright_git::conflict::Take::Both
        )],
        "before the edit: the one block took both"
    );
    let state = edited_in_the_editor(&app, &state, &repo);

    // The block's state: gone. The document has no blocks.
    let states = block_states(&state);
    assert!(states.is_empty(), "{states:?}");
    // `edited`: set, and the take recorded before the edit is kept.
    let choices = state
        .choices
        .get(Path::new("both.txt"))
        .expect("a choice recorded for both.txt");
    assert!(choices.edited, "{choices:?}");
    assert_eq!(
        choices.takes,
        [Some(mergewright_git::conflict::Take::Both)],
        "{choices:?}"
    );
    // What the view draws: the empty-file notice, and no block rows.
    assert_eq!(
        open(&state).notice,
        Some((
            "no conflict markers · ⏎ marks it resolved".to_owned(),
            false
        ))
    );
    let frame = crate::captures::frame_at(&state, 120, 40, None);
    let body = frame.join("\n");
    assert!(
        body.contains("no conflict markers · ⏎ marks it resolved"),
        "{body}"
    );
    assert!(
        body.contains("and edited"),
        "the edited text is on screen: {body}"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// Every block's state in the open document.
fn block_states(state: &AppState) -> Vec<mergewright_git::conflict::BlockState> {
    open(state)
        .file
        .as_ref()
        .expect("a document")
        .blocks()
        .iter()
        .map(|block| block.state)
        .collect()
}

/// What `$EDITOR` does to `both.txt` (a line of the result changed by
/// hand), then `e`'s answer fed back the way the loop feeds it, and the
/// re-read it queues run for real to its landing.
fn edited_in_the_editor(app: &App, state: &AppState, repo: &Path) -> AppState {
    std::fs::write(
        repo.join("both.txt"),
        "same on both sides\nthe line, as we changed it\nthe line, as they changed it, and edited\nand a tail\n",
    )
    .expect("writing the file");
    let (_, commands) = press_for_commands(state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        matches!(commands.as_slice(), [Command::OpenExternalEditor { .. }]),
        "{commands:?}"
    );
    let (state, commands) = mergewright::update::update(
        state,
        mergewright::update::AppEvent::Produced(
            mergewright_core::job::JobId::new(9_300),
            mergewright::update::Product::HandedOver(Ok(())),
        ),
    );
    for command in commands {
        app.jobs.dispatch(command);
    }
    app.pump(state, "the re-read", |state| {
        state
            .resolver
            .as_ref()
            .is_none_or(|open| open.job.is_none())
    })
}
