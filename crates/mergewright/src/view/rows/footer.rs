//! The graph footer's lines: search, compare, and the order notice.

use mergewright_term::{SanitizedText, sanitize_line};
use mergewright_ui::graph_screen::{OrderNotice, SHA_WIDTH, SearchLine, SearchTail, short_sha};

use crate::history::{History, Promotion, WalkFacts};
use crate::update::AppState;

/// The footer's search line: the prompt while it is open, and nothing once
/// `⏎` closes it — the search then stands in the header
/// ([`search_tail`], m8/15b). The query came from the keyboard and the
/// line is drawn, so it is sanitised like repository text.
pub(super) fn search_line(state: &AppState) -> Option<SearchLine> {
    state
        .search_prompt
        .as_ref()
        .map(|typed| SearchLine::Prompt(sanitize_line(typed)))
}

/// The header's search item: the standing search's line, worded as the
/// footer drew it before 15b, while its prompt is closed. `Esc` ends the
/// search (`update::keys::escape`), and with it this.
pub(crate) fn search_tail(state: &AppState) -> Option<SearchTail> {
    if state.search_prompt.is_some() {
        return None;
    }
    let search = state.search.as_ref()?;
    Some(SearchTail {
        text: sanitize_line(&search.line(&state.keymap)),
        failed: matches!(search.state, crate::search::SearchState::Failed(_)),
    })
}

/// The footer's compare-pin line, worded exactly as the design note's
/// "Compare (`x`)" bullet (m2-details-and-diffs/11): shown while a compare
/// base is pinned, in the same footer slot as [`search_line`]'s line.
pub(super) fn compare_footer_line(state: &AppState) -> Option<SanitizedText> {
    let base = state.compare_base?;
    Some(sanitize_line(&format!(
        "compare base {} — move, x again clears",
        short_sha(&base.oid.to_string(), usize::from(SHA_WIDTH))
    )))
}

/// What the header says about the order the rows are in (ADR-0008): nothing
/// once they are in the order asked for; while commit-date order stands in
/// for a topological one, that it is being sorted — or, once the sort is
/// stopped or has failed, that the order stays as it is.
pub(crate) fn order_notice(history: &History) -> Option<OrderNotice> {
    let provisional = history.walk.as_ref().is_some_and(WalkFacts::is_provisional);
    if !provisional {
        return None;
    }
    Some(match history.promotion {
        Some(Promotion::Kept) => OrderNotice::Provisional,
        Some(Promotion::Sorting | Promotion::Swapping { .. }) | None => OrderNotice::Sorting,
    })
}
