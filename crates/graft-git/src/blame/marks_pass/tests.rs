//! The second pass: the predicate, the merge, and `blame_with` through the
//! real spawn (samples and fixture helpers in `blame/tests.rs`).

use std::path::Path;

use super::*;
use crate::blame::tests::{
    HUMAN_2_38, PORCELAIN_2_38, SESSION_FILE, SKIPPED, head, marks_of, session_blame,
};
use crate::blame::{IGNORE_REVS_FILE, MarksFrom, find_ignore_revs, parse_porcelain};

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

/// The merge on 2.38.5's own two outputs: marks land by position; a line
/// count that differs (the working-tree file changed between the two
/// reads) is `Changed`, never a partial merge.
#[test]
fn marks_merge_by_position_and_a_count_mismatch_is_changed() {
    let (mut lines, _) = parse_porcelain(PORCELAIN_2_38).expect("parses");
    let marks = parse_marks(HUMAN_2_38).expect("parses");
    merge_marks(&mut lines, marks.clone()).expect("same count");
    let skipped: Vec<u32> = lines
        .iter()
        .filter(|line| line.mark == LineMark::Skipped)
        .map(|line| line.final_line)
        .collect();
    assert_eq!(skipped, SKIPPED);

    let (mut lines, _) = parse_porcelain(PORCELAIN_2_38).expect("parses");
    let mut longer = marks;
    longer.push(LineMark::Plain);
    match merge_marks(&mut lines, longer) {
        Err(BlameError::Changed { first, second }) => assert_eq!((first, second), (60, 61)),
        other => panic!("a count mismatch is Changed: {other:?}"),
    }
    assert!(
        lines.iter().all(|line| line.mark == LineMark::Plain),
        "nothing merged on a mismatch"
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
