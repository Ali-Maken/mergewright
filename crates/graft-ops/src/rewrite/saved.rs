//! The plan a rewrite runs, saved under `<git dir>/graft/rewrite/<entry-id>/`
//! **before** the journal's `Pending` line and removed when the entry closes
//! (ADR-0047 §1). A relaunch mid-rewrite reads it to continue, to name the
//! branch of the step in the banner, and to go home after an abort.
//!
//! What is in it:
//! - `plan.json` — his branch, the line of branches with their tips, `onto`.
//! - `todo` — the todo git runs; `GIT_SEQUENCE_EDITOR` copies it over git's.
//! - `messages/<full id>` — the message for the commit git asks on.
//! - `editor` — the `GIT_EDITOR` helper: it reads the last line of
//!   `rebase-merge/done`, and answers with that commit's message if one was
//!   written, otherwise with what git offered minus its comment lines — the
//!   same text git's default cleanup would have left.
//!
//! rewrite-engine-a wrote this with the format [`super::progress`] reads;
//! the writing half is unused until `apply::engine` lands in -b.
#![expect(
    dead_code,
    reason = "the run half: `apply::engine` wires it in rewrite-engine-b"
)]

use std::io;
use std::path::{Path, PathBuf};

use graft_core::journal::EntryId;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use serde::{Deserialize, Serialize};

use crate::operation::Tip;

use super::BranchLine;
use super::todo::GitTodo;

/// `commit.cleanup=whitespace` for every commit git makes in the rewrite,
/// through `GIT_CONFIG_COUNT` (git ≥ 2.31): a message Graft hands over is
/// kept as written, `#123 fixes login` included. git's default for an
/// edited message strips `#` lines; where no message was written, the
/// helper strips git's comments itself (`git stripspace -s`, which honours
/// `core.commentChar`).
pub(crate) const CLEANUP: [(&str, &str); 3] = [
    ("GIT_CONFIG_COUNT", "1"),
    ("GIT_CONFIG_KEY_0", "commit.cleanup"),
    ("GIT_CONFIG_VALUE_0", "whitespace"),
];

/// The saved plan's own facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedPlan {
    /// His branch, full name: where `HEAD` goes back to.
    pub(crate) home: String,
    /// Every branch that moves, bottom first, with its tip before.
    pub(crate) line: Vec<(String, String)>,
    /// Where the first kept row is replayed.
    pub(crate) onto: String,
}

impl SavedPlan {
    pub(crate) fn new(home: &str, line: &BranchLine, onto: Oid) -> Self {
        SavedPlan {
            home: home.to_owned(),
            line: line
                .branches
                .iter()
                .map(|tip| (tip.name.clone(), tip.oid.to_string()))
                .collect(),
            onto: onto.to_string(),
        }
    }

    /// The branch git runs on: the top of the line.
    pub(crate) fn top(&self) -> &str {
        self.line
            .last()
            .map_or(self.home.as_str(), |(name, _)| name)
    }

    /// The line as tips.
    ///
    /// # Errors
    /// A tip that is not a commit id: only a hand-edited file holds one, and
    /// a relaunch must not guess which branch it meant.
    pub(crate) fn tips(&self) -> io::Result<Vec<Tip>> {
        self.line
            .iter()
            .map(|(name, oid)| {
                Oid::from_hex(oid)
                    .map(|oid| Tip {
                        name: name.clone(),
                        oid,
                    })
                    .map_err(|_| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("plan.json: {name}'s tip {oid:?} is not a commit id"),
                        )
                    })
            })
            .collect()
    }
}

/// Where every rewrite's plan lives.
pub(crate) fn root(repo: &RepoHandle) -> PathBuf {
    repo.git_dir().join("graft").join("rewrite")
}

/// This entry's plan directory.
pub(crate) fn dir(repo: &RepoHandle, id: &EntryId) -> PathBuf {
    root(repo).join(id.as_str())
}

/// Write the plan, the todo, the messages and the helper.
pub(crate) fn save(
    repo: &RepoHandle,
    dir: &Path,
    plan: &SavedPlan,
    todo: &GitTodo,
) -> io::Result<()> {
    let messages = dir.join("messages");
    std::fs::create_dir_all(&messages)?;
    std::fs::write(dir.join("todo"), &todo.text)?;
    for (commit, message) in &todo.messages {
        std::fs::write(messages.join(commit.to_string()), message)?;
    }
    let json = serde_json::to_vec(plan).map_err(io::Error::other)?;
    std::fs::write(dir.join("plan.json"), json)?;
    std::fs::write(dir.join("editor"), helper(repo, dir))?;
    Ok(())
}

/// The plan saved in `dir`; `None` when there is none.
pub(crate) fn read(dir: &Path) -> io::Result<Option<SavedPlan>> {
    let bytes = match std::fs::read(dir.join("plan.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}

/// Every plan saved in this worktree.
pub(crate) fn all(repo: &RepoHandle) -> io::Result<Vec<SavedPlan>> {
    let entries = match std::fs::read_dir(root(repo)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut plans = Vec::new();
    for entry in entries {
        if let Some(plan) = read(&entry?.path())? {
            plans.push(plan);
        }
    }
    Ok(plans)
}

/// Remove the plan: the entry has closed. Gone already is fine.
pub(crate) fn remove(dir: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(dir) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// `GIT_SEQUENCE_EDITOR`: git runs it through the shell with the todo's path
/// as its argument.
pub(crate) fn sequence_editor(dir: &Path) -> String {
    format!("cp {}", quoted(&dir.join("todo")))
}

/// `GIT_EDITOR`: the helper, run through the shell with the message file's
/// path as its argument.
pub(crate) fn editor(dir: &Path) -> String {
    format!("sh {}", quoted(&dir.join("editor")))
}

/// The helper script. Paths are written in whole, so it needs nothing from
/// its environment but `git`.
fn helper(repo: &RepoHandle, dir: &Path) -> String {
    let done = repo.git_dir().join("rebase-merge").join("done");
    format!(
        "# Graft's message helper for one rewrite (ADR-0047 §1). git asks for\n\
         # a message; the one written before y is on file under the commit git\n\
         # is on, the last line of rebase-merge/done.\n\
         done={done}\n\
         messages={messages}\n\
         id=$(sed -n '$p' \"$done\" | cut -d' ' -f2)\n\
         if [ -n \"$id\" ] && [ -f \"$messages/$id\" ]; then\n\
         \tcat \"$messages/$id\" > \"$1\"\n\
         else\n\
         \tgit stripspace --strip-comments < \"$1\" > \"$1.graft\" && mv \"$1.graft\" \"$1\"\n\
         fi\n",
        done = quoted(&done),
        messages = quoted(&dir.join("messages")),
    )
}

/// A path in single quotes for `sh`, any `'` in it closed, escaped and
/// reopened.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}
