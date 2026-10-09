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

/// **The one press that reaches the banner** for the graph bar
/// (`HintContext::OperationLive`, m8/20d-b): one `⇥` where
/// [`one_tab_to_banner`] holds, else one `⇧⇥` where [`banner_way`] counts
/// one back and `app.prev_pane` is bound (the graph with the pane open).
/// `None`: no single bound press gets there, so the bar names none.
#[must_use]
pub fn one_press_to_banner(state: &AppState) -> Option<graft_ui::graph_screen::WayToBanner> {
    use graft_ui::graph_screen::WayToBanner;
    if one_tab_to_banner(state) {
        return Some(WayToBanner::Forward);
    }
    let back_bound = state
        .keymap
        .spell_keys(&[graft_ui::keymap::ActionId("app.prev_pane")])
        .is_some();
    (!state.modal_owns_keys() && back_bound && banner_way(state)?.back == Some(1))
        .then_some(WayToBanner::Back)
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
    let (at, shown, banner_drawn) = focus_now(state)?;
    next_of(at, shown, banner_drawn)
}

/// Where the keyboard is, whether the pane is open and whether a banner is
/// drawn; `None` while a modal (other than the focused banner) owns the keys.
fn focus_now(state: &AppState) -> Option<(TabTarget, bool, bool)> {
    let banner_drawn = crate::update::banner::banner_view(state).is_some();
    if state.banner_focus && banner_drawn {
        return Some((TabTarget::Banner, state.detail.shown, true));
    }
    if state.modal_owns_keys() {
        return None;
    }
    let (at, shown) = pane_place(state);
    Some((at, shown, banner_drawn))
}

/// Where the keyboard is among the panes (banner aside), as the cycle
/// names it, and whether the detail pane is open.
pub(crate) fn pane_place(state: &AppState) -> (TabTarget, bool) {
    use graft_ui::detail::DetailFocus;
    let shown = state.detail.shown;
    let at = match (shown, state.detail.focus) {
        (true, DetailFocus::Files) => TabTarget::Files,
        (true, DetailFocus::Diff) => TabTarget::Diff,
        _ => TabTarget::Graph,
    };
    (at, shown)
}

/// **The cycle, as one function**: where one `Tab` goes from `at`. Graph →
/// Files → Diff → (banner →) Graph with the pane open; Graph → banner →
/// Graph with it closed. Every other reading — `⇧⇥` ([`prev_of`]), the
/// counts ([`banner_way`]) — is derived from this one.
pub(crate) fn next_of(at: TabTarget, shown: bool, banner_drawn: bool) -> Option<TabTarget> {
    use TabTarget::{Banner, Diff, Files, Graph};
    match (at, shown) {
        (Graph, false) => banner_drawn.then_some(Banner),
        (Graph, true) => Some(Files),
        (Files, _) => Some(Diff),
        (Diff, _) if banner_drawn => Some(Banner),
        (Banner | Diff, _) => Some(Graph),
    }
}

/// **Where one `⇧⇥` goes from `at`**: the place whose [`next_of`] is `at`
/// — keymap §2.2's "previous pane", the same cycle run backwards (m8/20d).
/// Derived, never a second hand-kept order.
pub(crate) fn prev_of(at: TabTarget, shown: bool, banner_drawn: bool) -> Option<TabTarget> {
    use TabTarget::{Banner, Diff, Files, Graph};
    [Graph, Files, Diff, Banner]
        .into_iter()
        .filter(|&from| shown || matches!(from, Graph | Banner))
        .filter(|&from| banner_drawn || from != Banner)
        .find(|&from| from != at && next_of(from, shown, banner_drawn) == Some(at))
}

/// How many presses reach the banner from where the keyboard is, each way
/// (m8/20d). `back` is `None` when `⇧⇥` never gets there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BannerWay {
    /// `Tab`s (`app.next_pane`).
    pub forward: u8,
    /// `⇧⇥`s (`app.prev_pane`).
    pub back: Option<u8>,
}

impl BannerWay {
    /// `n` presses forward and no way back: a way read off a count alone.
    #[must_use]
    pub const fn tabs(n: u8) -> Self {
        Self {
            forward: n,
            back: None,
        }
    }
}

/// **The way to the banner**, counted by stepping [`next_of`] and
/// [`prev_of`] — the fns the `Tab` and `⇧⇥` handlers step — from the pane
/// the keyboard is on (under a text field or box, the pane `esc` hands it
/// back to). `None` when no banner is drawn or it already has the keyboard.
#[must_use]
pub(crate) fn banner_way(state: &AppState) -> Option<BannerWay> {
    if state.banner_focus || crate::update::banner::banner_view(state).is_none() {
        return None;
    }
    let (at, shown) = pane_place(state);
    let count = |step: fn(TabTarget, bool, bool) -> Option<TabTarget>| {
        let mut here = at;
        for presses in 1..=4u8 {
            here = step(here, shown, true)?;
            if here == TabTarget::Banner {
                return Some(presses);
            }
        }
        None
    };
    Some(BannerWay {
        forward: count(next_of)?,
        back: count(prev_of),
    })
}

/// **How many `Tab`s from here reach the banner**: [`banner_way`]'s
/// forward count. From the file list it is two (the diff, then the banner).
#[must_use]
pub(crate) fn tabs_to_banner(state: &AppState) -> Option<u8> {
    banner_way(state).map(|way| way.forward)
}

/// The keys a refusal names to reach the banner: `⇥ banner`, `⇥ ⇥ banner`
/// — one `app.next_pane` key per `tabs`, spelled from
/// `keys` (10b W1; m8/20b R5). The one speller of every "go to the banner"
/// clause going forward ([`way_keys`] picks the way). `None` when `tabs` is 0 or `app.next_pane` is unbound: the
/// clause drops (W3).
#[must_use]
pub(crate) fn tab_keys(tabs: u8, keys: &graft_ui::keymap::Keymap) -> Option<String> {
    if tabs == 0 {
        return None;
    }
    let key = keys.spell_keys(&[graft_ui::keymap::ActionId("app.next_pane")])?;
    let mut words = format!("{key} ").repeat(usize::from(tabs));
    words.push_str("banner");
    Some(words)
}

/// **The keys that reach the banner, the shorter way** (m8/20d): `⇧⇥
/// banner` from the graph with the pane open, `⇥ ⇥ banner` from the file
/// list (a tie goes forward). One direction unbound → the other; both →
/// `None` (the clause drops, W3).
#[must_use]
pub(crate) fn way_keys(way: BannerWay, keys: &graft_ui::keymap::Keymap) -> Option<String> {
    let forward = tab_keys(way.forward, keys).map(|words| (way.forward, words));
    let back = way.back.filter(|&n| n > 0).and_then(|n| {
        let key = keys.spell_keys(&[graft_ui::keymap::ActionId("app.prev_pane")])?;
        let mut words = format!("{key} ").repeat(usize::from(n));
        words.push_str("banner");
        Some((n, words))
    });
    match (forward, back) {
        (Some((f, words)), Some((b, _))) if f <= b => Some(words),
        (_, Some((_, words))) | (Some((_, words)), None) => Some(words),
        (None, None) => None,
    }
}
