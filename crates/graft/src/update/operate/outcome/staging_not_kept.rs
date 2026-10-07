//! The put-back fallback's line (30b): the write landed, its staging did
//! not come back, and the status row says which files and how to undo it.

use std::path::PathBuf;

use graft_ui::keymap::{ActionId, Keymap};

use crate::update::state::Status;

/// The key half of every [`staging_not_kept_rungs`] wording, spelled from
/// his keymap (m8/10f): `u undoes it, staging included`, `None` with undo
/// unbound — every rung then ends at its words (10b W3).
fn staging_undo(keys: &Keymap) -> Option<String> {
    keys.key_verb(&[ActionId("app.undo")], "undoes it, staging included")
}

/// `words · <clause>`, or `words` alone with no clause.
fn with_clause(words: String, clause: Option<&str>) -> String {
    match clause {
        Some(clause) => format!("{words} \u{b7} {clause}"),
        None => words,
    }
}

/// The fallback's status wordings, longest first (planner 30b, Decided 3).
/// `what` is what `Done` would put before its tick:
/// 1. `<what> ✓ · your edits are back · staging not kept: <paths> · u undoes it, staging included`
/// 2. `<what> ✓ · staging not kept: <paths> · u undoes it, staging included`
/// 3. `staging not kept: <paths> · u undoes it, staging included`
///
/// The cut drops `your edits are back` first: the files and the key are
/// what he must read. `<paths>` is `<first>[ +N more]`, as graft-ops'
/// `AfterState::StagingNotKept` words it (pinned by
/// `staging_not_kept_words_match_graft_ops`).
///
/// A staged-new file he deleted that git checked out again (`came_back`,
/// graft-ops' `came_back_words`) is named before the staging news on every
/// rung but a last one, which only a row too narrow for it reaches: a file
/// back on disk is never silent (review m8-14 Major 3).
pub(crate) fn staging_not_kept_rungs(
    what: &str,
    paths: &[PathBuf],
    came_back: &[PathBuf],
    keys: &Keymap,
) -> Vec<String> {
    let undo = staging_undo(keys);
    let undo = undo.as_deref();
    let first = paths
        .first()
        .map_or_else(String::new, |path| path.display().to_string());
    let named = match paths.len() {
        0 | 1 => first,
        n => format!("{first} +{} more", n - 1),
    };
    let staging = format!("staging not kept: {named}");
    let lost = match graft_ops::apply::came_back_words(came_back) {
        Some(came_back) => format!("{came_back} \u{b7} {staging}"),
        None => staging.clone(),
    };
    let done_line = crate::view::status::done_line;
    let mut rungs = vec![
        done_line(
            what,
            Some(&format!("your edits are back \u{b7} {lost}")),
            undo,
        ),
        done_line(what, Some(&lost), undo),
        with_clause(lost.clone(), undo),
    ];
    if lost != staging {
        rungs.push(with_clause(staging, undo));
    }
    rungs
}

/// The fallback's line for a status row `columns` wide: the longest rung
/// that fits the row's text (the row indents one cell), else the last —
/// `graft_ui::preview_box::fitting_line`'s rule, since the status line has
/// no ladder of its own (30b). **Sticky**, not `Done`: `Done` clears on the
/// next key, and this line is the only place that says the staging went.
/// The key half rides `keys`, so the footer's own fit keeps it whole.
pub(crate) fn staging_not_kept_status(
    what: &str,
    paths: &[PathBuf],
    came_back: &[PathBuf],
    columns: u16,
    keys: &Keymap,
) -> Status {
    // `fitting_line` measures a box `width` wide whose text is 4 cells
    // narrower; the status row's text is `columns - 1` (it indents one
    // cell), so the same rule is asked with the box 3 cells wider.
    let rungs = staging_not_kept_rungs(what, paths, came_back, keys);
    let picked = graft_ui::preview_box::fitting_line(&rungs, columns.saturating_add(3));
    let undo = staging_undo(keys);
    let reason = match &undo {
        Some(undo) => {
            let suffix = format!(" \u{b7} {undo}");
            picked.strip_suffix(&suffix).unwrap_or(&picked).to_owned()
        }
        None => picked,
    };
    Status::Sticky { reason, keys: undo }
}
