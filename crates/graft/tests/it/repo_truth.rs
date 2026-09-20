//! Git, as evidence: the repository truth every acceptance walk judges by,
//! read by running git — never from `AppState` — plus the isolated `git`
//! runner, `fsck`, and the key label the capture headers use. Shared by the
//! M3 and M4 walks and by their examples (included by `#[path]`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::event::{KeyCode, KeyPress};

/// Repository truth, entirely from git and the file system.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RepoTruth {
    /// `HEAD`'s commit; `None` on an unborn branch.
    pub head: Option<String>,
    /// Every ref but `refs/graft/`, name → object.
    pub refs: BTreeMap<String, String>,
    /// `ls-files -s` lines.
    pub index: Vec<String>,
    /// Every file under the worktree, untracked included, `.git` excluded.
    pub files: BTreeMap<PathBuf, Vec<u8>>,
    /// `stash list` lines.
    pub stash: Vec<String>,
    /// The index file's bytes, to prove a read never wrote it. Not part of
    /// [`diff_truth`]: a write and its undo legitimately rewrite the file
    /// while `index` (its entries) comes back equal.
    pub index_bytes: Vec<u8>,
}

/// Run git in `repo`; stdout on success, git's own stderr on failure.
///
/// This is the walk's *evidence*, so it must say the same thing on every
/// machine. It runs under [`graft_fixtures::git::run_isolated`]: no global or
/// system config, no ambient repository, `LC_ALL=C`, a 30 s deadline
/// (PROMPT.md §7.1). Before that, a global `commit.gpgsign = true` failed the
/// `commit` scenario — and with it all 22 fast-tier tests — on any machine
/// that signs its commits (m3-07 review, Major 1); the same setting is what
/// made `gpg` create a `~/.gnupg/` the suite had no business creating.
///
/// The **app** under test is untouched by this: it runs in-process with
/// whatever config the user really has, which is the point of testing it.
pub fn run_git(repo: &Path, args: &[&str]) -> Result<String, String> {
    run_git_with(repo, args, None)
}

pub fn run_git_with(repo: &Path, args: &[&str], stdin: Option<&[u8]>) -> Result<String, String> {
    // A nulled global config has no identity in it, and half of these
    // commands write an object. One spelling, on every command, rather than
    // a second helper to forget.
    let mut full = vec![
        "-c",
        "user.name=Graft Test",
        "-c",
        "user.email=test@example.invalid",
    ];
    full.extend_from_slice(args);
    graft_fixtures::git::run_isolated(repo, &full, stdin)
        .map(|out| String::from_utf8_lossy(&out).into_owned())
        .map_err(|why| format!("git {args:?} failed: {why}"))
}

/// Put the machine's global git config out of the **app's** reach, by
/// overriding it where it counts in the scratch repository itself.
///
/// [`run_git`] isolates the walk's own spawns with `GIT_CONFIG_GLOBAL`, but
/// the app under test runs in this process and its git children inherit this
/// process's environment, so that lever does not reach them. Repository-local
/// config does: it beats global for every key here.
///
/// The list is deliberately short and deliberately a list — these are the
/// settings that make a *write* fail or mean something else, and each one was
/// a way the 22 fast-tier tests could fail on a developer's machine and pass
/// on the machine that wrote them (PROMPT.md §7.1; m3-07 review, Major 1).
/// `commit.gpgsign` is the one that was actually proven: with it on, `git
/// commit` calls `gpg`, which fails and — on a machine with no keyring —
/// creates a `~/.gnupg/` that no test had any business creating.
///
/// An identity is set for the same reason in reverse: a machine with *no*
/// `user.email` would fail every commit here.
pub fn neutralise_global_config(repo: &Path) -> Result<(), String> {
    let empty = git_path(repo, "graft-walk-empty")?;
    std::fs::create_dir_all(&empty).map_err(|why| format!("{}: {why}", empty.display()))?;
    let nothing = empty.join("nothing");
    std::fs::write(&nothing, b"").map_err(|why| format!("{}: {why}", nothing.display()))?;
    let empty = empty.to_str().ok_or("the scratch path is not UTF-8")?;
    let nothing = nothing.to_str().ok_or("the scratch path is not UTF-8")?;
    for (key, value) in [
        // Signing: calls out to `gpg`, which the test has no key for.
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        // An identity the machine neither supplies nor overrides.
        ("user.name", "Graft Test"),
        ("user.email", "test@example.invalid"),
        // A global hook would run on every commit here.
        ("core.hooksPath", empty),
        // Both change what `status` and the index report as untracked.
        ("status.showUntrackedFiles", "normal"),
        ("core.excludesFile", nothing),
        // Line endings and attributes change the bytes this walk compares.
        ("core.autocrlf", "false"),
        ("core.safecrlf", "false"),
        ("core.attributesFile", nothing),
        // Nothing may repack or prune under a scenario that is counting
        // objects, and no filesystem monitor may answer for the worktree.
        ("gc.auto", "0"),
        ("core.fsmonitor", "false"),
    ] {
        run_git(repo, &["config", "--local", key, value])?;
    }
    Ok(())
}

/// The blob id `content` would have.
pub fn blob_of(repo: &Path, content: &[u8]) -> Result<String, String> {
    run_git_with(repo, &["hash-object", "--stdin"], Some(content)).map(|s| s.trim().to_owned())
}

/// The lines of a git listing.
pub fn lines(out: &str) -> Vec<String> {
    out.lines().map(str::to_owned).collect()
}

/// Read the repository's truth.
pub fn git_truth(repo: &Path) -> Result<RepoTruth, String> {
    let head = run_git(repo, &["rev-parse", "--verify", "-q", "HEAD"])
        .ok()
        .map(|s| s.trim().to_owned());
    let refs = run_git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"])?
        .lines()
        .filter(|line| !line.starts_with("refs/graft/"))
        .filter_map(|line| {
            line.split_once(' ')
                .map(|(name, oid)| (name.to_owned(), oid.to_owned()))
        })
        .collect();
    let index = lines(&run_git(repo, &["ls-files", "-s"])?);
    let stash = lines(&run_git(repo, &["stash", "list"])?);
    let index_path = git_path(repo, "index")?;
    let index_bytes = std::fs::read(&index_path).unwrap_or_default_if_missing()?;
    let mut files = BTreeMap::new();
    collect_files(repo, repo, &mut files)?;
    Ok(RepoTruth {
        head,
        refs,
        index,
        files,
        stash,
        index_bytes,
    })
}

/// `std::fs::read` where "no such file" is an empty answer, not an error:
/// an unborn repository has no index file yet.
trait MissingIsEmpty {
    fn unwrap_or_default_if_missing(self) -> Result<Vec<u8>, String>;
}

impl MissingIsEmpty for std::io::Result<Vec<u8>> {
    fn unwrap_or_default_if_missing(self) -> Result<Vec<u8>, String> {
        match self {
            Ok(bytes) => Ok(bytes),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(why) => Err(format!("reading the index file: {why}")),
        }
    }
}

/// Where git keeps `name` for this worktree (`.git/…` or
/// `.git/worktrees/<name>/…`), absolute.
pub fn git_path(repo: &Path, name: &str) -> Result<PathBuf, String> {
    let out = run_git(repo, &["rev-parse", "--git-path", name])?;
    let path = PathBuf::from(out.trim());
    Ok(if path.is_absolute() {
        path
    } else {
        repo.join(path)
    })
}

pub fn collect_files(
    root: &Path,
    dir: &Path,
    into: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", dir.display()))?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".git") {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, into)?;
        } else {
            let bytes = std::fs::read(&path).map_err(|why| format!("{}: {why}", path.display()))?;
            let relative = path
                .strip_prefix(root)
                .map_err(|why| format!("{}: {why}", path.display()))?;
            into.insert(relative.to_path_buf(), bytes);
        }
    }
    Ok(())
}

/// What differs, at most 20 lines.
pub fn diff_truth(before: &RepoTruth, after: &RepoTruth) -> Vec<String> {
    let mut out = Vec::new();
    if before.head != after.head {
        out.push(format!("HEAD: {:?} → {:?}", before.head, after.head));
    }
    for name in before.refs.keys().chain(after.refs.keys()) {
        match (before.refs.get(name), after.refs.get(name)) {
            (Some(a), Some(b)) if a != b => out.push(format!("{name}: {a} → {b}")),
            (Some(_), None) => out.push(format!("{name}: gone")),
            (None, Some(b)) => out.push(format!("{name}: new, {b}")),
            _ => {}
        }
    }
    out.dedup();
    if before.index != after.index {
        for line in &before.index {
            if !after.index.contains(line) {
                out.push(format!("index −  {line}"));
            }
        }
        for line in &after.index {
            if !before.index.contains(line) {
                out.push(format!("index +  {line}"));
            }
        }
    }
    for path in before.files.keys().chain(after.files.keys()) {
        match (before.files.get(path), after.files.get(path)) {
            (Some(a), Some(b)) if a != b => out.push(format!(
                "{}: {} bytes → {} bytes",
                path.display(),
                a.len(),
                b.len()
            )),
            (Some(_), None) => out.push(format!("{}: deleted", path.display())),
            (None, Some(b)) => out.push(format!("{}: new, {} bytes", path.display(), b.len())),
            _ => {}
        }
    }
    out.dedup();
    if before.stash != after.stash {
        out.push(format!(
            "stash list: {:?} → {:?}",
            before.stash, after.stash
        ));
    }
    out.truncate(20);
    out
}

/// `Ok` when nothing differs, the differences otherwise.
pub fn same(what: &str, before: &RepoTruth, after: &RepoTruth) -> Result<(), String> {
    let diff = diff_truth(before, after);
    if diff.is_empty() {
        Ok(())
    } else {
        Err(format!("{what}:\n  {}", diff.join("\n  ")))
    }
}

pub fn check(condition: bool, what: impl FnOnce() -> String) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what()) }
}

/// `git fsck --no-dangling` says nothing but notices.
pub fn fsck_clean(repo: &Path) -> Result<(), String> {
    // Not `run_git`: fsck's complaints come on stderr *and* it exits
    // non-zero when it has any, so both streams are wanted whatever the
    // status. Isolated all the same — a global `fsck.*` severity setting
    // would otherwise decide what this test means.
    let out = graft_fixtures::git::isolated(repo, &["fsck", "--no-dangling", "--no-progress"])
        .output()
        .map_err(|why| format!("git fsck: {why}"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let complaints: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with("notice:") && !line.trim().is_empty())
        .collect();
    check(out.status.success() && complaints.is_empty(), || {
        format!("fsck: {}", complaints.join(" | "))
    })
}

/// A capture's keys as the header of its file names them: `k ⏎ Space`.
#[must_use]
pub fn keys_label(keys: &[KeyPress]) -> String {
    keys.iter()
        .map(|key| {
            let name = match key.code {
                KeyCode::Char(' ') => "Space".to_owned(),
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Enter => "⏎".to_owned(),
                KeyCode::Escape => "Esc".to_owned(),
                KeyCode::Tab => "⇥".to_owned(),
                KeyCode::Right => "→".to_owned(),
                other => format!("{other:?}"),
            };
            if key.modifiers.control {
                format!("⌃{name}")
            } else {
                name
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
