//! Assembling the detail pane's view model from `AppState` (ARCHITECTURE.md).
//!
//! Pure, like the rest of `view`: no I/O, just reshaping what `update` already
//! keeps into what `mergewright_ui::detail_pane` draws. `header.radar` comes from
//! `view::radar::radar_lines_for` (`m2-details-and-diffs/13c`) for a selected
//! commit; the other subjects (`Compare`, `WorkingTree`, none selected) carry
//! no radar line. `explain` comes straight from `AppState::detail::explain`
//! (`m2-details-and-diffs/17`): `update::detail::open_explain` sets it.
//!
//! `Commit` carries an author and a committer identity and its parents in
//! full, so the header's author/committer/parents lines are never a gap.
//! Co-authors are not a separate field: git records them as `Co-authored-by:`
//! trailers inside the message body, so they are picked out of the same
//! bytes the graph already hydrates — no extra git call.

use std::sync::Arc;

use mergewright_core::oid::Oid;
use mergewright_term::sanitize_line;
use mergewright_ui::detail::{
    BodyView, CompareHeader, CompareState, CompareSummary, ContainedIn, DetailHeader, DetailView,
    DiffOptionsView, FileRowKind, FileTally, Subject,
};
use mergewright_ui::graph_screen::{SHA_WIDTH, relative_date, short_sha};

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
        Some(DetailSpec::WorkingTree) => {
            working_tree_header(state, state.refs.as_ref().and_then(|refs| refs.head()))
        }
        Some(DetailSpec::Commit(oid)) => commit_header(state, oid),
        Some(DetailSpec::Compare { base, head }) => compare_header(state, base, head),
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
        body_cursor: state.detail.body_cursor(),
        focus: state.detail.focus,
        mode: state.detail.mode,
        options: DiffOptionsView {
            context: state.detail.options.context,
            ignore_whitespace: state.detail.options.ignore_whitespace,
            word_tint: state.detail.word_tint,
        },
        explain: state.detail.explain.clone(),
        following: following(state),
        // The frame sets it from the tier in force.
        ascii: false,
    }
}

/// The pane is behind the cursor and will catch up when it rests
/// (G-settle): a follow is owed, and the pane is one that follows
/// (`update::pane_follows`, the same predicate that holds the follow).
fn following(state: &AppState) -> bool {
    crate::update::pane_follows(state) && crate::update::follow_behind(state)
}

/// The row for `oid`: the cursor's, or — while the pane is behind a moving
/// cursor (G-settle) — wherever it is in the window on hand. `None` once it
/// has left the window.
fn row_of(state: &AppState, oid: Oid) -> Option<&mergewright_graph::Row> {
    let history = state.history.as_ref()?;
    if let Some(row) = history.selected().filter(|row| row.commit.id == oid) {
        return Some(row);
    }
    history
        .window
        .as_ref()?
        .rows
        .iter()
        .find(|row| row.commit.id == oid)
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
    let Some(row) = row_of(state, oid) else {
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
    let parents = crate::history::parents::of(row, state.refs.as_deref())
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
    let radar = super::radar::radar_lines_for(
        state,
        oid,
        crate::history::parents::of(row, state.refs.as_deref()),
        &subject_text,
    );
    DetailHeader {
        subject: Subject::Commit {
            sha: sanitize_line(&sha),
        },
        summary: sanitize_line(&String::from_utf8_lossy(commit.subject())),
        author: sanitize_line(&format!("{author_name} <{author_email}>")),
        author_date: sanitize_line(&mergewright_ui::graph_screen::absolute_date(
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
fn co_authors_from(message: &[u8]) -> Vec<mergewright_term::sanitize::SanitizedText> {
    String::from_utf8_lossy(message)
        .lines()
        .filter_map(|line| {
            line.strip_prefix("Co-authored-by:")
                .or_else(|| line.strip_prefix("co-authored-by:"))
        })
        .map(|value| sanitize_line(value.trim()))
        .collect()
}

/// The header for a `base x head` comparison (m7-14): both sides by name
/// and what the read found, worded by `mergewright_ui::detail_pane`.
fn compare_header(state: &AppState, base: Oid, head: Oid) -> DetailHeader {
    DetailHeader {
        subject: Subject::Compare(CompareHeader {
            base: side_name(state, base),
            head: side_name(state, head),
            state: compare_state(&state.detail, base, head),
        }),
        ..empty_header()
    }
}

/// A compare side's name: the row's first local branch, else its first
/// remote, else its short sha.
fn side_name(state: &AppState, oid: Oid) -> mergewright_term::sanitize::SanitizedText {
    use crate::refs::RefLabel;
    let name = state.refs.as_deref().and_then(|refs| {
        let labels = refs.on(oid);
        labels
            .iter()
            .find_map(|label| match label {
                RefLabel::Branch { name, .. } => Some(name),
                _ => None,
            })
            .or_else(|| {
                labels.iter().find_map(|label| match label {
                    RefLabel::Remote(name) => Some(name),
                    _ => None,
                })
            })
            .map(|name| String::from_utf8_lossy(name).into_owned())
    });
    sanitize_line(&name.unwrap_or_else(|| short_sha(&oid.to_string(), usize::from(SHA_WIDTH))))
}

/// Where a compare's read stands. The span and the file list land in one
/// job, so a landed list always carries its summary.
fn compare_state(detail: &DetailState, base: Oid, head: Oid) -> CompareState {
    if base == head {
        return CompareState::IsBase;
    }
    match &detail.files {
        Loading::Failed(why) => CompareState::Failed(sanitize_line(why)),
        Loading::Ready(files) => match files.compare {
            Some(CompareSummary::Since { commits: 0, .. }) => CompareState::AlreadyIn,
            Some(CompareSummary::Since { commits, .. }) => CompareState::Since {
                commits,
                tally: Some(FileTally {
                    files: files.files as usize,
                    added: usize::try_from(files.added).unwrap_or(usize::MAX),
                    removed: usize::try_from(files.removed).unwrap_or(usize::MAX),
                }),
            },
            Some(CompareSummary::NoSharedHistory) => CompareState::NoSharedHistory,
            None => CompareState::Reading,
        },
        Loading::Idle | Loading::Pending(_) => CompareState::Reading,
    }
}

/// The header for the working tree against `HEAD`: counts from
/// [`AppState::working_tree`], no author or parents — there is no commit yet.
///
/// `head` is HEAD's own commit, read from the refs by the caller — never the
/// graph's top row, which is whatever sorts first: another branch's tip, or
/// a stash commit (review m6-usability-2 m-b). `None` (refs unread, or an
/// unborn branch) says `diff vs HEAD` with no id.
fn working_tree_header(state: &AppState, head: Option<Oid>) -> DetailHeader {
    let counts = state.working_tree.as_ref();
    let head = head.map_or_else(
        || "HEAD".to_owned(),
        |head| {
            format!(
                "HEAD {}",
                short_sha(&head.to_string(), usize::from(SHA_WIDTH))
            )
        },
    );
    DetailHeader {
        subject: Subject::WorkingTree {
            staged: counts.map_or(0, |c| u32::try_from(c.staged().count()).unwrap_or(u32::MAX)),
            unstaged: counts.map_or(0, |c| {
                u32::try_from(c.unstaged().count()).unwrap_or(u32::MAX)
            }),
            untracked: counts.map_or(0, |c| {
                u32::try_from(c.untracked().count()).unwrap_or(u32::MAX)
            }),
        },
        summary: sanitize_line("Uncommitted changes"),
        author: sanitize_line(""),
        author_date: sanitize_line(""),
        relative: sanitize_line(""),
        committer: None,
        co_authors: Vec::new(),
        parents: Vec::new(),
        against: Some(sanitize_line(&head)),
        contained: ContainedIn::NotAsked,
        radar: Vec::new(),
    }
}

/// The body: the commit message when the files cursor sits on the `✉
/// message` row, else the diff under it — mirroring [`DetailState::files`]
/// and [`DetailState::diff`]'s own loading states.
fn body_view(detail: &DetailState) -> BodyView {
    match &detail.files {
        // Its read was cancelled mid-burst (G-settle): it comes again at rest.
        Loading::Idle if detail.abandoned => BodyView::Loading(sanitize_line("reading…")),
        Loading::Idle => BodyView::Empty(sanitize_line("select a commit")),
        Loading::Pending(_) => BodyView::Loading(sanitize_line("reading…")),
        Loading::Failed(message) => {
            BodyView::Failed(sanitize_line(&format!("git diff-tree: {message}")))
        }
        Loading::Ready(files) if files.rows.is_empty() => {
            // The changes pane on a clean tree: not an error, and not
            // "select a file" either — there is nothing to select
            // (design note, "Changes pane").
            let empty = match detail.spec {
                Some(DetailSpec::WorkingTree) => "nothing to commit — working tree clean",
                _ => "no changes",
            };
            BodyView::Empty(sanitize_line(empty))
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
                Loading::Idle if detail.abandoned => BodyView::Loading(sanitize_line("reading…")),
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
mod tests;
