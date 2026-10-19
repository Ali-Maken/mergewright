//! The four answers' words, one row per variant (ease-pass 09 Scope; mock
//! G12 words, 01r's words for the states the mock never shows).

use mergewright_core::commit::Timestamp;
use mergewright_ops::branch_status::{Behind, BranchStatus, CandidateKind, Merged, Pushed, Who};
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
    said(&pane_words(status, kind, "main", NOW)[nth])
}

fn heads(name: &str) -> String {
    format!("refs/heads/{name}")
}

#[test]
fn pushed_words() {
    let table = [
        (Pushed::NoUpstream, Mark::Attention, "not pushed yet"),
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
                first: vec![b"Sara Khan".to_vec()],
                others: 0,
                last: at(NOW - 14 * DAY),
            },
            "Sara, last commit 2w ago",
        ),
        (
            Who::Authors {
                first: vec![b"Sara Khan".to_vec(), b"Omar".to_vec()],
                others: 0,
                last: at(NOW - 23 * 3600),
            },
            "Sara and Omar, last commit 23h ago",
        ),
        (
            Who::Authors {
                first: vec![b"Sara".to_vec(), b"Omar".to_vec()],
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
