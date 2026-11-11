//! A scripted reflog on top of a synthetic repository, for measuring how
//! long finding lost commits takes (m7-power-tools/19).
//!
//! The synthetic repository itself is shared and sealed, so this builds a
//! private repository beside it that **borrows its objects** through
//! `objects/info/alternates` and copies its refs into `packed-refs`: the
//! same history and the same ref count, for the cost of a ref listing. Into
//! that it writes:
//!
//! - two branches of its own, `refs/heads/reflog-a` and `reflog-b`, at
//!   `main`'s tip, so there are three reflogged branches;
//! - [`RECORDS`] reflog records, across `HEAD` (400), `main`, `reflog-a` and
//!   `reflog-b` (200 each), every old value distinct;
//! - [`LOST`] commits of its own, each on a `main` commit and held by
//!   nothing, as old values among `HEAD`'s newest 200.
//!
//! [`with_reflog_of`] takes the two counts (m7-power-tools/30p: J at its
//! caps, 2 000 records and 500 lost). The records split the same way, `HEAD`
//! two fifths and each branch one fifth; a lost commit is every tenth of
//! `HEAD`'s newest records, or closer when there are too many for that.
//!
//! Every other old value is a commit on `main`'s first-parent chain among
//! its newest thousand: a reflog is recent history, since git expires it
//! after 90 days.

use std::path::Path;

use super::{Scale, SyntheticError, borrow, git_error, io_error};
use crate::git;
use crate::scratch::Scratch;

/// Reflog records written in total.
pub const RECORDS: usize = 1000;
/// Old values held by nothing.
pub const LOST: usize = 20;

/// The reflogged refs, and how many records each gets. `HEAD` first.
const LOGS: Logs = [
    ("HEAD", 400),
    ("refs/heads/main", 200),
    ("refs/heads/reflog-a", 200),
    ("refs/heads/reflog-b", 200),
];

/// Each reflogged ref and how many records it gets, `HEAD` first.
type Logs = [(&'static str, usize); 4];

/// One lost commit every this many of `HEAD`'s newest records, at most.
const LOST_EVERY: usize = 10;

/// A private repository with scripted reflog records ([`RECORDS`] from
/// [`with_reflog`]), deleted when dropped.
#[derive(Debug)]
pub struct Reflogged {
    /// The repository's working-tree root.
    pub scratch: Scratch,
    /// The reflogged refs, `HEAD` first, then the three branches.
    pub logs: Vec<String>,
    /// The lost commits nothing holds ([`LOST`] from [`with_reflog`]), as
    /// hex ids.
    pub lost: Vec<String>,
}

/// Builds [`Reflogged`] over `scale`'s repository (built first if needed),
/// with [`RECORDS`] records and [`LOST`] lost commits.
///
/// # Errors
/// As [`with_reflog_of`].
pub fn with_reflog(scale: Scale) -> Result<Reflogged, SyntheticError> {
    with_reflog_of(scale, RECORDS, LOST)
}

/// Builds [`Reflogged`] over `scale`'s repository (built first if needed):
/// `records` scripted records on `HEAD` and the three branches, `lost` of
/// whose old values nothing holds. `with_reflog(scale)` is
/// `with_reflog_of(scale, RECORDS, LOST)`.
///
/// # Errors
/// [`SyntheticError`] if the synthetic repository cannot be had, a `git`
/// command fails, or a file cannot be written; `WrongSize` if `main`'s
/// first-parent chain is shorter than the records need; `TooManyLost` if
/// `lost` exceeds `HEAD`'s share of the records.
pub fn with_reflog_of(
    scale: Scale,
    records: usize,
    lost: usize,
) -> Result<Reflogged, SyntheticError> {
    let (logs, every) = spread(scale, records, lost)?;
    let source = borrow(scale)?;
    let scratch = Scratch::empty("reflog").map_err(io_error(scale, &std::env::temp_dir()))?;
    let dir = scratch.path().to_path_buf();
    git::init_repo(&dir).map_err(git_error(scale))?;
    let git_dir = dir.join(".git");

    let alternates = git_dir.join("objects/info/alternates");
    let objects = source.join(".git/objects");
    std::fs::write(&alternates, format!("{}\n", objects.display()))
        .map_err(io_error(scale, &alternates))?;

    // `for-each-ref` sorts by name, which is what `packed-refs` promises.
    let listed = git::run(
        &source,
        &["for-each-ref", "--format=%(objectname) %(refname)"],
    )
    .map_err(git_error(scale))?;
    let packed = git_dir.join("packed-refs");
    let mut refs = b"# pack-refs with: sorted\n".to_vec();
    refs.extend_from_slice(&listed);
    std::fs::write(&packed, refs).map_err(io_error(scale, &packed))?;

    let trunk = first_parents(scale, &dir, records, lost)?;
    let tip = trunk[0].clone();
    for branch in ["reflog-a", "reflog-b"] {
        let path = git_dir.join("refs/heads").join(branch);
        std::fs::write(&path, format!("{tip}\n")).map_err(io_error(scale, &path))?;
    }

    let lost_ids = lost_commits(scale, &dir, &trunk, lost, every)?;
    let mut held = trunk.iter();
    for (index, (name, count)) in logs.iter().enumerate() {
        let mut values = Vec::with_capacity(*count);
        for slot in 0..*count {
            let lost_here = index == 0 && slot < lost * every && slot % every == every - 1;
            let value = if lost_here {
                lost_ids[slot / every].clone()
            } else {
                held.next()
                    .cloned()
                    .ok_or_else(|| short(scale, records, lost, trunk.len()))?
            };
            values.push(value);
        }
        let path = git_dir.join("logs").join(name);
        write_log(&path, &tip, &values).map_err(io_error(scale, &path))?;
    }

    Ok(Reflogged {
        scratch,
        logs: logs.iter().map(|(name, _)| (*name).to_owned()).collect(),
        lost: lost_ids,
    })
}

/// The logs' record counts ([`split`]), and how far apart the lost
/// commits sit among `HEAD`'s newest records: every tenth, as
/// [`with_reflog`] always has, closer when `HEAD` has too few records for
/// that.
fn spread(scale: Scale, records: usize, lost: usize) -> Result<(Logs, usize), SyntheticError> {
    let logs = split(records);
    let head = logs[0].1;
    if lost > head {
        return Err(SyntheticError::TooManyLost {
            scale: scale.name(),
            lost,
            head,
        });
    }
    // `lost <= head`, so a quotient is at least 1; no lost commits, no gap.
    let every = head
        .checked_div(lost)
        .map_or(LOST_EVERY, |gap| LOST_EVERY.min(gap));
    Ok((logs, every))
}

/// `records` split over [`LOGS`] in its proportions (2 : 1 : 1 : 1), `HEAD`
/// taking what does not divide. `split(RECORDS)` is `LOGS`.
fn split(records: usize) -> Logs {
    let whole: usize = LOGS.iter().map(|(_, count)| count).sum();
    let mut logs = LOGS.map(|(name, count)| (name, records * count / whole));
    let given: usize = logs.iter().map(|(_, count)| count).sum();
    logs[0].1 += records - given;
    logs
}

/// `main`'s newest first-parent commits, enough for every held record.
fn first_parents(
    scale: Scale,
    dir: &Path,
    records: usize,
    lost: usize,
) -> Result<Vec<String>, SyntheticError> {
    let max_count = format!("--max-count={records}");
    let listed = git::run(
        dir,
        &["rev-list", "--first-parent", &max_count, "refs/heads/main"],
    )
    .map_err(git_error(scale))?;
    let trunk: Vec<String> = String::from_utf8_lossy(&listed)
        .lines()
        .map(str::to_owned)
        .collect();
    if trunk.len() < records - lost {
        return Err(short(scale, records, lost, trunk.len()));
    }
    Ok(trunk)
}

/// `count` commits, each on its own trunk commit's tree and parent (the
/// commit `every` records before the slot it fills), with a message of its
/// own so every id differs. Written into this repository's own object
/// store; no ref points at them.
fn lost_commits(
    scale: Scale,
    dir: &Path,
    trunk: &[String],
    count: usize,
    every: usize,
) -> Result<Vec<String>, SyntheticError> {
    let mut lost = Vec::with_capacity(count);
    for index in 0..count {
        let parent = trunk
            .get(index * every)
            .ok_or_else(|| short(scale, trunk.len() + 1, 0, trunk.len()))?;
        let tree = format!("{parent}^{{tree}}");
        let message = format!("lost {index}");
        let id = git::run(dir, &["commit-tree", &tree, "-p", parent, "-m", &message])
            .map_err(git_error(scale))?;
        lost.push(String::from_utf8_lossy(&id).trim().to_owned());
    }
    Ok(lost)
}

/// A reflog whose old values, newest first, are `values`, ending at `tip`.
/// Written oldest first, as git keeps it.
fn write_log(path: &Path, tip: &str, values: &[String]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = String::new();
    let newest_first = values.iter().enumerate().map(|(index, old)| {
        let new = if index == 0 { tip } else { &values[index - 1] };
        (old, new)
    });
    let records: Vec<_> = newest_first.collect();
    for (age, (old, new)) in records.iter().rev().enumerate() {
        let when = 1_700_000_000 + age;
        text.push_str(&format!(
            "{old} {new} {} {when} +0000\tfixture: step {age}\n",
            git::IDENTITY
        ));
    }
    std::fs::write(path, text)
}

/// `main`'s first-parent chain holds `got` commits, fewer than the held
/// records (`records - lost`) need: a scale too small for this fixture,
/// reported as such.
fn short(scale: Scale, records: usize, lost: usize, got: usize) -> SyntheticError {
    SyntheticError::WrongSize {
        scale: scale.name(),
        want: u32::try_from(records - lost).unwrap_or(u32::MAX),
        got: u32::try_from(got).unwrap_or(u32::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `with_reflog`'s split is the one it always wrote (400 / 200 / 200 /
    /// 200), so its output is unchanged.
    #[test]
    fn with_reflog_splits_its_records_as_before() {
        assert_eq!(split(RECORDS), LOGS);
        assert_eq!(split(2_000).map(|(_, count)| count), [800, 400, 400, 400]);
        // What does not divide goes to HEAD.
        assert_eq!(split(1_003).map(|(_, count)| count), [403, 200, 200, 200]);
    }

    /// More lost commits than `HEAD` has records is refused before anything
    /// is built; the gap is every tenth, or closer.
    #[test]
    fn with_reflog_of_refuses_more_lost_than_head_records() {
        let scale = Scale::of(10_000).with_commit_graph();
        assert!(matches!(
            spread(scale, 100, 41),
            Err(SyntheticError::TooManyLost {
                lost: 41,
                head: 40,
                ..
            })
        ));
        assert!(matches!(spread(scale, RECORDS, LOST), Ok((_, LOST_EVERY))));
        assert!(matches!(spread(scale, 2_000, 500), Ok((_, 1))));
    }

    /// m7-power-tools/30p-b: the counts given are the counts written. Heavy
    /// tier (this module is `heavy-tests`): it builds the 10k fixture.
    #[test]
    fn with_reflog_of_counts_what_it_is_given() {
        let scale = Scale::of(10_000).with_commit_graph();
        let (records, lost) = (100, 30);
        let built = with_reflog_of(scale, records, lost).unwrap_or_else(|why| panic!("{why}"));
        let git_dir = built.scratch.path().join(".git");
        let mut written = 0;
        let mut head_olds = Vec::new();
        for (name, count) in split(records) {
            let log = std::fs::read_to_string(git_dir.join("logs").join(name))
                .unwrap_or_else(|why| panic!("{name}'s log: {why}"));
            assert_eq!(log.lines().count(), count, "{name}'s records");
            written += count;
            if name == "HEAD" {
                head_olds = log
                    .lines()
                    .filter_map(|line| line.split(' ').next().map(str::to_owned))
                    .collect();
            }
        }
        assert_eq!(written, records, "every record written");
        assert_eq!(built.lost.len(), lost, "every lost commit made");
        for id in &built.lost {
            assert!(
                head_olds.contains(id),
                "lost {id} is one of HEAD's old values"
            );
        }
    }
}
