//! `parse_porcelain` and `parse_marks` on git's own bytes, and
//! `blame_with` through the real spawn.
//!
//! Every sample is a capture, none written by hand (18e replaced the hand-made
//! "2.38" one, which carried an `unblamable` line real 2.38.5 never writes).
//! The session samples are the `session` fixture at `HEAD`
//! (`74f9a9e`, whose `.git-blame-ignore-revs` lists the rustfmt commit):
//!
//! ```text
//! git -c blame.markIgnoredLines=true -c blame.markUnblamableLines=true \
//!     blame --porcelain -M -w --ignore-revs-file=.git-blame-ignore-revs HEAD -- src/auth/session.rs
//! git -c blame.markIgnoredLines=true -c blame.markUnblamableLines=true \
//!     -c blame.blankBoundary=false -c blame.coloring=none -c color.ui=never \
//!     blame -l -s -M -w --ignore-revs-file=.git-blame-ignore-revs HEAD -- src/auth/session.rs
//! ```
//!
//! The first as `PORCELAIN_*`, the second as `HUMAN_*`, run by git 2.38.5
//! (built from kernel.org's tarball, 18b) and git 2.54.0. The `EDGES_*`
//! samples are the same two commands, no rev, on the repository
//! `testdata/blame-edges.sh` builds: a quoted path, a skipped line, a line
//! both skipped and unblamable, an uncommitted line.

use super::*;
use crate::version::GitVersion;

const PORCELAIN_2_54: &[u8] = include_bytes!("../testdata/git-2.54/blame-porcelain.txt");
const PORCELAIN_2_38: &[u8] = include_bytes!("../testdata/git-2.38/blame-porcelain.txt");
const HUMAN_2_54: &[u8] = include_bytes!("../testdata/git-2.54/blame-human.txt");
const HUMAN_2_38: &[u8] = include_bytes!("../testdata/git-2.38/blame-human.txt");
const EDGES_2_38: &[u8] = include_bytes!("../testdata/git-2.38/blame-edges-porcelain.txt");
const EDGES_2_54: &[u8] = include_bytes!("../testdata/git-2.54/blame-edges-porcelain.txt");
const EDGES_HUMAN_2_38: &[u8] = include_bytes!("../testdata/git-2.38/blame-edges-human.txt");
const EDGES_HUMAN_2_54: &[u8] = include_bytes!("../testdata/git-2.54/blame-edges-human.txt");

/// The session fixture's lines git skipped under its list: rustfmt's.
const SKIPPED: [u32; 6] = [31, 32, 33, 35, 36, 37];

fn id(hex: &str) -> Oid {
    Oid::from_hex(hex).expect("a test id")
}

#[test]
fn the_2_54_sample_reads_every_line_with_its_commit_and_mark() {
    let (lines, commits) = parse_porcelain(PORCELAIN_2_54).expect("git 2.54's porcelain parses");
    assert_eq!(lines.len(), 60, "the fixture's file has 60 lines");
    for (index, line) in lines.iter().enumerate() {
        assert_eq!(line.final_line as usize, index + 1, "final lines in order");
        let commit = line
            .commit
            .expect("every line of a committed file has a commit");
        assert!(
            commits.contains_key(&commit),
            "line {} names a known commit",
            index + 1
        );
    }
    assert_eq!(
        lines[25].text,
        b"            self.token = resp.token().map_err(Expired::from)?;"
    );
    let skipped: Vec<u32> = lines
        .iter()
        .filter(|line| line.mark == LineMark::Skipped)
        .map(|line| line.final_line)
        .collect();
    assert_eq!(skipped, [31, 32, 33, 35, 36, 37], "git's `ignored` lines");
    let ana = &commits[&lines[0].commit.expect("line 1")];
    assert_eq!(ana.author, "Ana");
    assert_eq!(ana.author_time, 1_541_721_600);
    assert_eq!(ana.summary, "Add session");
    assert!(ana.boundary, "the root commit is git's boundary");
    assert_eq!(ana.previous, None);
    let priya = &commits[&lines[1].commit.expect("line 2")];
    assert_eq!(priya.author, "Priya");
    let (parent, old_path) = priya.previous.clone().expect("Priya's commit has a parent");
    assert_eq!(
        old_path,
        PathBuf::from("src/session.rs"),
        "the name before the rename"
    );
    assert_ne!(parent, priya.id, "previous is the parent, not the commit");
}

const ANA: &str = "6393164403945a213b6243945025f6cff2dd8689";
const TOM: &str = "544c71e0bff76203152b2a8997d2b3f3ea38ee01";

/// Review 18bcd Major 1: real 2.38.5 porcelain carries no `ignored` line, so
/// the skipped lines read `Plain` from the porcelain alone.
#[test]
fn the_2_38_porcelain_leaves_the_skipped_lines_plain() {
    let (lines, _) = parse_porcelain(PORCELAIN_2_38).expect("git 2.38.5's porcelain parses");
    assert_eq!(lines.len(), 60);
    assert_eq!(lines[30].text, b"    pub fn token(&self) -> &str {");
    assert!(
        lines.iter().all(|line| line.mark == LineMark::Plain),
        "2.38.5 writes no marks in porcelain"
    );
    let (lines_2_54, _) = parse_porcelain(PORCELAIN_2_54).expect("2.54 parses");
    let attribution = |lines: &[BlameLine]| -> Vec<(Option<Oid>, u32, Vec<u8>)> {
        lines
            .iter()
            .map(|line| (line.commit, line.orig_line, line.text.clone()))
            .collect()
    };
    assert_eq!(
        attribution(&lines),
        attribution(&lines_2_54),
        "same attribution on both gits"
    );
}

#[test]
fn the_2_38_edges_read_quoted_paths_and_uncommitted_lines() {
    let (lines, commits) = parse_porcelain(EDGES_2_38).expect("git 2.38.5's porcelain parses");
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[1].commit, Some(id(ANA)));
    assert_eq!((lines[4].orig_line, lines[4].final_line), (4, 5));
    assert!(
        lines.iter().all(|line| line.mark == LineMark::Plain),
        "2.38.5: no marks in porcelain"
    );
    assert_eq!(
        lines[5].commit, None,
        "the all-zero id is not committed yet"
    );
    assert_eq!(lines[5].text, b"let fresh = 1;");
    assert_eq!(
        commits.len(),
        2,
        "the uncommitted pseudo-commit is not a commit"
    );
    let tom = &commits[&id(TOM)];
    let (_, old_path) = tom.previous.clone().expect("previous");
    assert_eq!(
        path_bytes(&old_path),
        "src/\u{e9}.rs".as_bytes(),
        "C-quoting undone"
    );
    assert_eq!(commits[&id(ANA)].summary, "Add \u{e9} file");
}

/// 2.54 writes `unblamable` then `ignored` for line 4; the reader keeps it
/// unblamable, as `parse_marks` does for `*?`.
#[test]
fn the_2_54_edges_read_both_marks_on_one_line_as_unblamable() {
    use LineMark::{Plain, Skipped, Unblamable};
    let (lines, _) = parse_porcelain(EDGES_2_54).expect("git 2.54's porcelain parses");
    let marks: Vec<LineMark> = lines.iter().map(|line| line.mark).collect();
    assert_eq!(marks, [Plain, Plain, Skipped, Unblamable, Plain, Plain]);
    let human = parse_marks(EDGES_HUMAN_2_54).expect("the human format parses");
    assert_eq!(human, marks, "both readers agree");
    let human = parse_marks(EDGES_HUMAN_2_38).expect("the human format parses");
    assert_eq!(human, marks, "2.38.5's human format too");
}

#[test]
fn the_human_samples_mark_the_skipped_lines_on_both_gits() {
    for (git, sample) in [("2.38.5", HUMAN_2_38), ("2.54", HUMAN_2_54)] {
        let marks = parse_marks(sample).expect("the human format parses");
        assert_eq!(marks.len(), 60, "{git}: one mark per line");
        let skipped: Vec<u32> = (1..)
            .zip(&marks)
            .filter(|(_, mark)| **mark == LineMark::Skipped)
            .map(|(line, _)| line)
            .collect();
        assert_eq!(skipped, SKIPPED, "{git}");
        assert!(!marks.contains(&LineMark::Unblamable), "{git}");
    }
}

#[test]
fn parse_marks_reads_the_leading_run_only() {
    use LineMark::{Plain, Skipped, Unblamable};
    let id = "3f6c311a1b7f837c73d3b5eb4fa5180e047495";
    let bytes =
        format!("^?{id} 1) a ?* not a mark\n^*{id} 2) b\n*?{id} 3) c\n{id}00 4) ?\n^{id}0 5) *");
    assert_eq!(
        parse_marks(bytes.as_bytes()).expect("parses"),
        [Skipped, Unblamable, Unblamable, Plain, Plain],
        "a last line with no newline still counts"
    );
    assert_eq!(parse_marks(b"").expect("empty"), []);
}

#[test]
fn parse_marks_refuses_garbage_without_panicking() {
    for garbage in [
        &b"not a blame\n"[..],
        b"^?\n",
        b"??",
        b"\n\n",
        b" 1) x\n",
        b"\xff\xfe",
    ] {
        let error = parse_marks(garbage).expect_err("refused");
        assert_eq!(error.line, 1, "{garbage:?}");
    }
    let error = parse_marks(b"abc 1) x\nzz 2) y\n").expect_err("second line");
    assert_eq!(error.line, 2);
}

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

#[test]
fn the_new_errors_say_one_line_each() {
    let too_large = BlameError::TooLarge { cap: DEFAULT_CAP }.to_string();
    assert_eq!(
        too_large,
        "this file's blame is over 64 MiB, too large to show"
    );
    let changed = BlameError::Changed {
        first: 60,
        second: 61,
    }
    .to_string();
    assert_eq!(
        changed,
        "the file changed while blame read it; open blame again"
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

#[test]
fn a_truncated_entry_is_a_parse_error_not_a_panic() {
    let cut = &PORCELAIN_2_54[..PORCELAIN_2_54.len() / 2];
    // Cut mid-entry or mid-text: either an error, or the lines up to a
    // clean boundary — never a panic.
    if let Ok((lines, _)) = parse_porcelain(cut) {
        assert!(lines.len() < 60);
    }
    let error = parse_porcelain(b"not a blame\n").expect_err("garbage is refused");
    assert_eq!(error.line, 1);
    let error = parse_porcelain(b"1111111111111111111111111111111111111111 1 1 1\nauthor A\n")
        .expect_err("no text line");
    assert_eq!(error.line, 1);
}

#[test]
fn a_commit_first_seen_without_its_headers_is_refused() {
    let error = parse_porcelain(b"1111111111111111111111111111111111111111 1 1 1\n\tx\n")
        .expect_err("no author");
    assert!(error.what.contains("author"), "{}", error.what);
}

#[test]
fn revisions_count_skips_comments_and_blanks() {
    assert_eq!(count_revisions(b"# a comment\n\n  abc123\n\t\ndef456\n"), 2);
    assert_eq!(count_revisions(b""), 0);
}

#[test]
fn a_repeated_filename_keeps_the_commits_path() {
    let (_, commits) = parse_porcelain(EDGES_2_38).expect("parses");
    let ana = &commits[&id(ANA)];
    assert_eq!(ana.path, PathBuf::from("src/\u{e9}.rs"));
    // A later entry of the same commit repeats `filename`: the first stays.
    let bytes = b"\
1111111111111111111111111111111111111111 1 1 1
author A
author-time 1
summary s
filename a.rs
\tx
1111111111111111111111111111111111111111 2 2 1
filename a.rs
\ty
";
    let (lines, commits) = parse_porcelain(bytes).expect("parses");
    assert_eq!(lines.len(), 2);
    assert_eq!(
        commits[&id("1111111111111111111111111111111111111111")].path,
        PathBuf::from("a.rs")
    );
}

/// Review 18b Major 1: a porcelain cut at the runner's cap parses cleanly
/// when the cut lands inside a text line, so the flag alone must refuse it.
#[test]
fn a_porcelain_cut_at_the_output_cap_is_too_large() {
    // Inside the last text line: `\t}` without its newline.
    let cut = &PORCELAIN_2_38[..PORCELAIN_2_38.len() - 1];
    assert!(
        parse_porcelain(cut).is_ok(),
        "the premise: this cut parses as a shorter blame"
    );
    let output = GitOutput {
        stdout: cut.to_vec(),
        stderr: Vec::new(),
        truncated: true,
    };
    match read_porcelain(&output, 4096) {
        Err(BlameError::TooLarge { cap }) => assert_eq!(cap, 4096),
        other => panic!("a capped blame must not read as whole: {other:?}"),
    }
}

/// Review 18bcd Minor 3: the cap is refused at the call site, through the
/// real spawn. Replacing `read_porcelain(&output, knobs.cap)?` in
/// `blame_with` by `parse_porcelain(&output.stdout)` turns this red.
#[test]
fn a_blame_cut_at_the_output_cap_is_too_large_not_a_short_file() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Session);
    let repo = RepoHandle::open(&dir).expect("the fixture opens");
    let result = blame_with(
        &repo,
        &Rev::WorkingTree,
        Path::new(SESSION_FILE),
        &BlameOptions::default(),
        &(),
        &Knobs {
            cap: 1000,
            marks: MarksFrom::Auto,
        },
    );
    match result {
        Err(BlameError::TooLarge { cap }) => assert_eq!(cap, 1000),
        other => panic!("a capped blame must not read as whole: {other:?}"),
    }
}

const SESSION_FILE: &str = "src/auth/session.rs";

/// The session fixture's file, blamed at `HEAD` through `blame_with`.
fn session_blame(dir: &Path, marks: MarksFrom) -> Blame {
    let repo = RepoHandle::open(dir).expect("the fixture opens");
    blame_with(
        &repo,
        &Rev::Commit(head(dir)),
        Path::new(SESSION_FILE),
        &BlameOptions::default(),
        &(),
        &Knobs {
            cap: DEFAULT_CAP,
            marks,
        },
    )
    .unwrap_or_else(|why| panic!("blames: {why}"))
}

fn head(dir: &Path) -> Oid {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "rev-parse HEAD");
    id(String::from_utf8_lossy(&output.stdout).trim())
}

fn passes() -> usize {
    MARKS_PASSES.with(std::cell::Cell::get)
}

fn marks_of(blame: &Blame) -> Vec<LineMark> {
    blame.lines.iter().map(|line| line.mark).collect()
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
