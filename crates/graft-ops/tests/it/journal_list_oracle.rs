//! m7-power-tools/22: the journal's list against what real git writes.
//!
//! Each scenario is run with the system `git` (the shell's steps, dated so
//! their order is fixed) and Graft's own operations (the `●` rows), then
//! read back through `read_outside`, `find_lost` and `build` — never from a
//! hand-written record. What each asserts is the rows' kinds, words and
//! placement.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::time::{SystemTime, UNIX_EPOCH};

use graft_core::journal::{JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::read_commit;
use graft_git::repo::RepoHandle;
use graft_ops::facts::read_facts;
use graft_ops::journal_list::{
    HeldBy, JournalList, LOST_CAP, ListRow, LostScan, RECORDS_CAP, Recover, build, find_lost,
    read_outside, recover_for, recover_scope,
};
use graft_ops::operation::{Operation, ResetMode, ResetParams, ResetTarget, Tip};

use crate::support::{Never, TempRepo, git_version, journal_for, run};

/// The shell's clock for a scenario: fixed seconds, well before now, so
/// every step has its own second and Graft's steps (stamped now) come after.
pub(crate) const SHELL: i64 = 1_700_000_000;

/// `git` in the repository with its commit and reflog clock at `at`.
pub(crate) fn git_at(repo: &TempRepo, at: i64, args: &[&str]) -> String {
    let date = format!("{at} +0000");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(args)
        .env("GIT_COMMITTER_DATE", &date)
        .env("GIT_AUTHOR_DATE", &date)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned()
}

/// A commit of `file` with `subject`, at `at`.
pub(crate) fn commit_at(repo: &TempRepo, at: i64, file: &str, subject: &str) {
    repo.write(file, format!("{subject}\n").as_bytes());
    git_at(repo, at, &["add", "-A"]);
    git_at(repo, at, &["commit", "-q", "-m", subject]);
}

pub(crate) fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

pub(crate) fn now_ms() -> i64 {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970");
    i64::try_from(since.as_millis()).expect("fits")
}

/// `main` with A–D, each a second apart.
fn a_to_d() -> TempRepo {
    let repo = TempRepo::init();
    for (offset, subject) in ["A", "B", "C", "D"].iter().enumerate() {
        let offset = i64::try_from(offset).expect("small");
        commit_at(&repo, SHELL + offset, "f.txt", subject);
    }
    repo
}

fn entries(repo: &TempRepo) -> Vec<JournalEntry> {
    journal_for(&repo.open()).entries().to_vec()
}

/// The three stages, run the way the app runs them.
pub(crate) fn the_list(repo: &TempRepo) -> JournalList {
    let handle = repo.open();
    let entries = entries(repo);
    let outside = read_outside(&handle, RECORDS_CAP, &Never).expect("the reflogs read");
    let found =
        find_lost(&handle, &entries, &outside, LOST_CAP, &Never).expect("the lost scan runs");
    let subject_of = |tip: Oid| {
        let commit = read_commit(&handle, tip).expect("a lost tip reads");
        Some(String::from_utf8_lossy(commit.subject()).into_owned())
    };
    let exists = |name: &str| branch_exists(&handle, name);
    build(
        &entries,
        &outside,
        Some(&found),
        &subject_of,
        &exists,
        &[],
        LostScan::Done,
    )
}

pub(crate) fn branch_exists(handle: &RepoHandle, name: &str) -> bool {
    let full = RefName::new(format!("refs/heads/{name}").into_bytes()).expect("a ref name");
    handle
        .reference(&full)
        .expect("the ref store reads")
        .is_some()
}

/// Each row in one line: `$ <words>`, `● <kind>`, `✗ <subject> <held by>
/// under <row>`.
pub(crate) fn lines(list: &JournalList, entries: &[JournalEntry]) -> Vec<String> {
    list.rows
        .iter()
        .map(|row| match row {
            ListRow::Graft(id) => {
                let entry = entries
                    .iter()
                    .find(|entry| entry.id == *id)
                    .expect("its entry");
                format!("● {:?}", entry.kind)
            }
            ListRow::Outside(step) => format!("$ {}", step.words),
            ListRow::Lost { event, commit } => {
                let held = match &commit.held_by {
                    HeldBy::Branch { name, gone } => {
                        format!("{name}{}", if *gone { " gone" } else { "" })
                    }
                    HeldBy::Head => "HEAD".to_owned(),
                };
                // `the_list` reads every lost tip (it panics otherwise).
                let subject = commit.subject.as_deref().expect("the oracle read it");
                format!("✗ {subject} · {held} · under {event}")
            }
            ListRow::LostEdit { event, .. } => format!("✗ edit under {event}"),
        })
        .collect()
}

#[test]
fn shell_reset_hard_shows_the_lost_commit_under_it() {
    let repo = a_to_d();
    git_at(&repo, SHELL + 10, &["reset", "-q", "--hard", "HEAD~1"]);

    let list = the_list(&repo);

    assert_eq!(
        lines(&list, &[])[..3],
        [
            "$ reset main 1 commit back",
            "✗ D · main · under 0",
            "$ committed \"D\" on main",
        ]
    );
}

#[test]
fn a_deleted_branch_once_checked_out_is_found_from_head() {
    let repo = a_to_d();
    git_at(&repo, SHELL + 10, &["switch", "-q", "-c", "spike"]);
    commit_at(&repo, SHELL + 11, "spike.txt", "Try redis cache");
    git_at(&repo, SHELL + 12, &["switch", "-q", "main"]);
    git_at(&repo, SHELL + 13, &["branch", "-q", "-D", "spike"]);

    let list = the_list(&repo);

    assert_eq!(
        lines(&list, &[])[..4],
        [
            "$ switch from spike to main",
            "✗ Try redis cache · spike gone · under 0",
            "$ committed \"Try redis cache\" on spike",
            "$ switch from main to spike",
        ]
    );
}

/// `reflog.md` takeaway 12: a branch never checked out has only its own
/// log, and `branch -D` deletes that. Nothing is left to find.
#[test]
fn a_branch_deleted_and_never_checked_out_leaves_nothing() {
    let repo = a_to_d();
    let tree = repo.git(&["rev-parse", "HEAD^{tree}"]);
    let side = git_at(
        &repo,
        SHELL + 10,
        &["commit-tree", "-p", "HEAD", "-m", "Side work", &tree],
    );
    git_at(&repo, SHELL + 11, &["branch", "side", &side]);
    git_at(&repo, SHELL + 12, &["branch", "-q", "-D", "side"]);

    let list = the_list(&repo);

    let shown = lines(&list, &[]);
    assert!(
        shown
            .iter()
            .all(|line| !line.starts_with('✗') && !line.contains("side")),
        "{shown:#?}"
    );
}

fn graft_reset(repo: &TempRepo) -> JournalEntry {
    let op = Operation::Reset(ResetParams {
        target: ResetTarget::Head {
            branch: Tip {
                name: "refs/heads/main".to_owned(),
                oid: id(repo, "main"),
            },
            mode: ResetMode::Hard,
        },
        to: id(repo, "HEAD~1"),
    });
    run(repo, &op, now_ms())
}

#[test]
fn a_graft_reset_is_one_graft_row() {
    let repo = a_to_d();
    let reset = graft_reset(&repo);

    let list = the_list(&repo);

    let shown = lines(&list, std::slice::from_ref(&reset));
    assert_eq!(shown[0], "● Reset", "{shown:#?}");
    assert_eq!(
        list.rows
            .iter()
            .filter(|row| **row == ListRow::Graft(reset.id.clone()))
            .count(),
        1
    );
    assert!(
        shown
            .iter()
            .all(|line| !line.contains("reset main") && !line.contains("moved main")),
        "no `$` twin of Graft's reset: {shown:#?}"
    );
}

#[test]
fn a_shell_commit_is_an_outside_row() {
    let repo = a_to_d();
    commit_at(&repo, SHELL + 10, "e.txt", "Add metrics endpoint");

    let list = the_list(&repo);

    assert_eq!(
        lines(&list, &[])[0],
        "$ committed \"Add metrics endpoint\" on main"
    );
}

#[test]
fn a_shell_fetch_is_no_row() {
    let upstream = a_to_d();
    let repo = a_to_d();
    let before = lines(&the_list(&repo), &[]);
    repo.git(&[
        "remote",
        "add",
        "origin",
        &upstream.path().display().to_string(),
    ]);
    commit_at(&upstream, SHELL + 20, "u.txt", "Upstream work");
    git_at(&repo, SHELL + 21, &["fetch", "-q", "origin"]);

    let after = lines(&the_list(&repo), &[]);

    assert_eq!(after, before, "a remote-tracking log is not read");
}

#[test]
fn a_stash_held_commit_is_not_lost() {
    let repo = a_to_d();
    commit_at(&repo, SHELL + 10, "notes.txt", "Draft notes");
    repo.write("notes.txt", b"more notes\n");
    git_at(&repo, SHELL + 11, &["stash", "push", "-q"]);
    git_at(&repo, SHELL + 12, &["reset", "-q", "--hard", "HEAD~1"]);

    let list = the_list(&repo);

    let shown = lines(&list, &[]);
    assert_eq!(shown[0], "$ reset main 1 commit back", "{shown:#?}");
    assert!(
        shown.iter().all(|line| !line.starts_with('✗')),
        "{shown:#?}"
    );
}

/// Graft's own backup refs (`refs/tributary/**`) keep the reset's before-state,
/// but hold nothing for this list: D is lost, under the Graft row.
#[test]
fn a_graft_backup_ref_does_not_hold() {
    let repo = a_to_d();
    let reset = graft_reset(&repo);
    let graft_refs = repo.git(&["for-each-ref", "--format=%(refname)", "refs/tributary/"]);
    assert!(!graft_refs.is_empty(), "the reset made a backup ref");

    let list = the_list(&repo);

    let shown = lines(&list, &[reset]);
    assert_eq!(
        shown[..2],
        ["● Reset", "✗ D · main · under 0"],
        "{shown:#?}"
    );
}

// ── detached checkouts of a name that is not a branch (review m7-22, M1) ──

/// What `⏎` on the list's `✗` row offers, with the refs `recover_scope`
/// names read from the repository the way the app reads them.
fn recover_lost(repo: &TempRepo, list: &JournalList) -> Recover {
    let row = list
        .rows
        .iter()
        .position(|row| matches!(row, ListRow::Lost { .. }))
        .expect("a lost row");
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &recover_scope(list, row))
        .expect("the facts read");
    recover_for(list, row, &[], &facts).expect("a put-back or an offer")
}

/// He checks out `<name>` detached, commits, and switches back to `main`:
/// the commit is held by nothing. Returns the commit.
fn detached_work_on(repo: &TempRepo, name: &str) -> Oid {
    git_at(repo, SHELL + 11, &["checkout", "-q", name]);
    commit_at(repo, SHELL + 12, "detached.txt", "Detached work");
    let work = id(repo, "HEAD");
    git_at(repo, SHELL + 13, &["checkout", "-q", "main"]);
    work
}

/// `git checkout origin/main` leaves `HEAD` detached, though git's subject
/// reads `moving from main to origin/main`. The next switch leaves from the
/// hex id, git's proof. The list says `HEAD`, never "origin/main gone", and
/// `⏎` offers `restored`: a `refs/heads/origin/main` would shadow the
/// remote branch for every later `git rebase origin/main`.
#[test]
fn a_detached_checkout_of_a_remote_branch_is_not_a_branch() {
    let repo = a_to_d();
    git_at(
        &repo,
        SHELL + 10,
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let work = detached_work_on(&repo, "origin/main");

    let list = the_list(&repo);

    let short = &work.to_string()[..7];
    assert_eq!(
        lines(&list, &[])[..4],
        [
            format!("$ switch from {short} to main"),
            "✗ Detached work · HEAD · under 0".to_owned(),
            "$ committed \"Detached work\" on HEAD".to_owned(),
            "$ switch from main to origin/main".to_owned(),
        ]
    );
    assert_eq!(
        recover_lost(&repo, &list),
        Recover::OfferBranch {
            at: work,
            name: "restored".to_owned(),
            why: "no branch held it, only HEAD".to_owned(),
        }
    );
    assert!(
        repo.try_git(&["rev-parse", "--verify", "-q", "refs/heads/origin/main"])
            .is_err(),
        "no local branch of that name"
    );
}

/// The same with a tag: `git checkout v1` is detached at the tag.
#[test]
fn a_detached_checkout_of_a_tag_is_not_a_branch() {
    let repo = a_to_d();
    git_at(&repo, SHELL + 10, &["tag", "v1", "HEAD~1"]);
    let work = detached_work_on(&repo, "v1");

    let list = the_list(&repo);

    let short = &work.to_string()[..7];
    assert_eq!(
        lines(&list, &[])[..4],
        [
            format!("$ switch from {short} to main"),
            "✗ Detached work · HEAD · under 0".to_owned(),
            "$ committed \"Detached work\" on HEAD".to_owned(),
            "$ switch from main to v1".to_owned(),
        ]
    );
    assert_eq!(
        recover_lost(&repo, &list),
        Recover::OfferBranch {
            at: work,
            name: "restored".to_owned(),
            why: "no branch held it, only HEAD".to_owned(),
        }
    );
}

/// A Graft switch writes `checkout: moving from main to spike` and records
/// `HEAD`'s branches, no commit: it is still one `●` row.
#[test]
fn a_graft_switch_is_one_graft_row() {
    let repo = a_to_d();
    git_at(&repo, SHELL + 10, &["branch", "spike", "HEAD~1"]);
    let op = Operation::Checkout(graft_ops::operation::CheckoutParams {
        to: graft_ops::operation::CheckoutTarget::Branch("refs/heads/spike".to_owned()),
        head_before: graft_core::journal::HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    });
    let switch = run(&repo, &op, now_ms());
    assert_eq!(switch.kind, OperationKind::Checkout);

    let list = the_list(&repo);

    let shown = lines(&list, &[switch]);
    assert_eq!(shown[0], "● Checkout", "{shown:#?}");
    assert!(shown[1].starts_with("$ made spike at "), "{shown:#?}");
    assert!(
        shown.iter().all(|line| !line.contains("switch from")),
        "{shown:#?}"
    );
}

/// `<ref>`'s log, newest first, one subject per line: what the m7/38 NOTES
/// quote.
fn log_of(repo: &TempRepo, refname: &str) -> Vec<String> {
    repo.git(&["reflog", "--format=%gs", refname])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `git -c sequence.editor=… -c core.editor=true rebase -q -i <args>` at
/// `at`, the todo edited by the sed `script`; whether git finished.
fn rebase_i(repo: &TempRepo, at: i64, script: &str, args: &[&str]) -> bool {
    let date = format!("{at} +0000");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .arg("-c")
        .arg(format!("sequence.editor=sed -i.bak -e '{script}'"))
        .args(["-c", "core.editor=true", "rebase", "-q", "-i"])
        .args(args)
        .env("GIT_COMMITTER_DATE", &date)
        .env("GIT_AUTHOR_DATE", &date)
        .output()
        .expect("git runs");
    output.status.success()
}

/// The lost rows' subjects, newest first.
fn lost_subjects(list: &JournalList) -> Vec<String> {
    list.rows
        .iter()
        .filter_map(|row| match row {
            ListRow::Lost { commit, .. } => commit.subject.clone(),
            _ => None,
        })
        .collect()
}

/// m7/38: `rebase -i` squashing B C D into one, finished. git's interim
/// "# This is a combination of 2 commits." is in the result: no `✗` for it.
/// The old tip D is main's, named by the start and main's own finish.
#[test]
fn a_finished_squash_leaves_no_lost_row() {
    let repo = a_to_d();
    assert!(rebase_i(
        &repo,
        SHELL + 10,
        "2,3s/^pick/squash/",
        &["HEAD~3"]
    ));
    let head = log_of(&repo, "HEAD");
    eprintln!(
        "HEAD: {head:#?}\nmain: {:#?}",
        log_of(&repo, "refs/heads/main")
    );
    // git 2.54.0. B fast-forwards (no record); C's squash is the interim.
    assert_eq!(
        head[..4],
        [
            "rebase (finish): returning to refs/heads/main",
            "rebase (squash): B",
            "rebase (squash): # This is a combination of 2 commits.",
            "rebase (start): checkout HEAD~3",
        ],
        "{head:#?}"
    );

    let list = the_list(&repo);

    let subjects = lost_subjects(&list);
    assert!(
        subjects.iter().all(|subject| subject == "D"),
        "only the old tip may be lost: {subjects:#?}\n{:#?}",
        lines(&list, &[])
    );
}

/// The same kind of todo stopped by a conflict, then `rebase --abort`: the
/// copies made before the stop stay lost rows — an aborted span may hold his
/// work.
#[test]
fn an_aborted_rebase_keeps_its_lost_rows() {
    let repo = TempRepo::init();
    commit_at(&repo, SHELL, "f.txt", "A");
    commit_at(&repo, SHELL + 1, "b.txt", "B");
    commit_at(&repo, SHELL + 2, "c.txt", "C");
    commit_at(&repo, SHELL + 3, "f.txt", "D");
    git_at(&repo, SHELL + 4, &["switch", "-q", "-c", "side", "HEAD~3"]);
    commit_at(&repo, SHELL + 5, "f.txt", "X");
    git_at(&repo, SHELL + 6, &["switch", "-q", "main"]);
    assert!(
        !rebase_i(&repo, SHELL + 10, "2s/^pick/squash/", &["side"]),
        "D conflicts with X"
    );
    git_at(&repo, SHELL + 11, &["rebase", "--abort"]);
    let head = log_of(&repo, "HEAD");
    eprintln!("HEAD: {head:#?}");
    // git 2.54.0: `(abort)`, then the squash and the pick (both "B": the
    // squash of C is its group's last, so it carries the final message).
    assert_eq!(
        head[..4],
        [
            "rebase (abort): returning to refs/heads/main",
            "rebase (squash): B",
            "rebase (pick): B",
            "rebase (start): checkout side",
        ],
        "{head:#?}"
    );
    let squashed = id(&repo, "HEAD@{1}");
    let picked = id(&repo, "HEAD@{2}");

    let list = the_list(&repo);

    let tips: Vec<Oid> = list
        .rows
        .iter()
        .filter_map(|row| match row {
            ListRow::Lost { commit, .. } => Some(commit.run.tip),
            _ => None,
        })
        .collect();
    assert!(
        tips.contains(&squashed),
        "the squash stays lost: {:#?}",
        lines(&list, &[])
    );
    assert!(
        tips.contains(&picked),
        "the pick's copy stays lost: {:#?}",
        lines(&list, &[])
    );
}

/// An `edit` stop, a commit E, `reset --hard HEAD~1`, continue to the
/// finish: E was reset away by a record that is no rebase step — his lost
/// work.
#[test]
fn a_reset_during_an_edit_stop_stays_lost() {
    let repo = a_to_d();
    assert!(rebase_i(&repo, SHELL + 10, "1s/^pick/edit/", &["HEAD~2"]));
    commit_at(&repo, SHELL + 11, "e.txt", "E");
    git_at(&repo, SHELL + 12, &["reset", "-q", "--hard", "HEAD~1"]);
    git_at(
        &repo,
        SHELL + 13,
        &["-c", "core.editor=true", "rebase", "--continue"],
    );
    eprintln!("HEAD: {:#?}", log_of(&repo, "HEAD"));

    let list = the_list(&repo);

    let subjects = lost_subjects(&list);
    assert!(
        subjects.iter().any(|subject| subject == "E"),
        "{subjects:#?}\n{:#?}",
        lines(&list, &[])
    );
}

/// m8/14: a `rebase -i` whose picks after an `update-ref` line move `HEAD`
/// without a new commit writes `rebase: fast-forward` (no brackets) between
/// its start and finish. The run is still one rebase: one `$` row.
#[test]
fn a_rebase_that_fast_forwards_is_one_row() {
    let repo = TempRepo::init();
    commit_at(&repo, SHELL, "f.txt", "base");
    git_at(&repo, SHELL + 1, &["switch", "-q", "-c", "fa"]);
    commit_at(&repo, SHELL + 2, "a.txt", "A");
    git_at(&repo, SHELL + 3, &["switch", "-q", "-c", "fb"]);
    commit_at(&repo, SHELL + 4, "b.txt", "B");
    commit_at(&repo, SHELL + 5, "c.txt", "C");
    assert!(rebase_i(
        &repo,
        SHELL + 10,
        "s/^pick/pick/",
        &["--update-refs", "main"]
    ));
    let head = log_of(&repo, "HEAD");
    // git 2.54.0: A is skipped (already in place), B and C fast-forward.
    assert_eq!(
        head[..4],
        [
            "rebase (finish): returning to refs/heads/fb",
            "rebase: fast-forward",
            "rebase: fast-forward",
            "rebase (start): checkout main",
        ],
        "{head:#?}"
    );

    let list = the_list(&repo);

    let rows = lines(&list, &[]);
    assert_eq!(rows[0], "$ rebased fb onto main", "{rows:#?}");
    assert!(
        !rows[1].contains("rebase") && !rows[1].contains("fast-forward"),
        "the fast-forwards fold into the rebase: {rows:#?}"
    );
}
