//! `Ctrl+A` stage-all and the `d` discard confirm for the selected path.

use super::{
    AppState, Command, ConfirmThen, InFlight, JobId, Operation, Path, PathBuf, PendingConfirm,
    Status, StatusEntry, Target, WorkingTreeSection, conflict_refusal, in_flight, launch,
    operation_in_flight, sanitize_line, say_if_refused, selected_path, status_entry,
};

/// Whether `Ctrl+A` stages this entry.
///
/// Not every dirty path: `git add` on a path whose only change is a **staged
/// deletion** — or a staged rename's vanished old name, which
/// `graft_ops::operation::stage::pathspec` adds alongside the new one — is
/// `fatal: pathspec '<p>' did not match any files`, exit 128, nothing staged
/// at all. One such entry would make `Ctrl+A` fail outright rather than stage
/// the rest (m3/04 review M2, verified against git 2.54).
///
/// An unmerged path is skipped for a different reason: `git add` on a
/// conflicted file marks it resolved with the conflict markers still inside.
/// Resolving a conflict is M6's, and it is not something `Ctrl+A` should do
/// by accident.
pub(super) fn staged_by_stage_all(entry: &StatusEntry) -> bool {
    !entry.unmerged && (entry.unstaged.is_some() || entry.untracked)
}

/// Why [`stage_all`] staged nothing. Both callers (`Ctrl+A`, `space` at an
/// edit stop) say it (30n m6): a key that does nothing says why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NotStaged {
    /// Nothing unstaged or untracked: a clean tree, or every change staged.
    NothingToStage,
    /// Only conflicted paths are left, which `Ctrl+A` never stages.
    OnlyConflicted,
    /// The working tree's status is not read yet.
    Unread,
    /// A write is running.
    WriteRunning(InFlight),
}

impl NotStaged {
    /// The status line: `nothing to stage` as a refusal; a running write in
    /// `f`'s words (`Status::Failed(InFlight::words())`); a conflict in the
    /// changes pane's own words.
    pub(crate) fn status(&self) -> Status {
        match self {
            Self::NothingToStage => Status::sticky("nothing to stage"),
            Self::OnlyConflicted => Status::sticky(conflict_refusal("stage all")),
            Self::Unread => Status::Waiting(
                "still reading the changes \u{2014} try again in a moment".to_owned(),
            ),
            Self::WriteRunning(busy) => Status::Failed(busy.words()),
        }
    }
}

/// `Ctrl+A`: stage every unstaged and untracked path
/// `AppState::working_tree` lists. `Err` says why nothing was staged; the
/// caller puts [`NotStaged::status`] on the status line.
pub(crate) fn stage_all(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> Result<(), NotStaged> {
    if let Some(busy) = in_flight(next) {
        return Err(NotStaged::WriteRunning(busy));
    }
    let Some(status) = next.working_tree.as_ref() else {
        return Err(NotStaged::Unread);
    };
    let paths: Vec<PathBuf> = status
        .entries
        .iter()
        .filter(|entry| staged_by_stage_all(entry))
        .map(|entry| entry.path.clone())
        .collect();
    if paths.is_empty() {
        return Err(if status.entries.iter().any(|entry| entry.unmerged) {
            NotStaged::OnlyConflicted
        } else {
            NotStaged::NothingToStage
        });
    }
    // Unreachable `Err`: `operation_in_flight` returned above.
    let launched = launch(
        next,
        commands,
        Operation::Stage(Target::Paths(paths)),
        next_id,
    );
    say_if_refused(next, launched);
    Ok(())
}

/// `d`: opens the confirm box for discarding the file under the cursor —
/// the consequence in plain words, the exact command, `Cancel` under the
/// cursor. `false` under the same conditions as [`stage_selected`], and the
/// key is free to mean something else.
pub fn request_discard(next: &mut AppState) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    // A conflicted path has no "before" to discard to. Its entry is both
    // staged and unstaged, so the box below would offer a reset to HEAD and
    // throw away the resolution in progress; `M`/`e`/`space mark resolved` are
    // the conflict keys (design note). Refused in words, like `space` and
    // hunk-`d` above — never silently.
    if next.detail.selected_section() == Some(WorkingTreeSection::Unmerged) {
        next.status = Status::sticky(conflict_refusal("discard"));
        return true;
    }
    let Some(entry) = status_entry(next, &path) else {
        return false;
    };
    let command = discard_command(&path, entry);
    let counts = selected_counts(next);
    let entry = entry.clone();
    let shown = path.display();
    // The question and the button must name the same act. An untracked
    // file is deleted, not discarded, and a box reading "Delete notes.md?"
    // over a button saying `Discard` was the one box whose whole job is to
    // be unambiguous before destroying something (m3/04 review round,
    // closed here).
    let (question, verb) = if entry.untracked {
        (format!("Delete {shown}?"), "Delete")
    } else {
        (format!("Discard changes to {shown}?"), "Discard")
    };
    let mut consequence = discard_consequence(&entry, counts);
    consequence.push("Tributary keeps a backup first, so `u` can undo it afterwards.".to_string());
    let view = graft_ui::confirm::ConfirmView::new(
        sanitize_line(&question),
        consequence.iter().map(|line| sanitize_line(line)).collect(),
        sanitize_line(&command),
    )
    .with_labels("Cancel", verb);
    next.confirm = Some(PendingConfirm {
        view,
        then: ConfirmThen::operate(Operation::Discard(Target::Paths(vec![path]))),
    });
    true
}

/// The line counts of the file under the files cursor, as the pane's own file
/// list reports them: `(added, removed)`, or `None` for a binary file or
/// before the list has arrived.
pub(super) fn selected_counts(state: &AppState) -> Option<(u32, u32)> {
    let list = state.detail.list.as_ref()?;
    let entry = state.detail.selected_entry()?;
    let file = list.files.get(usize::try_from(entry).ok()?)?;
    Some((file.added?, file.removed?))
}

/// What a whole-file discard actually does to this path, in plain words.
///
/// Three classes, three different consequences, and the one boilerplate line
/// this used to carry was wrong for two of them (m3/04 review M6):
///
/// - **untracked** - `git clean -f` *deletes* the file. A box that said
///   "throws away the change" never used the word the user needed.
/// - **staged and unstaged** - `plan_discard` runs
///   `restore --staged --worktree --source=HEAD`, so *both* changes go, not
///   just the one the UNSTAGED row is showing. "and the index, if staged"
///   hedged about something we already know.
/// - **staged only / unstaged only** - the one change goes, and the box says
///   which side it comes back from.
///
/// PROMPT.md section 3 asks for the consequence named. The exact command is
/// the line below this one, but this is the line that gets read.
/// ` — 1 line` / ` — 7 lines`: "1 lines" reads as a bug in the counter.
pub(super) fn untracked_size(added: u32) -> String {
    if added == 1 {
        " \u{2014} 1 line".to_owned()
    } else {
        format!(" \u{2014} {added} lines")
    }
}

pub(super) fn discard_consequence(entry: &StatusEntry, counts: Option<(u32, u32)>) -> Vec<String> {
    let lines = match counts {
        Some((added, removed)) => format!(" (+{added} \u{2212}{removed})"),
        None => String::new(),
    };
    if entry.untracked {
        // An untracked file has no diff against the index, so the list often
        // reports no lines at all. "0 lines" would read as a fact; silence is
        // the truth.
        let size = match counts {
            Some((added, _)) if added > 0 => untracked_size(added),
            Some(_) | None => String::new(),
        };
        return vec![format!("Deletes the file from the working tree{size}.")];
    }
    if entry.is_staged() && entry.is_unstaged() {
        return vec![format!(
            "Both the staged and the unstaged change go back to HEAD{lines}."
        )];
    }
    if entry.is_staged() {
        vec![format!(
            "The staged change goes back to how HEAD has it{lines}."
        )]
    } else {
        vec![format!(
            "The change in the working tree goes back to how the index has it{lines}."
        )]
    }
}

/// The command the confirm box shows: what `graft_ops::operation::stage`
/// actually runs for a whole-file discard of this path's class
/// (`crates/graft-ops/src/operation/stage.rs::plan_discard`).
pub(super) fn discard_command(path: &Path, entry: &StatusEntry) -> String {
    let shown = path.display();
    if entry.untracked {
        format!("git clean -f -- {shown}")
    } else if entry.is_staged() {
        format!("git restore --staged --worktree --source=HEAD -- {shown}")
    } else {
        format!("git restore --worktree -- {shown}")
    }
}
