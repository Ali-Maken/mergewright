//! `O`/`T` settle the whole file and move on; `j`/`k`/`↓`/`↑` step between
//! files; `n`/`N` from a done file cross (m6-conflicts/17).

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_git::conflict::Take;

use super::advance::{keys, landed, notice, on, sent, three, three_alone};
use super::{WALK, press, resolver_card_on_screen, sticky, takes};
use crate::jobs::conflict::{StartOver, TakeFailure};
use crate::update::resolver::{DoneOpen, DoneShape, file_cursor, select_file};
use crate::update::{AppEvent, AppState, Command, Product, TakeAt, update};

/// How many `Stage` writes `commands` launch.
fn stages(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|command| {
            matches!(command, Command::Operate(_, op, ..)
                if matches!(**op, graft_ops::operation::Operation::Stage(_)))
        })
        .count()
}

/// `commands` less the evil-merge probe every event re-asks for, which is
/// not the key's doing.
fn acting(commands: &[Command]) -> Vec<&Command> {
    commands
        .iter()
        .filter(|command| !matches!(command, Command::EvilProbe(..)))
        .collect()
}

fn settling(state: &AppState) -> Option<Take> {
    state.resolver.as_ref().expect("open").settling
}

fn opened(commands: &[Command]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::OpenConflict(_, path) => Some(path.display().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn shift_t_waits_behind_a_queued_take() {
    let (state, first) = keys(&three_alone(), "o");
    let (state, commands) = press(&state, KeyCode::Char('T'));
    assert_eq!(takes(&commands), 0, "queued, not sent");
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(
        open.queued.iter().copied().collect::<Vec<_>>(),
        [(TakeAt::Every, Take::Theirs)]
    );
    assert_eq!(settling(&state), Some(Take::Theirs));

    let (state, commands) = landed(&state, &sent(&first)[0]);
    assert_eq!(stages(&commands), 0, "no Stage on the first landing");
    let every = sent(&commands);
    assert_eq!(every.len(), 1, "{commands:?}");
    assert_eq!((every[0].1, every[0].2), (TakeAt::Every, Take::Theirs));

    let (state, commands) = landed(&state, &every[0]);
    assert_eq!(stages(&commands), 1, "{commands:?}");
    assert_eq!(settling(&state), None);
}

#[test]
fn shift_t_with_nothing_open_but_a_take_in_flight_stages_after_it() {
    let (state, commands) = keys(&three_alone(), "ttt");
    let (state, second) = landed(&state, &sent(&commands)[0]);
    let (state, third) = landed(&state, &sent(&second)[0]);
    let last = sent(&third);
    assert_eq!(last.len(), 1, "the last open block's take is in flight");
    let (state, commands) = press(&state, KeyCode::Char('T'));
    assert!(acting(&commands).is_empty(), "no entry: {commands:?}");
    assert!(state.resolver.as_ref().expect("open").queued.is_empty());
    assert_eq!(settling(&state), Some(Take::Theirs));
    let (state, commands) = landed(&state, &last[0]);
    assert_eq!(stages(&commands), 1, "{commands:?}");
    assert_eq!(settling(&state), None);
}

#[test]
fn takes_refuse_while_a_file_settles() {
    let (state, _) = press(&three_alone(), KeyCode::Char('T'));
    assert_eq!(settling(&state), Some(Take::Theirs));
    let queued = state.resolver.as_ref().expect("open").queued.clone();
    let plain = |c| KeyPress::plain(KeyCode::Char(c));
    let bulk = |c| KeyPress::control(KeyCode::Char(c));
    let presses = [
        plain('o'),
        plain('t'),
        plain('b'),
        bulk('o'),
        bulk('t'),
        plain('O'),
        plain('T'),
    ];
    for (i, key) in presses.into_iter().enumerate() {
        let (after, commands) = update(&state, AppEvent::Core(Event::Key(key)));
        assert!(acting(&commands).is_empty(), "press {i}: {commands:?}");
        assert_eq!(sticky(&after).0, "still writing the last take", "press {i}");
        assert_eq!(after.resolver.as_ref().expect("open").queued, queued);
    }
}

/// `o` in flight with `T` queued behind it, and the `o` answered `failure`.
fn fail_under_settle(reread: bool) -> (AppState, Vec<Command>) {
    let (state, first) = keys(&three_alone(), "o");
    let (state, _) = press(&state, KeyCode::Char('T'));
    update(
        &state,
        AppEvent::Produced(
            sent(&first)[0].0,
            Product::Taken(Err(TakeFailure {
                words: "no".to_owned(),
                reread,
            })),
        ),
    )
}

#[test]
fn a_changed_file_is_not_staged() {
    let (state, commands) = fail_under_settle(true);
    assert_eq!(stages(&commands), 0);
    assert_eq!(settling(&state), None);
    assert_eq!(
        notice(&state),
        Some("no · 1 queued takes dropped · nothing staged")
    );
}

#[test]
fn a_failed_take_stages_nothing() {
    let (state, commands) = fail_under_settle(false);
    assert_eq!(stages(&commands), 0);
    assert_eq!(takes(&commands), 0);
    assert_eq!(settling(&state), None);
    assert_eq!(
        notice(&state),
        Some("no · 1 queued takes dropped · nothing staged")
    );
}

#[test]
fn shift_t_on_a_done_file_is_refused() {
    for (start_over, words) in [
        (
            StartOver::Can {
                kind: graft_ops::operation::RestartKind::Markers,
            },
            "three.txt is done · d starts it over",
        ),
        (
            StartOver::Cannot {
                why: "no".to_owned(),
            },
            "three.txt is done",
        ),
    ] {
        let mut state = three_alone();
        state.resolver.as_mut().expect("open").done = Some(DoneOpen {
            shape: DoneShape::Removed,
            start_over,
        });
        for key in ['O', 'T'] {
            let (after, commands) = press(&state, KeyCode::Char(key));
            assert!(acting(&commands).is_empty(), "{commands:?}");
            assert_eq!(sticky(&after).0, words);
        }
    }
}

#[test]
fn shift_t_on_a_binary_card_is_the_cards_t() {
    let mut state = resolver_card_on_screen(WALK);
    if let Some(card) = state
        .resolver
        .as_mut()
        .and_then(|open| open.whole_file.as_mut())
    {
        card.binary = true;
    }
    for (shift, plain) in [('T', 't'), ('O', 'o')] {
        let by_shift = press(&state, KeyCode::Char(shift));
        let by_plain = press(&state, KeyCode::Char(plain));
        assert!(
            !acting(&by_plain.1).is_empty(),
            "the card's `{plain}` writes"
        );
        assert_eq!(format!("{:?}", by_shift.1), format!("{:?}", by_plain.1));
        assert_eq!(by_shift.0.resolver, by_plain.0.resolver);
        assert_eq!(by_shift.0.status, by_plain.0.status);
    }
}

const ROWS: [&str; 3] = ["a.rs", "b.rs", "c.rs"];

#[test]
fn j_and_k_move_between_files_without_wrapping() {
    for (down, up) in [
        (KeyCode::Char('j'), KeyCode::Char('k')),
        (KeyCode::Down, KeyCode::Up),
    ] {
        for (row, path) in ROWS.iter().enumerate() {
            let state = on(path, three(), &ROWS, &ROWS);
            assert_eq!(file_cursor(&state), row);
            for (code, target) in [(down, row + 1), (up, row.wrapping_sub(1))] {
                let (after, commands) = press(&state, code);
                if let Some(next) = ROWS.get(target) {
                    assert_eq!(opened(&commands), [*next], "{code:?} from {row}");
                } else {
                    assert!(
                        acting(&commands).is_empty(),
                        "{code:?} at the end: {commands:?}"
                    );
                    assert_eq!(after.resolver, state.resolver, "stays on {row}");
                    assert_eq!(after.status, state.status, "the words unchanged");
                }
            }
        }
    }
}

#[test]
fn j_from_a_done_file_moves() {
    let mut state = on("a.rs", three(), &["a.rs", "b.rs"], &["b.rs"]);
    state.resolver.as_mut().expect("open").done = Some(DoneOpen {
        shape: DoneShape::Removed,
        start_over: StartOver::Can {
            kind: graft_ops::operation::RestartKind::Markers,
        },
    });
    let (_, commands) = press(&state, KeyCode::Char('j'));
    assert_eq!(opened(&commands), ["b.rs"]);
}

#[test]
fn j_while_a_take_is_in_flight_is_refused() {
    let (state, _) = keys(&on("a.rs", three(), &ROWS, &ROWS), "t");
    let (after, commands) = press(&state, KeyCode::Char('j'));
    assert!(acting(&commands).is_empty(), "{commands:?}");
    let mut expected = state.clone();
    let mut sent = Vec::new();
    select_file(&mut expected, &mut sent, 1, &mut || WALK);
    assert!(sent.is_empty());
    assert_eq!(after.status, expected.status);
    assert_eq!(after.resolver, state.resolver);
}

#[test]
fn page_keys_scroll_the_document() {
    let long = (0..200).fold(
        String::from("<<<<<<< HEAD\na\n=======\nb\n>>>>>>> t\n"),
        |mut text, n| {
            text.push_str(&format!("line {n}\n"));
            text
        },
    );
    let file = graft_git::conflict::parse(long.as_bytes()).expect("one block");
    let state = on("long.txt", file, &["long.txt"], &["long.txt"]);
    let top = |state: &AppState| state.resolver.as_ref().expect("open").top;
    let (down, _) = press(&state, KeyCode::PageDown);
    let page = top(&down);
    assert!(page > 1, "a page, not a line: {page}");
    let (back, _) = press(&down, KeyCode::PageUp);
    assert_eq!(top(&back), 0);
    let (bottom, _) = press(&state, KeyCode::Char('G'));
    assert!(top(&bottom) > page, "G to the bottom");
    let (again, _) = press(&bottom, KeyCode::Char('g'));
    assert_eq!(top(&again), 0);
    // One file listed: `j`/`k` have no row to step to, and scroll nothing.
    for code in [
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::Down,
        KeyCode::Up,
    ] {
        let (after, commands) = press(&down, code);
        assert!(acting(&commands).is_empty());
        assert_eq!(top(&after), page, "{code:?}");
    }
}

fn done_on(path: &str, paths: &[&str], unmerged: &[&str]) -> AppState {
    let mut state = on(path, three(), paths, unmerged);
    let open = state.resolver.as_mut().expect("open");
    open.file = Some(Arc::new(
        graft_git::conflict::parse(b"settled\n").expect("plain"),
    ));
    open.cursor_block = None;
    open.done = Some(DoneOpen {
        shape: DoneShape::Text { unstaged: false },
        start_over: StartOver::Can {
            kind: graft_ops::operation::RestartKind::Markers,
        },
    });
    state
}

#[test]
fn n_from_a_done_file_crosses_to_the_next_open_file() {
    let state = done_on("a.rs", &["a.rs", "b.rs"], &["b.rs"]);
    for code in [KeyCode::Char('n'), KeyCode::Char('N')] {
        let (after, commands) = press(&state, code);
        assert_eq!(opened(&commands), ["b.rs"], "{code:?}");
        assert_eq!(notice(&after), Some("▸ b.rs"));
    }
}

/// `n`/`N` on a done file with nothing left to cross to: rule 3's words, as
/// the same info notice a document gets (m6-conflicts/61), the status line
/// untouched.
#[test]
fn n_from_a_done_file_with_nothing_open_says_no_more() {
    use crate::update::resolver::walk::NO_MORE_THIS_WAY;

    let state = done_on("a.rs", &["a.rs", "b.rs"], &[]);
    for code in [KeyCode::Char('n'), KeyCode::Char('N')] {
        let (after, commands) = press(&state, code);
        assert!(acting(&commands).is_empty(), "{commands:?}");
        assert_eq!(
            after.resolver.as_ref().expect("open").notice,
            Some((NO_MORE_THIS_WAY.to_owned(), false)),
            "{code:?}"
        );
        assert_eq!(after.status, state.status, "{code:?}");
        assert_eq!(
            after.resolver.as_ref().expect("open").path,
            state.resolver.as_ref().expect("open").path
        );
    }
}

/// Every file done: 60's finish row takes the notice row and already names
/// the next key, so `n` at the edge draws nothing new — the finish row
/// stands, and `no more conflicts this way` is not drawn.
#[test]
fn the_finish_row_outranks_no_more_this_way() {
    let state = done_on("a.rs", &["a.rs", "b.rs"], &[]);
    let (after, _) = press(&state, KeyCode::Char('n'));
    assert!(
        crate::view::resolver::finish_row(&after).is_some(),
        "every file done: the finish row"
    );
    let frame = super::frame_text(&after);
    assert!(!frame.contains("no more conflicts"), "{frame}");
}
