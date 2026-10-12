//! What the bottom row says once a `--continue`/`--skip`/`--abort` closed
//! its entry (m6-conflicts/68): the words key on the `Resume` sent **and**
//! the entry's status, because a failed continue closes `Aborted` too.

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};

use super::*;
use crate::jobs::operate::Operated;

/// `done_applied` renamed, re-kinded and re-statused: only `kind`,
/// `description` and `status` are read by the closing words.
fn applied(
    kind: OperationKind,
    description: &str,
    status: EntryStatus,
) -> graft_ops::apply::Applied {
    let mut applied = done_applied();
    applied.entry.kind = kind;
    applied.entry.description = description.to_owned();
    applied.entry.status = status;
    applied
}

fn entry(kind: OperationKind, description: &str, status: EntryStatus) -> JournalEntry {
    applied(kind, description, status).entry
}

/// The closing record around `entry`, nothing came back from the index.
fn with_entry(entry: JournalEntry) -> graft_ops::apply::Applied {
    graft_ops::apply::Applied {
        entry,
        ..done_applied()
    }
}

fn aborted(reason: &str) -> EntryStatus {
    EntryStatus::Aborted {
        reason: reason.to_owned(),
    }
}

/// A rebase Graft began: `c` resumes it (one the shell began is handed to
/// git, 07c).
fn graft_began(mut state: AppState) -> AppState {
    state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    state
}

/// Every key through `update`, the commands kept.
fn keys(state: AppState, codes: &[KeyCode]) -> (AppState, Vec<Command>) {
    let mut state = state;
    let mut commands = Vec::new();
    for code in codes {
        let (next, sent) = crate::update::update(&state, AppEvent::Core(Event::Key(key(*code))));
        state = next;
        commands.extend(sent);
    }
    (state, commands)
}

/// The `Command::Resume` a key sequence sent: its id and what it asked.
fn sent_resume(commands: &[Command]) -> (JobId, Resume) {
    commands
        .iter()
        .find_map(|command| match command {
            Command::Resume(id, what, ..) => Some((*id, what.clone())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no resume was sent: {commands:?}"))
}

fn answer(state: &AppState, id: JobId, outcome: Operated) -> AppState {
    crate::update::update(
        state,
        AppEvent::Produced(id, Product::Resumed(Box::new(outcome))),
    )
    .0
}

/// **The failing test first.** `a y` on a Graft-begun rebase's stop, the
/// abort succeeds: the line says it aborted, and keeps its tick — the abort
/// the user asked for worked.
#[test]
fn aborting_a_rebase_does_not_read_as_done() {
    let (state, commands) = keys(
        stopped(InProgress::Rebase, &[]),
        &[KeyCode::Char('a'), KeyCode::Char('y')],
    );
    let (id, how) = sent_resume(&commands);
    assert_eq!(how, Resume::Abort);
    let state = answer(
        &state,
        id,
        Operated::Done(applied(
            OperationKind::Rebase,
            "rebase main onto fix",
            aborted("aborted"),
        )),
    );
    assert_eq!(state.status, Status::done("aborted: rebase main onto fix"));
    let row = bottom_row(&state, 120, 40);
    assert!(
        row.contains("aborted: rebase main onto fix \u{2713}"),
        "{row:?}"
    );
}

/// The rebase's entry, set aside around (m6-conflicts/74).
fn set_aside_around(mut entry: JournalEntry) -> JournalEntry {
    entry.put_back = Some(graft_core::journal::PutBack {
        stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    entry
}

/// `<what> ✓ · your edits are back`: the fact rides `also`, never `keys`
/// (review m6-final-fixes-74 M4).
fn edits_back(what: &str) -> Status {
    Status::Done {
        what: what.to_owned(),
        also: Some("your edits are back".to_owned()),
        keys: None,
    }
}

/// Review m6-final-fixes-74 M1: `c` finishes a stopped rebase, the
/// put-back ran clean but the drop failed. `graft_ops` says the edits are
/// back (`EditsAt::Back`); the line says so and offers no `S p`.
#[test]
fn a_continue_whose_drop_fails_says_the_edits_are_back() {
    let (state, commands) = keys(
        graft_began(stopped(InProgress::Rebase, &[])),
        &[KeyCode::Char('c')],
    );
    let (id, how) = sent_resume(&commands);
    assert_eq!(how, Resume::Continue);
    let failure = "rebase main onto origin/main: error: cannot lock ref";
    let state = answer(
        &state,
        id,
        Operated::Failed {
            error: failure.to_owned(),
            refresh: Some(graft_ops::apply::Refresh::Everything),
            remote: None,
            refused: None,
            edits: Some(graft_ops::EditsAt::Back { copy: Some(0) }),
        },
    );
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: failure.to_owned(),
            keys: Some("your edits are back \u{b7} a copy stays in stash@{0}".to_owned()),
        }
    );
    let row = bottom_row(&state, 120, 40);
    assert!(row.contains("your edits are back"), "{row:?}");
    assert!(
        !row.contains("S p"),
        "edits in the tree offer no S p: {row:?}"
    );
}

/// m6-conflicts/74: `a y` on a rebase that stopped with edits set aside;
/// the abort and the put-back both went through. 68's aborted wording,
/// extended: `aborted: rebase … ✓ · your edits are back`.
#[test]
fn aborting_a_stopped_rebase_says_your_edits_are_back() {
    let (state, commands) = keys(
        stopped(InProgress::Rebase, &[]),
        &[KeyCode::Char('a'), KeyCode::Char('y')],
    );
    let (id, how) = sent_resume(&commands);
    assert_eq!(how, Resume::Abort);
    let mut done = applied(
        OperationKind::Rebase,
        "rebase main onto origin/main",
        aborted("aborted"),
    );
    done.entry = set_aside_around(done.entry);
    let state = answer(&state, id, Operated::Done(done));
    assert_eq!(
        state.status,
        edits_back("aborted: rebase main onto origin/main")
    );
    let row = bottom_row(&state, 120, 40);
    assert!(
        row.contains("aborted: rebase main onto origin/main \u{2713} \u{b7} your edits are back"),
        "{row:?}"
    );
    // The `c` that finishes it says the same after its own words.
    assert_eq!(
        resumed_words(
            &Resume::Continue,
            &with_entry(set_aside_around(entry(
                OperationKind::Rebase,
                "rebase main onto origin/main",
                EntryStatus::Committed,
            ))),
        ),
        edits_back("rebase main onto origin/main")
    );
}

/// m6-conflicts/74, review 72+72b m8: `c` finishes a stopped rebase and
/// the put-back after it clashes — `resumed`'s `Clashed` arm. The
/// operation is over (no stop), the clash is recorded, and the resolver
/// opens on the clashed file with the set-aside's notice.
///
/// The journal read holds the rebase's `Pending` entry, as the app's does
/// while the rebase is stopped: nothing is launched on `c`, so the notice
/// names the rebase from it (19d-a3: without it no kind is named).
#[test]
fn a_continue_whose_put_back_clashes_lands_on_the_clash() {
    let mut before = graft_began(stopped(InProgress::Rebase, &[]));
    before.stopped = Some(stop(LiveOrigin::ThisRun, false, InProgress::Rebase, 0));
    let rebase = JournalEntry {
        id: graft_core::journal::EntryId::new(1, [0; 10]),
        put_back: Some(graft_core::journal::PutBack {
            stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
            why: graft_core::journal::PutBackWhy::SetAside,
        }),
        ..entry(
            OperationKind::Rebase,
            "rebase feature onto main",
            EntryStatus::Pending,
        )
    };
    before.journal.entries = crate::update::detail::Loading::Ready(std::sync::Arc::new(
        crate::jobs::undo::JournalRead {
            entries: vec![rebase],
            unreadable: Vec::new(),
            held: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        },
    ));
    let (state, commands) = keys(before, &[KeyCode::Char('c')]);
    let (id, how) = sent_resume(&commands);
    assert_eq!(how, Resume::Continue);
    let state = answer(
        &state,
        id,
        Operated::Clashed {
            entry: graft_core::journal::EntryId::new(1, [0; 10]),
            clash: graft_ops::clash::Clash {
                stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
                why: graft_core::journal::PutBackWhy::SetAside,
                paths: vec![std::path::PathBuf::from("notes.md")],
            },
        },
    );
    assert!(state.stopped.is_none(), "the rebase is over");
    assert!(state.clash.is_some(), "the clash is recorded");
    let open = state.resolver.as_ref().expect("landed in the resolver");
    assert_eq!(open.path, std::path::PathBuf::from("notes.md"));
    assert_eq!(
        open.notice.as_ref().map(|(words, _)| words.as_str()),
        Some("your edits clashed in 1 file \u{b7} esc, u undoes the rebase too")
    );
}

/// Review 72+72b m8: `kept_open`'s `Clashed` arm. `resumed` reaches
/// `kept_open` only when the finish summary's refusal test says the answer
/// is a refusal, and `finish::answers::refusal` says `None` for every
/// `Clashed` — a merge (the summary's one kind) sets nothing aside — so
/// the arm is reached here directly: it re-reads everything and leaves the
/// status quiet.
#[test]
fn kept_open_rereads_everything_after_a_clash() {
    let mut state = stopped(InProgress::Rebase, &[]);
    state.status = Status::Busy("continuing".to_owned());
    let mut next = {
        let mut at = 200u64;
        move || {
            at += 1;
            JobId::new(at)
        }
    };
    let commands = crate::update::banner::kept_open(
        &mut state,
        Operated::Clashed {
            entry: graft_core::journal::EntryId::new(1, [0; 10]),
            clash: graft_ops::clash::Clash {
                stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
                why: graft_core::journal::PutBackWhy::SetAside,
                paths: vec![std::path::PathBuf::from("notes.md")],
            },
        },
        &mut next,
    );
    assert_eq!(state.status, Status::Ready);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(..))),
        "{commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadRefs(..))),
        "{commands:?}"
    );
}

/// The table in the ticket, one row each.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one entry per row of the ticket's table is the point"
)]
fn the_resume_words_say_what_happened() {
    let rows: [(Resume, JournalEntry, Status); 8] = [
        (
            Resume::Continue,
            entry(
                OperationKind::Rebase,
                "rebase main onto fix",
                EntryStatus::Committed,
            ),
            Status::done("rebase main onto fix"),
        ),
        (
            Resume::ContinueWith {
                message: b"merge".to_vec(),
            },
            entry(
                OperationKind::Merge,
                "merge feature into main",
                EntryStatus::Committed,
            ),
            Status::done("merge feature into main"),
        ),
        (
            Resume::Skip,
            entry(
                OperationKind::Rebase,
                "rebase main onto fix",
                EntryStatus::Committed,
            ),
            Status::done("rebase main onto fix, skipped a commit"),
        ),
        // Shell-begun merge: its own `AbortMerge` entry already says so.
        (
            Resume::Abort,
            entry(
                OperationKind::AbortMerge,
                "abort merge of feature",
                EntryStatus::Committed,
            ),
            Status::done("abort merge of feature"),
        ),
        (
            Resume::Abort,
            entry(
                OperationKind::CherryPick,
                "cherry-pick abc1234",
                aborted("aborted"),
            ),
            Status::done("aborted: cherry-pick abc1234"),
        ),
        (
            Resume::Continue,
            entry(
                OperationKind::Rebase,
                "rebase main onto fix",
                aborted("git exited 1"),
            ),
            Status::sticky("rebase main onto fix did not finish: git exited 1"),
        ),
        (
            Resume::Skip,
            entry(
                OperationKind::Rebase,
                "rebase main onto fix",
                aborted("git exited 1"),
            ),
            Status::sticky("rebase main onto fix did not finish: git exited 1"),
        ),
        (
            Resume::Continue,
            entry(
                OperationKind::Rebase,
                "rebase main onto fix",
                EntryStatus::Pending,
            ),
            Status::sticky("rebase main onto fix did not finish: still open"),
        ),
    ];
    for (how, entry, want) in rows {
        let got = resumed_words(&how, &with_entry(entry.clone()));
        assert_eq!(got, want, "{how:?} over {:?}", entry.status);
        if !matches!(got, Status::Done { .. }) {
            let row = bottom_row(
                &AppState {
                    status: got,
                    ..AppState::default()
                },
                120,
                40,
            );
            assert!(!row.contains('\u{2713}'), "a failure drew a tick: {row:?}");
        }
    }
}

/// A resume answer that is not the one `resuming` waits for changes nothing.
#[test]
fn a_stale_resume_answer_is_dropped() {
    let (state, commands) = keys(
        stopped(InProgress::Rebase, &[]),
        &[KeyCode::Char('a'), KeyCode::Char('y')],
    );
    let (id, _) = sent_resume(&commands);
    let other_job = JobId::new(id.raw() + 1000);
    let after = answer(
        &state,
        other_job,
        Operated::Done(applied(
            OperationKind::Rebase,
            "rebase main onto fix",
            aborted("aborted"),
        )),
    );
    assert_eq!(after, state, "a stale answer moved the state");
    // `operating` and `resuming` disagreeing (never set apart by `resume`)
    // is dropped the same way: one guard.
    let mut split = state.clone();
    split.resuming = Some((other_job, Resume::Abort));
    let after = answer(
        &split,
        id,
        Operated::Done(applied(
            OperationKind::Rebase,
            "rebase main onto fix",
            aborted("aborted"),
        )),
    );
    assert_eq!(after, split);
}

/// A resume job that dies without answering leaves no `resuming` behind
/// for a later write's answer to be worded by.
#[test]
fn a_resume_that_dies_takes_its_resuming_with_it() {
    let (state, commands) = keys(
        stopped(InProgress::Rebase, &[]),
        &[KeyCode::Char('a'), KeyCode::Char('y')],
    );
    let (id, _) = sent_resume(&commands);
    assert!(state.resuming.is_some());
    let (state, _) = crate::update::update(&state, AppEvent::Core(Event::JobCancelled(id)));
    assert_eq!(state.operating, None);
    assert_eq!(state.resuming, None);
}

/// Ticket 30b: `c` finishes a stopped rebase, and its put-back could not
/// keep the staging (graft-ops `AfterState::StagingNotKept`). The banner
/// closes exactly as on `Done`; the line is the fallback's sticky rung, led
/// by what `resumed_done` puts first — never `could not confirm`.
#[test]
fn a_resume_whose_put_back_lost_staging_is_done_and_says_so() {
    let (state, commands) = keys(
        graft_began(stopped(InProgress::Rebase, &[])),
        &[KeyCode::Char('c')],
    );
    let (id, how) = sent_resume(&commands);
    assert_eq!(how, Resume::Continue);
    let mut done = applied(
        OperationKind::Rebase,
        "rebase main onto origin/main",
        EntryStatus::Committed,
    );
    done.entry = set_aside_around(done.entry);
    let as_done = answer(&state, id, Operated::Done(done.clone()));
    let paths = vec![std::path::PathBuf::from("one.txt")];
    let lost = answer(
        &state,
        id,
        Operated::StagingNotKept {
            applied: done,
            paths: paths.clone(),
        },
    );
    // Closed as on `Done`.
    assert_eq!(lost.in_progress, as_done.in_progress);
    assert_eq!(lost.stopped, as_done.stopped);
    assert!(lost.stopped.is_none() && !lost.banner_focus);
    assert_eq!(lost.land_on_head, as_done.land_on_head);
    // The sticky rung, led by the `Done`'s own words.
    assert_eq!(
        lost.status,
        crate::update::operate::staging_not_kept_status(
            "rebase main onto origin/main",
            &paths,
            &[],
            lost.size.columns,
            &lost.keymap,
        )
    );
    let row = bottom_row(&lost, 120, 40);
    assert!(row.contains("staging not kept: one.txt"), "{row:?}");
    assert!(row.contains("u undoes it, staging included"), "{row:?}");
    assert!(!row.contains("could not confirm"), "{row:?}");
}

/// 21b-4 5h (19d-c hand-back): a paused merge finished from the finish
/// summary keeps `your edits are back` after its own words — the summary's
/// `merged … into …` replaces `what`, never `also`.
#[test]
fn finishing_a_merge_keeps_your_edits_are_back() {
    let mut state = stopped(InProgress::Merge, &[]);
    let mut done = applied(
        OperationKind::Merge,
        "merge feature into main",
        EntryStatus::Committed,
    );
    done.entry = set_aside_around(done.entry);
    let mut n = 0;
    let mut ids = move || {
        n += 1;
        JobId::new(n)
    };
    crate::update::banner::resume::resumed_done(
        &mut state,
        &Resume::Continue,
        &done,
        Some(crate::update::finish::FinishAnswer::Merged(Some(
            "merged feature into main".to_owned(),
        ))),
        &mut ids,
    );
    assert_eq!(state.status, edits_back("merged feature into main"));
}
