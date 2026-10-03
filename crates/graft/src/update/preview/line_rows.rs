//! The chain preview's rows (m7/05, note R2/R3): what a rebase or rewrite
//! box adds when branches follow his, when the bottom branch is already on
//! `onto`, when the replay may stop at a step, and when a branch it moves is
//! on its upstream.
//!
//! Words exactly as the note's. The keep key's `· R to keep them` is drawn
//! only while `R` is live (`preview::keeps_drops`): a row naming an unbound
//! key is false.

use graft_ops::operation::{Operation, Source};
use graft_ops::preview::{ConflictForecast, DropSuggestion, ForkFate, LeftBehind, PreviewPlan};

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
///
/// A fork that follows (ADR-0059) moves too, but is not built on his: its
/// ref move is not drawn here; [`fork_rows`] names it.
pub(super) fn ref_rows(plan: &PreviewPlan) -> Vec<String> {
    let indent = " ".repeat(LABEL.len());
    plan.ref_moves
        .iter()
        .enumerate()
        .filter(|(at, moved)| *at == 0 || !follows(plan, &moved.name))
        .map(|(at, moved)| match at {
            0 => format!("{LABEL}{}", ref_line(plan, moved)),
            _ => format!("{indent}{} \u{b7} {FOLLOWS}", ref_line(plan, moved)),
        })
        .collect()
}

/// Whether `name` (in full) is a fork that follows.
fn follows(plan: &PreviewPlan, name: &str) -> bool {
    plan.forks.iter().any(|fork| {
        fork.branch.as_bytes() == name.as_bytes() && matches!(fork.fate, ForkFate::Follows { .. })
    })
}

/// One row per named fork (ADR-0059 §2, design note "Pass 2c"), in the
/// plan's order (follows first, then stays, each oldest fork point first),
/// then `+N more stay` for the ones past the cap:
///
/// - `feature-x follows · 2 commits from a1b2c3d`
/// - `feature-x stays on a1b2c3d · would conflict in src/a.rs`
///
/// The name leads, so a cut at the tail never reaches it. Empty for a plan
/// with no forks (every kind but a rewrite).
pub(crate) fn fork_rows(plan: &PreviewPlan) -> Vec<String> {
    let mut rows: Vec<String> = plan
        .forks
        .iter()
        .map(|fork| {
            let name = String::from_utf8_lossy(fork.branch.shorthand()).into_owned();
            match &fork.fate {
                ForkFate::Follows { .. } => format!(
                    "{name} follows \u{b7} {} {} from {}",
                    fork.commits,
                    if fork.commits == 1 {
                        "commit"
                    } else {
                        "commits"
                    },
                    short(fork.at)
                ),
                ForkFate::Stays(why) => {
                    format!("{name} stays on {} \u{b7} {}", short(fork.at), why.words())
                }
            }
        })
        .collect();
    if plan.forks_more > 0 {
        rows.push(format!("+{} more stay", plan.forks_more));
    }
    rows
}

/// [`fork_rows`] as the box draws them: under the `branches:` label's
/// indent when the plan moves a line, flush otherwise.
pub(super) fn fork_lines(plan: &PreviewPlan, line: bool) -> Vec<String> {
    let indent = if line {
        " ".repeat(LABEL.len())
    } else {
        String::new()
    };
    fork_rows(plan)
        .into_iter()
        .map(|row| format!("{indent}{row}"))
        .collect()
}

/// The branches left on old commits that are not forks, so no
/// [`fork_rows`] row names them: a rewrite's backup parked at a tip of the
/// line, and every one `r` leaves (its forks never follow yet, so it has
/// none). A rewrite's `left_behind_more` is its `forks_more`, already
/// counted on `+N more stay`; it rides here only for a plan with no forks.
pub(crate) fn not_forks(plan: &PreviewPlan) -> (Vec<LeftBehind>, usize) {
    let left = plan
        .left_behind
        .iter()
        .filter(|left| {
            !plan
                .forks
                .iter()
                .any(|fork| fork.branch.as_bytes() == left.branch.as_bytes())
        })
        .cloned()
        .collect();
    let more = if plan.forks.is_empty() && plan.forks_more == 0 {
        plan.left_behind_more
    } else {
        0
    };
    (left, more)
}

/// The name a rebase's `onto` goes by in words: `main` for a branch, the
/// short sha for a bare commit. The `already in <x>:` row and the rewrite
/// editor's base row opened from it (`R`) both say it, so they agree.
pub(crate) fn onto_label(onto: &Source) -> String {
    match onto {
        Source::Branch(name) => String::from_utf8_lossy(&slice::short_of(name)).into_owned(),
        Source::Commit(oid) => short(*oid),
    }
}

/// `already in main: feature-a's 3 commits match 2a1f0e9 "Add OAuth (#42)"
/// · dropped`, `… · dropped · R to keep them` when `keeps` (`R` is live),
/// or `already in main: not checked (<why>)`. `None` when the check found
/// nothing, or for any operation but a rebase.
pub(super) fn already_in_row(
    plan: &PreviewPlan,
    op: Option<&Operation>,
    keeps: bool,
) -> Option<String> {
    let Some(Operation::Rebase(params)) = op else {
        return None;
    };
    let onto = onto_label(&params.onto);
    match plan.already_in.as_ref()? {
        DropSuggestion::Drop(found) => {
            let branch = String::from_utf8_lossy(&slice::short_of(&String::from_utf8_lossy(
                found.branch.as_bytes(),
            )))
            .into_owned();
            let count = found.commits.len();
            Some(format!(
                "already in {onto}: {branch}'s {count} {} match {} \"{}\" \u{b7} dropped{}",
                if count == 1 { "commit" } else { "commits" },
                short(found.matches),
                found.subject,
                if keeps { " \u{b7} R to keep them" } else { "" }
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

/// The `left behind:` row's wordings, longest first, for `fitting_line`
/// (m7/37): the branches off the line that stay on the old commits.
///
/// - one: `left behind: feature-b stays on the old dd21dca`
/// - more: `left behind: feature-b, feature-c stay on old commits`
///
/// then the names alone, then the first name and `+N`. `more` (the ones
/// past the cap, never named) rides as `+N` on every rung. Empty when
/// nothing is left behind.
pub(crate) fn left_behind_rungs(left: &[LeftBehind], more: usize) -> Vec<String> {
    let Some(first) = left.first() else {
        return Vec::new();
    };
    let name = |left: &LeftBehind| {
        String::from_utf8_lossy(&slice::short_of(&String::from_utf8_lossy(
            left.branch.as_bytes(),
        )))
        .into_owned()
    };
    let past = |n: usize| {
        if n == 0 {
            String::new()
        } else {
            format!(" +{n}")
        }
    };
    let names = left.iter().map(name).collect::<Vec<_>>().join(", ");
    let names = format!("left behind: {names}{}", past(more));
    let full = match (left, more) {
        ([only], 0) => format!(
            "left behind: {} stays on the old {}",
            name(only),
            short(only.on)
        ),
        _ => format!("{names} stay on old commits"),
    };
    let first = format!(
        "left behind: {}{}",
        name(first),
        past(left.len() - 1 + more)
    );
    let mut rungs = vec![full, names, first];
    rungs.dedup();
    rungs
}

/// The box's `left behind:` row, the longest rung that fits `columns`.
/// `None` when nothing is left behind.
/// Only the branches [`not_forks`] keeps.
pub(super) fn left_behind_row(plan: &PreviewPlan, columns: u16) -> Option<String> {
    let (left, more) = not_forks(plan);
    let rungs = left_behind_rungs(&left, more);
    (!rungs.is_empty()).then(|| graft_ui::preview_box::fitting_line(&rungs, columns))
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
