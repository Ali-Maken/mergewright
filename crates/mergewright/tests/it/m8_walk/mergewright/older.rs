//! Repositories as an older build left them (ticket 57 Decision 3): the
//! layout mergewright-ops' `TempRepo::init_on(Generation)` and its rename tests
//! build, rebuilt here because those helpers live in mergewright-ops' own test
//! binary, out of this crate's reach. Never by running an older build.
//!
//! The work itself is real: with no generation's marker on disk, Mergewright's
//! names are in force (ADR-0057 Amendment §3), so `mergewright_ops::apply` run
//! here journals under `.git/graft/` and backs up under `refs/graft/`, as a
//! Mergewright build did. Only the interrupted set-aside's stash message is
//! written by hand, the way mergewright-ops' `set_aside_generations.rs` writes
//! it; the Tributary move is in [`super::tributary`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use mergewright_core::journal::{EntryStatus, HeadRef, JournalEntry};
use mergewright_core::oid::Oid;
use mergewright_core::on_disk::Generation;
use mergewright_git::repo::RepoHandle;
use mergewright_ops::apply::{Context, apply};
use mergewright_ops::journal::Journal;
use mergewright_ops::operation::{
    BranchCreateParams, CheckoutParams, CheckoutTarget, OnEdits, Operation, Target,
};

use crate::pty::PINNED_NOW;

/// The journal's file name in each generation's folder.
pub const JOURNAL: &str = "undo-journal.jsonl";

/// The branch the interrupted switch was headed for.
pub const SWITCHED_TO: &str = "b0";

/// The instant (ms) of the fixture's `n`th journal entry: a day before the
/// pinned clock, a minute apart, so J draws the same ages every run.
pub fn at(n: i64) -> i64 {
    (PINNED_NOW - 86_400) * 1_000 + n * 60_000
}

/// `git` in `repo` dated two days before the pinned clock; stdout, trimmed.
pub fn git(repo: &Path, args: &[&str]) -> String {
    git_at(repo, PINNED_NOW - 2 * 86_400, args)
}

/// `git` in `repo` with its dates at `secs`; stdout, trimmed.
pub fn git_at(repo: &Path, secs: i64, args: &[&str]) -> String {
    let date = format!("{secs} +0000");
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// `.git/<generation's dir>/<file>` under `repo`.
pub fn in_dir(repo: &Path, generation: Generation, file: &str) -> PathBuf {
    repo.join(".git").join(generation.dir()).join(file)
}

/// Every ref under `generation`'s prefix, name to id.
pub fn refs_of(repo: &Path, generation: Generation) -> BTreeMap<String, String> {
    git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            generation.ref_prefix(),
        ],
    )
    .lines()
    .filter_map(|line| line.split_once(' '))
    .map(|(name, oid)| (name.to_owned(), oid.to_owned()))
    .collect()
}

/// `old` as `to`'s prefix spells it.
pub fn moved(old: &BTreeMap<String, String>, to: Generation) -> BTreeMap<String, String> {
    old.iter()
        .map(|(name, oid)| {
            let from = Generation::of_ref(name).unwrap_or_else(|| panic!("not ours: {name}"));
            (
                name.replacen(from.ref_prefix(), to.ref_prefix(), 1),
                oid.clone(),
            )
        })
        .collect()
}

/// The journal as the app reads it, under the names in force.
pub fn journal(repo: &Path) -> Vec<JournalEntry> {
    let handle = open(repo);
    let path = mergewright_ops::journal::journal_path(&handle)
        .unwrap_or_else(|why| panic!("the journal path: {why:?}"));
    Journal::open_as(&path, handle.on_disk_names())
        .unwrap_or_else(|why| panic!("the journal opens: {why:?}"))
        .entries()
        .to_vec()
}

/// `repo` opened as the app opens it.
pub fn open(repo: &Path) -> RepoHandle {
    RepoHandle::open(repo).unwrap_or_else(|why| panic!("the repo opens: {why:?}"))
}

/// Run `op` as the app's job thread does, at the fixture's `n`th instant,
/// under whatever names are in force.
fn run(repo: &Path, op: &Operation, n: i64) {
    let handle = open(repo);
    let version = mergewright_git::version::system_git()
        .unwrap_or_else(|why| panic!("git is on PATH: {why:?}"))
        .version();
    let path = mergewright_ops::journal::journal_path(&handle)
        .unwrap_or_else(|why| panic!("the journal path: {why:?}"));
    let mut journal = Journal::open_as(&path, handle.on_disk_names())
        .unwrap_or_else(|why| panic!("the journal opens: {why:?}"));
    let random = [u8::try_from(n).unwrap_or(u8::MAX); 10];
    let mut ctx = Context::open(&handle, version, &mut journal, at(n), random, &())
        .unwrap_or_else(|why| panic!("the context opens: {why:?}"));
    if let Err(why) = apply(&mut ctx, op) {
        panic!("{op:?}: {}", why.explain(&mergewright_ops::keys::NoKeys));
    }
}

/// A file's bytes, or the reason it could not be read.
pub fn read(repo: &Path, file: &str) -> Vec<u8> {
    std::fs::read(repo.join(file)).unwrap_or_else(|why| panic!("{file}: {why}"))
}

/// `git init` in a new folder `repo` on `main`, with an identity and the
/// local settings the machine's global config would otherwise reach into.
pub fn init(repo: &Path) {
    std::fs::create_dir_all(repo).unwrap_or_else(|why| panic!("{}: {why}", repo.display()));
    git(repo, &["init", "-q", "-b", "main", "."]);
    for (key, value) in [
        ("user.name", "Mergewright Test"),
        ("user.email", "test@example.invalid"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        ("core.autocrlf", "false"),
        ("core.hooksPath", "/dev/null"),
    ] {
        git(repo, &["config", key, value]);
    }
}

/// The 20th entry of [`mergewright_era`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Last {
    /// A switch to [`SWITCHED_TO`] that set his edits aside and was killed
    /// before it put them back.
    InterruptedSwitch,
    /// A third stage.
    Stage,
}

/// What a Mergewright build left in `repo` (a new folder): 17 branch creates, two
/// stages and `last` — 20 entries, 3 backups under `refs/graft/`, no
/// marker; with [`Last::InterruptedSwitch`] the set-aside stash is listed
/// as `graft: …`. Hands back the [`state`] just before `last`.
pub fn mergewright_era(repo: &Path, last: Last) -> String {
    init(repo);
    std::fs::write(repo.join("a.rs"), b"fn one() {}\n").unwrap_or_else(|why| panic!("{why}"));
    git(repo, &["add", "a.rs"]);
    git(repo, &["commit", "-q", "-m", "start the api"]);
    let head = Oid::from_hex(&git(repo, &["rev-parse", "HEAD"]))
        .unwrap_or_else(|why| panic!("a commit id: {why:?}"));
    assert_eq!(open(repo).on_disk_names(), Generation::Graft);

    for n in 0..17 {
        let op = Operation::BranchCreate(BranchCreateParams {
            name: format!("b{n}"),
            at: head,
            checkout: false,
            on_edits: OnEdits::Refuse,
        });
        run(repo, &op, n);
    }
    for n in 0..2 {
        std::fs::write(repo.join("a.rs"), format!("fn one() {{ {n} }}\n"))
            .unwrap_or_else(|why| panic!("{why}"));
        run(
            repo,
            &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
            17 + n,
        );
    }
    std::fs::write(repo.join("a.rs"), b"fn one() { \"his edit\" }\n")
        .unwrap_or_else(|why| panic!("{why}"));
    let before = state(repo);
    match last {
        Last::InterruptedSwitch => interrupted_switch(repo),
        Last::Stage => run(
            repo,
            &Operation::Stage(Target::Paths(vec![PathBuf::from("a.rs")])),
            19,
        ),
    }
    reflogs_at_the_fixture_time(&repo.join(".git").join("logs"));
    before
}

/// The reflog lines the in-process run wrote carry the wall clock, while
/// the journal and the app's clock are pinned: stamped at the last entry's
/// instant instead, so J pairs the switch's reflog line with its entry and
/// draws the same ages every run. Lines git wrote with the fixture's date
/// stay as they are.
fn reflogs_at_the_fixture_time(dir: &Path) {
    let fixture_date = PINNED_NOW - 2 * 86_400;
    let stamp = (at(19) / 1_000).to_string();
    for entry in std::fs::read_dir(dir).unwrap_or_else(|why| panic!("{why}")) {
        let path = entry.unwrap_or_else(|why| panic!("{why}")).path();
        if path.is_dir() {
            reflogs_at_the_fixture_time(&path);
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{why}"));
        let mut out = String::new();
        for line in text.lines() {
            let (head, message) = line.split_once('\t').unwrap_or((line, ""));
            let mut words: Vec<&str> = head.split(' ').collect();
            let at_word = words.len() - 2;
            let written: i64 = words[at_word]
                .parse()
                .unwrap_or_else(|why| panic!("{line}: {why}"));
            if written > fixture_date {
                words[at_word] = &stamp;
            }
            out.push_str(&format!("{}\t{message}\n", words.join(" ")));
        }
        std::fs::write(&path, out).unwrap_or_else(|why| panic!("{why}"));
    }
}

/// What an undo has to put back, read from git and the disk: where `HEAD`
/// is, the commit, the index, `a.rs`'s bytes, the stash list.
pub fn state(repo: &Path) -> String {
    format!(
        "HEAD {}\ncommit {}\nindex {}\na.rs {:?}\nstash [{}]",
        git(repo, &["symbolic-ref", "-q", "HEAD"]),
        git(repo, &["rev-parse", "HEAD"]),
        git(repo, &["ls-files", "-s"]),
        String::from_utf8_lossy(&read(repo, "a.rs")),
        git(repo, &["stash", "list", "--format=%H %gs"]),
    )
}

/// The switch to [`SWITCHED_TO`] with his edits set aside, as a kill
/// between the switch and the put-back leaves it: the entry's pending line
/// only, `HEAD` on the new branch with a clean tree, and the set-aside in
/// the stash list under Mergewright's tag. Made by running the real switch, then
/// taking back what came after the kill — the outcome line, the put-back —
/// so every field of the pending line is the app's own.
fn interrupted_switch(repo: &Path) {
    let journal_file = in_dir(repo, Generation::Graft, JOURNAL);
    let pending_from = std::fs::read(&journal_file)
        .unwrap_or_else(|why| panic!("{why}"))
        .len();
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(format!("refs/heads/{SWITCHED_TO}")),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: OnEdits::SetAside,
    });
    run(repo, &op, 19);
    let written = std::fs::read(&journal_file).unwrap_or_else(|why| panic!("{why}"));
    let lines: Vec<&[u8]> = written[pending_from..]
        .split_inclusive(|byte| *byte == b'\n')
        .collect();
    let [pending, _outcome] = lines.as_slice() else {
        panic!(
            "the switch wrote a pending and an outcome line: {}",
            String::from_utf8_lossy(&written[pending_from..])
        );
    };
    let stash = match &journal(repo)
        .last()
        .and_then(|entry| entry.put_back.clone())
    {
        Some(put_back) => put_back.stash.to_string(),
        None => panic!("the switch set his edits aside"),
    };
    // The stash as a Mergewright build wrote it: its messages tagged `graft:`
    // and, so captures hold still, its commits dated as the fixture is.
    let mergewright_stash = as_graft_wrote(repo, &stash);
    let pending = String::from_utf8_lossy(pending).replace(&stash, &mergewright_stash);
    let mut cut = written[..pending_from].to_vec();
    cut.extend_from_slice(pending.as_bytes());
    std::fs::write(&journal_file, cut).unwrap_or_else(|why| panic!("{why}"));
    let last = journal(repo).pop().unwrap_or_else(|| panic!("an entry"));
    assert_eq!(last.status, EntryStatus::Pending, "{last:?}");

    git(repo, &["reset", "-q", "--hard", "HEAD"]);
    git(repo, &["stash", "clear"]);
    let subject = git(repo, &["log", "-1", "--format=%s", &mergewright_stash]);
    let message = subject
        .split_once(": ")
        .map_or(subject.as_str(), |(_, message)| message);
    assert!(message.starts_with("graft: your edits"), "{subject}");
    git(repo, &["stash", "store", "-m", message, &mergewright_stash]);
}

/// `commit` and its parents written again with this build's tag in their
/// messages turned into Mergewright's and the fixture's date: the commit a Mergewright
/// build's set-aside made. Its id.
fn as_graft_wrote(repo: &Path, commit: &str) -> String {
    let date = format!("{} +0000", PINNED_NOW - 2 * 86_400);
    let current = format!("{}: ", Generation::CURRENT.dir());
    let graft = format!("{}: ", Generation::Graft.dir());
    let text = git(repo, &["cat-file", "commit", commit]);
    let mut out = String::new();
    let mut in_message = false;
    for line in text.lines() {
        let line = if in_message {
            line.replace(&current, &graft)
        } else if let Some(parent) = line.strip_prefix("parent ") {
            format!("parent {}", as_graft_wrote(repo, parent))
        } else if line.starts_with("author ") || line.starts_with("committer ") {
            let (who, _) = line.rsplit_once("> ").unwrap_or_else(|| panic!("{line}"));
            format!("{who}> {date}")
        } else {
            in_message = line.is_empty();
            line.to_owned()
        };
        out.push_str(&line);
        out.push('\n');
    }
    let mut child = Command::new("git")
        .args(["hash-object", "-t", "commit", "-w", "--stdin"])
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|why| panic!("hash-object: {why}"));
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap_or_else(|| panic!("stdin"));
        stdin
            .write_all(out.as_bytes())
            .unwrap_or_else(|why| panic!("{why}"));
    }
    let done = child
        .wait_with_output()
        .unwrap_or_else(|why| panic!("{why}"));
    assert!(done.status.success(), "hash-object");
    String::from_utf8_lossy(&done.stdout).trim().to_owned()
}
