//! `submodule_log` against real submodules (m7-power-tools/13a): the commits
//! a pointer move pulled in or rewound, judged by git's own
//! `git diff --submodule=log` on the same move.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::Path;
use std::sync::atomic::AtomicBool;

use mergewright_core::oid::Oid;
use mergewright_git::run::RunError;
use mergewright_ops::facts::FactsError;
use mergewright_ops::submodule::{SubmoduleLog, submodule_log};

use crate::support::{Never, TempRepo};

const PATH: &str = "libs/parser";

/// A superproject with a real, checked-out submodule at [`PATH`], its
/// source a local directory (`protocol.file.allow=always`: the default is
/// `user` since git 2.38.1; still no network). The submodule holds `a`, `b`
/// on `main`; the superproject's commit `adds parser` records `b`.
struct Super {
    root: TempRepo,
    /// The submodule's source repository; the checkout clones it.
    _source: TempRepo,
}

impl Super {
    fn new() -> Super {
        let source = TempRepo::init();
        source.write("a.txt", b"a\n");
        source.commit_all("a");
        source.write("b.txt", b"b\n");
        source.commit_all("b");

        let root = TempRepo::init();
        root.write("README", b"super\n");
        root.commit_all("root");
        root.git(&[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            &source.path().to_string_lossy(),
            PATH,
        ]);
        root.git(&["commit", "-q", "-m", "adds parser"]);
        root.harden(Path::new(PATH));
        Super {
            root,
            _source: source,
        }
    }

    /// Run `git` inside the submodule's working directory.
    fn sub(&self, args: &[&str]) -> String {
        self.root.git_in(Path::new(PATH), args)
    }

    /// A commit in the submodule touching `name`, returning its id.
    fn sub_commit(&self, name: &str) -> Oid {
        self.root.write(
            &format!("{PATH}/{name}.txt"),
            format!("{name}\n").as_bytes(),
        );
        self.sub(&["add", "-A"]);
        self.sub(&["commit", "-q", "-m", name]);
        oid(&self.sub(&["rev-parse", "HEAD"]))
    }

    /// Record the submodule at `at` in a superproject commit; its id.
    fn record(&self, at: Oid) -> String {
        self.sub(&["checkout", "-q", &at.to_string()]);
        self.root.git(&["add", PATH]);
        self.root
            .git(&["commit", "-q", "-m", &format!("parser at {at}")]);
        self.root.git(&["rev-parse", "HEAD"])
    }
}

fn oid(hex: &str) -> Oid {
    Oid::from_hex(hex.trim()).expect("git printed an object id")
}

fn read(
    repo: &Super,
    from: Oid,
    to: Oid,
    stop: &dyn mergewright_git::run::Stop,
) -> Result<SubmoduleLog, FactsError> {
    submodule_log(&repo.root.open(), Path::new(PATH), from, to, stop)
}

fn subjects(rows: &[(Oid, String)]) -> Vec<&str> {
    rows.iter().map(|(_, subject)| subject.as_str()).collect()
}

#[test]
fn submodule_log_lists_pulled_in_and_rewound() {
    let repo = Super::new();
    let b = oid(&repo.sub(&["rev-parse", "HEAD"]));
    // `e` on a side line off `b` (the rewound side); `c`, `d` on `main`.
    repo.sub(&["checkout", "-q", "-b", "side"]);
    let e = repo.sub_commit("e");
    repo.sub(&["checkout", "-q", "main"]);
    let c = repo.sub_commit("c");
    let d = repo.sub_commit("d");
    assert_ne!(b, c);

    let log = read(&repo, e, d, &Never).expect("the log reads");

    assert_eq!(
        log,
        SubmoduleLog {
            pulled_in: vec![(d, "d".to_owned()), (c, "c".to_owned())],
            rewound: vec![(e, "e".to_owned())],
        }
    );

    // The oracle: git's own summary of the same move, recorded in two
    // superproject commits. `  > <subject>` pulled in, `  < <subject>`
    // rewound, in git's order.
    let before = repo.record(e);
    let after = repo.record(d);
    let git_says = repo.root.git(&["diff", "--submodule=log", &before, &after]);
    let mut pulled = Vec::new();
    let mut rewound = Vec::new();
    for line in git_says.lines().skip(1) {
        match line.trim_start().split_once(' ') {
            Some((">", subject)) => pulled.push(subject),
            Some(("<", subject)) => rewound.push(subject),
            _ => panic!("an unexpected summary line: {line:?} in {git_says}"),
        }
    }
    assert_eq!(subjects(&log.pulled_in), pulled, "git said {git_says}");
    assert_eq!(subjects(&log.rewound), rewound, "git said {git_says}");
}

#[test]
fn a_fast_forward_rewinds_nothing_and_no_move_lists_nothing() {
    let repo = Super::new();
    let b = oid(&repo.sub(&["rev-parse", "HEAD"]));
    let c = repo.sub_commit("c");

    assert_eq!(
        read(&repo, b, c, &Never).expect("the log reads"),
        SubmoduleLog {
            pulled_in: vec![(c, "c".to_owned())],
            rewound: vec![],
        }
    );
    assert_eq!(
        read(&repo, c, b, &Never).expect("the log reads"),
        SubmoduleLog {
            pulled_in: vec![],
            rewound: vec![(c, "c".to_owned())],
        }
    );
    assert_eq!(
        read(&repo, c, c, &Never).expect("the log reads"),
        SubmoduleLog::default()
    );
}

#[test]
fn submodule_log_honours_stop() {
    let repo = Super::new();
    let b = oid(&repo.sub(&["rev-parse", "HEAD"]));
    let c = repo.sub_commit("c");
    let stopped = AtomicBool::new(true);

    let result = read(&repo, b, c, &stopped);

    assert!(matches!(result, Err(FactsError::Stopped)), "{result:?}");
}

/// Measured for 13b (NOTES): a submodule never set up — its directory
/// empty, no `.git` — is git's own refusal, never a read of the
/// superproject's history in its place.
#[test]
fn a_submodule_not_set_up_is_gits_refusal() {
    let repo = Super::new();
    let b = oid(&repo.sub(&["rev-parse", "HEAD"]));
    let c = repo.sub_commit("c");
    // A fresh clone of the superproject alone: the submodule's commits are
    // nowhere in it. The second pair is a commit the *parent* holds, so a
    // read that fell through to the parent's history would succeed on it.
    let parent_head = oid(&repo.root.git(&["rev-parse", "HEAD"]));
    let fresh = TempRepo::init();
    fresh.git(&["fetch", "-q", &repo.root.path().to_string_lossy(), "main"]);
    fresh.git(&["reset", "-q", "--hard", "FETCH_HEAD"]);
    assert!(fresh.exists(PATH), "the gitlink's directory is there");
    assert!(!fresh.exists(&format!("{PATH}/.git")), "but not set up");

    for (from, to) in [(b, c), (parent_head, parent_head)] {
        let result = submodule_log(&fresh.open(), Path::new(PATH), from, to, &Never);
        assert!(
            matches!(
                result,
                Err(FactsError::History(RunError::Exited { status: 128, .. }))
            ),
            "{result:?}"
        );
    }
}

/// Measured for 13b (NOTES): the submodule is set up but lacks a commit
/// the pointer names (not fetched yet) — git's own refusal too.
#[test]
fn a_missing_commit_is_gits_refusal() {
    let repo = Super::new();
    let b = oid(&repo.sub(&["rev-parse", "HEAD"]));
    let absent = Oid::from_hex(&"1".repeat(40)).expect("forty hex digits");

    for (from, to) in [(b, absent), (absent, b)] {
        let result = read(&repo, from, to, &Never);
        assert!(
            matches!(
                result,
                Err(FactsError::History(RunError::Exited { status: 128, .. }))
            ),
            "{result:?}"
        );
    }
}
