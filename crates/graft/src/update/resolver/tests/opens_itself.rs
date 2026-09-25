//! The last file of a merge opens the finish summary on its own
//! (m6-conflicts/60, P11): once, when everything its stage started has
//! landed — and never on a state reached any other way.

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_git::state::InProgress;

use super::file_list::{click, listing, status};
use super::finish_here::{finish_row_showing, nothing_unmerged};
use super::press;
use super::words_and_c::{applied, done_read_lands, last_stage_lands, notice, owed_read};
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

/// How many summary reads `commands` asked for.
fn summaries(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|command| matches!(command, Command::ReadFinishSummary(_)))
        .count()
}

fn owed(state: &AppState) -> bool {
    state.resolver.as_ref().is_some_and(|open| open.finish_owed)
}

/// A status read answering that nothing is unmerged, `kind` in progress.
fn clean_read(kind: InProgress) -> Product {
    Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
        status: status(&[("gone.txt", false), ("conflicted.rs", false)]),
        in_progress: Some(crate::update::resolver::fixtures::operation(kind, None)),
        mergetool: None,
    })))
}

/// [`last_stage_lands`], for `kind`.
fn last_stage_lands_in(kind: InProgress) -> (AppState, Vec<Command>) {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.in_progress = Some(crate::update::resolver::fixtures::operation(kind, None));
    state.banner_focus = false;
    let id = JobId::new(4_500);
    state.operating = Some(id);
    state.status = Status::Busy("stage conflicted.rs".to_owned());
    state.resolver.as_mut().expect("open").staging = Some(id);
    let staged = Product::Operated(Box::new(crate::jobs::operate::Operated::Done(applied(
        "stage conflicted.rs",
    ))));
    update(&state, AppEvent::Produced(id, staged))
}

/// The flag is on the state `reopen` built, not lost with the old one.
#[test]
fn the_owed_open_survives_the_reopen() {
    let (landed, commands) = last_stage_lands();
    let open = landed.resolver.as_ref().expect("open");
    assert!(open.job.is_some() && open.reading, "a fresh opening read");
    assert!(open.finish_owed);
    let state = done_read_lands(&landed, &commands);
    assert!(owed(&state), "the done read landing does not spend it");
    assert!(state.finish.is_none(), "the status read is still out");
}

/// `reopen` refused (a box up): nothing owed, its words on the line.
#[test]
fn a_refused_reopen_opens_nothing() {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.banner_focus = false;
    state.confirm = Some(crate::update::confirm::PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            graft_term::sanitize_line("Discard changes?"),
            vec![graft_term::sanitize_line("1 file loses its edits.")],
            graft_term::sanitize_line("git restore ."),
        ),
        then: crate::update::confirm::ConfirmThen::Undo { redo: false },
    });
    let staged = JobId::new(4_500);
    state.resolver.as_mut().expect("open").staging = Some(staged);
    let mut commands = Vec::new();
    let mut ids = 4_500;
    crate::update::resolver::resolved_advance(&mut state, &mut commands, staged, &mut || {
        ids += 1;
        JobId::new(ids)
    });
    assert!(!owed(&state));
    assert_eq!(state.status, Status::sticky("finish the box first"));
    crate::update::resolver::finish_if_owed(&mut state, &mut commands, &mut || JobId::new(9_000));
    assert!(state.finish.is_none());
    assert_eq!(summaries(&commands), 0);
}

/// `T` then `⏎` typed faster than the disk: a take still landing holds the
/// summary; it opens once, after the take lands too.
#[test]
fn a_take_still_landing_holds_the_summary() {
    let (landed, commands) = last_stage_lands();
    let read = owed_read(&commands);
    let mut state = done_read_lands(&landed, &commands);
    let take = (
        crate::update::TakeAt::Block(0),
        graft_git::conflict::Take::Theirs,
    );
    let open = state.resolver.as_mut().expect("open");
    open.in_flight = Some(take);
    open.queued.push_back(take);
    let (held, sent) = update(&state, AppEvent::Produced(read, nothing_unmerged()));
    assert!(held.finish.is_none(), "held while a take is out");
    assert!(owed(&held), "still owed");
    assert_eq!(summaries(&sent), 0);

    let mut state = held;
    let open = state.resolver.as_mut().expect("open");
    open.in_flight = None;
    open.queued.clear();
    // The next answer, whatever it is, finds everything landed.
    let (opened, sent) = update(
        &state,
        AppEvent::Produced(JobId::new(1), nothing_unmerged()),
    );
    assert!(opened.finish.is_some(), "opened once the take landed");
    assert_eq!(summaries(&sent), 1, "once");
    let (again, sent) = update(
        &opened,
        AppEvent::Produced(JobId::new(2), nothing_unmerged()),
    );
    assert_eq!(summaries(&sent), 0, "never twice");
    assert!(again.finish.is_some());
}

/// The last `Stage` failed: the resolver stays on the file with the write's
/// own words; nothing is owed.
#[test]
fn a_failed_last_stage_opens_nothing() {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.banner_focus = false;
    let id = JobId::new(4_500);
    state.operating = Some(id);
    state.resolver.as_mut().expect("open").staging = Some(id);
    let (after, sent) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(crate::jobs::operate::Operated::Failed {
                error: "index.lock exists".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
                edits: None,
            })),
        ),
    );
    let open = after.resolver.as_ref().expect("the resolver stays");
    assert_eq!(open.staging, None);
    assert!(!open.finish_owed);
    assert!(after.finish.is_none());
    assert_eq!(summaries(&sent), 0);
    assert!(
        matches!(&after.status, Status::Failed(words) | Status::Sticky { reason: words, .. } if words.contains("index.lock")),
        "{:?}",
        after.status
    );
}

/// `Esc` before the status read lands: the resolver closes, and the read
/// landing opens nothing.
#[test]
fn esc_before_the_read_lands_cancels_the_open() {
    let (landed, commands) = last_stage_lands();
    let read = owed_read(&commands);
    let state = done_read_lands(&landed, &commands);
    let (closed, _) = press(&state, KeyCode::Escape);
    assert!(closed.resolver.is_none(), "Esc closed the resolver");
    let (after, sent) = update(&closed, AppEvent::Produced(read, nothing_unmerged()));
    assert!(after.finish.is_none());
    assert_eq!(summaries(&sent), 0);
}

/// `j` to the other file before the read lands: the fresh state owes
/// nothing, and the read landing opens nothing over it.
#[test]
fn switching_file_cancels_the_open() {
    let (landed, commands) = last_stage_lands();
    let read = owed_read(&commands);
    let state = done_read_lands(&landed, &commands);
    let (moved, _) = press(&state, KeyCode::Char('j'));
    assert_eq!(super::open_path(&moved), std::path::Path::new("gone.txt"));
    assert!(!owed(&moved));
    let (after, sent) = update(&moved, AppEvent::Produced(read, nothing_unmerged()));
    assert!(after.finish.is_none());
    assert_eq!(summaries(&sent), 0);
}

/// `d`, `e`, `M`, `n` cancel it too, whatever they then do.
#[test]
fn keys_that_move_him_on_cancel_the_open() {
    let (landed, commands) = last_stage_lands();
    let state = done_read_lands(&landed, &commands);
    for key in ['d', 'e', 'M', 'n', 'N', 'j'] {
        let (after, _) = press(&state, KeyCode::Char(key));
        assert!(!owed(&after), "{key}");
    }
    // A key that moves nothing — a scroll — keeps it.
    let (after, _) = press(&state, KeyCode::PageDown);
    assert!(owed(&after), "PgDn");
}

/// The status read fails: no summary, the failure's words on the line
/// (rule 3), and the owed open cancelled.
///
/// The spec expected the bar to still start `c finish` here. It cannot:
/// the status the app holds is the one from before the Stage, still
/// calling the file unmerged (`banner::continue_refusal`), so `c` refuses
/// — and the bar never names a key that answers "not yet"
/// (`view::rows::resolver_context`). Pinned as built; see the ticket NOTES.
#[test]
fn a_failed_status_read_cancels_the_open_and_says_so() {
    let (landed, commands) = last_stage_lands();
    let read = owed_read(&commands);
    let state = done_read_lands(&landed, &commands);
    let (after, sent) = update(
        &state,
        AppEvent::Produced(read, Product::WorkingTree(Err("status broke".to_owned()))),
    );
    assert!(after.finish.is_none());
    assert_eq!(summaries(&sent), 0);
    assert!(!owed(&after));
    assert_eq!(after.status, Status::Failed("status broke".to_owned()));
    assert_eq!(notice(&after), None, "no checking words left standing");
    assert!(
        crate::view::resolver::finish_row(&after).is_none(),
        "no finish row over a failure"
    );
    assert!(
        !super::bar(&after).iter().any(|hint| hint == "c finish"),
        "{:?}",
        super::bar(&after)
    );
}

/// The job running the read fails, or is cancelled: cancelled the same.
#[test]
fn a_read_that_never_answers_cancels_the_open() {
    let (landed, commands) = last_stage_lands();
    let read = owed_read(&commands);
    let state = done_read_lands(&landed, &commands);
    for event in [
        graft_core::event::Event::JobCancelled(read),
        graft_core::event::Event::JobFailed(
            read,
            graft_core::job::JobFailure::Failed("git died".to_owned()),
        ),
    ] {
        let (after, sent) = update(&state, AppEvent::Core(event));
        assert!(after.finish.is_none());
        assert!(!owed(&after));
        assert_eq!(summaries(&sent), 0);
    }
}

/// An edit from outside left a file conflicted: `c`'s refusal on the
/// notice, no summary, and the owed open dropped, not retried.
#[test]
fn a_status_still_conflicted_cancels_the_open() {
    let (landed, commands) = last_stage_lands();
    let read = owed_read(&commands);
    let state = done_read_lands(&landed, &commands);
    let conflicted = Product::WorkingTree(Ok(Box::new(crate::working_tree::WorkingTree {
        status: status(&[("gone.txt", true), ("conflicted.rs", false)]),
        in_progress: Some(crate::update::resolver::fixtures::operation(
            InProgress::Merge,
            None,
        )),
        mergetool: None,
    })));
    let (after, sent) = update(&state, AppEvent::Produced(read, conflicted));
    assert!(after.finish.is_none());
    assert!(!owed(&after));
    assert_eq!(summaries(&sent), 0);
    assert_eq!(
        notice(&after),
        Some(("1 file still conflicted · ⏎ marks done".to_owned(), false))
    );
}

/// A rebase's `c` writes, so nothing opens on its own: the finish row, and
/// the status line naming the key.
#[test]
fn the_last_file_of_a_rebase_shows_the_finish_row() {
    let (landed, commands) = last_stage_lands_in(InProgress::Rebase);
    assert!(!owed(&landed));
    let read = owed_read(&commands);
    let state = done_read_lands(&landed, &commands);
    let (after, sent) = update(
        &state,
        AppEvent::Produced(read, clean_read(InProgress::Rebase)),
    );
    assert!(after.finish.is_none());
    assert_eq!(summaries(&sent), 0);
    assert!(
        !sent
            .iter()
            .any(|command| matches!(command, Command::Resume(..))),
        "nothing written: {sent:?}"
    );
    assert_eq!(notice(&after), None);
    assert_eq!(
        crate::view::resolver::finish_row(&after)
            .map(|row| (row.full.as_str().to_owned(), row.short.as_str().to_owned())),
        Some((
            "✓ every file done — c continue the rebase".to_owned(),
            "✓ every file done — c continue".to_owned()
        ))
    );
    assert_eq!(
        after.status,
        Status::Done {
            what: "conflicted.rs done".to_owned(),
            also: None,
            keys: Some("every file done · c continues the rebase".to_owned()),
        }
    );
    assert_eq!(
        super::bar(&after).first().map(String::as_str),
        Some("c continue")
    );
}

/// Opened from the banner with every file already done (a relaunch, `u`):
/// no event, so no summary — the finish row instead.
#[test]
fn a_resolver_opened_on_a_done_merge_opens_no_summary() {
    let (landed, commands) = last_stage_lands();
    let read = owed_read(&commands);
    let mut state = done_read_lands(&landed, &commands);
    state.resolver.as_mut().expect("open").finish_owed = false;
    let (after, sent) = update(&state, AppEvent::Produced(read, nothing_unmerged()));
    assert!(after.finish.is_none());
    assert_eq!(summaries(&sent), 0);
    assert!(crate::view::resolver::finish_row(&after).is_some());
}

/// Merge: the finish row and the bar lead with `c`.
#[test]
fn after_esc_from_the_summary_the_finish_row_and_c_first() {
    let state = finish_row_showing();
    assert_eq!(
        crate::view::resolver::finish_row(&state)
            .map(|row| (row.full.as_str().to_owned(), row.short.as_str().to_owned())),
        Some((
            "✓ every file done — c review and finish the merge".to_owned(),
            "✓ every file done — c review & finish".to_owned()
        ))
    );
    assert_eq!(
        super::bar(&state),
        ["c finish", "d start over", "e edit", "esc close"]
    );
    // A finished line lasts until the next key: `Esc` was one, so the bar
    // has the row, and it starts `c finish`.
    assert_eq!(state.status, Status::Ready);
}

/// A click on the finish row does what `c` does.
#[test]
fn click_on_the_finish_row_is_c() {
    let state = finish_row_showing();
    let (_, _, split) =
        crate::view::resolver::resolver_split(&state, crate::update::resolver::banner_rows(&state));
    let (after, commands) = click(&state, split.doc.x + 3, split.doc.y + 1);
    assert!(after.finish.is_some(), "the summary opened");
    assert_eq!(summaries(&commands), 1);
    // The title row above it is not the finish row.
    let (after, commands) = click(&state, split.doc.x + 3, split.doc.y);
    assert!(after.finish.is_none());
    assert_eq!(summaries(&commands), 0);
}

/// A problem notice outranks the finish row.
#[test]
fn a_warning_notice_outranks_the_finish_row() {
    let mut state = finish_row_showing();
    assert!(crate::view::resolver::finish_row(&state).is_some());
    state.resolver.as_mut().expect("open").notice = Some(("take failed".to_owned(), true));
    assert!(crate::view::resolver::finish_row(&state).is_none());
    let doc = crate::view::resolver::resolver_view(&state, 100).expect("drawn");
    assert!(doc.finish.is_none());
    assert_eq!(
        doc.notice
            .map(|(text, problem)| (text.as_str().to_owned(), problem)),
        Some(("take failed".to_owned(), true))
    );
    // A plain notice does not.
    state.resolver.as_mut().expect("open").notice =
        Some(("3 queued takes dropped".to_owned(), false));
    assert!(crate::view::resolver::finish_row(&state).is_some());
}

/// Every `DocView` the resolver builds carries the finish row: the
/// document, a done card, and a whole-file card.
#[test]
fn finish_row_on_a_whole_file_card() {
    let state = finish_row_showing();
    let doc = crate::view::resolver::resolver_view(&state, 100).expect("drawn");
    assert!(doc.finish.is_some(), "the done document");

    let mut done_card = state.clone();
    let open = done_card.resolver.as_mut().expect("open");
    open.file = None;
    open.done = Some(crate::update::resolver::DoneOpen {
        shape: crate::update::resolver::DoneShape::Removed,
        start_over: crate::jobs::conflict::StartOver::Cannot {
            why: "gone".to_owned(),
        },
    });
    let doc = crate::view::resolver::resolver_view(&done_card, 100).expect("drawn");
    assert!(doc.finish.is_some(), "the done card");

    let mut card = super::resolver_card_on_screen(super::WALK);
    card.in_progress = state.in_progress.clone();
    card.working_tree = state.working_tree.clone();
    card.status_read = None;
    let doc = crate::view::resolver::resolver_view(&card, 100).expect("drawn");
    assert!(doc.finish.is_some(), "the whole-file card");
}
