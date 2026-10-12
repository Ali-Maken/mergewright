//! The footer's key hints, by context (`hint-bar-redesign`, ADR-0037 §4: the
//! bar is how anything is discoverable; a feature nobody can see doesn't
//! exist). Shows what the cursor or focus can do, filled **from the most
//! relevant end**: as many hints as the bar's own room holds, each on the
//! best of its three rungs — `c commit` → `c cmt` → `c` — that still leaves
//! room for the hints ahead of it ([`ladder`], ADR-0039's third amendment).
//! Never all hidden, never a hint cut in half, and never below the three
//! most relevant at their **floor rung** (ADR-0037 §4).
//!
//! Replaces the old fixed nineteen-entry list (`crates/graft-ui/MAP.md`'s
//! `graph_screen` entry: 144/155-column thresholds): that list answered
//! "what can this screen ever do", which grew too long to read at 80 or 120
//! columns and buried the keys that mattered right now. This one answers
//! "what does the *cursor* do right now" first, and only then what the whole
//! screen answers ([`TAIL`], the old list's leftovers in the old order, so a
//! wide terminal still teaches `q quit` or `x compare`).
//!
//! The per-context lists are the design note's priority table
//! (`docs/design-notes/m5-history-and-ghost-preview.md`, "Hint bar"). The
//! journal, stash and Map draw their own footers (`journal_screen`,
//! `stash_screen`, `map_screen`), which take the doorway from
//! [`doorway_words`] and shed by [`fit_place_footer`] (m8/11e).
//!
//! **A hint is only drawn once its key is bound.** Every entry carries a
//! [`HintState`]: the table stays as the design wrote it, but
//! [`HintState::Planned`] hints are dropped before the width rule
//! runs, so the bar never teaches a key that does nothing (m5-03 review,
//! Critical 2). `graft` proves both halves —
//! `update::tests::every_key_the_bar_draws_is_answered` (Live ⇒ bound, key
//! *and* verb, **and answered in the state the bar is drawn in**)
//! and `update::tests::every_planned_hint_is_still_unbound` (Planned ⇒ not
//! bound, so binding one fails the build until the table is flipped).

mod catalogue;
mod context;
mod doorway;
mod filters;
mod fitting;
mod hint;
mod keys;
mod lists;

use crate::keymap::{ActionId, Keymap};
use keys as hint_keys;

pub use catalogue::{every_context, every_hint};
pub use context::{ConfirmYes, HintContext, LiveC, RowVerbs, TabWord, WayToBanner};
pub(crate) use doorway::doorway_key;
pub use doorway::doorway_words;
use doorway::{doorway_hint, doorway_last, spelled, spelled_bar, words_for};
#[cfg(test)]
use filters::drawable;
use filters::{
    BACK_TO_BLAME, BACK_TO_HISTORY, BLAME, F_HISTORY, FOLD, SEARCH_NEXT, SEARCH_PREV,
    WAY_TO_BANNER_BACK, WAY_TO_BANNER_FORWARD, whole_bar,
};
pub use fitting::{
    HINT_SEPARATOR, StopWording, fit_status_line, hints_text, stopped_row, stopped_row_before_keys,
};
use fitting::{Words, fit_list};
pub(crate) use fitting::{fit_place_footer, place_close, place_footer_text};
pub(crate) use hint::{ACTIONS, DOORWAY, DOORWAY_BY_PALETTE};
use hint::{EXPLAIN, RESIZE, hint};
pub use hint::{Hint, HintKey, HintState, MARK_RESOLVED, MARK_RESOLVED_SHORT, NOT_A_BINDING, Rung};
#[cfg(test)]
use hint::{planned, short_form};
use lists::{TAIL, context_list};
pub use lists::{blame_place_list, history_place_list};

/// Whether a detail pane is on screen, so its divider can be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Divider {
    /// A detail pane is open: `<> resize` applies.
    Shown,
    /// No pane, no divider, no hint.
    Hidden,
}

/// The hints for `context` that fit in `columns` cells: **as many whole
/// hints as fit the row the bar actually has**, each at the best rung
/// [`ladder`] can afford it, and the ones that will not fit even at their
/// bare key dropped from the right — never below three (or the context's own
/// count, if it has fewer) plus `. more`. Never empty except for
/// [`HintContext::Preview`].
///
/// `columns` is the bar's own room, not the screen's, and there is no
/// width tier on top of it. The 3/6/all tiers this used to carry were
/// never spec: ADR-0037 §4 sets a floor (three most relevant keys, never
/// all hidden) and says nothing about width. Laid over a `columns` that is
/// already the bar's own region, a tier is a *cap*: at 160 columns with the
/// detail pane beside the graph the bar has about 92 cells and fell into
/// the "narrow" tier, so it drew three hints — while a 120-column screen
/// with the pane below kept its 120 cells and drew six. The wider terminal
/// said less (ADR-0039's 2026-09-21 amendment, m5/20 item 7).
///
/// `divider` adds `<> resize` when a detail pane is open, whichever context
/// the cursor is in (ADR-0028: the pane being shown is what makes the
/// divider exist).
///
/// [`HintContext::Banner`] is the exception to both trailers. The banner is
/// modal — `banner_key` answers every key but `Ctrl+C` — so `.` opens
/// nothing and `<>` moves no divider while it has focus. ADR-0039's doorway
/// is a promise that what the width rule dropped is still reachable; under a
/// modal that reaches nothing, drawing it would be the dead key the ADR's
/// own first condition rules out. The banner's list is short enough to draw
/// whole at 80 columns, so nothing is dropped and nothing needs a doorway.
///
/// Keys are spelled from `keys`, his keymap (ticket m8/10): a hint whose
/// every key is unbound is left out before the width rule runs, and so is a
/// hint whose key an earlier hint on the same bar already draws — after a
/// remap two actions can spell one chord, and the bar never draws a chord
/// twice ([`spelled`]).
#[must_use]
pub fn hints_for(
    context: &HintContext,
    divider: Divider,
    columns: u16,
    keys: &Keymap,
) -> Vec<Hint> {
    // The resolver below its own floor is the one context whose *keys*
    // depend on the width: under `conflict_resolver::MIN_COLUMNS` the
    // widget draws one line of text instead of the document, and the app
    // answers nothing but `Esc` — there is no document on screen to take a
    // side of. So the bar says the one thing that is true there, which is
    // also what that line itself says (m6-conflicts/04).
    if matches!(context, HintContext::Resolver { .. })
        && columns < crate::conflict_resolver::MIN_COLUMNS
    {
        return spelled(vec![hint(hint_keys::BACK, "close")], keys);
    }
    // The finish summary has a floor too (`finish_summary::FINISH_MIN`), but
    // in both dimensions, and this is told only the width: the app draws
    // `esc back` alone below it (`graft::view::finish`), so the width sweep
    // here stays monotone.
    let (list, trailer) = spelled_bar(context, divider, keys);
    let words = words_for(context);
    let fitted = fit_list(&list, trailer, columns, words, keys);
    if words == Words::Only {
        doorway_last(fitted)
    } else {
        fitted
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod spell_sweep;
