//! `y` (and `c`) in a merge's resolver: the review (ease-pass/05b).
//!
//! A merge stages nothing while the user picks; the review is where the
//! solved files are staged, inside the finish itself. So `y` opens it once
//! nothing the resolver knows of is still open, and says how many clashes
//! are left otherwise. Files the resolver has not read are the review
//! read's to judge (`graft_ops::finish::merge_review`).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::state::InProgress;
use graft_ui::keymap::{ActionId, Keymap};

use super::files::{RowState, listed_files};
use super::handover;
use super::resolved::set_notice;
use crate::update::events::Command;
use crate::update::finish::ReturnTo;
use crate::update::state::{AppState, Status};

/// Is the operation underneath a merge? Only a merge has a review; in a
/// rebase, cherry-pick, revert or stash put-back `y` is not bound to
/// anything the user can see.
pub(crate) fn in_a_merge(state: &AppState) -> bool {
    crate::update::banner::live(state).is_some_and(|detail| detail.kind == InProgress::Merge)
}

/// `y` (`resolver.review`), and plain `c` in a merge — its door too: the
/// finish stages the solved files, so it shows them first. `true` when
/// `key` was one of them; `y` outside a merge answers as an unbound key.
pub(super) fn review_keys(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let plain = !key.modifiers.control && !key.modifiers.alt;
    let c_in_a_merge = key.code == KeyCode::Char('c') && plain && in_a_merge(state);
    if !c_in_a_merge && !asks_review(key) {
        return false;
    }
    review_pressed(state, commands, next_id);
    true
}

/// `key` is one of `resolver.review`'s chords as shipped (`y`): the edge
/// hands a handler shipped chords only (`update/edge.rs`), so this is the
/// action, whatever he moved it to — as `unpick::asks_undo` reads undo.
fn asks_review(key: KeyPress) -> bool {
    use graft_ui::keymap::KeyChord;
    KeyChord::from_press(&key).is_some_and(|chord| {
        graft_ui::keymap::Keymap::shipped()
            .chords(&ActionId("resolver.review"))
            .contains(&chord)
    })
}

/// `y` (`resolver.review`), and `c` in a merge: the review, or how many
/// clashes are still open. Outside a merge, nothing — as an unbound key.
fn review_pressed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if !in_a_merge(state) {
        return;
    }
    // The review reads the files on disk: a pick still landing would not
    // be in it.
    if let Some(words) = handover::handover_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    if let Some(words) = clashes_open(state) {
        set_notice(state, (words, false));
        return;
    }
    crate::update::banner::continue_pressed(state, commands, ReturnTo::Resolver, next_id);
}

/// `N clash(es) still open · n goes to the next`, when any clash this
/// resolver has read is open.
fn clashes_open(state: &AppState) -> Option<String> {
    let open = open_clashes(state);
    let clashes = if open == 1 { "clash" } else { "clashes" };
    (open > 0).then(|| still_open_words(&format!("{open} {clashes} still open"), &state.keymap))
}

/// The notice once the status read a stage asked for has landed
/// (`status_read::settle`): `c`'s refusal in the notice's words, or none
/// when `c` would go on. In a merge the unmerged files may be solved on
/// disk, waiting for the finish to stage them, so a status cannot call
/// them conflicted: only the clashes this resolver has read count, and
/// with none open the review's read judges the rest.
#[must_use]
pub fn notice_once_checked(state: &AppState) -> Option<(String, bool)> {
    if in_a_merge(state) {
        return clashes_open(state).map(|words| (words, false));
    }
    crate::update::banner::continue_refusal(state)
        .map(|reason| finish_refused_notice(&reason, &state.keymap))
}

/// The clashes still open in the files this resolver has read: each open
/// block of a marker document, and one for each whole-file or binary
/// conflict (its one clash is the whole file). An unread file counts none
/// here; the review's read judges it from disk.
fn open_clashes(state: &AppState) -> usize {
    listed_files(state)
        .iter()
        .map(|row| row_open(&row.state))
        .sum()
}

/// One row's open clashes, as [`open_clashes`] counts them.
fn row_open(row: &RowState) -> usize {
    match row {
        RowState::Open { open, .. } => *open,
        RowState::WholeFile { .. } | RowState::Binary => 1,
        RowState::AllTaken | RowState::Done | RowState::Unread => 0,
    }
}

/// In a merge, whether a clash may still be open in a file other than the
/// one on screen — one open, or a file not read yet, which `n` walks to;
/// `None` outside a merge. What the words after a solved file point to:
/// `n` to that clash, or — none left — `y` to the review.
pub(super) fn open_elsewhere(state: &AppState) -> Option<bool> {
    in_a_merge(state).then(|| next_open_file(state).is_some())
}

/// The first file in the list's order, other than the one on screen, where
/// a clash may still be open — one open, or a file not read yet: where `n`
/// goes from a solved file, and the file the bar names (ease-pass/05c).
#[must_use]
pub(crate) fn next_open_file(state: &AppState) -> Option<std::path::PathBuf> {
    let here = state.resolver.as_ref().map(|open| open.path.as_path());
    listed_files(state)
        .into_iter()
        .find(|row| {
            Some(row.path.as_path()) != here
                && (row.state == RowState::Unread || row_open(&row.state) > 0)
        })
        .map(|row| row.path)
}

/// `head` and a merge's way on from a solved file: `n` while a clash is
/// open elsewhere, else `y` (ease-pass/05b).
pub(super) fn solved_way_on(head: &str, open_elsewhere: bool, keys: &Keymap) -> String {
    if open_elsewhere {
        still_open_words(head, keys)
    } else {
        reviews_the_merge(head, keys)
    }
}

/// `reason` and the way on: `{reason} · n goes to the next`, the key
/// spelled from his keymap (left out when unbound).
#[must_use]
pub fn still_open_words(reason: &str, keys: &Keymap) -> String {
    match keys.key_verb(&[ActionId("resolver.next_block")], "goes to the next") {
        Some(next) => format!("{reason} \u{b7} {next}"),
        None => reason.to_owned(),
    }
}

/// `{head} · y reviews the merge`, the key spelled from his keymap (left
/// out when unbound): the words a merge's resolver says when nothing is
/// open, whichever key asked.
#[must_use]
pub fn reviews_the_merge(head: &str, keys: &Keymap) -> String {
    match keys.key_verb(&[ActionId("resolver.review")], "reviews the merge") {
        Some(review) => format!("{head} \u{b7} {review}"),
        None => head.to_owned(),
    }
}

/// `c`'s refusal as the resolver's notice row says it: the reason, and the
/// key that marks a file done **here** — `⏎` (§2.6), where the banner's
/// line names `space`, the key outside (§2.6a).
///
/// Spelled from the resolver's own `resolver.mark_done` row: the place is
/// the open resolver, whose notice row this is, and its footer names the
/// same row (m8/10j ruling 5). Unbound → the reason alone.
pub(crate) fn finish_refused_notice(
    reason: &str,
    keys: &graft_ui::keymap::Keymap,
) -> (String, bool) {
    let marks = keys.key_verb(
        &[graft_ui::keymap::ActionId("resolver.mark_done")],
        "marks done",
    );
    let words = match marks {
        Some(marks) => format!("{reason} · {marks}"),
        None => reason.to_owned(),
    };
    (words, false)
}
