//! One-shot writes: switch, merge, hard reset, put-back, branch-from-stash.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use graft_core::journal::HeadRef;
use graft_ops::operation::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    Operation, RecoverParams, RecoverTarget, ResetMode,
};

use super::{
    MINE, TempRepo, applied_words, assert_refused, attempt, id, main_and_other,
    main_and_other_unignored, merge_other, preview_words, reset_main, sha7, switch_to,
};

// ── one-shot writes ─────────────────────────────────────────────────────

#[test]
fn switch_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    assert_refused(
        &repo,
        &switch_to("other"),
        "c.txt",
        "c.txt (ignored) is in the way of switching to other · move it aside first",
    );
}

#[test]
fn switch_detached_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let other = id(&repo, "other");
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Detached(other),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of switching to {} · move it aside first",
            sha7(other)
        ),
    );
}

#[test]
fn new_branch_switch_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = Operation::BranchCreate(BranchCreateParams {
        name: "fresh".to_owned(),
        at: id(&repo, "other"),
        checkout: true,
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of switching to fresh · move it aside first",
    );
    assert!(
        repo.try_git(&["rev-parse", "--verify", "-q", "refs/heads/fresh"])
            .is_err(),
        "no branch made"
    );
}

/// The stash's base tracks `c.txt` and `HEAD` doesn't: `stash branch`
/// switches to that base first.
#[test]
fn branch_from_stash_over_an_ignored_file_is_refused() {
    let repo = main_and_other_unignored();
    repo.git(&["switch", "-q", "other"]);
    repo.write("a.txt", b"a, edited on other\n");
    repo.git(&["stash", "push", "-q"]);
    let stash = id(&repo, "stash@{0}");
    repo.git(&["switch", "-q", "main"]);
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    let op = Operation::BranchFromStash(BranchFromStashParams {
        name: "from-stash".to_owned(),
        index: 0,
        commit: stash,
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of switching to from-stash · move it aside first",
    );
}

#[test]
fn merge_fast_forward_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = merge_other(&repo, FastForward::Only);
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of merging other · move it aside first",
    );
}

#[test]
fn merge_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = merge_other(&repo, FastForward::Never);
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of merging other · move it aside first",
    );
}

#[test]
fn hard_reset_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = reset_main(&repo, ResetMode::Hard);
    assert_refused(
        &repo,
        &op,
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of resetting main to {} · move it aside first",
            sha7(id(&repo, "other"))
        ),
    );
}

/// No false refusal: a soft or mixed reset writes no file.
#[test]
fn soft_and_mixed_resets_are_not_refused() {
    for mode in [ResetMode::Soft, ResetMode::Mixed] {
        let repo = main_and_other();
        let op = reset_main(&repo, mode);
        let outcome = attempt(&repo, &op, "c.txt");
        assert!(
            outcome.preview.is_ok(),
            "{mode:?}: {}",
            preview_words(&outcome.preview)
        );
        assert!(
            outcome.applied.is_ok(),
            "{mode:?}: {}",
            applied_words(&outcome.applied)
        );
        assert_eq!(outcome.bytes, MINE, "{mode:?}: his bytes");
        assert_eq!(
            id(&repo, "main"),
            id(&repo, "other"),
            "{mode:?}: main moved"
        );
    }
}

/// 23b's C1 case: a checked-out branch put back is a `reset --hard`.
#[test]
fn put_back_over_an_ignored_file_is_refused_in_the_way() {
    let repo = main_and_other();
    let main = id(&repo, "main");
    let other = id(&repo, "other");
    let op = Operation::Recover(RecoverParams {
        target: RecoverTarget::MoveRef {
            name: "main".to_owned(),
            to: other,
            expected: main,
        },
        because: "the shell's reset".to_owned(),
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of putting main back to {} · move it aside first",
            sha7(other)
        ),
    );
}

#[test]
fn untracked_file_in_the_way_is_refused_before_git() {
    let repo = main_and_other_unignored();
    repo.write("c.txt", MINE);
    assert_refused(
        &repo,
        &switch_to("other"),
        "c.txt",
        "c.txt is in the way of switching to other · S stashes it first",
    );
}

#[test]
fn a_directory_or_a_file_above_is_in_the_way() {
    // `other` tracks a file `dist`; an ignored directory `dist/` holds his.
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("dist", b"theirs\n");
    repo.commit_all("dist on other");
    repo.git(&["switch", "-q", "main"]);
    repo.write(".git/info/exclude", b"dist/\n");
    repo.write("dist/app.js", MINE);
    assert_refused(
        &repo,
        &switch_to("other"),
        "dist/app.js",
        "dist (ignored) is in the way of switching to other · move it aside first",
    );

    // `other` tracks `a/b`; an ignored file `a` holds his.
    let repo = TempRepo::init();
    repo.write("x.txt", b"x\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("a/b", b"theirs\n");
    repo.commit_all("a/b on other");
    repo.git(&["switch", "-q", "main"]);
    repo.write(".git/info/exclude", b"a\n");
    repo.write("a", MINE);
    assert_refused(
        &repo,
        &switch_to("other"),
        "a",
        "a (ignored) is in the way of switching to other · move it aside first",
    );
}

/// No false refusal: ignored build output no commit writes.
#[test]
fn ignored_build_output_elsewhere_is_not_in_the_way() {
    let repo = main_and_other_unignored();
    repo.write(".git/info/exclude", b"target/\n");
    repo.write("target/out.bin", MINE);
    let outcome = attempt(&repo, &switch_to("other"), "target/out.bin");
    assert!(
        outcome.preview.is_ok(),
        "{}",
        preview_words(&outcome.preview)
    );
    assert!(
        outcome.applied.is_ok(),
        "{}",
        applied_words(&outcome.applied)
    );
    assert_eq!(outcome.bytes, MINE);
}

/// A tracked file the target replaces with a directory is git's to remove,
/// not his: `a` tracked on `main`, `a/b` on `other`.
#[test]
fn a_tracked_file_above_is_not_in_the_way() {
    let repo = TempRepo::init();
    repo.write("a", b"tracked a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.git(&["rm", "-q", "a"]);
    repo.write("a/b", b"theirs\n");
    repo.commit_all("a/b on other");
    repo.git(&["switch", "-q", "main"]);
    let outcome = attempt(&repo, &switch_to("other"), "a/b");
    assert!(
        outcome.preview.is_ok(),
        "{}",
        preview_words(&outcome.preview)
    );
    assert!(
        outcome.applied.is_ok(),
        "{}",
        applied_words(&outcome.applied)
    );
    assert_eq!(outcome.bytes, b"theirs\n");
}

/// Whether the disk under `dir` finds a name whatever its case (macOS's
/// default APFS does; most Linux disks don't).
fn case_insensitive(dir: &std::path::Path) -> bool {
    std::fs::write(dir.join("CaseProbe"), b"").unwrap();
    let found = dir.join("caseprobe").exists();
    std::fs::remove_file(dir.join("CaseProbe")).unwrap();
    found
}

/// Review 28 M2: `other` renames the tracked `Readme.md` to `README.md`.
/// On a case-insensitive disk the stat of `README.md` finds `Readme.md` —
/// his tracked file, git's to rename, not a file in the way. Plain `git
/// switch other` succeeds. On a case-sensitive disk nothing is found under
/// the other spelling, so the test has nothing to show and returns early,
/// saying so.
#[test]
fn a_case_only_rename_is_not_in_the_way() {
    let repo = TempRepo::init();
    if !case_insensitive(repo.path()) {
        eprintln!("skipped: this disk is case-sensitive");
        return;
    }
    repo.write("Readme.md", b"read me\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.git(&["mv", "Readme.md", "README.md"]);
    repo.git(&["commit", "-q", "-m", "case only"]);
    repo.git(&["switch", "-q", "main"]);
    let outcome = attempt(&repo, &switch_to("other"), "README.md");
    assert_eq!(
        (
            preview_words(&outcome.preview),
            applied_words(&outcome.applied),
            outcome.bytes,
            repo.git(&["ls-files"]),
        ),
        (
            "preview drew a plan".to_owned(),
            "apply ran".to_owned(),
            b"read me\n".to_vec(),
            "README.md".to_owned(),
        ),
        "(preview, apply, the bytes, the index)"
    );
}
