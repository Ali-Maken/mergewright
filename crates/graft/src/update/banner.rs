//! The operation banner and the conflict interim
//! (m5-history-and-ghost-preview/09).
//!
//! M5 produces conflicts — `m` merge, `r` rebase, the `.` menu's cherry-pick
//! and revert — a whole milestone before M6 builds the three-pane resolver.
//! The honest interim (PROMPT.md §8) is this module: say which operation is
//! live and which files conflict, hand the files to the tool the user already
//! has (`M` their `merge.tool`, `e` their `$EDITOR`), and keep
//! Continue/Skip/Abort reachable the whole time. **Nothing here resolves a
//! conflict.** `KNOWN_GAPS.md` row 236 says so, with M6 as its expiry.
//!
//! Three rules shape the code:
//!
//! - **`a` abort is always offered.** Whatever else is broken — a
//!   `--continue` a hook refused, a mergetool that would not start, a
//!   relaunch into somebody else's half-finished rebase — the way back to
//!   before the operation is one key. Every refusal in this module keeps `a`
//!   in its key hints for that reason.
//! - **The banner is modal while focused** (`docs/design/02-keymap.md` §2.1,
//!   §2.6). It answers every key but `Ctrl+C`, so a `u` or an `S` mid-rebase
//!   cannot start a second thing on top of the first. `Tab`/`BackTab`/`Esc`
//!   leave, and the hint bar says so, so it is a visible mode and not a
//!   hidden one.
//! - **A live operation is one journal entry, `Pending` from the stop.**
//!   Nothing here writes a journal line. `graft_ops::apply::resume` writes
//!   the outcome line when the operation finishes or aborts, and a resolve
//!   step folds into the live entry rather than opening its own
//!   (`.scratch/m5-history-and-ghost-preview/as-built.md` §1).

use std::path::{Path, PathBuf};

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::state::{InProgress, InProgressDetail};
use graft_ops::apply::Resume;
use graft_term::sanitize_line;
use graft_ui::graph_screen::{BannerView, RewriteBannerView};

use crate::jobs::operate::Operated;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::events::Command;
use super::finish::ReturnTo;
use super::state::{AppState, Status};

mod stop;

pub use stop::BannerWay;
pub(super) use stop::land_in_resolver;
use stop::stopped_again;
pub(super) use stop::{ArrivalWatch, finished_lands_on, finished_lands_on_head, lands_on_changes};
pub use stop::{
    RewriteStop, StopLadder, one_press_to_banner, one_tab_to_banner, stop_line, stop_line_wordings,
    stop_reason, stop_wordings,
};
pub(crate) use stop::{TabTarget, banner_way, tab_target, way_keys};
pub(super) use stop::{
    edit_stop_lands_on_head, refs_answered, relaunch_lands_on_head, walk_started,
};
pub(crate) use stop::{edit_stop_of, progress_unread, read_rewrite_stop, stop_takes_focus};

mod focus_keys;
pub(super) use focus_keys::banner_press;
pub(crate) use focus_keys::{abort_key, continue_abort};
mod pressed;
mod resolve_words;
mod resume;
pub use focus_keys::{banner_key, take_focus};
use pressed::{abort_pressed, skip_pressed};
pub(crate) use pressed::{continue_pressed, continued_in_git};
pub(super) use pressed::{continue_refused, land_after_continue_in_git};
pub(crate) use resolve_words::{RefusedAt, refused_at, resolve_keys};
pub use resume::{cursor_conflict, edit_conflict, open_mergetool, resume, resumed};
pub(crate) use resume::{edit_or_mergetool, mergetool_fallback};
#[cfg(test)]
use resume::{kept_open, resumed_words};

/// The banner's word for an operation, as the header draws it.
///
/// [`InProgress::Bisect`] has none: a bisect is not a history operation with
/// a Continue/Skip/Abort shape (`git bisect good/bad` is a different
/// question), and bisect banners are M7's. Answering `None` here is what
/// keeps `banner_key` from ever claiming the keyboard during one.
fn kind_word(kind: InProgress) -> Option<&'static str> {
    match kind {
        InProgress::Merge => Some("MERGING"),
        InProgress::Rebase => Some("REBASING"),
        InProgress::CherryPick => Some("CHERRY-PICKING"),
        InProgress::Revert => Some("REVERTING"),
        InProgress::Bisect => None,
    }
}

/// The operation live right now, when it is one this module can act on.
///
/// Also what `update::undo` asks before it refuses `u`: the refusal points
/// at the banner, so it must only be given when there is a banner to point
/// at (a bisect has none).
pub(super) fn live(state: &AppState) -> Option<&InProgressDetail> {
    let detail = state.in_progress.as_ref()?;
    kind_word(detail.kind).map(|_| detail)
}

/// Every unmerged path in the last working-tree read, in git's own order.
#[must_use]
pub fn unmerged_paths(state: &AppState) -> Vec<PathBuf> {
    let Some(status) = state.working_tree.as_deref() else {
        return Vec::new();
    };
    status
        .entries
        .iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path.clone())
        .collect()
}

/// The first conflicted path, which is what `M` and `e` act on from the
/// banner — the banner has no cursor of its own, so "the first one" is the
/// only thing it can mean. The changes pane's `M`/`e` act on the row under
/// its cursor instead.
#[must_use]
pub fn first_conflicted(state: &AppState) -> Option<PathBuf> {
    unmerged_paths(state).into_iter().next()
}

/// Whether this frame draws the graph screen at all — and so whether the
/// operation banner already has a header to ride on.
///
/// No longer a mirror of `view::frame::draw_screen`'s match: both read
/// `AppState::graph_overlay`, the one enumeration of the overlays that
/// replace the graph (review M3, m5/31). It used to restate the four
/// conditions, and a fifth overlay added to the frame would have left this
/// answering `true` for a frame with no graph on it — the answer decides a
/// *row count*, and a row count the mouse disagrees with is a click landing
/// on the wrong row.
#[must_use]
pub fn graph_screen_on_frame(state: &AppState) -> bool {
    state.screen == crate::update::state::Screen::Graph && state.graph_overlay().is_none()
}

/// Rows the sub-100-column tab layout spends on the operation banner:
/// **0**, **1** for the row above the `[Graph] Detail` strip, or **2** with
/// its key hints on the bottom row while it has focus.
///
/// `0` whenever the frame already draws the banner somewhere else. Below
/// 100 columns the shown tab takes the whole frame, so with the **Detail**
/// tab up the graph screen is not drawn and the banner has nowhere to be —
/// that is what `draw_tab_banner` exists for (m5/16, review C1). With the
/// **Graph** tab up the graph screen *is* drawn, header and footer and all,
/// and drawing the tab banner too put the same text on two rows and the
/// same keys on two more (review M2).
///
/// The one number `view::frame::split_for_detail` draws from and
/// `update::geometry` and `AppState::graph_rows` count from, so a click
/// cannot land on a row the banner moved (review M1).
#[must_use]
pub fn tab_banner_rows(state: &AppState) -> u16 {
    if !state.detail.shown {
        return 0;
    }
    let size = (state.size.columns, state.size.rows);
    let layout = super::detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
    let graft_ui::detail_pane::DetailLayout::Tab { shown } = layout else {
        return 0;
    };
    if banner_view(state).is_none() {
        return 0;
    }
    if !shown && graph_screen_on_frame(state) {
        return 0;
    }
    1 + u16::from(state.banner_focus)
}

/// Rows the sub-100-column tab layout puts above the **graph** when the
/// graph is the tab on screen: the `[Graph] Detail` strip, and the
/// operation banner's own row above it if this frame draws one there.
///
/// `0` in every other layout, and `0` with the Detail tab up — the graph
/// is not drawn at all then. Counted by `AppState::graph_rows` and by
/// `update::geometry::rows_above_graph`, which is what puts a click on the
/// row the user is looking at.
#[must_use]
pub fn tab_rows_above_graph(state: &AppState) -> u16 {
    if !state.detail.shown {
        return 0;
    }
    let size = (state.size.columns, state.size.rows);
    let layout = super::detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
    match layout {
        // The strip's own row, plus the banner's if it is drawn above it.
        // The banner's *second* row (its key hints while focused) comes off
        // the bottom, not the top, so it is not here — `graph_rows` counts
        // it through `tab_banner_rows` instead.
        graft_ui::detail_pane::DetailLayout::Tab { shown: false } => {
            1.min(size.1) + 1.min(tab_banner_rows(state))
        }
        _ => 0,
    }
}

/// The banner the header draws, or `None` when no operation is live.
#[must_use]
pub fn banner_view(state: &AppState) -> Option<BannerView> {
    let detail = live(state)?;
    let kind = kind_word(detail.kind)?;
    let paths = unmerged_paths(state);
    let mut view = BannerView {
        kind: sanitize_line(kind),
        step: detail.step,
        conflict: paths
            .first()
            .map(|path| sanitize_line(&path.display().to_string())),
        focused: state.banner_focus,
        // A merge replays nothing, so it has no step to leave out. Offering
        // `s` on one would be a key that runs `git merge --skip`, which does
        // not exist.
        skippable: detail.kind != InProgress::Merge,
        rewrite: None,
    };
    // A rewrite's stop words itself from the todo (07b): the step counts his
    // rows only, and the row and branch follow it. A failed read, or none,
    // keeps today's words.
    if let Some(rewrite) = state
        .stopped
        .as_ref()
        .filter(|stop| stop.kind == detail.kind)
        .and_then(|stop| stop.rewrite.as_ref())
        .and_then(|read| read.as_ref().ok())
    {
        let progress = &rewrite.progress;
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        view.step = Some((count(progress.step.0), count(progress.step.1)));
        view.rewrite = progress.current.as_ref().map(|row| RewriteBannerView {
            action: row.action.word().to_owned(),
            sha7: row.commit.to_string().chars().take(7).collect(),
            subject: sanitize_line(&row.subject).as_str().to_owned(),
            // Inside a fork's section the step is the fork's, not the line
            // branch's (ADR-0059 §3).
            chain: match (&progress.fork, &progress.branch) {
                (Some(fork), _) => Some(graft_ui::graph_screen::BannerChain::Fork {
                    branch: sanitize_line(&fork.name).as_str().to_owned(),
                    at: fork.at.to_string().chars().take(7).collect(),
                }),
                (None, Some((name, index, of))) => {
                    let name = String::from_utf8_lossy(name.shorthand());
                    Some(graft_ui::graph_screen::BannerChain::Line {
                        branch: sanitize_line(&name).as_str().to_owned(),
                        index: *index,
                        of: *of,
                    })
                }
                (None, None) => None,
            },
            edit_hint: rewrite.edit.is_some(),
        });
    }
    Some(view)
}

/// The still-to-apply block at a rewrite stop (07c): this branch's pending
/// pick-like rows (`pending_here`), newest first, capped at
/// [`graft_ui::still_to_apply::MAX_ROWS`] with the rest counted, then each
/// branch of the line still to come as `<branch> (<author>): N commits
/// follow` (07d). `None` when no rewrite is stopped, its progress read
/// failed (the banner already says so), or nothing is pending.
#[must_use]
pub fn pending_view(state: &AppState) -> Option<graft_ui::PendingView> {
    let detail = live(state)?;
    let rewrite = state
        .stopped
        .as_ref()
        .filter(|stop| stop.kind == detail.kind)?
        .rewrite
        .as_ref()?
        .as_ref()
        .ok()?;
    if rewrite.progress.pending.is_empty() {
        return None;
    }
    let pending = rewrite.progress.pending_here();
    let cap = graft_ui::still_to_apply::MAX_ROWS;
    let rows = pending
        .iter()
        .rev()
        .take(cap)
        .map(|row| graft_ui::PendingRowView {
            action: row.action.word().to_owned(),
            sha7: row.commit.to_string().chars().take(7).collect(),
            subject: sanitize_line(&row.subject).as_str().to_owned(),
        })
        .collect();
    Some(graft_ui::PendingView {
        rows,
        more: pending.len().saturating_sub(cap),
        followers: rewrite
            .progress
            .followers
            .iter()
            .map(|f| {
                (
                    sanitize_line(&f.branch).as_str().to_owned(),
                    sanitize_line(&f.tip_author).as_str().to_owned(),
                    f.commits,
                )
            })
            .collect(),
    })
}

/// Screen rows the still-to-apply block takes from the graph body this
/// frame: the `pending` band of `graft_ui`'s own carve
/// (`geometry::graph_carve`) — the block's own height, capped at half the
/// rows left under the banner and the tour card, and none when that is
/// under three (review m7-10-to-07c Major 2). Read by
/// `geometry::rows_above_graph` and `AppState::graph_rows`, as
/// `preview_rows_taken` is.
#[must_use]
pub fn pending_rows_taken(state: &AppState) -> u16 {
    super::geometry::graph_carve(state).pending.3
}

/// Why `c` will not run yet, or `None` when it would.
///
/// `git <kind> --continue` refuses with its own message while paths are
/// unmerged; saying so here costs no job and names the three keys that get
/// the user out of it, which git's message does not.
#[must_use]
pub fn continue_refusal(state: &AppState) -> Option<String> {
    let count = unmerged_paths(state).len();
    if count == 0 {
        return None;
    }
    let files = if count == 1 { "file" } else { "files" };
    Some(format!("{count} {files} still conflicted"))
}

/// The keys a stopped-again or failed resume offers. `a` is in every one of
/// them: the way back is never more than one key away.
const STUCK_KEYS: &str = "M mergetool  e edit  a abort";

/// Whether `kind` has a banner to focus at all.
///
/// A bisect does not: it has no Continue/Skip/Abort shape, so focusing one
/// would claim the keyboard for a mode with no keys in it.
#[must_use]
pub fn banner_focusable(kind: InProgress) -> bool {
    kind_word(kind).is_some()
}

/// The key words the status line carries when it holds the footer row over
/// a focused banner: `c finishes the merge`, or `c continues the <kind>`.
/// `None` for a bisect, which has no banner and so no `c` (ticket 41).
///
/// One source for `resolver::every_file_done` and
/// `undo::land_after_undo`, so the two lines cannot word `c` apart.
pub(crate) fn continue_words(kind: InProgress) -> Option<String> {
    match kind {
        InProgress::Merge => Some("c finishes the merge".to_owned()),
        InProgress::Rebase | InProgress::CherryPick | InProgress::Revert => {
            Some(format!("c continues the {}", kind.name()))
        }
        InProgress::Bisect => None,
    }
}

#[cfg(test)]
pub(in crate::update) mod rewrite_stop_tests;
#[cfg(test)]
pub(in crate::update) mod tests;
