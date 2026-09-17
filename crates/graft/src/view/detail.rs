//! Assembling the detail pane's view model from `AppState` (ARCHITECTURE.md).
//!
//! Pure, like the rest of `view`: no I/O, just reshaping what `update` already
//! keeps into what `graft_ui::detail_pane` draws. `header.radar` comes from
//! `view::radar::radar_lines_for` (`m2-details-and-diffs/13c`) for a selected
//! commit; the other subjects (`Compare`, `WorkingTree`, none selected) carry
//! no radar line. `explain` stays `None` — no ticket has put an explain panel
//! in `AppState` yet.
//!
//! `Commit` carries an author and a committer identity and its parents in
//! full, so the header's author/committer/parents lines are never a gap.
//! Co-authors are not a separate field: git records them as `Co-authored-by:`
//! trailers inside the message body, so they are picked out of the same
//! bytes the graph already hydrates — no extra git call.

use std::sync::Arc;

use graft_core::oid::Oid;
use graft_term::sanitize_line;
use graft_ui::detail::{
    BodyView, ContainedIn, DetailHeader, DetailView, DiffOptionsView, FileRowKind, Subject,
};
use graft_ui::graph_screen::{SHA_WIDTH, relative_date, short_sha};

use crate::history::History;
use crate::update::AppState;
use crate::update::detail::{DetailSpec, DetailState, Loading};

/// The detail pane's view model for this frame.
#[must_use]
pub fn detail_view(state: &AppState) -> DetailView {
    // The pane's own `spec` (`update::detail::DetailState::open`/`follow`) is
    // what is actually loaded and shown; it can differ from the cursor's own
    // spec while a compare base is pinned (`AppState::compare_base`,
    // m2-details-and-diffs/11) — `selected_spec` never names `Compare`, so a
    // pinned compare's header would otherwise redraw as a plain commit every
    // frame. Nothing is open yet (`spec: None`) only before the pane's first
    // load, when `selected_spec` is exactly what will be asked for.
    let header = match state.detail.spec.or_else(|| state.selected_spec()) {
        Some(DetailSpec::WorkingTree) => working_tree_header(state),
        Some(DetailSpec::Commit(oid)) => commit_header(state, oid),
        Some(DetailSpec::Compare { base, head }) => compare_header(base, head),
        None => empty_header(),
    };
    let files = match &state.detail.files {
        Loading::Ready(files) => Some(Arc::clone(files)),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => None,
    };
    DetailView {
        header,
        files,
        body: body_view(&state.detail),
        body_top: state.detail.body_top,
        focus: state.detail.focus,
        mode: state.detail.mode,
        options: DiffOptionsView {
            context: state.detail.options.context,
            ignore_whitespace: state.detail.options.ignore_whitespace,
            word_tint: state.detail.word_tint,
        },
        explain: None,
    }
}

/// The header before anything is selected: every field empty, subject
/// [`Subject::None`].
fn empty_header() -> DetailHeader {
    DetailHeader {
        subject: Subject::None,
        summary: sanitize_line(""),
        author: sanitize_line(""),
        author_date: sanitize_line(""),
        relative: sanitize_line(""),
        committer: None,
        co_authors: Vec::new(),
        parents: Vec::new(),
        against: None,
        contained: ContainedIn::NotAsked,
        radar: Vec::new(),
    }
}

/// The header for one commit: subject, author, committer (when it differs),
/// co-authors (parsed from the message trailers), parents and "contained in"
/// state, all straight from the row the graph already hydrated.
fn commit_header(state: &AppState, oid: Oid) -> DetailHeader {
    let Some(row) = state
        .history
        .as_ref()
        .and_then(History::selected)
        .filter(|row| row.commit.id == oid)
    else {
        return empty_header();
    };
    let commit = &row.commit;
    let sha = short_sha(&commit.id.to_string(), usize::from(SHA_WIDTH));
    let author_name = String::from_utf8_lossy(&commit.author.name).into_owned();
    let author_email = String::from_utf8_lossy(&commit.author.email).into_owned();
    let committer_name = String::from_utf8_lossy(&commit.committer.name).into_owned();
    let committer_email = String::from_utf8_lossy(&commit.committer.email).into_owned();
    let committer = (author_name != committer_name || author_email != committer_email)
        .then(|| sanitize_line(&format!("{committer_name} <{committer_email}>")));
    let parents = commit
        .parents
        .iter()
        .map(|parent| sanitize_line(&short_sha(&parent.to_string(), usize::from(SHA_WIDTH))))
        .collect();
    let contained = state
        .detail
        .contained
        .get(&oid)
        .cloned()
        .unwrap_or_default();
    let subject_text = String::from_utf8_lossy(commit.subject());
    let radar = super::radar::radar_lines_for(state, oid, &commit.parents, &subject_text);
    DetailHeader {
        subject: Subject::Commit {
            sha: sanitize_line(&sha),
        },
        summary: sanitize_line(&String::from_utf8_lossy(commit.subject())),
        author: sanitize_line(&format!("{author_name} <{author_email}>")),
        author_date: sanitize_line(&graft_ui::graph_screen::absolute_date(
            commit.author.when.seconds,
            commit.author.when.offset_seconds,
        )),
        relative: sanitize_line(&relative_date(
            commit.author.when.seconds,
            state.harness.clock.now_seconds(),
        )),
        committer,
        co_authors: co_authors_from(&commit.message),
        parents,
        against: None,
        contained,
        radar,
    }
}

/// `Co-authored-by:` trailers, in the order the message carries them.
fn co_authors_from(message: &[u8]) -> Vec<graft_term::sanitize::SanitizedText> {
    String::from_utf8_lossy(message)
        .lines()
        .filter_map(|line| {
            line.strip_prefix("Co-authored-by:")
                .or_else(|| line.strip_prefix("co-authored-by:"))
        })
        .map(|value| sanitize_line(value.trim()))
        .collect()
}

/// The header for a `base x head` comparison: header-only per this ticket's
/// scope (`compare-mode` fills in the rest).
fn compare_header(base: Oid, head: Oid) -> DetailHeader {
    let base_sha = short_sha(&base.to_string(), usize::from(SHA_WIDTH));
    let head_sha = short_sha(&head.to_string(), usize::from(SHA_WIDTH));
    DetailHeader {
        subject: Subject::Compare {
            base: sanitize_line(&base_sha),
            head: sanitize_line(&head_sha),
        },
        ..empty_header()
    }
}

/// The header for the working tree against `HEAD`: counts from
/// [`AppState::working_tree`], no author or parents — there is no commit yet.
fn working_tree_header(state: &AppState) -> DetailHeader {
    let counts = state.working_tree.as_ref();
    let head = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .filter(|window| window.first == 0)
        .and_then(|window| window.rows.first())
        .map(|row| {
            format!(
                "HEAD {}",
                short_sha(&row.commit.id.to_string(), usize::from(SHA_WIDTH))
            )
        });
    DetailHeader {
        subject: Subject::WorkingTree {
            staged: counts.map_or(0, |c| u32::try_from(c.staged).unwrap_or(u32::MAX)),
            unstaged: counts.map_or(0, |c| u32::try_from(c.unstaged).unwrap_or(u32::MAX)),
            untracked: counts.map_or(0, |c| u32::try_from(c.untracked).unwrap_or(u32::MAX)),
        },
        summary: sanitize_line("Uncommitted changes"),
        author: sanitize_line(""),
        author_date: sanitize_line(""),
        relative: sanitize_line(""),
        committer: None,
        co_authors: Vec::new(),
        parents: Vec::new(),
        against: head.map(|head| sanitize_line(&head)),
        contained: ContainedIn::NotAsked,
        radar: Vec::new(),
    }
}

/// The body: the commit message when the files cursor sits on the `✉
/// message` row, else the diff under it — mirroring [`DetailState::files`]
/// and [`DetailState::diff`]'s own loading states.
fn body_view(detail: &DetailState) -> BodyView {
    match &detail.files {
        Loading::Idle => BodyView::Empty(sanitize_line("select a commit")),
        Loading::Pending(_) => BodyView::Loading(sanitize_line("reading…")),
        Loading::Failed(message) => {
            BodyView::Failed(sanitize_line(&format!("git diff-tree: {message}")))
        }
        Loading::Ready(files) => {
            let on_message = files
                .rows
                .get(files.cursor)
                .is_some_and(|row| matches!(row.kind, FileRowKind::Message { .. }));
            if on_message {
                return BodyView::Message {
                    lines: files.message.clone(),
                };
            }
            match &detail.diff {
                Loading::Idle => BodyView::Empty(sanitize_line("select a file")),
                Loading::Pending(_) => BodyView::Loading(sanitize_line("reading…")),
                Loading::Failed(message) => {
                    BodyView::Failed(sanitize_line(&format!("git diff: {message}")))
                }
                Loading::Ready(diff) => BodyView::Diff(Arc::clone(diff)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::Size;
    use crate::view::tests::{counts, history_of, oid};

    fn state_on_working_tree(counts: graft_git::status::WorkingTreeCounts) -> AppState {
        AppState {
            history: Some(history_of(3, 0)),
            working_tree: Some(counts),
            on_working_tree: true,
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        }
    }

    #[test]
    fn working_tree_header_reads_counts_and_head() {
        let state = state_on_working_tree(counts(2, 1, 3));
        let view = detail_view(&state);
        assert_eq!(
            view.header.subject,
            Subject::WorkingTree {
                staged: 2,
                unstaged: 1,
                untracked: 3,
            }
        );
        assert_eq!(view.header.summary.as_str(), "Uncommitted changes");
        assert!(view.header.author.as_str().is_empty());
        assert!(view.header.parents.is_empty());
        assert_eq!(view.header.contained, ContainedIn::NotAsked);
        let against = view.header.against.expect("HEAD is the topmost row");
        assert!(against.as_str().starts_with("HEAD "), "{against:?}");
    }

    #[test]
    fn commit_header_reads_the_selected_rows_facts() {
        let state = AppState {
            history: Some(history_of(3, 0)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };
        let view = detail_view(&state);
        assert_eq!(
            view.header.subject,
            Subject::Commit {
                sha: sanitize_line(&oid(3).to_string()[..7])
            }
        );
        assert_eq!(view.header.author.as_str(), "Test <test@example.invalid>");
        // Author and committer are identical in this fixture: no line shown.
        assert_eq!(view.header.committer, None);
        assert_eq!(view.header.parents.len(), 1);
    }
}
