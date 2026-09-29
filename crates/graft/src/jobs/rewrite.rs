//! The reads behind `a` into an older commit and absorb (m7-power-tools/09):
//! plan the rewrite, word its preview and its landing, and — once it has
//! landed — find the commits it changed on the new line.
//!
//! Under `jobs/` because it builds [`GitInvocation`]s (the staged numstat,
//! the first-parent read): reads, not writes. The write itself is the
//! ordinary `Operation::Rewrite` the preview confirms.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, Stop};
use graft_git::version::{GitVersion, system_git};
use graft_ops::absorb::{AbsorbMap, StagedHunk, StaysWhy};
use graft_ops::operation::{FactsScope, Refusal, Tip};
use graft_ops::rewrite::{Dependents, RewriteError, RewriteParams, RewriteRequest, TodoAction};

use crate::repos::RepoStore;

/// What planning a put-in came back with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RewritePlanned {
    /// A rewrite to preview.
    Plan(Box<PlannedPutIn>),
    /// `plan` refused, in its own words (`Refusal::explain`).
    Refused(Refusal),
    /// The `HEAD` path with nothing built on his branch: today's amend.
    NothingBuiltOnHead,
}

/// A planned put-in: what `y` runs, and the words around it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedPutIn {
    /// What `y` runs, as `Operation::Rewrite`.
    pub params: RewriteParams,
    /// Each target's first-parent steps below `HEAD`: a put-in keeps the
    /// commit count on his line, so the same steps below the new `HEAD`
    /// name the changed commit (the row tag's read).
    pub offsets: Vec<u32>,
    /// The status line once it lands.
    pub words: DoneWords,
    /// What the preview box says beyond the plan's own rows.
    pub shown: BoxWords,
}

/// The landing's status line: `<what> ✓ · <also> · u undoes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoneWords {
    /// Before the tick: `put into "Add OAuth callback"`.
    pub what: String,
    /// After it: `1 stays staged`.
    pub also: Option<String>,
}

/// The preview box's own words for a put-in (note R2's two boxes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoxWords {
    /// After `Preview: `.
    pub title: String,
    /// The confirm's verb: `y put it in` / `y put them in`.
    pub confirm: &'static str,
    /// Rows above the plan's own: the `gains` row, or absorb's hunk map.
    pub rows: Vec<String>,
}

/// The confirm verb of `a`'s box.
pub const PUT_IT_IN: &str = "put it in";
/// The confirm verb of absorb's box.
pub const PUT_THEM_IN: &str = "put them in";

/// Plans `request`. `head` is `Some` on the `HEAD` path (`a` on `HEAD`'s
/// row with other local branches about): `rewrite::dependents` is asked
/// first, and nothing built on his branch answers
/// [`RewritePlanned::NothingBuiltOnHead`] — today's amend, no rewrite.
///
/// # Errors
/// A read failed (the words the status line shows).
pub(super) fn plan(
    repos: &RepoStore,
    request: &RewriteRequest,
    head: Option<&Tip>,
    stop: &dyn Stop,
) -> Result<RewritePlanned, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let git = git_version()?;
    let default = repos
        .branches()
        .map_err(|error| error.to_string())?
        .default_branch()
        .cloned();
    if let Some(head) = head {
        let dependents =
            graft_ops::rewrite::dependents(&handle, &git, head, default.as_ref(), stop)
                .map_err(|error| error.explain())?;
        if let Dependents::Line(line) = &dependents
            && line.dependents().is_empty()
        {
            return Ok(RewritePlanned::NothingBuiltOnHead);
        }
    }
    planned(&handle, &git, request, default.as_ref(), stop)
}

/// The system git's version, kept from the first successful read in this
/// process. `git --version` was a spawn on every `a`, ahead of the
/// dependents check (09b: 14.5 ms of the check's 30.2 ms on a loaded
/// machine); the version does not change under a running Graft, as blame's
/// `SYSTEM_VERSION` already assumes (graft-git `blame/marks_pass.rs`).
static SYSTEM_VERSION: OnceLock<GitVersion> = OnceLock::new();

#[cfg(test)]
thread_local! {
    /// How many times [`git_version`] spawned `git --version` on this thread.
    static VERSION_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// [`SYSTEM_VERSION`], read on first use. A failed read is said and not
/// kept: the next plan asks again.
fn git_version() -> Result<GitVersion, String> {
    if let Some(version) = SYSTEM_VERSION.get() {
        return Ok(*version);
    }
    #[cfg(test)]
    VERSION_READS.with(|reads| reads.set(reads.get() + 1));
    let version = system_git().map_err(|error| error.to_string())?.version();
    // `Err` means another plan stored first; it read the same `git
    // --version` in the same process, so nothing is lost by dropping ours.
    let _stored_first_elsewhere = SYSTEM_VERSION.set(version);
    Ok(version)
}

/// `rewrite::plan`, then the words.
fn planned(
    handle: &RepoHandle,
    git: &GitVersion,
    request: &RewriteRequest,
    default: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewritePlanned, String> {
    let facts = graft_ops::facts::read_facts(handle, *git, stop, &FactsScope::default())
        .map_err(|error| error.explain())?;
    let params = match graft_ops::rewrite::plan(handle, git, request, &facts, default, stop) {
        Ok(params) => params,
        Err(RewriteError::Refused(refusal)) => return Ok(RewritePlanned::Refused(refusal)),
        Err(RewriteError::Facts(error)) => return Err(error.explain()),
        Err(RewriteError::Stopped) => return Err("stopped".to_owned()),
    };
    let subjects: BTreeMap<Oid, &str> = params
        .todo
        .iter()
        .map(|row| (row.commit, row.subject.as_str()))
        .collect();
    let (words, shown) = match request {
        RewriteRequest::AmendInto { target } => {
            // `plan` keeps the target's own row (it only adds a fixup below
            // it), so a missing one is a planner bug: said, not drawn as an
            // empty subject.
            let subject = subjects
                .get(target)
                .copied()
                .ok_or_else(|| format!("the plan has no row for {}", short(*target)))?;
            let gains = staged_numstat(handle, stop)?;
            amend_into_words(*target, subject, &gains)
        }
        RewriteRequest::Absorb => {
            let map = match graft_ops::rewrite::absorb_plan_map(handle, &facts, default, stop) {
                Ok(map) => map,
                Err(RewriteError::Refused(refusal)) => {
                    return Ok(RewritePlanned::Refused(refusal));
                }
                Err(RewriteError::Facts(error)) => return Err(error.explain()),
                Err(RewriteError::Stopped) => return Err("stopped".to_owned()),
            };
            absorb_words(&map, &subjects)
        }
        // `R`'s editor plans its own; it never reaches this job.
        RewriteRequest::Open { .. } | RewriteRequest::FromRebase(_) => {
            return Err("R is not a put-in".to_owned());
        }
    };
    let offsets = offsets(&params);
    Ok(RewritePlanned::Plan(Box::new(PlannedPutIn {
        params,
        offsets,
        words,
        shown,
    })))
}

/// Each target's first-parent steps below `HEAD`, oldest target first: a
/// target is a row that keeps its commit with one of `made`'s fixups right
/// after it; its steps are the kept rows above it. A target the plan drops
/// (its pieces undo it, 06b) keeps no row, so it has no step and no tag.
pub(crate) fn offsets(params: &RewriteParams) -> Vec<u32> {
    let todo = &params.todo;
    let mut offsets = Vec::new();
    for (index, row) in todo.iter().enumerate() {
        let gains = todo.get(index + 1).is_some_and(|next| {
            next.action == TodoAction::Fixup && params.made.contains(&next.commit)
        });
        if row.action.keeps() && gains {
            let above = todo[index + 1..]
                .iter()
                .filter(|later| later.action.keeps())
                .count();
            offsets.push(u32::try_from(above).unwrap_or(u32::MAX));
        }
    }
    offsets
}

/// A staged file's line counts, as `git diff --cached --numstat` says them;
/// `None` for a binary file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Gain {
    pub path: String,
    pub counts: Option<(u64, u64)>,
}

/// `git diff --cached --numstat -z --no-renames`: every staged file and its
/// added and removed lines.
fn staged_numstat(handle: &RepoHandle, stop: &dyn Stop) -> Result<Vec<Gain>, String> {
    let output = GitInvocation::new(handle)
        .args([
            GitArg::Flag("diff"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--numstat"),
            GitArg::Flag("-z"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag("--no-ext-diff"),
        ])
        .run(stop)
        .map_err(|error| error.to_string())?;
    Ok(parse_numstat(&output.stdout))
}

/// `<added>\t<removed>\t<path>\0` records; `-` counts are a binary file.
pub(crate) fn parse_numstat(stdout: &[u8]) -> Vec<Gain> {
    stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .filter_map(|record| {
            let text = String::from_utf8_lossy(record);
            let mut parts = text.splitn(3, '\t');
            let added = parts.next()?;
            let removed = parts.next()?;
            let path = parts.next()?.to_owned();
            let counts = added.parse().ok().zip(removed.parse().ok());
            Some(Gain { path, counts })
        })
        .collect()
}

/// `a`'s box: the title naming the target, and the `gains` row.
pub(crate) fn amend_into_words(
    target: Oid,
    subject: &str,
    gains: &[Gain],
) -> (DoneWords, BoxWords) {
    let short = short(target);
    let files: Vec<String> = gains.iter().map(gain_words).collect();
    let mut row = format!("{short}  {subject}  gains");
    if !files.is_empty() {
        row = format!("{row}  {}", files.join(" \u{b7} "));
    }
    (
        DoneWords {
            what: format!("put into \"{subject}\""),
            also: None,
        },
        BoxWords {
            title: format!("put your staged changes into {short} \"{subject}\""),
            confirm: PUT_IT_IN,
            rows: vec![row],
        },
    )
}

/// `src/auth.rs +4 −1`; `+0`/`−0` left out; a binary file by its path alone.
fn gain_words(gain: &Gain) -> String {
    let Some((added, removed)) = gain.counts else {
        return gain.path.clone();
    };
    let mut words = gain.path.clone();
    if added > 0 {
        words.push_str(&format!(" +{added}"));
    }
    if removed > 0 {
        words.push_str(&format!(" \u{2212}{removed}"));
    }
    words
}

/// `path  lines  what` rows, the first two columns padded to their widest.
fn aligned(all: &[(String, String, String)]) -> Vec<String> {
    // An empty map has no widest cell: no rows, so no width is used.
    let path_width = all
        .iter()
        .map(|row| row.0.chars().count())
        .max()
        .unwrap_or(0);
    let lines_width = all
        .iter()
        .map(|row| row.1.chars().count())
        .max()
        .unwrap_or(0);
    all.iter()
        .map(|(path, lines, what)| {
            format!("{path:<path_width$}  {lines:<lines_width$}  {what}")
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// Absorb's box: one row per hunk — `→ <sha>  <subject>` or `stays staged ·
/// <why>` — in the map's order, placed first; the landing's counts.
pub(crate) fn absorb_words(
    map: &AbsorbMap,
    subjects: &BTreeMap<Oid, &str>,
) -> (DoneWords, BoxWords) {
    let placed: Vec<(String, String, String)> = map
        .placed
        .iter()
        .map(|(hunk, target)| {
            // Empty only if the map names a commit the plan has no row for;
            // the row still names it by its sha, so nothing is hidden.
            let subject = subjects.get(target).copied().unwrap_or_default();
            (
                hunk.path.display().to_string(),
                lines_words(hunk),
                format!("\u{2192} {}  {subject}", short(*target)),
            )
        })
        .collect();
    let stays: Vec<(String, String, String)> = map
        .stays
        .iter()
        .map(|(hunk, why)| {
            (
                hunk.path.display().to_string(),
                lines_words(hunk),
                format!("stays staged \u{b7} {}", stays_words(*why)),
            )
        })
        .collect();
    let rows = aligned(&placed.into_iter().chain(stays).collect::<Vec<_>>());
    let hunks = map.placed.len();
    let mut targets: Vec<Oid> = map.placed.iter().map(|(_, target)| *target).collect();
    targets.sort_unstable();
    targets.dedup();
    let staying = map.stays.len();
    (
        DoneWords {
            what: format!(
                "put {hunks} {} into {} {}",
                plural(hunks, "hunk", "hunks"),
                targets.len(),
                plural(targets.len(), "commit", "commits")
            ),
            also: (staying > 0)
                .then(|| format!("{staying} {} staged", plural(staying, "stays", "stay"))),
        },
        BoxWords {
            title: "put staged changes into the commits they belong to".to_owned(),
            confirm: PUT_THEM_IN,
            rows,
        },
    )
}

fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

/// `lines 40–44`, `line 3`; a hunk that only removes lines is named by the
/// lines it removes from `HEAD`'s file.
fn lines_words(hunk: &StagedHunk) -> String {
    let range = if hunk.new.is_empty() {
        &hunk.old
    } else {
        &hunk.new
    };
    let last = range.end.saturating_sub(1).max(range.start);
    if last == range.start {
        format!("line {}", range.start)
    } else {
        format!("lines {}\u{2013}{last}", range.start)
    }
}

/// The words after `stays staged ·`, one per [`StaysWhy`] (note R2; 06c and
/// 06e carries).
pub(crate) fn stays_words(why: StaysWhy) -> &'static str {
    match why {
        StaysWhy::NoCommitChangedIt => "no commit on your branch changed these lines",
        StaysWhy::TwoCommitsChangedIt => "more than one commit changed these lines",
        StaysWhy::NewFile => "a new file · no commit has it yet",
        StaysWhy::BeforeYourBranch => "these lines are older than your branch",
        StaysWhy::Renamed => "a renamed file stays whole",
        StaysWhy::Submodule => "a submodule change has no lines to put in",
        StaysWhy::OutsideYourCommits => "these lines are older than the commits absorb reaches",
        StaysWhy::AlreadyPushed => "the commit that changed these lines is already pushed",
    }
}

/// The first seven hex digits, as every row names a commit.
fn short(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

/// After a put-in landed: the commit `offset` first-parent steps below
/// `HEAD`, for each of `offsets` — one `rev-list` read, as deep as the
/// deepest.
///
/// # Errors
/// The read failed, or the line is shorter than an offset (the landing
/// moved something the plan did not).
pub(super) fn resolve_row_tags(
    repos: &RepoStore,
    offsets: &[u32],
    stop: &dyn Stop,
) -> Result<Vec<Oid>, String> {
    let Some(deepest) = offsets.iter().max() else {
        return Ok(Vec::new());
    };
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let output = GitInvocation::new(&handle)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--first-parent"),
            GitArg::Count("--max-count=", deepest.saturating_add(1)),
            GitArg::Flag("HEAD"),
        ])
        .run(stop)
        .map_err(|error| error.to_string())?;
    let line: Vec<Oid> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|hex| Oid::from_hex(hex.trim()).map_err(|error| format!("{hex}: {error}")))
        .collect::<Result<_, _>>()?;
    offsets
        .iter()
        .map(|offset| {
            usize::try_from(*offset)
                .ok()
                .and_then(|at| line.get(at).copied())
                .ok_or_else(|| format!("HEAD's line is shorter than {offset} commits"))
        })
        .collect()
}

#[cfg(test)]
mod tests;
