//! `update` tests: sweep states. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// The row [`merge_on_screen`] puts a **merge** on, which is the row a
/// graph-row context's keys are swept against: `space fold` has a merge to
/// fold there, the way `H hide stash` has a stash to hide and `y sha` has a
/// commit to copy. A key that does nothing under one row's *data* — `space`
/// off a merge, `k` at the top — is a boundary, not a dead key, and this
/// sweep is not the place that would tell the difference.
pub(super) const MERGE_ROW: usize = 1;

/// One stash in the refs, so `H hide stash` has something to hide: with
/// none, `ViewChange::Stashes` returns `None` by design (`update::keys`).
pub(super) fn one_stash(state: &mut AppState) {
    let stash = Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let name = mergewright_core::ref_name::RefName::new("refs/stash").expect("a valid ref name");
    let badges = crate::refs::badges(
        &[(name, stash)],
        &mergewright_git::repo::HeadState::Detached(stash),
        &std::collections::HashMap::new(),
        &[(stash, 0)],
        |_, _| Err("no upstreams on this fixture".to_owned()),
    )
    .expect("badges");
    state.refs = Some(Arc::new(badges));
}

/// The selected commit made a stash's (`refs/stash` on it), with `HEAD`
/// detached elsewhere: the screen of [`HintContext::StashRow`] (m8/15 A4).
pub(super) fn stash_under_cursor(state: &mut AppState) {
    let selected = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .expect("a row under the cursor")
        .commit
        .id;
    let elsewhere = Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let name = mergewright_core::ref_name::RefName::new("refs/stash").expect("a valid ref name");
    let badges = crate::refs::badges(
        &[(name, selected)],
        &mergewright_git::repo::HeadState::Detached(elsewhere),
        &std::collections::HashMap::new(),
        &[(selected, 0)],
        |_, _| Err("no upstreams on this fixture".to_owned()),
    )
    .expect("badges");
    state.refs = Some(Arc::new(badges));
}

/// Whether `context` is, or wraps, a standing search's bar (m8/15b).
pub(super) fn holds_search(context: &HintContext) -> bool {
    match context {
        HintContext::Searching(_) => true,
        HintContext::BackToBlame(row)
        | HintContext::BackToHistory(row)
        | HintContext::OperationLive { row, .. }
        | HintContext::PaneFocused { row, .. } => holds_search(row),
        _ => false,
    }
}

/// A finished search over `state` with two hits — the first two rows on
/// screen — its prompt closed: the screen of [`HintContext::Searching`]
/// (m8/15b). Two, so `n` and `N` each move the selection.
pub(super) fn search_standing(state: &mut AppState) {
    let ids: Vec<Oid> = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .expect("rows on screen")
        .rows
        .iter()
        .take(2)
        .map(|row| row.commit.id)
        .collect();
    assert_eq!(ids.len(), 2, "two rows to find");
    let mut search = Search::new(Query::parse("Commit").expect("a query"), 0, (0, 0));
    search.batches = vec![Arc::from(ids)];
    search.state = SearchState::Done { hits: 2 };
    state.search = Some(search);
    state.search_prompt = None;
}

/// Whether `context` is, or wraps, a stash's row.
pub(super) fn holds_stash_row(context: &HintContext) -> bool {
    match context {
        HintContext::StashRow => true,
        HintContext::Marked(row)
        | HintContext::Searching(row)
        | HintContext::Rewritable(row)
        | HintContext::BackToBlame(row)
        | HintContext::BackToHistory(row)
        | HintContext::OperationLive { row, .. }
        | HintContext::PaneFocused { row, .. } => holds_stash_row(row),
        _ => false,
    }
}

/// The pane open on `spec`, with a two-file list in it so the keys that act
/// on a file have one under the cursor.
pub(super) fn show_pane(state: &mut AppState, spec: DetailSpec, focus: DetailFocus) {
    state.detail.shown = true;
    state.detail.spec = Some(spec);
    state.detail.focus = focus;
    let (files, list) = two_files();
    state.detail.files = Loading::Ready(files);
    state.detail.list = Some(list);
}

/// `DetailSpec` for the commit at row 0, for a state whose cursor is on the
/// working-tree row and so has no commit selected.
pub(super) fn head_spec(state: &AppState) -> Option<DetailSpec> {
    let row = state
        .history
        .as_ref()
        .and_then(History::selected)?
        .commit
        .id;
    Some(DetailSpec::Commit(row))
}

/// An operation stopped with its banner focused: a rebase when `s skip`
/// applies, a merge when it does not, with a conflicted file when the
/// banner's `M`/`e` are drawn.
pub(super) fn stopped_operation(walk: JobId, skippable: bool, conflicted: bool) -> AppState {
    let (mut state, _) = merge_on_screen(walk, 0);
    state.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind: if skippable {
            mergewright_git::state::InProgress::Rebase
        } else {
            mergewright_git::state::InProgress::Merge
        },
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    state.banner_focus = true;
    if conflicted {
        state.working_tree = Some(Arc::new(mergewright_git::status::WorkingTreeStatus {
            entries: vec![status_entry("conflicted.rs", true)],
        }));
    }
    state
}

/// The changes pane open and focused on its file list, over a working tree
/// whose files are the ones the list draws — `unmerged` makes the file
/// under the cursor a conflicted one, which is what swaps the pane's write
/// keys for the conflict keys.
pub(super) fn changes_pane_open(walk: JobId, unmerged: bool) -> AppState {
    let (mut state, _) = merge_on_screen(walk, 0);
    state.on_working_tree = true;
    state.working_tree = Some(Arc::new(mergewright_git::status::WorkingTreeStatus {
        entries: vec![
            status_entry("a.rs", unmerged),
            status_entry("b.rs", unmerged),
        ],
    }));
    show_pane(&mut state, DetailSpec::WorkingTree, DetailFocus::Files);
    let (view, raw) = changes_pane_files();
    state.detail.files = Loading::Ready(view);
    state.detail.list = Some(raw);
    state
}

/// The changes pane's file list: an `UNSTAGED` heading and the two files
/// under it, the cursor on the first. `space stage` reads the **section**
/// the cursor's row belongs to (`operate::stage_selected`), so a flat list
/// with no heading is a pane whose write keys cannot fire.
fn changes_pane_files() -> (DetailArc<FileListView>, DetailArc<FileList>) {
    let (view, list) = two_files();
    let mut view = (*view).clone();
    view.rows.insert(
        0,
        FileRow {
            depth: 0,
            name: sanitize_line("UNSTAGED"),
            kind: FileRowKind::Section {
                section: mergewright_ui::detail::WorkingTreeSection::Unstaged,
                count: 2,
            },
            entry: None,
        },
    );
    for row in view.rows.iter_mut().skip(1) {
        row.depth = 1;
    }
    view.cursor = 1;
    (DetailArc::new(view), list)
}

/// A commit's file list with a directory row under the cursor, which is
/// what `space` folds while the files region has focus
/// (`detail::keys::files_key`).
pub(super) fn pane_files() -> (DetailArc<FileListView>, DetailArc<FileList>) {
    let (view, list) = two_files();
    let mut view = (*view).clone();
    view.rows.insert(
        0,
        FileRow {
            depth: 0,
            name: sanitize_line("src"),
            kind: FileRowKind::Dir { collapsed: false },
            entry: None,
        },
    );
    for row in view.rows.iter_mut().skip(1) {
        row.depth = 1;
    }
    view.cursor = 0;
    (DetailArc::new(view), list)
}

/// [`commit_pane`]'s file list led by a `✉ message` row, the cursor on it:
/// a files-list row with no file to blame and no directory to fold
/// (m7-power-tools/20h).
pub(super) fn on_the_message(state: &mut AppState) {
    if let Loading::Ready(files) = &mut state.detail.files {
        let mut view = (**files).clone();
        view.rows.insert(
            0,
            FileRow {
                depth: 0,
                name: sanitize_line("message"),
                kind: FileRowKind::Message { lines: 1 },
                entry: None,
            },
        );
        view.message = vec![sanitize_line("the body")];
        view.cursor = 0;
        *files = DetailArc::new(view);
    }
}

/// A diff long enough to scroll: `j`/`k` in the diff region clamp to the
/// content they have, so a pane with nothing loaded answers them with
/// nothing, which is a fixture that cannot see what it is looking for.
pub(super) fn long_diff(
    path: &str,
) -> (
    DetailArc<DiffView>,
    DetailArc<mergewright_git::diff::FileDiff>,
) {
    use mergewright_ui::detail::{HunkView, LineKindView, LineView};

    let (view, raw) = diff_of(path);
    let mut view = (*view).clone();
    let line = |number: u32| LineView {
        kind: LineKindView::Context,
        old_no: Some(number),
        new_no: Some(number),
        text: sanitize_line("a line of context"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    };
    let mut lines = vec![LineView {
        kind: LineKindView::HunkHeading,
        old_no: None,
        new_no: None,
        text: sanitize_line("@@ -1,200 +1,200 @@"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }];
    lines.extend((1..=200).map(line));
    view.hunks = vec![HunkView {
        heading: sanitize_line("-1,200 +1,200"),
        lines,
    }];
    (DetailArc::new(view), raw)
}
