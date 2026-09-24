//! `c` in the resolver (m6-conflicts/31, P8): after the last file the
//! resolver stays, and `c` there finishes a merge through its summary or
//! continues anything else — refused in words while it cannot.

use std::path::Path;

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_git::state::InProgress;

use super::file_list::{listing, status};
use super::words_and_c::{applied, done_read_lands, last_stage_lands, notice, owed_read};
use super::{open_path, press, sticky};
use crate::update::finish::ReturnTo;
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

/// Every file done and read back: the stage landed, the file's done read
/// landed, and the status read the stage asked for said nothing is
/// unmerged.
fn every_file_done() -> AppState {
    let (landed, commands) = last_stage_lands();
    let owed = owed_read(&commands);
    let landed = done_read_lands(&landed, &commands);
    let read = Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
        status: status(&[("gone.txt", false), ("conflicted.rs", false)]),
        in_progress: Some(crate::update::resolver::fixtures::operation(
            InProgress::Merge,
            None,
        )),
        mergetool: None,
    })));
    update(&landed, AppEvent::Produced(owed, read)).0
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
    let state = every_file_done();
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

#[test]
fn the_resolver_stays_after_the_last_file() {
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
    // The status the stage asked for is out, so the notice says what `c`
    // would say — the words after it lands are in `words_and_c`.
    assert_eq!(
        notice(&landed),
        Some((crate::update::status_read::CHECKING.to_owned(), false))
    );
    assert!(
        !matches!(&landed.status, Status::Busy(words) if words == crate::update::status_read::CHECKING),
        "the status line is not written: {:?}",
        landed.status
    );
    let state = every_file_done();
    assert_eq!(
        notice(&state),
        Some(("every file done ✓ · c finishes the merge".to_owned(), false))
    );
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
    let mut state = every_file_done();
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
    state.banner_focus = false;
    let (after, commands) = press(&state, KeyCode::Char('c'));
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Resume(_, graft_ops::apply::Resume::Continue)
        )),
        "{commands:?}"
    );
    assert!(after.finish.is_none());
    assert!(after.resolver.is_none(), "closed to the banner as it sends");
    assert!(after.banner_focus);
}

/// The summary opened from here, with its read answered.
fn summary_open() -> AppState {
    let (state, commands) = press(&every_file_done(), KeyCode::Char('c'));
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadFinishSummary(id) => Some(*id),
            _ => None,
        })
        .expect("c reads the summary");
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
    let (state, _) = press(&state, KeyCode::Enter);
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
    assert_eq!(
        state.status,
        Status::Done("merged feature into main".to_owned())
    );
}

#[test]
fn a_hook_refusal_keeps_the_summary_and_the_resolver_under_it() {
    let state = summary_open();
    let (state, _) = press(&state, KeyCode::Enter);
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
            })),
        ),
    );
    assert!(state.finish.is_some(), "Kept: the summary stays");
    assert_eq!(open_path(&state), Path::new("conflicted.rs"));
}

/// `Ctrl+C` still passes the resolver, `c` or no `c`.
#[test]
fn ctrl_c_is_not_c() {
    let state = every_file_done();
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
