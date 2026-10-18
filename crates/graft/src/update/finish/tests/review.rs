//! The merge review's door from the banner (ease-pass/05b): `c` asks the
//! read, which judges the files on disk. Moved from `finish/tests.rs`
//! (ease-pass/05s), same names.

use super::*;

#[test]
fn c_on_a_resolved_merge_opens_the_summary_not_the_commit() {
    let state = stopped(InProgress::Merge, &[]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(
        resumes(&commands).is_empty(),
        "nothing written: {commands:?}"
    );
    assert!(state.operating.is_none());
    let open = finish(&state);
    assert!(
        commands.iter().any(
            |command| matches!(command, Command::ReadFinishSummary(id) if Some(*id) == open.job)
        ),
        "the summary is read under the id it waits on: {commands:?}"
    );
    assert_eq!(open.focus, FinishFocus::Files);
    assert_eq!(
        state.graph_overlay(),
        Some(crate::update::GraphOverlay::Finish)
    );
}

fn read_id(commands: &[Command]) -> JobId {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadFinishSummary(id) => Some(*id),
            _ => None,
        })
        .expect("the review is read")
}

/// Banner `c` in a merge with a file still unmerged (ease-pass/05b): the
/// files git still calls unmerged may be solved on disk, which only the
/// review's read can tell, so `c` asks it; a read that finds a clash still
/// open closes the review and refuses in today's words, counting the open
/// files. Nothing is written.
#[test]
fn c_with_a_file_still_open_is_refused_by_the_review_read() {
    let state = stopped(InProgress::Merge, &["notes.txt"]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(resumes(&commands).is_empty());
    let id = read_id(&commands);
    let (state, commands) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::FinishSummary(Err(ReviewFailed::StillOpen {
                files: 1,
                clashes: 1,
            })),
        ),
    );
    assert!(state.finish.is_none());
    assert!(state.banner_focus, "the banner has the keyboard back");
    assert!(resumes(&commands).is_empty());
    assert_eq!(
        state.status,
        crate::update::banner::continue_refused(
            "1 file still conflicted".to_owned(),
            crate::update::banner::refused_at(&state),
            &state.keymap,
        )
    );
}

/// Banner `c` in a merge whose unmerged file is solved on disk: the review
/// opens, and its `y` hands the solved files to the finish, which stages
/// them in the same operation (ease-pass/05b). Nothing is staged before.
#[test]
fn y_in_the_review_stages_the_files_the_read_found_solved() {
    let state = stopped(InProgress::Merge, &["notes.txt"]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::Operate(..) | Command::Resume(..))),
        "nothing staged or written on c: {commands:?}"
    );
    let id = read_id(&commands);
    let reviewed = Reviewed {
        summary: summary(),
        solved: vec![PathBuf::from("notes.txt")],
    };
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::FinishSummary(Ok(reviewed))),
    );
    assert_eq!(finish(&state).solved, vec![PathBuf::from("notes.txt")]);
    let (_, commands) = press(&state, plain(KeyCode::Char('y')));
    assert_eq!(
        resumes(&commands),
        vec![&Resume::Continue {
            solved: vec![PathBuf::from("notes.txt")]
        }]
    );
}
