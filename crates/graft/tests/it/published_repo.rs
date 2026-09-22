//! Nothing tracked in this repository names the machine it was written on.
//!
//! CLAUDE.md, Gates: *"Raw terminal streams never committed. The repo may be
//! published."* 124 tracked files carried the author's home directory, 47 a
//! scratch path and 5 a port the OS picked
//! (m5-history-and-ghost-preview/41). Every writer was fixed at its seam —
//! `graft_fixtures::capture` redacts what a generator drew before it compares
//! or writes it — and this is the test that keeps the fix true after the
//! ticket is forgotten. It runs on every build, over every tracked file, not
//! only over `docs/shots/`: the first leak this rule ever caught outside a
//! capture was in a perf log and a ticket.
//!
//! It is scoped to the **class**, never to one person's name. A test that
//! looked for one account would pass on every other machine while still
//! shipping that machine's paths, which is the shape of a guard that is not
//! one. `graft_fixtures::machine` says what the class is; this file only
//! walks the repository and asks.

use std::path::{Path, PathBuf};
use std::process::Command;

/// How many offending places the failure prints before it stops. Enough to
/// see the shape; the count says how many there are in all.
const SHOWN: usize = 20;

#[test]
fn no_tracked_file_names_this_machine() {
    let root = workspace_root();
    let mut complaints = Vec::new();
    let mut total = 0usize;
    let mut files = 0usize;

    for relative in tracked_files(&root) {
        let whole = root.join(&relative);
        // A tracked path that is not a file here — a submodule entry, a
        // symlink to nowhere — has no contents of its own to leak.
        if !whole.is_file() {
            continue;
        }
        let bytes = std::fs::read(&whole).unwrap_or_else(|why| {
            panic!(
                "tracked file {} could not be read: {why}",
                relative.display()
            )
        });
        let found = graft_fixtures::found(&bytes);
        if found.is_empty() {
            continue;
        }
        files += 1;
        total += found.len();
        for hit in found.iter().take(SHOWN.saturating_sub(complaints.len())) {
            complaints.push(format!(
                "  {}:{} — {}",
                relative.display(),
                line_of(&bytes, hit.at),
                hit.machine.why(),
            ));
        }
    }

    assert!(
        complaints.is_empty(),
        "{total} machine-specific paths or ports in {files} tracked files, in a repository \
         CLAUDE.md says may be published:\n{}\n\nA capture is redacted where it is written \
         (`graft_fixtures::capture`), so regenerate it:\n\n    \
         GRAFT_UPDATE_SHOTS=1 cargo nextest run\n\nAnything else — a ticket, a perf log, a \
         fixture in a test — is written by hand and says `<home>` instead. A file that \
         predates the rule is fixed with the writers' own function:\n\n    \
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

/// Every path `git` tracks, relative to the root.
///
/// Tracked, not walked: `target/`, the fixture cache and every other
/// untracked file is somebody's machine by definition and none of it is
/// published. What `git` would ship is exactly what this test reads.
fn tracked_files(root: &Path) -> Vec<PathBuf> {
    let listed = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .unwrap_or_else(|why| panic!("git ls-files must run to know what is tracked: {why}"));
    assert!(
        listed.status.success(),
        "git ls-files failed in {}: {}",
        root.display(),
        String::from_utf8_lossy(&listed.stderr).trim(),
    );
    let paths: Vec<PathBuf> = listed
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| PathBuf::from(String::from_utf8_lossy(name).into_owned()))
        .collect();
    assert!(
        !paths.is_empty(),
        "no tracked files in {} — the guard would pass by looking at nothing",
        root.display(),
    );
    paths
}
