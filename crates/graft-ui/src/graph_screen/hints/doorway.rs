//! Spelling a bar from his keymap and its doorway: `spelled`, the `? keys` / `⌃P keys` fallback, words-only placement (m8/21b-1 split from `hints.rs`).

use super::{DOORWAY, DOORWAY_BY_PALETTE, Divider, Hint, HintContext, Keymap, Words, whole_bar};

/// A words-only bar with its doorway, if drawn, moved to the end: where a
/// marked row ranks `? keys` mid-list (m8/11d R2) it still reads last, as
/// on every other words-only bar. Width-neutral — every join on a
/// words-only bar is the same ` · `, so a reorder cannot overflow.
pub(super) fn doorway_last(mut fitted: Vec<Hint>) -> Vec<Hint> {
    if let Some(at) = fitted
        .iter()
        .position(|hint| *hint == DOORWAY || *hint == DOORWAY_BY_PALETTE)
    {
        let door = fitted.remove(at);
        fitted.push(door);
    }
    fitted
}

/// [`whole_bar`] with [`spelled`] applied to the list and its doorway: the
/// doorway goes too when `.` is unbound, or when the list already draws it.
pub(super) fn spelled_bar(
    context: &HintContext,
    divider: Divider,
    keys: &Keymap,
) -> (Vec<Hint>, Option<Hint>) {
    let (list, trailer) = whole_bar(context, divider, keys);
    let list = spelled(list, keys);
    // `? keys`, or `⌃P keys` when `?` is unbound (m8/11b decision 1): the
    // one fallback, [`doorway_hint`], which the place footers spell too.
    let trailer = trailer.and_then(|door| {
        if door == DOORWAY {
            doorway_hint(keys)
        } else {
            Some(door)
        }
    });
    let trailer = trailer.filter(|door| {
        spelled(list.iter().cloned().chain([door.clone()]).collect(), keys).len() > list.len()
    });
    (list, trailer)
}

/// The doorway his keymap can spell: [`DOORWAY`], else
/// [`DOORWAY_BY_PALETTE`] when `app.help` is unbound, else `None`. The one
/// home of that fallback (m8/11e F2): the bar and the place footers both
/// read it.
pub(super) fn doorway_hint(keys: &Keymap) -> Option<Hint> {
    [DOORWAY, DOORWAY_BY_PALETTE]
        .into_iter()
        .find(|door| !door.labels(keys).is_empty())
}

/// The bar doorway as his keymap spells it: `? keys`, else `⌃P keys` when
/// `help` is unbound, else `None`. The one source: `spelled_bar` and the
/// journal / stash / Map footers all call it (m8/11e F2).
#[must_use]
pub fn doorway_words(keys: &Keymap) -> Option<String> {
    doorway_hint(keys).and_then(|door| door.text(keys))
}

/// The doorway's bare key (`?`, or `⌃P`): the rung below
/// [`doorway_words`] on a footer too narrow for the words (ADR-0039).
pub(crate) fn doorway_key(keys: &Keymap) -> Option<String> {
    doorway_hint(keys).and_then(|door| door.key_text(keys))
}

/// `list` less every hint that names no live key, and less every hint
/// drawing a chord an earlier hint already draws (the earlier one is the
/// more relevant: tables put a place's own keys first).
pub(super) fn spelled(list: Vec<Hint>, keys: &Keymap) -> Vec<Hint> {
    let mut drawn: Vec<String> = Vec::new();
    let mut kept = Vec::new();
    for hint in list {
        let labels = hint.labels(keys);
        if labels.is_empty() || labels.iter().any(|label| drawn.contains(label)) {
            continue;
        }
        drawn.extend(labels);
        kept.push(hint);
    }
    kept
}

/// Whether `context`'s bar is words-only (ADR-0058 §4): every context that
/// is not modal. The modal ones — the banner, a badge in the air, boxes,
/// prompts, the overlays, the resolver, the places, the rewrite editor —
/// keep the bare-key rung, because there the keys are the point.
pub(super) fn words_for(context: &HintContext) -> Words {
    match context {
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::GraphRow { .. }
        // The clash screen is modal but words-only: the mocks draw every
        // key with its words, and each has a short rung (ease-pass/03b).
        | HintContext::Clash(_)
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::ChangesPane { .. }
        | HintContext::Marked(_)
        | HintContext::PaneFocused { .. } => Words::Only,
        HintContext::BackToBlame(inner)
        | HintContext::BackToHistory(inner)
        | HintContext::Searching(inner) => words_for(inner),
        HintContext::OperationLive { row, .. } => words_for(row),
        HintContext::Preview
        | HintContext::Banner { .. }
        | HintContext::BannerPaused
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        | HintContext::Blame { .. }
        | HintContext::History { .. }
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage => Words::Keys,
    }
}
