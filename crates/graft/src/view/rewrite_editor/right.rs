//! The editor's right side: the after-state from the plan, its ghost notes and group notes.

use super::{
    AfterFacts, AfterMark, AfterRow, AfterView, ChangesView, ConflictForecast, HashMap, Loading,
    Oid, PreviewPlan, RightSide, RowFacts, SanitizedText, Target, TodoAction, ViewFacts,
    sanitize_line, sanitize_path_bytes, short,
};
use graft_ops::preview::{ForkFate, WhyStays};

#[cfg(test)]
mod tests;

/// The right-hand side: `⏎`'s file list, else the AFTER plan.
pub(super) fn right_side(facts: &ViewFacts<'_>) -> RightSide {
    if let Some((commit, files)) = facts.changes {
        let files = match files {
            Loading::Idle | Loading::Pending(_) => ChangesView::Reading,
            Loading::Failed(why) => ChangesView::Failed(sanitize_line(why)),
            Loading::Ready(list) => ChangesView::Files(
                list.files
                    .iter()
                    .map(|file| {
                        // git counts lines for both sides of a text file and
                        // neither of a binary one.
                        let counts = file.added.zip(file.removed);
                        (sanitize_path_bytes(&file.path), counts)
                    })
                    .collect(),
            ),
        };
        return RightSide::Changes {
            short: short(commit),
            files,
        };
    }
    RightSide::After(match &facts.after {
        AfterFacts::Checking => AfterView::Checking,
        AfterFacts::Failed(why) => {
            AfterView::Failed(sanitize_line(&format!("couldn't plan the rewrite: {why}")))
        }
        AfterFacts::Refused(words) => AfterView::Refused(sanitize_line(words)),
        AfterFacts::Invalid(words) => AfterView::Refused(sanitize_line(words)),
        AfterFacts::Ready(planned) => after_ready(facts, &planned.plan),
    })
}

/// The plan's ghosts, per branch top down (the last dependent first, his
/// branch last), then the base; below them the facts.
pub(super) fn after_ready(facts: &ViewFacts<'_>, plan: &PreviewPlan) -> AfterView {
    let owner = ghost_owners(plan);
    let rows_by_commit: HashMap<Oid, &RowFacts<'_>> =
        facts.rows.iter().map(|row| (row.commit, row)).collect();
    let stop = match &plan.conflicts {
        ConflictForecast::StopsAt { commit, paths, .. } => plan
            .ghosts
            .iter()
            .position(|ghost| ghost.replays == Some(*commit))
            .map(|at| (at, paths.as_slice())),
        _ => None,
    };
    let his_tip = plan.ref_moves.first().and_then(|moved| match moved.to {
        Some(Target::Ghost(at)) => Some(at),
        _ => None,
    });
    let mut rows = Vec::new();
    for (branch, moved) in plan.ref_moves.iter().enumerate().rev() {
        let mut labelled = false;
        let mine = owner
            .iter()
            .filter(|(_, by)| *by == branch)
            .map(|(at, _)| *at);
        for at in mine.rev() {
            // `ghost_owners` walks this plan's own ghosts, so every index it
            // hands out is in range; one that is not has nothing to draw.
            let Some(ghost) = plan.ghosts.get(at) else {
                continue;
            };
            let replayed = ghost
                .replays
                .and_then(|commit| rows_by_commit.get(&commit).copied());
            let follows = (branch > 0 && !labelled).then(|| follows_note(plan, &moved.name));
            let notes = ghost_notes(facts, replayed, follows, stop, at);
            rows.push(AfterRow {
                branch: (!labelled).then(|| sanitize_line(&short_ref(&moved.name))),
                mark: if Some(at) == his_tip {
                    AfterMark::NewTip
                } else {
                    AfterMark::New
                },
                subject: sanitize_line(&ghost.subject),
                note: (!notes.is_empty()).then(|| sanitize_line(&notes.join(" \u{b7} "))),
            });
            labelled = true;
        }
    }
    rows.push(AfterRow {
        branch: Some(sanitize_line(&facts.base.label)),
        mark: AfterMark::Unchanged {
            short: short(facts.base.oid),
        },
        subject: sanitize_line(&facts.base.subject),
        note: Some(sanitize_line("unchanged")),
    });
    AfterView::Ready {
        rows,
        facts: after_facts(plan, &owner),
    }
}

/// An AFTER row's notes: `follows` (`built on yours · follows` for a
/// dependent) on a following branch's first row, the group it stands for,
/// and where the forecast says git may stop.
pub(super) fn ghost_notes(
    facts: &ViewFacts<'_>,
    replayed: Option<&RowFacts<'_>>,
    follows: Option<&str>,
    stop: Option<(usize, &[std::path::PathBuf])>,
    at: usize,
) -> Vec<String> {
    let mut notes = Vec::new();
    notes.extend(follows.map(str::to_owned));
    if let Some(row) = replayed {
        notes.extend(group_note(facts, row));
    }
    match stop {
        Some((stops, paths)) if stops == at => {
            notes.push(format!("! may stop here \u{b7} {}", paths_words(paths)));
        }
        Some((stops, _)) if at > stops => notes.push("not checked past here".to_owned()),
        _ => {}
    }
    notes
}

/// Which branch (index into `ref_moves`) each ghost is on: from each moved
/// ref's new tip down its first parents, until a real commit or a ghost
/// another branch already holds (his branch's ghosts are claimed first, so
/// a dependent holds only its own picks).
pub(super) fn ghost_owners(plan: &PreviewPlan) -> Vec<(usize, usize)> {
    let mut owner: Vec<Option<usize>> = vec![None; plan.ghosts.len()];
    for (branch, moved) in plan.ref_moves.iter().enumerate() {
        let mut at = match moved.to {
            Some(Target::Ghost(at)) => Some(at),
            _ => None,
        };
        while let Some(index) = at {
            match owner.get(index) {
                Some(None) => owner[index] = Some(branch),
                _ => break,
            }
            at = match plan.ghosts[index].parents.first() {
                Some(Target::Ghost(below)) => Some(*below),
                _ => None,
            };
        }
    }
    owner
        .into_iter()
        .enumerate()
        .filter_map(|(at, by)| by.map(|by| (at, by)))
        .collect()
}

/// `3 in one` on a kept row with a squash melding in, `fixup melded in`
/// on one with fixups alone.
pub(super) fn group_note(facts: &ViewFacts<'_>, row: &RowFacts<'_>) -> Option<String> {
    if row.keeps < 2 {
        return None;
    }
    // The melded rows are the ones right above the kept row on screen.
    let at = facts.rows.iter().position(|it| it.commit == row.commit)?;
    let melded = facts.rows.get(at.saturating_sub(row.keeps - 1)..at)?;
    if melded.iter().all(|it| it.action == TodoAction::Fixup) {
        Some("fixup melded in".to_owned())
    } else {
        Some(format!("{} in one", row.keeps))
    }
}

/// The facts under the AFTER rows: who follows, what is on origin, the
/// conflict forecast.
pub(super) fn after_facts(plan: &PreviewPlan, owner: &[(usize, usize)]) -> Vec<SanitizedText> {
    let mut out = Vec::new();
    let followers: Vec<String> = plan
        .ref_moves
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, moved)| !is_follower_fork(plan, &moved.name))
        .map(|(branch, moved)| {
            let mine: Vec<usize> = owner
                .iter()
                .filter(|(_, by)| *by == branch)
                .map(|(at, _)| *at)
                .collect();
            let count = mine.len();
            let who = mine
                .last()
                .and_then(|&at| plan.ghosts.get(at))
                .map(|ghost| String::from_utf8_lossy(&ghost.author.name).into_owned());
            let commits = if count == 1 { "commit" } else { "commits" };
            match who {
                Some(who) => format!("{} ({who}, {count} {commits})", short_ref(&moved.name)),
                None => format!("{} ({count} {commits})", short_ref(&moved.name)),
            }
        })
        .collect();
    if !followers.is_empty() {
        let verb = if followers.len() == 1 {
            "follows"
        } else {
            "follow"
        };
        out.push(format!("built on yours: {} {verb}", followers.join(", ")));
    }
    out.extend(fork_facts(plan));
    if !plan.published.is_empty() {
        let names: Vec<String> = plan
            .published
            .iter()
            .map(|name| String::from_utf8_lossy(name.shorthand()).into_owned())
            .collect();
        out.push(format!(
            "on origin: {} \u{b7} P will need force after",
            names.join(", ")
        ));
    }
    out.push(match &plan.conflicts {
        ConflictForecast::Clean => "no conflicts expected".to_owned(),
        ConflictForecast::Likely { paths } => {
            format!("! may conflict \u{b7} {}", paths_words(paths))
        }
        ConflictForecast::NotChecked(why) => format!("conflicts: not checked ({why})"),
        ConflictForecast::StopsAt { step, of, .. } => {
            format!("! may stop at step {step} of {of} \u{b7} not checked past it")
        }
    });
    out.into_iter().map(|line| sanitize_line(&line)).collect()
}

/// Up to three paths, then `+N more`.
pub(super) fn paths_words(paths: &[std::path::PathBuf]) -> String {
    let mut shown: Vec<String> = paths
        .iter()
        .take(3)
        .map(|path| path.display().to_string())
        .collect();
    if paths.len() > 3 {
        shown.push(format!("+{} more", paths.len() - 3));
    }
    shown.join(", ")
}

/// Each fork's fate (ADR-0059, m8/14c) in the preview box's words, one
/// that would conflict a warning (`!`), any other stays row not; then who
/// else does not follow (m7/37). The longest wording: this side has no
/// width here, and the names lead the row, so the widget's cut at the tail
/// never reaches them.
fn fork_facts(plan: &PreviewPlan) -> Vec<String> {
    let mut out = Vec::new();
    for (fork, row) in plan
        .forks
        .iter()
        .map(Some)
        .chain(std::iter::once(None))
        .zip(crate::update::preview::fork_rows(plan))
    {
        let conflicts = fork.is_some_and(|fork| {
            matches!(fork.fate, ForkFate::Stays(WhyStays::WouldConflict { .. }))
        });
        out.push(if conflicts { format!("! {row}") } else { row });
    }
    let (left, more) = crate::update::preview::not_forks(plan);
    out.extend(
        crate::update::preview::left_behind_rungs(&left, more)
            .into_iter()
            .next(),
    );
    out
}

/// The note on a following branch's first AFTER row: a dependent is built
/// on his; a fork that follows is not (ADR-0059), and says only that it
/// follows.
fn follows_note(plan: &PreviewPlan, name: &str) -> &'static str {
    if is_follower_fork(plan, name) {
        "follows"
    } else {
        "built on yours \u{b7} follows"
    }
}

/// Whether `name` (in full) is a fork that follows (ADR-0059): it moves
/// with the line but is not built on his.
pub(super) fn is_follower_fork(plan: &PreviewPlan, name: &str) -> bool {
    plan.forks.iter().any(|fork| {
        fork.branch.as_bytes() == name.as_bytes() && matches!(fork.fate, ForkFate::Follows { .. })
    })
}

/// `refs/heads/feature-b` → `feature-b`.
pub(super) fn short_ref(name: &str) -> String {
    name.strip_prefix("refs/heads/").unwrap_or(name).to_owned()
}
