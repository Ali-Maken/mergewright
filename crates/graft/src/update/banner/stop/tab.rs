//! Where `Tab` goes while a banner is drawn, and the key hint that says so.

use super::AppState;

/// One `Tab` from here lands on the banner, and nothing modal stands in the
/// way: the graph footer then leads with `⇥ banner`
/// (`HintContext::OperationLive`, m6-conflicts/12 scope item 5).
///
/// Read off [`tabs_to_banner`], the one reading of the cycle
/// (m6-conflicts/70): with the detail pane open and its file list focused,
/// `Tab` goes to the diff first, and a hint promising the banner there would
/// name a key that does something else.
#[must_use]
pub fn one_tab_to_banner(state: &AppState) -> bool {
    !state.modal_owns_keys() && tabs_to_banner(state) == Some(1)
}

/// Where one `Tab` lands (m6-conflicts/70), as [`tab_target`] names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabTarget {
    /// The graph (`DetailFocus::Graph`).
    Graph,
    /// The detail pane's file list.
    Files,
    /// The detail pane's diff.
    Diff,
    /// The operation banner.
    Banner,
}

/// **Where one `Tab` goes from the current focus**: the one reading of the
/// cycle — `DetailState::cycle_focus` (Graph → Files → Diff → Graph),
/// [`crate::update::banner::take_focus`] (the banner, from the diff or with the pane
/// closed), and leaving the banner (always the graph,
/// `hand_the_keyboard_back`). `None`: `Tab` moves no focus here — a modal
/// owns the keys, or the pane is closed with no banner to go to.
///
/// "A banner is drawn" is asked through [`crate::update::banner::banner_view`], the call
/// `take_focus` makes, never worked out again from "an operation is live".
#[must_use]
pub(crate) fn tab_target(state: &AppState) -> Option<TabTarget> {
    use graft_ui::detail::DetailFocus;
    let banner_drawn = crate::update::banner::banner_view(state).is_some();
    if state.banner_focus && banner_drawn {
        return Some(TabTarget::Graph);
    }
    if state.modal_owns_keys() {
        return None;
    }
    if !state.detail.shown {
        return banner_drawn.then_some(TabTarget::Banner);
    }
    Some(match state.detail.focus {
        DetailFocus::Graph => TabTarget::Files,
        DetailFocus::Files => TabTarget::Diff,
        DetailFocus::Diff if banner_drawn => TabTarget::Banner,
        DetailFocus::Diff => TabTarget::Graph,
    })
}

/// **How many `Tab`s from here reach the banner**; `None` when no banner is
/// drawn (asked through [`crate::update::banner::banner_view`], as `take_focus` asks) or the
/// banner already has the keyboard. Walks the cycle [`tab_target`] reads:
/// from the file list it is two (the diff, then the banner).
#[must_use]
pub(crate) fn tabs_to_banner(state: &AppState) -> Option<u8> {
    use graft_ui::detail::DetailFocus;
    if state.banner_focus || crate::update::banner::banner_view(state).is_none() {
        return None;
    }
    if !state.detail.shown {
        return Some(1);
    }
    Some(match state.detail.focus {
        DetailFocus::Diff => 1,
        DetailFocus::Files => 2,
        DetailFocus::Graph => 3,
    })
}

/// The keys a refusal names to reach the banner: `"⇥ banner"`,
/// `"⇥ ⇥ banner"` — one `⇥` per [`tabs_to_banner`].
#[must_use]
pub(crate) fn tab_keys(tabs: u8) -> String {
    let mut keys = "⇥ ".repeat(usize::from(tabs.max(1)));
    keys.push_str("banner");
    keys
}
