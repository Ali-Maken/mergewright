//! The second pass: the predicate, `configured_list`, and `blame_with`
//! through the real spawn (fixture helpers in `blame/tests.rs`; the merge's
//! tests are in `blame/marks/tests.rs`, beside the samples' reader).

use std::path::Path;

use super::*;
use crate::blame::tests::{
    SESSION_FILE, SKIPPED, head, marks_of, session_blame, session_blame_with,
};
use crate::blame::{
    Blame, IGNORE_REVS_FILE, Knobs, LineMark, MarksFrom, PorcelainMarks, blame_with,
    find_ignore_revs, parse_porcelain,
};

#[test]
fn needs_marks_pass_holds_only_when_all_four_do() {
    let old = GitVersion {
        major: 2,
        minor: 38,
        patch: 5,
    };
    let just_before = GitVersion {
        major: 2,
        minor: 49,
        patch: 1,
    };
    let new = GitVersion::BLAME_PORCELAIN_MARKS;
    assert!(
        needs_marks_pass(true, false, true, Some(&old)),
        "all four hold"
    );
    assert!(needs_marks_pass(true, false, true, Some(&just_before)));
    assert!(
        needs_marks_pass(true, false, true, None),
        "unknown version: take the pass"
    );
    assert!(
        !needs_marks_pass(false, false, true, Some(&old)),
        "1 flips it"
    );
    assert!(
        !needs_marks_pass(true, true, true, Some(&old)),
        "2 flips it"
    );
    assert!(
        !needs_marks_pass(true, false, false, Some(&old)),
        "3 flips it"
    );
    assert!(
        !needs_marks_pass(true, false, true, Some(&new)),
        "4 flips it"
    );
}

fn passes() -> usize {
    MARKS_PASSES.with(std::cell::Cell::get)
}

/// The oracle on a git whose porcelain writes marks (2.50+): the forced
/// human-format pass gives git's own porcelain marks, line for line. An
/// older git's porcelain has none to compare against: there the human pass
/// is checked against the fixture's known lines only, and the test says so.
#[test]
fn the_human_format_marks_match_the_porcelain_marks() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let human = marks_of(&session_blame(&dir, MarksFrom::HumanFormat));
    let skipped = human
        .iter()
        .filter(|mark| **mark == LineMark::Skipped)
        .count();
    assert!(skipped >= 1, "not vacuous: the list skips lines");
    let git = system_git().expect("git's version reads").version();
    if git < GitVersion::BLAME_PORCELAIN_MARKS {
        eprintln!("git {git} writes no porcelain marks: the oracle needs git >= 2.50");
        assert_eq!(skipped, SKIPPED.len());
        return;
    }
    let repo = RepoHandle::open(&dir).expect("opens");
    let output = blame_invocation(
        &repo,
        &Rev::Commit(head(&dir)),
        Path::new(SESSION_FILE),
        BlameOptions::default(),
        find_ignore_revs(&repo).as_ref(),
        Format::Porcelain,
    )
    .run(&())
    .expect("git's porcelain");
    let (porcelain, _) = parse_porcelain(&output.stdout).expect("parses");
    let porcelain: Vec<LineMark> = porcelain.iter().map(|line| line.mark).collect();
    assert_eq!(human, porcelain, "git {git}: line for line");
}

/// Condition 3: a repository with no list pays one blame spawn, on any git.
#[test]
fn no_list_in_effect_is_one_blame_spawn() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    let dir = scratch.path();
    std::fs::remove_file(dir.join(IGNORE_REVS_FILE)).expect("removes the list");
    let before = passes();
    let blamed = session_blame(dir, MarksFrom::Auto);
    assert_eq!(passes(), before, "no human-format pass");
    assert!(blamed.lines.iter().all(|line| line.mark == LineMark::Plain));
}

/// With a list, the pass runs exactly when the porcelain had no marks: on
/// git before 2.50 once, on 2.50+ never. Either way the marks are right.
#[test]
fn a_list_in_effect_takes_the_pass_only_on_a_git_without_porcelain_marks() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let before = passes();
    let blamed = session_blame(&dir, MarksFrom::Auto);
    let git = system_git().expect("git's version reads").version();
    let expected = usize::from(git < GitVersion::BLAME_PORCELAIN_MARKS);
    assert_eq!(passes() - before, expected, "git {git}");
    let skipped: Vec<u32> = blamed
        .lines
        .iter()
        .filter(|line| line.mark == LineMark::Skipped)
        .map(|line| line.final_line)
        .collect();
    assert_eq!(skipped, SKIPPED, "git {git}");
}

/// Condition 3's other half: no root file, a configured list. On a git
/// without porcelain marks the pass still runs.
#[test]
fn a_configured_list_alone_is_a_list_in_effect() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    let dir = scratch.path();
    std::fs::rename(dir.join(IGNORE_REVS_FILE), dir.join("elsewhere-revs")).expect("moves");
    let status = std::process::Command::new("git")
        .args(["config", "blame.ignoreRevsFile", "elsewhere-revs"])
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success());
    let blamed = session_blame(dir, MarksFrom::Auto);
    let skipped: Vec<u32> = blamed
        .lines
        .iter()
        .filter(|line| line.mark == LineMark::Skipped)
        .map(|line| line.final_line)
        .collect();
    assert_eq!(skipped, SKIPPED);
}

// ---- The git < 2.50 path on any git (18f) --------------------------------
//
// `as_old_git` sets the Auto path's two git-dependent inputs the way git
// 2.38 gives them: its version, and a porcelain with no marks. Each test
// asserts the marks **and** the second-pass count, so a seam that stopped
// dropping the marks can't pass on a git that writes them.

fn as_old_git() -> Knobs {
    Knobs {
        git: GitFrom::Given(GitVersion::MINIMUM),
        porcelain_marks: PorcelainMarks::Dropped,
        ..Knobs::real()
    }
}

fn skipped_lines(blame: &Blame) -> Vec<u32> {
    blame
        .lines
        .iter()
        .filter(|line| line.mark == LineMark::Skipped)
        .map(|line| line.final_line)
        .collect()
}

fn git_ok(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

/// The session fixture with its list moved off the root and named by
/// `blame.ignoreRevsFile` instead.
fn session_with_configured_list() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    let dir = scratch.path();
    std::fs::rename(dir.join(IGNORE_REVS_FILE), dir.join("elsewhere-revs")).expect("moves");
    git_ok(dir, &["config", "blame.ignoreRevsFile", "elsewhere-revs"]);
    scratch
}

/// Review 18e-20f Major 1's mutation: `Some(0) => Ok(false)` in
/// `configured_list` turns this red on git 2.54.
#[test]
fn as_old_git_a_configured_list_alone_takes_the_pass() {
    let scratch = session_with_configured_list();
    let before = passes();
    let blamed = session_blame_with(scratch.path(), &as_old_git()).expect("blames");
    assert_eq!(passes() - before, 1, "one second pass");
    assert_eq!(skipped_lines(&blamed), SKIPPED);
}

#[test]
fn as_old_git_the_root_file_takes_the_pass() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let before = passes();
    let blamed = session_blame_with(&dir, &as_old_git()).expect("blames");
    assert_eq!(passes() - before, 1, "one second pass");
    assert_eq!(skipped_lines(&blamed), SKIPPED);
}

/// 18e's unblamable repository (`tests/it/blame.rs`
/// `an_unblamable_line_is_marked_unblamable`): an ignored commit added
/// line 2.
#[test]
fn as_old_git_an_unblamable_line_is_unblamable() {
    let temp = tempfile::tempdir().expect("a temp dir");
    let dir = temp.path();
    git_ok(dir, &["init", "-q", "-b", "main"]);
    git_ok(dir, &["config", "user.name", "T"]);
    git_ok(dir, &["config", "user.email", "t@example.com"]);
    std::fs::write(dir.join("a.rs"), "fn one() {}\nfn two() {}\n").expect("writes");
    git_ok(dir, &["add", "a.rs"]);
    git_ok(dir, &["commit", "-q", "-m", "add"]);
    std::fs::write(
        dir.join("a.rs"),
        "fn one() {}\nfn added_by_the_ignored_commit() {}\nfn two() {}\n",
    )
    .expect("writes");
    git_ok(dir, &["commit", "-q", "-am", "reformat"]);
    let ignored = head(dir);
    std::fs::write(dir.join(IGNORE_REVS_FILE), format!("{ignored}\n")).expect("writes");
    git_ok(dir, &["add", IGNORE_REVS_FILE]);
    git_ok(dir, &["commit", "-q", "-m", "list it"]);

    let repo = RepoHandle::open(dir).expect("opens");
    let before = passes();
    let blamed = blame_with(
        &repo,
        &Rev::Commit(head(dir)),
        Path::new("a.rs"),
        &BlameOptions::default(),
        &(),
        &as_old_git(),
    )
    .expect("blames");
    assert_eq!(passes() - before, 1, "one second pass");
    assert_eq!(
        marks_of(&blamed),
        [LineMark::Plain, LineMark::Unblamable, LineMark::Plain]
    );
}

#[test]
fn as_old_git_no_list_is_no_pass() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    let dir = scratch.path();
    std::fs::remove_file(dir.join(IGNORE_REVS_FILE)).expect("removes the list");
    let before = passes();
    let blamed = session_blame_with(dir, &as_old_git()).expect("blames");
    assert_eq!(passes() - before, 0, "no second pass");
    assert!(
        marks_of(&blamed)
            .iter()
            .all(|mark| *mark == LineMark::Plain)
    );
}

#[test]
fn as_old_git_ignore_revs_off_is_no_pass() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let repo = RepoHandle::open(&dir).expect("opens");
    let before = passes();
    let blamed = blame_with(
        &repo,
        &Rev::Commit(head(&dir)),
        Path::new(SESSION_FILE),
        &BlameOptions {
            ignore_revs: false,
            ..BlameOptions::default()
        },
        &(),
        &as_old_git(),
    )
    .expect("blames");
    assert_eq!(passes() - before, 0, "no second pass");
    assert!(
        marks_of(&blamed)
            .iter()
            .all(|mark| *mark == LineMark::Plain)
    );
}

/// Condition 4's edge: the first git with porcelain marks takes no pass.
/// `<` turned `<=` in `needs_marks_pass` turns this red.
#[test]
fn at_the_marks_version_no_pass() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let before = passes();
    let blamed = session_blame_with(
        &dir,
        &Knobs {
            git: GitFrom::Given(GitVersion::BLAME_PORCELAIN_MARKS),
            ..as_old_git()
        },
    )
    .expect("blames");
    assert_eq!(passes() - before, 0, "no second pass");
    assert!(
        marks_of(&blamed)
            .iter()
            .all(|mark| *mark == LineMark::Plain)
    );
}

/// Review 18e-20f Major 1 (b): set, unset (exit 1), and a config git can't
/// read (an error, never "unset").
#[test]
fn configured_list_reads_set_unset_and_refuses_a_broken_config() {
    let temp = tempfile::tempdir().expect("a temp dir");
    let dir = temp.path();
    git_ok(dir, &["init", "-q", "-b", "main"]);
    let repo = RepoHandle::open(dir).expect("opens");
    assert!(!configured_list(&repo, &()).expect("unset reads"), "unset");
    git_ok(dir, &["config", "blame.ignoreRevsFile", "revs"]);
    assert!(configured_list(&repo, &()).expect("set reads"), "set");
    std::fs::write(dir.join(".git/config"), "[core\nnot a config\n").expect("breaks it");
    match configured_list(&repo, &()) {
        Err(BlameError::Run(RunError::Exited { status, .. })) => {
            eprintln!("git config --get on a broken config exited {status}");
            assert!(status > 1, "not read as set or unset: {status}");
        }
        other => panic!("a broken config is an error: {other:?}"),
    }
}

/// Review 18e-20f Minor 2: `git --version` once per process. The count is
/// per thread and the cache per process, so under a shared-process runner
/// another test may have filled it (zero reads here); the second blame
/// never reads. Red with the cache removed.
#[test]
fn the_system_version_is_read_once_per_process() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let knobs = Knobs {
        porcelain_marks: PorcelainMarks::Dropped,
        ..Knobs::real()
    };
    let reads = || VERSION_READS.with(std::cell::Cell::get);
    let before = reads();
    session_blame_with(&dir, &knobs).expect("blames");
    let after_first = reads();
    session_blame_with(&dir, &knobs).expect("blames");
    assert!(after_first - before <= 1, "at most one read");
    assert_eq!(reads(), after_first, "the second blame reads no version");
    assert!(SYSTEM_VERSION.get().is_some(), "kept");
}

/// Review 18e-20f Minor 4: the second pass has its own cap refusal.
#[test]
fn a_second_pass_cut_at_its_cap_is_too_large() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let result = session_blame_with(
        &dir,
        &Knobs {
            marks: MarksFrom::HumanFormat,
            marks_cap: 1000,
            ..Knobs::real()
        },
    );
    match result {
        Err(BlameError::TooLarge { cap }) => assert_eq!(cap, 1000),
        other => panic!("a capped second pass must not read as whole: {other:?}"),
    }
}
