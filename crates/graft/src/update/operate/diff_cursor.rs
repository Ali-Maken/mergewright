//! Writes at the diff cursor: stage, unstage or discard the hunk or lines under it.

use super::{
    AppState, BodyRow, Command, ConfirmThen, DetailFocus, DetailSpec, JobId, LineKindView, Loading,
    Operation, PatchDirection, PatchError, PatchSide, PathBuf, PendingConfirm, Selection, Status,
    Target, WorkingTreeSection, launch, operation_in_flight, request_discard, sanitize_line,
    say_if_refused, selected_path, stage_selected,
};

/// What the diff region's line cursor names: a hunk of the file on show, and
/// how much of it.
pub(super) struct DiffUnit {
    /// Index into the raw [`FileDiff`]'s hunks — the same index the drawn
    /// [`graft_ui::detail::DiffView`] uses.
    hunk: usize,
    /// The whole hunk, or the one changed line under the cursor.
    selection: Selection,
    /// `true` when the cursor named one line rather than the hunk; only the
    /// wording of the status line and the confirm box differs.
    one_line: bool,
}

/// The unit under the diff cursor: the whole hunk on a `@@` heading or on a
/// context line, that one line on a `+`/`−` line. `None` off the changes
/// pane, on the truncation footer, or before a diff has arrived.
///
/// The line index it builds is into the **raw** hunk: `assemble_hunk` puts
/// the heading at `HunkView::lines[0]` and copies git's lines after it
/// one for one, so a drawn line `i` is raw line `i - 1`.
pub(super) fn diff_unit(state: &AppState) -> Option<DiffUnit> {
    // Every diff has a line cursor since m7-power-tools/20b; only the
    // changes pane's is a unit of a write.
    if state.detail.spec != Some(DetailSpec::WorkingTree) {
        return None;
    }
    let cursor = state.detail.body_cursor()?;
    let Loading::Ready(view) = &state.detail.diff else {
        return None;
    };
    match view.body_row(cursor)? {
        BodyRow::Heading { hunk } => Some(DiffUnit {
            hunk,
            selection: Selection::WholeHunk,
            one_line: false,
        }),
        BodyRow::Line { hunk, line } => {
            let kind = view.hunks.get(hunk)?.lines.get(line)?.kind;
            let whole = DiffUnit {
                hunk,
                selection: Selection::WholeHunk,
                one_line: false,
            };
            match kind {
                // A context line belongs to its hunk and to nothing smaller:
                // `Space` on one means the hunk around it.
                LineKindView::Context | LineKindView::HunkHeading => Some(whole),
                LineKindView::Added | LineKindView::Removed => Some(DiffUnit {
                    hunk,
                    selection: Selection::Lines(vec![line.checked_sub(1)?]),
                    one_line: true,
                }),
            }
        }
        BodyRow::Truncation => None,
    }
}

/// What `e` opens: the file the changes-pane cursor names, and the line of it
/// under the cursor when the diff region has focus
/// (m3-working-tree-undo-and-stash/05).
///
/// The **new** side's line number, because that is the file on disk — the old
/// side's numbering belongs to a version the editor would not be opening. A
/// removed line has no new number, so the cursor lands at the top of the file
/// rather than at a line that is not there any more.
pub fn edit_target(state: &AppState) -> Option<(PathBuf, Option<u32>)> {
    if !(state.detail.shown && state.detail.spec == Some(DetailSpec::WorkingTree)) {
        return None;
    }
    let path = selected_path(state)?;
    Some((path, cursor_line(state)))
}

/// The new-side line number under the diff cursor, or `None` off the diff
/// region, on a heading, or on a removed line.
pub(super) fn cursor_line(state: &AppState) -> Option<u32> {
    if state.detail.focus != DetailFocus::Diff || state.detail.spec != Some(DetailSpec::WorkingTree)
    {
        return None;
    }
    let cursor = state.detail.body_cursor()?;
    let Loading::Ready(view) = &state.detail.diff else {
        return None;
    };
    match view.body_row(cursor)? {
        BodyRow::Line { hunk, line } => view.hunks.get(hunk)?.lines.get(line)?.new_no,
        BodyRow::Heading { .. } | BodyRow::Truncation => None,
    }
}

/// Which way `graft_ops` will apply a patch cut from `section`, and which
/// diff it was cut from. Staging reads the worktree side forward; unstaging
/// and discarding both reverse what is already there.
///
/// `None` for a conflicted path: a file with conflict markers in it has no
/// clean side to cut a hunk against, so there is no patch to build and the
/// caller says so rather than guessing a side.
pub(super) const fn patch_direction(
    section: WorkingTreeSection,
) -> Option<(PatchDirection, PatchSide)> {
    match section {
        WorkingTreeSection::Staged => Some((PatchDirection::Reverse, PatchSide::Index)),
        WorkingTreeSection::Unstaged | WorkingTreeSection::Untracked => {
            Some((PatchDirection::Forward, PatchSide::Worktree))
        }
        WorkingTreeSection::Unmerged => None,
    }
}

/// Why a key in the changes pane will not act on a conflicted path, in the
/// same shape as [`side_by_side_refusal`]: the key is answered and the
/// status line carries the reason, because a key that silently does nothing
/// reads as broken.
pub(super) fn conflict_refusal(verb: &str) -> String {
    format!("{verb} does not act on a conflicted file \u{2014} resolve the conflict first")
}

/// Why a patch could not be built, in words that say what to do instead.
/// Never a silent no-op: the key is answered and the status line carries the
/// reason (design note, "Changes pane").
pub(super) fn patch_reason(verb: &str, error: &PatchError) -> String {
    match error {
        // The one refusal a user meets in ordinary work: the last line of a
        // file with no trailing newline cannot be moved on its own without
        // stranding git's `\ No newline at end of file` marker.
        PatchError::NoNewlineBoundary { .. } => format!(
            "{verb}: this file has no newline at the end, so that line cannot go on its own — put the cursor on the hunk's @@ line to {verb} the whole hunk"
        ),
        PatchError::Binary => {
            format!("{verb}: a binary file has no hunks — {verb} the whole file from the list")
        }
        PatchError::NotAChangedLine { .. } => {
            format!("{verb}: that line is unchanged; there is nothing on it to {verb}")
        }
        PatchError::NothingSelected => format!("{verb}: nothing under the cursor to {verb}"),
        PatchError::NoSuchHunk { .. } | PatchError::NoSuchLine { .. } => {
            format!("{verb}: the diff on screen is out of date — Ctrl+R re-reads it")
        }
    }
}

/// The patch bytes for the unit under the diff cursor, or the reason there
/// are none, already worded for the status line.
pub(super) fn patch_for(
    state: &AppState,
    unit: &DiffUnit,
    direction: PatchDirection,
    verb: &str,
) -> Result<Vec<u8>, String> {
    let raw = state
        .detail
        .diff_raw
        .as_ref()
        .ok_or_else(|| format!("{verb}: the diff is still loading"))?;
    graft_git::diff::hunk_patch(raw, unit.hunk, &unit.selection, direction)
        .map_err(|error| patch_reason(verb, &error))
}

/// Why a hunk-level write is refused in the side-by-side view, or `None` in
/// the unified view.
///
/// `graft_ui::detail_pane::side_by_side` draws no line cursor — there is no
/// single row to put one on once old and new sit in two columns. Acting on
/// the cursor anyway would write a hunk the user cannot see, which is the
/// same fault as review C1. Refusing in words and naming the way out beats
/// both acting unseen and a key that does nothing (m3/04 review M5).
pub(super) fn side_by_side_refusal(state: &AppState, verb: &str) -> Option<String> {
    if state.detail.mode == graft_ui::detail::DiffMode::SideBySide {
        // Spelled from his keymap (m8/10f); with the toggle unbound the
        // way out has no key to name, so the reason stands alone (W3).
        Some(
            match state
                .keymap
                .spell_keys(&[graft_ui::keymap::ActionId("diff.side_by_side")])
            {
                Some(key) => format!(
                    "side by side has no line cursor \u{2014} press {key} for the unified view to {verb} a hunk"
                ),
                None => "side by side has no line cursor".to_owned(),
            },
        )
    } else {
        None
    }
}

/// `Space`/`s` with the diff region focused: stage or unstage the hunk or
/// line under the line cursor. Direction comes from the file's section, the
/// same rule the file list follows.
///
/// Untracked files stage whole (ADR-0034): git has no index side to cut a
/// patch against, so `Space` there means the file.
///
/// `false` only when there is nothing to act on at all; a patch that cannot
/// be built answers the key and says why in the status line, because a key
/// that silently does nothing is worse than a refusal.
pub fn stage_at_diff_cursor(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(section) = next.detail.selected_section() else {
        return false;
    };
    if section == WorkingTreeSection::Untracked {
        return stage_selected(next, commands, next_id);
    }
    if let Some(reason) = side_by_side_refusal(next, "stage") {
        next.status = Status::sticky(reason);
        return true;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    let Some(unit) = diff_unit(next) else {
        return false;
    };
    let verb = if section == WorkingTreeSection::Staged {
        "unstage"
    } else {
        "stage"
    };
    let Some((direction, side)) = patch_direction(section) else {
        next.status = Status::sticky(conflict_refusal(verb));
        return true;
    };
    let patch_bytes = match patch_for(next, &unit, direction, verb) {
        Ok(bytes) => bytes,
        Err(reason) => {
            next.status = Status::sticky(reason);
            return true;
        }
    };
    let target = Target::Patch {
        path,
        patch: patch_bytes,
        side,
    };
    let op = if section == WorkingTreeSection::Staged {
        Operation::Unstage(target)
    } else {
        Operation::Stage(target)
    };
    let launched = launch(next, commands, op, next_id);
    say_if_refused(next, launched);
    true
}

/// The discard box's backup line, the undo key spelled from his keymap
/// (m8/10f); unbound, the backup is still kept and said, with no key
/// promised (10b W3).
fn backup_words(keys: &graft_ui::keymap::Keymap) -> String {
    match keys.spell_keys(&[graft_ui::keymap::ActionId("app.undo")]) {
        Some(undo) => format!("Graft keeps a backup first, so `{undo}` can undo it afterwards."),
        None => "Graft keeps a backup first.".to_owned(),
    }
}

/// `d` with the diff region focused: the confirm box for discarding the hunk
/// or line under the line cursor. An untracked file has no hunks to discard
/// one of, so it falls back to the whole-file box (ADR-0034).
pub fn request_discard_at_diff_cursor(next: &mut AppState) -> bool {
    if operation_in_flight(next) {
        return false;
    }
    let Some(section) = next.detail.selected_section() else {
        return false;
    };
    if section == WorkingTreeSection::Untracked {
        return request_discard(next);
    }
    if let Some(reason) = side_by_side_refusal(next, "discard") {
        next.status = Status::sticky(reason);
        return true;
    }
    let Some(path) = selected_path(next) else {
        return false;
    };
    let Some(unit) = diff_unit(next) else {
        return false;
    };
    // A discard always reverses what is on disk, whichever side it came from.
    let Some((_, side)) = patch_direction(section) else {
        next.status = Status::sticky(conflict_refusal("discard"));
        return true;
    };
    let patch_bytes = match patch_for(next, &unit, PatchDirection::Reverse, "discard") {
        Ok(bytes) => bytes,
        Err(reason) => {
            next.status = Status::sticky(reason);
            return true;
        }
    };
    let counts = patch_counts(&patch_bytes);
    let unit_word = if unit.one_line { "line" } else { "hunk" };
    let shown = path.display();
    let view = graft_ui::confirm::ConfirmView::new(
        sanitize_line(&format!("Discard this {unit_word} of {shown}?")),
        vec![
            sanitize_line(&format!(
                "{} added / {} removed lines go back to how they were.",
                counts.0, counts.1
            )),
            sanitize_line(&backup_words(&next.keymap)),
        ],
        sanitize_line(&discard_patch_command(side)),
    )
    // The question and the button name the same act (m3/04 review M6).
    // This box asks "Discard this hunk of …?"; a button reading `Confirm`
    // leaves the user to remember which act they are confirming.
    .with_labels("Cancel", "Discard");
    next.confirm = Some(PendingConfirm {
        view,
        then: ConfirmThen::operate(Operation::Discard(Target::Patch {
            path,
            patch: patch_bytes,
            side,
        })),
    });
    true
}

/// Added and removed line counts of a built patch, read back off its own
/// body so the confirm box can only ever name what the patch actually
/// carries. The `+++`/`---` header lines are skipped.
pub(super) fn patch_counts(patch: &[u8]) -> (usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    let mut in_hunk = false;
    for line in patch.split(|byte| *byte == b'\n') {
        if line.starts_with(b"@@") {
            in_hunk = true;
            continue;
        }
        if !in_hunk {
            continue;
        }
        match line.first() {
            Some(b'+') => added += 1,
            Some(b'-') => removed += 1,
            _ => {}
        }
    }
    (added, removed)
}

/// The command the confirm box shows for a patch discard: exactly what
/// `graft_ops::operation::stage::plan_discard` runs — the working tree first,
/// then the index when the hunk came from the index.
pub(super) fn discard_patch_command(side: PatchSide) -> String {
    match side {
        PatchSide::Worktree => "git apply -R -".to_string(),
        PatchSide::Index => "git apply -R -  &&  git apply --cached -R -".to_string(),
    }
}
