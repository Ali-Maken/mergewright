//! Which wakes nothing asked for (m7-power-tools/12 `idle_gap_wakes`;
//! ADR-0065 D1). An input may cause reads, and each read must be asked by
//! the update handling the input, by the loop's own `Rested` after it, or by
//! the first window answering that input's `ShowRows`. Each such read wakes
//! the loop at most twice: its `Started` and its one final answer. Nothing
//! else may: no read asked by another read's answer, no timer, no poll.

use std::collections::HashSet;
use std::ops::Range;

use mergewright::update::{AppEvent, Command, RestRead};
use mergewright_core::job::JobId;

use super::paced::{Handed, Kind};

/// Which wake the loop was handling when it sent a command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AskedFrom {
    /// An input's own update (a key).
    #[default]
    Input,
    /// The loop's own `Rested` after an input.
    Rest,
    /// The first window answering an input's `ShowRows`.
    InputWindow,
    /// Any other job event: an answer asking on its own account.
    Answer,
}

/// A command the loop sent that carries its job's id, and what asked it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ask {
    pub(crate) id: JobId,
    pub(crate) from: AskedFrom,
}

/// Which of a job's wakes an event is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Role {
    /// `AppEvent::Started`.
    Started,
    /// The one answer that ends the job.
    Final,
    /// Anything between (`Progress`): never asked for.
    Between,
}

/// The id a command carries, for the reads that can wake at rest.
pub(crate) fn command_id(command: &Command) -> Option<JobId> {
    match command {
        Command::AtRest(id, RestRead::CountFolded { .. } | RestRead::BranchStatus(_))
        | Command::ReadRefs(id)
        | Command::ReadWorkingTree(id)
        | Command::ReadHistory(id, _)
        | Command::ReadJournal(id)
        | Command::ReadRemotes(id)
        | Command::ContainedIn(id, _)
        | Command::EvilProbe(id, _)
        | Command::ForcePushProbe(id, ..)
        | Command::DuplicatesProbe(id, _)
        | Command::ReadStashes(id)
        | Command::ResolveRowTags(id, _)
        | Command::TourProgress(id) => Some(*id),
        _ => None,
    }
}

/// The job and role an event is, if it is a job's.
pub(crate) fn job_of(event: &AppEvent) -> Option<(JobId, Role)> {
    match event {
        AppEvent::Started(id, _) => Some((*id, Role::Started)),
        AppEvent::Produced(id, _) => Some((*id, Role::Final)),
        AppEvent::Progress(id, _) => Some((*id, Role::Between)),
        _ => None,
    }
}

/// The wakes in `handed[range]` that nothing asked for. A key and the
/// loop's own rest never count. A window counts unless it is the first
/// answering a `ShowRows` the loop sent (`windows`). Another job event
/// counts unless it is the `Started` or the first final answer of an id
/// asked from an input, its rest or its first window (`asks`); a second
/// answer, an id nobody asked for, and a read asked by an answer all count.
pub(super) fn unasked(
    handed: &[Handed],
    range: Range<usize>,
    windows: &[u64],
    asks: &[Ask],
) -> Vec<&'static str> {
    let mut answered: HashSet<u64> = HashSet::new();
    let mut seen: HashSet<(JobId, Role)> = HashSet::new();
    for before in &handed[..range.start] {
        note(before, &mut answered, &mut seen);
    }
    let mut woke = Vec::new();
    for wake in &handed[range] {
        let was_asked = match wake.kind {
            Kind::Key(..) | Kind::Rest => continue,
            Kind::Window(id) => windows.contains(&id) && answered.insert(id),
            Kind::Other => wake.job.is_some_and(|(id, role)| {
                role != Role::Between
                    && asks
                        .iter()
                        .any(|ask| ask.id == id && ask.from != AskedFrom::Answer)
                    && seen.insert((id, role))
            }),
        };
        if !was_asked {
            woke.push(wake.label);
        }
    }
    woke
}

/// Remembers what an earlier wake already answered.
fn note(wake: &Handed, answered: &mut HashSet<u64>, seen: &mut HashSet<(JobId, Role)>) {
    match (wake.kind, wake.job) {
        (Kind::Window(id), _) => {
            answered.insert(id);
        }
        (_, Some(job)) => {
            seen.insert(job);
        }
        _ => {}
    }
}
