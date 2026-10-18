//! The tour card's words (ticket m8-polish-and-ship/19b): one line per
//! `mergewright_ops::tour::TourTask`, every key spelled from his live keymap
//! (ADR-0055) — a tour that teaches the default keys after he moved them is
//! helix #13483's complaint.

use mergewright_ops::tour::TourTask;
use mergewright_term::sanitize_line;
use mergewright_ui::keymap::{ActionId, Keymap};
use mergewright_ui::tour_card::TourCardView;

use mergewright_ops::keys::OpKey;

use crate::update::AppState;
use crate::update::op_keys::op_action;

/// The card for this frame: `None` outside the tour. Once all eight are
/// done the tour is being left; the last task is shown until it is.
#[must_use]
pub(crate) fn card(state: &AppState) -> Option<TourCardView> {
    let tour = state.tour.as_ref()?;
    let at = tour.current().unwrap_or(TourTask::ALL.len() - 1);
    let task = TourTask::ALL[at];
    let keys = &state.keymap;
    Some(TourCardView {
        step: at + 1,
        of: TourTask::ALL.len(),
        title: sanitize_line(title(task)),
        line: sanitize_line(&line(task, keys)),
        ticks: TourTask::ALL
            .iter()
            .zip(tour.done)
            .map(|(task, done)| (sanitize_line(tick(*task)), done))
            .collect(),
        leave: keys
            .key_verb(&[crate::update::tour::LEAVE], "leave")
            .map(|words| sanitize_line(&words)),
    })
}

/// The task in a few words, on the card's top border.
#[must_use]
pub(crate) fn title(task: TourTask) -> &'static str {
    match task {
        TourTask::StageHunk => "stage a change",
        TourTask::Commit => "commit it",
        TourTask::Undo => "undo that commit",
        TourTask::StashAndBack => "stash and back",
        TourTask::Branch => "make a branch",
        TourTask::MergeWithPreview => "merge feature",
        TourTask::ResolveConflict => "resolve a conflict",
        TourTask::Push => "push",
    }
}

/// The task's one word among the ticks.
fn tick(task: TourTask) -> &'static str {
    match task {
        TourTask::StageHunk => "stage",
        TourTask::Commit => "commit",
        TourTask::Undo => "undo",
        TourTask::StashAndBack => "stash",
        TourTask::Branch => "branch",
        TourTask::MergeWithPreview => "merge",
        TourTask::ResolveConflict => "conflict",
        TourTask::Push => "push",
    }
}

/// His key for `action`, or `—` when he unbound it.
fn key(keys: &Keymap, action: ActionId) -> String {
    keys.spell_keys(&[action])
        .unwrap_or_else(|| "\u{2014}".to_owned())
}

/// The line that teaches `task`, keys spelled from `keys`. Ops keys through
/// `op_action` (the table the ops crate's words read); the rest by their
/// own rows: `app.open`, `graph.branch`, `stash.save`, `stash.pop`,
/// `confirm.yes`, `resolver.take_theirs`, `resolver.mark_done`.
#[must_use]
pub(crate) fn line(task: TourTask, keys: &Keymap) -> String {
    let op = |op: OpKey| key(keys, op_action(op));
    let row = |id: &'static str| key(keys, ActionId(id));
    match task {
        TourTask::StageHunk => format!(
            "{} on the top row opens your changes \u{b7} {} stages the change under the cursor",
            row("app.open"),
            op(OpKey::Stage)
        ),
        TourTask::Commit => format!("{} writes a commit from what you staged", op(OpKey::Commit)),
        TourTask::Undo => format!(
            "{} takes back the last thing you did \u{2014} nothing is lost",
            op(OpKey::Undo)
        ),
        TourTask::StashAndBack => format!(
            "{} opens the stashes \u{b7} {} sets your edits aside \u{b7} {} brings them back",
            op(OpKey::Stash),
            row("stash.save"),
            row("stash.pop")
        ),
        TourTask::Branch => format!("{} makes a branch at the cursor", row("graph.branch")),
        TourTask::MergeWithPreview => format!(
            "{} on feature shows the merge before it happens \u{b7} {} does it",
            op(OpKey::Merge),
            row("confirm.yes")
        ),
        // The keys the app answers on this path (19c-b's walk, 19c-c C+1):
        // `y` on the merge preview lands in the resolver already; with the
        // clash solved, `y` opens the merge's review (ease-pass/05b — a
        // merge's `⏎` marks nothing), whose `y` finishes.
        TourTask::ResolveConflict => format!(
            "{} on clash \u{b7} {} stops on its conflict \u{b7} {} takes theirs \u{b7} {} reviews the merge \u{b7} {} finishes",
            op(OpKey::Merge),
            row("confirm.yes"),
            row("resolver.take_theirs"),
            row("resolver.review"),
            row("confirm.yes")
        ),
        TourTask::Push => format!("{} sends main to origin", op(OpKey::Push)),
    }
}

/// The header's path in the practice repository (mock-ups §1.0, ADR-0056):
/// `practice · <tmp> · a throwaway repo, not yours`.
#[must_use]
pub(crate) fn practice_path(shown: &str) -> String {
    format!("practice \u{b7} {shown} \u{b7} a throwaway repo, not yours")
}

#[cfg(test)]
mod tests;
