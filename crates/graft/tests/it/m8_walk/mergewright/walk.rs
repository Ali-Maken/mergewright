//! What every Mergewright walk shares: his repository's place, the launch
//! and quit over the pty, the capture with its "what right looks like"
//! line, and the truth checks read from git.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::OperationKind;
use graft_core::on_disk::Generation;

use super::older::{journal, refs_of};
use crate::pty::{Isolated, Pty, Screen};

/// His repository under the isolated home, drawn `~/code/api`.
pub fn his_repo(isolated: &Isolated) -> PathBuf {
    isolated.home().join("code").join("api")
}

/// The files under `.git/<generation's dir>/` the move carries: the
/// move's `M`. A generation's lock and marker are its own, not his, and
/// are not carried.
pub fn files_in(repo: &Path, generation: Generation) -> usize {
    fn count(dir: &Path) -> usize {
        match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .map(|entry| entry.unwrap_or_else(|why| panic!("{why}")).path())
                .filter(|path| {
                    path.file_name().is_none_or(|name| {
                        name != graft_ops::rename::LOCK_FILE
                            && name != graft_core::on_disk::MIGRATED_MARKER
                    })
                })
                .map(|path| if path.is_dir() { count(&path) } else { 1 })
                .sum(),
            Err(why) => panic!("{}: {why}", dir.display()),
        }
    }
    count(&repo.join(".git").join(generation.dir()))
}

/// `mwr <repo>` at `size`: the graph drawn, the cursor `▸` on a row,
/// nothing still loading.
pub fn launch(isolated: &Isolated, repo: &Path, size: (u16, u16)) -> (Pty, Screen) {
    let pty = Pty::spawn(isolated, repo, isolated.root.path(), size);
    let screen = pty.wait_for("the graph, the cursor on a row, nothing loading", |s| {
        s.has("\u{25b8}")
            && s.lines()[0].contains("api")
            && !s.has("sorting\u{2026}")
            && !s.has("reading\u{2026}")
    });
    (pty, screen)
}

/// `q`, and the app ends cleanly.
pub fn quit(mut pty: Pty) {
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// `J`, and the journal once nothing in it is still loading and its cursor
/// `▸` has landed on a row.
pub fn journal_screen(pty: &mut Pty, has: &str) -> Screen {
    pty.send(b"J");
    pty.wait_for(&format!("J with {has:?}, nothing loading"), |s| {
        s.has(has)
            && !s.has("finding lost commits\u{2026}")
            && s.lines().iter().any(|line| line.starts_with('\u{25b8}'))
    })
}

/// `Esc` out of J, back on the graph (`q` there is not quit).
pub fn close_journal(pty: &mut Pty) {
    pty.send(b"\x1b");
    pty.wait_for("the graph again", |s| !s.lines()[0].starts_with("JOURNAL"));
}

/// The cursor's row.
pub fn cursor_row(screen: &Screen) -> String {
    screen
        .lines()
        .into_iter()
        .find(|line| line.starts_with('\u{25b8}'))
        .unwrap_or_else(|| panic!("no cursor row:\n{}", screen.text()))
}

/// The capture under `docs/shots/`, with what right looks like on top.
pub fn shot(name: &str, right: &str, screen: &Screen) {
    crate::captures::check_shot(
        name,
        &format!(
            "# m8-polish-and-ship/57, `mwr` over a pty (Unicode tier): what right looks like —\n\
             # {right}\n# Colour unverified (rule 5).\n\n{}\n",
            screen.text()
        ),
    );
}

/// No ref is left under an earlier generation's prefix, and every backup
/// the older build made is under the new names: the ref is there and holds
/// the very commit or one built on it (recovering an interrupted entry
/// adds an `interrupted, recovered` commit whose parent is the old backup).
pub fn every_backup_kept(repo: &Path, old: &BTreeMap<String, String>) {
    for older in Generation::CURRENT.older() {
        assert_eq!(refs_of(repo, older), BTreeMap::new(), "{older:?}");
    }
    let expected = super::older::moved(old, Generation::CURRENT);
    let now = refs_of(repo, Generation::CURRENT);
    assert_eq!(
        now.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    for (name, oid) in &expected {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["merge-base", "--is-ancestor", oid, &now[name]])
            .status()
            .unwrap_or_else(|why| panic!("{why}"));
        assert!(status.success(), "{name}: {oid} is not in {}", now[name]);
    }
    println!(
        "truth: refs under {}: {} · under {}: {now:?}",
        Generation::Graft.ref_prefix(),
        refs_of(repo, Generation::Graft).len(),
        Generation::CURRENT.ref_prefix()
    );
}

/// The journal's `Migrate` rows, oldest first, after `entries` entries in
/// all: what J's rename rows are read from.
pub fn renames(repo: &Path, entries: usize) -> Vec<String> {
    let read = journal(repo);
    assert_eq!(read.len(), entries, "{read:#?}");
    let rows: Vec<String> = read
        .iter()
        .filter(|entry| entry.kind == OperationKind::Migrate)
        .map(|entry| entry.description.clone())
        .collect();
    println!("truth: journal {} entries, renames {rows:?}", read.len());
    rows
}
