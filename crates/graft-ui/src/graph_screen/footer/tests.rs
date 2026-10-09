//! The stop's rung is the last one (m6-conflicts/12, scope item 3): the
//! rows above it in `footer_line` still win the whole row.

use graft_term::sanitize_line;

use crate::graph_screen::tests::{draw, linear, text, view};
use crate::graph_screen::{HintContext, ProblemLine, SearchLine, StopWording, WalkView};

fn last_row(view: &crate::graph_screen::GraphView<'_>) -> String {
    text(&draw(view, 120, 8)).pop().expect("eight rows")
}

#[test]
fn a_problem_still_wins_over_a_stop() {
    let rows = linear(3);
    let mut shown = view(&rows, WalkView::Complete { rows: 3 }, 0);
    shown.stopped = Some(StopWording::new(
        sanitize_line("merge stopped · 1 file conflicted"),
        Vec::new(),
    ));
    // The focused banner's short list, which leaves the reason room.
    shown.hint_context = HintContext::Banner {
        skippable: false,
        conflicted: true,
        merge: false,
    };
    assert!(
        last_row(&shown).starts_with(" merge stopped · 1 file conflicted ·"),
        "the control: with nothing above it the stop leads the row: {}",
        last_row(&shown)
    );
    shown.problem = Some(ProblemLine {
        reason: sanitize_line("a hook refused the commit"),
        keys: Some(sanitize_line("a abort")),
    });
    let row = last_row(&shown);
    assert!(
        row.starts_with(" a hook refused the commit · a abort"),
        "{row}"
    );
    assert!(!row.contains("merge stopped"), "{row}");
}

/// m8/15b (15 A2; replaces `a_search_line_still_wins_over_a_stop`):
/// while the `/` field is open it is the row he is typing in.
#[test]
fn the_search_prompt_still_wins_over_a_stop() {
    let rows = linear(3);
    let mut shown = view(&rows, WalkView::Complete { rows: 3 }, 0);
    shown.stopped = Some(StopWording::new(
        sanitize_line("merge stopped · 1 file conflicted"),
        Vec::new(),
    ));
    shown.search = Some(SearchLine::Prompt(sanitize_line("fix")));
    let row = last_row(&shown);
    assert!(row.starts_with(" /fix_"), "{row}");
    assert!(!row.contains("merge stopped"), "{row}");
}

/// m8/15b (15 A2): after `⏎` the search leaves the footer. The stop's
/// row is drawn — its reason, then the stop's own keys, `n N` after —
/// and the search's line is in the header's tail.
#[test]
fn after_enter_a_stop_row_is_drawn_and_the_search_moves_to_the_header() {
    use crate::graph_screen::hints::RowVerbs;
    use crate::graph_screen::{LiveC, SearchTail};
    let rows = linear(3);
    let mut shown = view(&rows, WalkView::Complete { rows: 3 }, 0);
    shown.stopped = Some(StopWording::new(
        sanitize_line("merge stopped · 1 file conflicted"),
        Vec::new(),
    ));
    let search = "/fix  2 matches (1/2)  n/N next/prev  esc close";
    shown.search = None;
    shown.search_tail = Some(SearchTail {
        text: sanitize_line(search),
        failed: false,
    });
    shown.hint_context = HintContext::OperationLive {
        row: Box::new(HintContext::Searching(Box::new(HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        }))),
        c: LiveC::Finish,
        way_to_banner: None,
        space_stages: false,
    };
    let drawn = text(&draw(&shown, 120, 8));
    let row = drawn.last().expect("eight rows");
    // The stop's row as `stopped_row` draws it beside a laddered row
    // bar: the reason leads where the bar leaves it twelve cells, else
    // the bar alone (m6-conflicts/12) — either way the stop's, not the
    // search's.
    assert!(
        row.starts_with(" merge stopped") || row.starts_with(" c finish"),
        "{row}"
    );
    let finish = row.find("c finish").unwrap_or_else(|| panic!("{row}"));
    let next = row.find("n next").unwrap_or_else(|| panic!("{row}"));
    let prev = row.find("N prev").unwrap_or_else(|| panic!("{row}"));
    assert!(finish < next && next < prev, "the stop's keys lead: {row}");
    assert!(!row.contains("/fix"), "{row}");
    assert!(drawn[0].contains(search), "header: {:?}", drawn[0]);
}

fn words(line: crate::graph_screen::CommitGraphLine, width: usize) -> String {
    super::commit_graph_segments(line, width, crate::keymap::Keymap::shipped())
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

/// m7-power-tools/36 (M3): off the graph `B` is blame and `X` nothing,
/// so the row keeps its fact and drops the keys.
#[test]
fn the_offer_row_drops_its_keys_off_the_graph() {
    use crate::graph_screen::CommitGraphLine;
    for width in [200, 120, 100, 80, 40] {
        let without = super::commit_graph_segments(
            CommitGraphLine::Offered { keys: false },
            width,
            crate::keymap::Keymap::shipped(),
        );
        assert!(without.iter().all(|(_, key)| !key), "{width}: {without:?}");
        let text = words(CommitGraphLine::Offered { keys: false }, width);
        assert!(
            !text.contains('B') && !text.contains('X'),
            "{width}: {text}"
        );
    }
    assert_eq!(
        words(CommitGraphLine::Offered { keys: false }, 120),
        " no commit-graph file: history sorts on every open"
    );
    // With the keys: byte-identical to the words before 36.
    assert_eq!(
        words(CommitGraphLine::Offered { keys: true }, 120),
        " no commit-graph file: history sorts on every open · B build one (git commit-graph \
         write --reachable) · X never here"
    );
    assert_eq!(
        words(CommitGraphLine::Offered { keys: true }, 80),
        " no commit-graph file · B build one · X never here"
    );
}
