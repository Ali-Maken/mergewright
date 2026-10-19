//! The branch pane's view (ease-pass 09, G12): the focused branch, its four
//! answers in the mock's words, and the route it was built on, trunk first.
//! The read behind it is `update::branch_pane`'s; this only words it.

use mergewright_graph::Route;
use mergewright_ops::branch_status::{
    Behind, BranchStatus, CandidateKind, Merged, Pushed, Who, short_name,
};
use mergewright_term::sanitize_line;
use mergewright_ui::branch_pane::{BranchPaneView, Mark, PaneLine, RouteStop};
use mergewright_ui::graph_screen::{first_name, relative_date};

use crate::history::History;
use crate::refs::RefLabel;
use crate::update::AppState;
use crate::update::branch_pane::{StatusRead, focused, identity_of};

/// What the pane shows for the cursor row: its branch's answers, or that
/// the commit is on no branch. `None` while the detail pane is open (it
/// replaces this one) or before the cursor has a row.
pub(crate) fn pane_view(state: &AppState, history: &History, now: i64) -> Option<BranchPaneView> {
    if !has_pane(state) {
        return None;
    }
    let row = history.selected()?;
    let Some(focused) = focused(state) else {
        let hex = row.commit.id.to_string();
        return Some(BranchPaneView::NotOnBranch {
            sha7: sanitize_line(hex.get(..7).unwrap_or(&hex)),
        });
    };
    let short = short_name(&focused.branch.name);
    let here = state.refs.as_deref().is_some_and(|refs| {
        refs.on(focused.branch.tip).iter().any(|label| {
            matches!(label, RefLabel::Branch { name, current: true, .. } if name == short.as_bytes())
        })
    });
    let route = super::entries::frame_route(state, history);
    let trunk = focused
        .candidates
        .iter()
        .find(|candidate| candidate.kind == CandidateKind::Trunk)
        .map_or("", |trunk| short_name(&trunk.name));
    let (stops, lines) = match state.look.reads.get(&focused.key) {
        Some(StatusRead::Read(status)) => (
            Some(stops(status, &focused.branch, &route)),
            pane_words(status, focused.branch.kind, trunk, now),
        ),
        Some(StatusRead::Failed(reason)) => (None, failed(reason)),
        // Asked and not landed, or not asked yet (mid-burst).
        Some(StatusRead::Reading) | None => (None, std::array::from_fn(|_| PaneLine::Reading)),
    };
    Some(BranchPaneView::Branch {
        heading: sanitize_line(short),
        role: route.role(identity_of(&focused.branch)),
        here,
        route: stops,
        lines,
    })
}

/// The graph view carries a pane: the detail pane is closed (it replaces
/// this one) and the cursor is on a row. Whether it is drawn is the width's
/// call (`branch_pane::pane_width`).
fn has_pane(state: &AppState) -> bool {
    !state.detail.shown
        && state
            .history
            .as_ref()
            .is_some_and(|history| history.selected().is_some())
}

/// The columns the pane's slot takes off the graph's rows at this width: its
/// width, its `│` rule and a space (`graph_screen::ScreenRows::pane`); 0 with
/// no pane. For the arithmetic that builds no graph view: the lane cap and
/// the mouse.
pub(crate) fn pane_slot(state: &AppState, columns: u16) -> u16 {
    match mergewright_ui::branch_pane::pane_width(columns) {
        Some(width) if has_pane(state) => width + 2,
        _ => 0,
    }
}

/// The route, trunk first, the branch itself last, each in its role.
fn stops(
    status: &BranchStatus,
    branch: &mergewright_ops::branch_status::Candidate,
    route: &Route,
) -> Vec<RouteStop> {
    status
        .route
        .iter()
        .rev()
        .chain(std::iter::once(branch))
        .map(|stop| RouteStop {
            name: sanitize_line(short_name(&stop.name)),
            role: route.role(identity_of(stop)),
        })
        .collect()
}

/// A failed read: each line says why, and only the pane says it.
fn failed(reason: &str) -> [PaneLine; 4] {
    std::array::from_fn(|_| PaneLine::Failed(sanitize_line(reason)))
}

/// The four answers in the mock's words (G12; 01r's words for the states the
/// mock never shows): pushed, merged, behind, who. `trunk` is the default
/// branch's short name; `now` dates the last commit.
pub(crate) fn pane_words(
    status: &BranchStatus,
    kind: CandidateKind,
    trunk: &str,
    now: i64,
) -> [PaneLine; 4] {
    let line = |mark, words: String| PaneLine::Known {
        mark,
        words: sanitize_line(&words),
    };
    let (mark, words) = pushed(status.pushed);
    let pushed = line(mark, words);
    let merged = match &status.merged {
        Merged::MainLine => line(Mark::Plain, "the main line: branches merge into it".into()),
        Merged::Into(into) => line(Mark::Fine, format!("merged into {}", short_name(into))),
        Merged::NotYet { into: Some(into) } if kind == CandidateKind::Integration => {
            line(Mark::Plain, format!("not merged into {}", short_name(into)))
        }
        Merged::NotYet { .. } => line(Mark::Plain, "not merged".into()),
        Merged::NothingToMerge => line(Mark::Plain, "nothing of its own to merge".into()),
    };
    let behind = match &status.behind {
        Behind::NoBase if kind == CandidateKind::Trunk => {
            line(Mark::Plain, "no upstream to compare with".into())
        }
        Behind::NoBase => line(Mark::Plain, format!("shares no history with {trunk}")),
        Behind::UpToDate { base } => {
            line(Mark::Fine, format!("up to date with {}", short_name(base)))
        }
        Behind::Behind { base, count } => line(
            Mark::Attention,
            format!(
                "{} has {count} new {} since it left",
                short_name(base),
                commits(*count)
            ),
        ),
    };
    let who = match &status.who {
        Who::Everyone { last } => line(
            Mark::Plain,
            format!("everyone, {}", last_commit(last.seconds, now)),
        ),
        Who::Authors {
            first,
            others,
            last,
        } => line(
            Mark::Plain,
            format!(
                "{}, {}",
                authors(first, *others),
                last_commit(last.seconds, now)
            ),
        ),
        Who::NoOwnCommits => line(Mark::Plain, "no commits of its own".into()),
    };
    [pushed, merged, behind, who]
}

fn pushed(pushed: Pushed) -> (Mark, String) {
    match pushed {
        Pushed::NoUpstream => (Mark::Attention, "not pushed yet".into()),
        Pushed::InSync => (Mark::Fine, "pushed".into()),
        Pushed::ToPush(n) => (Mark::Attention, format!("{n} {} to push", commits(n))),
        Pushed::ToPull(n) => (Mark::Attention, format!("{n} new {} to pull", commits(n))),
        Pushed::Both { push, pull } => (Mark::Attention, format!("{push} to push, {pull} to pull")),
    }
}

fn commits(n: usize) -> &'static str {
    if n == 1 { "commit" } else { "commits" }
}

/// `last commit 2w ago`; `just now` and `in the future` read as words.
fn last_commit(then: i64, now: i64) -> String {
    match relative_date(then, now).as_str() {
        "now" => "last commit just now".to_owned(),
        "future" => "last commit in the future".to_owned(),
        age => format!("last commit {age} ago"),
    }
}

/// `Sara`, `Sara and Omar`, `Sara, Omar and 3 others`: first names only.
fn authors(first: &[Vec<u8>], others: usize) -> String {
    let given: Vec<String> = first
        .iter()
        .map(|name| first_name(&String::from_utf8_lossy(name)).to_owned())
        .collect();
    match (given.as_slice(), others) {
        ([], _) => format!("{others} {}", if others == 1 { "person" } else { "people" }),
        ([one], 0) => one.clone(),
        ([rest @ .., last], 0) => format!("{} and {last}", rest.join(", ")),
        (named, 1) => format!("{} and 1 other", named.join(", ")),
        (named, more) => format!("{} and {more} others", named.join(", ")),
    }
}

#[cfg(test)]
mod tests;
