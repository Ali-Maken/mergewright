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
use crate::run::DEFAULT_CAP;

pub(super) const PORCELAIN_2_54: &[u8] = include_bytes!("../testdata/git-2.54/blame-porcelain.txt");
pub(super) const PORCELAIN_2_38: &[u8] = include_bytes!("../testdata/git-2.38/blame-porcelain.txt");
pub(super) const HUMAN_2_54: &[u8] = include_bytes!("../testdata/git-2.54/blame-human.txt");
pub(super) const HUMAN_2_38: &[u8] = include_bytes!("../testdata/git-2.38/blame-human.txt");
pub(super) const EDGES_2_38: &[u8] =
    include_bytes!("../testdata/git-2.38/blame-edges-porcelain.txt");
pub(super) const EDGES_2_54: &[u8] =
    include_bytes!("../testdata/git-2.54/blame-edges-porcelain.txt");
pub(super) const EDGES_HUMAN_2_38: &[u8] =
    include_bytes!("../testdata/git-2.38/blame-edges-human.txt");
pub(super) const EDGES_HUMAN_2_54: &[u8] =
    include_bytes!("../testdata/git-2.54/blame-edges-human.txt");

/// The session fixture's lines git skipped under its list: rustfmt's.
pub(super) const SKIPPED: [u32; 6] = [31, 32, 33, 35, 36, 37];

pub(super) fn id(hex: &str) -> Oid {
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
            ..Knobs::real()
        },
    );
    match result {
        Err(BlameError::TooLarge { cap }) => assert_eq!(cap, 1000),
        other => panic!("a capped blame must not read as whole: {other:?}"),
    }
}

pub(super) const SESSION_FILE: &str = "src/auth/session.rs";

/// The session fixture's file, blamed at `HEAD` through `blame_with`.
pub(super) fn session_blame(dir: &Path, marks: MarksFrom) -> Blame {
    session_blame_with(
        dir,
        &Knobs {
            marks,
            ..Knobs::real()
        },
    )
    .unwrap_or_else(|why| panic!("blames: {why}"))
}

/// [`session_blame`] with every knob the caller's, the result unwrapped by
/// the caller.
pub(super) fn session_blame_with(dir: &Path, knobs: &Knobs) -> Result<Blame, BlameError> {
    let repo = RepoHandle::open(dir).expect("the fixture opens");
    blame_with(
        &repo,
        &Rev::Commit(head(dir)),
        Path::new(SESSION_FILE),
        &BlameOptions::default(),
        &(),
        knobs,
    )
}

pub(super) fn head(dir: &Path) -> Oid {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "rev-parse HEAD");
    id(String::from_utf8_lossy(&output.stdout).trim())
}

pub(super) fn marks_of(blame: &Blame) -> Vec<LineMark> {
    blame.lines.iter().map(|line| line.mark).collect()
}
