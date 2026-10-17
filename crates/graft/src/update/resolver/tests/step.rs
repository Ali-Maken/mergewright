//! `←`/`→` (`h`/`l`): the previous / next clash **in the file on screen**,
//! open or taken, clamped at its ends (ticket ease-pass/05r, review
//! ease-pass-05 M1); and what `n`/`N` say once no clash is open anywhere.

use std::path::Path;

use graft_core::event::KeyCode;
use graft_git::conflict::{ConflictFile, Take};

use super::advance::{cursor, landed, notice, on, open_path, sent, three, three_alone};
use super::file_keys::acting;
use super::{WALK, press, resolver_card_on_screen, sticky};
use crate::update::AppState;
use crate::update::resolver::{BlockAt, step_in_file};

const LEFT: KeyCode = KeyCode::Left;
const RIGHT: KeyCode = KeyCode::Right;

/// What `n`/`N` say with nothing open anywhere and a taken clash here.
const NONE_OPEN_TAKEN: &str = "no open clashes left \u{b7} \u{2190}/\u{2192} step through them";

fn at(index: usize, open: bool) -> BlockAt {
    BlockAt { index, open }
}

/// `three.txt`, every block taken, the cursor on `cursor`, in a list of
/// `paths` where only `three.txt` is unmerged.
fn three_taken(paths: &[&str], cursor: usize) -> AppState {
    three_taken_among(paths, &["three.txt"], cursor)
}

/// [`three_taken`] in a list where `unmerged` are still unmerged.
fn three_taken_among(paths: &[&str], unmerged: &[&str], cursor: usize) -> AppState {
    let mut file = three();
    for block in 0..3 {
        file.take(block, Take::Ours).expect("the block");
    }
    let mut state = on("three.txt", file, paths, unmerged);
    state.resolver.as_mut().expect("open").cursor_block = Some(cursor);
    state
}

/// A document with no conflict blocks at all.
fn no_blocks() -> AppState {
    let file: ConflictFile = graft_git::conflict::parse(b"plain\n").expect("plain");
    let mut state = on("plain.txt", file, &["plain.txt"], &["plain.txt"]);
    state.resolver.as_mut().expect("open").cursor_block = None;
    state
}

fn walked(state: &AppState, codes: &[KeyCode]) -> AppState {
    codes
        .iter()
        .fold(state.clone(), |state, code| press(&state, *code).0)
}

// ------------------------------------------------------------ step_in_file

#[test]
fn step_in_file_takes_every_block_and_stops_at_the_edges() {
    let mixed = [at(0, false), at(1, true), at(2, false)];
    assert_eq!(step_in_file(&mixed, 0, true), Some(1));
    assert_eq!(step_in_file(&mixed, 1, true), Some(2), "a taken one too");
    assert_eq!(step_in_file(&mixed, 2, true), None, "past the last");
    assert_eq!(step_in_file(&mixed, 2, false), Some(1));
    assert_eq!(step_in_file(&mixed, 1, false), Some(0), "a taken one too");
    assert_eq!(step_in_file(&mixed, 0, false), None, "before the first");

    let all_open = [at(0, true), at(1, true)];
    assert_eq!(step_in_file(&all_open, 0, true), Some(1));
    assert_eq!(step_in_file(&all_open, 1, false), Some(0));
    let all_taken = [at(0, false), at(1, false)];
    assert_eq!(step_in_file(&all_taken, 0, true), Some(1));
    assert_eq!(step_in_file(&all_taken, 1, false), Some(0));

    let one = [at(0, false)];
    assert_eq!(step_in_file(&one, 0, true), None);
    assert_eq!(step_in_file(&one, 0, false), None);
    assert_eq!(step_in_file(&[], 0, true), None);
    assert_eq!(step_in_file(&[], 0, false), None);
}

// ------------------------------------------------------------- the keys

/// Every clash taken: `→`/`l` walk down through them, `←`/`h` back up, and
/// at either end the cursor stays and the notice says which end.
#[test]
fn the_arrows_reach_taken_clashes_and_stop_at_the_ends() {
    for (back, on_) in [(LEFT, RIGHT), (KeyCode::Char('h'), KeyCode::Char('l'))] {
        let state = three_taken(&["three.txt"], 0);
        let down = walked(&state, &[on_]);
        assert_eq!(cursor(&down), Some(1), "{on_:?}");
        assert_eq!(notice(&down), None, "a step clears the notice");
        let last = walked(&down, &[on_]);
        assert_eq!(cursor(&last), Some(2));
        let past = walked(&last, &[on_]);
        assert_eq!(cursor(&past), Some(2), "clamped");
        assert_eq!(notice(&past), Some("last clash in three.txt"));

        let up = walked(&past, &[back, back]);
        assert_eq!(cursor(&up), Some(0), "{back:?}");
        let before = walked(&up, &[back]);
        assert_eq!(cursor(&before), Some(0), "clamped");
        assert_eq!(notice(&before), Some("first clash in three.txt"));
    }
}

/// Clamped to the file: at its last clash `→` neither crosses to the next
/// file nor wraps, and sends nothing.
#[test]
fn the_arrows_never_leave_the_file() {
    // `z.txt` is unmerged too: `n` or `N` would cross to it.
    let paths = ["three.txt", "z.txt"];
    let state = three_taken_among(&paths, &paths, 2);
    let (after, commands) = press(&state, RIGHT);
    assert!(acting(&commands).is_empty(), "{commands:?}");
    assert_eq!(
        (open_path(&after), cursor(&after)),
        (Path::new("three.txt"), Some(2))
    );
    let first = walked(&after, &[LEFT, LEFT]);
    let (after, commands) = press(&first, LEFT);
    assert!(acting(&commands).is_empty(), "{commands:?}");
    assert_eq!(
        (open_path(&after), cursor(&after)),
        (Path::new("three.txt"), Some(0))
    );
}

/// A step lands as `N` lands: on the block, scrolled into view — in a
/// window too short for the whole document, the same `top` `N` gives
/// landing on that block.
#[test]
fn a_step_scrolls_its_clash_into_view() {
    let mut state = three_taken(&["three.txt"], 1);
    state.size.rows = 12;
    let top = |state: &AppState| state.resolver.as_ref().expect("open").top;
    let stepped = walked(&state, &[RIGHT]);
    // `N` from outside the file's blocks lands on its last.
    state.resolver.as_mut().expect("open").cursor_block = None;
    let by_n = walked(&state, &[KeyCode::Char('N')]);
    assert_eq!((cursor(&stepped), cursor(&by_n)), (Some(2), Some(2)));
    assert!(top(&stepped) > 0, "scrolled: top {}", top(&stepped));
    assert_eq!(top(&stepped), top(&by_n), "the window `N` gives");
}

/// A whole-file card has no clashes to step through: said, nothing moves.
#[test]
fn the_arrows_on_a_card_say_there_are_no_clashes() {
    let state = resolver_card_on_screen(WALK);
    for code in [LEFT, RIGHT, KeyCode::Char('h'), KeyCode::Char('l')] {
        let (after, commands) = press(&state, code);
        assert!(acting(&commands).is_empty(), "{code:?}: {commands:?}");
        assert_eq!(
            notice(&after),
            Some("gone.txt has no clashes to step through"),
            "{code:?}"
        );
    }
}

/// A document with no blocks: the same words.
#[test]
fn the_arrows_on_a_file_with_no_clashes_say_so() {
    let (after, _) = press(&no_blocks(), RIGHT);
    assert_eq!(
        notice(&after),
        Some("plain.txt has no clashes to step through")
    );
}

/// A file marked done in a rebase is read-only: the arrows get the done
/// file's words, as every other key there does.
#[test]
fn the_arrows_on_a_done_file_in_a_rebase_get_the_done_words() {
    let mut state = super::file_keys::done_on("a.rs", &["a.rs", "b.rs"], &["b.rs"]);
    state.in_progress = Some(crate::update::resolver::fixtures::operation(
        graft_git::state::InProgress::Rebase,
        None,
    ));
    for code in [LEFT, RIGHT, KeyCode::Char('h'), KeyCode::Char('l')] {
        let (after, commands) = press(&state, code);
        assert!(acting(&commands).is_empty(), "{code:?}");
        assert_eq!(
            sticky(&after),
            ("a.rs is done \u{b7} d starts it over", None)
        );
    }
}

/// While a take is in flight the arrows walk the document as it will stand
/// once it lands — exactly what `N` walks then.
#[test]
fn the_arrows_while_a_take_runs_walk_what_n_walks() {
    let (taking, commands) = press(&three_alone(), KeyCode::Char('o'));
    assert_eq!(sent(&commands).len(), 1, "a take in flight");
    assert_eq!(cursor(&taking), Some(1));
    let (by_arrow, sent_arrow) = press(&taking, LEFT);
    let (by_n, _) = press(&taking, KeyCode::Char('N'));
    assert!(acting(&sent_arrow).is_empty());
    assert_eq!(cursor(&by_arrow), cursor(&by_n));
    assert_eq!(cursor(&by_arrow), Some(0), "onto the block being taken");
    // The take lands under it; the cursor stays where the step put it.
    let (after, _) = landed(&by_arrow, &sent(&commands)[0]);
    assert_eq!(cursor(&after), Some(0));
}

// ------------------------------------------- `n`/`N` with nothing open

/// `N` before the first clash, nothing open anywhere, the file's clashes
/// all taken: not `no more conflicts this way` — they are there — but that
/// none is open, and the keys that reach them.
#[test]
fn n_and_shift_n_with_every_clash_taken_point_to_the_arrows() {
    let (back, _) = press(&three_taken(&["three.txt"], 0), KeyCode::Char('N'));
    assert_eq!(cursor(&back), Some(0), "unmoved");
    assert_eq!(notice(&back), Some(NONE_OPEN_TAKEN));
    let (on_, _) = press(&three_taken(&["three.txt"], 2), KeyCode::Char('n'));
    assert_eq!(cursor(&on_), Some(2), "unmoved");
    assert_eq!(notice(&on_), Some(NONE_OPEN_TAKEN));
}

/// With no clash in the file at all there is nothing to step through, so
/// the words stop at `no open clashes left`.
#[test]
fn n_and_shift_n_with_no_clashes_say_only_none_open() {
    for code in [KeyCode::Char('n'), KeyCode::Char('N')] {
        let (after, _) = press(&no_blocks(), code);
        assert_eq!(notice(&after), Some("no open clashes left"), "{code:?}");
    }
}
