//! Reflog records into steps, and steps onto Graft's own entries
//! (ADR-0046 §2).
//!
//! git writes more than one record for one act: a `reset` moves the branch
//! and `HEAD` (two records, same old and new, same second); a rebase writes
//! a `HEAD` record per replayed commit and one finish record on the branch;
//! `git stash` resets `HEAD` to itself. The list shows one row per act, so
//! the records are grouped first:
//!
//! 1. A run of `HEAD` records whose subjects start `rebase` folds into the
//!    branch's `rebase (finish): refs/heads/<b> onto …` record written
//!    within [`PAIR_WINDOW_SECS`] of the run's newest, or into the run's
//!    newest record when there is none.
//! 2. A record that moved nothing (old = new) is dropped, unless it is a
//!    `checkout:` — a switch between branches on one commit moves `HEAD`'s
//!    branch, if not its commit. Dropping one loses no lost commit: `HEAD`'s
//!    log is contiguous, so its value is the next record's old value, or
//!    `HEAD`'s value now.
//! 3. A `HEAD` record pairs with a branch record of the same old and new
//!    values written within [`PAIR_WINDOW_SECS`] of it.
//!
//! A step is Graft's when one of its records is one of Graft's moves: a
//! journal entry's `refs_before`/`refs_after` for that ref, old and new
//! alike; or, for a switch (whose entry records `HEAD`'s branches but no
//! commit), `checkout: moving from <before> to <after>` within
//! [`CHECKOUT_WINDOW_MS`] of the entry.

use std::collections::BTreeMap;

use graft_core::journal::{HeadRef, JournalEntry};
use graft_git::reflog::ReflogRecord;

use super::{OutsideRecord, PAIR_WINDOW_SECS};

/// How close (ms) a `checkout:` record must be to a Graft switch's entry to
/// be that switch's. A switch's entry records `HEAD`'s branch before and
/// after but no commit, so the record's (ref, old, new) has nothing to match
/// and time stands in; a minute covers a slow checkout with its set-aside,
/// and the same switch in the shell within a minute of Graft's is the only
/// thing it could mistake.
pub(super) const CHECKOUT_WINDOW_MS: i64 = 60_000;

/// One act: the records that are one row.
#[derive(Clone, Debug)]
pub(super) struct Step {
    /// The record the row stands for (an index into the records).
    pub primary: usize,
    /// Every record of the act, the primary included.
    pub members: Vec<usize>,
    /// How many `HEAD` records folded into it.
    pub folded: usize,
    /// A rebase's upstream, from its `rebase (start): checkout <x>` record.
    pub onto: Option<String>,
}

/// The records grouped into steps, and which branch `HEAD` was on at each.
#[derive(Clone, Debug)]
pub(super) struct Steps {
    /// Newest first.
    pub steps: Vec<Step>,
    /// Per record: the branch, short name, it moved (a branch record's own;
    /// for a `HEAD` record, the branch `HEAD` was on, where the logs say).
    pub on: Vec<Option<String>>,
}

/// The steps, newest first.
pub(super) fn steps(outside: &[OutsideRecord]) -> Steps {
    let records: Vec<&ReflogRecord> = outside.iter().map(|each| &each.record).collect();
    let mut taken = vec![false; records.len()];
    let mut steps = fold_rebases(&records, &mut taken);
    for (index, record) in records.iter().enumerate() {
        if !taken[index] && record.old == record.new && !is_checkout(&record.subject) {
            taken[index] = true;
        }
    }
    steps.extend(pair_head_records(&records, &mut taken));
    for (index, _) in records.iter().enumerate() {
        if !taken[index] {
            steps.push(Step {
                primary: index,
                members: vec![index],
                folded: 0,
                onto: None,
            });
        }
    }
    steps.sort_by_key(|step| step.primary);
    let on = head_branches(&records, &steps);
    Steps { steps, on }
}

fn is_checkout(subject: &str) -> bool {
    subject.starts_with("checkout: moving from ")
}

pub(super) fn is_rebase(subject: &str) -> bool {
    subject.starts_with("rebase (") || subject.starts_with("rebase -i (")
}

/// `rebase (finish): refs/heads/<b> onto …` → `refs/heads/<b>`.
fn rebase_finish_branch(subject: &str) -> Option<&str> {
    let rest = subject
        .strip_prefix("rebase (finish): ")
        .or_else(|| subject.strip_prefix("rebase -i (finish): "))?;
    let name = rest.split(' ').next()?;
    name.starts_with("refs/heads/").then_some(name)
}

fn close_in_time(left: &ReflogRecord, right: &ReflogRecord) -> bool {
    (left.at - right.at).abs() <= PAIR_WINDOW_SECS
}

/// Step 1: each run of `HEAD` rebase records, with its branch's finish.
fn fold_rebases(records: &[&ReflogRecord], taken: &mut [bool]) -> Vec<Step> {
    let head: Vec<usize> = (0..records.len())
        .filter(|index| records[*index].refname == "HEAD")
        .collect();
    let mut steps = Vec::new();
    let mut at = 0;
    while at < head.len() {
        if !is_rebase(&records[head[at]].subject) {
            at += 1;
            continue;
        }
        let mut end = at;
        while end + 1 < head.len() && is_rebase(&records[head[end + 1]].subject) {
            end += 1;
        }
        let run: Vec<usize> = head[at..=end].to_vec();
        steps.push(fold_one(records, taken, &run));
        at = end + 1;
    }
    steps
}

/// One rebase run (newest first) into one step.
fn fold_one(records: &[&ReflogRecord], taken: &mut [bool], run: &[usize]) -> Step {
    let newest = run[0];
    let finish = (0..records.len())
        .filter(|index| {
            !taken[*index]
                && rebase_finish_branch(&records[*index].subject).is_some()
                && records[*index].refname.starts_with("refs/heads/")
                && close_in_time(records[*index], records[newest])
        })
        .min_by_key(|index| index.abs_diff(newest));
    let onto = run.last().and_then(|oldest| {
        let subject = &records[*oldest].subject;
        subject
            .strip_prefix("rebase (start): checkout ")
            .or_else(|| subject.strip_prefix("rebase -i (start): checkout "))
            .map(str::to_owned)
    });
    for index in run {
        taken[*index] = true;
    }
    let mut members = run.to_vec();
    let (primary, folded) = match finish {
        Some(branch) => {
            taken[branch] = true;
            members.push(branch);
            (branch, run.len())
        }
        None => (newest, run.len() - 1),
    };
    Step {
        primary,
        members,
        folded,
        onto,
    }
}

/// Step 3: each `HEAD` record with the branch record that is its twin.
fn pair_head_records(records: &[&ReflogRecord], taken: &mut [bool]) -> Vec<Step> {
    let mut steps = Vec::new();
    for head in 0..records.len() {
        if taken[head] || records[head].refname != "HEAD" {
            continue;
        }
        let twin = (0..records.len())
            .filter(|index| {
                let branch = records[*index];
                !taken[*index]
                    && branch.refname.starts_with("refs/heads/")
                    && branch.old == records[head].old
                    && branch.new == records[head].new
                    && close_in_time(branch, records[head])
            })
            .min_by_key(|index| index.abs_diff(head));
        if let Some(branch) = twin {
            taken[head] = true;
            taken[branch] = true;
            steps.push(Step {
                primary: branch,
                members: vec![branch, head],
                folded: 0,
                onto: None,
            });
        }
    }
    steps
}

/// The short name of `refs/heads/<b>`, else `None`.
pub(super) fn branch_short(refname: &str) -> Option<&str> {
    refname.strip_prefix("refs/heads/")
}

/// git's name for where `HEAD` went in a `checkout:` subject is what the
/// user typed; a full hex id there is a detached `HEAD`, not a branch.
pub(super) fn is_hex_id(name: &str) -> bool {
    (name.len() == 40 || name.len() == 64) && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// `checkout: moving from <a> to <b>` → `(a, b)`.
pub(super) fn checkout_names(subject: &str) -> Option<(&str, &str)> {
    subject
        .strip_prefix("checkout: moving from ")?
        .split_once(" to ")
}

/// Which branch each record moved: a branch record its own; a `HEAD` record
/// the branch `HEAD` was on — its twin's, or the newest older switch's
/// target, or a rebase's `returning to` branch. `None` where the logs do not
/// say (detached, or older than the records read).
fn head_branches(records: &[&ReflogRecord], steps: &[Step]) -> Vec<Option<String>> {
    let mut on: Vec<Option<String>> = records
        .iter()
        .map(|record| branch_short(&record.refname).map(str::to_owned))
        .collect();
    let mut twin_of: BTreeMap<usize, usize> = BTreeMap::new();
    for step in steps {
        if records[step.primary].refname != "HEAD" {
            for member in &step.members {
                twin_of.insert(*member, step.primary);
            }
        }
    }
    let next_leave = next_switch_from(records);
    let mut current: Option<String> = None;
    for index in (0..records.len()).rev() {
        let record = records[index];
        if record.refname != "HEAD" {
            continue;
        }
        if let Some(twin) = twin_of.get(&index) {
            current = branch_short(&records[*twin].refname).map(str::to_owned);
        }
        on[index].clone_from(&current);
        current = after_head_record(&record.subject, current, next_leave[index]);
    }
    on
}

/// Per record, the `from` of the next newer `checkout:` record in `HEAD`'s
/// log: how git named where `HEAD` was when it next left. Records are newest
/// first.
fn next_switch_from<'a>(records: &[&'a ReflogRecord]) -> Vec<Option<&'a str>> {
    let mut next = vec![None; records.len()];
    let mut newer: Option<&'a str> = None;
    for (index, record) in records.iter().enumerate() {
        next[index] = newer;
        if record.refname == "HEAD"
            && let Some((from, _)) = checkout_names(&record.subject)
        {
            newer = Some(from);
        }
    }
    next
}

/// Where `HEAD` is after a record with this subject. `next_leave` is the
/// `from` of the next switch away ([`next_switch_from`]).
fn after_head_record(
    subject: &str,
    current: Option<String>,
    next_leave: Option<&str>,
) -> Option<String> {
    if let Some((_, to)) = checkout_names(subject) {
        // `to` is what he typed: `origin/main`, `v1` and `HEAD~3` read like
        // branch names but leave `HEAD` detached (review m7-22, M1). It is a
        // branch only when the next switch leaves it by that name — git
        // writes the hex id when leaving a detached `HEAD`. Otherwise it
        // stays unknown until a branch record pairs with a later `HEAD`
        // record (the twin, above).
        return (!is_hex_id(to) && next_leave == Some(to)).then(|| to.to_owned());
    }
    if let Some(rest) = subject
        .strip_prefix("rebase (finish): returning to ")
        .or_else(|| subject.strip_prefix("rebase -i (finish): returning to "))
    {
        return branch_short(rest).map(str::to_owned);
    }
    if subject.starts_with("rebase (start)") || subject.starts_with("rebase -i (start)") {
        return None;
    }
    current
}

/// Per step, the journal entry (an index into `entries`) it is one of
/// Graft's own moves for, or `None` for the shell's.
///
/// Each (entry, ref) takes at most one step, the nearest in time, so the
/// same move made again in the shell later is still drawn.
pub(super) fn match_entries(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    grouped: &Steps,
) -> Vec<Option<usize>> {
    let mut owner: Vec<Option<usize>> = vec![None; grouped.steps.len()];
    for (entry_index, entry) in entries.iter().enumerate().rev() {
        for (key, before) in &entry.refs_before {
            let Some(after) = entry.refs_after.get(key) else {
                continue;
            };
            if before == after {
                continue;
            }
            let hit = |record: usize| {
                let record_ref = &outside[record].record;
                record_ref.old == *before
                    && record_ref.new == *after
                    && moves_ref(record, key, entry, outside, &grouped.on)
            };
            claim(&mut owner, entry_index, entry, outside, grouped, hit);
        }
        if let Some((from, to)) = switch_names(entry) {
            let hit = |record: usize| {
                let record_ref = &outside[record].record;
                checkout_names(&record_ref.subject) == Some((from.as_str(), to.as_str()))
                    && (record_ref.at.saturating_mul(1000) - entry.at_unix_ms).abs()
                        <= CHECKOUT_WINDOW_MS
            };
            claim(&mut owner, entry_index, entry, outside, grouped, hit);
        }
    }
    owner
}

/// The unclaimed step nearest the entry in time with a record `hit` accepts
/// becomes the entry's.
fn claim(
    owner: &mut [Option<usize>],
    entry_index: usize,
    entry: &JournalEntry,
    outside: &[OutsideRecord],
    grouped: &Steps,
    hit: impl Fn(usize) -> bool,
) {
    let best = grouped
        .steps
        .iter()
        .enumerate()
        .filter(|(index, step)| owner[*index].is_none() && step.members.iter().any(|m| hit(*m)))
        .min_by_key(|(_, step)| {
            (outside[step.primary].record.at.saturating_mul(1000) - entry.at_unix_ms).abs()
        })
        .map(|(index, _)| index);
    if let Some(index) = best {
        owner[index] = Some(entry_index);
    }
}

/// Does the record at `record` move the ref `key` the entry recorded? Its
/// own log's ref, or `HEAD` while on that branch (by the logs, or by the
/// entry's own `HEAD`).
fn moves_ref(
    record: usize,
    key: &str,
    entry: &JournalEntry,
    outside: &[OutsideRecord],
    on: &[Option<String>],
) -> bool {
    let refname = &outside[record].record.refname;
    if refname == key {
        return true;
    }
    if refname != "HEAD" {
        return false;
    }
    let Some(short) = branch_short(key) else {
        return false;
    };
    let head_on = |head: &HeadRef| matches!(head, HeadRef::Branch(full) if full == key);
    on[record].as_deref() == Some(short)
        || head_on(&entry.head_before)
        || entry.head_after.as_ref().is_some_and(head_on)
}

/// A switch's `(from, to)` as git's `checkout:` subject names them: short
/// branch names, a detached `HEAD` as its full hex id.
fn switch_names(entry: &JournalEntry) -> Option<(String, String)> {
    let after = entry.head_after.as_ref()?;
    if *after == entry.head_before {
        return None;
    }
    Some((head_name(&entry.head_before), head_name(after)))
}

fn head_name(head: &HeadRef) -> String {
    match head {
        HeadRef::Branch(full) | HeadRef::Unborn(full) => {
            branch_short(full).unwrap_or(full).to_owned()
        }
        HeadRef::Detached(oid) => oid.to_string(),
    }
}
