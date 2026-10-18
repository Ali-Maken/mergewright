//! The merge preview in the app (ease-pass 10): the plan's [`MergeLook`]
//! turned into the box's counts and the incoming list, and the keys that
//! move between them.
//!
//! Every merge preview gets this box, whatever opened it (`m`, drag-drop,
//! the `.` menu, the palette): the branch is by the **op kind**, never
//! `opened_by`. `⏎` opens the list, `esc` comes back; `y` merges from
//! either — except over unrelated histories, which git refuses, so `y`
//! is withheld and the box says why in one line.

use mergewright_git::history_reads::MergeCheck;
use mergewright_ops::operation::{MergeParams, Source};
use mergewright_ops::preview::{ConflictForecast, MergeLook};
use mergewright_ui::graph_screen::columns::relative_date;
use mergewright_ui::incoming_list::{IncomingRow, IncomingView};
use mergewright_ui::preview_box::MergeParts;

use super::{
    AppState, Command, JobId, KeyCode, KeyPress, Operation, PreviewBox, PreviewState,
    confirm_by_key, conflict_line, sanitize_line, short, short_name_of,
};

/// What the merge preview shows: the box, or the commits that come in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Showing {
    /// The box (every preview's only state but a merge's).
    #[default]
    Box,
    /// `⏎` from a merge box: its incoming commits, in the box's place.
    Incoming(IncomingView),
}

/// Who wrote what comes in: at most this many names, then `+M`.
const WHO_SHOWN: usize = 3;

/// The merge this preview is of, when it is one.
fn merge_params(preview: &PreviewState) -> Option<&MergeParams> {
    match preview.subject.operation()? {
        Operation::Merge(params) => Some(params),
        _ => None,
    }
}

/// The landed plan's merge look, when this is a merge that changes
/// something and could be drawn.
pub(super) fn look(preview: &PreviewState) -> Option<&MergeLook> {
    merge_params(preview)?;
    if !preview.acts() {
        return None;
    }
    preview.planned()?.plan.merge.as_ref()
}

/// `y` is withheld: the two sides share no history, and git refuses that
/// merge (`mergewright_ops::operation::history::plan_merge` passes no
/// `--allow-unrelated-histories`).
pub(super) fn withheld(preview: &PreviewState) -> bool {
    look(preview).is_some_and(MergeLook::unrelated)
}

/// `sara/fix-toc` and `wallet-release`: the source and the branch merged
/// into, short.
fn names(params: &MergeParams) -> (String, String) {
    let from = match &params.source {
        Source::Branch(name) => short_name_of(name),
        Source::Commit(oid) => short(*oid),
    };
    (from, short_name_of(&params.into.name))
}

/// A name's first word: `Sara Ali` → `Sara`.
fn first_name(name: &[u8]) -> String {
    let name = String::from_utf8_lossy(name);
    name.split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// The merge box for `preview`, when it is a merge whose plan landed with
/// a look; `None` sends the caller to the ordinary box.
pub(super) fn merge_box(preview: &PreviewState) -> Option<PreviewBox> {
    let look = look(preview)?;
    let params = merge_params(preview)?;
    let (from, into) = names(params);
    let newest_first = look.incoming.commits.iter().rev();
    let mut who: Vec<String> = Vec::new();
    for commit in newest_first {
        let name = first_name(&commit.author.name);
        if !name.is_empty() && !who.contains(&name) {
            who.push(name);
        }
    }
    let who_more = who.len().saturating_sub(WHO_SHOWN);
    who.truncate(WHO_SHOWN);
    let (changed, changed_capped, decide) = match &look.check {
        MergeCheck::Checked(forecast) => (
            forecast.changed.len(),
            forecast.changed_capped,
            forecast
                .clashes
                .iter()
                .map(|clash| (sanitize_line(&clash.path.to_string_lossy()), clash.places))
                .collect(),
        ),
        MergeCheck::NotCheckable(_) => (0, false, Vec::new()),
    };
    let not_checkable = match &look.check {
        MergeCheck::NotCheckable(why) if !look.unrelated() => Some(sanitize_line(&conflict_line(
            &ConflictForecast::NotChecked(why.clone()),
            None,
        ))),
        _ => None,
    };
    let withheld = look
        .unrelated()
        .then(|| sanitize_line(&format!("Can't merge: {from} and {into} share no history.")));
    let title = sanitize_line(&format!("Merge {from} into {into}"));
    Some(PreviewBox::new(title).with_merge(MergeParts {
        from: sanitize_line(&from),
        into: sanitize_line(&into),
        incoming: look.incoming.commits.len(),
        incoming_capped: look.incoming.capped,
        who: who.iter().map(|name| sanitize_line(name)).collect(),
        who_more,
        changed,
        changed_capped,
        decide,
        not_checkable,
        withheld,
        edits: edits(preview),
        fast_forward: look.fast_forward,
    }))
}

/// His edits around the merge, in the words every other preview uses:
/// the set-aside row (its longest wording; the row cuts it to the box)
/// and what putting them back may do, when that is not clean.
fn edits(preview: &PreviewState) -> Vec<mergewright_term::SanitizedText> {
    let Some(planned) = preview.planned() else {
        return Vec::new();
    };
    let plan = &planned.plan;
    let mut lines = Vec::new();
    if let Some(aside) = plan.set_aside.filter(|aside| aside.files > 0) {
        let wordings = mergewright_ui::preview_box::set_aside_row(aside.files);
        lines.extend(wordings.first().map(|line| sanitize_line(line)));
    }
    if let Some(put_back) = plan
        .put_back
        .as_ref()
        .filter(|it| **it != ConflictForecast::Clean)
    {
        let words = conflict_line(&ConflictForecast::Clean, Some(put_back));
        lines.push(sanitize_line(&words));
    }
    lines
}

/// The incoming list for `preview`, newest first, ages against the app's
/// clock and glyphs in the tier in force.
fn incoming_view(state: &AppState, preview: &PreviewState) -> Option<IncomingView> {
    let look = look(preview)?;
    let (from, _) = names(merge_params(preview)?);
    let now = state.harness.clock.now_seconds();
    let rows = look
        .incoming
        .commits
        .iter()
        .rev()
        .map(|commit| IncomingRow {
            subject: sanitize_line(&commit.subject),
            who: sanitize_line(&first_name(&commit.author.name)),
            age: sanitize_line(&relative_date(commit.author.when.seconds, now)),
            is_merge: commit.parents.len() > 1,
        })
        .collect();
    Some(IncomingView {
        from: sanitize_line(&from),
        rows,
        capped: look.incoming.capped,
        withheld: look.unrelated(),
        cursor: 0,
        ascii: state.tier() == mergewright_core::capabilities::RenderingTier::Ascii,
    })
}

/// A merge preview's own keys: `⏎` opens the list from the box; in the
/// list `esc` is back to the box, `j`/`k` move, `y` merges (unless
/// withheld), everything else is swallowed. `None`: not a merge preview's
/// key, the caller goes on as for any preview.
pub(super) fn merge_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<()> {
    let preview = state.preview.as_ref()?;
    look(preview)?;
    let plain = !key.modifiers.control && !key.modifiers.alt;
    if let super::Showing::Box = preview.showing {
        if key.code != KeyCode::Enter {
            return None;
        }
        let list = incoming_view(state, preview)?;
        state.preview.as_mut()?.showing = Showing::Incoming(list);
        return Some(());
    }
    let withheld = withheld(preview);
    let preview = state.preview.as_mut()?;
    let Showing::Incoming(list) = &mut preview.showing else {
        return Some(());
    };
    match key.code {
        KeyCode::Escape => preview.showing = Showing::Box,
        KeyCode::Down | KeyCode::Char('j') if plain => list.down(),
        KeyCode::Up | KeyCode::Char('k') if plain => list.up(),
        KeyCode::Char('y') if plain && !withheld => confirm_by_key(state, commands, next_id),
        // Swallowed: the list keeps the keyboard, as the box does.
        _ => {}
    }
    Some(())
}

#[cfg(test)]
mod tests;
