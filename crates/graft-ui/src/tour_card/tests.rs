use graft_term::sanitize_line;

use super::{TICKS_FROM, TOUR_CARD_ROWS, TourCardView, card_lines, ticks_text};
use crate::frame_box::cells;

fn view() -> TourCardView {
    let names = [
        "stage", "commit", "undo", "stash", "branch", "merge", "conflict", "push",
    ];
    TourCardView {
        step: 3,
        of: 8,
        title: sanitize_line("undo that commit"),
        line: sanitize_line("u takes back the last thing you did \u{2014} nothing is lost"),
        ticks: names
            .iter()
            .enumerate()
            .map(|(at, name)| (sanitize_line(name), at < 2))
            .collect(),
        leave: Some(sanitize_line("esc leave")),
    }
}

#[test]
fn the_card_is_three_rows_each_exactly_the_width() {
    assert_eq!(TOUR_CARD_ROWS, 3);
    for width in [20, 80, 99, 100, 120, 180] {
        for line in card_lines(&view(), width) {
            assert_eq!(cells(&line), usize::from(width), "{width}: {line:?}");
        }
    }
}

#[test]
fn the_top_border_names_the_step_the_task_and_the_way_out() {
    let [top, _, bottom] = card_lines(&view(), 120);
    assert!(
        top.starts_with("\u{256d}\u{2500} practice 3 of 8 \u{b7} undo that commit \u{2500}"),
        "{top:?}"
    );
    assert!(top.ends_with(" esc leave \u{256e}"), "{top:?}");
    assert!(bottom.starts_with('\u{2570}') && bottom.ends_with('\u{256f}'));
}

#[test]
fn the_ticks_are_drawn_from_100_columns_and_dropped_below() {
    let ticks = ticks_text(&view().ticks);
    assert_eq!(
        ticks,
        "\u{2713} stage \u{2713} commit \u{b7} undo stash branch merge conflict push"
    );
    // A short line, so the ticks fit at exactly 100 columns.
    let mut short = view();
    short.line = sanitize_line("u takes back the last thing");
    let [_, wide, _] = card_lines(&short, TICKS_FROM);
    assert!(wide.contains(&ticks), "{wide:?}");
    assert!(wide.starts_with("\u{2502} u takes back"), "{wide:?}");
    let [_, narrow, _] = card_lines(&short, TICKS_FROM - 1);
    assert!(!narrow.contains('\u{2713}'), "{narrow:?}");
    assert!(narrow.contains("u takes back"), "{narrow:?}");
}

/// Wide enough, but the line and the ticks do not both fit: the line wins
/// whole and the ticks are not drawn (never a cut tick list).
#[test]
fn ticks_that_do_not_fit_beside_the_line_are_not_drawn() {
    let [_, middle, _] = card_lines(&view(), TICKS_FROM);
    assert!(!middle.contains('\u{2713}'), "{middle:?}");
    assert!(middle.contains("nothing is lost"), "{middle:?}");
    let [_, roomy, _] = card_lines(&view(), 120);
    assert!(roomy.contains(&ticks_text(&view().ticks)), "{roomy:?}");
}

#[test]
fn a_narrow_card_keeps_the_leave_words_and_cuts_the_title() {
    let [top, _, _] = card_lines(&view(), 40);
    assert!(top.ends_with(" esc leave \u{256e}"), "{top:?}");
    assert!(top.contains('\u{2026}'), "{top:?}");
}

#[test]
fn an_unbound_leave_draws_no_way_out_words() {
    let mut unbound = view();
    unbound.leave = None;
    let [top, _, _] = card_lines(&unbound, 120);
    assert!(!top.contains("leave"), "{top:?}");
    assert!(top.ends_with("\u{2500}\u{256e}"), "{top:?}");
}
