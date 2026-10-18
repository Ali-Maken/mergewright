//! Repository truth on both sides — the work clone, the bare's refs, the
//! journal, every status line the loop drew and the remote's own counters —
//! read by running git, never from `AppState`; and the marks a run takes.

use std::collections::BTreeMap;
use std::path::Path;

use mergewright_core::journal::JournalEntry;
use mergewright_fixtures::remote::{HttpRemote, HungRemote};

use super::fixture::Fixture;
use crate::repo_truth::{RepoTruth, diff_truth as diff_work, git_truth, run_git};
use mergewright_core::on_disk::CURRENT_DIR;

/// Repository truth on both sides, plus what the loop drew, entirely from
/// git, the file system and the remote's own counters.
#[derive(Clone, Debug, Default)]
pub struct Truth {
    /// The work clone (`m3_walk::git_truth`).
    pub work: RepoTruth,
    /// The bare's `for-each-ref`: name → object.
    pub bare: BTreeMap<String, String>,
    /// The work clone's undo journal, as written; empty before any write.
    pub journal: Vec<JournalEntry>,
    /// Every distinct status line the loop drew so far, in order.
    pub statuses: Vec<String>,
    /// The HTTP remote's `(authorised, rejected)` request counts.
    pub http: Option<(usize, usize)>,
    /// The hung remote's connection count.
    pub hung_connections: Option<usize>,
}

/// Read both repositories now.
pub fn truth(fixture: &Fixture, statuses: &[String]) -> Result<Truth, String> {
    let work = git_truth(fixture.work())?;
    let bare = run_git(
        fixture.bare(),
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    )?
    .lines()
    .filter_map(|line| {
        let (name, oid) = line.split_once(' ')?;
        Some((name.to_owned(), oid.to_owned()))
    })
    .collect();
    Ok(Truth {
        work,
        bare,
        journal: journal(fixture.work())?,
        statuses: statuses.to_vec(),
        http: fixture.http.as_ref().map(HttpRemote::requests),
        hung_connections: fixture.hung.as_ref().map(HungRemote::connections),
    })
}

/// The journal's entries, or none when no write has happened yet.
pub fn journal(work: &Path) -> Result<Vec<JournalEntry>, String> {
    let path = work.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    if !path.exists() {
        return Ok(Vec::new());
    }
    mergewright_ops::journal::Journal::open(&path)
        .map(|journal| journal.entries().to_vec())
        .map_err(|why| format!("the journal: {why}"))
}

/// What moved between two truths, 20 lines at most: the work clone's
/// differences first (`m3_walk::diff_truth`), then the bare's refs, then
/// the journal's length.
#[must_use]
pub fn diff_truth(before: &Truth, after: &Truth) -> Vec<String> {
    let mut out = diff_work(&before.work, &after.work);
    for (name, was) in &before.bare {
        match after.bare.get(name) {
            Some(now) if now != was => out.push(format!("bare {name}: {was} → {now}")),
            Some(_) => {}
            None => out.push(format!("bare {name}: gone (was {was})")),
        }
    }
    for (name, now) in &after.bare {
        if !before.bare.contains_key(name) {
            out.push(format!("bare {name}: new {now}"));
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

/// The truths a run recorded: after setup, and at every `Mark`.
#[derive(Default)]
pub struct Marks {
    /// After `setup`, before the app opened.
    pub before: Truth,
    /// At each `Step::Mark`.
    pub at: BTreeMap<&'static str, Truth>,
}

impl Marks {
    /// The truth recorded under `name`.
    pub fn get(&self, name: &str) -> Result<&Truth, String> {
        self.at
            .get(name)
            .ok_or_else(|| format!("no mark named {name}"))
    }
}
