//! `Selecting`: the pane's cursor onto the landing's file row
//! (m7-power-tools/20b, split out in 20o).

use graft_git::diff::FileList;
use graft_term::sanitize::sanitize_path_bytes;
use graft_ui::detail::{FileListView, FileRowKind, WorkingTreeSection};

use crate::update::events::Command;
use crate::update::state::{AppState, Status};

use super::super::state::{DetailSpec, Loading};
use super::{LandingStage, LineLanding, path_bytes, short};

/// `Selecting`: once the list is in, the cursor on `path`'s row — unfolding
/// any folded directory it is inside (20m B) — and, on the changes pane, on
/// the row [`LineLanding::section`] names. A path the spec does not touch
/// leaves the pane as it opened and says so.
pub(super) fn select(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    landing: &LineLanding,
) -> Option<LandingStage> {
    let detail = &state.detail;
    if !detail.shown || detail.spec != Some(landing.spec) {
        return None;
    }
    let files = match &detail.files {
        Loading::Pending(_) => return Some(LandingStage::Selecting),
        Loading::Ready(files) => files,
        Loading::Idle | Loading::Failed(_) => return None,
    };
    let list = detail.list.as_ref()?;
    let wanted = path_bytes(&landing.path);
    // Every row, folded or not, with the changes-pane section it is in.
    let rows: Vec<(usize, Option<WorkingTreeSection>)> = (0..files.rows.len())
        .filter(|&row| row_is(files, list, row, &wanted))
        .map(|row| (row, files.section_at(row)))
        .collect();
    let in_section = |wanted: WorkingTreeSection| {
        rows.iter()
            .find_map(|&(row, section)| (section == Some(wanted)).then_some(row))
    };
    let row = rows.first().map(|&(first, _)| match landing.section {
        Some(section) => in_section(section).ok_or(first),
        // On disk: a partly staged path is read on its Unstaged row first
        // (`index_line` moves it on to the Staged row when the line is not
        // an unstaged one). An Unmerged row, or a path with one row, is
        // its first.
        None => match (
            in_section(WorkingTreeSection::Unmerged),
            in_section(WorkingTreeSection::Staged),
            in_section(WorkingTreeSection::Unstaged),
        ) {
            (None, Some(_), Some(unstaged)) => Ok(unstaged),
            _ => Ok(first),
        },
    });
    let row = match row {
        Some(Ok(row)) => row,
        Some(Err(first)) => {
            // No row in the section asked for: the tree changed from
            // outside since. The path's first row, at its first change.
            if let Some(pending) = state.line_landing.as_mut() {
                pending.at = None;
            }
            first
        }
        None => {
            say_missing(state, landing.spec, &wanted);
            return None;
        }
    };
    let AppState {
        detail, local_jobs, ..
    } = state;
    if let Loading::Ready(files) = &mut detail.files {
        unfold_to(std::sync::Arc::make_mut(files), row);
    }
    commands.extend(detail.select_file_row(row, &mut || local_jobs.next_id()));
    Some(LandingStage::Placing)
}

/// Row `row` of `files` is the file `wanted`.
pub(super) fn row_is(files: &FileListView, list: &FileList, row: usize, wanted: &[u8]) -> bool {
    files
        .rows
        .get(row)
        .and_then(|found| found.entry)
        .and_then(|entry| usize::try_from(entry).ok())
        .and_then(|entry| list.files.get(entry))
        .is_some_and(|file| file.path == wanted)
}

/// Unfold every folded directory `row` sits inside, with the pane's own
/// unfold (`FileListView::toggle_dir`), so `row` is visible.
fn unfold_to(files: &mut FileListView, row: usize) {
    let Some(mut depth) = files.rows.get(row).map(|found| found.depth) else {
        return;
    };
    for above in (0..row).rev() {
        let Some(found) = files.rows.get(above) else {
            continue;
        };
        if found.depth >= depth {
            continue;
        }
        depth = found.depth;
        if matches!(found.kind, FileRowKind::Dir { collapsed: true }) {
            files.toggle_dir(above);
        }
    }
}

/// Say `wanted` is not among `spec`'s files; the pane stays as it opened.
fn say_missing(state: &mut AppState, spec: DetailSpec, wanted: &[u8]) {
    let words = match spec {
        DetailSpec::Commit(commit) => format!("{}'s files", short(commit)),
        DetailSpec::Compare { base, head } => {
            format!("the files of {}..{}", short(base), short(head))
        }
        DetailSpec::WorkingTree => "the uncommitted changes".to_owned(),
    };
    // A path is the repository's bytes: sanitised before it is words.
    let path = sanitize_path_bytes(wanted);
    state.status = Status::sticky(format!("{} isn't among {words}", path.as_str()));
}
