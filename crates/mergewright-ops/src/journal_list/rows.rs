//! Events in order, and the lost rows under them.
//!
//! An event is a journal entry (with the steps that were its own moves) or
//! one of the shell's steps. They sort newest first by time — an entry by
//! its `at_unix_ms`, a step by its record's second — and on a tie an entry
//! first, then journal order or log order. That order depends on the event
//! alone, so a later stage (more events, lost rows) never reorders the rows
//! of an earlier one.
//!
//! A lost run sits under the newest event whose old value was its tip: the
//! event after which nothing held it.

use std::cmp::Ordering;

use mergewright_core::journal::JournalEntry;
use mergewright_core::oid::Oid;
use mergewright_git::history_reads::Unreached;
use mergewright_git::reflog::ReflogRecord;

use super::fate::judged_by;
use super::steps::{Steps, branch_short, checkout_names, is_hex_id};
use super::words::words_on;
use super::{HeldBy, ListRow, LostCommit, LostEdit, OutsideRecord, OutsideStep, fate_of};

/// One event, before its rows are made.
#[derive(Clone, Debug)]
pub(super) enum Event {
    /// A journal entry, and the steps (indices into `Steps::steps`) that
    /// were its own.
    Mergewright { entry: usize, own: Vec<usize> },
    /// One of the shell's steps.
    Outside { step: usize },
}

/// The events, newest first.
pub(super) fn events(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    grouped: &Steps,
    owner: &[Option<usize>],
) -> Vec<Event> {
    let mut timed: Vec<(i64, Event)> = Vec::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        let own = (0..grouped.steps.len())
            .filter(|step| owner[*step] == Some(entry_index))
            .collect();
        timed.push((
            entry.at_unix_ms,
            Event::Mergewright {
                entry: entry_index,
                own,
            },
        ));
    }
    for (step_index, step) in grouped.steps.iter().enumerate() {
        if owner[step_index].is_none() {
            let at = outside[step.primary].record.at.saturating_mul(1000);
            timed.push((at, Event::Outside { step: step_index }));
        }
    }
    timed.sort_by(|(left_at, left), (right_at, right)| {
        right_at
            .cmp(left_at)
            .then_with(|| newer_first(left, right, grouped))
    });
    timed.into_iter().map(|(_, event)| event).collect()
}

/// The tie-break at one instant: an entry before a step; later journal
/// entries first; steps in log order (records come newest first).
fn newer_first(left: &Event, right: &Event, grouped: &Steps) -> Ordering {
    match (left, right) {
        (Event::Mergewright { entry: left, .. }, Event::Mergewright { entry: right, .. }) => {
            right.cmp(left)
        }
        (Event::Outside { step: left }, Event::Outside { step: right }) => grouped.steps[*left]
            .primary
            .cmp(&grouped.steps[*right].primary),
        (Event::Mergewright { .. }, Event::Outside { .. }) => Ordering::Less,
        (Event::Outside { .. }, Event::Mergewright { .. }) => Ordering::Greater,
    }
}

/// What the lost rows are made from.
pub(super) struct Lost<'a> {
    pub found: Option<&'a Unreached>,
    pub subject_of: &'a dyn Fn(Oid) -> Option<String>,
    pub branch_exists: &'a dyn Fn(&str) -> bool,
    pub edits: &'a [LostEdit],
}

/// What a value was, as far as one record or ref says: the hint behind
/// [`HeldBy`], strongest first.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Hint {
    /// A branch's own record, or a journal entry's ref.
    Branch(String),
    /// `checkout: moving from <name> to …`: `HEAD` was on it.
    SwitchedFrom(String),
    /// A `HEAD` record while on a branch, by the logs.
    On(String),
    /// Nothing more.
    Head,
}

/// The rows: each event, then its lost commits (in candidate order), then
/// its lost edits.
pub(super) fn assemble(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    grouped: &Steps,
    events: &[Event],
    lost: &Lost<'_>,
) -> Vec<ListRow> {
    let olds: Vec<Vec<(Oid, Hint)>> = events
        .iter()
        .map(|event| old_values(entries, outside, grouped, event))
        .collect();
    let mut under: Vec<Vec<LostCommit>> = vec![Vec::new(); events.len()];
    if let Some(found) = lost.found {
        let in_force = crate::journal::in_force_all(entries);
        for run in &found.lost {
            if let Some((event, hint)) = place(&olds, run.tip) {
                let mergewright = match &events[event] {
                    Event::Mergewright { entry, .. } => Some(judged_by(entries, &in_force, *entry)),
                    Event::Outside { .. } => None,
                };
                under[event].push(LostCommit {
                    run: *run,
                    subject: (lost.subject_of)(run.tip),
                    held_by: held_by(hint, run.tip, outside, lost.branch_exists),
                    fate: fate_of(mergewright, run.tip),
                });
            }
            // A tip no event had as an old value cannot be placed. Every
            // candidate `find_lost` passes is some record's or entry's old
            // value, and the only records `steps` drops moved nothing (their
            // value is the next record's old one), so this is reached only
            // by a `lost` read from other inputs than these.
        }
    }
    let mut rows = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let at = rows.len();
        rows.push(event_row(entries, outside, grouped, event));
        for commit in std::mem::take(&mut under[index]) {
            rows.push(ListRow::Lost { event: at, commit });
        }
        if let Event::Mergewright { entry, .. } = event {
            for edit in lost
                .edits
                .iter()
                .filter(|edit| edit.entry == entries[*entry].id)
            {
                rows.push(ListRow::LostEdit {
                    event: at,
                    edit: edit.clone(),
                });
            }
        }
    }
    rows
}

fn event_row(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    grouped: &Steps,
    event: &Event,
) -> ListRow {
    match event {
        Event::Mergewright { entry, .. } => ListRow::Mergewright(entries[*entry].id.clone()),
        Event::Outside { step } => {
            let step = &grouped.steps[*step];
            let primary = &outside[step.primary];
            let branch = grouped.on[step.primary].as_deref().unwrap_or("HEAD");
            ListRow::Outside(OutsideStep {
                record: primary.record.clone(),
                words: words_on(&primary.record, primary.back, branch, step.onto.as_deref()),
                folded: step.folded,
            })
        }
    }
}

/// Every old value an event had, with what it says held it.
fn old_values(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    grouped: &Steps,
    event: &Event,
) -> Vec<(Oid, Hint)> {
    let mut olds = Vec::new();
    let from_step = |step: usize, olds: &mut Vec<(Oid, Hint)>| {
        for member in &grouped.steps[step].members {
            let record = &outside[*member].record;
            if let Some(old) = record.old {
                olds.push((old, record_hint(record, grouped.on[*member].as_deref())));
            }
        }
    };
    match event {
        Event::Outside { step } => from_step(*step, &mut olds),
        Event::Mergewright { entry, own } => {
            for step in own {
                from_step(*step, &mut olds);
            }
            for (key, before) in super::branch_or_head(&entries[*entry].refs_before) {
                let hint =
                    branch_short(key).map_or(Hint::Head, |name| Hint::Branch(name.to_owned()));
                olds.push((before, hint));
            }
        }
    }
    olds
}

fn record_hint(record: &ReflogRecord, on: Option<&str>) -> Hint {
    if let Some(name) = branch_short(&record.refname) {
        return Hint::Branch(name.to_owned());
    }
    if let Some((from, _)) = checkout_names(&record.subject) {
        // git writes a full hex id as `from` when `HEAD` was detached: the
        // tip it left was `HEAD`'s alone, whatever branch `on` inferred
        // (review m7-22, M1).
        return if is_hex_id(from) {
            Hint::Head
        } else {
            Hint::SwitchedFrom(from.to_owned())
        };
    }
    on.map_or(Hint::Head, |name| Hint::On(name.to_owned()))
}

/// The newest event with `tip` as an old value, and the strongest hint it
/// gives.
fn place(olds: &[Vec<(Oid, Hint)>], tip: Oid) -> Option<(usize, Hint)> {
    olds.iter().enumerate().find_map(|(event, values)| {
        values
            .iter()
            .filter(|(oid, _)| *oid == tip)
            .map(|(_, hint)| hint.clone())
            .min()
            .map(|hint| (event, hint))
    })
}

/// What held `tip`, by the hint. A branch only inferred from `HEAD`'s
/// records (`Hint::On`) that exists names `tip` only if its own log ever
/// held it (m7/38): `HEAD` on `feature-b` mid-rebase moved through commits
/// `feature-b` never had, and "was feature-b's tip" would send `⏎` to put
/// `feature-b` on one. A gone branch's log went with it: inferred stands.
fn held_by(
    hint: Hint,
    tip: Oid,
    outside: &[OutsideRecord],
    branch_exists: &dyn Fn(&str) -> bool,
) -> HeldBy {
    match hint {
        Hint::On(name)
            if branch_exists(&name)
                && !branch_log_names(
                    &format!("refs/heads/{name}"),
                    tip,
                    outside.iter().map(|outside| &outside.record),
                ) =>
        {
            HeldBy::Head
        }
        Hint::Branch(name) | Hint::SwitchedFrom(name) | Hint::On(name) => HeldBy::Branch {
            gone: !branch_exists(&name),
            name,
        },
        Hint::Head => HeldBy::Head,
    }
}

/// Whether `branch`'s own log (`refs/heads/<b>`, full name) names `commit`
/// as an old or new value in `records`.
pub(super) fn branch_log_names<'a>(
    branch: &str,
    commit: Oid,
    records: impl IntoIterator<Item = &'a ReflogRecord>,
) -> bool {
    records.into_iter().any(|record| {
        record.refname == branch && (record.old == Some(commit) || record.new == Some(commit))
    })
}
