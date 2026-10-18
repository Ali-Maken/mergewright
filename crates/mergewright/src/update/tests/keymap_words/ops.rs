//! mergewright-ops' words (m8/10e): every line that names a key, spelled through
//! `OpKeys` from his keymap.

use std::path::PathBuf;

use mergewright_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use mergewright_core::oid::Oid;
use mergewright_git::remote::RemoteFailure;
use mergewright_git::run::RunError;
use mergewright_git::state::InProgress;
use mergewright_ops::apply::{AfterState, ApplyError};
use mergewright_ops::keys::{KeyNames, NoKeys, OpKey};
use mergewright_ops::operation::{Refusal, RemoteNames, RewriteKey};
use mergewright_ops::restore::RestoreError;
use mergewright_ops::undo::{Direction, Drift, UndoError};
use mergewright_ui::keymap::Keymap;

use super::{label, letter, swept, unbinding, whole_tokens};
use crate::update::op_keys::OpKeys;

/// One site: what it words, the keys it names, and its line with each key
/// spelled by `spell` — the pre-10e literal when `spell` is [`letter`].
struct Site {
    name: &'static str,
    keys: &'static [OpKey],
    words: fn(&dyn KeyNames) -> String,
    expected: fn(&dyn Fn(OpKey) -> String) -> String,
}

fn commit() -> Oid {
    Oid::from_sha1([0xab; 20])
}

fn entry(kind: OperationKind, description: &str) -> Box<JournalEntry> {
    Box::new(JournalEntry {
        id: EntryId::new(1, [0; 10]),
        at_unix_ms: 0,
        kind,
        description: description.to_owned(),
        status: EntryStatus::Committed,
        touches: Touches::default(),
        head_before: HeadRef::Unborn("refs/heads/main".to_owned()),
        head_after: None,
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: None,
        pins: Vec::new(),
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    })
}

fn push_failed(failure: RemoteFailure) -> ApplyError {
    ApplyError::Remote {
        entry: entry(OperationKind::Push, "push main to origin/main"),
        failure,
        stderr_tail: Vec::new(),
        names: Box::new(RemoteNames {
            local: "main".to_owned(),
            remote_side: "origin/main".to_owned(),
            remote: "origin".to_owned(),
        }),
        moves_unread: None,
    }
}

fn bad_ref() -> RestoreError {
    RestoreError::BadRefName {
        name: "refs/heads/x..y".to_owned(),
        reason: "two dots".to_owned(),
    }
}

fn cancelled_partway() -> RestoreError {
    RestoreError::CancelledPartway {
        applied: vec!["main".to_owned()],
        rollback: Box::new(bad_ref()),
    }
}

fn failed_partway() -> RestoreError {
    RestoreError::FailedPartway {
        failure: Box::new(bad_ref()),
        applied: vec!["main".to_owned()],
        rollback: Box::new(bad_ref()),
    }
}

fn drift(direction: Direction) -> UndoError {
    UndoError::Drift {
        drift: Box::new(Drift {
            head: true,
            ..Drift::default()
        }),
        direction,
    }
}

fn not_on_your_branch(by: RewriteKey) -> Refusal {
    Refusal::NotOnYourBranch {
        commit: commit(),
        by,
    }
}

fn in_the_way() -> Refusal {
    Refusal::InTheWay {
        doing: "switching to other".to_owned(),
        ignored: Vec::new(),
        untracked: vec![PathBuf::from("new.txt")],
    }
}

fn git_failed() -> ApplyError {
    ApplyError::Git {
        entry: entry(OperationKind::Checkout, "checkout fix"),
        error: RunError::Stopped,
        recoverable: true,
        edits: None,
        paused: None,
    }
}

fn paused() -> ApplyError {
    ApplyError::Paused {
        entry: entry(OperationKind::Merge, "merge feature into main"),
        kind: InProgress::Merge,
        paths: Vec::new(),
    }
}

fn conflicted(paths: Vec<PathBuf>) -> ApplyError {
    ApplyError::Conflicted {
        entry: entry(OperationKind::Merge, "merge feature into main"),
        kind: InProgress::Merge,
        paths,
    }
}

/// Every mergewright-ops line that names a key: 29 sites.
const SITES: &[Site] = &[
    Site {
        name: "Refusal::HeadUnborn",
        keys: &[OpKey::Commit],
        words: |k| Refusal::HeadUnborn.explain(k),
        expected: |s| {
            format!(
                "HEAD has no commit yet · {} makes the first one",
                s(OpKey::Commit)
            )
        },
    },
    Site {
        name: "Refusal::LeaseStale",
        keys: &[OpKey::Fetch],
        words: |k| {
            Refusal::LeaseStale {
                remote_side: "origin/main".to_owned(),
            }
            .explain(k)
        },
        expected: |s| {
            format!(
                "force-push refused: origin/main is not where the box said (fetched since, or \
                 never fetched) · {} to fetch, then look",
                s(OpKey::Fetch)
            )
        },
    },
    Site {
        name: "Refusal::DirtyTree",
        keys: &[OpKey::Stash],
        words: |k| Refusal::DirtyTree { count: 2 }.explain(k),
        expected: |s| {
            format!(
                "needs a clean tree · {} stashes 2 changed files first",
                s(OpKey::Stash)
            )
        },
    },
    Site {
        name: "Refusal::CannotSkip",
        keys: &[OpKey::Continue, OpKey::Abort],
        words: |k| Refusal::CannotSkip(InProgress::Merge).explain(k),
        expected: |s| {
            format!(
                "a merge has nothing to skip · {} continues, {} aborts",
                s(OpKey::Continue),
                s(OpKey::Abort)
            )
        },
    },
    Site {
        name: "Refusal::MessageNotHere",
        keys: &[OpKey::Continue],
        words: |k| {
            Refusal::MessageNotHere {
                kind: InProgress::Rebase,
            }
            .explain(k)
        },
        expected: |s| {
            format!(
                "a rebase keeps each commit's own message · {} continues",
                s(OpKey::Continue)
            )
        },
    },
    Site {
        name: "Refusal::NothingStagedFor",
        keys: &[OpKey::Stage],
        words: |k| Refusal::NothingStagedFor { target: commit() }.explain(k),
        expected: |s| {
            format!(
                "{} · {} stages",
                Refusal::NothingStagedFor { target: commit() }.explain(&NoKeys),
                s(OpKey::Stage)
            )
        },
    },
    Site {
        name: "Refusal::NotOnABranch",
        keys: &[OpKey::Switch],
        words: |k| Refusal::NotOnABranch.explain(k),
        expected: |s| {
            format!(
                "you're not on a branch · switch first ({})",
                s(OpKey::Switch)
            )
        },
    },
    Site {
        name: "Refusal::NothingToAbsorb",
        keys: &[OpKey::Commit],
        words: |k| Refusal::NothingToAbsorb.explain(k),
        expected: |s| {
            format!(
                "none of your staged lines belong to a commit on your branch · {} commits them",
                s(OpKey::Commit)
            )
        },
    },
    Site {
        name: "Refusal::AllPushed",
        keys: &[OpKey::Commit],
        words: |k| {
            Refusal::AllPushed {
                branch: "main".to_owned(),
            }
            .explain(k)
        },
        expected: |s| {
            format!(
                "every commit on main is already pushed · absorb only changes unpushed commits \
                 · {} commits them",
                s(OpKey::Commit)
            )
        },
    },
    Site {
        name: "Refusal::NoCommitsOfItsOwn",
        keys: &[OpKey::Commit],
        words: |k| {
            Refusal::NoCommitsOfItsOwn {
                branch: "main".to_owned(),
            }
            .explain(k)
        },
        expected: |s| {
            format!(
                "main has no commits of its own yet · {} commits them",
                s(OpKey::Commit)
            )
        },
    },
    Site {
        name: "Refusal::AlreadyPushed",
        keys: &[OpKey::Amend],
        words: |k| Refusal::AlreadyPushed { commit: commit() }.explain(k),
        expected: |s| {
            format!(
                "{} · {} only changes older commits you haven't pushed",
                Refusal::AlreadyPushed { commit: commit() }.explain(&NoKeys),
                s(OpKey::Amend)
            )
        },
    },
    Site {
        name: "Refusal::BeyondReach",
        keys: &[OpKey::Amend],
        words: |k| {
            Refusal::BeyondReach {
                commit: commit(),
                branch: "main".to_owned(),
            }
            .explain(k)
        },
        expected: |s| {
            let depth = mergewright_ops::rewrite::ABSORB_FALLBACK_DEPTH;
            format!(
                "{} · on main {} reaches the last {depth} unpushed commits",
                Refusal::BeyondReach {
                    commit: commit(),
                    branch: "main".to_owned(),
                }
                .explain(&NoKeys),
                s(OpKey::Amend)
            )
        },
    },
    Site {
        name: "Refusal::NotOnYourBranch (R)",
        keys: &[OpKey::Rewrite],
        words: |k| not_on_your_branch(RewriteKey::Open).explain(k),
        expected: |s| {
            format!(
                "{} rewrites your branch · {}",
                s(OpKey::Rewrite),
                not_on_your_branch(RewriteKey::Open).explain(&NoKeys)
            )
        },
    },
    Site {
        name: "Refusal::NotOnYourBranch (a)",
        keys: &[OpKey::Amend],
        words: |k| not_on_your_branch(RewriteKey::AmendInto).explain(k),
        expected: |s| {
            format!(
                "{} puts staged changes into a commit on your branch · {}",
                s(OpKey::Amend),
                not_on_your_branch(RewriteKey::AmendInto).explain(&NoKeys)
            )
        },
    },
    Site {
        name: "Refusal::InTheWay",
        keys: &[OpKey::Stash],
        words: |k| in_the_way().explain(k),
        expected: |s| {
            format!(
                "{} · {} stashes it first",
                in_the_way().explain(&NoKeys),
                s(OpKey::Stash)
            )
        },
    },
    Site {
        name: "Refusal::NoUpstream",
        keys: &[OpKey::Push],
        words: |k| {
            Refusal::NoUpstream {
                branch: "main".to_owned(),
            }
            .explain(k)
        },
        expected: |s| {
            format!(
                "main has no upstream — {} pushes and sets one",
                s(OpKey::Push)
            )
        },
    },
    Site {
        name: "Refusal::UpstreamChanged",
        keys: &[OpKey::Pull],
        words: |k| Refusal::UpstreamChanged.explain(k),
        expected: |s| {
            format!(
                "the branch's upstream changed since it was read — press {} again",
                s(OpKey::Pull)
            )
        },
    },
    Site {
        name: "Refusal::NotFastForward",
        keys: &[OpKey::Merge],
        words: |k| {
            Refusal::NotFastForward {
                branch: "main".to_owned(),
            }
            .explain(k)
        },
        expected: |s| {
            format!(
                "main has diverged; a fast-forward is not possible — {} merges instead",
                s(OpKey::Merge)
            )
        },
    },
    Site {
        name: "AfterState::StagingNotKept",
        keys: &[OpKey::Undo],
        words: |k| {
            AfterState::StagingNotKept {
                paths: vec![PathBuf::from("f.txt")],
            }
            .explain(k)
        },
        expected: |s| {
            format!(
                "your edits are back · staging not kept: f.txt · {} undoes it, staging included",
                s(OpKey::Undo)
            )
        },
    },
    Site {
        name: "ApplyError::Git (recoverable)",
        keys: &[OpKey::Undo],
        words: |k| git_failed().explain(k),
        expected: |s| {
            format!(
                "{} · {} restores the backup",
                git_failed().explain(&NoKeys),
                s(OpKey::Undo)
            )
        },
    },
    Site {
        name: "ApplyError::Paused",
        keys: &[OpKey::Continue, OpKey::Abort],
        words: |k| paused().explain(k),
        expected: |s| {
            format!(
                "merge feature into main: stopped · the merge is paused · {} continues, {} aborts",
                s(OpKey::Continue),
                s(OpKey::Abort)
            )
        },
    },
    Site {
        name: "ApplyError::Conflicted (hook)",
        keys: &[OpKey::Continue, OpKey::Abort],
        words: |k| conflicted(Vec::new()).explain(k),
        expected: |s| {
            format!(
                "merge feature into main: a git hook stopped it · {} continues, {} aborts",
                s(OpKey::Continue),
                s(OpKey::Abort)
            )
        },
    },
    Site {
        name: "ApplyError::Conflicted",
        keys: &[OpKey::Continue, OpKey::Abort],
        words: |k| conflicted(vec![PathBuf::from("f.txt")]).explain(k),
        expected: |s| {
            format!(
                "merge feature into main: 1 unmerged · resolve, then {} continues, {} aborts",
                s(OpKey::Continue),
                s(OpKey::Abort)
            )
        },
    },
    Site {
        name: "remote: push rejected",
        keys: &[OpKey::Pull, OpKey::Merge, OpKey::Rebase, OpKey::Push],
        words: |k| push_failed(RemoteFailure::NonFastForward).explain(k),
        expected: |s| {
            format!(
                "push rejected: origin/main has commits you don't · {} pull, then {} or {} · {} \
                 force with lease",
                s(OpKey::Pull),
                s(OpKey::Merge),
                s(OpKey::Rebase),
                s(OpKey::Push)
            )
        },
    },
    Site {
        name: "remote: push refused (stale lease)",
        keys: &[OpKey::Fetch],
        words: |k| push_failed(RemoteFailure::StaleInfo).explain(k),
        expected: |s| {
            format!(
                "push refused: origin/main moved since your last fetch (stale lease) · {} to \
                 fetch, then look",
                s(OpKey::Fetch)
            )
        },
    },
    Site {
        name: "RestoreError::CancelledPartway",
        keys: &[OpKey::Redo],
        words: |k| cancelled_partway().explain(k),
        expected: |s| {
            format!(
                "{} · {} restores the backup",
                cancelled_partway().explain(&NoKeys),
                s(OpKey::Redo)
            )
        },
    },
    Site {
        name: "RestoreError::FailedPartway",
        keys: &[OpKey::Redo],
        words: |k| failed_partway().explain(k),
        expected: |s| {
            format!(
                "{} · {} restores the backup",
                failed_partway().explain(&NoKeys),
                s(OpKey::Redo)
            )
        },
    },
    Site {
        name: "UndoError::Drift (u)",
        keys: &[OpKey::Undo],
        words: |k| drift(Direction::Undo).explain(k),
        expected: |s| {
            format!(
                "{} — {} again to go ahead",
                drift(Direction::Undo).explain(&NoKeys),
                s(OpKey::Undo)
            )
        },
    },
    Site {
        name: "UndoError::Drift (U)",
        keys: &[OpKey::Redo],
        words: |k| drift(Direction::Redo).explain(k),
        expected: |s| {
            format!(
                "{} — {} again to go ahead",
                drift(Direction::Redo).explain(&NoKeys),
                s(OpKey::Redo)
            )
        },
    },
];

/// Pin: each site under the shipped keymap is the pre-10e literal, byte for
/// byte (W6); under the swept keymap it names each moved key's new label
/// and no default letter of it as a whole token (W1).
#[test]
fn each_ops_word_spells_the_remapped_key() {
    assert_eq!(SITES.len(), 29, "every mergewright-ops site that names a key");
    let shipped = Keymap::shipped();
    let swept = swept();
    for site in SITES {
        assert_eq!(
            (site.words)(&OpKeys(shipped)),
            (site.expected)(&|key| letter(key).to_owned()),
            "{}: shipped keymap, byte-identical",
            site.name
        );
        let line = (site.words)(&OpKeys(&swept));
        assert_eq!(
            line,
            (site.expected)(&|key| label(&swept, key)),
            "{}: swept keymap",
            site.name
        );
        // The same line with the keys spelled as nothing: how often each
        // default letter stands alone in the words themselves (`a merge`).
        let bare = (site.expected)(&|_| String::new());
        for key in site.keys {
            let moved = label(&swept, *key);
            assert_ne!(moved, letter(*key), "{}: {key:?} moved", site.name);
            assert!(line.contains(&moved), "{}: {line}", site.name);
            assert_eq!(
                whole_tokens(&line, letter(*key)),
                whole_tokens(&bare, letter(*key)),
                "{}: no default {:?} left: {line}",
                site.name,
                letter(*key)
            );
        }
    }
}

/// W3: an unbound key drops its clause, alternatives keep the bound ones,
/// and no `—` stands in for a missing key.
#[test]
fn an_unbound_key_drops_its_clause() {
    let no_merge = unbinding("graph.merge");
    assert_eq!(
        push_failed(RemoteFailure::NonFastForward).explain(&OpKeys(&no_merge)),
        "push rejected: origin/main has commits you don't · p pull, then r · P force with lease"
    );
    let no_stash = unbinding("graph.stash");
    assert_eq!(
        Refusal::DirtyTree { count: 2 }.explain(&OpKeys(&no_stash)),
        "needs a clean tree"
    );
    let words = in_the_way().explain(&OpKeys(&no_stash));
    assert_eq!(words, in_the_way().explain(&NoKeys));
    assert!(!words.contains("stashes"), "{words}");
    for site in SITES {
        let keyless = (site.words)(&NoKeys);
        assert!(!keyless.contains(" · ·"), "{}: {keyless}", site.name);
        assert!(!keyless.ends_with(" · "), "{}: {keyless}", site.name);
        // A `—` is only ever the words' own dash before a key clause, so
        // with no key there is none.
        assert!(!keyless.contains('—'), "{}: {keyless}", site.name);
    }
}
