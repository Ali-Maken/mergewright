//! `c` in the resolver (m6-conflicts/31, P8): after the last file the
//! resolver stays, and `c` there finishes a merge through its summary or
//! continues anything else — refused in words while it cannot. Since
//! m6-conflicts/60 a merge's summary opens on its own once the last stage
//! and its reads have landed; `Esc` from it leaves the finish row.

use std::path::Path;

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_git::state::InProgress;

use super::file_list::{listing, status};
use super::words_and_c::{applied, done_read_lands, last_stage_lands, notice, owed_read};
use super::{open_path, press, sticky};
use crate::update::finish::ReturnTo;
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

/// The status read the last stage asked for, answering that nothing is
/// unmerged.
pub(super) fn nothing_unmerged() -> Product {
    Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
        rewrite: None,
        status: status(&[("gone.txt", false), ("conflicted.rs", false)]),
        in_progress: Some(crate::update::resolver::fixtures::operation(
            InProgress::Merge,
            None,
        )),
        mergetool: None,
    })))
}

/// Every file done and read back: the stage landed, the file's done read
/// landed, and the status read the stage asked for said nothing is
/// unmerged — so the summary opened on its own (m6-conflicts/60). The
/// commands of that last answer ride along.
pub(super) fn every_file_done_sent() -> (AppState, Vec<Command>) {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let landed = done_read_lands(&landed, &commands);
    update(&landed, AppEvent::Produced(owed, nothing_unmerged()))
}

/// [`every_file_done_sent`]'s state: the summary open over the resolver.
fn every_file_done() -> AppState {
    every_file_done_sent().0
}

/// The summary that opened on its own, closed with `Esc`: the resolver
/// with the finish row (m6-conflicts/60, J14).
pub(super) fn finish_row_showing() -> AppState {
    let (state, _) = press(&every_file_done(), KeyCode::Escape);
    assert!(state.finish.is_none(), "Esc closed the summary");
    state
}

/// No command that writes, resumes or reads a summary.
fn nothing_sent(commands: &[Command]) {
    assert!(
        !commands.iter().any(|command| matches!(
            command,
            Command::Resume(..) | Command::ReadFinishSummary(_) | Command::Operate(..)
        )),
        "{commands:?}"
    );
}

/// Ticket 60's premise, pinned: the status read after the last `Stage` is
/// asked while that Stage's answer is handled — by `operate::operated`'s
/// `refresh_scope` (`operate.rs`, `Refresh::WorkingTree`), before
/// `resolved_advance` runs — so it is owed the moment the owed open is set.
/// A read asked later would let the open spend on a status from before the
/// Stage.
#[test]
fn the_read_after_the_last_stage_is_owed_when_it_lands() {
    let (landed, commands) = last_stage_lands();
    assert!(
        crate::update::status_read::owed(&landed),
        "the status read is out when the Stage's answer returns"
    );
    let owed = owed_read(&commands);
    assert_eq!(landed.status_read, Some(owed), "and it is the newest one");
    assert!(
        landed
            .resolver
            .as_ref()
            .is_some_and(|open| open.finish_owed),
        "the owed open is set on the same answer"
    );
}

#[test]
fn c_in_the_resolver_opens_the_summary() {
    let state = finish_row_showing();
    let (after, commands) = press(&state, KeyCode::Char('c'));
    let finish = after.finish.as_ref().expect("c opened the summary");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert!(after.resolver.is_some(), "the resolver stays under it");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "{commands:?}"
    );
}

/// Rewritten for m6-conflicts/60 (was `the_resolver_stays_after_the_last_file`):
/// the resolver stays on the last file while the reads are out, the
/// status line says the file is done and every file is done, and once the
/// status read lands the summary opens over it on its own.
#[test]
fn the_last_file_of_a_merge_opens_the_summary_once_the_reads_land() {
    let (landed, commands) = last_stage_lands();
    let open = landed.resolver.as_ref().expect("the resolver stays");
    assert_eq!(open.path, Path::new("conflicted.rs"), "on the same path");
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::OpenConflict(_, path) if path == Path::new("conflicted.rs")
        )),
        "reopened through its read: {commands:?}"
    );
    assert!(!landed.banner_focus);
    assert!(landed.finish.is_none(), "not before the reads land");
    // The status the stage asked for is out, so the notice says what `c`
    // would say.
    assert_eq!(
        notice(&landed),
        Some((crate::update::status_read::CHECKING.to_owned(), false))
    );
    assert_eq!(
        landed.status,
        Status::Done {
            what: "conflicted.rs done".to_owned(),
            also: None,
            keys: Some("every file done".to_owned()),
        },
        "the summary is coming, so no key is named"
    );
    let (state, commands) = every_file_done_sent();
    let finish = state.finish.as_ref().expect("the summary opened");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert_eq!(finish.focus, crate::update::finish::FinishFocus::Files);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "{commands:?}"
    );
    assert!(state.resolver.is_some(), "the resolver under it");
    assert!(
        !state.resolver.as_ref().is_some_and(|open| open.finish_owed),
        "spent"
    );
    assert_eq!(state.status, landed.status, "the status line kept");
    assert!(!state.banner_focus);
}

#[test]
fn the_last_file_reopens_read_only() {
    let (landed, commands) = last_stage_lands();
    let before = notice(&landed);
    let state = done_read_lands(&landed, &commands);
    let open = state.resolver.as_ref().expect("open");
    assert!(open.done.is_some(), "done, read-only");
    assert_eq!(notice(&state), before, "the notice survives the read");
    let (after, commands) = press(&state, KeyCode::Char('o'));
    assert_eq!(
        sticky(&after).0,
        crate::update::resolver::done_refusal(&state).expect("a done file")
    );
    nothing_sent(&commands);
}

#[test]
fn c_with_files_open_is_refused_in_words() {
    let state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    let (after, commands) = press(&state, KeyCode::Char('c'));
    assert_eq!(
        notice(&after),
        Some(("1 file still conflicted · ⏎ marks done".to_owned(), false))
    );
    assert!(after.finish.is_none());
    // The fixture's ambient walk work (an `EvilProbe`) rides along with
    // any key; nothing `c` would send is there.
    nothing_sent(&commands);
}

#[test]
fn c_while_a_take_is_landing_is_refused() {
    let mut state = finish_row_showing();
    let open = state.resolver.as_mut().expect("open");
    open.job = Some(JobId::new(8_000));
    open.in_flight = Some((
        crate::update::TakeAt::Block(0),
        graft_git::conflict::Take::Ours,
    ));
    let (after, commands) = press(&state, KeyCode::Char('c'));
    assert_eq!(
        sticky(&after).0,
        "a take is still landing · try again in a moment"
    );
    assert!(after.finish.is_none());
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn c_in_a_rebase_resolver_continues() {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &[]);
    state.in_progress = Some(crate::update::resolver::fixtures::operation(
        InProgress::Rebase,
        None,
    ));
    state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    state.banner_focus = false;
    let (after, commands) = press(&state, KeyCode::Char('c'));
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Resume(_, graft_ops::apply::Resume::Continue, ..)
        )),
        "{commands:?}"
    );
    assert!(after.finish.is_none());
    assert!(after.resolver.is_none(), "closed to the banner as it sends");
    assert!(after.banner_focus);
}

/// The summary that opened on its own, with its read answered.
fn summary_open() -> AppState {
    let (state, commands) = every_file_done_sent();
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadFinishSummary(id) => Some(*id),
            _ => None,
        })
        .expect("the summary is read");
    let summary = std::sync::Arc::new(graft_ops::finish::FinishSummary {
        into: b"main".to_vec(),
        from: b"feature".to_vec(),
        by_you: Vec::new(),
        by_git: Vec::new(),
        message: b"Merge branch 'feature'\n".to_vec(),
        recorded: true,
    });
    update(
        &state,
        AppEvent::Produced(id, Product::FinishSummary(Ok(summary))),
    )
    .0
}

#[test]
fn esc_from_the_summary_returns_to_the_resolver() {
    let state = summary_open();
    // `⇥` to the subject, and a word typed onto git's message.
    let (mut state, _) = press(&state, KeyCode::Tab);
    for character in " now".chars() {
        state = press(&state, KeyCode::Char(character)).0;
    }
    let (state, _) = press(&state, KeyCode::Escape);
    assert!(state.finish.is_none());
    assert_eq!(open_path(&state), Path::new("conflicted.rs"));
    assert_eq!(
        state
            .finish_draft
            .as_ref()
            .map(|(subject, _)| subject.as_str()),
        Some("Merge branch 'feature' now")
    );
    // The next key reaches the resolver: `o` on a done file is refused in
    // its words.
    let (after, _) = press(&state, KeyCode::Char('o'));
    assert_eq!(
        sticky(&after).0,
        crate::update::resolver::done_refusal(&state).expect("a done file")
    );
}

#[test]
fn a_merge_committed_from_the_resolver_lands_on_the_graph() {
    let state = summary_open();
    let (state, _) = press(&state, KeyCode::Char('y'));
    let id = state.operating.expect("the commit runs");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Done(applied(
                "continue merge",
            )))),
        ),
    );
    assert!(state.finish.is_none(), "the summary closed");
    assert!(state.resolver.is_none(), "and the resolver with it");
    assert_eq!(state.status, Status::done("merged feature into main"));
}

/// The commit ran but its record is short: the merge is over, so the
/// resolver under the summary closes with it (review m6-amend-31, Minor 1).
#[test]
fn an_incomplete_commit_from_the_resolver_closes_it() {
    let state = summary_open();
    let (state, _) = press(&state, KeyCode::Char('y'));
    let id = state.operating.expect("the commit runs");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Incomplete {
                applied: applied("continue merge"),
                error: "could not read HEAD".to_owned(),
            })),
        ),
    );
    assert!(state.finish.is_none(), "the summary closed");
    assert!(state.resolver.is_none(), "and the resolver with it");
}

/// The `--continue` stopped on new conflicts: the stale resolver closes and
/// the new stop opens a fresh one on its first path.
#[test]
fn a_new_stop_from_the_resolver_opens_its_own_resolver() {
    let state = summary_open();
    let (state, _) = press(&state, KeyCode::Char('y'));
    let id = state.operating.expect("the commit runs");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Conflicted {
                rewrite: None,
                description: "continue merge".to_owned(),
                kind: InProgress::Merge,
                paths: vec!["fresh.txt".into()],
                stalled: None,
                entry: graft_core::journal::EntryId::new(1, [0; 10]),
            })),
        ),
    );
    assert!(state.finish.is_none(), "the summary closed");
    assert_eq!(open_path(&state), Path::new("fresh.txt"));
}

#[test]
fn a_hook_refusal_keeps_the_summary_and_the_resolver_under_it() {
    let state = summary_open();
    let (state, _) = press(&state, KeyCode::Char('y'));
    let id = state.operating.expect("the commit runs");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Failed {
                error: "the commit-msg hook said no".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
                edits: None,
            })),
        ),
    );
    assert!(state.finish.is_some(), "Kept: the summary stays");
    assert_eq!(open_path(&state), Path::new("conflicted.rs"));
}

/// `Ctrl+C` still passes the resolver, `c` or no `c`.
#[test]
fn ctrl_c_is_not_c() {
    let state = finish_row_showing();
    let (after, commands) = update(
        &state,
        AppEvent::Core(graft_core::event::Event::Key(KeyPress {
            code: KeyCode::Char('c'),
            modifiers: Modifiers {
                control: true,
                ..Modifiers::NONE
            },
        })),
    );
    assert!(after.finish.is_none());
    nothing_sent(&commands);
}
