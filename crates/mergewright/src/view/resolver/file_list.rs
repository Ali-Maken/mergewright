//! The resolver's file list in the widget's terms: [`listed_files`],
//! sanitised. The draw and the mouse both build it here.

use mergewright_ops::conflict::FileState;
use mergewright_term::sanitize_line;
use mergewright_ui::conflict_resolver::{FileListView, FileRow, FileRowLook, ListTail};
use mergewright_ui::keymap::ActionId;

use super::how_words;
use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::resolver::review::in_a_merge;
use crate::update::resolver::{RowState, file_cursor, listed_files, side_names};

/// The resolver's file list, sanitised. Each row reads as its file stands
/// in mergewright-ops' terms ([`FileState`], ease-pass/11): `? N open` with the
/// one clash count the review's read also uses, `✓ done` for a merge's
/// solved file as for a done one (the review's `y` stages it). Elsewhere a
/// file with every clash taken still waits for `⏎`.
///
/// In a merge with nothing open and every file read, the list ends with
/// `✓ Review the merge  y` — the notice row's finish words are not drawn
/// there (`super::finish_row`).
#[must_use]
pub fn file_list_view(state: &AppState) -> FileListView {
    let Some(open) = state.resolver.as_ref() else {
        return FileListView::default();
    };
    let names = side_names(state);
    let merge = in_a_merge(state);
    let listed = listed_files(state);
    let rows: Vec<FileRow> = listed
        .iter()
        .map(|file| FileRow {
            name: sanitize_line(&file.path.display().to_string()),
            state: match (file_state(&file.state), &file.state) {
                (None, _) => FileRowLook::Unread,
                (Some(_), RowState::WholeFile { how }) => FileRowLook::WholeFile {
                    how: sanitize_line(&how_words(*how, false, &names)),
                },
                (Some(_), RowState::Binary) => FileRowLook::Binary,
                (Some(now @ FileState::Open { .. }), RowState::Open { of, .. }) => {
                    FileRowLook::Open {
                        open: now.open_clashes(),
                        of: *of,
                    }
                }
                (Some(FileState::Solved), _) if merge => FileRowLook::Solved,
                (Some(FileState::Solved), _) => FileRowLook::AllTaken,
                // `Open` here is never reached: `file_state` gives it only
                // for an open, whole-file or binary row, each matched above.
                (Some(FileState::Done | FileState::Open { .. }), _) => FileRowLook::Done,
            },
        })
        .collect();
    let nothing_open = listed
        .iter()
        .all(|file| file_state(&file.state).is_some_and(|now| now.open_clashes() == 0));
    let tail = match state.keymap.spell_keys(&[ActionId("resolver.review")]) {
        Some(key) if merge && nothing_open && !listed.is_empty() => ListTail::Review { key },
        _ => ListTail::None,
    };
    FileListView {
        rows,
        cursor: file_cursor(state),
        unavailable: matches!(open.files, Loading::Failed(_)),
        tail,
    }
}

/// A listed row as mergewright-ops' [`FileState`] names it: an open document or
/// a whole-file card (one clash) is `Open`, every clash picked `Solved`,
/// no longer unmerged `Done`; `None` while it is unread.
fn file_state(row: &RowState) -> Option<FileState> {
    match row {
        RowState::Open { open, .. } => Some(FileState::Open { open: *open }),
        RowState::WholeFile { .. } | RowState::Binary => Some(FileState::Open { open: 0 }),
        RowState::AllTaken => Some(FileState::Solved),
        RowState::Done => Some(FileState::Done),
        RowState::Unread => None,
    }
}
