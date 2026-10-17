//! The base choice, pure; the fixture repositories the route (`route`),
//! footing (`footing`) and status (`status`) tests share.

use std::path::Path;
use std::process::Command;

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use proptest::prelude::*;

use super::{Apart, Candidate, CandidateKind, choose_base, short_name};

#[test]
fn short_name_table() {
    let cases = [
        ("refs/heads/main", "main"),
        ("refs/heads/sara/fix-toc", "sara/fix-toc"),
        ("refs/remotes/origin/main", "origin/main"),
        ("refs/tags/v1.2", "v1.2"),
        ("main", "main"),
        ("HEAD", "HEAD"),
    ];
    for (full, short) in cases {
        assert_eq!(short_name(full), short, "{full}");
    }
}

fn cand(name: &str, kind: CandidateKind) -> Candidate {
    Candidate {
        name: name.to_owned(),
        tip: Oid::from_hex("1111111111111111111111111111111111111111").expect("an id"),
        kind,
    }
}

fn apart(branch_only: usize, base_only: usize) -> Apart {
    Apart {
        branch_only,
        base_only,
    }
}

/// Worked example: the trunk loses to a nearer integration branch, a
/// candidate at the branch's tip is skipped, the branch itself is skipped.
#[test]
fn choose_base_worked_example() {
    let measured = [
        (
            cand("refs/heads/sara/fix-toc", CandidateKind::Other),
            apart(1, 0),
        ),
        (cand("refs/heads/main", CandidateKind::Trunk), apart(5, 2)),
        (
            cand("refs/heads/wallet-release", CandidateKind::Integration),
            apart(2, 7),
        ),
        (cand("refs/heads/twin", CandidateKind::Other), apart(0, 0)),
    ];
    let base =
        choose_base("refs/heads/sara/fix-toc", &measured, CandidateKind::Other).expect("a base");
    assert_eq!(base.name, "refs/heads/wallet-release");
}

#[test]
fn choose_base_ties_go_trunk_then_integration_then_name() {
    let measured = [
        (cand("refs/heads/b", CandidateKind::Other), apart(3, 0)),
        (cand("refs/heads/a", CandidateKind::Other), apart(3, 0)),
        (
            cand("refs/heads/rel", CandidateKind::Integration),
            apart(3, 0),
        ),
        (cand("refs/heads/main", CandidateKind::Trunk), apart(3, 0)),
    ];
    let base = choose_base("refs/heads/x", &measured, CandidateKind::Other).expect("a base");
    assert_eq!(base.name, "refs/heads/main");
    let base = choose_base("refs/heads/x", &measured[..3], CandidateKind::Other).expect("a base");
    assert_eq!(base.name, "refs/heads/rel");
    let base = choose_base("refs/heads/x", &measured[..2], CandidateKind::Other).expect("a base");
    assert_eq!(base.name, "refs/heads/a");
}

#[test]
fn choose_base_none_when_every_candidate_holds_the_branch() {
    let measured = [
        (cand("refs/heads/main", CandidateKind::Trunk), apart(0, 4)),
        (cand("refs/heads/x", CandidateKind::Other), apart(2, 0)),
    ];
    assert_eq!(
        choose_base("refs/heads/x", &measured, CandidateKind::Other),
        None
    );
}

fn kind() -> impl Strategy<Value = CandidateKind> {
    prop_oneof![
        Just(CandidateKind::Trunk),
        Just(CandidateKind::Integration),
        Just(CandidateKind::Other),
    ]
}

fn measured() -> impl Strategy<Value = Vec<(Candidate, Apart)>> {
    prop::collection::vec(
        (0u8..6, kind(), 0usize..4, 0usize..4).prop_map(|(n, kind, branch_only, base_only)| {
            (
                cand(&format!("refs/heads/b{n}"), kind),
                apart(branch_only, base_only),
            )
        }),
        0..8,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(graft_fixtures::proptest_cases()))]

    /// The answer is never the branch itself, never a candidate holding
    /// the branch, never above `at_most`, and never an ordinary branch with
    /// commits the branch lacks (R1); no eligible candidate has fewer
    /// branch-only commits; an eligible one with as few sorts after it by
    /// kind, then name.
    #[test]
    fn choose_base_picks_the_nearest(measured in measured(), me in 0u8..6, at_most in kind()) {
        let branch = format!("refs/heads/b{me}");
        let eligible: Vec<&(Candidate, Apart)> = measured
            .iter()
            .filter(|(c, a)| {
                c.name != branch
                    && a.branch_only != 0
                    && c.kind <= at_most
                    && (c.kind != CandidateKind::Other || a.base_only == 0)
            })
            .collect();
        match choose_base(&branch, &measured, at_most) {
            None => prop_assert!(eligible.is_empty()),
            Some(base) => {
                prop_assert_ne!(&base.name, &branch);
                let (_, mine) = measured
                    .iter()
                    .find(|(c, _)| std::ptr::eq(c, base))
                    .expect("the answer is one of the inputs");
                prop_assert_ne!(mine.branch_only, 0);
                prop_assert!(base.kind <= at_most, "{base:?} is above {at_most:?}");
                prop_assert!(
                    base.kind != CandidateKind::Other || mine.base_only == 0,
                    "{base:?} is an ordinary branch with {} commits of its own",
                    mine.base_only
                );
                for (other, theirs) in eligible {
                    prop_assert!(theirs.branch_only >= mine.branch_only);
                    if theirs.branch_only == mine.branch_only {
                        prop_assert!(
                            (other.kind, &other.name) >= (base.kind, &base.name),
                            "{other:?} should have won over {base:?}"
                        );
                    }
                }
            }
        }
    }
}

// ---- fixture repositories ------------------------------------------------

/// A scratch repository with a clock: every commit is one minute after the
/// last, so "newest" is never a tie.
struct Repo {
    dir: tempfile::TempDir,
    clock: std::cell::Cell<i64>,
}

const ALICE: (&str, &str) = ("Alice Ames", "alice@example.com");

impl Repo {
    fn new() -> Self {
        let repo = Repo {
            dir: tempfile::tempdir().expect("a scratch directory"),
            clock: std::cell::Cell::new(1_700_000_000),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn git(&self, args: &[&str]) -> String {
        self.git_as(ALICE, args)
    }

    fn git_as(&self, (name, email): (&str, &str), args: &[&str]) -> String {
        let when = format!("{} +0000", self.clock.get());
        let out = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .env("GIT_AUTHOR_NAME", name)
            .env("GIT_AUTHOR_EMAIL", email)
            .env("GIT_COMMITTER_NAME", name)
            .env("GIT_COMMITTER_EMAIL", email)
            .env("GIT_AUTHOR_DATE", &when)
            .env("GIT_COMMITTER_DATE", &when)
            .output()
            .expect("git runs");
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout)
            .expect("utf-8")
            .trim()
            .to_owned()
    }

    /// One commit on the checked-out branch, by `who`.
    fn commit_as(&self, who: (&str, &str), name: &str) {
        self.clock.set(self.clock.get() + 60);
        std::fs::write(self.path().join(format!("{name}.txt")), name).expect("write");
        self.git_as(who, &["add", "-A"]);
        self.git_as(who, &["commit", "-q", "-m", name]);
    }

    fn commit(&self, name: &str) {
        self.commit_as(ALICE, name);
    }

    fn oid(&self, rev: &str) -> Oid {
        Oid::from_hex(&self.git(&["rev-parse", rev])).expect("an id")
    }

    fn cand(&self, branch: &str, kind: CandidateKind) -> Candidate {
        Candidate {
            name: format!("refs/heads/{branch}"),
            tip: self.oid(branch),
            kind,
        }
    }

    fn handle(&self) -> RepoHandle {
        RepoHandle::open(self.path()).expect("it opens")
    }
}

/// `main ← wallet-release ← sara/fix-toc`, plus `ops/ci` off `main`:
///
/// ```text
/// m1 m2 ─ main
///     └ w1 w2 ─ wallet-release
///           └ s1 s2 ─ sara/fix-toc
///     └ o1 ─ ops/ci
/// ```
fn stack() -> Repo {
    let repo = Repo::new();
    repo.commit("m1");
    repo.commit("m2");
    repo.git(&["switch", "-q", "-c", "wallet-release"]);
    repo.commit("w1");
    repo.commit("w2");
    repo.git(&["switch", "-q", "-c", "sara/fix-toc"]);
    repo.commit("s1");
    repo.commit("s2");
    repo.git(&["switch", "-q", "-c", "ops/ci", "main"]);
    repo.commit("o1");
    repo.git(&["switch", "-q", "sara/fix-toc"]);
    repo
}

fn stack_candidates(repo: &Repo) -> Vec<Candidate> {
    vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("wallet-release", CandidateKind::Integration),
        repo.cand("sara/fix-toc", CandidateKind::Other),
        repo.cand("ops/ci", CandidateKind::Other),
    ]
}

fn names(route: &[Candidate]) -> Vec<&str> {
    route.iter().map(|c| c.name.as_str()).collect()
}

fn find<'a>(candidates: &'a [Candidate], name: &str) -> &'a Candidate {
    candidates
        .iter()
        .find(|c| c.name == format!("refs/heads/{name}"))
        .expect("a candidate by that name")
}

mod footing;
mod route;
mod status;
