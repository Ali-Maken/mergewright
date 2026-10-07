//! Find anything (m8-polish-and-ship/12, J26–J28): the real binary on a
//! pty, walking the design note's journeys `seek-branch`, `seek-tag`,
//! `rerun-recent` and (12b) `seek-commit-sha`, `seek-commit-words`,
//! `file-history`, counting keys against its `journey-counts` fence. The
//! start is the graph with its cursor on HEAD; every run is isolated by
//! `pty::Isolated` (no real config, no real recent list).
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::path::Path;

use crate::app_harness::git;
use crate::pty::{Isolated, Pty, Screen};

/// The design note whose `journey-counts` fence these walks are held to.
const NOTE: &str = "../../docs/design-notes/m8-polish-and-ship.md";
/// `⌃P`.
const CTRL_P: &[u8] = b"\x10";
/// The cursor glyph on the graph's row.
const CURSOR: char = '\u{25b8}';

fn noted_keys(id: &str) -> usize {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(NOTE);
    let note =
        std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    crate::m5_walk::parse_counts(&note)
        .unwrap_or_else(|why| panic!("the M8 fence does not parse: {why}"))
        .iter()
        .find(|row| row.id == id)
        .and_then(|row| row.keys)
        .unwrap_or_else(|| panic!("the M8 fence has no count for {id}"))
}

/// A journey's inputs, counted as the fence counts them: each entry one
/// step (a typed name is one step, as the fence's `<name>`).
fn assert_within_the_note(id: &str, keys: &[&[u8]]) {
    let noted = noted_keys(id);
    assert!(
        keys.len() <= noted,
        "{id}: took {} inputs, the note allows {noted}",
        keys.len()
    );
}

/// The feature-branch corpus with a branch `qzseek` and a tag `vqz9` on
/// commits below HEAD; the scratch, HEAD's oid and the subjects of the
/// branch's and the tag's commits.
struct Repo {
    scratch: graft_fixtures::Scratch,
    head: String,
    branch_subject: String,
    tag_subject: String,
}

fn repo() -> Repo {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
    let path = scratch.path();
    // Launched in a repository, the cursor lands on HEAD (`main`; ticket
    // 12d), where the walks start.
    git(path, &["branch", "qzseek", "HEAD~2"]);
    git(path, &["tag", "vqz9", "HEAD~3"]);
    let subject = |rev: &str| {
        git(path, &["log", "-1", "--format=%s", rev])
            .trim()
            .to_owned()
    };
    Repo {
        head: git(path, &["rev-parse", "HEAD"]).trim().to_owned(),
        branch_subject: subject("qzseek"),
        tag_subject: subject("vqz9"),
        scratch,
    }
}

fn spawn(repo: &Repo, isolated: &Isolated) -> Pty {
    let path = repo.scratch.path();
    let ceiling = path.parent().expect("a parent");
    let pty = Pty::spawn(isolated, path, ceiling, (120, 36));
    let head_subject = git(path, &["log", "-1", "--format=%s"]).trim().to_owned();
    // The first frame may draw before the layout settles; the start state
    // is the settled graph with its cursor on HEAD.
    pty.wait_for("the graph, cursor on HEAD", |s| {
        s.lines()
            .iter()
            .any(|l| l.starts_with(CURSOR) && l.contains(&head_subject))
    });
    pty
}

/// Waits until the graph's cursor row holds `subject` and the palette is
/// closed.
fn seeked_to(pty: &Pty, subject: &str) -> Screen {
    pty.wait_for("the graph's cursor on the sought commit", |s| {
        !s.has("find anything")
            && s.lines()
                .iter()
                .any(|l| l.starts_with(CURSOR) && l.contains(subject))
    })
}

fn quit(mut pty: Pty) {
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// J26 `seek-branch` = 3: `⌃P qzseek ⏎` puts the graph's cursor on the
/// branch's tip; HEAD does not move (a seek, not a switch).
#[test]
fn walk_seek_branch() {
    let repo = repo();
    let isolated = Isolated::new();
    let mut pty = spawn(&repo, &isolated);
    let keys: [&[u8]; 3] = [CTRL_P, b"qzseek", b"\r"];
    pty.send(keys[0]);
    pty.wait_for("the palette", |s| s.has("find anything"));
    pty.send(keys[1]);
    pty.wait_for("the branch row", |s| {
        s.has("BRANCHES")
            && s.lines()
                .iter()
                .any(|l| l.contains("│\u{25b8}") && l.contains("qzseek"))
    });
    pty.send(keys[2]);
    seeked_to(&pty, &repo.branch_subject);
    assert_within_the_note("seek-branch", &keys);
    assert_eq!(
        git(repo.scratch.path(), &["rev-parse", "HEAD"]).trim(),
        repo.head,
        "HEAD unchanged"
    );
    assert_ne!(
        git(repo.scratch.path(), &["branch", "--show-current"]).trim(),
        "qzseek",
        "not switched to the sought branch"
    );
    quit(pty);
}

/// J27 `seek-tag` = 3: `⌃P vqz9 ⏎` puts the cursor on the tagged commit.
#[test]
fn walk_seek_tag() {
    let repo = repo();
    let isolated = Isolated::new();
    let mut pty = spawn(&repo, &isolated);
    let keys: [&[u8]; 3] = [CTRL_P, b"vqz9", b"\r"];
    pty.send(keys[0]);
    pty.wait_for("the palette", |s| s.has("find anything"));
    pty.send(keys[1]);
    pty.wait_for("the tag row", |s| {
        s.has("TAGS")
            && s.lines()
                .iter()
                .any(|l| l.contains("│\u{25b8}") && l.contains("vqz9"))
    });
    pty.send(keys[2]);
    seeked_to(&pty, &repo.tag_subject);
    assert_within_the_note("seek-tag", &keys);
    assert_eq!(
        git(repo.scratch.path(), &["rev-parse", "HEAD"]).trim(),
        repo.head,
        "HEAD unchanged"
    );
    quit(pty);
}

/// J28 `rerun-recent` = 2: after one palette run (`Undo journal`), `⌃P ⏎`
/// runs it again — the recent row is first, under the cursor.
#[test]
fn walk_rerun_recent() {
    let repo = repo();
    let isolated = Isolated::new();
    let mut pty = spawn(&repo, &isolated);
    // The one earlier run: not part of the journey.
    pty.send(CTRL_P);
    pty.wait_for("the palette", |s| s.has("find anything"));
    pty.send(b"undo journal");
    pty.wait_for("the journal row", |s| {
        s.lines()
            .iter()
            .any(|l| l.contains("\u{25b8}J") && l.contains("Undo journal"))
    });
    pty.send(b"\r");
    pty.wait_for("the journal", |s| s.has("J/esc close"));
    pty.send(b"\x1b");
    pty.wait_for("back on the graph", |s| !s.has("J/esc close"));

    let keys: [&[u8]; 2] = [CTRL_P, b"\r"];
    pty.send(keys[0]);
    let open = pty.wait_for("the palette, recent first", |s| {
        s.has("COMMANDS RUN RECENTLY")
    });
    assert!(
        open.lines()
            .iter()
            .any(|l| l.contains("\u{25b8}J") && l.contains("Undo journal")),
        "the recent run under the cursor\n{}",
        open.text()
    );
    pty.send(keys[1]);
    pty.wait_for("the journal again", |s| {
        s.has("J/esc close") && !s.has("find anything")
    });
    assert_within_the_note("rerun-recent", &keys);
    pty.send(b"\x1b");
    pty.wait_for("back on the graph", |s| !s.has("J/esc close"));
    quit(pty);
}

/// 12b's fixture, on [`repo`]'s: `tools/harmonica.txt` added by `Tune the
/// instrument`, then `Untangle the quokka ledger`, then `Polish` (HEAD), so
/// both sought commits lie below HEAD. The words `quokka ledger` and the
/// path `harmonica` match no action, branch, tag or other group's row
/// (the fence's fixture for `seek-commit-words`).
struct Finds {
    repo: Repo,
    /// `Tune the instrument`'s full sha.
    tune: String,
}

fn finds() -> Finds {
    let mut repo = repo();
    let path = repo.scratch.path().to_path_buf();
    std::fs::create_dir_all(path.join("tools")).expect("a directory");
    std::fs::write(path.join("tools/harmonica.txt"), "do re mi\n").expect("a file");
    git(&path, &["add", "tools/harmonica.txt"]);
    git(&path, &["commit", "--quiet", "-m", "Tune the instrument"]);
    let tune = git(&path, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &path,
        &[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "Untangle the quokka ledger",
        ],
    );
    git(
        &path,
        &["commit", "--quiet", "--allow-empty", "-m", "Polish"],
    );
    git(&path, &["rev-parse", "HEAD"])
        .trim()
        .clone_into(&mut repo.head);
    Finds { repo, tune }
}

/// Waits for the palette's cursor on a row of `group` holding `needle`.
fn cursor_in_group(pty: &Pty, group: &str, needle: &str) {
    pty.wait_for("the find row under the palette's cursor", |s| {
        s.has(group)
            && s.lines()
                .iter()
                .any(|l| l.contains("│\u{25b8}") && l.contains(needle))
    });
}

/// J29 `seek-commit-sha` = 3: `⌃P <sha prefix> ⏎` puts the graph's cursor
/// on that commit, found in all of history, not only the rows loaded.
#[test]
fn walk_seek_commit_sha() {
    let finds = finds();
    let isolated = Isolated::new();
    let mut pty = spawn(&finds.repo, &isolated);
    let prefix = &finds.tune[..12];
    let keys: [&[u8]; 3] = [CTRL_P, prefix.as_bytes(), b"\r"];
    pty.send(keys[0]);
    pty.wait_for("the palette", |s| s.has("find anything"));
    pty.send(keys[1]);
    cursor_in_group(&pty, "COMMITS", "Tune the instrument");
    pty.send(keys[2]);
    seeked_to(&pty, "Tune the instrument");
    assert_within_the_note("seek-commit-sha", &keys);
    assert_eq!(
        git(finds.repo.scratch.path(), &["rev-parse", "HEAD"]).trim(),
        finds.repo.head,
        "HEAD unchanged"
    );
    quit(pty);
}

/// J30 `seek-commit-words` = 3: `⌃P quokka ledger ⏎`, on the fence's
/// fixture (the words match one commit and nothing else).
#[test]
fn walk_seek_commit_words() {
    let finds = finds();
    let isolated = Isolated::new();
    let mut pty = spawn(&finds.repo, &isolated);
    let keys: [&[u8]; 3] = [CTRL_P, b"quokka ledger", b"\r"];
    pty.send(keys[0]);
    pty.wait_for("the palette", |s| s.has("find anything"));
    pty.send(keys[1]);
    cursor_in_group(&pty, "COMMITS", "Untangle the quokka ledger");
    pty.send(keys[2]);
    seeked_to(&pty, "Untangle the quokka ledger");
    assert_within_the_note("seek-commit-words", &keys);
    quit(pty);
}

/// J31 `file-history` = 3: `⌃P harmonica ⏎` ends on the history place for
/// `tools/harmonica.txt`. `Esc` (not part of the journey) lands back on
/// the graph, its cursor on HEAD where it was.
#[test]
fn walk_file_history() {
    let finds = finds();
    let isolated = Isolated::new();
    let mut pty = spawn(&finds.repo, &isolated);
    let keys: [&[u8]; 3] = [CTRL_P, b"harmonica", b"\r"];
    pty.send(keys[0]);
    pty.wait_for("the palette", |s| s.has("find anything"));
    pty.send(keys[1]);
    cursor_in_group(&pty, "FILES", "tools/harmonica.txt");
    pty.send(keys[2]);
    pty.wait_for("the history place for the path", |s| {
        !s.has("find anything")
            && s.has("HISTORY \u{b7} tools/harmonica.txt")
            && s.has("Tune the instrument")
    });
    assert_within_the_note("file-history", &keys);
    pty.send(b"\x1b");
    pty.wait_for("the graph, cursor on HEAD as before", |s| {
        !s.has("HISTORY \u{b7}")
            && s.lines()
                .iter()
                .any(|l| l.starts_with(CURSOR) && l.contains("Polish"))
    });
    quit(pty);
}
