use mergewright_core::capabilities::ColorDepth;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use super::*;
use crate::fit::fit;
use crate::paint::color;
use crate::test_support::terminal;
use crate::theme::{ThemeKind, TokenColor};

#[test]
fn a_failure_reads_mark_space_words() {
    assert_eq!(marked("push refused", false), "⊗ push refused");
    assert_eq!(marked("push refused", true), "(x) push refused");
}

/// Glyphs other marks draw, read as literals because none is a public
/// const: journal `Lost` `✗` (`journal_screen/view.rs`), rewrite `Drop` `✗`
/// and its ASCII `x` (`rewrite_editor/draw.rs:104,120`), `⇢` replaced and
/// deleted, `⚠`/`ⓘ` radar (`graph_screen/render.rs:316,318`), branch-pane
/// `!` `✓` `·`, conflict `?` `◀` `▶` `◆`.
const OTHER_MARKS: [&str; 14] = [
    "✗", "x", "⇢", "->", "⚠", "ⓘ", "!", "✓", "·", "?", "◀", "▶", "◆", "i",
];

#[test]
fn the_mark_is_one_cell_and_no_other_marks_glyph() {
    assert_eq!(FAILED_MARK.width(), 1);
    // The width function `fit` uses: a width-1 cut keeps it whole.
    assert_eq!(fit(FAILED_MARK, 1), FAILED_MARK);
    for other in OTHER_MARKS {
        assert_ne!(FAILED_MARK, other);
        assert_ne!(FAILED_MARK_ASCII, other);
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "must be unmarked")]
fn premarked_words_are_refused_in_debug() {
    let _ = marked("✗ push refused", false);
}

#[test]
fn the_mark_draws_in_error_ink_and_survives_no_color() {
    let caps = terminal(ColorDepth::TrueColor, false);
    let theme = Theme::new(ThemeKind::Dark, &caps);
    let words = Style::default();
    let drawn = spans("push refused", false, &theme, words);
    assert_eq!(drawn[0].content, FAILED_MARK);
    assert_eq!(drawn[0].style.fg, Some(color(theme.token(FAILED_INK))));
    assert_eq!(FAILED_INK, Token::Removed);
    assert_ne!(theme.token(FAILED_INK), TokenColor::Default);
    let plain = theme.colourless();
    let drawn = spans("push refused", false, &plain, words);
    assert_eq!(drawn[0].content, FAILED_MARK, "the glyph still draws");
    assert_eq!(drawn[0].style.fg, Some(color(TokenColor::Default)));
}

#[test]
fn the_mark_ink_differs_from_warning_in_every_theme() {
    let caps = terminal(ColorDepth::TrueColor, false);
    for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
        let theme = Theme::new(kind, &caps);
        assert_ne!(
            theme.token(FAILED_INK),
            theme.token(Token::Warning),
            "{kind:?}: removed {:?} and warning {:?} share a colour",
            theme.token(FAILED_INK),
            theme.token(Token::Warning),
        );
    }
}

/// Every failed body arm (D4 and R2) and the test that renders it at both
/// tiers through [`assert_marked`]: `(arm, test file, test name)`.
const BODY_ARMS: [(&str, &str, &str); 21] = [
    (
        "PickerBody::Failed",
        include_str!("../picker/tests.rs"),
        "a_failed_body_draws_the_failure_mark",
    ),
    (
        "HistoryList::Failed",
        include_str!("body_tests.rs"),
        "failed_trace_and_failed_file_draw_the_failure_mark",
    ),
    (
        "FilePane::Failed",
        include_str!("body_tests.rs"),
        "failed_trace_and_failed_file_draw_the_failure_mark",
    ),
    (
        "BlameBody::Failed",
        include_str!("body_tests.rs"),
        "a_failed_blame_draws_the_failure_mark",
    ),
    (
        "StashStatus::Failed",
        include_str!("body_tests.rs"),
        "a_failed_list_and_a_failed_notice_draw_the_failure_mark",
    ),
    (
        "StashView.notice (failure)",
        include_str!("body_tests.rs"),
        "a_failed_list_and_a_failed_notice_draw_the_failure_mark",
    ),
    (
        "JournalStatus::Failed",
        include_str!("body_tests.rs"),
        "a_failed_read_and_a_row_failure_draw_the_failure_mark",
    ),
    (
        "journal row failure line",
        include_str!("body_tests.rs"),
        "a_failed_read_and_a_row_failure_draw_the_failure_mark",
    ),
    (
        "MapStatus::Failed",
        include_str!("../map_screen/tests.rs"),
        "a_failed_map_draws_the_failure_mark",
    ),
    (
        "LookBody::Failed",
        include_str!("../merge_look/tests.rs"),
        "a_failed_look_draws_the_failure_mark",
    ),
    (
        "PaneLine::Failed",
        include_str!("../branch_pane/tests.rs"),
        "a_failed_line_draws_the_failure_mark",
    ),
    (
        "detail BodyView::Failed",
        include_str!("body_tests.rs"),
        "failed_body_compare_and_contained_in_draw_the_failure_mark",
    ),
    (
        "CompareState::Failed",
        include_str!("body_tests.rs"),
        "failed_body_compare_and_contained_in_draw_the_failure_mark",
    ),
    (
        "ContainedIn::Failed",
        include_str!("body_tests.rs"),
        "failed_body_compare_and_contained_in_draw_the_failure_mark",
    ),
    (
        "CombinedList::Failed",
        include_str!("../conflict_resolver/files/tests/combined.rs"),
        "a_failed_combined_read_draws_the_failure_mark",
    ),
    (
        "status_screen HistoryState::Failed",
        include_str!("../status_screen/tests.rs"),
        "a_failed_history_state_draws_the_failure_mark",
    ),
    (
        "WalkView::Failed",
        include_str!("../graph_screen/tests/problem_line.rs"),
        "a_failed_walk_and_a_failed_search_draw_the_failure_mark",
    ),
    (
        "SearchTail.failed",
        include_str!("../graph_screen/tests/problem_line.rs"),
        "a_failed_walk_and_a_failed_search_draw_the_failure_mark",
    ),
    (
        "AfterView::Failed / ChangesView::Failed",
        include_str!("body_tests.rs"),
        "failed_after_and_failed_changes_draw_the_failure_mark",
    ),
    (
        "commit_editor BodyView::Failed",
        include_str!("../commit_editor/tests.rs"),
        "a_failed_diff_body_draws_the_failure_mark",
    ),
    (
        "finish_summary problem notice (B3)",
        include_str!("body_tests.rs"),
        "a_problem_notice_is_marked",
    ),
];

/// B1 and B2: every arm has its render test, and that test draws the arm at
/// the Unicode and the ASCII tier. The renders themselves live beside each
/// widget's own fixtures; this pins the list, so a new `Failed` body that
/// joins D4 has to join it.
#[test]
fn every_failed_body_draws_the_mark() {
    for (arm, source, name) in BODY_ARMS {
        let at = source
            .find(&format!("fn {name}("))
            .unwrap_or_else(|| panic!("{arm}: no test `{name}`"));
        let body = &source[at..];
        let body = &body[..body.find("\n}\n").unwrap_or(body.len())];
        assert!(
            body.contains("assert_marked") || body.contains("⊗"),
            "{arm}: `{name}` never checks the mark"
        );
        assert!(
            body.contains("ascii") || body.contains("[false, true]"),
            "{arm}: `{name}` never draws the ASCII tier"
        );
    }
}

/// B2, restated: the ASCII helper refuses a `⊗` and a missing `(x) `.
#[test]
#[should_panic(expected = "ASCII tier drew")]
fn every_failed_body_draws_the_ascii_mark() {
    assert_marked("(x) why ⊗ why", "why", true);
}
