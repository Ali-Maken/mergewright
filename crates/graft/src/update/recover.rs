//! A put-back from the journal, once it has run (m7-power-tools/24): the
//! line it leaves and the tags it puts on the graph.
//!
//! Pure over the operation's params; the subject of a commit is asked of
//! the caller, because a restored commit is on no branch until the put-back
//! runs, so the graph's rows may not hold it — the journal's list does.

use graft_core::oid::Oid;
use graft_ops::journal_list::ListRow;
use graft_ops::operation::{Operation, RecoverParams, RecoverTarget};
use graft_term::sanitize_line;
use graft_ui::graph_screen::RowTag;

use super::state::{AppState, Status};

/// The key the line names: the put-back is a journal entry like any other.
const UNDOES: &str = "u undoes";

/// The line a put-back's `Done` leaves (note "Where he lands" 2b):
/// `put back ✓ · main has "<subject>" again · u undoes`,
/// `spike is back at "<subject>" ✓ · u undoes`,
/// `your edit to src/a.rs is back ✓ · u undoes`.
///
/// The tick is the renderer's (`Status::Done`), so it follows `what`. A
/// subject nobody can name is the commit's sha7, never a blank.
pub(crate) fn done_status(
    params: &RecoverParams,
    subject_of: impl Fn(Oid) -> Option<String>,
) -> Status {
    let named = |oid: Oid| match subject_of(oid) {
        Some(subject) => format!("\"{}\"", sanitize_line(&subject)),
        None => super::place::short(oid),
    };
    let (what, also) = match &params.target {
        RecoverTarget::MoveRef { name, to, .. } => (
            "put back".to_owned(),
            Some(format!("{} has {} again", sanitize_line(name), named(*to))),
        ),
        RecoverTarget::Recreate { name, at } => (
            format!("{} is back at {}", sanitize_line(name), named(*at)),
            None,
        ),
        RecoverTarget::Files { paths, .. } => (files_back(paths), None),
    };
    Status::Done {
        what,
        also,
        keys: Some(UNDOES.to_owned()),
    }
}

/// `your edit to src/a.rs is back`, `your edits to src/a.rs +2 more are
/// back`.
fn files_back(paths: &[std::path::PathBuf]) -> String {
    // `lost_edits` never makes an edit with no files; the words still hold.
    let Some(first) = paths.first() else {
        return "your edits are back".to_owned();
    };
    let first = sanitize_line(&first.to_string_lossy());
    match paths.len() {
        1 => format!("your edit to {first} is back"),
        count => format!("your edits to {first} +{} more are back", count - 1),
    }
}

/// The tags a put-back leaves on the graph: `↺ put back` on what it
/// restored, `○ <branch> was here` on the tip a moved branch left. A file
/// put-back tags nothing: it lands on the changes pane.
pub(crate) fn put_back_tags(params: &RecoverParams) -> Vec<(Oid, RowTag)> {
    match &params.target {
        RecoverTarget::MoveRef { name, to, expected } => vec![
            (*to, RowTag::PutBack),
            (
                *expected,
                RowTag::WasHere {
                    branch: sanitize_line(name).to_string(),
                },
            ),
        ],
        RecoverTarget::Recreate { at, .. } => vec![(*at, RowTag::PutBack)],
        RecoverTarget::Files { .. } => Vec::new(),
    }
}

/// A finished write that was a put-back tags its rows. Called on `Done`,
/// after `operate::claim_write` cleared the last write's tags; the next
/// write clears these the same way.
pub(super) fn tag_rows(state: &mut AppState, launched: Option<&Operation>) {
    let Some(Operation::Recover(params)) = launched else {
        return;
    };
    for (at, tag) in put_back_tags(params) {
        state.row_tags.set(at, tag);
    }
}

/// A commit's subject as the screen knows it: a resident graph row's, else
/// the journal list's lost commit's. `None` when neither holds it.
pub(super) fn subject_in(state: &AppState, oid: Oid) -> Option<String> {
    let resident = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .and_then(|window| window.rows.iter().find(|row| row.commit.id == oid))
        .map(|row| String::from_utf8_lossy(row.commit.subject()).into_owned());
    resident.or_else(|| {
        state
            .journal
            .list
            .as_ref()?
            .rows
            .iter()
            .find_map(|row| match row {
                ListRow::Lost { commit, .. } if commit.run.tip == oid => {
                    Some(commit.subject.clone())
                }
                _ => None,
            })
    })
}

#[cfg(test)]
mod tests;
