//! A finished rebase's in-between commits (m7/38).
//!
//! git writes a `HEAD` record for each step of a rebase: a pick's copy, each
//! squash's interim "# This is a combination of 2 commits.", the final
//! result. Once the rebase finishes, the interim ones are in the result
//! (pick) or melded into it (squash, fixup, reword): nothing holds them, yet
//! they are not his lost work. The branch's old tip is not among them — the
//! `(start)` record and the branch's own `(finish): … onto` record name it.
//!
//! The rule reads the **raw** records (before `steps` drops any or an entry
//! claims any): a commit is scratch when every record naming it, as old or
//! new, in any log, is a step record of a **finished** span of `HEAD`'s log.
//! An aborted or unfinished span is never scratch: it may hold his
//! resolution work.

use std::collections::HashSet;

use mergewright_core::oid::Oid;
use mergewright_git::reflog::ReflogRecord;

use super::steps::is_rebase;

/// The commits named only by step records of finished rebases in `HEAD`'s
/// log. `records` newest first, as `read_outside` gives them.
///
/// A span opens at `rebase (start)` (or `rebase -i (start)`) and closes at
/// `(finish)` (finished), `(abort)`, the next `(start)` or the log's end
/// (unfinished). Other records inside a span (a `commit` or `reset` during
/// an `edit` stop) don't close it, and are not step records. One pass, no
/// read.
pub(super) fn rebase_scratch<'a>(
    records: impl IntoIterator<Item = &'a ReflogRecord>,
) -> HashSet<Oid> {
    let records: Vec<&ReflogRecord> = records.into_iter().collect();
    let step = finished_steps(&records);
    let mut by_step = HashSet::new();
    let mut by_other = HashSet::new();
    for (index, record) in records.iter().enumerate() {
        let named = if step[index] {
            &mut by_step
        } else {
            &mut by_other
        };
        named.extend(record.old);
        named.extend(record.new);
    }
    by_step.retain(|oid| !by_other.contains(oid));
    by_step
}

/// Per record, whether it is a step record of a finished span of `HEAD`'s
/// log. Walks `HEAD`'s records oldest first.
fn finished_steps(records: &[&ReflogRecord]) -> Vec<bool> {
    let mut step = vec![false; records.len()];
    // The open span's step records; `None` outside a span.
    let mut open: Option<Vec<usize>> = None;
    for index in (0..records.len()).rev() {
        let record = records[index];
        if record.refname != "HEAD" {
            continue;
        }
        match rebase_step(&record.subject) {
            // A new start closes an open span unfinished: drop it.
            Some("start") => open = Some(Vec::new()),
            Some("finish") => {
                for member in open.take().into_iter().flatten() {
                    step[member] = true;
                }
            }
            Some("abort") => open = None,
            Some(_) => {
                if let Some(span) = open.as_mut() {
                    span.push(index);
                }
            }
            // Not a rebase record: inside a span (an `edit` stop's commit or
            // reset) it neither closes the span nor is a step.
            None => {}
        }
    }
    step
}

/// `rebase (<step>): …` or `rebase -i (<step>): …` → `<step>`. git's
/// unparenthesized `rebase: fast-forward` is no step here: a fast-forward
/// lands on a commit he made, never on an interim one.
fn rebase_step(subject: &str) -> Option<&str> {
    if !is_rebase(subject) {
        return None;
    }
    let (_, rest) = subject.split_once('(')?;
    let (step, _) = rest.split_once(')')?;
    Some(step)
}
