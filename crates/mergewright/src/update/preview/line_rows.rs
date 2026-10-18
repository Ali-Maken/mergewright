//! The chain preview's rows (m7/05, note R2/R3): what a rebase or rewrite
//! box adds when branches follow his, when the bottom branch is already on
//! `onto`, when the replay may stop at a step, and when a branch it moves is
//! on its upstream.
//!
//! Words exactly as the note's. The keep key's `· R to keep them` is drawn
//! only while `R` is live (`preview::keeps_drops`): a row naming an unbound
//! key is false.

use mergewright_ops::operation::{Operation, Source};
use mergewright_ops::preview::{
    ConflictForecast, DropSuggestion, Fork, ForkFate, LeftBehind, PreviewPlan, WhyStays,
};

use mergewright_ui::keymap::{ActionId, Keymap};

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

/// How loud a [`ForkRow`] is (design note "Pass 2c", rulings R3/R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ForkTone {
    /// A fork that follows: the dependents' look.
    Follows,
    /// A fork that stays because it would conflict: led by `!`, a warning.
    Warning,
    /// Every other branch that stays: true, nothing to act on.
    Muted,
}

/// One row of [`fork_rows`]: its words and its tone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForkRow {
    pub(crate) text: String,
    pub(crate) tone: ForkTone,
}

/// One row per named fork (ADR-0059 §2, design note "Pass 2c"), in the
/// plan's order (follows first, then stays, each oldest fork point first);
/// for a rewrite (`rewrite`), each branch parked at the old tip of the
/// line, last of the stays (ruling R1); then the count past the cap:
///
/// - `feature-x follows · 2 commits from a1b2c3d` (`· from a1b2c3d` with
///   no commits of its own)
/// - `! feature-x stays on a1b2c3d · would conflict in src/a.rs`
/// - `feature-x stays on a1b2c3d · the default branch`
/// - `keep stays on a1b2c3d · on the old tip`
/// - `+1 more stays`, `+N more stay`
///
/// The name leads every row but the warning's `!`, so a cut at the tail
/// never reaches it. Empty for a plan with no forks and no such branch
/// (every kind but a rewrite; `r` says `left behind:` instead).
pub(crate) fn fork_rows(plan: &PreviewPlan, rewrite: bool) -> Vec<ForkRow> {
    let mut rows: Vec<ForkRow> = plan.forks.iter().map(fork_row).collect();
    if rewrite {
        rows.extend(not_forks(plan).0.into_iter().map(|left| ForkRow {
            text: format!(
                "{} stays on {} \u{b7} on the old tip",
                String::from_utf8_lossy(left.branch.shorthand()),
                short(left.on)
            ),
            tone: ForkTone::Muted,
        }));
    }
    match plan.forks_more {
        0 => {}
        1 => rows.push(muted("+1 more stays".to_owned())),
        more => rows.push(muted(format!("+{more} more stay"))),
    }
    rows
}

fn muted(text: String) -> ForkRow {
    ForkRow {
        text,
        tone: ForkTone::Muted,
    }
}

/// One fork's row, by its fate.
fn fork_row(fork: &Fork) -> ForkRow {
    let name = String::from_utf8_lossy(fork.branch.shorthand()).into_owned();
    let at = short(fork.at);
    match &fork.fate {
        ForkFate::Follows { .. } => ForkRow {
            text: match fork.commits {
                0 => format!("{name} follows \u{b7} from {at}"),
                1 => format!("{name} follows \u{b7} 1 commit from {at}"),
                n => format!("{name} follows \u{b7} {n} commits from {at}"),
            },
            tone: ForkTone::Follows,
        },
        ForkFate::Stays(why @ WhyStays::WouldConflict { .. }) => ForkRow {
            text: format!("! {name} stays on {at} \u{b7} {}", why.words()),
            tone: ForkTone::Warning,
        },
        // Every other reason, named so a new one is a decision: nothing
        // would break, so no warning (`! ` is `WouldConflict`'s alone).
        // The last three are ADR-0059 §6's gates (14d).
        ForkFate::Stays(
            why @ (WhyStays::StopsBelow
            | WhyStays::HasMerge { .. }
            | WhyStays::Pushed { .. }
            | WhyStays::Elsewhere { .. }
            | WhyStays::Nested { .. }
            | WhyStays::DefaultBranch
            | WhyStays::PastCap
            | WhyStays::NotChecked(_)
            | WhyStays::Carries { .. }
            | WhyStays::WouldLose { .. }
            | WhyStays::WouldGain { .. }),
        ) => muted(format!("{name} stays on {at} \u{b7} {}", why.words())),
    }
}

/// [`fork_rows`] as the box draws them, in words only (the box draws
/// every line in one tone, ruling R3): under the `branches:` label's
/// indent when the plan moves a line, flush otherwise.
pub(super) fn fork_lines(plan: &PreviewPlan, line: bool, rewrite: bool) -> Vec<String> {
    let indent = if line {
        " ".repeat(LABEL.len())
    } else {
        String::new()
    };
    fork_rows(plan, rewrite)
        .into_iter()
        .map(|row| format!("{indent}{}", row.text))
        .collect()
}

/// The branches left on old commits that are not forks: a rewrite's branch
/// parked at a tip of the line (a [`fork_rows`] row, `· on the old tip`),
/// and every one `r` leaves (its forks never follow yet, so it has none;
/// its `left behind:` row). A rewrite's `left_behind_more` is its
/// `forks_more`, already counted by [`fork_rows`]; it rides here only for a
/// plan with no forks.
fn not_forks(plan: &PreviewPlan) -> (Vec<LeftBehind>, usize) {
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
    keys: &Keymap,
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
                keeps
                    .then(|| keep_them(keys))
                    .flatten()
                    .map_or_else(String::new, |keep| format!(" \u{b7} {keep}"))
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
/// Only the branches [`not_forks`] keeps, and never for a rewrite: its own
/// are [`fork_rows`] rows, one vocabulary per box (ruling R1).
pub(super) fn left_behind_row(plan: &PreviewPlan, rewrite: bool, columns: u16) -> Option<String> {
    if rewrite {
        return None;
    }
    let (left, more) = not_forks(plan);
    let rungs = left_behind_rungs(&left, more);
    (!rungs.is_empty()).then(|| mergewright_ui::preview_box::fitting_line(&rungs, columns))
}

/// `on origin: feature-a · P will need force after`: the branches whose
/// upstream holds a commit this rewrites. `None` when there are none.
pub(super) fn published_row(plan: &PreviewPlan, keys: &Keymap) -> Option<String> {
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
    Some(published_words(&names.join(", "), keys))
}

/// `on origin: <names> · P will need force after`: `P` is the graph's
/// push (`graph.push`), pressed there once the rewrite lands. Unbound, the
/// fact alone.
pub(crate) fn published_words(names: &str, keys: &Keymap) -> String {
    keys.after(
        &format!("on origin: {names}"),
        ActionId("graph.push"),
        "will need force after",
    )
}

/// `R to keep them`: the preview's `R` (`preview.rewrite`) opens the editor
/// with the suggested drops. `None` when unbound.
pub(crate) fn keep_them(keys: &Keymap) -> Option<String> {
    keys.key_verb(&[ActionId("preview.rewrite")], "to keep them")
}
