//! m7-power-tools/28a (ADR-0049): a file git doesn't keep, where a write
//! would put one of its own, is refused before the write — in the preview
//! and again at `y` — and never journaled.
//!
//! Each test puts his bytes (`MINE`) in a file git doesn't keep, usually
//! ignored through `.git/info/exclude`, then reads the outcome back from
//! the disk and the journal, never from the value the run returned alone.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod one_shot;
#[cfg(feature = "heavy-tests")]
mod perf;
mod pull;
mod replay;
mod stash;

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_ops::apply::{Applied, ApplyError};
use graft_ops::facts::read_facts;
use graft_ops::operation::{
    CheckoutParams, CheckoutTarget, FastForward, MergeParams, Operation, ResetMode, ResetParams,
    ResetTarget, Source, Tip,
};
use graft_ops::preview::{self, PreviewError, PreviewPlan};

use crate::support::{NOW, Never, TempRepo, git_version, journal_for, try_run};

const MINE: &[u8] = b"MINE\n";

// ── the repository ──────────────────────────────────────────────────────

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("an object id")
}

fn sha7(oid: Oid) -> String {
    oid.to_string()[..7].to_owned()
}

/// `main` holds `a.txt`; `other` adds `c.txt` = `theirs` on top. Checked
/// out on `main`, with `c.txt` ignored and holding his bytes.
fn main_and_other() -> TempRepo {
    let repo = main_and_other_unignored();
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        "",
        "status lists nothing"
    );
    repo
}

/// The same, with no ignore rule and no `c.txt` yet.
fn main_and_other_unignored() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("c.txt", b"theirs\n");
    repo.commit_all("two on other");
    repo.git(&["switch", "-q", "main"]);
    repo
}

fn switch_to(name: &str) -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(format!("refs/heads/{name}")),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    })
}

fn main_tip(repo: &TempRepo) -> Tip {
    Tip {
        name: "refs/heads/main".to_owned(),
        oid: id(repo, "main"),
    }
}

fn merge_other(repo: &TempRepo, ff: FastForward) -> Operation {
    Operation::Merge(MergeParams {
        source: Source::Branch("refs/heads/other".to_owned()),
        into: main_tip(repo),
        source_tip: id(repo, "other"),
        ff,
    })
}

fn reset_main(repo: &TempRepo, mode: ResetMode) -> Operation {
    Operation::Reset(ResetParams {
        target: ResetTarget::Head {
            branch: main_tip(repo),
            mode,
        },
        to: id(repo, "other"),
    })
}

// ── the four checks ─────────────────────────────────────────────────────

/// What one attempt did: the preview, the run, the file's bytes after, and
/// the journal's entry count before and after.
struct Outcome {
    preview: Result<PreviewPlan, PreviewError>,
    applied: Result<Applied, ApplyError>,
    bytes: Vec<u8>,
    entries: (usize, usize),
}

fn entry_count(repo: &TempRepo) -> usize {
    journal_for(&repo.open()).entries().len()
}

fn preview_in(handle: &RepoHandle, op: &Operation) -> Result<PreviewPlan, PreviewError> {
    let facts =
        read_facts(handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    preview::plan(handle, &git_version(), op, &facts, &Never)
}

fn attempt(repo: &TempRepo, op: &Operation, path: &str) -> Outcome {
    let before = entry_count(repo);
    let preview = preview_in(&repo.open(), op);
    let applied = try_run(repo, op, NOW);
    Outcome {
        preview,
        applied,
        bytes: std::fs::read(repo.path().join(path)).unwrap_or_default(),
        entries: (before, entry_count(repo)),
    }
}

fn preview_words(preview: &Result<PreviewPlan, PreviewError>) -> String {
    match preview {
        Err(PreviewError::Refused(refusal)) => refusal.explain(),
        Err(other) => format!("preview failed: {}", other.explain()),
        Ok(_) => "preview drew a plan".to_owned(),
    }
}

fn applied_words(applied: &Result<Applied, ApplyError>) -> String {
    match applied {
        Err(ApplyError::Refused(refusal)) => refusal.explain(),
        Err(other) => format!("apply failed: {}", other.explain()),
        Ok(_) => "apply ran".to_owned(),
    }
}

/// All four of the ticket's checks, asserted together so a red run says
/// every one that failed.
fn assert_refused(repo: &TempRepo, op: &Operation, path: &str, words: &str) {
    let outcome = attempt(repo, op, path);
    let seen = (
        preview_words(&outcome.preview),
        applied_words(&outcome.applied),
        String::from_utf8_lossy(&outcome.bytes).into_owned(),
        outcome.entries.1 - outcome.entries.0,
    );
    let wanted = (
        words.to_owned(),
        words.to_owned(),
        String::from_utf8_lossy(MINE).into_owned(),
        0,
    );
    assert_eq!(
        seen, wanted,
        "(preview, apply, {path}'s bytes, new journal entries)"
    );
}

/// `path` ignored through `info/exclude`, holding his bytes.
fn ignore(repo: &TempRepo, path: &str) {
    repo.write(".git/info/exclude", format!("{path}\n").as_bytes());
    repo.write(path, MINE);
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        "",
        "status lists nothing"
    );
}
