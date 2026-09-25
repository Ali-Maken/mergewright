//! A stash put back that clashed (ADR-0045 §2–3, m6-conflicts/72b): whether
//! one is live ([`refresh_clash`]), what landing on it does
//! ([`clashed`]), and its words — the notice, the stop line, the close.
//!
//! **Not a [`super::Stop`].** A clash has no git state: no `MERGE_HEAD`, no
//! `c`, no `a`. It is [`AppState::clash`], and nothing that reads
//! `stopped` changes; only the stop line reads both.

use graft_core::job::JobId;
use graft_core::journal::{EntryId, PutBackWhy};
use graft_ops::clash::{Clash, StashClash, live_clash};
use graft_ops::operation::Operation;

use super::detail::Loading;
use super::events::Command;
use super::relaunch::LiveOrigin;
use super::state::{AppState, ClashState, RefsRead, Status};

/// **Recompute [`AppState::clash`] from what the app holds.** Pure over
/// state: `graft_ops::clash::live_clash` over the journal as last read,
/// git's in-progress state, whether anything is unmerged and the stash list
/// from the refs.
///
/// Waits while any of the three reads is out, so it never judges a fresh
/// status against a journal from before the write (a pop clash would read
/// as none, and back). A clash already held for the same entry keeps its
/// origin; a new one is `PreviousRun` when the launch's reads find it and
/// `ThisRun` after (an undo or redo back into a clash).
pub(crate) fn refresh_clash(state: &mut AppState) {
    if state.journal.reading.is_some()
        || super::status_read::owed(state)
        || matches!(state.refs_read, RefsRead::Pending(_))
    {
        return;
    }
    let (Loading::Ready(read), Some(status), Some(refs)) = (
        &state.journal.entries,
        state.working_tree.as_deref(),
        state.refs.as_deref(),
    ) else {
        return;
    };
    let any_unmerged = status.entries.iter().any(|entry| entry.unmerged);
    let in_progress = state.in_progress.as_ref().map(|detail| detail.kind);
    let found = live_clash(&read.entries, in_progress, any_unmerged, refs.stashes());
    let launch = !state.clash_facts_seen;
    state.clash_facts_seen = true;
    state.clash = found.map(|clash| {
        let origin = match &state.clash {
            Some(was) if was.clash.entry == clash.entry => was.origin,
            _ if launch => LiveOrigin::PreviousRun,
            _ => LiveOrigin::ThisRun,
        };
        ClashState { clash, origin }
    });
}

/// [`refresh_clash`], and — when it has just found a clash the last run
/// left — the launch's landing: the changes pane open, its cursor on the
/// first `CONFLICTS` row, so one `⏎` opens the resolver (design note,
/// "Relaunch with a clash open"). Under a modal the landing is skipped; the
/// stop line still says it.
pub(super) fn refresh_and_land(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.clash_pane_owed && state.uncommitted_row_shown() {
        state.clash_pane_owed = false;
        let first = super::banner::unmerged_paths(state).into_iter().next();
        super::relaunch::land_on_the_work(state, commands, first, next_id);
    }
    let had = state.clash.is_some();
    refresh_clash(state);
    let from_last_run = state
        .clash
        .as_ref()
        .is_some_and(|clash| clash.origin == LiveOrigin::PreviousRun);
    if had || !from_last_run || state.modal_owns_keys() {
        return;
    }
    let first = super::banner::unmerged_paths(state).into_iter().next();
    super::relaunch::land_on_the_work(state, commands, first, next_id);
}

/// `operate::operated`'s `Clashed` arm: the put-back clashed. Records the
/// clash straight away (`ThisRun`: the landing never waits for a read),
/// re-reads everything, and lands in the resolver on the first clashed file
/// with the notice saying which stash and the way back.
///
/// The stash's position comes from the op that was launched
/// (`Running::op`, which holds every launched op since m6-conflicts/67);
/// anything else — a set-aside, 74's — looks it up in the refs by commit.
pub(super) fn clashed(
    state: &mut AppState,
    launched: Option<&Operation>,
    entry: EntryId,
    clash: &Clash,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let index =
        match launched {
            Some(
                Operation::StashPop { index, commit } | Operation::StashApply { index, commit },
            ) if *commit == clash.stash => Some(*index),
            _ => state.refs.as_deref().and_then(|refs| {
                refs.stashes()
                    .iter()
                    .find(|(commit, _)| *commit == clash.stash)
                    .map(|(_, index)| *index)
            }),
        };
    state.clash = Some(ClashState {
        clash: StashClash {
            entry,
            stash: clash.stash,
            index,
            why: clash.why,
        },
        origin: LiveOrigin::ThisRun,
    });
    // Clears the launch's `Busy`: nothing is wrong, the notice says where he
    // is.
    state.status = Status::Ready;
    let mut commands =
        super::operate::refresh_scope(state, graft_ops::apply::Refresh::Everything, next_id);
    let Some(first) = clash.paths.first().cloned() else {
        return commands;
    };
    let notice = notice_words(index, clash.paths.len());
    if super::resolver::land_on_clash(state, &mut commands, first, next_id) {
        super::resolver::set_notice(state, (notice, false));
    }
    commands
}

/// `stash@{0}`, or `the stash` when its position is not known.
fn stash_words(index: Option<usize>) -> String {
    index.map_or_else(
        || "the stash".to_owned(),
        |index| format!("stash@{{{index}}}"),
    )
}

/// Theirs' name in the resolver: `stash@{0}` for a clash Graft recorded;
/// `None` — the role alone — for a shell pop, which has no record.
#[must_use]
pub(crate) fn stash_name(state: &AppState) -> Option<String> {
    let index = state.clash.as_ref()?.clash.index?;
    Some(format!("stash@{{{index}}}"))
}

/// The landing's notice: `stash@{0} clashed in 2 files · esc, u puts it all
/// back`.
fn notice_words(index: Option<usize>, files: usize) -> String {
    let noun = if files == 1 { "file" } else { "files" };
    format!(
        "{} clashed in {files} {noun} \u{b7} esc, u puts it all back",
        stash_words(index)
    )
}

/// The stop line's wordings while a clash is open and nothing is stopped,
/// fullest first (the ladder `banner::stop_wordings` keeps): the origin
/// goes, then the count, then the name.
///
/// `stash@{0} clashed · 1 file conflicted`; from the launch
/// `stash@{0} clash from last run · 1 file conflicted`.
#[must_use]
pub(crate) fn stop_wordings(clash: &ClashState, conflicted: usize) -> Vec<String> {
    let name = stash_words(clash.clash.index);
    let full_head = match clash.origin {
        LiveOrigin::PreviousRun => format!("{name} clash from last run"),
        LiveOrigin::ThisRun | LiveOrigin::OutsideGraft => format!("{name} clashed"),
    };
    let short_head = format!("{name} clashed");
    let noun = if conflicted == 1 { "file" } else { "files" };
    let count = |head: &str, full: bool| match (conflicted, full) {
        (0, _) => head.to_owned(),
        (n, true) => format!("{head} \u{b7} {n} {noun} conflicted"),
        (n, false) => format!("{head} \u{b7} {n} conflicted"),
    };
    let mut rungs: Vec<String> = Vec::with_capacity(4);
    for words in [
        count(&full_head, true),
        count(&short_head, false),
        short_head.clone(),
        "stash clashed".to_owned(),
    ] {
        if rungs.last() != Some(&words) {
            rungs.push(words);
        }
    }
    rungs
}

/// The close after the last file of an **apply** clash: nothing runs, the
/// stash stays. `every file done · stash@{0} kept ✓` (the tick is the
/// renderer's).
#[must_use]
pub(crate) fn kept_status(index: Option<usize>) -> Status {
    Status::Done {
        what: format!("every file done \u{b7} {} kept", stash_words(index)),
        keys: None,
    }
}

/// The `DropPutBack` the close launched answered `Done`:
/// `every file done · stash@{0} dropped ✓ · u brings it back`.
#[must_use]
pub(crate) fn dropped_status(index: Option<usize>) -> Status {
    Status::Done {
        what: format!("every file done \u{b7} {} dropped", stash_words(index)),
        keys: Some("u brings it back".to_owned()),
    }
}

/// Whether finishing this clash drops its stash: a pop or a set-aside does,
/// an apply never (ADR-0045 §3).
#[must_use]
pub(crate) fn drops(why: PutBackWhy) -> bool {
    match why {
        PutBackWhy::Pop | PutBackWhy::SetAside => true,
        PutBackWhy::Apply => false,
    }
}

#[cfg(test)]
mod tests;
