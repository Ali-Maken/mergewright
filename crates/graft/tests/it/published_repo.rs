//! Nothing a commit from here would carry names the machine it was written
//! on.
//!
//! CLAUDE.md, Gates: *"Raw terminal streams never committed. The repo may be
//! published."* 132 files carried the author's home directory, 48 a scratch
//! path and 6 a port the OS picked (m5-history-and-ghost-preview/41). Every
//! writer was fixed at its seam — `graft_fixtures::capture` redacts what a
//! generator drew before it compares or writes it — and this is the test that
//! keeps the fix true after the ticket is forgotten. It runs on every build,
//! over every file, not only over `docs/shots/`: the first leak this rule
//! ever caught outside a capture was in a perf log and a ticket.
//!
//! It is scoped to the **class**, never to one person's name. A test that
//! looked for one account would pass on every other machine while still
//! shipping that machine's paths, which is the shape of a guard that is not
//! one. `graft_fixtures::capture::machine` says what the class is; this file
//! only walks the repository and asks.
//!
//! ## What it reads, exactly
//!
//! The index **and** the working tree's un-ignored files — [`in_scope`] says
//! how and why. It was written reading the index alone, and that was a hole
//! of the same shape as the one the ticket exists to close: a file is not in
//! the index until it is `git add`ed, so the gate run a builder finishes on
//! could not see the file they were about to commit, and the first run able
//! to catch it was the run after the commit. This module's own neighbour went
//! in that way and tripped the guard one merge later.

use std::path::{Path, PathBuf};
use std::process::Command;

/// How many offending places the failure prints before it stops. Enough to
/// see the shape; the count says how many there are in all.
const SHOWN: usize = 20;

/// Whether `git` already has a file, which changes what the reader must do
/// about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// In the index: tracked, or staged and not yet committed.
    Indexed,
    /// In the working tree and not ignored, so `git add -A` would take it.
    Untracked,
}

impl Origin {
    fn note(self) -> &'static str {
        match self {
            Self::Indexed => "",
            Self::Untracked => " (untracked, and not ignored — a commit would take it)",
        }
    }
}

#[test]
fn nothing_a_commit_would_carry_names_this_machine() {
    let root = workspace_root();
    let mut complaints = Vec::new();
    let mut total = 0usize;
    let mut files = 0usize;

    for (relative, origin) in in_scope(&root) {
        let whole = root.join(&relative);
        // A path that is not a file here — a submodule entry, a symlink to
        // nowhere, something staged for deletion — has no contents of its own
        // to leak.
        if !whole.is_file() {
            continue;
        }
        let bytes = std::fs::read(&whole)
            .unwrap_or_else(|why| panic!("{} could not be read: {why}", relative.display()));
        let found = graft_fixtures::found(&bytes);
        if found.is_empty() {
            continue;
        }
        files += 1;
        total += found.len();
        for hit in found.iter().take(SHOWN.saturating_sub(complaints.len())) {
            complaints.push(format!(
                "  {}:{} — {}{}",
                relative.display(),
                line_of(&bytes, hit.at),
                hit.machine.why(),
                origin.note(),
            ));
        }
    }

    assert!(
        complaints.is_empty(),
        "{total} machine-specific paths or ports in {files} files a commit from here would \
         carry, in a repository CLAUDE.md says may be published:\n{}\n\nA capture is redacted \
         where it is written (`graft_fixtures::capture`), so regenerate it:\n\n    \
         TRIB_UPDATE_SHOTS=1 cargo nextest run\n\nAnything else — a ticket, a perf log, a \
         fixture in a test — is written by hand and says `<home>` instead; a test that needs \
         one of these literally assembles it from pieces, the way \
         `graft_fixtures::capture::machine` does. A file that predates the rule is fixed with \
         the writers' own function:\n\n    \
         cargo run -q -p graft-fixtures -- redact <path>...\n",
        complaints.join("\n"),
    );
}

/// The 1-based line `at` falls on, for a message a reader can act on.
///
/// Counted the naive way on purpose: it runs once per offending place, and
/// the only run that has any is the one that is about to fail.
#[allow(
    clippy::naive_bytecount,
    reason = "a dependency for a line number printed only on the way to a panic"
)]
fn line_of(bytes: &[u8], at: usize) -> usize {
    bytes[..at].iter().filter(|byte| **byte == b'\n').count() + 1
}

/// The workspace root, from this crate's own location.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|why| panic!("the workspace root must be reachable from the crate: {why}"))
}

/// Every path a commit from here would carry, and whether `git` has it yet.
///
/// Two listings, because the answer is the union of two questions and the
/// reader needs to know which one a complaint came from:
///
/// * `--cached` — the index: tracked files, and new files already `git add`ed.
/// * `--others --exclude-standard` — the working tree minus what `.gitignore`
///   covers, which is exactly what `git add -A` would newly stage.
///
/// The index alone is the natural thing to write and it is not enough: a new
/// file is invisible until it is added, so a builder's last green gate can
/// precede the failure by one commit. Including untracked files costs a
/// stricter test — a stray file in the working tree can fail the suite — and
/// that is the right price: it fails *before* the commit rather than after,
/// and the message says the file is untracked so the fix is obvious.
///
/// What this still does **not** see, deliberately:
///
/// * content staged in the index that the working tree no longer has. It
///   reads files from disk, not blobs from the index, so a file staged and
///   then edited is judged by what is on disk. Closing that means
///   `git cat-file` per path; the case is `git add` followed by an edit that
///   *removes* a leak, which is nobody's failure mode.
/// * anything `.gitignore` excludes, which cannot reach a commit without
///   `git add -f`.
fn in_scope(root: &Path) -> Vec<(PathBuf, Origin)> {
    let indexed = listed(root, &["ls-files", "--cached", "-z"]);
    assert!(
        !indexed.is_empty(),
        "no files in the index at {} — the guard would pass by looking at nothing",
        root.display(),
    );
    let untracked = listed(root, &["ls-files", "--others", "--exclude-standard", "-z"]);
    indexed
        .into_iter()
        .map(|path| (path, Origin::Indexed))
        .chain(untracked.into_iter().map(|path| (path, Origin::Untracked)))
        .collect()
}

/// One `git ls-files` listing, as paths relative to the root.
fn listed(root: &Path, args: &[&str]) -> Vec<PathBuf> {
    let listed = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|why| panic!("git {} must run to know the scope: {why}", args.join(" ")));
    assert!(
        listed.status.success(),
        "git {} failed in {}: {}",
        args.join(" "),
        root.display(),
        String::from_utf8_lossy(&listed.stderr).trim(),
    );
    listed
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| PathBuf::from(String::from_utf8_lossy(name).into_owned()))
        .collect()
}
