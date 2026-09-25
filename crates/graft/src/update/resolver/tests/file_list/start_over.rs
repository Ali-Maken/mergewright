//! `d` on a done file is offered exactly where ops can start it over, and
//! the words never promise what ops refuses (m6-conflicts/28).

use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_ops::conflict::DoneFile;
use graft_ops::operation::{OneSided, RestartKind};

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
    let state = open_gone(Ok(text(StartOver::Can {
        kind: graft_ops::operation::RestartKind::Markers,
    })));
    assert_eq!(
        identity_row(&state),
        "▸ gone.txt · done ✓ · d starts it over"
    );
    assert_eq!(bar(&state), ["d start over", "e edit", "esc close"]);
    let (after, _) = press(&state, KeyCode::Char('t'));
    assert_eq!(sticky(&after).0, "gone.txt is done · d starts it over");
    assert!(d_opens_the_box(&state));
}

// ── the box's words by kind (m6-conflicts/58) ──────────────────────────────

const MARKERS: RestartKind = RestartKind::Markers;

fn whole(shape: OneSided) -> RestartKind {
    RestartKind::WholeFile { shape }
}

/// `d` pressed on `gone.txt` done with this gate: the box's title, lines,
/// command, cursor, and what confirming it sends.
fn the_box(opening: Answer) -> (String, Vec<String>, String, usize, ConfirmThen) {
    let state = open_gone(opening);
    let (after, _) = press(&state, KeyCode::Char('d'));
    let confirm = after.confirm.expect("`d` opened the box");
    (
        confirm.view.title.as_str().to_owned(),
        confirm
            .view
            .message
            .iter()
            .map(|line| line.as_str().to_owned())
            .collect(),
        confirm.view.command.as_str().to_owned(),
        confirm.view.cursor,
        confirm.then,
    )
}

/// The marker box, byte for byte as it read before whole files could start
/// over (`d_starts_over_behind_a_box` holds the same words in a live merge).
#[test]
fn restart_box_words_for_markers_unchanged() {
    let (title, message, command, cursor, then) =
        the_box(Ok(text(StartOver::Can { kind: MARKERS })));
    assert_eq!(title, "Start gone.txt over?");
    assert_eq!(
        message,
        [
            "the conflict comes back with all its markers",
            "takes and edits in this file are lost · not undoable while the merge is open",
        ]
    );
    assert_eq!(command, "git checkout --merge -- gone.txt");
    assert_eq!(cursor, 0, "on Cancel");
    assert_eq!(
        then,
        ConfirmThen::RestartConflict {
            path: "gone.txt".into(),
            kind: MARKERS,
        }
    );
}

/// A whole-file choice: the box says the choice comes back, and names the
/// `checkout` of the side that has the file — never `--merge`.
#[test]
fn whole_file_box_names_the_surviving_side() {
    for (shape, flag) in [
        (OneSided::DeletedByThem, "--ours"),
        (OneSided::AddedByUs, "--ours"),
        (OneSided::DeletedByUs, "--theirs"),
        (OneSided::AddedByThem, "--theirs"),
    ] {
        for opening in [text, removed] {
            let (title, message, command, cursor, then) =
                the_box(Ok(opening(StartOver::Can { kind: whole(shape) })));
            assert_eq!(title, "Start gone.txt over?");
            assert_eq!(
                message,
                [
                    "the choice comes back: keep or delete",
                    "edits since the choice are lost · not undoable while the merge is open",
                ],
                "{shape:?}"
            );
            assert_eq!(command, format!("git checkout {flag} -- gone.txt"));
            assert!(!command.contains("--merge"));
            assert_eq!(cursor, 0, "on Cancel");
            assert_eq!(
                then,
                ConfirmThen::RestartConflict {
                    path: "gone.txt".into(),
                    kind: whole(shape),
                }
            );
        }
    }
}

/// No operation open (a conflicted stash pop): the box claims none.
#[test]
fn whole_file_box_with_no_operation_open() {
    let mut state = open_gone(Ok(text(StartOver::Can {
        kind: whole(OneSided::DeletedByThem),
    })));
    state.in_progress = None;
    let (after, _) = press(&state, KeyCode::Char('d'));
    let confirm = after.confirm.expect("`d` opened the box");
    let message: Vec<&str> = confirm
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(
        message,
        [
            "the choice comes back: keep or delete",
            "edits since the choice are lost"
        ]
    );
}

/// A done whole-file file ops can start over — kept (on disk) or deleted
/// (removed) — offers `d` on the bar and the identity row.
#[test]
fn done_whole_file_offers_start_over() {
    let can = || StartOver::Can {
        kind: whole(OneSided::DeletedByThem),
    };
    for state in [open_gone(Ok(text(can()))), open_gone(Ok(removed(can())))] {
        assert!(offers_d(&state), "{:?}", bar(&state));
        assert_eq!(bar(&state)[0], "d start over");
        assert_eq!(
            identity_row(&state),
            "▸ gone.txt · done ✓ · d starts it over"
        );
        assert!(d_opens_the_box(&state));
    }
}

/// The gate is asked before the read, so a failed read still carries it.
#[test]
fn an_unreadable_done_file_keeps_its_gate() {
    let state = open_gone(unreadable(StartOver::Can {
        kind: graft_ops::operation::RestartKind::Markers,
    }));
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
    for (name, start_over) in [
        (
            "can",
            StartOver::Can {
                kind: graft_ops::operation::RestartKind::Markers,
            },
        ),
        ("cannot", cannot(GONE)),
    ] {
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
