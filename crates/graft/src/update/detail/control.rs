//! Job-id minting, routing a detail job's [`AppEvent`] to [`DetailState`],
//! the pane's effective layout, and the Explain panel's open/close.

use graft_core::job::JobId;
use graft_core::oid::Oid;

use crate::history::History;
use crate::update::events::{AppEvent, Command, Product};
use crate::update::state::AppState;

use super::LOCAL_JOB_BIT;
use super::state::{DetailSpec, DetailState, Loading, PaneSize};
use graft_ui::detail::{DetailFocus, DiffView};
use graft_ui::detail_pane::DetailLayout;

/// Hands out ids for jobs `update` names itself (see [`LOCAL_JOB_BIT`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalJobs(u64);

impl LocalJobs {
    /// The next unused id.
    pub fn next_id(&mut self) -> JobId {
        self.0 += 1;
        JobId::new(LOCAL_JOB_BIT | self.0)
    }
}

/// A detail job's event. The pane names its jobs itself, so a start needs
/// no answer; products are checked against what is still pending.
pub(crate) fn detail_event(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    let AppState {
        detail, local_jobs, ..
    } = next;
    match event {
        AppEvent::Produced(id, Product::Files(result)) => {
            commands.extend(detail.files_arrived(id, result, &mut || local_jobs.next_id()));
        }
        AppEvent::Produced(id, Product::FileDiff(key, result)) => {
            detail.diff_arrived(id, key, result);
        }
        AppEvent::Produced(_, Product::ContainedIn(oid, result)) => {
            detail.contained_arrived(oid, result);
        }
        AppEvent::Produced(id, Product::EvilProbe(oid, verdict)) => {
            detail.evil_probed(id, oid, verdict);
        }
        // Starts, and anything `answer` did not route here.
        AppEvent::Started(..)
        | AppEvent::Produced(..)
        | AppEvent::Progress(..)
        | AppEvent::Core(_) => {}
    }
}

/// The pane's actual layout for `size`: the breakpoint's kind first (the
/// percentage default), then that kind's saved override, if any, folded back
/// in. Two steps because which override applies depends on the kind the
/// first step already picked (`PaneSize::for_layout`).
#[must_use]
pub fn effective_layout(
    focus: DetailFocus,
    pane_size: &PaneSize,
    size: (u16, u16),
) -> DetailLayout {
    let kind = DetailLayout::compute(size.0, size.1, focus, None);
    let override_cells = pane_size.for_layout(&kind);
    DetailLayout::compute(size.0, size.1, focus, override_cells)
}

/// Answers `explain_requested`, if there is one this tick
/// (`m2-details-and-diffs/17`): opens the pane on that commit, then resolves
/// the finding it was marked for and turns it into `DetailState::explain`.
///
/// The `(parents, subject)` lookup is exactly `view::detail::commit_header`'s
/// own — the selected row filtered by id — not a second one: a marked row is
/// always the row a context menu was opened on, which is the selection.
/// Nothing to explain (the marker's finding is stale, e.g. policy changed
/// underneath) clears `explain_requested` and leaves `explain` alone rather
/// than erroring.
pub fn open_explain(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    let Some(oid) = state.detail.explain_requested.take() else {
        return Vec::new();
    };
    let commands = state.detail.open(DetailSpec::Commit(oid), next_id);
    let facts = state
        .history
        .as_ref()
        .and_then(History::selected)
        .filter(|row| row.commit.id == oid)
        .map(|row| {
            (
                row.commit.parents.clone(),
                String::from_utf8_lossy(row.commit.subject()).into_owned(),
            )
        });
    let Some((parents, subject)) = facts else {
        return commands;
    };
    let Some(finding) = crate::view::radar::findings_for(state, oid, &parents, &subject)
        .into_iter()
        .next()
    else {
        return commands;
    };
    let short = |o: Oid| {
        graft_ui::graph_screen::short_sha(
            &o.to_string(),
            usize::from(graft_ui::graph_screen::SHA_WIDTH),
        )
    };
    let explanation = graft_ops::mistake_radar::explain::explain(&finding, &short);
    state
        .detail
        .explain_highlight
        .clone_from(&explanation.highlight);
    let mut view = crate::view::build_explain_view(&explanation);
    view.severity = finding.severity;
    state.detail.explain = Some(view);
    commands
}

/// Close the Explain panel, keeping the pane itself open on the same commit
/// (`Esc`'s first press while Explain is open, `m2-details-and-diffs/17`).
pub fn close_explain(state: &mut AppState) {
    state.detail.explain = None;
    state.detail.explain_highlight.clear();
}

/// Content rows [`graft_ui::detail_pane::diff_body`] draws for `view`: one
/// heading row per hunk plus its non-heading lines, plus a truncation-footer
/// row when the diff was cut short.
pub(crate) fn diff_content_len(view: &DiffView) -> usize {
    view.body_len()
}

/// The diff body's current length, for [`DetailState::scroll_body`]'s clamp;
/// `0` while nothing is loaded, so a scroll key is a harmless no-op then.
pub(crate) fn diff_content_len_of(detail: &DetailState) -> usize {
    match &detail.diff {
        Loading::Ready(diff) => diff_content_len(diff),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => 0,
    }
}
