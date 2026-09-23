//! `e` and `M`: handing the file to another program, and reading it back (m6-conflicts/04).

use super::*;

// ------------------------------------------------- handing the file over

#[test]
fn e_hands_over_and_rereads() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    let state = app.key(&state, KeyCode::Char('n'));
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
    let (_, commands) = graft::update::update(
        &state,
        graft::update::AppEvent::Produced(
            graft_core::job::JobId::new(9_100),
            graft::update::Product::HandedOver(Ok(())),
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
            .any(|command| matches!(command, Command::ReadWorkingTree)),
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
        matches!(&state.status, Status::Done(line) if line == "both.txt resolved outside"),
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
    configured.mergetool = Some("vimdiff".to_owned());
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
    let state = app.pump(state, "the refused take", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.notice.is_some())
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
