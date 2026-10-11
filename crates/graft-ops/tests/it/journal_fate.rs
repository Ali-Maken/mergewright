//! m8/46 (usability Q1): a commit an undoable step left behind reads
//! "replaced" (or "deleted"), not lost — on real repositories, with Graft's
//! own operations and the system `git`. Every old tip is read with
//! `git rev-parse` before the step that leaves it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use graft_core::oid::Oid;
use graft_ops::journal_list::{Fate, JournalList, ListRow};
use graft_ops::operation::{Operation, RebaseParams, RefDeleteParams, Source, Tip};

use crate::journal_list_oracle::{SHELL, commit_at, git_at, id, now_ms, the_list};
use crate::support::{TempRepo, run, run_undo};

/// `main` with A and M, and each of `branches` off A with one commit of its
/// own; `HEAD` back on `main`. Every step in the shell's past.
fn diverged(branches: &[&str]) -> TempRepo {
    let repo = TempRepo::init();
    commit_at(&repo, SHELL, "a.txt", "A");
    for (offset, name) in branches.iter().enumerate() {
        let at = SHELL + 10 * (i64::try_from(offset).expect("small") + 1);
        git_at(&repo, at, &["switch", "-q", "-c", name, "main"]);
        commit_at(
            &repo,
            at + 1,
            &format!("{name}.txt"),
            &format!("work on {name}"),
        );
    }
    git_at(&repo, SHELL + 100, &["switch", "-q", "main"]);
    commit_at(&repo, SHELL + 101, "m.txt", "M");
    repo
}

/// Graft's rebase of the checked-out branch onto `main`.
fn rebase_onto_main(repo: &TempRepo, at: i64) {
    let branch = repo.git(&["symbolic-ref", "HEAD"]);
    let op = Operation::Rebase(RebaseParams::new(
        Tip {
            oid: id(repo, &branch),
            name: branch,
        },
        Source::Branch("main".to_owned()),
        id(repo, "main"),
    ));
    run(repo, &op, at);
}

/// The fate of every lost row whose tip is `tip`.
fn fates_of(list: &JournalList, tip: Oid) -> Vec<Fate> {
    list.rows
        .iter()
        .filter_map(|row| match row {
            ListRow::Lost { commit, .. } if commit.run.tip == tip => Some(commit.fate),
            _ => None,
        })
        .collect()
}

/// Whether the row `event` points at is a `●` row.
fn under_graft(list: &JournalList, tip: Oid) -> bool {
    list.rows.iter().any(|row| match row {
        ListRow::Lost { event, commit } if commit.run.tip == tip => {
            matches!(list.rows[*event], ListRow::Graft(_))
        }
        _ => false,
    })
}

#[test]
fn an_undoable_rebase_replaces_its_old_tip() {
    let repo = diverged(&["feature"]);
    git_at(&repo, SHELL + 110, &["switch", "-q", "feature"]);
    let old_tip = id(&repo, "feature");
    rebase_onto_main(&repo, now_ms());

    let list = the_list(&repo);

    assert_ne!(id(&repo, "feature"), old_tip, "the rebase moved feature");
    assert_eq!(fates_of(&list, old_tip), [Fate::Replaced], "{list:#?}");
    assert!(under_graft(&list, old_tip), "{list:#?}");
}

/// The first rebase is not the next undo — the second is — but both are in
/// force and `u` reaches each in turn.
#[test]
fn an_older_in_force_rebase_still_replaced() {
    let repo = diverged(&["one", "two"]);
    let at = now_ms();
    git_at(&repo, SHELL + 110, &["switch", "-q", "one"]);
    let one = id(&repo, "one");
    rebase_onto_main(&repo, at);
    // On git's real clock, as the rebases are: a switch dated in the past
    // between two records dated now would put `HEAD`'s log out of time
    // order, which no real session writes.
    repo.git(&["switch", "-q", "two"]);
    let two = id(&repo, "two");
    rebase_onto_main(&repo, at + 1_000);

    let list = the_list(&repo);

    assert_eq!(fates_of(&list, one), [Fate::Replaced], "{list:#?}");
    assert_eq!(fates_of(&list, two), [Fate::Replaced], "{list:#?}");
}

#[test]
fn an_undone_rebase_leaves_no_replaced_row() {
    let repo = diverged(&["feature"]);
    let at = now_ms();
    git_at(&repo, SHELL + 110, &["switch", "-q", "feature"]);
    let old_tip = id(&repo, "feature");
    rebase_onto_main(&repo, at);
    let rebased = id(&repo, "feature");
    run_undo(&repo, at + 1_000, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));

    let list = the_list(&repo);

    assert_eq!(id(&repo, "feature"), old_tip, "the undo put feature back");
    assert_eq!(fates_of(&list, old_tip), [], "{list:#?}");
    // What the undo dropped is redo's business: lost, never "replaced".
    assert_eq!(fates_of(&list, rebased), [Fate::Lost], "{list:#?}");
    assert!(
        list.rows.iter().all(|row| !matches!(
            row,
            ListRow::Lost { commit, .. } if commit.fate != Fate::Lost
        )),
        "{list:#?}"
    );
}

#[test]
fn a_shell_reset_stays_lost() {
    let repo = diverged(&[]);
    let old_tip = id(&repo, "main");
    git_at(&repo, SHELL + 110, &["reset", "-q", "--hard", "HEAD~1"]);

    let list = the_list(&repo);

    assert_eq!(fates_of(&list, old_tip), [Fate::Lost], "{list:#?}");
    assert!(
        list.rows.iter().any(|row| match row {
            ListRow::Lost { event, commit } if commit.run.tip == old_tip =>
                matches!(list.rows[*event], ListRow::Outside(_)),
            _ => false,
        }),
        "under the `$` row: {list:#?}"
    );
}

#[test]
fn a_deleted_branch_reads_deleted() {
    let repo = diverged(&["spike"]);
    let old_tip = id(&repo, "spike");
    let op = Operation::BranchDelete(RefDeleteParams {
        name: "refs/heads/spike".to_owned(),
        expected: old_tip,
    });
    run(&repo, &op, now_ms());

    let list = the_list(&repo);

    assert!(
        repo.try_git(&["rev-parse", "--verify", "refs/heads/spike"])
            .is_err()
    );
    assert_eq!(fates_of(&list, old_tip), [Fate::Deleted], "{list:#?}");
    assert!(under_graft(&list, old_tip), "{list:#?}");
}

// ── review m8 g1, Major 2 and Minor 6 (ticket 46r) ─────────────────────

/// `main` moves on by one commit without touching `HEAD`'s log, as a fetch
/// moves `origin/main`: a commit made with `commit-tree`, set with
/// `update-ref`.
fn main_moves_on(repo: &TempRepo, subject: &str) {
    let tree = repo.git(&["rev-parse", "main^{tree}"]);
    let made = repo.git(&["commit-tree", &tree, "-p", "main", "-m", subject]);
    repo.git(&[
        "update-ref",
        "-m",
        "fetch: fast-forward",
        "refs/heads/main",
        &made,
    ]);
}

/// Whether every lost row is one `u` brings back.
fn no_lost_row(list: &JournalList) -> bool {
    list.rows.iter().all(|row| match row {
        ListRow::Lost { commit, .. } => commit.fate != Fate::Lost,
        _ => true,
    })
}

/// What `y` runs: the preview's plan, its line filled (branches that follow
/// his, so git is started on the top one).
fn rebase_line_onto_main(repo: &TempRepo, at: i64) {
    let branch = repo.git(&["symbolic-ref", "HEAD"]);
    let op = Operation::Rebase(RebaseParams::new(
        Tip {
            oid: id(repo, &branch),
            name: branch,
        },
        Source::Branch("main".to_owned()),
        id(repo, "main"),
    ));
    let handle = repo.open();
    let git = crate::support::git_version();
    let facts =
        graft_ops::facts::read_facts(&handle, git, &crate::support::Never, &op.facts_scope())
            .expect("the facts read");
    let (op, _) =
        graft_ops::preview::plan_as_run(&handle, &git, op, &facts, &crate::support::Never)
            .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    run(repo, &op, at);
}

/// Two rebases with no switch between them write one unbroken run of
/// `rebase` records on `HEAD`: each old tip is still `u`'s to bring back.
#[test]
fn back_to_back_rebases_each_replace_their_old_tip() {
    let repo = diverged(&["feature"]);
    let at = now_ms();
    git_at(&repo, SHELL + 110, &["switch", "-q", "feature"]);
    let first = id(&repo, "feature");
    rebase_onto_main(&repo, at);
    main_moves_on(&repo, "M2");
    let second = id(&repo, "feature");
    rebase_onto_main(&repo, at + 1_000);

    let list = the_list(&repo);

    // The reviewer's layout: six `rebase` records on `HEAD`, nothing between.
    let head_log = repo.git(&["reflog", "show", "--format=%gs", "HEAD"]);
    let newest: Vec<&str> = head_log.lines().take(7).collect();
    assert!(
        newest[..6]
            .iter()
            .all(|subject| subject.starts_with("rebase")),
        "{newest:#?}"
    );
    assert_eq!(fates_of(&list, first), [Fate::Replaced], "{list:#?}");
    assert_eq!(fates_of(&list, second), [Fate::Replaced], "{list:#?}");
    assert!(under_graft(&list, first) && under_graft(&list, second));
    assert!(no_lost_row(&list), "{list:#?}");
}

/// The reviewer's chain: a line rebase (git started on its top branch, so
/// each run returns to `two` while `HEAD` ends on `one`), twice, no switch
/// of his between.
#[test]
fn a_line_rebase_then_another_each_replace_their_old_tips() {
    let repo = diverged(&["one"]);
    git_at(&repo, SHELL + 200, &["switch", "-q", "-c", "two", "one"]);
    commit_at(&repo, SHELL + 201, "two.txt", "work on two");
    git_at(&repo, SHELL + 210, &["switch", "-q", "one"]);
    let at = now_ms();
    let one = id(&repo, "one");
    let two = id(&repo, "two");
    rebase_line_onto_main(&repo, at);
    // Graft switches back after the line: a `checkout:` record ends the
    // run, so the reviewer's unbroken layout needs the case above.
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/one");
    main_moves_on(&repo, "M2");
    let one_again = id(&repo, "one");
    let two_again = id(&repo, "two");
    rebase_line_onto_main(&repo, at + 1_000);

    let list = the_list(&repo);

    // `one`'s old tip is `two`'s parent: one run, by `two`'s tip.
    assert_eq!(id(&repo, &format!("{two}~1")), one);
    assert_eq!(id(&repo, &format!("{two_again}~1")), one_again);
    assert_eq!(fates_of(&list, two), [Fate::Replaced], "{list:#?}");
    assert_eq!(fates_of(&list, two_again), [Fate::Replaced], "{list:#?}");
    assert!(no_lost_row(&list), "{list:#?}");
}

/// Minor 6: `u` then `U` puts the rebase back in force; `u` would undo it
/// again and bring the old tip back, so the tip is not lost.
#[test]
fn a_redone_rebase_replaces_its_old_tip_again() {
    let repo = diverged(&["feature"]);
    let at = now_ms();
    git_at(&repo, SHELL + 110, &["switch", "-q", "feature"]);
    let old_tip = id(&repo, "feature");
    rebase_onto_main(&repo, at);
    run_undo(&repo, at + 1_000, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    crate::support::run_redo(&repo, at + 2_000, false)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)));

    let list = the_list(&repo);

    assert_ne!(
        id(&repo, "feature"),
        old_tip,
        "the redo moved feature again"
    );
    assert_eq!(fates_of(&list, old_tip), [Fate::Replaced], "{list:#?}");
}
