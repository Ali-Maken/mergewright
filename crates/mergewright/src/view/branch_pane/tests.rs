//! The four answers' words, one row per variant (ease-pass 09 Scope; mock
//! G12 words, 01r's words for the states the mock never shows).

use mergewright_core::commit::Timestamp;
use mergewright_git::mailmap::Mailmap;
use mergewright_ops::branch_status::{
    Author, Behind, BranchStatus, CandidateKind, Merged, Pushed, Who,
};
use mergewright_ui::branch_pane::{Mark, PaneLine};

use super::pane_words;

const NOW: i64 = 1_700_000_000;
const DAY: i64 = 86_400;

fn at(seconds: i64) -> Timestamp {
    Timestamp {
        seconds,
        offset_seconds: 0,
    }
}

fn base() -> BranchStatus {
    BranchStatus {
        route: Vec::new(),
        pushed: Pushed::InSync,
        merged: Merged::NotYet { into: None },
        behind: Behind::NoBase,
        who: Who::NoOwnCommits,
    }
}

fn said(line: &PaneLine) -> (Mark, String) {
    match line {
        PaneLine::Known { mark, words } => (*mark, words.as_str().to_owned()),
        other => panic!("an answer, not {other:?}"),
    }
}

/// The line `nth` (0 pushed, 1 merged, 2 behind, 3 who) for `status`.
fn line(status: &BranchStatus, kind: CandidateKind, nth: usize) -> (Mark, String) {
    said(&pane_words(status, (kind, "main"), NOW, None)[nth])
}

/// An author named `name` at an address of their own.
fn author(name: &str) -> Author {
    Author {
        name: name.to_owned(),
        email: format!("{}@example.com", name.to_lowercase().replace(' ', ".")),
    }
}

fn heads(name: &str) -> String {
    format!("refs/heads/{name}")
}

#[test]
fn pushed_words() {
    let table = [
        (Pushed::NoUpstream, Mark::Attention, "no upstream set"),
        (Pushed::InSync, Mark::Fine, "pushed"),
        (Pushed::ToPush(1), Mark::Attention, "1 commit to push"),
        (Pushed::ToPull(3), Mark::Attention, "3 new commits to pull"),
        (
            Pushed::Both { push: 2, pull: 1 },
            Mark::Attention,
            "2 to push, 1 to pull",
        ),
    ];
    for (pushed, mark, words) in table {
        let status = BranchStatus { pushed, ..base() };
        assert_eq!(
            line(&status, CandidateKind::Other, 0),
            (mark, words.to_owned())
        );
    }
}

#[test]
fn merged_words() {
    let table = [
        (
            Merged::MainLine,
            CandidateKind::Trunk,
            "the main line: branches merge into it",
        ),
        (
            Merged::Into(heads("development")),
            CandidateKind::Other,
            "merged into development",
        ),
        (
            Merged::NotYet {
                into: Some(heads("wallet-release")),
            },
            CandidateKind::Other,
            "not merged",
        ),
        (
            Merged::NotYet {
                into: Some(heads("development")),
            },
            CandidateKind::Integration,
            "not merged into development",
        ),
        (
            Merged::NotYet { into: None },
            CandidateKind::Other,
            "not merged",
        ),
        (
            Merged::NothingToMerge,
            CandidateKind::Other,
            "nothing of its own to merge",
        ),
    ];
    for (merged, kind, words) in table {
        let mark = if matches!(merged, Merged::Into(_)) {
            Mark::Fine
        } else {
            Mark::Plain
        };
        let status = BranchStatus { merged, ..base() };
        assert_eq!(line(&status, kind, 1), (mark, words.to_owned()));
    }
}

#[test]
fn behind_words() {
    let table = [
        (
            Behind::NoBase,
            CandidateKind::Other,
            Mark::Plain,
            "shares no history with main",
        ),
        (
            Behind::NoBase,
            CandidateKind::Trunk,
            Mark::Plain,
            "no upstream to compare with",
        ),
        (
            Behind::UpToDate {
                base: heads("integration"),
            },
            CandidateKind::Other,
            Mark::Fine,
            "up to date with integration",
        ),
        (
            Behind::Behind {
                base: heads("development"),
                count: 4,
            },
            CandidateKind::Other,
            Mark::Attention,
            "development has 4 new commits since it left",
        ),
        (
            Behind::Behind {
                base: "refs/remotes/origin/main".to_owned(),
                count: 1,
            },
            CandidateKind::Trunk,
            Mark::Attention,
            "origin/main has 1 new commit since it left",
        ),
    ];
    for (behind, kind, mark, words) in table {
        let status = BranchStatus { behind, ..base() };
        assert_eq!(line(&status, kind, 2), (mark, words.to_owned()));
    }
}

#[test]
fn who_words() {
    let table = [
        (
            Who::Everyone {
                last: at(NOW - 10 * DAY),
            },
            "everyone, last commit 10d ago",
        ),
        (
            Who::Authors {
                first: vec![author("Sara Khan")],
                others: 0,
                last: at(NOW - 14 * DAY),
            },
            "Sara, last commit 2w ago",
        ),
        (
            Who::Authors {
                first: vec![author("Sara Khan"), author("Omar")],
                others: 0,
                last: at(NOW - 23 * 3600),
            },
            "Sara and Omar, last commit 23h ago",
        ),
        (
            Who::Authors {
                first: vec![author("Sara"), author("Omar")],
                others: 3,
                last: at(NOW),
            },
            "Sara, Omar and 3 others, last commit just now",
        ),
        (Who::NoOwnCommits, "no commits of its own"),
    ];
    for (who, words) in table {
        let status = BranchStatus { who, ..base() };
        assert_eq!(
            line(&status, CandidateKind::Other, 3),
            (Mark::Plain, words.to_owned())
        );
    }
}

/// The who line for one author `who`, under `mailmap`.
fn who_line(who: Author, mailmap: Option<&Mailmap>) -> String {
    let status = BranchStatus {
        who: Who::Authors {
            first: vec![who],
            others: 0,
            last: at(NOW - 23 * 3600),
        },
        ..base()
    };
    said(&pane_words(&status, (CandidateKind::Other, "main"), NOW, mailmap)[3]).1
}

/// A repository whose `.mailmap` joins `ali@home.example` to
/// `ali@work.example`, and whose `user.email` is the latter when `email`;
/// its mailmap as the app reads it (`RepoHandle::mailmap`).
fn ali_mailmap(email: bool) -> Mailmap {
    let dir = tempfile::tempdir().expect("a temp dir");
    let run = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("the tool runs");
        assert!(status.success(), "{args:?}");
    };
    run(&["init", "-q"]);
    if email {
        run(&["config", "user.email", "ali@work.example"]);
    }
    std::fs::write(
        dir.path().join(".mailmap"),
        "Ali Reza <ali@work.example> <ali@home.example>\n",
    )
    .expect(".mailmap written");
    mergewright_git::repo::RepoHandle::open(dir.path())
        .expect("opens")
        .mailmap()
}

/// ease-pass 09 ruling Y: the user's own author reads `you`, also under a
/// second address the mailmap joins to his; with no `user.email`, his first
/// name, and nothing reads as an error.
#[test]
fn who_says_you_for_the_users_own_authors() {
    let work = Author {
        name: "Ali Reza".to_owned(),
        email: "ali@work.example".to_owned(),
    };
    let home = Author {
        name: "ali".to_owned(),
        email: "ali@home.example".to_owned(),
    };
    let mailmap = ali_mailmap(true);
    assert_eq!(
        who_line(work.clone(), Some(&mailmap)),
        "you, last commit 23h ago"
    );
    assert_eq!(who_line(home, Some(&mailmap)), "you, last commit 23h ago");
    assert_eq!(
        who_line(author("Sara Khan"), Some(&mailmap)),
        "Sara, last commit 23h ago"
    );
    let unset = ali_mailmap(false);
    assert_eq!(
        who_line(work.clone(), Some(&unset)),
        "Ali, last commit 23h ago"
    );
    assert_eq!(who_line(work, None), "Ali, last commit 23h ago");
}
