//! The blame place's bar words (design note pass 2b B1), m7-power-tools/20c
//! (moved out of `hints/tests.rs`, ease-pass/12).

use super::*;

fn words(list: &[Hint]) -> Vec<String> {
    list.iter()
        .filter_map(|hint| hint.text(Keymap::shipped()))
        .collect()
}

#[test]
fn blame_bar_words_in_order() {
    assert_eq!(
        words(&blame_place_list(true, true)),
        [
            "⏎ that commit",
            "^ before it",
            "L history of these lines",
            "/ find",
            "n/N match",
            "space ages",
            "w whitespace",
            "i ignore list",
            "esc back",
        ]
    );
}

#[test]
fn n_n_only_while_finding() {
    assert!(!words(&blame_place_list(false, true)).contains(&"n/N match".to_owned()));
    assert!(words(&blame_place_list(true, false)).contains(&"n/N match".to_owned()));
}

#[test]
fn i_only_with_the_file() {
    assert!(!words(&blame_place_list(true, false)).contains(&"i ignore list".to_owned()));
    assert!(words(&blame_place_list(false, true)).contains(&"i ignore list".to_owned()));
}

/// m7-power-tools/20k, 20n: without lines (a read in flight, or a
/// failed one) the line keys go.
#[test]
fn a_place_without_lines_draws_no_line_keys() {
    let reading = HintContext::Blame {
        finding: true,
        ignore_file: true,
        lines: false,
    };
    assert_eq!(
        words(&lists::context_list(&reading)),
        [
            "/ find",
            "space ages",
            "w whitespace",
            "i ignore list",
            "esc back"
        ]
    );
}
