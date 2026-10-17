//! What the summary's rows say you chose (m6-conflicts/35), and when
//! those choices and the draft are forgotten.

use super::*;

// ------------------------------------------------ what the rows say you chose

/// `gone_notes_and_readme_merge` stopped, the app open on it.
pub(super) fn stopped_on_gone_and_notes() -> (graft_fixtures::Scratch, App, AppState) {
    let scratch = crate::resolver::gone_notes_and_readme_merge();
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    (scratch, app, state)
}

/// The resolver, landed on `path` after the file before it was done, with
/// nothing in flight.
pub(super) fn advanced_to(app: &App, state: AppState, path: &str) -> AppState {
    app.pump(state, "the next file", |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new(path)
                    && open.job.is_none()
                    && (open.file.is_some() || open.whole_file.is_some())
            })
    })
}

/// `o` on `gone.txt`'s card, the last file: the resolver stays on it, read
/// back as done (m6-conflicts/31), and the status the stage asked for has
/// landed.
pub(super) fn gone_kept_last(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('o'));
    app.pump(state, "the last file done", |state| {
        state.operating.is_none() && state.status_read.is_none() && last_file_read_back(state)
    })
}

/// The resolver is on a done file with its read landed — where every file
/// done leaves it (m6-conflicts/31).
fn last_file_read_back(state: &AppState) -> bool {
    state
        .resolver
        .as_ref()
        .is_some_and(|open| open.done.is_some() && open.job.is_none())
}

/// `c` where the user is — the resolver after the last file
/// (m6-conflicts/31), else the banner — and the summary read.
pub(super) fn c_for_the_summary(app: &App, state: AppState) -> AppState {
    let state = if state.resolver.is_some() {
        state
    } else {
        crate::resolver::on_the_banner(app, state)
    };
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(
        state.finish.is_some(),
        "`c` opens the summary: {:?}",
        state.status
    );
    app.pump(state, "the finish summary", |state| {
        state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
}

/// The drawn line naming `path` under `RESOLVED BY YOU`, at 120×40.
fn row_for(state: &AppState, path: &str) -> String {
    let drawn = crate::captures::frame(state, 120, 40);
    let from = drawn
        .iter()
        .position(|line| line.contains("RESOLVED BY YOU"))
        .unwrap_or_else(|| panic!("no RESOLVED BY YOU section:\n{}", drawn.join("\n")));
    drawn
        .iter()
        .skip(from)
        .find(|line| line.contains(path))
        .cloned()
        .unwrap_or_else(|| panic!("no row for {path}:\n{}", drawn.join("\n")))
}

/// `T` on `notes.txt`, then `o` on `gone.txt`, the last file: both resolved
/// in the app, back on the banner.
fn both_resolved_in_the_app(app: &App, state: AppState) -> AppState {
    let state = crate::resolver::resolver_on(app, state, "notes.txt");
    let state = app.press(&state, KeyPress::plain(KeyCode::Char('T')));
    let state = advanced_to(app, state, "gone.txt");
    gone_kept_last(app, &state)
}

#[test]
fn the_summary_words_what_you_chose() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let repo = scratch.path().to_owned();
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let state = app.press(&state, KeyPress::control(KeyCode::Char('t')));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = advanced_to(&app, state, "gone.txt");
    let state = gone_kept_last(&app, &state);
    let state = c_for_the_summary(&app, state);

    let notes = row_for(&state, "notes.txt");
    assert!(
        notes.contains("theirs · feature in 2 of 2 conflicts"),
        "{notes}"
    );
    let gone = row_for(&state, "gone.txt");
    assert!(gone.contains("kept · yours · main"), "{gone}");
    drop(state);
    app.close();

    // A relaunch remembers no choices: the rows say what the result is.
    let (app, state) = App::open(&repo);
    let state = app.pump(state, "the resolved merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    assert!(state.choices.is_empty(), "nothing carried across");
    let state = c_for_the_summary(&app, state);
    let notes = row_for(&state, "notes.txt");
    assert!(notes.contains("same as theirs · feature"), "{notes}");
    let gone = row_for(&state, "gone.txt");
    assert!(gone.contains("same as yours · main"), "{gone}");
    drop(state);
    app.close();
}

#[test]
fn shift_t_records_its_choices() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let state = both_resolved_in_the_app(&app, state);
    let state = c_for_the_summary(&app, state);

    let notes = row_for(&state, "notes.txt");
    assert!(
        notes.contains("theirs · feature in 2 of 2 conflicts"),
        "{notes}"
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// Review m6-amend-19-34 Major 3: a file that kept yours is staged but equal
/// to `HEAD`, so `space` on its row must say so plainly, not warn
/// `<path> is not staged`.
#[test]
fn space_on_a_kept_yours_row_warns_nothing() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let state = both_resolved_in_the_app(&app, state);
    let state = c_for_the_summary(&app, state);
    let row = match &state.finish.as_ref().expect("the summary").summary {
        graft::update::detail::Loading::Ready(summary) => summary
            .by_you
            .iter()
            .position(|file| file.path == Path::new("gone.txt"))
            .expect("gone.txt is resolved by you"),
        other => panic!("the summary is not read: {other:?}"),
    };
    // The cursor opens wherever the summary puts it: go to the top first.
    let mut state = state;
    for _ in 0..4 {
        state = app.key(&state, KeyCode::Char('k'));
    }
    for _ in 0..row {
        state = app.key(&state, KeyCode::Char('j'));
    }
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the diff read", |state| {
        state.finish.as_ref().is_some_and(|open| {
            open.diff_job.is_none()
                || !matches!(
                    open.expanded,
                    Some((_, graft::update::detail::Loading::Pending(_)))
                )
        })
    });
    let drawn = crate::captures::frame(&state, 120, 40).join("\n");
    assert!(!drawn.contains("not staged"), "{drawn}");
    assert!(drawn.contains("no change to show"), "{drawn}");
    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn a_start_over_forgets_the_choices() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let state = app.key(&state, KeyCode::Char('t'));
    let state = crate::resolver::settled(&app, state);
    assert!(
        state.choices.contains_key(Path::new("notes.txt")),
        "the take was recorded: {:?}",
        state.choices
    );

    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_some(), "`d` asks first");
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the restart", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none() && open.file.is_some())
    });
    assert_eq!(
        crate::resolver::open(&state)
            .file
            .as_ref()
            .map(|file| file.open()),
        Some(2),
        "the file is conflicted afresh"
    );
    assert!(
        !state.choices.contains_key(Path::new("notes.txt")),
        "no choices for a file started over: {:?}",
        state.choices
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// The app's own merge of [`about_to_merge`], landed in the resolver, and
/// `T` on each file until every one is done — the resolver stays on the
/// last (m6-conflicts/31): `choices` holds both. A merge begun in the shell
/// is not the app's to commit or abort.
/// The open document has no clash left open.
fn solved(open: &graft::update::resolver::ResolverState) -> bool {
    open.file.as_ref().is_some_and(|file| file.open() == 0)
}

fn every_file_theirs(app: &App, state: AppState) -> AppState {
    let mut state = crate::conflict_interim::landed_in_the_resolver(app, state);
    let mut guard = 0;
    // A merge's `T` on a document takes every open block and stages
    // nothing (ease-pass/05b): the file is solved, not done. On a card it
    // stages, as a whole-file choice always has.
    while let Some(was) = state
        .resolver
        .as_ref()
        .filter(|open| open.done.is_none() && !solved(open))
        .map(|open| open.path.clone())
    {
        assert!(guard < 4, "`T` never solved every file");
        guard += 1;
        state = app.press(&state, KeyPress::plain(KeyCode::Char('T')));
        state = app.pump(state, "the file solved and the next", |state| {
            state.operating.is_none()
                && state.resolver.as_ref().is_none_or(|open| {
                    open.job.is_none()
                        && open.queued.is_empty()
                        && if open.path == was {
                            open.done.is_some() || solved(open)
                        } else {
                            open.file.is_some() || open.whole_file.is_some()
                        }
                })
        });
    }
    app.pump(state, "the status the last stage asked for", |state| {
        state.status_read.is_none()
    })
}

/// [`every_file_theirs`], then the summary opened, a subject typed, `Esc`:
/// the draft is kept.
fn choices_and_a_draft(app: &App, state: AppState) -> AppState {
    let state = every_file_theirs(app, state);
    let state = c_for_the_summary(app, state);
    let state = app.key(&state, KeyCode::Tab);
    let state = typed(app, state, " (draft)");
    let state = app.key(&state, KeyCode::Escape);
    assert_eq!(state.choices.len(), 2, "{:?}", state.choices);
    assert!(state.finish_draft.is_some(), "the draft is kept");
    state
}

#[test]
fn choices_and_draft_clear_when_the_merge_ends() {
    // Done: the merge commits.
    let scratch = about_to_merge();
    let (app, state) = App::open(scratch.path());
    let state = choices_and_a_draft(&app, state);
    let state = c_for_the_summary(&app, state);
    let state = committed(&app, &state, KeyPress::plain(KeyCode::Char('y')));
    assert!(
        !scratch.path().join(".git/MERGE_HEAD").exists(),
        "the merge committed: {:?}",
        state.finish.as_ref().map(|open| &open.notice)
    );
    assert!(state.choices.is_empty(), "after Done: {:?}", state.choices);
    assert_eq!(state.finish_draft, None, "after Done");
    drop(state);
    app.close();
    drop(scratch);

    // Abort: the merge is thrown away.
    let scratch = about_to_merge();
    let (app, state) = App::open(scratch.path());
    let state = choices_and_a_draft(&app, state);
    // `Esc` from the summary went back to the resolver; its own `Esc`
    // lands on the banner, where `a` aborts.
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none() && state.banner_focus);
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(state.confirm.is_some(), "`a` asks first");
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the abort", |state| state.operating.is_none());
    assert!(
        !scratch.path().join(".git/MERGE_HEAD").exists(),
        "the merge is over"
    );
    assert!(
        state.choices.is_empty(),
        "after an abort: {:?}",
        state.choices
    );
    assert_eq!(state.finish_draft, None, "after an abort");
    drop(state);
    app.close();
    drop(scratch);
}

/// Review m6-amend-35 Major 1: a merge finished outside the app (`git
/// commit` in the shell) ends the operation too. The next read that finds
/// no merge must drop its choices and draft, or the next merge's summary
/// words old choices and an unedited `⏎` commits the old draft.
#[test]
fn choices_and_draft_clear_when_the_merge_ends_outside() {
    let scratch = about_to_merge();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = choices_and_a_draft(&app, state);
    // The solved files are still unmerged until the review's `y`
    // (ease-pass/05b): finishing in the shell stages them first, as git asks.
    git(repo, &["add", "-u"]);
    git(repo, &["commit", "-q", "--no-edit"]);
    // The resolver the summary went back to is modal: `Esc` closes it.
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none());
    // The focused banner swallows `Ctrl+R` (`update/keys.rs::banner_key`);
    // `Tab` gives the keyboard back first.
    let state = if state.banner_focus {
        app.key(&state, KeyCode::Tab)
    } else {
        state
    };
    let state = app.press(&state, KeyPress::control(KeyCode::Char('r')));
    let state = app.pump(state, "the read that finds no merge", |state| {
        state.in_progress.is_none() && state.status_read.is_none()
    });
    assert!(
        state.choices.is_empty(),
        "after the shell's commit: {:?}",
        state.choices
    );
    assert_eq!(state.finish_draft, None, "after the shell's commit");
    drop(state);
    app.close();
    drop(scratch);
}

fn edited(state: &AppState, path: &str) -> bool {
    state
        .choices
        .get(Path::new(path))
        .is_some_and(|choices| choices.edited)
}

fn handed_back(state: &AppState, result: Result<(), String>) -> AppState {
    let (state, _rereads) = update(
        state,
        AppEvent::Produced(graft_core::job::JobId::new(0), Product::HandedOver(result)),
    );
    state
}

/// The terminal handovers that mark a file `edited in your editor`: a
/// clean return from `$EDITOR` (the changes pane's `e`, the resolver's `e`)
/// or the mergetool. The harness cannot hand the terminal over, so each
/// command is answered the way the loop answers it (`event_loop::running`).
#[test]
fn a_clean_handover_marks_the_file_edited() {
    let (scratch, app, state) = stopped_on_gone_and_notes();

    // The changes pane's `e` on `notes.txt`: a failed handover marks nothing.
    let state = crate::resolver::on_the_row(&app, state, "notes.txt");
    let (state, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflictEditor { .. })),
        "{commands:?}"
    );
    let state = handed_back(&state, Err("vi exited 1, message unchanged".to_owned()));
    assert!(!edited(&state, "notes.txt"), "a failed handover");
    assert_eq!(state.conflict_editing, None, "the path is spent");
    let (state, _) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    let state = handed_back(&state, Ok(()));
    assert!(edited(&state, "notes.txt"), "{:?}", state.choices);

    // The mergetool on `gone.txt`.
    let (state, _rereads) = update(
        &state,
        AppEvent::Produced(
            graft_core::job::JobId::new(0),
            Product::Mergetool {
                path: "gone.txt".into(),
                result: Ok(()),
            },
        ),
    );
    assert!(edited(&state, "gone.txt"), "{:?}", state.choices);
    drop(state);
    app.close();

    // The resolver's own `e`, in a fresh session.
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let (state, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenExternalEditor { .. })),
        "{commands:?}"
    );
    let state = handed_back(&state, Ok(()));
    assert!(edited(&state, "notes.txt"), "{:?}", state.choices);
    drop(state);
    app.close();
    drop(scratch);
}
