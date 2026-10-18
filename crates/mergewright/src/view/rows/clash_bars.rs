//! The bars an operation's clashes put the footer in: the focused banner's
//! and the resolver's. Each is one function, read by both the graph's
//! classifier and the frame that draws the footer, so the bar the key
//! sweep checks is the bar on screen.

use mergewright_term::sanitize_line;
use mergewright_ui::graph_screen::HintContext;
use mergewright_ui::graph_screen::hints::{ClashAt, ClashBar};

use crate::update::AppState;
use crate::update::resolver::review::{in_a_merge, next_open_file};

/// The hint context a focused banner puts the footer in. One function
/// rather than two literals: the tab layout draws the banner's hints
/// outside the graph screen (`view::frame`), and the two must not drift.
///
/// `merge` comes from the live operation's kind, begun in Mergewright or in the
/// shell: `c` opens the finish summary on both, so the bar says `c finish`
/// (m6-conflicts/70, n2).
///
/// A conflicted stop whose resolver has been open is **paused** (Q7 a,
/// ease-pass/05c): `BannerPaused`, leading `⏎ back to the clashes`. A
/// launch onto a live operation has not opened it yet: `⏎ resolve`.
pub(crate) fn banner_context(
    state: &AppState,
    banner: &mergewright_ui::graph_screen::BannerView,
) -> HintContext {
    use mergewright_git::state::InProgress;
    let merge = crate::update::commit_editor::live_operation(state)
        .is_some_and(|(kind, _)| kind == InProgress::Merge);
    let conflicted = banner.conflict.is_some();
    if conflicted && state.picks.resolver_seen() {
        return HintContext::BannerPaused { merge };
    }
    HintContext::Banner {
        skippable: banner.skippable,
        conflicted,
        merge,
    }
}

/// The resolver's bar: the one context both the classifier
/// (`rows::overlay_context`) and the resolver's own footer
/// (`view::frame::draw_resolver_footer`) draw from (m6-05 review, Major 1).
///
/// A marker document draws the clash bar ([`clash_bar`], ease-pass/11);
/// everything else today's resolver bar.
///
/// `B`/`d` act the moment they are pressed (`update::resolver::toggle_base`
/// / `restart_box`, m6-conflicts/05), so a document passes `true` for both.
/// A whole-file card has no blocks for either to act on, and draws its own
/// shorter list. `restart` is `start_over_refusal`'s answer, the same one
/// `d` itself asks (m6-conflicts/28): the bar offers `d` exactly where it
/// opens the box.
#[must_use]
pub fn resolver_context(state: &AppState) -> HintContext {
    if let Some(bar) = clash_bar(state) {
        return HintContext::Clash(bar);
    }
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

/// The clash bar for the marker document on screen (ease-pass/11, mock
/// M11), sides named by the operation's roles:
///
/// - `Open`: the cursor's clash is open — the picks.
/// - `Picked`: the cursor's clash is taken, the file still has one open.
/// - `FileSolved`: nothing open here, a clash may be open in `next` (one
///   open, or a file not read yet) — `n` goes there.
/// - `AllSolved`, in a merge: nothing open anywhere — `y` reviews.
///
/// `None` — today's resolver bar — on a whole-file card, a done (staged)
/// file (`d` starts it over), while the file's read is out, and outside a
/// merge once nothing is open anywhere: there the file waits for `⏎` and
/// `c` refuses until every file is marked, which the resolver bar's
/// `finishable` already judges (m6-conflicts/31).
#[must_use]
pub fn clash_bar(state: &AppState) -> Option<ClashBar> {
    let open = state.resolver.as_ref()?;
    if open.done.is_some() || open.whole_file.is_some() {
        return None;
    }
    let file = open.file.as_ref()?;
    let at = if file.open() > 0 {
        let cursor_open = open
            .cursor_block
            .and_then(|index| file.blocks().into_iter().find(|at| at.index == index))
            .is_none_or(|at| at.state == mergewright_git::conflict::BlockState::Open);
        if cursor_open {
            ClashAt::Open
        } else {
            ClashAt::Picked
        }
    } else {
        match next_open_file(state) {
            Some(next) => ClashAt::FileSolved {
                next: sanitize_line(&next.display().to_string()),
            },
            None if in_a_merge(state) => ClashAt::AllSolved,
            None => return None,
        }
    };
    Some(ClashBar {
        at,
        roles: crate::update::resolver::side_names(state).roles,
    })
}
