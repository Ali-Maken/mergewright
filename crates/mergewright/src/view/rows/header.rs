//! The graph header's parts: the filter words, the rows a frame draws
//! (live or a preview's), and the location line.

use mergewright_term::{sanitize_line, sanitize_path, sanitize_path_bytes};
use mergewright_ui::graph_column::{NodeKind, RowPaint};
use mergewright_ui::graph_screen::{Badge, BadgeKind, Filters, GraphEntry, Location};
use std::path::{Path, PathBuf};

use crate::history::History;
use crate::repository::{Head, RepositoryFacts};
use crate::update::AppState;

use super::reveal_index;

/// Marks each drawn row only a stash reaches (m7-power-tools/16, note
/// "Pass 2a" R7): node `□`, the row dimmed, `only in stash@{n}` last among
/// its badges — last, so `update::geometry::badge_at`, which counts refs
/// from the front, never takes it for one. The cursor's highlight and a
/// drag's paint win over the dimming: the row is still one the cursor can
/// stand on. The pixel tier reads none of this (`view::strips`, ticket NOTES).
/// The header's filter words: what the walk leaves out, and the focus.
pub(super) fn filters(state: &AppState, history: &History) -> Filters {
    Filters {
        first_parent: history.view.follow == mergewright_graph::Follow::FirstParent,
        stash_hidden: !history.view.inclusion.hidden_stashes.is_empty(),
        focused: state.focus,
        every_branch: history.merged_branches() == crate::config::MergedBranches::Shown,
    }
}

pub(super) fn mark_stash_only(state: &AppState, rows: &mut [GraphEntry<'_>]) {
    if state.stash_only.commits.is_empty() {
        return;
    }
    for entry in rows {
        let Some(n) = state.stash_only.entry(entry.line.row.commit.id) else {
            continue;
        };
        entry.line.node = NodeKind::StashOnly;
        if entry.line.paint == RowPaint::Normal {
            entry.line.paint = RowPaint::Dimmed;
        }
        entry.badges.push(Badge {
            name: sanitize_line(""),
            kind: BadgeKind::OnlyInStash(n),
        });
    }
}

/// A footer rung, or `None` while a modal owns the bottom row
/// (`AppState::modal_owns_bottom_row`): the modal's keys stand there, not
/// the graph's search, compare or stop line.
pub(super) fn modal_blanked<'a, T>(
    state: &'a AppState,
    rung: impl FnOnce(&'a AppState) -> Option<T>,
) -> Option<T> {
    if state.modal_owns_bottom_row() {
        None
    } else {
        rung(state)
    }
}

/// The preview whose picture — ghost rows and all — the graph draws this
/// frame instead of the live window: one is open and its plan has landed.
/// Asked by [`rows_for_frame`] for the rows and by `view::graph_tier` for
/// the tier (ADR-0060), so the two cannot disagree on when it is drawn.
pub(in crate::view) fn preview_drawn(
    state: &AppState,
) -> Option<&crate::update::preview::PreviewState> {
    state
        .preview
        .as_ref()
        .filter(|preview| !preview.rows.is_empty())
}

/// The rows this frame draws, and which of them reveals its refs on hover.
///
/// A preview is modal: while one is open the graph draws its slice instead
/// of the live window, so the picture the box describes and the rows on
/// screen are the same picture (design note hard choice 1). There is no
/// hovered row over a picture, and the choice cursor lives in the box, so
/// nothing is revealed — the selected commit is painted by
/// `preview_entries` itself.
pub(super) fn rows_for_frame<'a>(
    state: &'a AppState,
    history: &'a History,
    live: Vec<GraphEntry<'a>>,
    now_seconds: i64,
) -> (Vec<GraphEntry<'a>>, Option<usize>) {
    if let Some(preview) = preview_drawn(state) {
        return (
            super::super::preview::preview_entries(state, preview, now_seconds),
            None,
        );
    }
    let reveal = reveal_index(state, history, &live);
    (live, reveal)
}

/// Characters of a detached `HEAD`'s id the header shows (ADR-0058 §4,
/// `detached <sha7>`).
const HEADER_SHA: usize = 7;

/// Where the header says the user is: the repository's name, its path with
/// `home` written `~`, and the branch — or `detached <sha7>` (DESIGN.md
/// §1.0, ADR-0058 §4). Every string is sanitised on the way.
pub(in crate::view) fn location(facts: &RepositoryFacts, home: Option<&Path>) -> Location {
    let repo = facts.path.file_name().map_or_else(
        || sanitize_path(&facts.path),
        |name| sanitize_path(Path::new(name)),
    );
    let branch = match &facts.head {
        Head::Branch(name) | Head::Unborn(name) => sanitize_path_bytes(name),
        Head::Detached(id) => {
            let hex = id.to_string();
            sanitize_line(&format!("detached {}", &hex[..HEADER_SHA.min(hex.len())]))
        }
        Head::Unreadable(why) => sanitize_line(&format!("HEAD unreadable: {why}")),
    };
    Location {
        repo,
        path: sanitize_path(&home_relative(&facts.path, home)),
        branch,
        // Set by `practised` when the tour is on (m8/19b).
        practice: false,
    }
}

/// The header in the practice repository (ADR-0056, mock-ups §1.0): `◇`,
/// and `practice · <tmp> · a throwaway repo, not yours` for its path.
pub(in crate::view) fn practised(mut at: Location, state: &AppState) -> Location {
    if state.tour.is_some() {
        at.practice = true;
        at.path = sanitize_line(&super::super::tour::practice_path(at.path.as_str()));
    }
    at
}

/// `path` with a leading `home` written `~`, the way a shell prompt shows
/// it. A path outside `home` (or no `home` at all) is returned whole.
pub(crate) fn home_relative(path: &Path, home: Option<&Path>) -> PathBuf {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        // Only a real directory counts: `HOME=/` would turn every path
        // into `~/…`, which says nothing.
        Some(rest) if home.is_some_and(|home| home.parent().is_some()) => Path::new("~").join(rest),
        _ => path.to_path_buf(),
    }
}
