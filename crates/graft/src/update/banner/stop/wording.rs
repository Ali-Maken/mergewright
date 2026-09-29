//! The words the footer gives a stopped operation: `stop_reason`, `stop_line` and their ladders.

use super::*;

/// Why the live operation stopped, in the footer's words
/// (m6-conflicts/12, scope item 2): `merge stopped · 1 file conflicted`,
/// `rebase stopped again at step 4 of 7 · 2 files conflicted`,
/// `merge in progress · begun outside Graft · 1 file conflicted`.
///
/// `live` is the working-tree read's detail and its unmerged count. It is
/// used **only** when its kind is the stop's: then the step and the count
/// follow the read, so resolving a file shows at once. Otherwise the stop's
/// own kind and count are used and the step is left out — a step from a
/// different operation would be a wrong number.
#[must_use]
pub fn stop_reason(
    stop: &crate::update::state::Stop,
    live: Option<(&InProgressDetail, usize)>,
) -> String {
    Wording::of(stop, live).words(Origin::Full, true, Count::Full)
}

/// [`stop_reason`] and the shorter wordings the footer falls back to when
/// the full one does not fit (m6-conflicts/24, review m6-amend-11-12 M1),
/// fullest first.
///
/// Rung 0 is [`stop_reason`], every part spelled out. Every later rung has
/// had to shorten, so it says the count short (`13 conflicted`) and drops
/// whole parts, lowest value first (m6-conflicts/61, "shorten both"): first
/// the origin goes short (`from the shell`, `from last run`), then it goes,
/// then the step, then the count. So the count is drawn whole or not at
/// all, and the last wording — the only one the footer ever cuts — carries
/// no number. A rung that repeats the one before (a stop from this run has
/// no origin to shorten) is skipped.
#[must_use]
pub fn stop_wordings(
    stop: &crate::update::state::Stop,
    live: Option<(&InProgressDetail, usize)>,
) -> Vec<String> {
    let wording = Wording::of(stop, live);
    let mut rungs: Vec<String> = Vec::with_capacity(5);
    for (origin, step, count) in [
        (Origin::Full, true, Count::Full),
        (Origin::Short, true, Count::Short),
        (Origin::Gone, true, Count::Short),
        (Origin::Gone, false, Count::Short),
        (Origin::Gone, false, Count::Gone),
    ] {
        let words = wording.words(origin, step, count);
        // A part the stop does not have drops nothing: skip the repeat.
        if rungs.last() != Some(&words) {
            rungs.push(words);
        }
    }
    rungs
}

/// How much of who began the stop a wording says.
#[derive(Clone, Copy)]
pub(super) enum Origin {
    /// `begun outside Graft`, `from a previous run`.
    Full,
    /// `from the shell`, `from last run`.
    Short,
    /// Nothing.
    Gone,
}

/// How a wording says the conflicted count.
#[derive(Clone, Copy)]
pub(super) enum Count {
    /// `13 files conflicted`: the full rung only.
    Full,
    /// `13 conflicted`: every rung that has had to shorten.
    Short,
    /// Nothing.
    Gone,
}

/// The facts [`stop_reason`] words, gathered once for every wording.
pub(super) struct Wording {
    name: &'static str,
    origin: crate::update::relaunch::LiveOrigin,
    again: bool,
    at_step: Option<(u32, u32)>,
    count: usize,
}

impl Wording {
    /// `live` counts only when its kind is the stop's (see [`stop_reason`]).
    fn of(stop: &crate::update::state::Stop, live: Option<(&InProgressDetail, usize)>) -> Self {
        let same_kind = live.filter(|(detail, _)| detail.kind == stop.kind);
        Self {
            name: stop.kind.name(),
            origin: stop.origin,
            again: stop.again,
            at_step: same_kind.and_then(|(detail, _)| detail.step),
            count: same_kind.map_or(stop.paths_at_stop, |(_, count)| count),
        }
    }

    /// The words with each droppable part at the form asked: who began it
    /// (`origin`), the step, the count.
    fn words(&self, origin: Origin, step: bool, count: Count) -> String {
        use crate::update::relaunch::LiveOrigin;

        let name = self.name;
        let at_step = self.at_step.filter(|_| step);
        let mut reason = match self.origin {
            LiveOrigin::ThisRun => {
                let again = if self.again { " again" } else { "" };
                let at =
                    at_step.map_or_else(String::new, |(at, of)| format!(" at step {at} of {of}"));
                format!("{name} stopped{again}{at}")
            }
            LiveOrigin::PreviousRun | LiveOrigin::OutsideGraft => {
                let mut words = match (self.origin, origin) {
                    (LiveOrigin::PreviousRun, Origin::Full) => {
                        format!("{name} still in progress from a previous run")
                    }
                    // `still` goes: `from last run` carries it.
                    (LiveOrigin::PreviousRun, Origin::Short) => {
                        format!("{name} in progress \u{b7} from last run")
                    }
                    (LiveOrigin::PreviousRun, Origin::Gone) => format!("{name} still in progress"),
                    (_, Origin::Full) => format!("{name} in progress \u{b7} begun outside Graft"),
                    (_, Origin::Short) => format!("{name} in progress \u{b7} from the shell"),
                    (_, Origin::Gone) => format!("{name} in progress"),
                };
                if let Some((at, of)) = at_step {
                    words.push_str(&format!(" \u{b7} step {at} of {of}"));
                }
                words
            }
        };
        if self.count > 0 {
            match count {
                Count::Full => {
                    let files = if self.count == 1 { "file" } else { "files" };
                    reason.push_str(&format!(" \u{b7} {} {files} conflicted", self.count));
                }
                Count::Short => reason.push_str(&format!(" \u{b7} {} conflicted", self.count)),
                Count::Gone => {}
            }
        }
        reason
    }
}

/// The stop line's wordings, fullest first, and whether the reason
/// outranks the bar's tail keys (`graft_ui::graph_screen::StopWording::before_keys`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopLadder {
    /// Fullest first; each shorter one drops whole parts.
    pub wordings: Vec<String>,
    /// Set only by a stash clash's ladder (`update::clash::stop_wordings`),
    /// which has no banner to name it (review 72+72b M1). Every stop git
    /// has part-way keeps its bar whole: its banner says it.
    pub before_keys: bool,
}

/// [`stop_reason`] for the stop standing in `state`, worded from its last
/// working-tree read. `None` while nothing is stopped.
#[must_use]
pub fn stop_line(state: &AppState) -> Option<String> {
    stop_line_wordings(state)?.wordings.into_iter().next()
}

/// [`stop_wordings`] for the stop standing in `state`, worded like
/// [`stop_line`] (whose words are the first). With nothing stopped, a stash
/// clash's (m6-conflicts/72b: `stash@{0} clashed · 1 file conflicted`).
/// `None` while neither stands.
#[must_use]
pub fn stop_line_wordings(state: &AppState) -> Option<StopLadder> {
    let Some(stop) = state.stopped.as_ref() else {
        return clash_wordings(state);
    };
    let count = unmerged_paths(state).len();
    let live = state.in_progress.as_ref().map(|detail| (detail, count));
    let mut wordings = stop_wordings(stop, live);
    // A rebase that set edits aside and stopped (ADR-0045 §4,
    // m6-conflicts/74): the fullest wording says where they are, and it is
    // the first part to go. Added here, not on `Stop`: every other reader
    // of the stop is unchanged.
    if has_edits_aside(state)
        && let Some(full) = wordings.first()
    {
        let aside = format!("{full} \u{b7} edits aside");
        wordings.insert(0, aside);
    }
    Some(StopLadder {
        wordings,
        before_keys: false,
    })
}

/// A live entry — the stop's, `Pending` — set edits aside and waits to put
/// them back (`JournalEntry::put_back`, read off the journal as last read).
pub(super) fn has_edits_aside(state: &AppState) -> bool {
    let crate::update::detail::Loading::Ready(read) = &state.journal.entries else {
        return false;
    };
    read.entries.iter().any(|entry| {
        state.live_entries.contains(&entry.id)
            && entry.status == graft_core::journal::EntryStatus::Pending
            && crate::update::clash::is_set_aside_entry(entry)
    })
}

/// The open stash clash's wordings, counted from the last working-tree
/// read; `None` without one.
pub(super) fn clash_wordings(state: &AppState) -> Option<StopLadder> {
    let clash = state.clash.as_ref()?;
    Some(crate::update::clash::stop_wordings(
        clash,
        unmerged_paths(state).len(),
    ))
}
