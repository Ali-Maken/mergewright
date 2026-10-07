//! A plan's lines in the preview box: refs, `HEAD`, working tree, conflicts, undo.

use super::{
    ConflictForecast, GhostCommit, HeadMove, Oid, Operation, PreviewBox, PreviewPlan, RefMove,
    SanitizedText, Target, UndoNote, WorktreeEffect, line_rows, nothing_to_do, sanitize_line,
    slice,
};

/// The wording table: ghosts, refs, `HEAD`, the working tree and the edits
/// set aside around it, the forecast, the undo note, and the slice's own
/// edge. `columns` picks the set-aside row's wording; every other row is
/// one wording, cut by the box.
///
/// A rebase or rewrite moving a line of branches draws them under a
/// `branches:` label, the squash-merge suggestion under them, `on origin:`
/// above the forecast, and a stop forecast by its own cut ladder
/// (`line_rows`, m7/05). A plain one-branch rebase draws as before.
pub(super) fn plan_lines(
    plan: &PreviewPlan,
    op: Option<&Operation>,
    keeps: bool,
    clipped: usize,
    columns: u16,
    keys: &graft_ui::keymap::Keymap,
) -> Vec<SanitizedText> {
    let mut lines: Vec<String> = Vec::new();
    lines.extend(ghost_line(&plan.ghosts));
    let line = line_rows::is_line(plan, op);
    if line {
        lines.extend(line_rows::ref_rows(plan));
    } else {
        lines.extend(plan.ref_moves.iter().map(|moved| ref_line(plan, moved)));
    }
    // Each fork's fate (ADR-0059 §2): it follows, or stays and why; for a
    // rewrite, a branch parked at the old tip stays too.
    let rewrite = matches!(op, Some(Operation::Rewrite(_)));
    lines.extend(line_rows::fork_lines(plan, line, rewrite));
    // Who `r` does not take along (m7/37): every branch it leaves on a
    // commit it replaces (its forks never follow yet).
    lines.extend(line_rows::left_behind_row(plan, rewrite, columns));
    lines.extend(line_rows::already_in_row(plan, op, keeps, keys));
    lines.extend(head_line(plan));
    lines.push(worktree_line(&plan.worktree));
    // The edits a rebase or checkout sets aside and puts back after
    // (ADR-0045 §4, m6-conflicts/74): under the tree line it qualifies.
    // No staging words: every put-back keeps the staging (ADR-0045
    // amendment b), and `SetAside` carries only the file count (30c).
    // A put-in's staged edits go into commits: with nothing unstaged there
    // is nothing of his to put back, and no row (review Minor 6).
    if let Some(aside) = plan.set_aside.filter(|aside| aside.files > 0) {
        lines.push(graft_ui::preview_box::fitting_line(
            &graft_ui::preview_box::set_aside_row(aside.files),
            columns,
        ));
    }
    lines.extend(line_rows::published_row(plan, keys));
    // A stop forecast cuts its paths, then its subject, then `· not
    // checked past it`, never the step (note R3).
    lines.push(match line_rows::forecast_rungs(plan) {
        Some(wordings) => {
            let wordings: Vec<String> = wordings
                .into_iter()
                .map(|wording| with_put_back(wording, plan.put_back.as_ref()))
                .collect();
            graft_ui::preview_box::fitting_line(&wordings, columns)
        }
        None => conflict_line(&plan.conflicts, plan.put_back.as_ref()),
    });
    // A plan that changes nothing has nothing to take back, and offering
    // `u` over it teaches the box says things it does not mean
    // (review 08 Minor 7).
    if !nothing_to_do(plan) {
        lines.push(undo_line(&plan.undo));
    }
    // The slice's edge, named rather than left for the eye to misread: a
    // lane that simply stops is the accepted cost of a bounded picture
    // (hard choice 1), but the user is told it is happening.
    if clipped > 0 {
        lines.push(format!(
            "{clipped} {} continue outside this view",
            if clipped == 1 { "lane" } else { "lanes" }
        ));
    }
    if nothing_to_do(plan) {
        lines.push("nothing to do".to_owned());
    }
    lines.iter().map(|line| sanitize_line(line)).collect()
}

/// `2 commits replayed`, `1 merge commit`, `1 new commit` — one line per
/// kind of ghost the plan draws, in that order. Nothing when there are no
/// ghosts.
pub(super) fn ghost_line(ghosts: &[GhostCommit]) -> Option<String> {
    if ghosts.is_empty() {
        return None;
    }
    let replayed = ghosts.iter().filter(|it| it.replays.is_some()).count();
    let merges = ghosts
        .iter()
        .filter(|it| it.replays.is_none() && it.parents.len() > 1)
        .count();
    let fresh = ghosts.len() - replayed - merges;
    let mut parts = Vec::new();
    if replayed > 0 {
        parts.push(format!(
            "{replayed} {} replayed",
            if replayed == 1 { "commit" } else { "commits" }
        ));
    }
    if merges > 0 {
        parts.push(format!(
            "{merges} merge {}",
            if merges == 1 { "commit" } else { "commits" }
        ));
    }
    if fresh > 0 {
        parts.push(format!(
            "{fresh} new {}",
            if fresh == 1 { "commit" } else { "commits" }
        ));
    }
    Some(parts.join(" · "))
}

/// One ref move: `topic e7e39a7 → 5 new commits`, `old a1b2c3d → deleted`,
/// `topic (new) → 3c2b1a0`.
///
/// A ref landing on a predicted commit says **how many new commits** it
/// would have, not `ghost`. `ghost` is our word: it appears nowhere else on
/// screen, and the user is left to connect `○` and `—` to a noun the
/// interface never defines (m5/20 item 4).
pub(super) fn ref_line(plan: &PreviewPlan, moved: &RefMove) -> String {
    let name = String::from_utf8_lossy(&slice::short_of(&moved.name)).into_owned();
    let from = moved.from.map_or_else(|| "(new)".to_owned(), short);
    let to = match moved.to {
        None => "deleted".to_owned(),
        Some(Target::Real(oid)) => short(oid),
        Some(Target::Ghost(at)) => new_commits(new_commits_at(plan, at)),
    };
    format!("{name} {from} \u{2192} {to}")
}

/// `5 new commits`, singular at one.
pub(super) fn new_commits(count: usize) -> String {
    if count == 1 {
        "1 new commit".to_owned()
    } else {
        format!("{count} new commits")
    }
}

/// How many predicted commits the ref would gain: the ghost it lands on and
/// every ghost below it in the same chain.
///
/// Walked through `parents`, not counted as `at + 1`: the plan's ghost list
/// is one flat vector, and a plan with two chains in it would otherwise
/// count the other chain's commits into this ref's total.
pub(super) fn new_commits_at(plan: &PreviewPlan, at: usize) -> usize {
    let mut count = 0;
    let mut next = Some(at);
    while let Some(index) = next {
        let Some(ghost) = plan.ghosts.get(index) else {
            break;
        };
        count += 1;
        next = ghost.parents.iter().find_map(|parent| match parent {
            Target::Ghost(parent) => Some(*parent),
            Target::Real(_) => None,
        });
    }
    count
}

/// Where `HEAD` goes, when the plan moves it.
pub(super) fn head_line(plan: &PreviewPlan) -> Option<String> {
    match plan.head.as_ref()? {
        HeadMove::ToBranch(name) => Some(format!(
            "HEAD \u{2192} {}",
            String::from_utf8_lossy(&slice::short_of(name))
        )),
        HeadMove::Detached(Target::Real(oid)) => Some(format!("HEAD detaches at {}", short(*oid))),
        HeadMove::Detached(Target::Ghost(at)) => Some(format!(
            "HEAD detaches at the newest of {}",
            new_commits(new_commits_at(plan, *at))
        )),
    }
}

/// What happens to the files on disk. `LOSES` is the word the user must
/// read first; the backup that makes it recoverable comes after it.
pub(super) fn worktree_line(effect: &WorktreeEffect) -> String {
    match effect {
        WorktreeEffect::Untouched => "working tree: untouched".to_owned(),
        // `files_label`, not `{n} files`: "1 files updated" reads as a bug
        // in the counter, and every other count in this box already agrees
        // with its noun.
        WorktreeEffect::Updated { paths, more } => format!(
            "working tree: {} updated{}",
            graft_ui::stash_screen::files_label(paths.len()),
            if *more { ", more" } else { "" }
        ),
        // Only the stash preview draws this, through `stash_box`'s own line;
        // here for any other box that one day carries it.
        WorktreeEffect::IndexOnly { paths, more } => format!(
            "index only: {} staged{}",
            graft_ui::stash_screen::files_label(paths.len()),
            if *more { ", more" } else { "" }
        ),
        WorktreeEffect::Lost { paths } => {
            format!("working tree: LOSES {} (backup saved)", list(paths, 2))
        }
        WorktreeEffect::RestoredFromBackup => {
            "index and working tree restored from backup".to_owned()
        }
        // A new branch switched to where `HEAD` is: nothing set aside,
        // nothing rewritten (10b).
        WorktreeEffect::EditsStay => "your edits stay as they are".to_owned(),
        // Not `Untouched` and not an empty `Updated`: both read as "nothing
        // happens to your files", which is the one thing this does not mean.
        WorktreeEffect::NotShown { why } => format!("working tree: changes not listed ({why})"),
    }
}

/// The forecast, in the only terms `merge-tree` can support: `may`, never
/// `will`, and a hole named as a hole.
///
/// No `(forecast)` on the end of either line. `02-keymap.md` §2.1 (extended
/// 2026-09-21): a trailing parenthesis carries a **key**, not a word,
/// anywhere the user might press what is inside it — and `forecast` is our
/// word, not theirs (CONTEXT.md, "These are our words"). `expected` and
/// `may` already say the line is a prediction, which is all the
/// parenthetical was adding.
///
/// With edits set aside the line folds in the put-back's own forecast
/// (`PreviewPlan::put_back`, review m6-final-fixes-74 M2; the note's
/// "Preview: the conflict line folds the put-back" table): a clean operation
/// says what putting the edits back may do; an operation that may stop
/// itself was never followed to the put-back, and says so after its own
/// words, where a cut at the tail takes it first.
pub(super) fn conflict_line(
    forecast: &ConflictForecast,
    put_back: Option<&ConflictForecast>,
) -> String {
    match (forecast, put_back) {
        (ConflictForecast::Clean, None | Some(ConflictForecast::Clean)) => {
            "no conflicts expected".to_owned()
        }
        // A put-back is one merge, never a chain: it is never `StopsAt`,
        // and would read as its paths if it were.
        (
            ConflictForecast::Clean,
            Some(ConflictForecast::Likely { paths } | ConflictForecast::StopsAt { paths, .. }),
        ) => {
            format!("your edits may clash when put back: {}", list(paths, 3))
        }
        (ConflictForecast::Clean, Some(ConflictForecast::NotChecked(why))) => {
            format!("your edits' put-back: not checked ({why})")
        }
        (ConflictForecast::Likely { paths }, put_back) => {
            with_put_back(format!("may conflict: {}", list(paths, 3)), put_back)
        }
        (ConflictForecast::NotChecked(why), put_back) => {
            with_put_back(format!("conflicts: not checked ({why})"), put_back)
        }
        // `plan_lines` draws a stop by `line_rows::forecast_rungs`, which
        // knows the ghost's subject; this is the bare wording for any other
        // caller.
        (
            ConflictForecast::StopsAt {
                step,
                of,
                commit,
                paths,
            },
            put_back,
        ) => with_put_back(
            format!(
                "may stop at step {step} of {of} ({}): {} · not checked past it",
                short(*commit),
                list(paths, 3)
            ),
            put_back,
        ),
    }
}

/// The operation's own forecast line, then ` · put-back not checked` when
/// edits are set aside: an operation that may stop is not followed further.
pub(super) fn with_put_back(line: String, put_back: Option<&ConflictForecast>) -> String {
    match put_back {
        Some(_) => format!("{line} · put-back not checked"),
        None => line,
    }
}

/// What `u` would take back afterwards.
pub(super) fn undo_line(note: &UndoNote) -> String {
    match note {
        UndoNote::Undoable => "undo: u takes this back".to_owned(),
        UndoNote::NotUndoable(why) => format!("not undoable: {why}"),
    }
}

/// The first `cap` paths, with `(+n)` for the rest.
pub(super) fn list(paths: &[std::path::PathBuf], cap: usize) -> String {
    let shown: Vec<String> = paths
        .iter()
        .take(cap)
        .map(|path| path.display().to_string())
        .collect();
    let rest = paths.len().saturating_sub(cap);
    if rest == 0 {
        shown.join(", ")
    } else {
        format!("{} (+{rest})", shown.join(", "))
    }
}

/// Three of `paths`, then the rest: `(+n)` when the list is whole, `and
/// more` when it was cut at `PATHS_CAP` — the true count is not known then,
/// so a number would be wrong (review 86, m2).
pub(super) fn capped_list(paths: &[std::path::PathBuf], more: bool) -> String {
    if more {
        format!("{} and more", list(&paths[..paths.len().min(3)], 3))
    } else {
        list(paths, 3)
    }
}

/// A commit id's first seven hex digits, the width the graph's SHA column
/// uses.
pub(super) fn short(oid: Oid) -> String {
    let hex = oid.to_string();
    hex[..7.min(hex.len())].to_owned()
}

/// The box with its cursor put back where the state says it is.
///
/// [`PreviewBox`]'s cursor is private and starts on the safe choice by
/// construction (ADR-0038 §1), so it is walked rather than assigned: there
/// is deliberately no way to build a box whose acting choice is where the
/// cursor rests.
pub(super) fn with_cursor(mut view: PreviewBox, cursor: usize) -> PreviewBox {
    for _ in 0..cursor {
        view.right();
    }
    view
}

// --- geometry -----------------------------------------------------------
