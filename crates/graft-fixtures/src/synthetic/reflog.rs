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
const LOGS: [(&str, usize); 4] = [
    ("HEAD", 400),
    ("refs/heads/main", 200),
    ("refs/heads/reflog-a", 200),
    ("refs/heads/reflog-b", 200),
];

/// One lost commit every this many of `HEAD`'s newest records.
const LOST_EVERY: usize = 10;

/// A private repository with [`RECORDS`] scripted reflog records, deleted
/// when dropped.
#[derive(Debug)]
pub struct Reflogged {
    /// The repository's working-tree root.
    pub scratch: Scratch,
    /// The reflogged refs, `HEAD` first, then the three branches.
    pub logs: Vec<String>,
    /// The [`LOST`] commits nothing holds, as hex ids.
    pub lost: Vec<String>,
}

/// Builds [`Reflogged`] over `scale`'s repository (built first if needed).
///
/// # Errors
/// [`SyntheticError`] if the synthetic repository cannot be had, a `git`
/// command fails, or a file cannot be written; `WrongSize` if `main`'s
/// first-parent chain is shorter than the records need.
pub fn with_reflog(scale: Scale) -> Result<Reflogged, SyntheticError> {
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

    let trunk = first_parents(scale, &dir)?;
    let tip = trunk[0].clone();
    for branch in ["reflog-a", "reflog-b"] {
        let path = git_dir.join("refs/heads").join(branch);
        std::fs::write(&path, format!("{tip}\n")).map_err(io_error(scale, &path))?;
    }

    let lost = lost_commits(scale, &dir, &trunk)?;
    let mut held = trunk.iter();
    for (index, (name, records)) in LOGS.iter().enumerate() {
        let mut values = Vec::with_capacity(*records);
        for slot in 0..*records {
            let lost_here =
                index == 0 && slot < LOST * LOST_EVERY && slot % LOST_EVERY == LOST_EVERY - 1;
            let value = if lost_here {
                lost[slot / LOST_EVERY].clone()
            } else {
                held.next()
                    .cloned()
                    .ok_or_else(|| short(scale, trunk.len()))?
            };
            values.push(value);
        }
        let path = git_dir.join("logs").join(name);
        write_log(&path, &tip, &values).map_err(io_error(scale, &path))?;
    }

    Ok(Reflogged {
        scratch,
        logs: LOGS.iter().map(|(name, _)| (*name).to_owned()).collect(),
        lost,
    })
}

/// `main`'s newest first-parent commits, enough for every held record.
fn first_parents(scale: Scale, dir: &Path) -> Result<Vec<String>, SyntheticError> {
    let max_count = format!("--max-count={RECORDS}");
    let listed = git::run(
        dir,
        &["rev-list", "--first-parent", &max_count, "refs/heads/main"],
    )
    .map_err(git_error(scale))?;
    let trunk: Vec<String> = String::from_utf8_lossy(&listed)
        .lines()
        .map(str::to_owned)
        .collect();
    if trunk.len() < RECORDS - LOST {
        return Err(short(scale, trunk.len()));
    }
    Ok(trunk)
}

/// [`LOST`] commits, each on its own trunk commit's tree and parent, with a
/// message of its own so every id differs. Written into this repository's
/// own object store; no ref points at them.
fn lost_commits(scale: Scale, dir: &Path, trunk: &[String]) -> Result<Vec<String>, SyntheticError> {
    let mut lost = Vec::with_capacity(LOST);
    for index in 0..LOST {
        let parent = &trunk[index * LOST_EVERY];
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
            "{old} {new} Graft Fixture <fixture@example.invalid> {when} +0000\tfixture: step {age}\n"
        ));
    }
    std::fs::write(path, text)
}

/// `main`'s first-parent chain holds `got` commits, fewer than the held
/// records need: a scale too small for this fixture, reported as such.
fn short(scale: Scale, got: usize) -> SyntheticError {
    SyntheticError::WrongSize {
        scale: scale.name(),
        want: u32::try_from(RECORDS - LOST).unwrap_or(u32::MAX),
        got: u32::try_from(got).unwrap_or(u32::MAX),
    }
}
