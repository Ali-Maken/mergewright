//! The chain preview's rows (m7/05, note R2/R3): what a rebase or rewrite
//! box adds when branches follow his, when the bottom branch is already on
//! `onto`, when the replay may stop at a step, and when a branch it moves is
//! on its upstream.
//!
//! Words exactly as the note's, but for the keep key: `· R to keep them`
//! waits for `rebase-preview-keeps-drops`, which binds it. A row naming an
//! unbound key is false.

use graft_ops::operation::{Operation, Source};
use graft_ops::preview::{ConflictForecast, DropSuggestion, PreviewPlan};

use super::{ref_line, short, slice};

/// `branches: ` — the label, and the indent the rows under it take.
const LABEL: &str = "branches: ";

/// A dependent's words: it moves because it is built on his.
const FOLLOWS: &str = "built on yours \u{b7} follows";

/// Whether `plan` moves a line of branches: a rebase or rewrite moving
/// more than his branch. A plain one-branch rebase draws as before m7.
pub(super) fn is_line(plan: &PreviewPlan, op: Option<&Operation>) -> bool {
    matches!(op, Some(Operation::Rebase(_) | Operation::Rewrite(_))) && plan.ref_moves.len() > 1
}

/// One row per branch of the line, bottom first, under a `branches:`
/// label: his branch's row as [`ref_line`] draws it, each dependent's
/// with [`FOLLOWS`] after it. The branch name leads every row, so a cut at
/// the tail never reaches it.
pub(super) fn ref_rows(plan: &PreviewPlan) -> Vec<String> {
    let indent = " ".repeat(LABEL.len());
    plan.ref_moves
        .iter()
        .enumerate()
        .map(|(at, moved)| match at {
            0 => format!("{LABEL}{}", ref_line(plan, moved)),
            _ => format!("{indent}{} \u{b7} {FOLLOWS}", ref_line(plan, moved)),
        })
        .collect()
}

/// `already in main: feature-a's 3 commits match 2a1f0e9 "Add OAuth (#42)"
/// · dropped`, or `already in main: not checked (<why>)`. `None` when the
/// check found nothing, or for any operation but a rebase.
pub(super) fn already_in_row(plan: &PreviewPlan, op: Option<&Operation>) -> Option<String> {
    let Some(Operation::Rebase(params)) = op else {
        return None;
    };
    let onto = match &params.onto {
        Source::Branch(name) => String::from_utf8_lossy(&slice::short_of(name)).into_owned(),
        Source::Commit(oid) => short(*oid),
    };
    match plan.already_in.as_ref()? {
        DropSuggestion::Drop(found) => {
            let branch = String::from_utf8_lossy(&slice::short_of(&String::from_utf8_lossy(
                found.branch.as_bytes(),
            )))
            .into_owned();
            let count = found.commits.len();
            Some(format!(
                "already in {onto}: {branch}'s {count} {} match {} \"{}\" \u{b7} dropped",
                if count == 1 { "commit" } else { "commits" },
                short(found.matches),
                found.subject
            ))
        }
        DropSuggestion::NotChecked(why) => Some(format!("already in {onto}: not checked ({why})")),
    }
}

/// The stop forecast's wordings, longest first, for `fitting_line`: the
/// paths go first, then the subject, then `· not checked past it`. The
/// step count and the commit are never cut. Not the stopped banner's
/// `stop_wordings`: that ladder is for a stop that happened, and a grep
/// test (`stop_ladder_readers`) pins who reads it.
pub(super) fn forecast_rungs(plan: &PreviewPlan) -> Option<Vec<String>> {
    let ConflictForecast::StopsAt {
        step,
        of,
        commit,
        paths,
    } = &plan.conflicts
    else {
        return None;
    };
    let ghost = plan
        .ghosts
        .iter()
        .find(|ghost| ghost.replays == Some(*commit));
    // A row with a ghost is one git picks (pick, reword or edit); a melding
    // row has none, and is named by its id alone.
    let what = match ghost {
        Some(_) => format!("pick {}", short(*commit)),
        None => short(*commit),
    };
    let head = format!("may stop at step {step} of {of}");
    let named = match ghost {
        Some(ghost) => format!("{head} ({what} {})", ghost.subject),
        None => format!("{head} ({what})"),
    };
    let bare = format!("{head} ({what})");
    let tail = " \u{b7} not checked past it";
    Some(vec![
        format!("{named}: {}{tail}", super::list(paths, 3)),
        format!("{named}{tail}"),
        format!("{bare}{tail}"),
        bare,
    ])
}

/// `on origin: feature-a · P will need force after`: the branches whose
/// upstream holds a commit this rewrites. `None` when there are none.
pub(super) fn published_row(plan: &PreviewPlan) -> Option<String> {
    if plan.published.is_empty() {
        return None;
    }
    let names: Vec<String> = plan
        .published
        .iter()
        .map(|name| {
            String::from_utf8_lossy(&slice::short_of(&String::from_utf8_lossy(name.as_bytes())))
                .into_owned()
        })
        .collect();
    Some(format!(
        "on origin: {} \u{b7} P will need force after",
        names.join(", ")
    ))
}
