//! A stash put back that clashed (ADR-0045 §2–3, m6-conflicts/72b): whether
//! one is live ([`refresh_clash`]), what landing on it does
//! ([`clashed`]), and its words — the notice, the stop line, the close.
//!
//! **Not a [`super::Stop`].** A clash has no git state: no `MERGE_HEAD`, no
//! `c`, no `a`. It is [`AppState::clash`], and nothing that reads
//! `stopped` changes; only the stop line reads both.

use graft_core::job::JobId;
use graft_core::journal::{EntryId, JournalEntry, OperationKind, PutBackWhy};
use graft_ops::clash::{Clash, StashClash, clash_candidate, live_clash};
use graft_ops::operation::Operation;
use graft_term::sanitize_line;

use super::banner::StopLadder;
use super::detail::Loading;
use super::events::Command;
use super::relaunch::LiveOrigin;
use super::state::{AppState, ClashState, IndexRead, RefsRead, Status};

/// What [`refresh_clash`] needs before it can judge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Judged {
    /// It judged, or is waiting on a read already out: nothing to ask.
    Done,
    /// A clash candidate exists and no current index is held: ask for one.
    NeedsIndex,
}

/// **Recompute [`AppState::clash`] from what the app holds.** Pure over
/// state: `graft_ops::clash::clash_candidate` over the journal as last read,
/// git's in-progress state, whether anything is unmerged and the stash list
/// from the refs; then, while a candidate exists, the tie to the index now
/// ([`AppState::clash_index`]; ADR-0045 §2, amended 2026-09-25).
///
/// Waits while any of the reads is out — the journal, status, refs, or the
/// index — so it never judges a fresh status against a journal from before
/// the write (a pop clash would read as none, and back). With a candidate
/// and no current index it keeps the value held and answers
/// [`Judged::NeedsIndex`]. An index read that failed is no clash, and the
/// status says so (rule 3: worded, never a silent "no").
///
/// A clash already held for the same entry keeps its origin; a new one is
/// `PreviousRun` when the launch's reads find it and `ThisRun` after (an
/// undo or redo back into a clash). With no clash left, the landing's
/// pane owed goes too (review 72+72b m1).
pub(crate) fn refresh_clash(state: &mut AppState) -> Judged {
    if state.journal.reading.is_some()
        || super::status_read::owed(state)
        || matches!(state.refs_read, RefsRead::Pending(_))
        || matches!(state.clash_index, IndexRead::Pending { .. })
    {
        return Judged::Done;
    }
    let (Loading::Ready(read), Some(status), Some(refs)) = (
        &state.journal.entries,
        state.working_tree.as_deref(),
        state.refs.as_deref(),
    ) else {
        return Judged::Done;
    };
    let any_unmerged = status.entries.iter().any(|entry| entry.unmerged);
    let in_progress = state.in_progress.as_ref().map(|detail| detail.kind);
    let candidate = clash_candidate(&read.entries, in_progress, any_unmerged, refs.stashes());
    let found = match (candidate, &state.clash_index) {
        (None, _) => {
            // Nothing Graft put back is in question: no read, no cost.
            state.clash_index = IndexRead::Unknown;
            None
        }
        (Some(_), IndexRead::Unknown | IndexRead::Pending { .. }) => {
            return Judged::NeedsIndex;
        }
        (Some(_), IndexRead::Read(Err(why))) => {
            state.status = Status::Failed(
                sanitize_line(&format!("can't tell which stash clashed \u{b7} {why}")).to_string(),
            );
            None
        }
        // The walk again over the same facts, whole: `live_clash` is the one
        // rule, and the tie is part of it.
        (Some(_), IndexRead::Read(Ok(index_now))) => {
            live_clash(&read.entries, in_progress, index_now, refs.stashes())
        }
    };
    let launch = !state.clash_facts_seen;
    state.clash_facts_seen = true;
    // A clash that ended, or gave way to another: its picks go with it
    // (ease-pass 05). An operation's picks never meet this — it has none.
    let was = state.clash.as_ref().map(|was| &was.clash.entry);
    if was.is_some() && was != found.as_ref().map(|clash| &clash.entry) {
        state.picks.clear();
    }
    state.clash = found.map(|clash| {
        let origin = match &state.clash {
            Some(was) if was.clash.entry == clash.entry => was.origin,
            _ if launch => LiveOrigin::PreviousRun,
            _ => LiveOrigin::ThisRun,
        };
        ClashState { clash, origin }
    });
    if state.clash.is_none() {
        state.clash_pane_owed = false;
    }
    Judged::Done
}

/// A journal, working-tree or refs read landed: any index held is stale,
/// then [`refresh_clash`] and the landings ([`judge_and_land`]).
pub(super) fn refresh_and_land(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    state.clash_index = match std::mem::take(&mut state.clash_index) {
        IndexRead::Pending { id, .. } => IndexRead::Pending { id, stale: true },
        IndexRead::Unknown | IndexRead::Read(_) => IndexRead::Unknown,
    };
    judge_and_land(state, commands, next_id);
}

/// The index read `id` answered. Any other id is an older read's, dropped.
/// A read that went stale while it was out is not judged: one more is
/// asked in its place, so one read is out at a time.
pub(super) fn index_answered(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    read: Result<graft_core::journal::IndexState, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let IndexRead::Pending { id: asked, stale } = state.clash_index else {
        return;
    };
    if asked != id {
        return;
    }
    state.clash_index = if stale {
        IndexRead::Unknown
    } else {
        IndexRead::Read(read)
    };
    judge_and_land(state, commands, next_id);
}

/// [`refresh_clash`], asking for the index when it needs one, and the two
/// landings:
/// - the pane the clash landing owes (`clash_pane_owed`), once the
///   uncommitted row shows, while the clash stands and no box is over the
///   resolver (it waits under one; review 72+72b m1);
/// - when it has just found a clash the last run left, the launch's
///   landing: the changes pane open, its cursor on the first `CONFLICTS`
///   row, so one `⏎` opens the resolver (design note, "Relaunch with a clash
///   open"). Under a modal the landing is skipped; the stop line still says
///   it.
fn judge_and_land(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let had = state.clash.is_some();
    if refresh_clash(state) == Judged::NeedsIndex {
        let id = next_id();
        state.clash_index = IndexRead::Pending { id, stale: false };
        commands.push(Command::ReadIndexState { id });
    }
    if state.clash_pane_owed
        && state.clash.is_some()
        && state.uncommitted_row_shown()
        && !box_over_the_landing(state)
    {
        state.clash_pane_owed = false;
        let first = super::banner::unmerged_paths(state).into_iter().next();
        super::relaunch::land_on_the_work(state, commands, first, next_id);
    }
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

/// A modal other than the resolver the clash landing opened owns the keys:
/// a text field, a confirm, a preview, the finish summary or the commit
/// editor. The pane owed waits under it.
fn box_over_the_landing(state: &AppState) -> bool {
    state.text_field_open()
        || state.confirm.is_some()
        || state.preview.is_some()
        || state.finish.is_some()
        || state.commit_editor.is_some()
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
    // A set-aside's `u` takes back the operation it was set aside around:
    // the launched one, or — with nothing launched, from the `c` that
    // finished a stopped rebase or merge — the journal's entry.
    let kind = match launched {
        Some(op) => Some(op.kind()),
        None => entry_kind(state, &entry),
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
    let set_aside = clash.why == PutBackWhy::SetAside;
    let undoes = set_aside.then(|| undone_with_edits(kind)).flatten();
    let notice = notice_words(index, clash.paths.len(), set_aside, undoes, &state.keymap);
    if super::resolver::land_on_clash(state, &mut commands, first, next_id) {
        super::resolver::set_notice(state, (notice, false));
    } else {
        // Refused — a resolver or a box already owns the keys. The words
        // still reach him, on the status line (review 72+72b m2): drawn as
        // a notice (`Waiting` is the one status drawn neither as a failure
        // nor with a tick), and the stop line keeps saying it after the next
        // key clears it.
        state.status = Status::Waiting(notice);
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
/// back`. A set-aside's (`set_aside`) names the edits and the operation `u`
/// undoes with them (`undoes`: `rebase`, `switch`, `merge` …): `your edits
/// clashed in 2 files · esc, u undoes the rebase too` (m6-conflicts/74). Its
/// operation not known (`undoes` `None`), it keeps the pop's clause and
/// names no kind (19d-a3).
fn notice_words(
    index: Option<usize>,
    files: usize,
    set_aside: bool,
    undoes: Option<&str>,
    keys: &graft_ui::keymap::Keymap,
) -> String {
    let noun = if files == 1 { "file" } else { "files" };
    let who = if set_aside {
        "your edits".to_owned()
    } else {
        stash_words(index)
    };
    let words = format!("{who} clashed in {files} {noun}");
    match undo_clause_words(undoes, keys) {
        Some(clause) => format!("{words} \u{b7} {clause}"),
        None => words,
    }
}

/// What `u` does for a clash, as its landing notice says it: `esc, u puts
/// it all back`, or for a set-aside whose operation is known `esc, u undoes
/// the <rebase|switch|merge …> too`.
/// The one wording: the notice and the resolver's answer to `u`
/// (m6-conflicts/85 m-i) both take it from here.
///
/// Two keys (W5): `esc` closes the resolver (`app.back` there), then `u`
/// on the graph (`app.undo`). Esc unbound → `u …` alone; undo unbound →
/// `None`, the clause goes.
pub(crate) fn undo_clause_words(
    undoes: Option<&str>,
    keys: &graft_ui::keymap::Keymap,
) -> Option<String> {
    use graft_ui::keymap::ActionId;
    let verb = match undoes {
        Some(what) => format!("undoes the {what} too"),
        None => "puts it all back".to_owned(),
    };
    let undo = keys.key_verb(&[ActionId("app.undo")], &verb)?;
    Some(match keys.spell_keys(&[ActionId("app.back")]) {
        Some(back) => format!("{back}, {undo}"),
        None => undo,
    })
}

/// [`undo_clause_words`] for the clash standing, or `None` with no clash.
/// A set-aside's operation is read off the journal entry that put the edits
/// back, as [`clashed`] words it at the landing.
#[must_use]
pub(crate) fn undo_clause(state: &AppState) -> Option<String> {
    let clash = state.clash.as_ref()?;
    let undoes = (clash.clash.why == PutBackWhy::SetAside)
        .then(|| undone_with_edits(entry_kind(state, &clash.clash.entry)))
        .flatten();
    undo_clause_words(undoes, &state.keymap)
}

/// The kind of the journal entry `id`, when the journal read holds it.
fn entry_kind(state: &AppState, id: &EntryId) -> Option<OperationKind> {
    let Loading::Ready(read) = &state.journal.entries else {
        return None;
    };
    read.entries
        .iter()
        .find(|entry| entry.id == *id)
        .map(|entry| entry.kind)
}

/// What a set-aside's `u` takes back with the edits, in the notice's
/// words: the entry's own kind name (graft-ops `apply::kind_name`, the
/// journal's), so a cherry-pick says `cherry-pick` and a pull `pull`
/// (m8-polish-and-ship/19d-a2). A checkout and a switching create (only a
/// switching create sets edits aside, 10b) keep the shipped `switch`.
///
/// Every kind is named, no catch-all: a kind added later is worded here
/// on purpose. A kind not known — nothing launched, and the journal read
/// does not hold the entry — is `None`: the notice names no kind rather
/// than guess one (19d-a3, ticket §R7.1; it said `rebase` before).
fn undone_with_edits(kind: Option<OperationKind>) -> Option<&'static str> {
    let kind = kind?;
    Some(match kind {
        OperationKind::Checkout | OperationKind::BranchCreate => "switch",
        // The kinds that set edits aside (graft-ops `Operation::sets_aside`).
        OperationKind::Rebase
        | OperationKind::Rewrite
        | OperationKind::CherryPick
        | OperationKind::Pull
        | OperationKind::Recover
        | OperationKind::Merge
        // The rest never carry a `SetAside` put-back; were one to, its own
        // name is still the truest word.
        | OperationKind::Stage
        | OperationKind::Unstage
        | OperationKind::Discard
        | OperationKind::Commit
        | OperationKind::Amend
        | OperationKind::StashPush
        | OperationKind::StashApply
        | OperationKind::StashPop
        | OperationKind::StashDrop
        | OperationKind::Fetch
        | OperationKind::Push
        | OperationKind::Revert
        | OperationKind::Reset
        | OperationKind::BranchDelete
        | OperationKind::TagCreate
        | OperationKind::TagDelete
        | OperationKind::BranchFromStash
        | OperationKind::ResolveConflict
        | OperationKind::RestartConflict
        | OperationKind::FinishMerge
        | OperationKind::AbortMerge
        | OperationKind::RevertTo
        | OperationKind::Undo
        | OperationKind::Redo
        | OperationKind::Migrate => graft_ops::apply::kind_name(kind),
    })
}

/// The stop line's wordings while a clash is open and nothing is stopped,
/// fullest first (the ladder `banner::stop_wordings` keeps): the origin
/// goes, then the count, then the name.
///
/// `stash@{0} clashed · 1 file conflicted`; from the launch
/// `stash@{0} clash from last run · 1 file conflicted`.
#[must_use]
pub(crate) fn stop_wordings(clash: &ClashState, conflicted: usize) -> StopLadder {
    // A set-aside's clash is `your edits`, never the stash he never named
    // (m6-conflicts/74); its last rung is its own name, not `stash clashed`.
    let set_aside = clash.clash.why == PutBackWhy::SetAside;
    let name = if set_aside {
        "your edits".to_owned()
    } else {
        stash_words(clash.clash.index)
    };
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
        if set_aside {
            short_head.clone()
        } else {
            "stash clashed".to_owned()
        },
    ] {
        if rungs.last() != Some(&words) {
            rungs.push(words);
        }
    }
    // A clash has no banner: nothing else on screen names the stash, so
    // its reason outranks the bar's tail keys (review 72+72b M1).
    StopLadder {
        wordings: rungs,
        before_keys: true,
    }
}

/// The close after the last file of an **apply** clash: nothing runs, the
/// stash stays. `✓ every file done · stash@{0} kept` (the tick is the
/// renderer's).
#[must_use]
pub(crate) fn kept_status(index: Option<usize>) -> Status {
    Status::Done {
        what: format!("every file done \u{b7} {} kept", stash_words(index)),
        also: None,
        keys: None,
    }
}

/// The `DropPutBack` the close launched answered `Done`:
/// `✓ every file done · stash@{0} dropped  u brings it back`. A
/// set-aside's names what it dropped the same way, never the stash's
/// position — he never named it: `✓ every file done · set-aside copy
/// dropped  u brings it back` (ADR-0045 §3 amended, m6-conflicts/84). Not `your
/// edits are back`: after `o branch` in every block they are not, and `u`
/// brings the copy back.
#[must_use]
pub(crate) fn dropped_status(
    index: Option<usize>,
    why: PutBackWhy,
    keys: &graft_ui::keymap::Keymap,
) -> Status {
    let dropped = match why {
        PutBackWhy::SetAside => "set-aside copy".to_owned(),
        PutBackWhy::Pop | PutBackWhy::Apply => stash_words(index),
    };
    Status::Done {
        what: format!("every file done \u{b7} {dropped} dropped"),
        also: None,
        keys: brings_back(keys),
    }
}

/// `your edits are back` for an entry that set edits aside, answered
/// `Done`: the put-back ran clean and the set-aside copy is dropped
/// (ADR-0045 §4). `None` for every other entry, so a clean-tree rebase's
/// line is unchanged (m6-conflicts/74). Drawn after the tick, as
/// `Status::Done::also`: `✓ rebase main onto origin/main · your edits are
/// back`. The one place the clause is worded: the op's own done line and a
/// stopped rebase's close both take it from here.
///
/// A staged-new file he had deleted from disk comes back with them (git's
/// `stash apply --index`); the clause names it, `your edits are back ·
/// notes.md came back from the index` (m8-polish-and-ship/14).
#[must_use]
pub(crate) fn edits_back(applied: &graft_ops::apply::Applied) -> Option<String> {
    if !is_set_aside_entry(&applied.entry) {
        return None;
    }
    Some(
        match graft_ops::apply::came_back_words(&applied.came_back) {
            Some(came_back) => format!("your edits are back \u{b7} {came_back}"),
            None => "your edits are back".to_owned(),
        },
    )
}

/// Did `entry` set edits aside around its operation (ADR-0045 §4)?
#[must_use]
pub(crate) fn is_set_aside_entry(entry: &JournalEntry) -> bool {
    entry
        .put_back
        .as_ref()
        .is_some_and(|put_back| put_back.why == PutBackWhy::SetAside)
}

/// Is the clash standing a set-aside's (m6-conflicts/74)? Its names and
/// words are `your edits`, never the stash.
#[must_use]
pub(crate) fn is_set_aside_clash(state: &AppState) -> bool {
    state
        .clash
        .as_ref()
        .is_some_and(|clash| clash.clash.why == PutBackWhy::SetAside)
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
pub(crate) mod tests;

/// `u brings it back`: a dropped stash or set-aside copy, `app.undo` on
/// the graph; `None` when unbound.
pub(crate) fn brings_back(keys: &graft_ui::keymap::Keymap) -> Option<String> {
    keys.key_verb(&[graft_ui::keymap::ActionId("app.undo")], "brings it back")
}
