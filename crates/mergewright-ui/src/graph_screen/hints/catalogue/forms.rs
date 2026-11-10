//! The row-form wrappers [`every_context`](super::every_context) applies to
//! each plain context: marked, searching, pane-focused, operation-live.

use super::super::{HintContext, LiveC, RowVerbs, WayToBanner};
use super::LIVE_C;

/// `context` wrapped in [`HintContext::OperationLive`] every way `mergewright`
/// can wrap it — each [`LiveC`], one `⇥` from the banner or not — for the
/// contexts `mergewright::view::rows::footer_context` wraps: a graph row, marked
/// or not, and a pane focused on a commit (its `c` passes through to the
/// graph's). Not the changes pane: it carries its own `live`
/// ([`changes_pane_live_forms`]).
///
/// Exhaustive on purpose: see [`every_context`].
pub(super) fn operation_live_form(context: &HintContext) -> Vec<HintContext> {
    match context {
        // `R rewrite` is drawn only while nothing is live (08, ruling 10).
        HintContext::Marked(row) if matches!(**row, HintContext::Rewritable(_)) => Vec::new(),
        // A live operation hides `o` and `a` (mergewright `view::rows::live_wrapped`
        // passes `RowVerbs::NONE`): a row with either never reaches the bar.
        _ if has_row_verbs(context) => Vec::new(),
        // `R` is never live, so neither is a search over it.
        HintContext::Searching(row) if over_rewritable(row) => Vec::new(),
        HintContext::WorkingTreeRow
        | HintContext::StashRow
        | HintContext::PlainRow { .. }
        | HintContext::Marked(_)
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. } => {
            // `space stage` only at an `edit` stop with the graph focused
            // (07d D1): never for a pane with the keyboard.
            let pane = matches!(context, HintContext::PaneFocused { .. });
            LIVE_C
                .into_iter()
                .flat_map(|c| {
                    let stages: &[bool] = if c == LiveC::ContinueAmends && !pane {
                        &[false, true]
                    } else {
                        &[false]
                    };
                    stages.iter().flat_map(move |&space_stages| {
                        [Some(WayToBanner::Forward), Some(WayToBanner::Back), None]
                            .into_iter()
                            .map(move |way_to_banner| HintContext::OperationLive {
                                row: Box::new(context.clone()),
                                c,
                                way_to_banner,
                                space_stages,
                            })
                    })
                })
                .collect()
        }
        HintContext::ChangesPane { .. }
        | HintContext::Banner { .. }
        | HintContext::BannerPaused { .. }
        | HintContext::Preview
        | HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Clash(_)
        | HintContext::GraphRow { .. }
        | HintContext::Finish { .. }
        | HintContext::Review(_)
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::Rewritable(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => Vec::new(),
    }
}
/// `context` is `R rewrite`'s row, marked or not.
fn over_rewritable(context: &HintContext) -> bool {
    match context {
        HintContext::Rewritable(_) => true,
        HintContext::Marked(row) => over_rewritable(row),
        _ => false,
    }
}

/// The cursor row inside `context` carries `o take` or `a amend`.
fn has_row_verbs(context: &HintContext) -> bool {
    match context {
        HintContext::PlainRow { verbs } | HintContext::GraphRow { verbs, .. } => {
            *verbs != RowVerbs::NONE
        }
        HintContext::Marked(row)
        | HintContext::Rewritable(row)
        | HintContext::Searching(row)
        | HintContext::PaneFocused { row, .. } => has_row_verbs(row),
        _ => false,
    }
}

/// `context` wrapped in [`HintContext::Marked`], for the contexts a Mistake
/// Radar marker can actually reach — the cursor's own row, once
/// `mergewright::view::rows::hint_context` has ruled out the working tree, the
/// changes pane and a drag in the air. Everything else has no cursor row to
/// carry a marker.
///
/// Exhaustive on purpose: see [`every_context`].
pub(super) fn marked_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::PlainRow { .. } | HintContext::StashRow | HintContext::Rewritable(_) => {
            Some(HintContext::Marked(Box::new(context.clone())))
        }
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::BannerPaused { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Clash(_)
        | HintContext::GraphRow { .. }
        | HintContext::Finish { .. }
        | HintContext::Review(_)
        | HintContext::Marked(_)
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. }
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => None,
    }
}

/// `context` wrapped in [`HintContext::Searching`] (m8/15b), for the
/// graph-row forms `mergewright::view::rows::footer_context` wraps while a search
/// stands with its prompt closed: a row, marked or under `R`. Not a pane's
/// bar, an overlay's or a box's: the search's keys are the graph's.
///
/// Exhaustive on purpose: see [`every_context`].
pub(super) fn searching_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::WorkingTreeRow
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::Marked(_) => Some(HintContext::Searching(Box::new(context.clone()))),
        HintContext::Banner { .. }
        | HintContext::BannerPaused { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Clash(_)
        | HintContext::GraphRow { .. }
        | HintContext::Finish { .. }
        | HintContext::Review(_)
        | HintContext::Searching(_)
        | HintContext::PaneFocused { .. }
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => None,
    }
}

/// `context` wrapped in [`HintContext::PaneFocused`], for the contexts
/// `mergewright::view::rows::hint_context` can reach with the pane focused and
/// `⏎` spent — a cursor row, marked or not.
///
/// Not the working-tree row: a pane open on *it* is the changes pane, so
/// the classifier answers `ChangesPane` long before it looks at the cursor
/// row. Not the changes pane or the banner either — neither list has an
/// `⏎` this could take, and wrapping a list to change nothing would be a
/// context nobody can tell from the one inside it.
///
/// Not the overlays: a pane focused with one of them behind it draws
/// neither the overlay's list nor a row's, because the graph is not there
/// to answer the row's keys. That is [`HintContext::PaneOverOverlay`],
/// which [`every_context`] names in its own right.
///
/// Exhaustive on purpose: see [`every_context`].
pub(super) fn pane_focused_form(context: &HintContext) -> Vec<HintContext> {
    match context {
        // `R` needs the graph's keyboard (08, ruling 10): never under a pane.
        HintContext::Marked(row) if matches!(**row, HintContext::Rewritable(_)) => Vec::new(),
        HintContext::PlainRow { .. } | HintContext::StashRow | HintContext::Marked(_) => {
            // `(opens, blamable, folds)`: the files list on a file, on a
            // directory, on neither (the `✉ message` row); the diff on a
            // line or not. Never `folds` without `opens` (only the files
            // list folds), nor with `blamable` (a directory has no file to
            // blame): neither shape exists in the app.
            // The last: the Detail tab, graph hidden (m8/11b decision 3).
            [
                (true, true, false, false),
                (true, false, true, false),
                (true, false, false, false),
                (false, true, false, false),
                (false, false, false, false),
                (false, false, false, true),
            ]
            .into_iter()
            .map(
                |(opens, blamable, folds, graph_hidden)| HintContext::PaneFocused {
                    row: Box::new(context.clone()),
                    opens,
                    blamable,
                    folds,
                    graph_hidden,
                },
            )
            .collect()
        }
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::BannerPaused { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Clash(_)
        | HintContext::GraphRow { .. }
        | HintContext::Finish { .. }
        | HintContext::Review(_)
        | HintContext::PaneFocused { .. }
        | HintContext::Searching(_)
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_)
        | HintContext::History { .. }
        | HintContext::BackToHistory(_)
        | HintContext::Rewritable(_)
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage
        | HintContext::OperationLive { .. } => Vec::new(),
    }
}
