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

fn aborted(reason: &str) -> EntryStatus {
    EntryStatus::Aborted {
        reason: reason.to_owned(),
    }
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
            Command::Resume(id, what) => Some((*id, what.clone())),
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
fn set_aside(mut entry: JournalEntry) -> JournalEntry {
    entry.put_back = Some(graft_core::journal::PutBack {
        stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    entry
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
    done.entry = set_aside(done.entry);
    let state = answer(&state, id, Operated::Done(done));
    assert_eq!(
        state.status,
        Status::done_with_keys(
            "aborted: rebase main onto origin/main",
            "your edits are back"
        )
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
            &set_aside(entry(
                OperationKind::Rebase,
                "rebase main onto origin/main",
                EntryStatus::Committed,
            )),
        ),
        Status::done_with_keys("rebase main onto origin/main", "your edits are back")
    );
}

/// m6-conflicts/74, review 72+72b m8: `c` finishes a stopped rebase and
/// the put-back after it clashes — `resumed`'s `Clashed` arm. The
/// operation is over (no stop), the clash is recorded, and the resolver
/// opens on the clashed file with the set-aside's notice.
#[test]
fn a_continue_whose_put_back_clashes_lands_on_the_clash() {
    let mut before = stopped(InProgress::Rebase, &[]);
    before.stopped = Some(stop(LiveOrigin::ThisRun, false, InProgress::Rebase, 0));
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
        let got = resumed_words(&how, &entry);
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
