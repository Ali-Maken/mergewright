//! The bars an operation's clashes put the footer in: the focused banner's
//! and the resolver's. Each is one function, read by both the graph's
//! classifier and the frame that draws the footer, so the bar the key
//! sweep checks is the bar on screen.

use graft_term::sanitize_line;
use graft_ui::graph_screen::HintContext;
use graft_ui::graph_screen::hints::{ClashAt, ClashBar};

use crate::update::AppState;
use crate::update::resolver::review::{in_a_merge, next_open_file};

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

/// The resolver's bar: the one context both the classifier
/// (`rows::overlay_context`) and the resolver's own footer
/// (`view::frame::draw_resolver_footer`) draw from (m6-05 review, Major 1).
///
/// A merge's file with nothing left open draws the clash bar
/// ([`merge_solved_bar`]); everything else today's resolver bar.
///
/// `B`/`d` act the moment they are pressed (`update::resolver::toggle_base`
/// / `restart_box`, m6-conflicts/05), so a document passes `true` for both.
/// A whole-file card has no blocks for either to act on, and draws its own
/// shorter list. `restart` is `start_over_refusal`'s answer, the same one
/// `d` itself asks (m6-conflicts/28): the bar offers `d` exactly where it
/// opens the box.
#[must_use]
pub fn resolver_context(state: &AppState) -> HintContext {
    if let Some(bar) = merge_solved_bar(state) {
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

/// In a merge, a marker document on screen with every clash picked draws
/// the clash bar (ease-pass/05c): `⏎` marks nothing
/// in a merge, so the bar points on instead — `n` to the next file a clash
/// may be open in (`FileSolved`), or, none left, `y` to the review
/// (`AllSolved`). `None` outside a merge, while the file is still open or
/// its read is out, on a whole-file card, and on a done file.
fn merge_solved_bar(state: &AppState) -> Option<ClashBar> {
    if !in_a_merge(state) {
        return None;
    }
    let open = state.resolver.as_ref()?;
    // A done (staged) file keeps its read-only bar: `d` starts it over.
    let solved = open.done.is_none() && open.file.as_ref().is_some_and(|file| file.open() == 0);
    if !solved {
        return None;
    }
    let at = match next_open_file(state) {
        Some(next) => ClashAt::FileSolved {
            next: sanitize_line(&next.display().to_string()),
        },
        None => ClashAt::AllSolved,
    };
    Some(ClashBar {
        at,
        roles: crate::update::resolver::side_names(state).roles,
    })
}
