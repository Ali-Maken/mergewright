//! Building [`GraphView`]: the rows the walk last sent, folded and painted
//! for this frame.

use graft_graph::Ancestry;
use graft_graph::fold::Keep;
use graft_graph::row::Row;
use graft_term::{SanitizedText, sanitize_line, sanitize_path};
use graft_ui::graph_column::RowPaint;
use graft_ui::graph_screen::{
    BadgeKind, Compaction, Filters, GraphEntry, GraphView, HintContext, LiveC, OrderNotice,
    ProblemLine, SHA_WIDTH, SearchLine, TabWord, WalkView, short_sha,
};
use std::collections::BTreeSet;

use crate::history::{History, Phase, Promotion, WalkFacts};
use crate::update::{AppState, GraphOverlay};

use super::entries::{Thread, thread, uncommitted_shown, visible_rows};

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
/// too, to say which badge a mouse cell belongs to. When the frame overrode
/// the icons and the lane cap *after* building the view, the hit-test
/// measured against a view nothing ever drew, and with Nerd icons or past
/// eight lanes a drag landed on the wrong badge.
#[must_use]
pub fn graph_view(state: &AppState, now_seconds: i64) -> GraphView<'_> {
    let location = state
        .repository
        .as_ref()
        .map(|facts| sanitize_path(&facts.path));
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

    let walk = walk_view(state, history);
    let thread = thread(history);
    let live = visible_rows(state, history, thread.as_ref(), now_seconds);
    let compaction = compaction(state, history, thread.as_ref(), &live);
    let banner_op = crate::update::banner::banner_view(state);
    let (rows, reveal) = rows_for_frame(state, history, live, now_seconds);
    // The footer follows the *box*, not the slice. While the plan is still
    // in flight the rows are empty and the graph keeps what it had, but the
    // box already has the keyboard — a footer offering `m merge  r rebase`
    // there names two keys the modal box swallows (review 08 Minor 10).
    let hint_context = if state.preview.is_some() {
        HintContext::Preview
    } else {
        footer_context(state, &rows, banner_op.as_ref())
    };

    GraphView {
        location,
        problem,
        walk,
        rows,
        banner_op,
        icons: state.icons,
        compaction,
        filters: Filters {
            first_parent: history.view.follow == graft_graph::Follow::FirstParent,
            folded: history.view.inclusion.folded.len(),
            stash_hidden: !history.view.inclusion.hidden_stashes.is_empty(),
            focused: state.focus,
        },
        reveal,
        order: order_notice(history),
        ascii: false,
        pixel: false,
        graphics: None,
        notice: None,
        search: modal_blanked(state, search_line),
        compare_footer: modal_blanked(state, compare_footer_line),
        stopped: modal_blanked(state, stopped_line),
        // Its `B`/`X` are swallowed by every modal, or typed into a
        // password (m6-conflicts/78, Minor 3).
        commit_graph: modal_blanked(state, AppState::commit_graph_line),
        banner: super::frame::banner_lines(state),
        divider_shown: divider_shown(state),
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
                graft_ops::mistake_radar::probes::EvilVerdict::Evil { .. }
            )
        )) && state.radar_column_seen.get(),
    }
}

/// A footer rung, or `None` while a modal owns the bottom row
/// (`AppState::modal_owns_bottom_row`): the modal's keys stand there, not
/// the graph's search, compare or stop line.
fn modal_blanked<'a, T>(
    state: &'a AppState,
    rung: impl FnOnce(&'a AppState) -> Option<T>,
) -> Option<T> {
    if state.modal_owns_bottom_row() {
        None
    } else {
        rung(state)
    }
}

/// The rows this frame draws, and which of them reveals its refs on hover.
///
/// A preview is modal: while one is open the graph draws its slice instead
/// of the live window, so the picture the box describes and the rows on
/// screen are the same picture (design note hard choice 1). There is no
/// hovered row over a picture, and the choice cursor lives in the box, so
/// nothing is revealed — the selected commit is painted by
/// `preview_entries` itself.
fn rows_for_frame<'a>(
    state: &'a AppState,
    history: &'a History,
    live: Vec<GraphEntry<'a>>,
    now_seconds: i64,
) -> (Vec<GraphEntry<'a>>, Option<usize>) {
    if let Some(preview) = state
        .preview
        .as_ref()
        .filter(|preview| !preview.rows.is_empty())
    {
        return (
            super::preview::preview_entries(state, preview, now_seconds),
            None,
        );
    }
    let reveal = reveal_index(state, history, &live);
    (live, reveal)
}

/// The view before any history has arrived: the header says so, and there is
/// nothing else to show.
fn opening_view(
    state: &AppState,
    location: Option<SanitizedText>,
    problem: Option<ProblemLine>,
) -> GraphView<'_> {
    GraphView {
        location,
        problem,
        walk: WalkView::Opening,
        rows: Vec::new(),
        banner_op: None,
        icons: state.icons,
        compaction: Compaction::at(max_lanes(state)),
        filters: Filters::default(),
        reveal: None,
        order: None,
        ascii: false,
        pixel: false,
        graphics: None,
        notice: None,
        search: modal_blanked(state, search_line),
        compare_footer: modal_blanked(state, compare_footer_line),
        stopped: modal_blanked(state, stopped_line),
        commit_graph: modal_blanked(state, AppState::commit_graph_line),
        banner: super::frame::banner_lines(state),
        divider_shown: divider_shown(state),
        // Wrapped like every graph-row bar: an operation can be live
        // before the first rows land (m6-conflicts/52).
        hint_context: live_wrapped(state, HintContext::PlainRow),
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

/// The footer's search line: the prompt while it is open, else the search's
/// state. The query came from the keyboard and the line is drawn, so it is
/// sanitised like repository text.
fn search_line(state: &AppState) -> Option<SearchLine> {
    if let Some(typed) = &state.search_prompt {
        return Some(SearchLine::Prompt(sanitize_line(typed)));
    }
    let search = state.search.as_ref()?;
    Some(SearchLine::Status {
        text: sanitize_line(&search.line()),
        failed: matches!(search.state, crate::search::SearchState::Failed(_)),
    })
}

/// The footer's compare-pin line, worded exactly as the design note's
/// "Compare (`x`)" bullet (m2-details-and-diffs/11): shown while a compare
/// base is pinned, in the same footer slot as [`search_line`]'s line.
fn compare_footer_line(state: &AppState) -> Option<SanitizedText> {
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

/// Which of `rows` the footer reveals in full, if any (ADR-0023): the
/// hovered row, else none. `hover` is an absolute history row number; `rows` is indexed from
/// the top of the viewport, offset by one when the uncommitted row draws
/// above it (`visible_rows`) — the same arithmetic, so the two never
/// disagree about which row is which. `None` when the row named is not
/// actually on screen (scrolled past, or the uncommitted row itself, which
/// has no subject to reveal).
fn reveal_index(state: &AppState, history: &History, rows: &[GraphEntry<'_>]) -> Option<usize> {
    let absolute = state.hover?;
    let offset = usize::from(uncommitted_shown(state));
    let index = offset + absolute.checked_sub(history.top)?;
    (index < rows.len()).then_some(index)
}

/// How the graph folds its lanes: the session's limit is filled in by
/// [`super::view`], the expansion flag is the user's, and [`keep_for`]
/// weighs the reserved integration lanes (`1..=n`, after lane 0), the
/// cursor's own column and the selection's thread ahead of everything else
/// (ADR-0025) — a priority a fold still overrides once the budget runs out,
/// not a veto.
/// Is there a divider on screen for `<>` to move?
///
/// The pane has to be **open** and in a **split**: under 100 columns it is a
/// tab strip with nothing between the two tabs to drag, so `<>` is a key
/// that does nothing and the footer must not offer it. `view::frame` already
/// passes `Divider::Hidden` for the Detail tab's own bar and says why; this
/// is the same answer for the Graph tab, which keeps the graph screen's
/// footer (m5-history-and-ghost-preview/33, review M6's width sweep).
pub(crate) fn divider_shown(state: &AppState) -> bool {
    state.detail.shown && !super::frame::tabbed(state, super::frame::screen_area(state))
}

/// The lane cap this frame folds at: `graph.max_lanes` if the config file
/// set one, else the width breakpoint — resolved against the room the graph
/// has, not the whole screen (ADR-0039's amendment,
/// [`super::frame::graph_columns`]).
///
/// The room is read off `AppState::size` rather than taken as an argument on
/// purpose: the hit-test has no `Frame` to ask, and a cap the drawing path
/// could pass a different width to is the very divergence this function
/// exists to close. `event_loop::graphics` already reads the screen
/// rectangle the same way.
pub(crate) fn max_lanes(state: &AppState) -> usize {
    graft_ui::graph_screen::max_lanes_for(
        super::frame::graph_columns(state, super::frame::screen_area(state)),
        state.max_lanes,
    )
}

fn compaction(
    state: &AppState,
    history: &History,
    thread: Option<&Thread>,
    rows: &[GraphEntry<'_>],
) -> Compaction {
    let integration = history
        .walk
        .as_ref()
        .map_or(0, |walk| walk.integration.len());
    let reserved: BTreeSet<usize> = (1..=integration).collect();
    let cursor = rows
        .iter()
        .position(|entry| entry.line.paint == RowPaint::Highlighted);
    let drawn: Vec<Row> = rows
        .iter()
        .map(|entry| entry.line.row.as_ref())
        .cloned()
        .collect();
    Compaction {
        max_lanes: max_lanes(state),
        expanded: state.lanes_expanded,
        keep: keep_for(
            &drawn,
            cursor,
            &reserved,
            thread.map(|thread| &thread.ancestry),
        ),
    }
}

/// **The bar under a live modal box**, or `None` with no box up (rulings
/// 07b, Q1).
///
/// A box claims the next key before any screen, overlay or pane does —
/// `credential_key`, then `name_prompt_claimed`, then `confirm_key`, the
/// order `update::core_event` and `update::keys::key_event` ask them in —
/// so a key of the screen behind it is a key that does nothing, and a line
/// never offers one of those (`update::resolver::answers`). This is asked
/// **first**, before any screen's own context: by [`footer_context`] and
/// [`hint_context`] for the graph's footer and the detail pane's bar, and
/// by `view::frame`, once after whichever screen drew, for every other bar
/// on the frame (the resolver's, the overlays' own). The same shape as F10's
/// `draw_modal_boxes`: one decision, not one per screen.
pub(crate) fn box_context(state: &AppState) -> Option<HintContext> {
    if state.credential.is_some() {
        Some(HintContext::Credential)
    } else if let Some(prompt) = &state.name_prompt {
        Some(HintContext::NamePrompt {
            toggle: matches!(
                prompt.kind,
                crate::update::name_prompt::NameKind::Branch { .. }
            ),
        })
    } else {
        // `y` on the bar exactly where `confirm_key` lets it act: both read
        // `PendingConfirm::yes` (m6-conflicts/50).
        state
            .confirm
            .as_ref()
            .map(|pending| HintContext::ConfirmBox { yes: pending.yes() })
    }
}

/// Why the live operation stopped, worded from this frame's working-tree
/// read (`banner::stop_reason`) so the step and the count stay true as
/// files get resolved, with the shorter wordings the footer falls back to
/// (`banner::stop_wordings`). `None` while nothing is stopped.
pub(crate) fn stopped_line(state: &AppState) -> Option<graft_ui::graph_screen::StopWording> {
    let ladder = crate::update::banner::stop_line_wordings(state)?;
    let mut wordings = ladder.wordings.iter().map(|line| sanitize_line(line));
    let full = wordings.next()?;
    let wording = graft_ui::graph_screen::StopWording::new(full, wordings.collect());
    Some(if ladder.before_keys {
        wording.before_keys()
    } else {
        wording
    })
}

/// What the footer offers under this frame's rows.
///
/// A focused banner owns the keyboard, so the footer must list the banner's
/// keys and nothing else: a hint bar offering `m merge` under a modal
/// banner names a key that does nothing. An *unfocused* banner claims
/// nothing, so the cursor's own context stands.
pub(crate) fn footer_context(
    state: &AppState,
    rows: &[GraphEntry<'_>],
    banner: Option<&graft_ui::graph_screen::BannerView>,
) -> HintContext {
    if let Some(boxed) = box_context(state) {
        return boxed;
    }
    match banner {
        // The resolver stands over the banner as well as the graph, and it
        // is modal: `c`, `s`, `a` reach the file, not the operation. Its
        // own bar is what `hint_context` answers (m6-conflicts/04).
        // The finish summary likewise, over the resolver too (m6-conflicts/19).
        _ if state.resolver.is_some() || state.finish.is_some() => hint_context(state, rows),
        Some(banner) if banner.focused => banner_context(state, banner),
        _ => back_to_blame(state, live_wrapped(state, hint_context(state, rows))),
    }
}

/// `row` wrapped in [`HintContext::OperationLive`] whenever an operation is
/// live and `row` is a graph row's bar (m6-conflicts/52, decision B) — with
/// or without a banner (a bisect has none), one `⇥` from it or not — so no
/// graph-row bar says `c commit` while `c` does otherwise.
///
/// `⇥ banner` leads only while one `Tab` lands on the banner
/// (m6-conflicts/12 scope item 5). A pane focused on a commit's files or
/// diff is wrapped too: `c` there passes through to the graph's keys
/// (`update::detail::keys` claims no plain `c`). Not the changes pane's
/// bar: it carries its own `live` ([`hint_context`]), and from its diff its
/// own `space`/`d` are not answered (the sweep in `update::tests`).
fn live_wrapped(state: &AppState, row: HintContext) -> HintContext {
    let Some(c) = live_c(state) else {
        return row;
    };
    if !matches!(
        row,
        HintContext::WorkingTreeRow
            | HintContext::BranchRow { .. }
            | HintContext::PlainRow
            | HintContext::Marked(_)
            | HintContext::PaneFocused { .. }
    ) {
        return row;
    }
    HintContext::OperationLive {
        row: Box::new(row),
        c,
        tab_to_banner: crate::update::banner::one_tab_to_banner(state),
    }
}

/// What `c` does in the graph or the changes pane right now, or `None`
/// while nothing is live and `c` commits. The one mapping from
/// [`pane_c_acts`](crate::update::commit_editor::pane_c_acts) to the bar's
/// words, so the label and the key cannot come apart.
fn live_c(state: &AppState) -> Option<LiveC> {
    use crate::update::commit_editor::{live_operation, pane_c_acts};
    use graft_git::state::InProgress;
    live_operation(state).map(|live| match (live.0, pane_c_acts(live)) {
        (_, false) => LiveC::Refuses,
        (InProgress::Merge, true) => LiveC::Finish,
        (_, true) => LiveC::Continue,
    })
}

/// The hint context a focused banner puts the footer in. One function
/// rather than two literals: the tab layout draws the banner's hints
/// outside the graph screen (`view::frame`), and the two must not drift.
///
/// `merge` comes from the live operation's kind, begun in Graft or in the
/// shell: `c` opens the finish summary on both, so the bar says `c finish`
/// (m6-conflicts/70, n2).
pub(crate) fn banner_context(
    state: &AppState,
    banner: &graft_ui::graph_screen::BannerView,
) -> HintContext {
    use graft_git::state::InProgress;
    HintContext::Banner {
        skippable: banner.skippable,
        conflicted: banner.conflict.is_some(),
        merge: crate::update::commit_editor::live_operation(state)
            .is_some_and(|(kind, _)| kind == InProgress::Merge),
    }
}

/// The changes pane's `⇥` word: where one `Tab` lands from here, read off
/// the one reading of the cycle (`update::banner::tab_target`,
/// m6-conflicts/70). A target the pane's bar never shows while it has
/// focus (`Files`, or no move at all under a modal, which draws its own
/// bar) falls to `graph`, the word the pane said before.
fn tab_word(state: &AppState) -> TabWord {
    use crate::update::banner::{TabTarget, tab_target};
    match tab_target(state) {
        Some(TabTarget::Diff) => TabWord::Diff,
        Some(TabTarget::Banner) => TabWord::Banner,
        Some(TabTarget::Graph | TabTarget::Files) | None => TabWord::Graph,
    }
}

/// The hint context the detail pane hints with when it owns the whole
/// screen — the sub-100-column `Tab` layout with the Detail tab up, where
/// the graph screen is not drawn and its footer is not there to borrow
/// (m5-history-and-ghost-preview/29).
///
/// Built from state alone, with no drawn rows: an overlay (the Map view,
/// the commit editor) can own the primary tab while the pane is up, and
/// there is no `GraphView` on that path to read a context off. So the
/// cursor row's own badges — the one thing the rows carry that state does
/// not — are out of reach here and the base is `PlainRow`. The graph's own
/// path passes `GraphView::hint_context` instead, which does have them.
pub(crate) fn pane_hint_context(state: &AppState) -> HintContext {
    back_to_blame(state, hint_context(state, &[]))
}

/// `context` led by `esc back to blame` exactly when the next `Esc` goes
/// back to the hidden trail (m7-power-tools/20, 20i): the same predicate
/// the key asks (`update::place::esc_goes_back`), so a search, Focus,
/// Explain or a held failure — whose `Esc` comes first — keeps its own bar.
/// Only a graph row's or a pane's bar is wrapped; an overlay, a drag or a
/// box has its own.
fn back_to_blame(state: &AppState, context: HintContext) -> HintContext {
    if !crate::update::place::esc_goes_back(state) {
        return context;
    }
    match context {
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow
        | HintContext::Marked(_)
        | HintContext::PaneFocused { .. }
        | HintContext::ChangesPane { .. }
        | HintContext::OperationLive { .. } => HintContext::BackToBlame(Box::new(context)),
        other => other,
    }
}

/// **The bar an open overlay puts the footer in**, or `None` when the graph
/// is the thing behind the cursor after all.
///
/// The Map view, the journal browser and the stash list stand *where the
/// graph stands* and answer the keys the graph would: each one's
/// `shown_key` ends in a catch-all that claims the key and does nothing
/// with it (`update::map`, `update::journal`, `update::stash`). So while
/// one is open the graph's own vocabulary — `b`, `t`, `c`, `m`, `r`, `y`,
/// `q`, `x`, `.` — reaches nothing at all.
///
/// Which of the two bars depends on **who has the keyboard**, exactly as
/// `update::keys::pane_and_overlay_key` decides it:
///
/// - the graph has focus, so the overlay is first and answers everything:
///   its own list;
/// - the detail pane has focus, so `detail::keys::detail_key` is first and
///   the overlay swallows whatever the pane leaves: the pane's own keys
///   and the two globals that are dispatched above both
///   ([`HintContext::PaneOverOverlay`]).
///
/// Until m5/42 this function did not exist and the footer under an overlay
/// was the graph's row context — a bar of nine verbs, every one of them
/// dead (m5/36's probe, this ticket's
/// `update::tests::the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows`).
///
/// **The commit editor is the overlay that is not here.** It is modal in
/// the stronger sense — `commit_editor::editor_key` claims every key but
/// `Ctrl+C` before the panes are reached — and it never draws a hint bar
/// to be wrong on: `view::frame::draw_commit_editor` refuses the `Tab`
/// layout outright, and the split layout above 100 columns draws no hint
/// row (`view::frame::split_for_detail`). A bar drawn for it would be a
/// bar the app never shows.
fn overlay_context(state: &AppState) -> Option<HintContext> {
    let overlay = match state.graph_overlay()? {
        GraphOverlay::Map => HintContext::Map,
        GraphOverlay::Journal => HintContext::Journal,
        GraphOverlay::Stash => HintContext::Stash,
        GraphOverlay::CommitEditor => return None,
        // Modal in the stronger sense, like the commit editor — but unlike
        // it, the resolver draws a bar, so it has one. It covers the detail
        // pane too, which is why it answers before the focus test below:
        // there is no pane on screen to have the keyboard.
        //
        GraphOverlay::Resolver => return Some(resolver_context(state)),
        // Modal, covers the detail pane, draws its own bar: the resolver's
        // case (m6-conflicts/19).
        GraphOverlay::Finish => return Some(super::finish::finish_context(state)),
        // Modal over the graph and the pane (keymap §2.1): its own bar.
        GraphOverlay::Place => return place_context(state),
    };
    if state.graph_has_focus() {
        return Some(overlay);
    }
    Some(HintContext::PaneOverOverlay {
        opens: crate::update::detail::enter_opens(&state.detail),
    })
}

/// The blame place's bar (m7-power-tools/20): `n N` only while a find is
/// active, `i` only while the repository lists revisions to ignore — the
/// conditions `update::place::place_key` answers on; without lines (a read
/// in flight, or a failed one) no `⏎`, `^` or `n N`, which need them (20k,
/// 20n).
pub(crate) fn place_context(state: &AppState) -> Option<HintContext> {
    let blame = state.trail.on_screen()?;
    Some(HintContext::Blame {
        finding: blame.find.is_some(),
        ignore_file: blame.ignore_line().is_some(),
        lines: matches!(blame.body, crate::update::place::BlameLoad::Ready { .. }),
    })
}

/// The resolver's bar: the one context both the classifier above and the
/// resolver's own footer (`view::frame::draw_resolver_footer`) draw from,
/// so the bar the key sweep checks is the bar on screen (m6-05 review,
/// Major 1 — the footer had its own hard-coded copy, which drew neither
/// `B` nor `d`).
///
/// `B`/`d` act the moment they are pressed (`update::resolver::toggle_base`
/// / `restart_box`, m6-conflicts/05), so a document passes `true` for both.
/// A whole-file card has no blocks for either to act on, and draws its own
/// shorter list. `restart` is `start_over_refusal`'s answer, the same one
/// `d` itself asks (m6-conflicts/28): the bar offers `d` exactly where it
/// opens the box.
#[must_use]
pub fn resolver_context(state: &AppState) -> HintContext {
    let card = state
        .resolver
        .as_ref()
        .is_some_and(|open| open.whole_file.is_some());
    let done = state
        .resolver
        .as_ref()
        .is_some_and(|open| open.done.is_some());
    HintContext::Resolver {
        base: !card,
        restart: crate::update::resolver::start_over_refusal(state).is_none(),
        whole_file: card,
        done,
        roles: crate::update::resolver::side_names(state).roles,
        // `c` would act rather than refuse (m6-conflicts/31): the bar
        // never names a key that answers "not yet".
        finishable: crate::update::banner::banner_view(state).is_some()
            && crate::update::banner::continue_refusal(state).is_none(),
    }
}

/// The footer's context for this frame (`hints::hints_for`, ADR-0037 §4):
/// the changes pane and the working-tree row both win over anything the
/// cursor sits on, since they are full-screen states — **the pane first**,
/// because it is the more specific of the two. The pane opens *on* the
/// uncommitted-changes row and the cursor stays there, so `on_working_tree`
/// and `on_changes_pane()` are true together for the whole time the pane is
/// up; testing the row first made the pane's own keys unreachable and hinted
/// `⏎ changes` to someone already in the changes (m5-03 review, Critical 1).
///
/// Otherwise the cursor row's own badges say whether it is a branch, and its
/// Mistake Radar marker (`GraphEntry::marker`) wraps whichever context in
/// `Marked`, exactly as `hints_for` interprets it. `Journal` and `Stash` are
/// never produced here — `view::frame` draws those overlays instead of the
/// graph screen while they are shown, so the graph's own footer never has to
/// hint them. `<> resize` is not a context: `GraphView::divider_shown` carries
/// it, for every context alike.
fn hint_context(state: &AppState, rows: &[GraphEntry<'_>]) -> HintContext {
    // A live box before anything: it takes the key before the drag does.
    if let Some(boxed) = box_context(state) {
        return boxed;
    }
    // A badge in the air first: while the gesture is running, the keys the
    // cursor's row would offer are not the ones that end it. First here
    // because it is first in `update::keys::key_event` too — `drag_key`
    // takes the key before any overlay or pane sees it.
    if state.drag.is_some() {
        return HintContext::Dragging;
    }
    // Then an overlay, for the same reason: it stands where the graph
    // stands and answers the keys the graph would (m5/42).
    if let Some(context) = overlay_context(state) {
        return context;
    }
    // The changes pane's own bar, but only while the pane has focus:
    // `space`, `d`, `⌃A` and `e` are read by `changes_pane_write_key`,
    // which refuses every one of them with the graph focused
    // (`update::detail::keys`, `DetailFocus::Graph => return false`). With
    // the pane open and focus back on the graph the row under the cursor is
    // what the bar is about, which the rest of this function works out.
    if state.on_changes_pane() && !state.graph_has_focus() {
        return HintContext::ChangesPane {
            unmerged: cursor_file_unmerged(state),
            live: live_c(state),
            tab: tab_word(state),
        };
    }
    if state.on_working_tree {
        return HintContext::WorkingTreeRow;
    }
    let cursor = rows
        .iter()
        .find(|entry| entry.line.paint == RowPaint::Highlighted);
    let row = match cursor {
        None => HintContext::PlainRow,
        Some(cursor) => {
            let base = cursor
                .badges
                .iter()
                .find_map(|badge| match badge.kind {
                    BadgeKind::Branch { current, .. } => Some(HintContext::BranchRow { current }),
                    _ => None,
                })
                .unwrap_or(HintContext::PlainRow);
            if cursor.marker.is_some() {
                HintContext::Marked(Box::new(base))
            } else {
                base
            }
        }
    };
    // Last, over whichever row context came out. A focused pane leaves four
    // of the graph screen's keys doing nothing (`AppState::graph_has_focus`)
    // and, with the diff focused, `⏎` as well — it has already opened
    // everything there is to open. Both conditions are the handlers' own
    // functions, read here rather than restated, which is the whole point:
    // the bar and the key cannot disagree if they read one answer
    // (m5 review Q2, m5/31).
    if state.graph_has_focus() {
        row
    } else {
        HintContext::PaneFocused {
            row: Box::new(row),
            opens: crate::update::detail::enter_opens(&state.detail),
            // `B`'s own guard (`update::place::blamable`): not while the
            // commit-graph offer takes the key.
            blamable: crate::update::place::blamable(state),
            // The files list's `space` on a directory row (m7-power-tools/20h).
            folds: crate::update::detail::space_folds(&state.detail),
        }
    }
}

/// Whether the changes pane's cursor is on an unmerged (conflicted) file —
/// the cursor's own file, not "any file in the tree": while one conflict
/// exists, every clean file's row would otherwise lose `space stage` and
/// `d discard` (m5-03 review, Minor 4). A section row, a directory row, or
/// no file list at all: not unmerged.
///
/// Matched by path bytes, the way `update::operate` matches the cursor's
/// file to its status entry — the file list is git's diff output and the
/// status entries are git's porcelain output, and only the path joins them.
fn cursor_file_unmerged(state: &AppState) -> bool {
    use std::ffi::OsStr;

    let Some(list) = state.detail.list.as_ref() else {
        return false;
    };
    let Some(entry) = state.detail.selected_entry() else {
        return false;
    };
    let Some(file) = usize::try_from(entry)
        .ok()
        .and_then(|index| list.files.get(index))
    else {
        return false;
    };
    state.working_tree.as_deref().is_some_and(|status| {
        status.entries.iter().any(|candidate| {
            candidate.unmerged
                && OsStr::as_encoded_bytes(candidate.path.as_os_str()) == file.path.as_slice()
        })
    })
}

/// The columns a fold weighs above the ordinary rank (ADR-0025), built once
/// and shared by the text and pixel tiers so a lane never folds differently
/// between them: the cursor's own column when its row is among `rows`,
/// `reserved` as given, and the selection's thread (`Ancestry::columns`).
pub(super) fn keep_for(
    rows: &[Row],
    cursor: Option<usize>,
    reserved: &BTreeSet<usize>,
    thread: Option<&Ancestry>,
) -> Keep {
    Keep {
        selected: cursor.and_then(|at| rows.get(at)).map(|row| row.lane),
        reserved: reserved.clone(),
        preferred: thread.map(Ancestry::columns).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use graft_term::sanitize_line;
    use graft_ui::graph_screen::BannerView;

    use super::*;

    fn banner(focused: bool, conflict: Option<&str>) -> BannerView {
        BannerView {
            kind: sanitize_line("MERGING"),
            step: None,
            conflict: conflict.map(sanitize_line),
            focused,
            skippable: false,
        }
    }

    /// A focused banner owns the keyboard, so the footer is the banner's —
    /// whatever the cursor is sitting on. Untested until now: the tick for
    /// it cited the hint-bar walk, which enumerates `HintContext` variants
    /// and never calls this function, so a `footer_context` that ignored
    /// `focused` passed every test in the crate (m5-09 review, Major 5).
    #[test]
    fn a_focused_banner_takes_the_footer() {
        let state = AppState::default();
        assert_eq!(
            footer_context(&state, &[], Some(&banner(true, Some("notes.md")))),
            HintContext::Banner {
                skippable: false,
                conflicted: true,
                merge: false,
            }
        );
    }

    /// An unfocused banner claims no key, so the cursor's own context
    /// stands.
    #[test]
    fn an_unfocused_banner_leaves_the_footer_to_the_cursor() {
        let state = AppState::default();
        assert_eq!(
            footer_context(&state, &[], Some(&banner(false, Some("notes.md")))),
            hint_context(&state, &[]),
        );
    }

    /// With nothing unmerged the banner's `M`/`e` refuse in words, so the
    /// bar must not offer them: `conflicted` is what carries that.
    #[test]
    fn a_banner_with_no_conflict_says_so_in_its_context() {
        let state = AppState::default();
        assert_eq!(
            footer_context(&state, &[], Some(&banner(true, None))),
            HintContext::Banner {
                skippable: false,
                conflicted: false,
                merge: false,
            }
        );
    }
}
