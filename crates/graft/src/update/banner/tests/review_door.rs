//! Banner `c` in a merge: the review's door (ease-pass/05b, 05s). The read
//! judges the files on disk; the review opens only when it lands, and a
//! clash still open opens none. The first test moved from `banner/tests.rs`
//! (ease-pass/05s), same name.

use std::path::PathBuf;

use graft_ops::finish::FinishSummary;

use super::*;
use crate::update::finish::{ReturnTo, ReviewFailed, Reviewed};

/// What the review's read finds: `notes.txt` solved on disk.
fn reviewed() -> Reviewed {
    Reviewed {
        summary: std::sync::Arc::new(FinishSummary {
            into: b"main".to_vec(),
            from: b"feature".to_vec(),
            by_you: Vec::new(),
            by_git: Vec::new(),
            message: b"Merge branch 'feature'\n".to_vec(),
            recorded: true,
        }),
        solved: vec![PathBuf::from("notes.txt")],
    }
}

/// The review's one read in `commands`, answered `answer`.
fn lands(
    state: &AppState,
    commands: &[Command],
    answer: Result<Reviewed, ReviewFailed>,
) -> AppState {
    let reads: Vec<JobId> = commands
        .iter()
        .filter_map(|command| match command {
            Command::ReadFinishSummary(id) => Some(*id),
            _ => None,
        })
        .collect();
    let [id] = reads.as_slice() else {
        panic!("one review read: {commands:?}");
    };
    crate::update::update(
        state,
        AppEvent::Produced(*id, Product::FinishSummary(answer)),
    )
    .0
}

/// `c` on a resolved merge **begun in the shell** (no live entry of Graft's)
/// opens the finish summary like any merge, once its read lands — no
/// shell check here (m6-20 B1).
/// The `⏎` that follows sends `Resume::Continue`, which `graft_ops` routes
/// to `Operation::FinishMerge` since ticket 20 Part A.
#[test]
fn c_on_a_shell_begun_merge_opens_the_summary() {
    let mut state = stopped(InProgress::Merge, &[]);
    assert!(state.live_entries.is_empty(), "begun in the shell");
    assert_eq!(
        crate::update::commit_editor::live_operation(&state),
        Some((
            InProgress::Merge,
            crate::update::commit_editor::Begun::InShell
        ))
    );
    let (claimed, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(claimed);
    let state = lands(&state, &commands, Ok(reviewed()));
    let finish = state.finish.as_ref().expect("the summary is open");
    assert_eq!(finish.return_to, crate::update::finish::ReturnTo::Banner);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Resume(..))),
        "no Resume before ⏎: {commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "the summary is read: {commands:?}"
    );
}

/// A clash still open in a file the read found: no review is drawn, before
/// or after, and the banner refuses in today's words, counting files.
#[test]
fn banner_c_in_a_merge_with_a_clash_open_opens_no_review() {
    let mut state = stopped(InProgress::Merge, &["notes.txt"]);
    let (claimed, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(claimed);
    assert!(state.finish.is_none(), "nothing drawn while the read runs");
    let (_, again) = press(&mut state, KeyCode::Char('c'));
    assert_eq!(review_reads(&again), 0, "one read in flight: {again:?}");
    assert!(state.finish.is_none());
    let state = lands(
        &state,
        &commands,
        Err(ReviewFailed::StillOpen {
            files: 1,
            clashes: 2,
        }),
    );
    assert!(state.finish.is_none(), "the review is never drawn");
    assert!(state.banner_focus);
    assert_eq!(
        state.status,
        continue_refused(
            "1 file still conflicted".to_owned(),
            crate::update::banner::refused_at(&state),
            &state.keymap,
        )
    );
}

#[test]
fn banner_c_with_every_file_solved_opens_the_review_when_the_read_lands() {
    let mut state = stopped(InProgress::Merge, &["notes.txt"]);
    let (_, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(state.finish.is_none());
    let (_, again) = press(&mut state, KeyCode::Char('c'));
    assert_eq!(review_reads(&again), 0, "one read in flight: {again:?}");
    let state = lands(&state, &commands, Ok(reviewed()));
    let finish = state.finish.as_ref().expect("the review is open");
    assert_eq!(finish.return_to, ReturnTo::Banner);
    assert_eq!(finish.solved, vec![PathBuf::from("notes.txt")]);
}

/// How many review reads `commands` sends.
fn review_reads(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|command| matches!(command, Command::ReadFinishSummary(_)))
        .count()
}

/// Two clashes, neither taken.
const TWO_OPEN: &[u8] = b"<<<<<<< HEAD\nours 1\n=======\ntheirs 1\n>>>>>>> feature\n\
    middle\n\
    <<<<<<< HEAD\nours 2\n=======\ntheirs 2\n>>>>>>> feature\n";

/// Banner `c` always asks the read (planner, 2026-10-18): once the resolver
/// has closed its per-file count is gone, and one kept would go stale while
/// he fixes files elsewhere. So a resolver left with a clash open refuses
/// nothing at once: one read, and no notice or review before its answer.
#[test]
fn banner_c_after_the_resolver_closed_with_a_clash_open_asks_the_read() {
    use crate::jobs::conflict::{OpenedFile, Opening};
    use graft_git::status::UnmergedHow;

    let state = stopped(InProgress::Merge, &["notes.txt"]);
    let (state, commands) =
        crate::update::update(&state, AppEvent::Core(Event::Key(key(KeyCode::Enter))));
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`⏎` opens the resolver: {commands:?}"));
    let file = graft_git::conflict::parse(TWO_OPEN).expect("two open blocks");
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Produced(
            read,
            Product::ConflictOpened(Ok(Opening::Unmerged(std::sync::Arc::new(OpenedFile {
                file: Some(file),
                how: UnmergedHow::BothModified,
                binary: false,
                on_disk: None,
                unreadable: None,
                start_over: None,
            })))),
        ),
    );
    let open = state.resolver.as_ref().expect("the resolver is open");
    assert!(open.file.is_some(), "the file it read, clashes open");
    let mut state = step(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "the resolver closed");
    assert!(state.banner_focus);
    assert!(state.picks.resolver_seen());
    let status = state.status.clone();

    let (claimed, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(claimed);
    assert_eq!(
        review_reads(&commands),
        1,
        "the read is asked: {commands:?}"
    );
    assert!(state.finish.is_none(), "no review before the answer");
    assert_eq!(state.status, status, "no notice before the answer");
}
