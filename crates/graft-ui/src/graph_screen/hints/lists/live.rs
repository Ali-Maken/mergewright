//! A list said the way it acts while an operation is live: `c` as finish or
//! continue, and the graph's write keys that only refuse dropped.

use super::super::keys::{AMEND, COMMIT, FOLD};
use super::super::{Hint, HintKey, LiveC, hint};
use crate::keymap::ActionId;

/// `list` with its `c commit` said the way `c` acts while an operation is
/// live (m6-conflicts/52), in the same place: `c finish`, `c continue`, or
/// gone where `c` only refuses in words. Every other hint is untouched.
pub(super) fn said_live(list: Vec<Hint>, c: LiveC) -> Vec<Hint> {
    // At an `edit` stop `a` amends the stopped commit, and it leads: it is
    // the step the stop is for (07b, note R4's bar).
    let mut said = match c {
        LiveC::ContinueAmends => vec![AMEND_IT],
        LiveC::Finish | LiveC::Continue | LiveC::Refuses => Vec::new(),
    };
    said.extend(list.into_iter().filter_map(|hint| {
        if hint.is(AMEND_IT.keys) && c == LiveC::ContinueAmends {
            return None;
        }
        if !hint.is(COMMIT) {
            return Some(hint);
        }
        match c {
            LiveC::Finish => Some(Hint {
                verb: "finish",
                ..hint
            }),
            LiveC::Continue | LiveC::ContinueAmends => Some(Hint {
                verb: "continue",
                ..hint
            }),
            LiveC::Refuses => None,
        }
    }));
    said
}

/// `a amend it`, at a rewrite's `edit` stop.
const AMEND_IT: Hint = hint(AMEND, "amend it");

/// `space stage`, at a rewrite's `edit` stop on the rows where `a` amends
/// (m7-power-tools/07d D1).
const SPACE_STAGE: Hint = hint(FOLD, "stage");

/// A graph row's bar while an operation is live: [`said_live`]'s list with
/// `c` ranked first after `a amend it` — above the graph's write keys,
/// which a live operation refuses (review m7-10-to-07c Minor 7), so the
/// width ladder drops them before the key that finishes — and, where
/// `space_stages`, `space stage` right after it in place of `space fold`.
pub(crate) fn said_live_on_the_graph(list: Vec<Hint>, c: LiveC, space_stages: bool) -> Vec<Hint> {
    let said = said_live(list, c);
    let (c_hint, rest): (Vec<Hint>, Vec<Hint>) = said.into_iter().partition(|hint| hint.is(COMMIT));
    let mut rest = rest.into_iter().peekable();
    let mut ranked = Vec::new();
    if c == LiveC::ContinueAmends {
        ranked.extend(rest.next_if(|hint| hint.is(AMEND_IT.keys)));
    }
    ranked.extend(c_hint);
    if space_stages {
        ranked.push(SPACE_STAGE);
    }
    ranked.extend(rest.filter(|hint| !(space_stages && hint.is(SPACE_STAGE.keys))));
    ranked.retain(|hint| !refused_while_live(hint));
    ranked
}

/// The graph's write keys that refuse while an operation is live
/// (m8-polish-and-ship/15 A1, keymap §2.1 mode rule: a key that can only
/// say "no" is not offered). Each member's refusing line, in `graft`:
/// - `app.undo` — `update::undo::start_undo` → `in_progress_refusal`
///   (`update/undo.rs:344`, words from `live_refusal_words`). The one
///   exception, a merge begun in the shell (keymap §2.6a), is not told
///   apart by the bar's context, so `u` is not offered there either: a key
///   left out is recoverable through `?`, a key that refuses is a lie.
/// - `graph.merge`, `graph.rebase` — `preview::history_key` →
///   `open_preview` → `preview_refusal` (`update/preview.rs:380`); on a
///   detached `HEAD` (an `edit` stop) `m` refuses earlier
///   (`update/preview/history.rs:112`).
/// - `graph.take` — `preview::take_key` → `open_preview` → `preview_refusal`.
/// - `graph.rewrite` — `rewrite_editor::open_rewrite` → `preview_refusal`
///   (`update/rewrite_editor/open.rs:48`).
///
/// Held against the app by `graft`'s
/// `no_key_a_live_operation_bar_draws_refuses_while_live`.
const REFUSED_WHILE_LIVE: &[ActionId] = &[
    ActionId("app.undo"),
    ActionId("graph.merge"),
    ActionId("graph.rebase"),
    ActionId("graph.take"),
    ActionId("graph.rewrite"),
];

/// Whether `hint` names a [`REFUSED_WHILE_LIVE`] key.
fn refused_while_live(hint: &Hint) -> bool {
    hint.keys.iter().any(|key| {
        matches!(key, HintKey::Key(id) | HintKey::EveryChord(id) if REFUSED_WHILE_LIVE.contains(id))
    })
}
