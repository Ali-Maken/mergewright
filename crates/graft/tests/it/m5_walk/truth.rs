//! Repository truth — refs, `HEAD`, the index, the files, git's own
//! in-progress state, the journal, the bare's refs — read by running git,
//! never from `AppState`; the one claim the walk takes from the app (the
//! preview's plan); and the marks a run takes.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use graft::update::preview::Planned;
use graft_core::journal::JournalEntry;
pub use graft_ops::operation::Side;

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
    let path = repo.join(".git/tributary/undo-journal.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    graft_ops::journal::Journal::open(&path)
        .map(|journal| journal.entries().to_vec())
        .map_err(|why| format!("the journal: {why}"))
}

// ───────────────────────────── conflicts (m6-conflicts/07) ─────────────────────────────

/// The working-tree bytes of `path` as the resolution left them: what `⏎`
/// staged, byte for byte. A file the resolution removed is an error naming
/// it, never an empty file.
pub fn resolved_bytes(repo: &Path, path: &str) -> Result<Vec<u8>, String> {
    let file = repo.join(path);
    std::fs::read(&file).map_err(|why| format!("{}: {why}", file.display()))
}

/// What git itself gives for one side of a conflicted `path`: a copy of the
/// whole repository (index stages and all — a `git clone` carries neither)
/// in a scratch directory, `git checkout --ours|--theirs -- <path>` there,
/// and the bytes it wrote. `repo` is never touched; it must still hold the
/// conflict, so call this on a fresh build of the same fixture.
pub fn side_bytes(repo: &Path, path: &str, side: Side) -> Result<Vec<u8>, String> {
    let scratch = tempfile::tempdir().map_err(|why| format!("a scratch directory: {why}"))?;
    let copy = scratch.path().join("repo");
    copy_tree(repo, &copy)?;
    let flag = match side {
        Side::Ours => "--ours",
        Side::Theirs => "--theirs",
    };
    run_git(&copy, &["checkout", flag, "--", path])?;
    resolved_bytes(&copy, path)
}

/// `path`'s unmerged index entries, `git ls-files -u -- <path>`, one per
/// stage: `<mode> <oid> <stage>\t<path>`. Empty once it is resolved.
pub fn unmerged(repo: &Path, path: &str) -> Result<Vec<String>, String> {
    Ok(run_git(repo, &["ls-files", "-u", "--", path])?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Git says an operation is part-way through: `rebase-merge/` or
/// `MERGE_HEAD` under the git directory.
pub fn operation_on_disk(repo: &Path) -> Result<bool, String> {
    let git_dir = run_git(repo, &["rev-parse", "--absolute-git-dir"])?;
    let git_dir = Path::new(git_dir.trim());
    Ok(git_dir.join("rebase-merge").is_dir() || git_dir.join("MERGE_HEAD").is_file())
}

/// The journal file's last line is a `Pending` line with no outcome after
/// it: the raw line says `"status":"pending"`, and the app's own reader
/// agrees that the newest entry is still `Pending`. `false` for a journal
/// that is missing or unreadable — the caller's check names that.
pub fn last_journal_line_is_pending(repo: &Path) -> bool {
    let path = repo.join(".git/tributary/undo-journal.jsonl");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        // Missing or unreadable: there is no Pending line to find, which is
        // exactly the answer `false` gives; the caller's message says so.
        return false;
    };
    let raw_says = raw
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.contains("\"status\":\"pending\""));
    let reader_says = journal(repo).is_ok_and(|entries| {
        entries
            .last()
            .is_some_and(|entry| entry.status == graft_core::journal::EntryStatus::Pending)
    });
    raw_says && reader_says
}

/// Copy a directory tree, files and directories only (the fixtures hold no
/// links a copy must keep).
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|why| format!("{}: {why}", to.display()))?;
    let entries = std::fs::read_dir(from).map_err(|why| format!("{}: {why}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", from.display()))?;
        let kind = entry
            .file_type()
            .map_err(|why| format!("{}: {why}", entry.path().display()))?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)
                .map_err(|why| format!("{}: {why}", entry.path().display()))?;
        } else {
            return Err(format!(
                "{}: not a file or a directory; the copy would lose it",
                entry.path().display()
            ));
        }
    }
    Ok(())
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
