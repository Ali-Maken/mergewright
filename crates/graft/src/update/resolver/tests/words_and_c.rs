//! After the last file's `Stage` lands, the resolver's notice row and `c`
//! must say the same thing at every moment — before the status read the
//! write asked for has come back, and after (review m6-amend-13, Minor 1).
//! Since m6-conflicts/31 the resolver stays open after the last file, so
//! the words are on its notice row, not the status line.

use std::path::Path;

use graft_core::event::KeyCode;
use graft_core::job::JobId;

use super::file_list::listing;
use super::press;
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

/// A write that ran and journaled cleanly, described as `description`.
pub(super) fn applied(description: &str) -> graft_ops::apply::Applied {
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };
    graft_ops::apply::Applied {
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Stage,
            description: description.to_owned(),
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
            merge_before: None,
            merge_after: None,
        },
        refresh: graft_ops::apply::Refresh::WorkingTree,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    }
}

/// A `Stage` of `path` that ran and journaled cleanly.
pub(super) fn staged(path: &str) -> Product {
    Product::Operated(Box::new(crate::jobs::operate::Operated::Done(applied(
        &format!("stage {path}"),
    ))))
}

/// A merge whose other file is done; `⏎` on `conflicted.rs`, the last one,
/// has launched its `Stage` and the answer has just come back. Nothing has
/// re-read the status yet: it still calls `conflicted.rs` unmerged.
pub(super) fn last_stage_lands() -> (AppState, Vec<Command>) {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.banner_focus = false;
    let id = JobId::new(4_500);
    state.operating = Some(id);
    state.status = Status::Busy("stage conflicted.rs".to_owned());
    state.resolver.as_mut().expect("open").staging = Some(id);
    update(&state, AppEvent::Produced(id, staged("conflicted.rs")))
}

/// The done read `resolved_advance`'s reopen asked for, answered: the file
/// just staged, on disk as it now stands.
pub(super) fn done_read_lands(state: &AppState, commands: &[Command]) -> AppState {
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, path) if path == Path::new("conflicted.rs") => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the last file is read again: {commands:?}"));
    let file = graft_git::conflict::parse(b"one\nours a\ntwo\nours b\nthree\n")
        .expect("plain text parses");
    let answer = crate::jobs::conflict::Opening::Done(
        std::sync::Arc::new(graft_ops::conflict::DoneFile::Text {
            file,
            unstaged: false,
        }),
        crate::jobs::conflict::StartOver::Can {
            kind: graft_ops::operation::RestartKind::Markers,
        },
    );
    update(
        state,
        AppEvent::Produced(id, Product::ConflictOpened(Ok(answer))),
    )
    .0
}

/// The notice row's words.
pub(super) fn notice(state: &AppState) -> Option<(String, bool)> {
    state.resolver.as_ref().and_then(|open| open.notice.clone())
}

/// The notice row invites `c`: the notice's words, or the finish row that
/// takes the row once every file is done (m6-conflicts/60).
fn invites_c(state: &AppState) -> bool {
    notice(state).is_some_and(|(words, _)| words.contains("· c "))
        || crate::view::resolver::finish_row(state).is_some()
}

/// The finish row's full words, when it shows.
fn finish_row(state: &AppState) -> Option<String> {
    crate::view::resolver::finish_row(state).map(|row| row.full.as_str().to_owned())
}

/// `state` with no summary owed: the words the row and `c` say wherever it
/// does not open on its own — after `Esc` from it, or a rebase
/// (m6-conflicts/60).
fn unowed(mut state: AppState) -> AppState {
    if let Some(open) = state.resolver.as_mut() {
        open.finish_owed = false;
    }
    state
}

const FINISH_ROW: &str = "✓ every file done — c review and finish the merge";

/// `c` ran `--continue` — or, on a merge, opened the finish summary whose
/// `⏎` does (m6-conflicts/19).
fn continued(commands: &[Command]) -> bool {
    commands.iter().any(|command| {
        matches!(
            command,
            Command::Resume(_, graft_ops::apply::Resume::Continue) | Command::ReadFinishSummary(_)
        )
    })
}

/// **The words and the key agree**: a notice that invites `c` is one where
/// `c` continues; and `c` never answers with an error-looking refusal while
/// nothing is wrong, which `nothing_wrong` says of the repository.
fn agree(state: &AppState, nothing_wrong: bool, when: &str) {
    let (after, commands) = press(state, KeyCode::Char('c'));
    if invites_c(state) {
        assert!(
            continued(&commands),
            "{when}: the notice says {:?} but c answered {:?} / {:?}",
            notice(state),
            notice(&after),
            after.status
        );
    }
    if nothing_wrong {
        assert!(
            !matches!(after.status, Status::Sticky { .. } | Status::Failed(_)),
            "{when}: every file is done, yet c said {:?}",
            after.status
        );
        assert!(
            notice(&after).is_none_or(|(words, problem)| !problem && !words.contains("conflicted")),
            "{when}: every file is done, yet c said {:?}",
            notice(&after)
        );
    }
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

fn checking() -> (String, bool) {
    (crate::update::status_read::CHECKING.to_owned(), false)
}

/// The read the `Stage`'s answer asked for.
pub(super) fn owed_read(commands: &[Command]) -> JobId {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadWorkingTree(id) => Some(*id),
            _ => None,
        })
        .expect("the stage asked for the status")
}

/// The review's probe: stage the last file, press `c` before the status
/// read lands. Before m6-amend-13 the line said `c finishes the merge` and
/// `c` answered `1 file still conflicted`.
#[test]
fn c_before_the_status_read_lands_agrees_with_the_notice() {
    let (landed, commands) = last_stage_lands();
    assert!(landed.resolver.is_some(), "the resolver stays");
    let landed = done_read_lands(&landed, &commands);
    assert_eq!(notice(&landed), Some(checking()));
    agree(&landed, true, "before the read");
}

/// `m6-conflicts/31`: while the status read is out, the notice row — not
/// the status line — says it is checking. Rewritten for m6-conflicts/60:
/// when it lands, the summary opens over the resolver on its own and the
/// notice is cleared.
#[test]
fn a_status_read_out_shows_checking_on_the_notice_row() {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let landed = done_read_lands(&landed, &commands);
    assert_eq!(notice(&landed), Some(checking()));
    assert!(
        !matches!(&landed.status, Status::Busy(words) if words == crate::update::status_read::CHECKING),
        "the status line is not the one checking: {:?}",
        landed.status
    );
    let (fresh, _) = update(&landed, AppEvent::Produced(owed, read(&[])));
    assert_eq!(notice(&fresh), None);
    assert!(fresh.finish.is_some(), "the summary opened on its own");
    assert_eq!(fresh.status, landed.status, "the status line untouched");
}

/// Every moment from the last `Stage` landing to its status read landing,
/// and after: the notice row and `c` say the same thing. Rewritten for
/// m6-conflicts/60: with no summary owed (so `c` is what moves on), the
/// read landing puts up the finish row instead of notice words.
#[test]
fn the_notice_and_c_agree_before_and_after_the_read_lands() {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let landed = unowed(done_read_lands(&landed, &commands));

    // The read is out: the notice says it is checking, and so does `c`.
    assert_eq!(notice(&landed), Some(checking()));
    agree(&landed, true, "before the read");
    let (pressed, sent) = press(&landed, KeyCode::Char('c'));
    assert_eq!(
        notice(&pressed),
        Some(checking()),
        "c says what the notice says"
    );
    assert!(!continued(&sent), "{sent:?}");

    // An older read lands — from before the stage, still calling the file
    // unmerged. It is not the one waited for: nothing changes.
    let (older, _) = update(
        &landed,
        AppEvent::Produced(JobId::new(1), read(&["conflicted.rs"])),
    );
    assert_eq!(notice(&older), Some(checking()));
    agree(&older, true, "after an older read");

    // The read the stage asked for lands: every file done, and `c` goes.
    let (fresh, _) = update(&older, AppEvent::Produced(owed, read(&[])));
    assert_eq!(notice(&fresh), None);
    assert_eq!(finish_row(&fresh).as_deref(), Some(FINISH_ROW));
    agree(&fresh, true, "after the read");

    // The same, when `c` was pressed while it was checking.
    let (fresh, _) = update(&pressed, AppEvent::Produced(owed, read(&[])));
    assert_eq!(notice(&fresh), None);
    assert_eq!(finish_row(&fresh).as_deref(), Some(FINISH_ROW));
    agree(&fresh, true, "after the read, c pressed meanwhile");

    // An edit from outside left a file conflicted after all: the notice is
    // `c`'s own refusal, and `c` says exactly that again.
    let (outside, _) = update(&older, AppEvent::Produced(owed, read(&["gone.txt"])));
    assert!(!invites_c(&outside), "{:?}", notice(&outside));
    let (again, sent) = press(&outside, KeyCode::Char('c'));
    assert!(!continued(&sent), "{sent:?}");
    assert_eq!(notice(&again), notice(&outside), "the notice and c agree");
    assert_eq!(
        notice(&outside),
        Some(("1 file still conflicted · ⏎ marks done".to_owned(), false))
    );
}

/// The read the notice waits on ends with no status: the notice stops
/// saying it is checking, and claims nothing either way.
#[test]
fn an_abandoned_read_clears_the_notice() {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let landed = done_read_lands(&landed, &commands);
    let (ended, _) = update(
        &landed,
        AppEvent::Core(graft_core::event::Event::JobCancelled(owed)),
    );
    assert_eq!(notice(&ended), None);
    assert_eq!(ended.status_read, None);
    agree(&ended, false, "after the read was cancelled");
}

/// The read the notice waits on fails: the notice stops saying it is
/// checking, and the failure is on the status line in its own words
/// (review m6-amend-31, Major 1).
#[test]
fn a_failed_read_clears_the_notice() {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let landed = done_read_lands(&landed, &commands);
    let (ended, _) = update(
        &landed,
        AppEvent::Produced(owed, Product::WorkingTree(Err("status broke".to_owned()))),
    );
    assert_eq!(notice(&ended), None);
    assert_eq!(ended.status, Status::Failed("status broke".to_owned()));
}

/// The job running the read the notice waits on fails outright: the same.
#[test]
fn a_failed_read_job_clears_the_notice() {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let landed = done_read_lands(&landed, &commands);
    let (ended, _) = update(
        &landed,
        AppEvent::Core(graft_core::event::Event::JobFailed(
            owed,
            graft_core::job::JobFailure::Failed("git died".to_owned()),
        )),
    );
    assert_eq!(notice(&ended), None);
    assert!(
        matches!(&ended.status, Status::Failed(words) if words.contains("git died")),
        "{:?}",
        ended.status
    );
}
