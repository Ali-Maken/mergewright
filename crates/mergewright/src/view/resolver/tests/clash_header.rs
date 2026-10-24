//! The clash screen's top row and the list's `Combined by git` section
//! through the whole frame (ease-pass/11b), over `clash_look`'s stop:
//! `sara/fix-toc` merged into `wallet-release`.

use mergewright_git::state::InProgress;
use mergewright_ui::keymap::Keymap;

use super::clash_look::{merge, row_with, stopped};
use super::frame;
use crate::update::banner::banner_view;
use crate::update::detail::Loading;

const MERGING: &str = "merging sara/fix-toc into wallet-release";

/// M5: the top row names both branches in a merge, at every width the
/// list is a column.
#[test]
fn a_merge_header_says_merging_from_into() {
    for columns in [100u16, 120, 200] {
        let drawn = frame(&merge(), columns, 40);
        let top = drawn.lines().next().unwrap_or_default();
        assert_eq!(top.trim_end(), MERGING, "{columns}");
    }
}

/// Any other operation's header is its banner's words, unchanged.
#[test]
fn a_rebase_header_is_its_banner_words() {
    let state = stopped(InProgress::Rebase);
    let banner = banner_view(&state).expect("a rebase is live");
    let want = mergewright_ui::graph_screen::banner_line(&banner, 120, Keymap::shipped());
    let drawn = frame(&state, 120, 40);
    let top = drawn.lines().next().unwrap_or_default();
    assert_eq!(top.trim_end(), want);
    assert!(!top.contains("merging"), "{top}");
}

/// The column ends `Combined by git (2)` and both names, under the files.
#[test]
fn the_list_shows_what_git_combined() {
    let drawn = frame(&merge(), 120, 40);
    let heading = row_with(&drawn, "Combined by git");
    assert!(heading.starts_with("Combined by git (2) "), "{heading}");
    let names = row_with(&drawn, "refund.rs");
    assert!(names.starts_with("✓ refund.rs, locale.rs "), "{names}");
}

/// Reading and failed draw their words; nothing combined draws nothing.
#[test]
fn the_section_says_reading_failed_or_nothing() {
    let with = |combined: Loading<std::sync::Arc<Vec<std::path::PathBuf>>>| {
        let mut state = merge();
        state.resolver.as_mut().expect("open").combined = combined;
        frame(&state, 120, 40)
    };
    let reading = with(Loading::Idle);
    assert!(row_with(&reading, "Combined by git").starts_with("Combined by git · reading…"));
    let failed = with(Loading::Failed("index locked".to_owned()));
    assert!(row_with(&failed, "Combined by git").starts_with("Combined by git · can't tell:"));
    row_with(&failed, "⊗ index locked");
    let none = with(Loading::Ready(std::sync::Arc::new(Vec::new())));
    assert!(!none.contains("Combined"), "{none}");
}

/// Q12 a: a rebase draws no section, whatever was read.
#[test]
fn a_rebase_draws_no_section() {
    let mut state = stopped(InProgress::Rebase);
    state.resolver.as_mut().expect("open").combined = Loading::Idle;
    let drawn = frame(&state, 120, 40);
    assert!(!drawn.contains("Combined"), "{drawn}");
}
