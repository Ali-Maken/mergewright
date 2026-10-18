//! The resolver's file list in the widget's terms: [`listed_files`],
//! sanitised. The draw and the mouse both build it here.

use graft_term::sanitize_line;
use graft_ui::conflict_resolver::{FileListView, FileRow, FileRowLook};

use super::how_words;
use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::resolver::review::in_a_merge;
use crate::update::resolver::{RowState, file_cursor, listed_files, side_names};

/// The resolver's file list, sanitised. In a merge a file with every clash
/// picked reads `solved`: `⏎` marks nothing there, the review's `y` stages
/// it (ease-pass/05b, 05c). Elsewhere it still waits for `⏎`.
#[must_use]
pub fn file_list_view(state: &AppState) -> FileListView {
    let Some(open) = state.resolver.as_ref() else {
        return FileListView::default();
    };
    let names = side_names(state);
    let merge = in_a_merge(state);
    FileListView {
        rows: listed_files(state)
            .into_iter()
            .map(|file| FileRow {
                name: sanitize_line(&file.path.display().to_string()),
                state: match file.state {
                    RowState::Open { open, of } => FileRowLook::Open { open, of },
                    RowState::AllTaken if merge => FileRowLook::Solved,
                    RowState::AllTaken => FileRowLook::AllTaken,
                    RowState::Done => FileRowLook::Done,
                    RowState::WholeFile { how } => FileRowLook::WholeFile {
                        how: sanitize_line(&how_words(how, false, &names)),
                    },
                    RowState::Binary => FileRowLook::Binary,
                    RowState::Unread => FileRowLook::Unread,
                },
            })
            .collect(),
        cursor: file_cursor(state),
        unavailable: matches!(open.files, Loading::Failed(_)),
    }
}
