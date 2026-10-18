//! The preview boxes that are not a plan's lines: closing, ready, stash and put-back.

use super::{
    Again, AppState, Operation, PreviewBox, PreviewPlan, PreviewState, PreviewSubject,
    RecoverParams, WorktreeEffect, capped_list, conflict_line, nothing_to_do, plan_lines,
    sanitize_line, undo_line, worktree_line,
};

/// A put-in's own words over the rewrite's box (m7-power-tools/09, note
/// R2): the title names what goes where, the rows above the plan's say
/// what each commit gains, and the footer names the confirm by its verb
/// (`y put it in`), as a stash put-back's does. Only for the operation the
/// put-in planned — an `R`-editor rewrite keeps the rewrite's words.
pub(super) fn put_in_words(
    mut view: PreviewBox,
    state: &AppState,
    preview: &PreviewState,
) -> PreviewBox {
    let Some(shown) = preview
        .subject
        .operation()
        .and_then(|op| state.amend_into.shown_for(op))
    else {
        return view;
    };
    view.title = sanitize_line(&format!("Preview: {}", shown.title));
    if preview.planned().is_some() && preview.undrawable.is_none() {
        let mut lines: Vec<_> = shown.rows.iter().map(|row| sanitize_line(row)).collect();
        lines.append(&mut view.lines);
        view.lines = lines;
    }
    if preview.acts() || preview.planning() {
        view.confirm_key = false;
        view.again = Some(Again::confirm(shown.confirm));
    }
    view
}

/// A box with nothing to confirm: one line saying why, and `Close`.
pub(super) fn closing_box(line: &str, subject: &PreviewSubject) -> PreviewBox {
    let mut view = PreviewBox::new(sanitize_line(&planning_title(subject)))
        .with_choices(vec![sanitize_line("Close")]);
    view.lines = vec![sanitize_line(line)];
    // The footer (`esc back`) follows from the one choice, in mergewright-ui.
    // The flag is vestigial: kept only for a test outside this wave's
    // files (m7-power-tools/36 NOTES), removed with it.
    view.back_only = stash_put_back(subject).is_some();
    view
}

/// The box for a plan that landed, for a frame `columns` wide. `op` is the
/// operation it was planned for, when it is one: a rebase or rewrite draws
/// its line (m7/05). `keeps`: `R` is live, and the already-in row says so.
pub(super) fn ready_box(
    plan: &PreviewPlan,
    op: Option<&Operation>,
    keeps: bool,
    clipped: usize,
    columns: u16,
    keys: &mergewright_ui::keymap::Keymap,
) -> PreviewBox {
    let title = sanitize_line(&format!("Preview: {}", plan.description));
    let mut view = if nothing_to_do(plan) {
        PreviewBox::new(title).with_choices(vec![sanitize_line("Close")])
    } else {
        PreviewBox::new(title)
    };
    view.lines = plan_lines(plan, op, keeps, clipped, columns, keys);
    view.commands = plan
        .commands
        .iter()
        .map(|line| sanitize_line(line))
        .collect();
    view
}

/// Which stash a put-back preview is about, and whether it pops: `None` for
/// every other subject.
pub(super) fn stash_put_back(subject: &PreviewSubject) -> Option<(usize, bool)> {
    stash_put_back_op(subject.operation()?)
}

/// [`stash_put_back`] for an operation on its own.
pub(super) fn stash_put_back_op(op: &Operation) -> Option<(usize, bool)> {
    match op {
        Operation::StashApply { index, .. } => Some((*index, false)),
        Operation::StashPop { index, .. } => Some((*index, true)),
        _ => None,
    }
}

/// The verb on a stash put-back's confirm button and footer.
pub(super) fn stash_verb(pop: bool) -> &'static str {
    if pop { "pop" } else { "apply" }
}

/// The box for a stash put-back (m6-conflicts/86): the files it writes,
/// listed as the conflict line lists its paths; the forecast in the
/// preview's own words; one row for the stash's fate, kept whole; and
/// `Cancel` / `Apply` (or `Pop`).
pub(super) fn stash_box(
    plan: &PreviewPlan,
    index: usize,
    pop: bool,
    keys: &mergewright_ui::keymap::Keymap,
) -> PreviewBox {
    let name = format!("stash@{{{index}}}");
    let title = sanitize_line(&format!("Preview: {}", plan.description));
    let label = if pop { "Pop" } else { "Apply" };
    let mut view =
        PreviewBox::new(title).with_choices(vec![sanitize_line("Cancel"), sanitize_line(label)]);
    let files = match &plan.worktree {
        WorktreeEffect::Updated { paths, more } => {
            format!("working tree: {}", capped_list(paths, *more))
        }
        // Only the index changes (review 86, m1): its paths, never an empty
        // `working tree:` line.
        WorktreeEffect::IndexOnly { paths, more } => {
            format!("index only: {}", capped_list(paths, *more))
        }
        other => worktree_line(other),
    };
    let mut lines = vec![files, conflict_line(&plan.conflicts, None)];
    if pop {
        lines.push(popped_line(&name, keys));
    } else {
        lines.push(format!("{name} stays in the list"));
        lines.push(undo_line(&plan.undo));
    }
    view.lines = lines.iter().map(|line| sanitize_line(line)).collect();
    view.commands = plan
        .commands
        .iter()
        .map(|line| sanitize_line(line))
        .collect();
    view
}

/// The box for a put-back from J (m7-power-tools/24-a2, J1): `now` /
/// `after y` / `your files:` / `u undoes it` (`recover::preview_lines`),
/// the plan's own git lines, and `Cancel` / `Put back` — the buttons every
/// other preview has, cursor on `Cancel` (ADR-0038 §1). The title is set
/// in [`preview_box`] for every state of the box.
pub(super) fn put_back_box(
    state: &AppState,
    plan: &PreviewPlan,
    params: &RecoverParams,
) -> PreviewBox {
    let subject_of = |oid| crate::update::recover::subject_in(state, oid);
    let title = sanitize_line(&crate::update::recover::preview_title(params, subject_of));
    let mut view = PreviewBox::new(title)
        .with_choices(vec![sanitize_line("Cancel"), sanitize_line("Put back")]);
    view.lines = crate::update::recover::preview_lines(
        params,
        plan.files.as_ref(),
        subject_of,
        &state.keymap,
    )
    .iter()
    .map(|line| sanitize_line(line))
    .collect();
    view.commands = plan
        .commands
        .iter()
        .map(|line| sanitize_line(line))
        .collect();
    view
}

/// The box's title before the plan lands, where `plan.description` is not
/// known yet: the subject's own words, which is all the app has.
pub(super) fn planning_title(subject: &PreviewSubject) -> String {
    match subject {
        PreviewSubject::Operation { op, .. } => format!("Preview: {}", op.describe()),
        PreviewSubject::RevertTo(_) => "Preview: revert to before this".to_owned(),
    }
}

/// The pop box's `stash@{n} is dropped · u brings it back`, the clause
/// spelled by [`crate::update::clash::brings_back`]; dropped when unbound.
pub(crate) fn popped_line(name: &str, keys: &mergewright_ui::keymap::Keymap) -> String {
    let dropped = format!("{name} is dropped");
    match crate::update::clash::brings_back(keys) {
        Some(clause) => format!("{dropped} \u{b7} {clause}"),
        None => dropped,
    }
}
