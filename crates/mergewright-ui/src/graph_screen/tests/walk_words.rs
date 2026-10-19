//! The header's walk words: resting, reading and failing read apart, and
//! only a failure reads as one. Split out of `graph_screen::tests` for size.

use mergewright_term::sanitize_line;

use super::super::WalkView;
use super::super::walk_text::walk_text;
use crate::theme::Token;

/// A walk resting between requests reads as rows so far; one working on an
/// ask reads as reading. The two never share words, so the header can say
/// when `Esc` has something to stop (m6-conflicts/85 A1).
#[test]
fn a_paused_walk_and_a_reading_one_read_apart() {
    let (paused, _) = walk_text(&WalkView::Paused { laid_out: 500 });
    let (reading, _) = walk_text(&WalkView::Reading { laid_out: 500 });
    assert_eq!(paused, "500 rows so far");
    assert_eq!(reading, "reading\u{2026} 500 rows");
}

#[test]
fn a_walk_that_is_fine_never_reads_as_an_error() {
    for walk in [
        WalkView::Opening,
        WalkView::Reading { laid_out: 0 },
        WalkView::Reading { laid_out: 1000 },
        WalkView::Paused { laid_out: 1000 },
        WalkView::Complete { rows: 0 },
        WalkView::Cancelled { laid_out: 500 },
    ] {
        let (words, token) = walk_text(&walk);
        assert!(
            !words.contains("fail") && !words.contains("error"),
            "{words}"
        );
        assert_ne!(token, Token::Warning, "{walk:?}");
    }
    let (words, token) = walk_text(&WalkView::Failed(sanitize_line("bad object")));
    assert!(words.contains("bad object"));
    assert_eq!(token, Token::Warning);
}
