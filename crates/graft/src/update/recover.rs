//! A put-back from the journal, once it has run (m7-power-tools/24): the
//! line it leaves and the tags it puts on the graph.
//!
//! Pure over the operation's params; the subject of a commit is asked of
//! the caller, because a restored commit is on no branch until the put-back
//! runs, so the graph's rows may not hold it — the journal's list does.

use graft_core::oid::Oid;
use graft_ops::journal_list::ListRow;
use graft_ops::operation::{FilesEffect, Operation, RecoverParams, RecoverTarget};
use graft_term::sanitize_line;
use graft_ui::graph_screen::RowTag;

use super::state::{AppState, Status};

/// The line a put-back's `Done` leaves (note "Where he lands" 2b):
/// `✓ put back · main has "<subject>" again  u undoes`,
/// `✓ spike is back at "<subject>"  u undoes`,
/// `✓ your edit to src/a.rs is back  u undoes`.
///
/// The tick is the renderer's (`Status::Done`), so it follows `what`. A
/// subject nobody can name is the commit's sha7, never a blank.
pub(crate) fn done_status(
    params: &RecoverParams,
    subject_of: impl Fn(Oid) -> Option<String>,
    keys: &graft_ui::keymap::Keymap,
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
        keys: undoes(keys),
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

/// The put-back box's title (J1, m7-power-tools/24-a2):
/// `Preview: put main back to 9a8b7c6 "Add rate limiter"`,
/// `Preview: make spike again at 4d5e6f7 "Try it"`,
/// `Preview: put back your edit to src/a.rs`. A subject nobody can name is
/// left out: the sha7 is already there.
///
/// Plain words; `preview_box` sanitises the whole line.
pub(crate) fn preview_title(
    params: &RecoverParams,
    subject_of: impl Fn(Oid) -> Option<String>,
) -> String {
    let at = |oid: Oid| match subject_of(oid) {
        Some(subject) => format!("{} \"{subject}\"", super::place::short(oid)),
        None => super::place::short(oid),
    };
    let what = match &params.target {
        RecoverTarget::MoveRef { name, to, .. } => format!("put {name} back to {}", at(*to)),
        RecoverTarget::Recreate { name, at: tip } => format!("make {name} again at {}", at(*tip)),
        RecoverTarget::Files { paths, .. } => format!("put back {}", your_edits(paths)),
    };
    format!("Preview: {what}")
}

/// The width the box's `now` / `after y` labels are padded to (J1).
const LABEL: usize = 10;

/// The put-back box's lines (J1): `now` and `after y` for a branch,
/// `your files: …` when the plan says ([`files_words`]; `None` draws no
/// line, never an invented one), and `u undoes it`.
///
/// Plain strings; `preview_box` sanitises each into `PreviewBox.lines`.
pub(crate) fn preview_lines(
    params: &RecoverParams,
    files: Option<&FilesEffect>,
    subject_of: impl Fn(Oid) -> Option<String>,
    keys: &graft_ui::keymap::Keymap,
) -> Vec<String> {
    // `main → 3d4e5f6  Add metrics endpoint`, the sha7 alone when no
    // subject is known.
    let tip = |name: &str, oid: Oid| match subject_of(oid) {
        Some(subject) => format!("{name} \u{2192} {}  {subject}", super::place::short(oid)),
        None => format!("{name} \u{2192} {}", super::place::short(oid)),
    };
    let labelled = |label: &str, rest: String| format!("{label:<LABEL$}{rest}");
    let mut lines = match &params.target {
        RecoverTarget::MoveRef { name, to, expected } => vec![
            labelled("now", tip(name, *expected)),
            labelled("after y", tip(name, *to)),
        ],
        RecoverTarget::Recreate { name, at } => vec![
            labelled("now", format!("{name} is gone")),
            labelled("after y", tip(name, *at)),
        ],
        // No branch moves: the files line says it all.
        RecoverTarget::Files { .. } => Vec::new(),
    };
    lines.extend(files.map(files_words));
    lines.extend(undoes_it(keys));
    lines
}

/// `your files: …` from the plan's [`FilesEffect`]: `unchanged (main isn't
/// checked out)`, or the clauses that apply joined by ` · ` —
/// `src/a.rs comes back`, `src/b.rs +2 more go`, `what's in src/a.rs now
/// is replaced`, `src/a.rs stays staged as you had it` (m7/30p R2),
/// `1 unsaved file set aside, put back after` — and
/// `unchanged` when none does.
pub(crate) fn files_words(effect: &FilesEffect) -> String {
    let clauses = match effect {
        FilesEffect::Unchanged { not_checked_out } => {
            return format!("your files: unchanged ({not_checked_out})");
        }
        FilesEffect::Changes {
            come_back,
            go,
            set_aside,
            replaced,
            still_staged,
        } => {
            let mut clauses: Vec<String> = [
                listed(come_back, "comes back", "come back"),
                listed(go, "goes", "go"),
                replaced_words(replaced),
                listed(
                    still_staged,
                    "stays staged as you had it",
                    "stay staged as you had them",
                ),
            ]
            .into_iter()
            .flatten()
            .collect();
            match set_aside {
                0 => {}
                1 => clauses.push("1 unsaved file set aside, put back after".to_owned()),
                count => clauses.push(format!("{count} unsaved files set aside, put back after")),
            }
            clauses
        }
    };
    if clauses.is_empty() {
        return "your files: unchanged".to_owned();
    }
    format!("your files: {}", clauses.join(" \u{b7} "))
}

/// `what's in src/a.rs now is replaced` / `what's in src/a.rs +N more now
/// is replaced` (m7/30o R1): his changes a file put-back writes over. No
/// key named: the box's last line is already `u undoes it`.
fn replaced_words(paths: &[std::path::PathBuf]) -> Option<String> {
    let first = paths.first()?.to_string_lossy();
    Some(match paths.len() {
        1 => format!("what's in {first} now is replaced"),
        count => format!("what's in {first} +{} more now is replaced", count - 1),
    })
}

/// `<p> comes back` / `<p> +N more come back`; `None` for no paths.
fn listed(paths: &[std::path::PathBuf], one: &str, many: &str) -> Option<String> {
    let first = paths.first()?.to_string_lossy();
    Some(match paths.len() {
        1 => format!("{first} {one}"),
        count => format!("{first} +{} more {many}", count - 1),
    })
}

/// `your edit to src/a.rs`, `your edits to src/a.rs +2 more`.
fn your_edits(paths: &[std::path::PathBuf]) -> String {
    // `lost_edits` never makes an edit with no files; the words still hold.
    let Some(first) = paths.first() else {
        return "your edits".to_owned();
    };
    let first = first.to_string_lossy();
    match paths.len() {
        1 => format!("your edit to {first}"),
        count => format!("your edits to {first} +{} more", count - 1),
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
                // Unread (`None`) is unknown here too: the caller's words
                // fall back to the sha7 (`subject_of`).
                ListRow::Lost { commit, .. } if commit.run.tip == oid => commit.subject.clone(),
                _ => None,
            })
    })
}

#[cfg(test)]
mod tests;

/// `u undoes`, the key a put-back's done line names (`app.undo`, on the
/// graph it lands on); `None` when unbound.
pub(crate) fn undoes(keys: &graft_ui::keymap::Keymap) -> Option<String> {
    keys.key_verb(&[graft_ui::keymap::ActionId("app.undo")], "undoes")
}

/// `u undoes it`, the put-back preview's last line; none when unbound.
pub(crate) fn undoes_it(keys: &graft_ui::keymap::Keymap) -> Option<String> {
    keys.key_verb(&[graft_ui::keymap::ActionId("app.undo")], "undoes it")
}
