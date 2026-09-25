//! m6-conflicts/49: `P` offers the force-with-lease box only while the tip
//! git refused is still the tip, on refs that answer the last read asked.
//!
//! The rule (`docs/design-notes/m6-usability-fixes.md`, "Decided here" 3):
//! the lease needs (a) a rejection of this same push, (b) current refs and
//! remote facts not being re-read, (c) `HEAD` still at the refused tip.
//! A read in flight when the answer would be the lease → words, no box.
//! Anything else → the plain box: git then decides, and a push that is still
//! not a fast-forward is refused again and asks once more.

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::remote::{Remote, RemoteFailure, TrackedUpstream};
use graft_ops::apply::Refresh;
use graft_ops::operation::{Lease, Operation};

use super::super::push_box::REFS_PENDING;
use super::super::{RejectedPush, push_box, push_params, remote_failed};
use crate::jobs::remote::RemoteFacts;
use crate::refs::RefBadges;
use crate::update::detail::Loading;
use crate::update::events::{Command, Product};
use crate::update::state::{AppState, RefsRead, Status};
use crate::update::{AppEvent, update};

/// `origin/main`'s tip as last fetched.
const THEIRS: &str = "9f8e7d6c5b4a39281706f5e4d3c2b1a098765432";
/// The tip git refused.
const SENT: &str = "1111111111111111111111111111111111111111";
/// The tip after a rebase onto `origin/main` (or a commit, or anything
/// else that moved `HEAD`).
const MOVED: &str = "2222222222222222222222222222222222222222";

const REJECTED: &str = "push rejected: origin/main has moved · p pull first · P force with lease";

fn oid(hex: &str) -> Oid {
    Oid::from_hex(hex).expect("40 hex digits")
}

/// `main` checked out at `tip`, diverged from `origin/main`.
fn refs_on_main(tip: &str) -> RefBadges {
    let main = graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec()).expect("a ref name");
    crate::refs::badges(
        &[(main.clone(), oid(tip))],
        &graft_git::repo::HeadState::Branch(main),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 1,
                behind: 1,
            })
        },
    )
    .expect("no ahead/behind walk can fail here")
}

/// On `main` with an upstream fetched at `THEIRS`, the refs current with
/// `HEAD` at `SENT`.
fn on_sent() -> AppState {
    let mut state = AppState {
        remotes: Loading::Ready(std::sync::Arc::new(RemoteFacts {
            remotes: vec![Remote {
                name: "origin".to_owned(),
                fetch_url: "file:///srv/origin.git".to_owned(),
                push_url: None,
            }],
            head_branch: Some("refs/heads/main".to_owned()),
            head_upstream: Some(TrackedUpstream {
                remote: "origin".to_owned(),
                remote_branch: "refs/heads/main".to_owned(),
                tracking_ref: "refs/remotes/origin/main".to_owned(),
                tracking_tip: Some(oid(THEIRS)),
            }),
        })),
        ..AppState::default()
    };
    land_refs(&mut state, JobId::new(1), SENT);
    state
}

/// A refs read asked under `id` and answered with `HEAD` at `tip`.
fn land_refs(state: &mut AppState, id: JobId, tip: &str) {
    state.refs_read.asked(id);
    answer_refs(state, id, tip);
}

/// The refs answer for `id`, through `update` as the pool delivers it.
fn answer_refs(state: &mut AppState, id: JobId, tip: &str) {
    let (next, _) = update(
        state,
        AppEvent::Produced(id, Product::Refs(Ok(refs_on_main(tip)))),
    );
    *state = next;
}

/// The remote facts re-read a write's refresh asked for land, unchanged.
fn answer_remotes(state: &mut AppState) {
    let id = state.remote.reading.expect("a remote facts read in flight");
    let Loading::Ready(facts) = &state.remotes else {
        panic!("facts held: {:?}", state.remotes);
    };
    let facts = facts.clone();
    let (next, _) = update(state, AppEvent::Produced(id, Product::Remotes(Ok(facts))));
    *state = next;
}

/// git refused the push `P` would build now, as not a fast-forward.
fn reject(state: &mut AppState) {
    let params = push_params(state).expect("an upstream to push to");
    let status = remote_failed(
        state,
        &Operation::Push(params),
        &RemoteFailure::NonFastForward,
        REJECTED,
    );
    state.status = status;
}

fn press_p(state: &AppState) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('P')))),
    )
}

/// The title of the box `P` opened, or why none opened.
fn box_title(state: &AppState) -> Result<String, String> {
    state
        .confirm
        .as_ref()
        .map(|confirm| confirm.view.title.as_str().to_owned())
        .ok_or_else(|| format!("no box; status {:?}", state.status))
}

fn assert_plain(state: &AppState, why: &str) {
    let params = push_params(state).unwrap_or_else(|words| panic!("{why}: {words}"));
    assert_eq!(params.lease, None, "{why}");
    let view = push_box(&params, Some(1), Some(oid(MOVED)));
    assert!(
        view.title.as_str().starts_with("Push "),
        "{why}: {}",
        view.title.as_str()
    );
    assert!(
        !view.command.as_str().contains("--force"),
        "{why}: {}",
        view.command.as_str()
    );
    let (next, commands) = press_p(state);
    let title = box_title(&next).unwrap_or_else(|words| panic!("{why}: {words}"));
    assert!(title.starts_with("Push "), "{why}: {title}");
    assert!(commands.is_empty(), "{why}: P only opens the box");
}

/// The refs job a write asked for ends without an answer — it failed (a
/// panic, a stopped runner) or was cancelled. The refs are then `Unknown`,
/// not still being read: `P` gives the plain box, never the wait words
/// for good (review 47-49, M2).
#[test]
fn a_refs_read_that_fails_or_is_cancelled_gives_the_plain_box() {
    let ends: [fn(JobId) -> Event; 2] = [
        |id| {
            Event::JobFailed(
                id,
                graft_core::job::JobFailure::Failed("runner stopped".to_owned()),
            )
        },
        Event::JobCancelled,
    ];
    for end in ends {
        let mut state = on_sent();
        reject(&mut state);
        let refs = write_done(&mut state);
        let (next, _) = update(&state, AppEvent::Core(end(refs)));
        state = next;
        assert_eq!(state.refs_read, RefsRead::Unknown, "{:?}", end(refs));
        assert_plain(&state, "the refs read ended without an answer");
    }
}

fn assert_lease(state: &AppState, why: &str) {
    let params = push_params(state).unwrap_or_else(|words| panic!("{why}: {words}"));
    assert_eq!(
        params.lease,
        Some(Lease {
            expected: oid(THEIRS)
        }),
        "{why}"
    );
    let (next, _) = press_p(state);
    let title = box_title(&next).unwrap_or_else(|words| panic!("{why}: {words}"));
    assert!(title.starts_with("Force-push "), "{why}: {title}");
}

/// A write's `Done` whose refresh re-reads everything: the id its refs read
/// was asked under.
fn write_done(state: &mut AppState) -> JobId {
    let id = JobId::new(40);
    state.operating = Some(id);
    let (next, commands) = update(
        state,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(crate::jobs::operate::Operated::Done(applied(
                "rebase main onto origin/main",
            )))),
        ),
    );
    *state = next;
    assert_eq!(state.operating, None, "Done clears the write");
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadRefs(id) => Some(*id),
            _ => None,
        })
        .expect("a Done on Everything re-reads the refs")
}

fn applied(description: &str) -> graft_ops::apply::Applied {
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };
    graft_ops::apply::Applied {
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Rebase,
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
        refresh: Refresh::Everything,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    }
}

/// The review's M4: rejected, then `p` · `r r` moved `HEAD` onto the
/// teammate's work. A plain push now fast-forwards; the lease would be a
/// force-push nobody needs.
#[test]
fn a_rebase_after_a_rejected_push_gets_the_plain_box() {
    let mut state = on_sent();
    reject(&mut state);
    land_refs(&mut state, JobId::new(2), MOVED);
    assert_eq!(state.refs_read, RefsRead::Current(JobId::new(2)));
    assert_plain(&state, "HEAD moved after the rejection");
}

/// Between a write's `Done` and its refs answer the refs still name the
/// old `HEAD`: `P` says so in words and opens nothing. Once the answer
/// lands, the plain box.
#[test]
fn p_between_a_writes_done_and_its_refs_read_says_so() {
    let mut state = on_sent();
    reject(&mut state);
    let refs = write_done(&mut state);
    let (next, commands) = press_p(&state);
    assert!(next.confirm.is_none(), "{:?}", next.confirm);
    assert_eq!(next.status, Status::Waiting(REFS_PENDING.to_owned()));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "{commands:?}"
    );
    answer_refs(&mut state, refs, MOVED);
    assert_plain(&state, "the rebase's refs landed");
}

/// Every transition of `RefsRead::{asked, answered}`.
#[test]
fn refs_read_follows_ask_and_answer() {
    let (one, two) = (JobId::new(1), JobId::new(2));
    let asked = [
        (RefsRead::Unknown, RefsRead::Pending(two)),
        (RefsRead::Pending(one), RefsRead::Pending(two)),
        (RefsRead::Current(one), RefsRead::Pending(two)),
    ];
    for (from, to) in asked {
        let mut read = from;
        read.asked(two);
        assert_eq!(read, to, "asked from {from:?}");
    }
    let answered = [
        // The read asked answers.
        (RefsRead::Pending(two), two, true, RefsRead::Current(two)),
        (RefsRead::Pending(two), two, false, RefsRead::Unknown),
        // An older read answers while a newer one is pending: still waiting.
        (RefsRead::Pending(two), one, true, RefsRead::Pending(two)),
        (RefsRead::Pending(two), one, false, RefsRead::Pending(two)),
        // An older answer lands over the current one: it overwrote it.
        (RefsRead::Current(two), one, true, RefsRead::Unknown),
        // An older read failing leaves the current answer alone.
        (RefsRead::Current(two), one, false, RefsRead::Current(two)),
        (RefsRead::Unknown, one, true, RefsRead::Unknown),
        (RefsRead::Unknown, one, false, RefsRead::Unknown),
    ];
    for (from, id, ok, to) in answered {
        let mut read = from;
        read.answered(id, ok);
        assert_eq!(read, to, "{from:?} answered by {id:?} ok={ok}");
    }
}

/// `took_fact` stores every refs answer; one from an older read landing
/// after the current one leaves refs nobody can vouch for → the plain box.
#[test]
fn an_older_refs_answer_after_the_current_one_counts_as_unknown() {
    let mut state = on_sent();
    reject(&mut state);
    state.refs_read.asked(JobId::new(2));
    state.refs_read.asked(JobId::new(3));
    answer_refs(&mut state, JobId::new(3), SENT);
    assert_eq!(state.refs_read, RefsRead::Current(JobId::new(3)));
    // The older read lands last, still naming the refused tip.
    answer_refs(&mut state, JobId::new(2), SENT);
    assert_eq!(state.refs_read, RefsRead::Unknown);
    assert_plain(&state, "an older answer overwrote the current one");
}

/// A refs read that failed is not in flight any more: no wait words, and
/// the refs held are from before it → the plain box.
#[test]
fn a_failed_refs_read_gives_the_plain_box_not_the_wait_words() {
    let mut state = on_sent();
    reject(&mut state);
    let refs = write_done(&mut state);
    let (next, _) = update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Err("refs unreadable".to_owned()))),
    );
    state = next;
    assert_eq!(state.refs_read, RefsRead::Unknown);
    assert_plain(&state, "the refs read failed");
}

/// Scenario 10's fast-tier twin: nothing moved between the rejection and
/// the next `P`. Once the rejection's own refresh has landed, the lease.
/// While the remote facts are re-read it says so, as `p` and `f` do.
#[test]
fn a_rejected_push_with_the_tip_unchanged_still_offers_the_lease() {
    let mut state = on_sent();
    reject(&mut state);
    assert_eq!(
        state
            .remote
            .last_rejected_push
            .as_ref()
            .and_then(|last| last.sent),
        Some(oid(SENT))
    );
    // The rejection's refresh: refs and remote facts both re-read.
    state.remote.reading = Some(JobId::new(5));
    state.refs_read.asked(JobId::new(4));
    assert_eq!(
        press_p(&state).0.status,
        Status::Waiting(REFS_PENDING.to_owned())
    );
    answer_refs(&mut state, JobId::new(4), SENT);
    let (next, _) = press_p(&state);
    assert!(next.confirm.is_none(), "{:?}", next.confirm);
    assert_eq!(
        next.status,
        Status::Waiting("still reading the remotes — try again in a moment".to_owned())
    );
    answer_remotes(&mut state);
    assert_lease(&state, "the tip git refused is still the tip");
}

/// A rejection recorded while the refs were not current has no tip to
/// compare with (`sent: None`) → the plain box, however the refs settle.
#[test]
fn a_rejection_recorded_without_current_refs_gets_the_plain_box() {
    for (at_rejection, label) in [
        (RefsRead::Unknown, "Unknown"),
        (RefsRead::Pending(JobId::new(9)), "Pending"),
    ] {
        let mut state = on_sent();
        state.refs_read = at_rejection;
        reject(&mut state);
        assert_eq!(
            state
                .remote
                .last_rejected_push
                .as_ref()
                .map(|last| last.sent),
            Some(None),
            "{label}"
        );
        land_refs(&mut state, JobId::new(10), SENT);
        assert_plain(&state, label);
    }
    // No refs at all: nothing to compare either.
    let mut state = on_sent();
    reject(&mut state);
    state.refs = None;
    assert_plain(&state, "no refs");
}

/// A commit on top of the refused tip is still not a fast-forward, but
/// the rule offers the lease only on the tip git refused: the plain box,
/// and git refuses it again if it must, which asks once more.
#[test]
fn a_commit_after_a_rejected_push_gets_the_plain_box() {
    let mut state = on_sent();
    reject(&mut state);
    let refs = write_done(&mut state);
    answer_refs(&mut state, refs, MOVED);
    assert_plain(&state, "a commit moved HEAD forward");
}

/// `u` back to the refused tip: the remote still has what that tip lacks,
/// so the lease is right again.
#[test]
fn undo_back_to_the_rejected_tip_offers_the_lease_again() {
    let mut state = on_sent();
    reject(&mut state);
    let refs = write_done(&mut state);
    answer_refs(&mut state, refs, MOVED);
    assert_plain(&state, "moved");
    let refs = write_done(&mut state);
    answer_refs(&mut state, refs, SENT);
    answer_remotes(&mut state);
    assert_lease(&state, "undone back to the refused tip");
}

/// `P` and the `.` menu's `Push (P)` both build through `push_params`:
/// after a move, both open the plain box.
#[test]
fn every_push_params_caller_gets_the_plain_box_after_a_move() {
    let mut state = on_sent();
    reject(&mut state);
    let refs = write_done(&mut state);
    answer_refs(&mut state, refs, MOVED);

    let (by_key, _) = press_p(&state);
    let mut by_menu = state.clone();
    let mut ids = 100u64;
    let commands = crate::update::context_menu::apply_context_action(
        &mut by_menu,
        graft_ui::context_menu::ContextAction::Push,
        0,
        None,
        &[],
        &mut || {
            ids += 1;
            JobId::new(ids)
        },
    );
    assert!(commands.is_empty(), "{commands:?}");
    for (caller, after) in [("P", &by_key), (". menu Push", &by_menu)] {
        let title = box_title(after).unwrap_or_else(|words| panic!("{caller}: {words}"));
        assert!(title.starts_with("Push "), "{caller}: {title}");
        assert!(
            matches!(
                &after.confirm.as_ref().expect("a box").then,
                crate::update::confirm::ConfirmThen::Operate(Operation::Push(params))
                    if params.lease.is_none()
            ),
            "{caller}"
        );
    }
}

/// The sticky `… · P force with lease` must not outlive the rebase that
/// made it false: the rebase's own line replaces it.
#[test]
fn the_rejection_line_is_gone_after_the_rebase_lands() {
    let mut state = on_sent();
    reject(&mut state);
    assert!(
        matches!(&state.status, Status::Sticky { keys: Some(keys), .. }
        if keys.contains("P force with lease"))
    );
    let refs = write_done(&mut state);
    answer_refs(&mut state, refs, MOVED);
    let shown = format!("{:?}", state.status);
    assert!(!shown.contains("force with lease"), "{shown}");
    assert!(!matches!(state.status, Status::Sticky { .. }), "{shown}");
}

/// `RejectedPush` holds exactly what `P` compares: the push, and the tip.
#[test]
fn a_rejection_records_its_params_and_the_tip_sent() {
    let mut state = on_sent();
    let params = push_params(&state).expect("an upstream");
    reject(&mut state);
    assert_eq!(
        state.remote.last_rejected_push,
        Some(RejectedPush {
            params,
            sent: Some(oid(SENT)),
        })
    );
}
