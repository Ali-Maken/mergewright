//! J's `/` find (m7-power-tools/30o-c, keymap §2.11): a row by the words
//! that name it, behaving key for key as blame's find (§2.8,
//! `update::place::keys`'s `prompt_key`, `step_find` and `escape`).
//!
//! As blame's, the find is not incremental: typing fills the prompt on the
//! bar, `⏎` keeps the find and moves the cursor to the first hit at or
//! below the row it is on, wrapping; `n`/`N` step the kept find's hits,
//! wrapping; `Esc` while typing closes the prompt (the cursor never moved),
//! and `Esc` with a find kept drops it. The hits are worked out once per
//! find (`⏎`) and once per list rebuild ([`refresh_hits`]), so rows
//! arriving in stages 2 and 3 keep the find, the cursor stays on its
//! `RowKey`, and a frame or an `n` never scans the list (30p-d).
//!
//! Two keys blame's prompt drops, J's takes: a paste (the ticket's
//! acceptance) goes into the prompt. `Ctrl+U` does nothing, as in blame.

use std::collections::HashMap;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::EntryId;
use graft_ops::journal_list::{HeldBy, ListRow};

use super::{AppState, JournalState, row_key};

/// J's find: the prompt while he types, and the find `⏎` kept. Both can be
/// up at once, as in blame: a new `/` over a kept find leaves the old one in
/// force until `⏎`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct JournalFind {
    /// The `/` prompt: `Some` while the keys go into it.
    pub(crate) typing: Option<String>,
    /// The find `⏎` kept: `n`/`N` step its hits, the title counts them.
    pub(crate) kept: Option<String>,
    /// Row indices into `journal.list` that hold `kept`, newest first.
    /// Rebuilt by [`refresh_hits`] only.
    pub(crate) hits: Vec<usize>,
}

/// Recompute `find.hits` from `kept`. Called by `find_key` on `⏎` and by
/// `rebuild` (the list changed under a kept find); nowhere else, so a
/// frame or a step reads the hits without scanning the list.
pub(crate) fn refresh_hits(journal: &mut JournalState) {
    journal.find.hits = match &journal.find.kept {
        Some(query) => hits(journal, query),
        None => Vec::new(),
    };
}

/// Rows whose naming words hold `query`, as indices into the list, newest
/// first (the list's order). Blank → empty. Case-insensitive literal,
/// folded as `search::Query` and blame's find do (`to_lowercase` on both
/// sides).
///
/// The naming words, per row (R4): a `●` row's description; a `$` row's
/// plain words, git's subject and the ref's short name; a `✗` commit's
/// sha7, subject and the branch that held it; a lost edit's paths. Never
/// the verdict words (`aborted by you`, `failed: …`): state, not names.
pub(crate) fn hits(journal: &JournalState, query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let Some(list) = &journal.list else {
        return Vec::new();
    };
    let folded = query.to_lowercase();
    // One lookup per call: a scan of the entries per `●` row made a find
    // over a long journal rows × entries.
    let descriptions: HashMap<&EntryId, &str> = journal
        .newest_first()
        .into_iter()
        .map(|entry| (&entry.id, entry.description.as_str()))
        .collect();
    (0..list.rows.len())
        .filter(|&index| {
            names(&list.rows[index], &descriptions)
                .iter()
                .any(|words| words.to_lowercase().contains(&folded))
        })
        .collect()
}

/// The words that name `row`.
fn names(row: &ListRow, descriptions: &HashMap<&EntryId, &str>) -> Vec<String> {
    match row {
        // The list is built from these very entries (`rebuild`); an id not
        // among them names nothing, as the view draws such a row blank.
        ListRow::Graft(id) => descriptions
            .get(id)
            .map(|description| (*description).to_owned())
            .into_iter()
            .collect(),
        ListRow::Outside(step) => {
            let refname = &step.record.refname;
            let short = refname.strip_prefix("refs/heads/").unwrap_or(refname);
            vec![
                step.words.clone(),
                step.record.subject.clone(),
                short.to_owned(),
            ]
        }
        ListRow::Lost { commit, .. } => {
            let mut sha = commit.run.tip.to_string();
            sha.truncate(7);
            let mut words = vec![sha];
            if let Some(subject) = &commit.subject {
                words.push(subject.clone());
            }
            if let HeldBy::Branch { name, .. } = &commit.held_by {
                words.push(name.clone());
            }
            words
        }
        ListRow::LostEdit { edit, .. } => edit
            .paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    }
}

/// The find's count for the title: which hit the cursor is at (the hits at
/// or above it, at least 1, as blame's `FindTitle`) and how many. `None`
/// with no find kept. Reads the cached [`JournalFind::hits`]; the list is
/// not scanned per frame.
pub(crate) fn count(journal: &JournalState) -> Option<(usize, usize)> {
    journal.find.kept.as_ref()?;
    let hits = &journal.find.hits;
    let selected = journal.selected();
    let at = hits.partition_point(|&at| at <= selected).max(1);
    Some((at, hits.len()))
}

/// The find's prompt is taking the keys: every global binding (`u`, `U`,
/// `Ctrl+Z`) must leave them to it (`AppState::text_field_open`).
#[must_use]
pub(crate) fn typing(state: &AppState) -> bool {
    state.journal.shown && state.journal.find.typing.is_some()
}

/// `/`, typing, `⏎`, `Esc`, and `n`/`N` while a find is kept. `true` =
/// handled, and the key goes nowhere else.
///
/// Runs ahead of the graph's search (`update::core_event`): with J up, `/`,
/// `n` and `N` are J's (§2.11). A box over J (the put-back box, a confirm)
/// or the name prompt owns the keys first, so this answers only when no
/// modal does — its own prompt aside.
pub(crate) fn find_key(state: &mut AppState, key: KeyPress) -> bool {
    if !state.journal.shown {
        return false;
    }
    let plain = !key.modifiers.control && !key.modifiers.alt;
    if state.journal.find.typing.is_some() {
        if !plain {
            // `Ctrl+C` and the like: not the prompt's. Every global binding
            // asks `text_field_open` first, so none fires on a letter.
            return false;
        }
        state.journal.touched = true;
        prompt_key(state, key);
        return true;
    }
    // J's keys come first only while the keyboard is on it (graph focus,
    // `update::keys::pane_and_overlay_key`); with the changes pane focused
    // over it, `/` does nothing: the graph's search stands down under an
    // overlay (`update::search::graph_is_shown`, m8/09e).
    if !plain
        || state.modal_owns_keys()
        || state.detail.focus != graft_ui::detail::DetailFocus::Graph
    {
        return false;
    }
    match key.code {
        KeyCode::Char('/') => {
            // A key: the lost rows' preselect is off from here (24's rule).
            state.journal.touched = true;
            state.journal.find.typing = Some(String::new());
            true
        }
        KeyCode::Char(step @ ('n' | 'N')) if state.journal.find.kept.is_some() => {
            state.journal.touched = true;
            step_find(&mut state.journal, step == 'n');
            true
        }
        KeyCode::Escape if state.journal.find.kept.is_some() => {
            state.journal.touched = true;
            state.journal.find.kept = None;
            refresh_hits(&mut state.journal);
            true
        }
        _ => false,
    }
}

/// A paste while the prompt is open goes into it. `false` with no prompt:
/// the paste is some other field's.
pub(crate) fn find_paste(state: &mut AppState, text: &str) -> bool {
    if !typing(state) {
        return false;
    }
    if let Some(prompt) = state.journal.find.typing.as_mut() {
        // One line: a pasted newline is not a `⏎`.
        prompt.extend(text.chars().filter(|c| !c.is_control()));
    }
    true
}

/// The prompt's keys, as blame's `prompt_key`: text in, `⏎` keeps the find
/// and moves to the first hit at or below the cursor (wrapping), `Esc`
/// closes the prompt and nothing else.
fn prompt_key(state: &mut AppState, key: KeyPress) {
    let journal = &mut state.journal;
    let Some(prompt) = journal.find.typing.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Char(typed) => prompt.push(typed),
        KeyCode::Backspace => {
            prompt.pop();
        }
        KeyCode::Escape => journal.find.typing = None,
        KeyCode::Enter => {
            let typed = std::mem::take(prompt);
            journal.find.typing = None;
            // An empty query closes the prompt: nothing to find (blame's).
            if !typed.is_empty() {
                journal.find.kept = Some(typed);
                refresh_hits(journal);
                let from = journal.selected();
                let hits = &journal.find.hits;
                if let Some(&at) = hits.iter().find(|&&at| at >= from).or_else(|| hits.first()) {
                    move_to(journal, at);
                }
            }
        }
        _ => {}
    }
}

/// `n`/`N`: the next (older) or previous (newer) hit, wrapping.
fn step_find(journal: &mut JournalState, older: bool) {
    let hits = &journal.find.hits;
    let from = journal.selected();
    let next = if older {
        hits.iter().find(|&&at| at > from).or_else(|| hits.first())
    } else {
        hits.iter()
            .rev()
            .find(|&&at| at < from)
            .or_else(|| hits.last())
    };
    if let Some(&at) = next {
        move_to(journal, at);
    }
}

fn move_to(journal: &mut JournalState, index: usize) {
    if let Some(list) = &journal.list
        && let Some(key) = row_key(list, index)
    {
        journal.cursor = Some(key);
    }
}

#[cfg(test)]
mod tests;
