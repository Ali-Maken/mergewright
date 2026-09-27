//! m7-power-tools/16b: the force-with-lease box names the commits that
//! leave the remote. Read once per lease key `(sent, expected)` in
//! `update`'s settle tail; `P` before it lands says `still reading`.

use std::sync::Arc;

use graft_core::commit::{Identity, Timestamp};
use graft_core::job::JobId;
use graft_git::history_reads::CommitSummary;

use super::super::super::{Leaves, LeavesRead, push_box, push_params, request_leaves};
use super::{
    AppEvent, AppState, Command, Loading, Product, RemoteFacts, SENT, THEIRS, box_title, oid,
    on_sent, press_p, reject_unread, update,
};

/// `origin/main` after a fetch that brought one more commit.
const FETCHED: &str = "3333333333333333333333333333333333333333";

fn commit(hex_digit: char, subject: &str, author: &str) -> CommitSummary {
    CommitSummary {
        id: oid(&hex_digit.to_string().repeat(40)),
        parents: Vec::new(),
        subject: subject.to_owned(),
        author: Identity {
            name: author.as_bytes().to_vec(),
            email: b"a@example.com".to_vec(),
            when: Timestamp {
                seconds: 0,
                offset_seconds: 0,
            },
        },
    }
}

/// The key the read in flight was asked for, and its id.
fn asked(commands: &[Command]) -> Vec<(JobId, String, String)> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::ReadLeaves { id, sent, expected } => {
                Some((*id, sent.to_string(), expected.to_string()))
            }
            _ => None,
        })
        .collect()
}

/// Asks the read the settle tail would ask now, then lands `result` for
/// the key it asked. With no lease key (refs not current, `HEAD` moved)
/// nothing is asked and nothing lands: `P` offers no lease there anyway.
pub(super) fn answer_leaves(state: &mut AppState, result: Result<Leaves, String>) {
    let mut commands = Vec::new();
    request_leaves(state, &mut commands, &mut || JobId::new(900));
    if let Some((id, sent, expected)) = asked(&commands).pop() {
        land(state, id, &sent, &expected, result);
    }
}

fn land(
    state: &mut AppState,
    id: JobId,
    sent: &str,
    expected: &str,
    result: Result<Leaves, String>,
) -> Vec<Command> {
    let (next, commands) = update(
        state,
        AppEvent::Produced(
            id,
            Product::Leaves {
                sent: oid(sent),
                expected: oid(expected),
                result,
            },
        ),
    );
    *state = next;
    commands
}

fn message(state: &AppState) -> Vec<String> {
    state
        .confirm
        .as_ref()
        .unwrap_or_else(|| panic!("no box; status {:?}", state.status))
        .view
        .message
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect()
}

fn rejected_with(result: Result<Leaves, String>) -> AppState {
    let mut state = on_sent();
    reject_unread(&mut state);
    answer_leaves(&mut state, result);
    let (next, _) = press_p(&state);
    next
}

fn two_leave() -> Leaves {
    Leaves {
        newest: vec![
            commit('c', "Fix typo", "Sara"),
            commit('d', "Add retry to the fetch", "Ali"),
        ],
        count: 2,
    }
}

#[test]
fn the_force_box_lists_what_leaves() {
    let state = rejected_with(Ok(two_leave()));
    assert!(box_title(&state).expect("a box").starts_with("Force-push "));
    let lines = message(&state);
    let at = lines
        .iter()
        .position(|line| line == "leaves origin/main:")
        .unwrap_or_else(|| panic!("{lines:#?}"));
    let first = &lines[at + 1];
    let second = &lines[at + 2];
    assert!(first.starts_with("ccccccc  Fix typo "), "{lines:#?}");
    assert!(first.ends_with("Sara"), "{lines:#?}");
    assert!(
        second.starts_with("ddddddd  Add retry to the fetch "),
        "{lines:#?}"
    );
    assert!(second.ends_with("Ali"), "{lines:#?}");
    // Columns: both authors start in the same cell.
    assert_eq!(first.find("Sara"), second.find("Ali"), "{lines:#?}");
    assert_eq!(lines.len(), at + 3, "nothing after the list: {lines:#?}");
}

#[test]
fn the_lease_box_keeps_the_history_warning_above_the_list() {
    let lines = message(&rejected_with(Ok(two_leave())));
    let warning = lines
        .iter()
        .position(|line| line.starts_with("rewrites history for everyone who has 9f8e7d6"))
        .unwrap_or_else(|| panic!("{lines:#?}"));
    assert_eq!(warning, 1, "{lines:#?}");
    assert_eq!(lines[2], "leaves origin/main:", "{lines:#?}");
}

#[test]
fn nothing_leaving_says_so() {
    let lines = message(&rejected_with(Ok(Leaves::default())));
    assert_eq!(lines[2], "nothing on origin/main is lost", "{lines:#?}");
    assert_eq!(lines.len(), 3, "{lines:#?}");
}

#[test]
fn more_than_three_is_cut_with_more() {
    let lines = message(&rejected_with(Ok(Leaves {
        newest: vec![
            commit('a', "five", "Sara"),
            commit('b', "four", "Sara"),
            commit('c', "three", "Sara"),
        ],
        count: 5,
    })));
    assert_eq!(lines[2], "leaves origin/main:", "{lines:#?}");
    assert!(lines[3].starts_with("aaaaaaa  five"), "{lines:#?}");
    assert!(lines[5].starts_with("ccccccc  three"), "{lines:#?}");
    assert_eq!(lines[6], "+2 more", "{lines:#?}");
    assert_eq!(lines.len(), 7, "{lines:#?}");
}

#[test]
fn p_before_the_read_lands_says_still_reading() {
    let mut state = on_sent();
    reject_unread(&mut state);
    let (next, commands) = press_p(&state);
    assert!(next.confirm.is_none(), "no box before the answer");
    let shown = format!("{:?}", next.status);
    assert!(
        shown.contains("still reading what leaves origin/main"),
        "{shown}"
    );
    assert_eq!(asked(&commands).len(), 1, "the settle tail asks the read");
    // Asked, not answered: `P` again still waits.
    let (again, commands) = press_p(&next);
    assert!(again.confirm.is_none());
    assert!(format!("{:?}", again.status).contains("still reading what leaves"));
    assert!(asked(&commands).is_empty(), "one read per key");
}

#[test]
fn a_failed_leaves_read_is_shown_and_force_stays_offered() {
    let state = rejected_with(Err("git rev-list failed".to_owned()));
    assert!(box_title(&state).expect("a box").starts_with("Force-push "));
    let lines = message(&state);
    assert_eq!(
        lines[2], "couldn't read what leaves origin/main: git rev-list failed",
        "{lines:#?}"
    );
}

/// The box is at least 64 cells wide at 80 columns (`confirm_area`,
/// snapped outward): 60 cells of content. Each commit row fits that, the
/// subject cut with `…`, so no row wraps.
#[test]
fn a_long_subject_is_cut_to_the_box() {
    let long = "Rewrite the retry loop so a flaky remote does not stall every fetch for a minute";
    let mut state = on_sent();
    reject_unread(&mut state);
    answer_leaves(
        &mut state,
        Ok(Leaves {
            newest: vec![commit('e', long, "Sara Lindqvist")],
            count: 1,
        }),
    );
    let params = push_params(&state).expect("the lease");
    let read = LeavesRead::Read(Leaves {
        newest: vec![commit('e', long, "Sara Lindqvist")],
        count: 1,
    });
    let view = push_box(&params, Some(1), Some(oid(SENT)), Some(&read));
    let row = view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .find(|line| line.starts_with("eeeeeee"))
        .expect("the commit row");
    assert!(row.contains('…'), "{row}");
    assert!(row.ends_with("Sara Lindqvist"), "{row}");
    assert!(unicode_width::UnicodeWidthStr::width(row) <= 60, "{row}");
    // At 80 columns the box is 64 cells, 60 of content: the whole message
    // draws unwrapped — 2 rows for line 1, 1 each for the rest.
    let rows = graft_ui::confirm::confirm_rows(&view, 64);
    assert_eq!(
        usize::from(rows),
        view.message.len() + 1 + 6,
        "{:#?}",
        view.message
    );
}

fn facts_at(state: &AppState, tip: &str) -> Arc<RemoteFacts> {
    let Loading::Ready(facts) = &state.remotes else {
        panic!("facts held");
    };
    let mut facts = (**facts).clone();
    facts
        .head_upstream
        .as_mut()
        .expect("an upstream")
        .tracking_tip = Some(oid(tip));
    Arc::new(facts)
}

/// `f` lands new remote facts: the settle tail reacts.
fn fetch_moves_tip(state: &mut AppState, tip: &str) -> Vec<Command> {
    let id = JobId::new(700);
    state.remote.reading = Some(id);
    let facts = facts_at(state, tip);
    let (next, commands) = update(state, AppEvent::Produced(id, Product::Remotes(Ok(facts))));
    *state = next;
    commands
}

#[test]
fn a_fetch_that_moves_the_remote_tip_rereads_what_leaves() {
    let mut state = on_sent();
    reject_unread(&mut state);
    answer_leaves(&mut state, Ok(two_leave()));
    let commands = fetch_moves_tip(&mut state, FETCHED);
    let reads = asked(&commands);
    assert_eq!(reads.len(), 1, "{commands:?}");
    assert_eq!((reads[0].1.as_str(), reads[0].2.as_str()), (SENT, FETCHED));
    let (waiting, _) = press_p(&state);
    assert!(format!("{:?}", waiting.status).contains("still reading what leaves"));
    land(
        &mut state,
        reads[0].0,
        SENT,
        FETCHED,
        Ok(Leaves {
            newest: vec![commit('f', "Fetched later", "Kim")],
            count: 1,
        }),
    );
    let (shown, _) = press_p(&state);
    let lines = message(&shown);
    assert!(
        lines[0].contains("3333333"),
        "the lease names the new tip: {lines:#?}"
    );
    assert!(lines[3].starts_with("fffffff  Fetched later"), "{lines:#?}");
}

#[test]
fn a_superseded_leaves_answer_is_dropped() {
    let mut state = on_sent();
    reject_unread(&mut state);
    let mut commands = Vec::new();
    request_leaves(&mut state, &mut commands, &mut || JobId::new(901));
    let (old, _, _) = asked(&commands).pop().expect("a read");
    fetch_moves_tip(&mut state, FETCHED);
    let after = land(&mut state, old, SENT, THEIRS, Ok(two_leave()));
    // Dropped, not stored: storing it would re-key and ask the current
    // key's read a second time.
    assert!(asked(&after).is_empty(), "{after:?}");
    let held = state
        .remote
        .last_rejected_push
        .as_ref()
        .and_then(|rejected| rejected.leaves.clone());
    assert_eq!(held, Some((oid(SENT), oid(FETCHED), LeavesRead::Reading)));
}

#[test]
fn an_unchanged_tip_starts_no_second_read() {
    let mut state = on_sent();
    reject_unread(&mut state);
    answer_leaves(&mut state, Ok(two_leave()));
    let commands = fetch_moves_tip(&mut state, THEIRS);
    assert!(asked(&commands).is_empty(), "{commands:?}");
}
