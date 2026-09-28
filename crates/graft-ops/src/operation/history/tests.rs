//! Table tests for the history operations: the exact command lines, the
//! browser line, `touches`, the facts each asks for, and every refusal one
//! fact away from a passing baseline.

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_core::commit::Timestamp;
use graft_core::error::RefNameRule;
use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::run::GitArg;
use graft_git::stash::StashEntry;
use graft_git::state::InProgress;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_git::version::GitVersion;

use super::super::{FactsScope, Operation, Refusal, RepoFacts, SetAside, Target};
use super::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    MergeParams, OnEdits, PickParams, RebaseParams, RefDeleteParams, ResetMode, ResetParams,
    ResetTarget, RevertParams, Source, TagCreateParams, Tip,
};

const MAIN: &str = "refs/heads/main";
const FEAT: &str = "refs/heads/feat";

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).unwrap()
}

fn hex(byte: u8) -> String {
    oid(byte).to_string()
}

fn tip(name: &str, byte: u8) -> Tip {
    Tip {
        name: name.to_owned(),
        oid: oid(byte),
    }
}

fn entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: PathBuf::from(path),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

/// On `main` at `0x0a`, `feat` at `0x0b`, `v1` absent, one stash, clean.
fn facts() -> RepoFacts {
    RepoFacts {
        bare: false,
        head: HeadRef::Branch(MAIN.to_owned()),
        in_progress: None,
        status: WorkingTreeStatus { entries: vec![] },
        stashes: vec![StashEntry {
            index: 0,
            commit: oid(0x50),
            message: "On main: wip".to_owned(),
            at: Timestamp {
                seconds: 0,
                offset_seconds: 0,
            },
            paths: vec![PathBuf::from("a.rs")],
        }],
        git: GitVersion {
            major: 2,
            minor: 54,
            patch: 0,
        },
        remotes: vec![],
        upstream: None,
        refs: BTreeMap::from([
            (MAIN.to_owned(), Some(oid(0x0a))),
            (FEAT.to_owned(), Some(oid(0x0b))),
            ("HEAD".to_owned(), Some(oid(0x0a))),
            ("refs/tags/v1".to_owned(), None),
            ("refs/heads/nb".to_owned(), None),
        ]),
        markers: vec![],
        incoming: None,
        edit_stop: None,
    }
}

fn lines(op: &Operation, facts: &RepoFacts) -> Vec<String> {
    op.plan(facts)
        .iter()
        .map(super::super::Invocation::command_line)
        .collect()
}

fn merge() -> Operation {
    Operation::Merge(MergeParams {
        source: Source::Branch(FEAT.to_owned()),
        into: tip(MAIN, 0x0a),
        source_tip: oid(0x0b),
        ff: FastForward::Default,
    })
}

fn rebase() -> Operation {
    Operation::Rebase(RebaseParams::new(
        tip(MAIN, 0x0a),
        Source::Branch(FEAT.to_owned()),
        oid(0x0b),
    ))
}

fn pick() -> Operation {
    Operation::CherryPick(PickParams {
        commits: vec![oid(0x0c)],
        subject: "fix the thing".to_owned(),
        head: tip(MAIN, 0x0a),
    })
}

fn revert() -> Operation {
    Operation::Revert(RevertParams {
        commit: oid(0x0c),
        subject: "fix the thing".to_owned(),
        mainline: None,
        parents: 1,
        parent: Some(oid(0x0b)),
        head: tip(MAIN, 0x0a),
    })
}

fn reset(mode: ResetMode) -> Operation {
    Operation::Reset(ResetParams {
        target: ResetTarget::Head {
            branch: tip(MAIN, 0x0a),
            mode,
        },
        to: oid(0x0d),
    })
}

fn move_branch() -> Operation {
    Operation::Reset(ResetParams {
        target: ResetTarget::Branch(tip(FEAT, 0x0b)),
        to: oid(0x0d),
    })
}

/// A previewed checkout of `feat`: edits are set aside (ADR-0045 §4).
fn checkout() -> Operation {
    checkout_with(OnEdits::SetAside)
}

fn checkout_with(on_edits: OnEdits) -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(FEAT.to_owned()),
        head_before: HeadRef::Branch(MAIN.to_owned()),
        on_edits,
    })
}

/// `nb` at `HEAD`'s own commit (`0x0a`), launched as `b ⇥` on a clean tree
/// launches it: `OnEdits::Refuse`.
fn branch_create(checkout: bool) -> Operation {
    branch_create_at(checkout, 0x0a, OnEdits::Refuse)
}

fn branch_create_at(checkout: bool, at: u8, on_edits: OnEdits) -> Operation {
    Operation::BranchCreate(BranchCreateParams {
        name: "nb".to_owned(),
        at: oid(at),
        checkout,
        on_edits,
    })
}

/// `origin/feat` at `0x0f` as the list saw it, switched to as a new `rf`.
fn track() -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Track {
            remote: tip(ORIGIN_FEAT, 0x0f),
            local: "rf".to_owned(),
        },
        head_before: HeadRef::Branch(MAIN.to_owned()),
        on_edits: OnEdits::Refuse,
    })
}

const ORIGIN_FEAT: &str = "refs/remotes/origin/feat";

/// `facts()` with `origin/feat` at `0x0f` and no `rf` yet: what `track()`
/// reads.
fn track_facts() -> RepoFacts {
    let mut f = facts();
    f.refs.insert(ORIGIN_FEAT.to_owned(), Some(oid(0x0f)));
    f.refs.insert("refs/heads/rf".to_owned(), None);
    f
}

fn branch_delete() -> Operation {
    Operation::BranchDelete(RefDeleteParams {
        name: FEAT.to_owned(),
        expected: oid(0x0b),
    })
}

fn tag_create(message: Option<&str>) -> Operation {
    Operation::TagCreate(TagCreateParams {
        name: "v1".to_owned(),
        at: oid(0x0a),
        message: message.map(str::to_owned),
    })
}

fn branch_from_stash() -> Operation {
    Operation::BranchFromStash(BranchFromStashParams {
        name: "nb".to_owned(),
        index: 0,
        commit: oid(0x50),
    })
}

fn baseline_ops() -> Vec<Operation> {
    vec![
        merge(),
        rebase(),
        pick(),
        revert(),
        reset(ResetMode::Mixed),
        move_branch(),
        checkout(),
        branch_create(true),
        branch_delete(),
        tag_create(None),
        tag_create(Some("release")),
        Operation::TagDelete(RefDeleteParams {
            name: MAIN.to_owned(),
            expected: oid(0x0a),
        }),
        branch_from_stash(),
    ]
}

// ---- describe -------------------------------------------------------------

#[test]
fn describe_reads_like_the_browser_line() {
    let cases = [
        (merge(), "merge feat into main"),
        (rebase(), "rebase main onto feat"),
        (pick(), "cherry-pick 0c0c0c0 \"fix the thing\""),
        (revert(), "revert 0c0c0c0 \"fix the thing\""),
        (reset(ResetMode::Hard), "reset main to 0d0d0d0 (hard)"),
        (move_branch(), "move feat to 0d0d0d0"),
        (checkout(), "checkout feat"),
        (branch_create(false), "branch nb at 0a0a0a0"),
        (branch_delete(), "delete branch feat"),
        (tag_create(None), "tag v1 at 0a0a0a0"),
        (branch_from_stash(), "branch nb from stash@{0}"),
    ];
    for (op, expected) in cases {
        assert_eq!(op.describe(), expected);
    }
    let detached = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Detached(oid(0x0c)),
        head_before: HeadRef::Branch(MAIN.to_owned()),
        on_edits: OnEdits::SetAside,
    });
    assert_eq!(detached.describe(), "checkout 0c0c0c0 (detached)");
    let many = Operation::CherryPick(PickParams {
        commits: vec![oid(1), oid(2)],
        subject: "x".to_owned(),
        head: tip(MAIN, 0x0a),
    });
    assert_eq!(many.describe(), "cherry-pick 2 commits");
}

#[test]
fn kind_matches_the_variant() {
    let kinds: Vec<OperationKind> = baseline_ops().iter().map(Operation::kind).collect();
    assert_eq!(
        kinds,
        [
            OperationKind::Merge,
            OperationKind::Rebase,
            OperationKind::CherryPick,
            OperationKind::Revert,
            OperationKind::Reset,
            OperationKind::Reset,
            OperationKind::Checkout,
            OperationKind::BranchCreate,
            OperationKind::BranchDelete,
            OperationKind::TagCreate,
            OperationKind::TagCreate,
            OperationKind::TagDelete,
            OperationKind::BranchFromStash,
        ]
    );
    for op in baseline_ops() {
        assert!(op.is_history(), "{}", op.describe());
        assert!(!op.is_remote(), "{}", op.describe());
    }
    assert_eq!(merge().starts_in_progress(), Some(InProgress::Merge));
    assert_eq!(rebase().starts_in_progress(), Some(InProgress::Rebase));
    assert_eq!(pick().starts_in_progress(), Some(InProgress::CherryPick));
    assert_eq!(revert().starts_in_progress(), Some(InProgress::Revert));
    assert_eq!(checkout().starts_in_progress(), None);
}

// ---- plan -----------------------------------------------------------------

#[test]
fn plan_merge_supplies_gits_own_subject_and_never_an_editor() {
    let f = facts();
    let plan = merge().plan(&f);
    assert_eq!(plan.len(), 1);
    assert_eq!(
        plan[0].args,
        vec![
            GitArg::Flag("merge"),
            GitArg::Flag("--no-edit"),
            // Spelled out on every arm, so `merge.ff` in the reader's config
            // cannot turn the previewed fast-forward into a merge commit.
            GitArg::Flag("--ff"),
            GitArg::Flag("-m"),
            GitArg::Word("Merge branch 'feat' into main".to_owned()),
            GitArg::Ref(FEAT.as_bytes().to_vec()),
        ]
    );
    let by_commit = Operation::Merge(MergeParams {
        source: Source::Commit(oid(0x0b)),
        into: tip(MAIN, 0x0a),
        source_tip: oid(0x0b),
        ff: FastForward::Only,
    });
    assert_eq!(
        lines(&by_commit, &f),
        [format!(
            "git merge --no-edit --ff-only -m 'Merge commit '\\''{}'\\'' into main' {}",
            hex(0x0b),
            hex(0x0b)
        )]
    );
    let no_ff = Operation::Merge(MergeParams {
        ff: FastForward::Never,
        ..match merge() {
            Operation::Merge(params) => params,
            _ => unreachable!(),
        }
    });
    assert!(lines(&no_ff, &f)[0].contains(" --no-ff "));
}

/// 02b's `<top>` rule (05b): a line of 2+ branches starts git on its top,
/// by short name; a one-branch line is today's string, byte for byte.
#[test]
fn a_line_rebase_names_its_top() {
    let f = facts();
    let one = format!("git rebase -i --no-update-refs --no-autostash --empty=drop {FEAT}");
    assert_eq!(lines(&rebase(), &f), [one]);

    let mut line = rebase();
    if let Operation::Rebase(params) = &mut line {
        params.line.branches.push(tip("refs/heads/feature-b", 0x0c));
        params.line.branches.push(tip("refs/heads/feature-c", 0x0d));
    }
    assert_eq!(
        lines(&line, &f),
        [format!(
            "git rebase -i --no-update-refs --no-autostash --empty=drop {FEAT} feature-c"
        )]
    );
}

#[test]
fn plan_replays_pin_by_id_and_never_open_an_editor() {
    let f = facts();
    // `--no-update-refs` and `--no-autostash`: a rebase must move only the
    // branches the entry records (its line lands as `update-ref` rows Graft
    // writes, ADR-0047), and must never stash outside one. `-i` runs on
    // Graft's own todo — the editors are the engine's helpers, never the
    // user's; `--empty=drop` keeps `-i` dropping an already-upstream commit
    // as the plain rebase did.
    assert_eq!(
        lines(&rebase(), &f),
        [format!(
            "git rebase -i --no-update-refs --no-autostash --empty=drop {FEAT}"
        )]
    );
    assert_eq!(
        lines(&pick(), &f),
        [format!("git cherry-pick --no-edit {}", hex(0x0c))]
    );
    assert_eq!(
        lines(&revert(), &f),
        [format!("git revert --no-edit {}", hex(0x0c))]
    );
    let of_merge = Operation::Revert(RevertParams {
        mainline: Some(1),
        parents: 2,
        ..match revert() {
            Operation::Revert(params) => params,
            _ => unreachable!(),
        }
    });
    assert_eq!(
        lines(&of_merge, &f),
        [format!("git revert --no-edit -m 1 {}", hex(0x0c))]
    );
}

#[test]
fn plan_reset_moves_head_by_mode_and_other_branches_by_compare_and_swap() {
    let f = facts();
    for (mode, flag) in [
        (ResetMode::Soft, "--soft"),
        (ResetMode::Mixed, "--mixed"),
        (ResetMode::Hard, "--hard"),
    ] {
        assert_eq!(
            lines(&reset(mode), &f),
            [format!("git reset {flag} {}", hex(0x0d))]
        );
    }
    assert_eq!(
        lines(&move_branch(), &f),
        [format!("git update-ref {FEAT} {} {}", hex(0x0d), hex(0x0b))]
    );
}

#[test]
fn plan_switch_branch_tag_and_stash_branch() {
    let f = facts();
    assert_eq!(lines(&checkout(), &f), ["git switch feat"]);
    let detached = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Detached(oid(0x0c)),
        head_before: HeadRef::Branch(MAIN.to_owned()),
        on_edits: OnEdits::SetAside,
    });
    assert_eq!(
        lines(&detached, &f),
        [format!("git switch --detach {}", hex(0x0c))]
    );
    assert_eq!(
        lines(&branch_create(false), &f),
        [format!("git branch nb {}", hex(0x0a))]
    );
    assert_eq!(
        lines(&branch_create(true), &f),
        [
            format!("git branch nb {}", hex(0x0a)),
            "git switch nb".to_owned()
        ]
    );
    // `branch -D`, so the branch's config goes with it (review m7-09..21c
    // Minor 13).
    assert_eq!(
        lines(&branch_delete(), &f),
        ["git branch -D feat".to_owned()]
    );
    assert_eq!(
        lines(&tag_create(None), &f),
        [format!("git tag v1 {}", hex(0x0a))]
    );
    let annotated = tag_create(Some("release\r\nnotes\n\n")).plan(&f);
    assert_eq!(
        annotated[0].command_line(),
        format!("git tag -a -F - v1 {}", hex(0x0a))
    );
    assert_eq!(
        annotated[0].stdin.as_deref(),
        Some(&b"release\nnotes\n"[..])
    );
    assert_eq!(
        lines(&branch_from_stash(), &f),
        ["git stash branch nb 'stash@{0}'"]
    );
}

// ---- touches and scope ----------------------------------------------------

#[test]
fn touches_name_the_branch_head_is_on_and_the_trees_that_move() {
    let f = facts();
    let both = |refs: &[&str]| Touches {
        refs: refs.iter().map(|r| (*r).to_owned()).collect(),
        index: true,
        worktree: true,
    };
    let refs_only = |refs: &[&str]| Touches {
        refs: refs.iter().map(|r| (*r).to_owned()).collect(),
        index: false,
        worktree: false,
    };
    assert_eq!(merge().touches(&f), both(&[MAIN]));
    assert_eq!(rebase().touches(&f), both(&[MAIN]));
    assert_eq!(pick().touches(&f), both(&[MAIN]));
    assert_eq!(revert().touches(&f), both(&[MAIN]));
    assert_eq!(reset(ResetMode::Soft).touches(&f), refs_only(&[MAIN]));
    assert_eq!(
        reset(ResetMode::Mixed).touches(&f),
        Touches {
            refs: vec![MAIN.to_owned()],
            index: true,
            worktree: false,
        }
    );
    assert_eq!(reset(ResetMode::Hard).touches(&f), both(&[MAIN]));
    assert_eq!(move_branch().touches(&f), refs_only(&[FEAT]));
    // `HEAD` itself is never a touched ref: `SetHead` puts it back.
    assert_eq!(checkout().touches(&f), both(&[]));
    assert_eq!(
        branch_create(false).touches(&f),
        refs_only(&["refs/heads/nb"])
    );
    assert_eq!(branch_create(true).touches(&f), both(&["refs/heads/nb"]));
    assert_eq!(branch_delete().touches(&f), refs_only(&[FEAT]));
    assert_eq!(tag_create(None).touches(&f), refs_only(&["refs/tags/v1"]));
    assert_eq!(
        branch_from_stash().touches(&f),
        both(&["refs/heads/nb", "refs/stash"])
    );
}

/// The scope is what the operation's facts have to answer: what `check`
/// reads, and — since m5/07 — what `preview::plan` needs to draw the box.
/// `HEAD` on a branch-from-stash is the preview's: it reaches the stash
/// commit by asking for what is on it and not on `HEAD`.
#[test]
fn facts_scope_names_the_refs_check_and_the_preview_read() {
    let scope = |refs: &[&str]| FactsScope {
        remotes: false,
        refs: refs.iter().map(|r| (*r).to_owned()).collect(),
    };
    assert_eq!(merge().facts_scope(), scope(&[MAIN, FEAT]));
    assert_eq!(rebase().facts_scope(), scope(&[MAIN, FEAT]));
    assert_eq!(pick().facts_scope(), scope(&[MAIN]));
    assert_eq!(move_branch().facts_scope(), scope(&[FEAT]));
    assert_eq!(checkout().facts_scope(), scope(&["HEAD", FEAT]));
    assert_eq!(
        branch_create(true).facts_scope(),
        scope(&["refs/heads/nb", "HEAD"])
    );
    assert_eq!(branch_delete().facts_scope(), scope(&[FEAT]));
    assert_eq!(tag_create(None).facts_scope(), scope(&["refs/tags/v1"]));
    assert_eq!(
        branch_from_stash().facts_scope(),
        scope(&["refs/heads/nb", "HEAD"])
    );
    assert_eq!(
        Operation::Stage(Target::Paths(vec![PathBuf::from("a")])).facts_scope(),
        FactsScope::default()
    );
}

// ---- check ----------------------------------------------------------------

#[test]
fn every_kind_passes_the_baseline() {
    let f = facts();
    for op in baseline_ops() {
        assert_eq!(op.check(&f), Ok(()), "{}", op.describe());
    }
}

#[test]
fn a_moved_tip_refuses_with_its_name() {
    let mut f = facts();
    f.refs.insert(FEAT.to_owned(), Some(oid(0x0e)));
    let moved = Err(Refusal::Moved {
        name: FEAT.to_owned(),
    });
    assert_eq!(merge().check(&f), moved);
    assert_eq!(rebase().check(&f), moved);
    assert_eq!(move_branch().check(&f), moved);
    assert_eq!(branch_delete().check(&f), moved);
    // A source given as a bare commit is not pinned to any ref.
    let by_commit = Operation::Merge(MergeParams {
        source: Source::Commit(oid(0x0e)),
        into: tip(MAIN, 0x0a),
        source_tip: oid(0x0e),
        ff: FastForward::Default,
    });
    assert_eq!(by_commit.check(&f), Ok(()));
    let mut main_moved = facts();
    main_moved.refs.insert(MAIN.to_owned(), Some(oid(0x0e)));
    let moved_main = Err(Refusal::Moved {
        name: MAIN.to_owned(),
    });
    for op in [merge(), rebase(), pick(), revert(), reset(ResetMode::Mixed)] {
        assert_eq!(op.check(&main_moved), moved_main, "{}", op.describe());
    }
}

#[test]
fn head_on_another_branch_refuses_branch_changed() {
    let mut f = facts();
    f.head = HeadRef::Branch(FEAT.to_owned());
    for op in [
        merge(),
        rebase(),
        pick(),
        revert(),
        reset(ResetMode::Mixed),
        checkout(),
    ] {
        assert_eq!(
            op.check(&f),
            Err(Refusal::BranchChanged),
            "{}",
            op.describe()
        );
    }
    // Detached `HEAD` is a tip named `HEAD`.
    let mut detached = facts();
    detached.head = HeadRef::Detached(oid(0x0a));
    let pick_detached = Operation::CherryPick(PickParams {
        commits: vec![oid(0x0c)],
        subject: "x".to_owned(),
        head: tip("HEAD", 0x0a),
    });
    assert_eq!(pick_detached.check(&detached), Ok(()));
    detached.head = HeadRef::Detached(oid(0x0e));
    assert_eq!(
        pick_detached.check(&detached),
        Err(Refusal::Moved {
            name: "HEAD".to_owned()
        })
    );
}

/// Flipped by m6-conflicts/73 (ADR-0045 §4): a rebase or checkout over
/// changed tracked files sets them aside instead of refusing. A branch from
/// a stash still refuses. Flipped again by m7-power-tools/10b: a new branch
/// switched to at `HEAD`'s own commit leaves the edits as they are, so it
/// no longer refuses (`switching_create_elsewhere_refuse_refuses_dirty` has
/// the case that still does).
#[test]
fn a_dirty_tracked_tree_refuses_a_stash_branch_but_not_rebase_checkout_or_create_at_head() {
    let mut f = facts();
    f.status.entries.push(StatusEntry {
        unstaged: Some(Change::Modified),
        ..entry("a.rs")
    });
    f.status.entries.push(StatusEntry {
        staged: Some(Change::Added),
        ..entry("b.rs")
    });
    f.status.entries.push(StatusEntry {
        untracked: true,
        ..entry("junk")
    });
    let dirty = Err(Refusal::DirtyTree { count: 2 });
    assert_eq!(branch_from_stash().check(&f), dirty);
    for op in [
        branch_create(true),
        rebase(),
        checkout(),
        merge(),
        pick(),
        revert(),
        reset(ResetMode::Hard),
        move_branch(),
        branch_create(false),
        branch_delete(),
        tag_create(None),
    ] {
        assert_eq!(op.check(&f), Ok(()), "{}", op.describe());
    }
}

/// `facts()` with the working tree's status set to `entries`.
fn facts_over(entries: Vec<StatusEntry>) -> RepoFacts {
    let mut f = facts();
    f.status.entries = entries;
    f
}

/// ADR-0045 §4: `sets_aside` is `Some` for a rebase, a checkout or (m7/11a,
/// "Where I'd differ" 4) a cherry-pick exactly when tracked files are changed, counting them; `staged` is 0 since every
/// put-back keeps the index (amendment 2026-09-26 b, m7-power-tools/30).
/// Untracked files never count; every other kind is `None`, dirty or not.
#[test]
fn sets_aside_table() {
    let clean = facts_over(vec![]);
    let tracked_only = facts_over(vec![StatusEntry {
        unstaged: Some(Change::Modified),
        ..entry("a.rs")
    }]);
    let staged_and_unstaged = staged_and_unstaged();
    let untracked_only = facts_over(vec![StatusEntry {
        untracked: true,
        ..entry("junk")
    }]);

    for op in [rebase(), checkout(), pick()] {
        let what = op.describe();
        assert_eq!(op.sets_aside(&clean), None, "{what}: clean");
        assert_eq!(
            op.sets_aside(&tracked_only),
            Some(SetAside { files: 1 }),
            "{what}: tracked only"
        );
        assert_eq!(
            op.sets_aside(&staged_and_unstaged),
            // Every put-back keeps the index (ADR-0045 amendment
            // 2026-09-26 b): no staging is forecast lost, so only the file
            // count is set aside (30c removed the always-0 `staged`).
            Some(SetAside { files: 5 }),
            "{what}: staged and unstaged"
        );
        assert_eq!(
            op.sets_aside(&untracked_only),
            None,
            "{what}: untracked only"
        );
    }
    for op in [merge(), revert(), reset(ResetMode::Hard)] {
        for f in [&clean, &tracked_only, &staged_and_unstaged, &untracked_only] {
            assert_eq!(op.sets_aside(f), None, "{}", op.describe());
        }
    }
}

/// Every staged shape `sets_aside_table` counts, beside an unstaged edit and
/// an untracked file.
fn staged_and_unstaged() -> RepoFacts {
    facts_over(vec![
        StatusEntry {
            unstaged: Some(Change::Modified),
            ..entry("a.rs")
        },
        StatusEntry {
            staged: Some(Change::Added),
            ..entry("b.rs")
        },
        StatusEntry {
            staged: Some(Change::Modified),
            unstaged: Some(Change::Modified),
            ..entry("c.rs")
        },
        StatusEntry {
            staged: Some(Change::Deleted),
            ..entry("d.rs")
        },
        StatusEntry {
            original_path: Some(PathBuf::from("old.rs")),
            staged: Some(Change::Renamed),
            ..entry("e.rs")
        },
        StatusEntry {
            untracked: true,
            ..entry("junk")
        },
    ])
}

/// A set-aside rebase or checkout records `refs/stash` in `touches`, so the
/// entry's before and after say whether the stash list moved (ADR-0045 §4
/// step 2, §6). A clean one does not.
#[test]
fn a_set_aside_touches_the_stash_ref() {
    let mut dirty = facts();
    dirty.status.entries.push(StatusEntry {
        unstaged: Some(Change::Modified),
        ..entry("a.rs")
    });
    for op in [rebase(), checkout()] {
        assert!(
            op.touches(&dirty).refs.contains(&"refs/stash".to_owned()),
            "{}",
            op.describe()
        );
        assert!(
            !op.touches(&facts()).refs.contains(&"refs/stash".to_owned()),
            "{}",
            op.describe()
        );
    }
}

#[test]
fn new_names_must_be_valid_and_free() {
    let f = facts();
    let bad = Operation::BranchCreate(BranchCreateParams {
        name: "a..b".to_owned(),
        at: oid(0x0a),
        checkout: false,
        on_edits: OnEdits::Refuse,
    });
    assert_eq!(
        bad.check(&f),
        Err(Refusal::BadRefName {
            name: "a..b".to_owned(),
            rule: RefNameRule::ForbiddenSequence,
        })
    );
    let taken = Operation::BranchCreate(BranchCreateParams {
        name: "feat".to_owned(),
        at: oid(0x0a),
        checkout: false,
        on_edits: OnEdits::Refuse,
    });
    let mut with_feat = facts();
    with_feat
        .refs
        .insert("refs/heads/feat".to_owned(), Some(oid(0x0b)));
    assert_eq!(
        taken.check(&with_feat),
        Err(Refusal::RefExists {
            name: "feat".to_owned()
        })
    );
    let mut tag_taken = facts();
    tag_taken
        .refs
        .insert("refs/tags/v1".to_owned(), Some(oid(1)));
    assert_eq!(
        tag_create(None).check(&tag_taken),
        Err(Refusal::RefExists {
            name: "v1".to_owned()
        })
    );
    assert_eq!(
        tag_create(Some("\n\n")).check(&f),
        Err(Refusal::EmptySubject)
    );
    let mut stash_gone = facts();
    stash_gone.stashes.clear();
    assert_eq!(
        branch_from_stash().check(&stash_gone),
        Err(Refusal::NoSuchStash)
    );
}

#[test]
fn deleting_needs_the_ref_present_and_not_checked_out() {
    let mut f = facts();
    f.refs.insert(FEAT.to_owned(), None);
    assert_eq!(
        branch_delete().check(&f),
        Err(Refusal::NoSuchRef {
            name: FEAT.to_owned()
        })
    );
    let current = Operation::BranchDelete(RefDeleteParams {
        name: MAIN.to_owned(),
        expected: oid(0x0a),
    });
    assert_eq!(
        current.check(&facts()),
        Err(Refusal::CheckedOut {
            branch: "main".to_owned()
        })
    );
    let mut gone = facts();
    gone.refs.insert(FEAT.to_owned(), None);
    assert_eq!(
        checkout().check(&gone),
        Err(Refusal::NoSuchRef {
            name: FEAT.to_owned()
        })
    );
}

#[test]
fn revert_mainline_must_match_the_commit() {
    let f = facts();
    let with = |mainline: Option<u8>, parents: usize| {
        Operation::Revert(RevertParams {
            commit: oid(0x0c),
            subject: "x".to_owned(),
            mainline,
            parents,
            parent: Some(oid(0x0b)),
            head: tip(MAIN, 0x0a),
        })
    };
    assert_eq!(with(None, 1).check(&f), Ok(()));
    assert_eq!(with(Some(1), 2).check(&f), Ok(()));
    assert_eq!(with(Some(2), 2).check(&f), Ok(()));
    assert_eq!(with(Some(1), 1).check(&f), Err(Refusal::MainlineOnNonMerge));
    assert_eq!(with(Some(3), 2).check(&f), Err(Refusal::MainlineOnNonMerge));
    assert_eq!(with(Some(0), 2).check(&f), Err(Refusal::MainlineOnNonMerge));
    assert_eq!(with(None, 2).check(&f), Err(Refusal::MergeNeedsMainline));
    let empty = Operation::CherryPick(PickParams {
        commits: vec![],
        subject: String::new(),
        head: tip(MAIN, 0x0a),
    });
    assert_eq!(empty.check(&f), Err(Refusal::NoCommits));
}

#[test]
fn in_progress_and_unmerged_refuse_history_operations() {
    let mut f = facts();
    f.in_progress = Some(InProgress::Rebase);
    for op in baseline_ops() {
        assert_eq!(
            op.check(&f),
            Err(Refusal::InProgress(InProgress::Rebase)),
            "{}",
            op.describe()
        );
    }
    let mut unmerged = facts();
    unmerged.status.entries.push(StatusEntry {
        unmerged: true,
        ..entry("c.rs")
    });
    for op in baseline_ops() {
        assert_eq!(
            op.check(&unmerged),
            Err(Refusal::Unmerged {
                paths: vec![PathBuf::from("c.rs")]
            }),
            "{}",
            op.describe()
        );
    }
}

#[test]
fn staging_an_unmerged_path_is_mark_resolved_unless_markers_remain() {
    let mut f = facts();
    f.in_progress = Some(InProgress::Merge);
    f.status.entries.push(StatusEntry {
        unmerged: true,
        ..entry("c.rs")
    });
    f.status.entries.push(StatusEntry {
        unstaged: Some(Change::Modified),
        ..entry("d.rs")
    });
    let stage =
        |list: &[&str]| Operation::Stage(Target::Paths(list.iter().map(PathBuf::from).collect()));
    assert_eq!(stage(&["c.rs"]).check(&f), Ok(()));
    // A stage that mixes in a path that is not unmerged is not "mark
    // resolved": the general refusal stands.
    assert_eq!(
        stage(&["c.rs", "d.rs"]).check(&f),
        Err(Refusal::Unmerged {
            paths: vec![PathBuf::from("c.rs")]
        })
    );
    let mut marked = f.clone();
    marked.markers.push(PathBuf::from("c.rs"));
    assert_eq!(
        stage(&["c.rs"]).check(&marked),
        Err(Refusal::MarkersRemain {
            path: PathBuf::from("c.rs")
        })
    );
    assert_eq!(
        lines(&stage(&["c.rs"]), &f),
        ["git --literal-pathspecs add -- c.rs"]
    );
}

#[test]
fn every_history_refusal_explains_itself_in_one_line() {
    let all = [
        Refusal::Moved {
            name: FEAT.to_owned(),
        },
        Refusal::DirtyTree { count: 1 },
        Refusal::DirtyTree { count: 3 },
        Refusal::BadRefName {
            name: "a..b".to_owned(),
            rule: RefNameRule::ForbiddenSequence,
        },
        Refusal::RefExists {
            name: "v1".to_owned(),
        },
        Refusal::MarkersRemain {
            path: PathBuf::from("c.rs"),
        },
        Refusal::NotInProgress,
        Refusal::NotOnBranch {
            commit: oid(0x0c),
            branch: "main".to_owned(),
        },
        Refusal::MainlineOnNonMerge,
        Refusal::MergeNeedsMainline,
        Refusal::NoCommits,
        Refusal::NoSuchRef {
            name: FEAT.to_owned(),
        },
        Refusal::CheckedOut {
            branch: "main".to_owned(),
        },
        Refusal::CannotSkip(InProgress::Merge),
        Refusal::NotFastForward {
            branch: "main".to_owned(),
        },
    ];
    for refusal in all {
        let text = refusal.explain();
        assert!(!text.is_empty() && !text.contains('\n'), "{text}");
    }
    assert_eq!(
        Refusal::DirtyTree { count: 1 }.explain(),
        "needs a clean tree · S stashes 1 changed file first"
    );
    assert_eq!(
        Refusal::Moved {
            name: FEAT.to_owned()
        }
        .explain(),
        "feat moved since the preview was drawn — press the key again"
    );
}

// ── the switch rule (m7-power-tools/10b) ────────────────────────────────

/// One unstaged edit to a tracked file, and an untracked file.
fn one_edit(mut f: RepoFacts) -> RepoFacts {
    f.status.entries = vec![
        StatusEntry {
            unstaged: Some(Change::Modified),
            ..entry("a.rs")
        },
        StatusEntry {
            untracked: true,
            ..entry("junk")
        },
    ];
    f
}

/// The instant path's stale "clean": a checkout launched with `Refuse`
/// over a tree that has an edit is refused, never set aside unseen.
#[test]
fn checkout_refuse_refuses_a_dirty_tree() {
    let dirty = one_edit(facts());
    let instant = checkout_with(OnEdits::Refuse);
    assert_eq!(instant.check(&dirty), Err(Refusal::DirtyTree { count: 1 }));
    assert_eq!(instant.sets_aside(&dirty), None);
    // Clean, or untracked only: it runs.
    assert_eq!(instant.check(&facts()), Ok(()));
    let untracked = facts_over(vec![StatusEntry {
        untracked: true,
        ..entry("junk")
    }]);
    assert_eq!(instant.check(&untracked), Ok(()));
    // The previewed one sets the edit aside, as before.
    assert_eq!(checkout().check(&dirty), Ok(()));
    assert_eq!(checkout().sets_aside(&dirty), Some(SetAside { files: 1 }));
}

/// `b ⇥` at `HEAD`'s commit over edits: nothing is set aside either way,
/// and `Refuse` does not refuse (`switch -c` leaves index and files).
#[test]
fn switching_create_at_head_sets_nothing_aside_and_never_refuses() {
    let dirty = one_edit(facts());
    for on_edits in [OnEdits::Refuse, OnEdits::SetAside] {
        let op = branch_create_at(true, 0x0a, on_edits);
        assert_eq!(op.check(&dirty), Ok(()), "{on_edits:?}");
        assert_eq!(op.sets_aside(&dirty), None, "{on_edits:?}");
        assert!(
            !op.touches(&dirty).refs.contains(&"refs/stash".to_owned()),
            "{on_edits:?}"
        );
    }
}

/// `b ⇥` at another commit: `Refuse` refuses the edits; `SetAside` never
/// refuses them and sets them aside, `refs/stash` touched.
#[test]
fn switching_create_elsewhere_refuse_refuses_dirty() {
    let dirty = one_edit(facts());
    let refuse = branch_create_at(true, 0x0b, OnEdits::Refuse);
    assert_eq!(refuse.check(&dirty), Err(Refusal::DirtyTree { count: 1 }));
    assert_eq!(refuse.check(&facts()), Ok(()));
    let aside = branch_create_at(true, 0x0b, OnEdits::SetAside);
    assert_eq!(aside.check(&dirty), Ok(()));
    assert_eq!(aside.sets_aside(&dirty), Some(SetAside { files: 1 }));
    assert_eq!(aside.sets_aside(&facts()), None, "clean");
    assert_eq!(
        aside.touches(&dirty).refs,
        ["refs/heads/nb".to_owned(), "refs/stash".to_owned()]
    );
    // A create that does not switch never sets anything aside.
    let quiet = branch_create_at(false, 0x0b, OnEdits::SetAside);
    assert_eq!(quiet.sets_aside(&dirty), None);
}

#[test]
fn track_plans_an_explicit_tracking_create_and_names_its_branch() {
    let f = track_facts();
    assert_eq!(track().check(&f), Ok(()));
    assert_eq!(
        track().facts_scope(),
        FactsScope {
            remotes: false,
            refs: ["HEAD", ORIGIN_FEAT, "refs/heads/rf"]
                .iter()
                .map(|r| (*r).to_owned())
                .collect(),
        }
    );
    assert_eq!(
        lines(&track(), &f),
        ["git switch --create rf --track refs/remotes/origin/feat"]
    );
    assert_eq!(
        track().touches(&f),
        Touches {
            refs: vec!["refs/heads/rf".to_owned()],
            index: true,
            worktree: true,
        }
    );
    assert_eq!(track().describe(), "checkout rf (new, tracks origin/feat)");
    assert!(track().moves_head());
}

#[test]
fn track_refuses_when_the_local_name_exists() {
    let mut f = track_facts();
    f.refs.insert("refs/heads/rf".to_owned(), Some(oid(0x0f)));
    assert_eq!(
        track().check(&f),
        Err(Refusal::RefExists {
            name: "rf".to_owned()
        })
    );
}

#[test]
fn track_refuses_when_the_remote_moved() {
    let mut f = track_facts();
    f.refs.insert(ORIGIN_FEAT.to_owned(), Some(oid(0x0e)));
    assert_eq!(
        track().check(&f),
        Err(Refusal::Moved {
            name: ORIGIN_FEAT.to_owned()
        })
    );
    f.refs.insert(ORIGIN_FEAT.to_owned(), None);
    assert_eq!(
        track().check(&f),
        Err(Refusal::NoSuchRef {
            name: ORIGIN_FEAT.to_owned()
        })
    );
}

/// A tracking checkout follows the same edits rule as any other.
#[test]
fn track_refuse_refuses_a_dirty_tree_and_set_aside_touches_the_stash() {
    let dirty = one_edit(track_facts());
    assert_eq!(track().check(&dirty), Err(Refusal::DirtyTree { count: 1 }));
    let Operation::Checkout(mut params) = track() else {
        unreachable!("track() is a checkout")
    };
    params.on_edits = OnEdits::SetAside;
    let aside = Operation::Checkout(params);
    assert_eq!(aside.check(&dirty), Ok(()));
    assert_eq!(
        aside.touches(&dirty).refs,
        ["refs/heads/rf".to_owned(), "refs/stash".to_owned()]
    );
}

// ── what a pick cannot take (m7-power-tools/11a) ─────────────────────────

/// The pick of `0x0c` onto `main` at `0x0a`, as `pick()` builds it.
fn pick_params() -> PickParams {
    let Operation::CherryPick(params) = pick() else {
        unreachable!("pick() is a cherry-pick")
    };
    params
}

/// The first seven hex digits, as the refusal words name a commit.
fn sha7(byte: u8) -> String {
    hex(byte)[..7].to_owned()
}

/// `HEAD`'s own commit is on `HEAD`: refused in words before git runs
/// (`KNOWN_GAPS` :806).
#[test]
fn picking_head_is_refused_already_on_branch() {
    let refused = pick_params().check_commit(oid(0x0a), true, 1);
    assert_eq!(
        refused,
        Err(Refusal::AlreadyOnBranch {
            commit: oid(0x0a),
            branch: "main".to_owned(),
        })
    );
    assert_eq!(
        refused.map_err(|why| why.explain()),
        Err(format!("{} is already on main", sha7(0x0a)))
    );
}

/// An ancestor of `HEAD` is refused the same way; a detached `HEAD` names
/// itself.
#[test]
fn picking_an_ancestor_is_refused_already_on_branch() {
    assert_eq!(
        pick_params().check_commit(oid(0x01), true, 1),
        Err(Refusal::AlreadyOnBranch {
            commit: oid(0x01),
            branch: "main".to_owned(),
        })
    );
    let detached = PickParams {
        head: tip("HEAD", 0x0a),
        ..pick_params()
    };
    assert_eq!(
        detached
            .check_commit(oid(0x01), true, 1)
            .map_err(|why| why.explain()),
        Err(format!("{} is already on HEAD", sha7(0x01)))
    );
}

/// A merge commit `HEAD` lacks is refused: taking one needs a mainline,
/// which Graft does not offer yet. A merge `HEAD` already has reads
/// "already on" — the truer of the two. One parent, or none (a root), takes.
#[test]
fn picking_a_merge_is_refused() {
    let params = pick_params();
    let refused = params.check_commit(oid(0x0c), false, 2);
    assert_eq!(refused, Err(Refusal::PickMerge { commit: oid(0x0c) }));
    assert_eq!(
        refused.map_err(|why| why.explain()),
        Err(format!(
            "{} is a merge · taking a merge isn't in Graft yet",
            sha7(0x0c)
        ))
    );
    assert!(matches!(
        params.check_commit(oid(0x0c), true, 2),
        Err(Refusal::AlreadyOnBranch { .. })
    ));
    assert_eq!(params.check_commit(oid(0x0c), false, 1), Ok(()));
    assert_eq!(params.check_commit(oid(0x0c), false, 0), Ok(()));
}

/// A pick that sets edits aside records `refs/stash` beside its branch, as
/// a rebase does: undo's stash step reads its before and after (ADR-0045
/// §6). Clean, only the branch.
#[test]
fn a_pick_that_sets_aside_touches_the_stash() {
    assert_eq!(pick().touches(&facts()).refs, [MAIN.to_owned()]);
    let dirty = facts_over(vec![StatusEntry {
        unstaged: Some(Change::Modified),
        ..entry("a.rs")
    }]);
    assert_eq!(pick().check(&dirty), Ok(()), "no clean-tree rule");
    assert_eq!(
        pick().touches(&dirty).refs,
        [MAIN.to_owned(), "refs/stash".to_owned()]
    );
}
