//! A submodule diff's inside facts — `changed inside`, `untracked inside` —
//! merged into the diff as it lands, from the file list's row for the same
//! path (review m7-16b..21b, Minor 3).
//!
//! The facts come from git's status, which the file-list job reads
//! (`jobs::detail`'s `SubmoduleRows`); the diff job reads only the pointer's
//! move and cannot know them without a status read of its own. The row
//! already carries them, so the diff takes them here, keyed by path, and the
//! renderer draws the diff it is given (m7-power-tools/30i had it copy them
//! from the cursor row at draw time).

use std::sync::Arc;

use mergewright_git::diff::FileList;
use mergewright_ui::detail::{DiffView, FileListView, FileRowKind, FileStatusView};

use super::state::Loading;

/// `view` with the inside facts of `path`'s submodule row, when the list
/// has one; otherwise `view` as it is.
pub(super) fn with_inside_facts(
    files: &Loading<Arc<FileListView>>,
    list: Option<&FileList>,
    path: &[u8],
    view: Arc<DiffView>,
) -> Arc<DiffView> {
    let FileStatusView::Submodule(arrived) = &view.status else {
        return view;
    };
    let (Loading::Ready(files), Some(list)) = (files, list) else {
        return view;
    };
    let row = files.rows.iter().find_map(|row| {
        let entry = list.files.get(usize::try_from(row.entry?).ok()?)?;
        match &row.kind {
            FileRowKind::File {
                status: FileStatusView::Submodule(facts),
                ..
            } if entry.path == path => Some(facts),
            _ => None,
        }
    });
    let Some(row) = row else {
        return view;
    };
    if (arrived.changed_inside, arrived.untracked_inside)
        == (row.changed_inside, row.untracked_inside)
    {
        return view;
    }
    let mut merged = view.as_ref().clone();
    if let FileStatusView::Submodule(submodule) = &mut merged.status {
        submodule.changed_inside = row.changed_inside;
        submodule.untracked_inside = row.untracked_inside;
    }
    Arc::new(merged)
}

#[cfg(test)]
mod tests {
    use mergewright_core::job::JobId;
    use mergewright_git::diff::DiffOptions;
    use mergewright_term::sanitize::sanitize_line;
    use mergewright_ui::detail::SubmoduleView;

    use super::super::cursor::tests::{files_of, raw_diff};
    use super::super::state::{DetailSpec, DetailState, DiffKey, MAX_LINES};
    use super::*;

    fn submodule(changed: bool, untracked: bool) -> FileStatusView {
        FileStatusView::Submodule(Box::new(SubmoduleView {
            changed_inside: changed,
            untracked_inside: untracked,
            ..SubmoduleView::default()
        }))
    }

    /// Review m7-16b..21b, Minor 3: the diff that lands for a submodule
    /// carries its row's inside facts; the renderer no longer copies them
    /// from whatever row the cursor is on.
    #[test]
    fn submodule_facts_arrive_with_the_diff() {
        let (mut view, list) = files_of(&["libs/other", "libs/parser"], None, false);
        for (row, entry) in view.rows.iter_mut().zip(list.files.iter()) {
            let parser = entry.path == b"libs/parser";
            if let FileRowKind::File { status, .. } = &mut row.kind {
                *status = submodule(parser, parser);
            }
        }
        let mut raw = raw_diff().as_ref().clone();
        raw.entry.path = b"libs/parser".to_vec();
        let id = JobId::new(7);
        let mut state = DetailState {
            files: Loading::Ready(Arc::new(view)),
            list: Some(Arc::new(list)),
            diff: Loading::Pending(id),
            ..DetailState::default()
        };
        let key = DiffKey {
            spec: DetailSpec::WorkingTree,
            entry: 1,
            section: None,
            options: DiffOptions::default(),
            max_lines: MAX_LINES,
        };
        let arrived = DiffView {
            path: sanitize_line("libs/parser"),
            old_path: None,
            status: submodule(false, false),
            hunks: Vec::new(),
            binary: None,
            truncated: None,
            untracked: false,
        };
        state.diff_arrived(id, key, Ok((Arc::new(arrived), Arc::new(raw))));
        let Loading::Ready(landed) = &state.diff else {
            panic!("{:?}", state.diff);
        };
        assert_eq!(landed.status, submodule(true, true));
    }
}
