//! `d` on a done file is offered exactly where ops can start it over, and
//! the words never promise what ops refuses (m6-conflicts/28).

use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_ops::conflict::DoneFile;

use super::super::{bar, frame_text, press, resolver_card_on_screen, resolver_on_screen, sticky};
use super::{identity_row, open_gone};
use crate::jobs::conflict::{OpenFailure, Opening, StartOver};
use crate::update::confirm::ConfirmThen;
use crate::update::{AppState, Status};

/// The opening read's answer, as the job sends it.
type Answer = Result<Opening, OpenFailure>;

/// One shape the resolver opens a done file as, for a given gate.
type Shape = fn(StartOver) -> Answer;

const GONE: &str = "gone.txt: deleted by them — no markers to take from";

fn cannot(why: &str) -> StartOver {
    StartOver::Cannot {
        why: why.to_owned(),
    }
}

fn text(start_over: StartOver) -> Opening {
    let file = graft_git::conflict::parse(b"our plan\n").expect("plain text parses");
    Opening::Done(
        Arc::new(DoneFile::Text {
            file,
            unstaged: false,
        }),
        start_over,
    )
}

fn removed(start_over: StartOver) -> Opening {
    Opening::Done(Arc::new(DoneFile::Removed), start_over)
}

fn binary(start_over: StartOver) -> Opening {
    Opening::Done(Arc::new(DoneFile::Binary { unstaged: false }), start_over)
}

fn unreadable(start_over: StartOver) -> Answer {
    Err(OpenFailure {
        words: "could not read gone.txt: denied".to_owned(),
        not_unmerged: false,
        done: Some(start_over),
    })
}

/// `d` pressed: whether it opened the restart box.
fn d_opens_the_box(state: &AppState) -> bool {
    let (after, _) = press(state, KeyCode::Char('d'));
    matches!(
        after.confirm.as_ref().map(|box_| &box_.then),
        Some(ConfirmThen::RestartConflict { .. })
    )
}

fn offers_d(state: &AppState) -> bool {
    bar(state).iter().any(|hint| hint.starts_with("d "))
}

#[test]
fn a_done_file_that_cannot_start_over_hides_d() {
    let state = open_gone(Ok(text(cannot(GONE))));
    assert_eq!(identity_row(&state), "▸ gone.txt · done ✓");
    assert_eq!(bar(&state), ["e edit", "esc close"]);

    let (after, commands) = press(&state, KeyCode::Char('d'));
    assert_eq!(sticky(&after).0, format!("can't start over · {GONE}"));
    assert!(after.confirm.is_none(), "no box");
    assert!(commands.is_empty(), "nothing sent: {commands:?}");

    let (after, _) = press(&state, KeyCode::Char('t'));
    assert_eq!(sticky(&after).0, "gone.txt is done");
}

#[test]
fn a_done_file_that_can_start_over_offers_d() {
    let state = open_gone(Ok(text(StartOver::Can)));
    assert_eq!(
        identity_row(&state),
        "▸ gone.txt · done ✓ · d starts it over"
    );
    assert_eq!(bar(&state), ["d start over", "e edit", "esc close"]);
    let (after, _) = press(&state, KeyCode::Char('t'));
    assert_eq!(sticky(&after).0, "gone.txt is done · d starts it over");
    assert!(d_opens_the_box(&state));
}

/// The gate is asked before the read, so a failed read still carries it.
#[test]
fn an_unreadable_done_file_keeps_its_gate() {
    let state = open_gone(unreadable(StartOver::Can));
    assert!(frame_text(&state).contains("d starts it over"));
    assert!(offers_d(&state));

    let state = open_gone(unreadable(cannot(GONE)));
    let drawn = frame_text(&state);
    assert!(!drawn.contains("d starts it over"), "{drawn}");
    assert!(!offers_d(&state));
}

/// Rule 3: a question that could not be asked is never a yes.
#[test]
fn a_gate_that_could_not_be_read_hides_d() {
    let why = "could not check: the undo journal is unreadable: journal line 3";
    let state = open_gone(Ok(text(cannot(why))));
    assert!(!offers_d(&state));
    assert_eq!(identity_row(&state), "▸ gone.txt · done ✓");
    let (after, _) = press(&state, KeyCode::Char('d'));
    assert_eq!(sticky(&after).0, format!("can't start over · {why}"));
    assert!(after.confirm.is_none());
}

/// **The guard against a third repeat** (m6-05 M1, m6-amend-25 M1): over
/// every shape the resolver opens, the bar offers `d` exactly when `d`
/// opens the box.
#[test]
fn d_is_in_the_bar_exactly_when_the_box_opens() {
    let walk = crate::update::JobId::new(4_100);
    let mut fixtures: Vec<(String, AppState)> = vec![
        ("document".to_owned(), resolver_on_screen(walk)),
        (
            "unmerged whole-file card".to_owned(),
            resolver_card_on_screen(walk),
        ),
    ];
    for (name, start_over) in [("can", StartOver::Can), ("cannot", cannot(GONE))] {
        let shapes: [(&str, Shape); 4] = [
            ("done text", |to| Ok(text(to))),
            ("removed", |to| Ok(removed(to))),
            ("binary", |to| Ok(binary(to))),
            ("unreadable", unreadable),
        ];
        for (shape, answer) in shapes {
            fixtures.push((
                format!("{shape}, {name}"),
                open_gone(answer(start_over.clone())),
            ));
        }
    }
    for (name, state) in &fixtures {
        assert!(
            !matches!(state.status, Status::Sticky { .. }),
            "{name}: a clean start"
        );
        assert_eq!(
            offers_d(state),
            d_opens_the_box(state),
            "{name}: the bar {:?}",
            bar(state)
        );
    }
    let offered = fixtures.iter().filter(|(_, state)| offers_d(state)).count();
    assert_eq!(offered, 5, "the document and the four `can` shapes");
}
