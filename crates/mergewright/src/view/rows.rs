//! Building [`GraphView`]: the rows the walk last sent, folded and painted
//! for this frame.

use mergewright_term::sanitize_line;
use mergewright_ui::graph_screen::hints::RowVerbs;
use mergewright_ui::graph_screen::{
    BadgeKind, Compaction, Filters, GraphEntry, GraphView, HintContext, Location, ProblemLine,
    WalkView, spaced,
};

use crate::history::{History, Phase};
use crate::update::AppState;

use super::entries::{thread, visible_rows};

pub(crate) mod clash_bars;
mod context;
mod fold;
pub(crate) use clash_bars::banner_context;
pub use clash_bars::resolver_context;
pub(crate) use context::back_to_place;
use context::{cursor_row_context, live_wrapped, search_wrapped};
pub(super) use fold::fold_tail;
mod footer;
mod header;
mod hint_context;
mod window;
use footer::{compare_footer_line, search_line};
pub(crate) use footer::{order_notice, search_tail};
pub(crate) use header::home_relative;
use header::{filters, mark_stash_only, modal_blanked, rows_for_frame};
pub(super) use header::{location, practised, preview_drawn};
pub(super) use hint_context::keep_for;
use hint_context::live_c;
pub(crate) use hint_context::{
    box_context, footer_context, pane_hint_context, place_context, stopped_line,
};
pub(crate) use window::divider_shown;
pub use window::max_lanes;
use window::{compaction, reveal_index};

/// Everything the graph screen draws: the rows in the viewport, the cursor
/// among them, and where the walk is.
///
/// The rows are borrowed from the window the walk last sent; only the
/// subjects, which came out of the repository, are copied — sanitised.
///
/// **The one place a drawn `GraphView` is built** (m5-history-and-ghost-preview/33,
/// review M1). Everything that decides where a cell lands is settled here:
/// the badge glyphs ([`AppState::icons`]), the lane cap ([`max_lanes`], which
/// widens or narrows the graph column) and the harness banner (which pushes
/// the rows down). `view::frame` adds only fields that repaint what is
/// already placed — the tier's glyph set, the footer notice, the pixel
/// rectangle — because those need the `Session` and move nothing.
///
/// That matters because [`super::hit::badge_index_at`] calls this function
/// too, to say which badge a mouse cell belongs to: when the frame overrode
/// the icons and the lane cap *after* building the view, a drag with Nerd
/// icons or past eight lanes landed on the wrong badge.
#[must_use]
pub fn graph_view(state: &AppState, now_seconds: i64) -> GraphView<'_> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let location = state
        .repository
        .as_ref()
        .map(|facts| practised(location(facts, home.as_deref()), state));
    // An open modal owns the bottom row: its own refusal is the only problem,
    // and the search, compare and stop rungs are blanked (`modal_blanked`).
    // `reveal` is not: under a box `draw_box_bars` writes the whole bar over
    // the footer, and under the preview `rows_for_frame` reveals nothing, so
    // the footer's subject never shows under a modal while the hovered row's
    // refs keep their reveal (m6-conflicts/69, Decided 1).
    let problem = super::status::bottom_row_problem(state);
    let Some(history) = &state.history else {
        return opening_view(state, location, problem);
    };

    let thread = thread(history);
    let mut live = visible_rows(state, history, thread.as_ref(), now_seconds);
    mark_stash_only(state, &mut live);
    let compaction = compaction(state, history, thread.as_ref(), &live);
    let banner_op = crate::update::banner::banner_view(state);
    let (rows, reveal) = rows_for_frame(state, history, live, now_seconds);
    // The footer follows the *box*, not the slice: while the plan is in
    // flight the rows are empty but the box has the keyboard, and a footer
    // offering `m merge  r rebase` names two keys it swallows (review 08 m10).
    let hint_context = if state.preview.is_some() {
        HintContext::Preview
    } else {
        footer_context(state, &rows, banner_op.as_ref())
    };

    GraphView {
        location,
        problem,
        walk: walk_view(state, history),
        rows,
        banner_op,
        pending: crate::update::banner::pending_view(state),
        tour: super::tour::card(state),
        icons: state.icons,
        keys: &state.keymap,
        compaction,
        filters: filters(state, history),
        reveal,
        order: order_notice(history),
        ascii: false,
        pixel: false,
        graphics: None,
        push: crate::update::commit_editor::push_words(state, false)
            .map(|words| mergewright_term::sanitize_line(&words)),
        notice: None,
        status_row: crate::update::commit_editor::done_row(state),
        search: modal_blanked(state, search_line),
        search_tail: modal_blanked(state, search_tail),
        compare_footer: modal_blanked(state, compare_footer_line),
        stopped: modal_blanked(state, stopped_line),
        // Its `B`/`X` are swallowed by every modal, or typed into a
        // password (m6-conflicts/78, Minor 3).
        commit_graph: modal_blanked(state, AppState::commit_graph_line),
        banner: super::frame::banner_lines(state),
        divider_shown: divider_shown(state),
        spaced: spaced(state.size.rows),
        hint_context,
        // `visible_rows` (just above) has already set `radar_column_seen`
        // for any row it built with a marker, so reading it here also
        // covers this frame's own findings, not just earlier ones
        // (`AppState::radar_column_seen`, m2-details-and-diffs/13b).
        // Before the context is `Ready` the only marker a row can carry is
        // a found evil merge — its probe reads no context — so that one
        // opens the column too (m7-power-tools/33c).
        marker_column: (matches!(
            state.radar.context,
            crate::update::detail::Loading::Ready(_)
        ) || matches!(
            state.detail.evil,
            crate::update::detail::Loading::Ready(
                mergewright_ops::mistake_radar::probes::EvilVerdict::Evil { .. }
            )
        )) && state.radar_column_seen.get(),
    }
}

/// The view before any history has arrived: the header says so, and there is
/// nothing else to show.
fn opening_view(
    state: &AppState,
    location: Option<Location>,
    problem: Option<ProblemLine>,
) -> GraphView<'_> {
    GraphView {
        location,
        problem,
        walk: WalkView::Opening,
        rows: Vec::new(),
        banner_op: None,
        pending: None,
        tour: super::tour::card(state),
        icons: state.icons,
        keys: &state.keymap,
        compaction: Compaction::at(max_lanes(state)),
        filters: Filters::default(),
        reveal: None,
        order: None,
        ascii: false,
        pixel: false,
        graphics: None,
        push: None,
        notice: None,
        status_row: crate::update::commit_editor::done_row(state),
        search: modal_blanked(state, search_line),
        search_tail: modal_blanked(state, search_tail),
        compare_footer: modal_blanked(state, compare_footer_line),
        stopped: modal_blanked(state, stopped_line),
        commit_graph: modal_blanked(state, AppState::commit_graph_line),
        banner: super::frame::banner_lines(state),
        divider_shown: divider_shown(state),
        spaced: spaced(state.size.rows),
        // Wrapped like every graph-row bar: an operation can be live
        // before the first rows land (m6-conflicts/52).
        hint_context: live_wrapped(
            state,
            HintContext::PlainRow {
                verbs: RowVerbs::NONE,
            },
        ),
        // No history has arrived yet, so the radar context is never `Ready`
        // here — `state.history` is `None`, which `radar::request_context`
        // requires before it asks for one. Sticky findings from an earlier
        // repository never apply: a fresh `AppState` is fresh state.
        marker_column: matches!(
            state.radar.context,
            crate::update::detail::Loading::Ready(_)
        ) && state.radar_column_seen.get(),
    }
}

/// The header's words for where the walk is. Reading (or jumping) exactly
/// when `update::walk_is_reading` says so — the one predicate `Esc` reads
/// too, so the header never says reading while `Esc` would do nothing, nor
/// rests while `Esc` would stop it (m6-conflicts/85 A1).
fn walk_view(state: &AppState, history: &History) -> WalkView {
    let reading = crate::update::walk_is_reading(state);
    match &history.phase {
        Phase::Streaming if reading && history.jump.is_some() => WalkView::Jumping {
            laid_out: history.rows,
        },
        Phase::Streaming if reading => WalkView::Reading {
            laid_out: history.rows,
        },
        Phase::Streaming => WalkView::Paused {
            laid_out: history.rows,
        },
        Phase::Complete => WalkView::Complete { rows: history.rows },
        Phase::Cancelled => WalkView::Cancelled {
            laid_out: history.rows,
        },
        Phase::Failed(reason) => WalkView::Failed(sanitize_line(reason)),
    }
}

#[cfg(test)]
mod tests;
