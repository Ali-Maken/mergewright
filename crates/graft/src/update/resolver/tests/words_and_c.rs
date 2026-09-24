//! After the last file's `Stage` lands, the status line and `c` must say
//! the same thing at every moment — before the status read the write asked
//! for has come back, and after (review m6-amend-13, Minor 1).

use graft_core::event::KeyCode;
use graft_core::job::JobId;

use super::file_list::listing;
use super::press;
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

/// A `Stage` of `path` that ran and journaled cleanly.
fn staged(path: &str) -> Product {
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };
    Product::Operated(Box::new(crate::jobs::operate::Operated::Done(
        graft_ops::apply::Applied {
            entry: JournalEntry {
                id: EntryId::new(1, [0; 10]),
                at_unix_ms: 1,
                kind: OperationKind::Stage,
                description: format!("stage {path}"),
                status: EntryStatus::Committed,
                touches: Touches::default(),
                head_before: HeadRef::Branch("refs/heads/main".to_owned()),
                head_after: None,
                refs_before: std::collections::BTreeMap::new(),
                refs_after: std::collections::BTreeMap::new(),
                index_before: None,
                index_after: None,
                worktree_before: None,
                worktree_after: None,
                backup_ref: None,
                pins: Vec::new(),
                commands: Vec::new(),
                reverses: None,
            },
            refresh: graft_ops::apply::Refresh::WorkingTree,
            step_backup: None,
        },
    )))
}

/// A merge whose other file is done; `⏎` on `conflicted.rs`, the last one,
/// has launched its `Stage` and the answer has just come back. Nothing has
/// re-read the status yet: it still calls `conflicted.rs` unmerged.
fn last_stage_lands() -> (AppState, Vec<Command>) {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.banner_focus = false;
    let id = JobId::new(4_500);
    state.operating = Some(id);
    state.status = Status::Busy("stage conflicted.rs".to_owned());
    state.resolver.as_mut().expect("open").staging = Some(id);
    update(&state, AppEvent::Produced(id, staged("conflicted.rs")))
}

/// The line invites `c`.
fn invites_c(state: &AppState) -> bool {
    matches!(&state.status, Status::Done(words) if words.contains("· c "))
}

/// `c` ran `--continue`.
fn continued(commands: &[Command]) -> bool {
    commands.iter().any(|command| {
        matches!(
            command,
            Command::Resume(_, graft_ops::apply::Resume::Continue)
        )
    })
}

/// **The words and the key agree**: a line that invites `c` is one where
/// `c` continues; and `c` never answers with an error-looking refusal while
/// nothing is wrong, which `nothing_wrong` says of the repository.
fn agree(state: &AppState, nothing_wrong: bool, when: &str) {
    let (after, commands) = press(state, KeyCode::Char('c'));
    if invites_c(state) {
        assert!(
            continued(&commands),
            "{when}: the line says {:?} but c answered {:?}",
            state.status,
            after.status
        );
    }
    if nothing_wrong {
        assert!(
            !matches!(after.status, Status::Sticky { .. } | Status::Failed(_)),
            "{when}: every file is done, yet c said {:?}",
            after.status
        );
    }
}

/// The review's probe: stage the last file, press `c` before the status
/// read lands. Before the fix the line said `c finishes the merge` and `c`
/// answered `1 file still conflicted`.
#[test]
fn c_before_the_status_read_lands_agrees_with_the_line() {
    let (landed, _) = last_stage_lands();
    assert!(
        landed.resolver.is_none(),
        "the last file closed the resolver"
    );
    assert!(landed.banner_focus, "the keyboard is on the banner");
    agree(&landed, true, "before the read");
}

/// A status read answering: `unmerged` of the two files still unmerged,
/// the merge still in progress.
fn read(unmerged: &[&str]) -> Product {
    let entries: Vec<(&str, bool)> = ["gone.txt", "conflicted.rs"]
        .into_iter()
        .map(|path| (path, unmerged.contains(&path)))
        .collect();
    Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
        status: super::file_list::status(&entries),
        in_progress: Some(crate::update::resolver::fixtures::operation(
            graft_git::state::InProgress::Merge,
            None,
        )),
        mergetool: None,
    })))
}

fn checking() -> Status {
    Status::Busy(crate::update::status_read::CHECKING.to_owned())
}

/// The read the `Stage`'s answer asked for.
fn owed_read(commands: &[Command]) -> JobId {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadWorkingTree(id) => Some(*id),
            _ => None,
        })
        .expect("the stage asked for the status")
}

/// Every moment from the last `Stage` landing to its status read landing,
/// and after: the line and `c` say the same thing.
#[test]
fn the_line_and_c_agree_before_and_after_the_read_lands() {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let done = Status::Done("every file done ✓ · c finishes the merge".to_owned());

    // The read is out: the line says it is checking, and so does `c`.
    assert_eq!(landed.status, checking());
    agree(&landed, true, "before the read");
    let (pressed, sent) = press(&landed, KeyCode::Char('c'));
    assert_eq!(pressed.status, checking(), "c says what the line says");
    assert!(!continued(&sent), "{sent:?}");

    // An older read lands — from before the stage, still calling the file
    // unmerged. It is not the one waited for: nothing changes.
    let (older, _) = update(
        &landed,
        AppEvent::Produced(JobId::new(1), read(&["conflicted.rs"])),
    );
    assert_eq!(older.status, checking());
    agree(&older, true, "after an older read");

    // The read the stage asked for lands: every file done, and `c` goes.
    let (fresh, _) = update(&older, AppEvent::Produced(owed, read(&[])));
    assert_eq!(fresh.status, done);
    agree(&fresh, true, "after the read");

    // The same, when `c` was pressed while it was checking.
    let (fresh, _) = update(&pressed, AppEvent::Produced(owed, read(&[])));
    assert_eq!(fresh.status, done);
    agree(&fresh, true, "after the read, c pressed meanwhile");

    // An edit from outside left a file conflicted after all: the line is
    // `c`'s own refusal, and `c` says exactly that again.
    let (outside, _) = update(&older, AppEvent::Produced(owed, read(&["gone.txt"])));
    assert!(!invites_c(&outside), "{:?}", outside.status);
    let (again, sent) = press(&outside, KeyCode::Char('c'));
    assert!(!continued(&sent), "{sent:?}");
    assert_eq!(again.status, outside.status, "the line and c agree");
    assert_eq!(super::sticky(&outside).0, "1 file still conflicted");
}

/// The read the line waits on ends with no status: the line stops saying
/// it is checking, and claims nothing either way.
#[test]
fn a_read_that_never_answers_stops_the_checking_line() {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let (ended, _) = update(
        &landed,
        AppEvent::Core(graft_core::event::Event::JobCancelled(owed)),
    );
    assert_eq!(ended.status, Status::Ready);
    assert_eq!(ended.status_read, None);
    agree(&ended, false, "after the read was cancelled");
}
