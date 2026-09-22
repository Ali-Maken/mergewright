//! Repository truth — refs, `HEAD`, the index, the files, git's own
//! in-progress state, the journal, the bare's refs — read by running git,
//! never from `AppState`; the one claim the walk takes from the app (the
//! preview's plan); and the marks a run takes.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use graft::update::preview::Planned;
use graft_core::journal::JournalEntry;

use super::fixture::Fixture;
use crate::repo_truth::{RepoTruth, diff_truth as diff_repo, git_truth, run_git};

/// Repository truth, entirely from git and the file system.
#[derive(Clone, Debug, Default)]
pub struct Truth {
    /// Refs, `HEAD`, `ls-files -s`, file bytes, stashes, `.git/index` bytes
    /// (`repo_truth::git_truth`).
    pub repo: RepoTruth,
    /// `symbolic-ref HEAD`, or `None` when detached.
    pub head_symbolic: Option<String>,
    /// `write-tree` of the index; `None` while the index holds unmerged
    /// entries, which `write-tree` refuses.
    pub index_tree: Option<String>,
    /// What git says is part-way through: `merge`, `rebase`, `cherry-pick`,
    /// `revert`, from the files under `.git`; `None` when nothing is.
    pub in_progress: Option<String>,
    /// The undo journal, as written; empty before any write.
    pub journal: Vec<JournalEntry>,
    /// The bare's `for-each-ref`, for a remote scenario.
    pub bare: Option<BTreeMap<String, String>>,
    /// Every distinct status line the loop drew so far, in order.
    pub statuses: Vec<String>,
}

/// Read the repository now.
pub fn truth(fixture: &Fixture, statuses: &[String]) -> Result<Truth, String> {
    let repo = fixture.repo();
    let head_symbolic = run_git(repo, &["symbolic-ref", "-q", "HEAD"])
        .ok()
        .map(|out| out.trim().to_owned())
        .filter(|name| !name.is_empty());
    let unmerged = run_git(repo, &["ls-files", "-u"])?;
    let index_tree = if unmerged.trim().is_empty() {
        Some(run_git(repo, &["write-tree"])?.trim().to_owned())
    } else {
        None
    };
    let bare = match fixture.bare() {
        Some(bare) => Some(refs_of(bare)?),
        None => None,
    };
    Ok(Truth {
        repo: git_truth(repo)?,
        head_symbolic,
        index_tree,
        in_progress: in_progress(repo)?,
        journal: journal(repo)?,
        bare,
        statuses: statuses.to_vec(),
    })
}

/// `for-each-ref` as name → object.
pub fn refs_of(repo: &Path) -> Result<BTreeMap<String, String>, String> {
    Ok(
        run_git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"])?
            .lines()
            .filter_map(|line| {
                let (name, oid) = line.split_once(' ')?;
                Some((name.to_owned(), oid.to_owned()))
            })
            .collect(),
    )
}

/// Which of git's own state files is present.
fn in_progress(repo: &Path) -> Result<Option<String>, String> {
    let git_dir = run_git(repo, &["rev-parse", "--git-dir"])?;
    let git_dir = repo.join(git_dir.trim());
    for (marker, word) in [
        ("MERGE_HEAD", "merge"),
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
    ] {
        if git_dir.join(marker).exists() {
            return Ok(Some(word.to_owned()));
        }
    }
    Ok(None)
}

/// The journal's entries, or none when no write has happened yet.
pub fn journal(repo: &Path) -> Result<Vec<JournalEntry>, String> {
    let path = repo.join(".git/graft/undo-journal.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    graft_ops::journal::Journal::open(&path)
        .map(|journal| journal.entries().to_vec())
        .map_err(|why| format!("the journal: {why}"))
}

/// What moved between two truths, 20 lines at most: the repository's
/// differences first, then `HEAD`'s attachment, git's in-progress state,
/// the bare's refs, and the journal's length.
#[must_use]
pub fn diff_truth(before: &Truth, after: &Truth) -> Vec<String> {
    let mut out = diff_repo(&before.repo, &after.repo);
    if before.head_symbolic != after.head_symbolic {
        out.push(format!(
            "HEAD: {:?} → {:?}",
            before.head_symbolic, after.head_symbolic
        ));
    }
    if before.in_progress != after.in_progress {
        out.push(format!(
            "in progress: {:?} → {:?}",
            before.in_progress, after.in_progress
        ));
    }
    if let (Some(was), Some(now)) = (&before.bare, &after.bare) {
        for (name, from) in was {
            match now.get(name) {
                Some(to) if to != from => out.push(format!("bare {name}: {from} → {to}")),
                Some(_) => {}
                None => out.push(format!("bare {name}: gone (was {from})")),
            }
        }
        for (name, to) in now {
            if !was.contains_key(name) {
                out.push(format!("bare {name}: new {to}"));
            }
        }
    }
    if before.journal.len() != after.journal.len() {
        out.push(format!(
            "journal: {} → {} entries",
            before.journal.len(),
            after.journal.len()
        ));
    }
    out.truncate(20);
    out
}

/// The preview's own prediction, taken at [`super::steps::Step::Claim`]:
/// the one thing the walk reads from `AppState`.
#[derive(Clone, Debug)]
pub struct Claim {
    /// The plan behind the box.
    pub plan: Arc<Planned>,
}

/// The truths a run recorded: after setup, at every `Mark`, and the claim.
#[derive(Default)]
pub struct Marks {
    /// After `setup`, before the app opened.
    pub before: Truth,
    /// At each `Step::Mark`.
    pub at: BTreeMap<&'static str, Truth>,
    /// At `Step::Claim`, the most recent.
    pub claim: Option<Claim>,
}

impl Marks {
    /// The truth recorded under `name`.
    pub fn get(&self, name: &str) -> Result<&Truth, String> {
        self.at
            .get(name)
            .ok_or_else(|| format!("no mark named {name}"))
    }

    /// The claim the preview made.
    pub fn claim(&self) -> Result<&Claim, String> {
        self.claim
            .as_ref()
            .ok_or_else(|| "no claim was taken (Step::Claim)".to_owned())
    }
}
