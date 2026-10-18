//! `c` in the resolver (m6-conflicts/31, P8): after the last file the
//! resolver stays, and `c` there continues anything but a merge — refused
//! in words while it cannot. A merge stages nothing while the user picks
//! (ease-pass/05b): `y` (or `c`) opens its review once every clash is
//! solved, and `Esc` from the review is the resolver again.

use std::path::Path;

use mergewright_core::event::{KeyCode, KeyPress, Modifiers};
use mergewright_core::job::JobId;
use mergewright_git::state::InProgress;

use super::file_list::{listing, status};
use super::words_and_c::{applied, done_read_lands, last_stage_lands, notice};
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

/// A merge whose other file is done and whose last, `conflicted.rs`, has
/// every block taken: solved on disk, still unmerged — nothing is staged
/// until the review's `y` (ease-pass/05b).
pub(super) fn merge_solved() -> AppState {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.banner_focus = false;
    if let Some(file) = state.resolver.as_mut().and_then(|open| open.file.as_mut()) {
        std::sync::Arc::make_mut(file).take_all(mergewright_git::conflict::Take::Theirs);
    }
    state
}

/// Every clash solved and `y` pressed: the review open over the resolver.
/// The commands `y` sent ride along.
pub(super) fn every_file_done_sent() -> (AppState, Vec<Command>) {
    press(&merge_solved(), KeyCode::Char('y'))
}

/// [`every_file_done_sent`]'s state, its read landed: the summary open over
/// the resolver (it opens only when the read lands, ease-pass/05s).
fn every_file_done() -> AppState {
    summary_open()
}

/// The review's one read in `commands`, answered with a summary: the
/// review opens now (ease-pass/05s).
pub(super) fn review_lands(state: &AppState, commands: &[Command]) -> AppState {
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadFinishSummary(id) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the summary is read: {commands:?}"));
    let summary = std::sync::Arc::new(mergewright_ops::finish::FinishSummary {
        into: b"main".to_vec(),
        from: b"feature".to_vec(),
        by_you: Vec::new(),
        by_git: Vec::new(),
        message: b"Merge branch 'feature'\n".to_vec(),
        recorded: true,
    });
    update(
        state,
        AppEvent::Produced(id, Product::FinishSummary(Ok(summary.into()))),
    )
    .0
}

/// The review `y` opened, closed with `Esc`: the resolver, every clash
/// solved.
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

#[test]
fn c_in_the_resolver_opens_the_summary() {
    let state = finish_row_showing();
    let (after, commands) = press(&state, KeyCode::Char('c'));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "{commands:?}"
    );
    // The review opens when its read lands (ease-pass/05s).
    let after = review_lands(&after, &commands);
    let finish = after.finish.as_ref().expect("c opened the summary");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert!(after.resolver.is_some(), "the resolver stays under it");
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
        Some((
            "2 clashes still open · n goes to the next".to_owned(),
            false
        ))
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
        mergewright_git::conflict::Take::Ours,
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
    state.live_entries = vec![mergewright_core::journal::EntryId::new(1, [0; 10])];
    state.banner_focus = false;
    let (after, commands) = press(&state, KeyCode::Char('c'));
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Resume(_, mergewright_ops::apply::Resume::Continue { .. }, ..)
        )),
        "{commands:?}"
    );
    assert!(after.finish.is_none());
    assert!(after.resolver.is_none(), "closed to the banner as it sends");
    assert!(after.banner_focus);
}

/// The review `y` opened, with its read answered.
fn summary_open() -> AppState {
    let (state, commands) = every_file_done_sent();
    review_lands(&state, &commands)
}

#[test]
fn esc_from_the_summary_returns_to_the_resolver() {
    let state = summary_open();
    let (state, _) = press(&state, KeyCode::Escape);
    assert!(state.finish.is_none());
    assert_eq!(open_path(&state), Path::new("conflicted.rs"));
    // The next key reaches the resolver: `n` with every clash solved says
    // where the review is.
    let (after, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(
        notice(&after),
        Some((
            "no open clashes left \u{b7} y reviews the merge".to_owned(),
            false
        ))
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
    assert_eq!(state.status, Status::done("Merged feature into main."));
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
                entry: mergewright_core::journal::EntryId::new(1, [0; 10]),
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
        AppEvent::Core(mergewright_core::event::Event::Key(KeyPress {
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
