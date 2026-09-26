//! `parse_porcelain` on git's own bytes. The 2.54 sample is `git blame
//! --porcelain -w -M --ignore-revs-file .git-blame-ignore-revs` on the
//! `session` fixture, captured by git 2.54.0; the 2.38 sample is written by
//! hand to the porcelain format 2.38 documents (no 2.38 binary on the build
//! machine), and covers what the 2.54 capture does not: `unblamable`, a
//! quoted path, an uncommitted line.

use super::*;

const PORCELAIN_2_54: &[u8] = include_bytes!("../testdata/git-2.54/blame-porcelain.txt");

const PORCELAIN_2_38: &[u8] = b"\
1111111111111111111111111111111111111111 1 1 2
author Ana
author-mail <ana@example.invalid>
author-time 1541721600
author-tz +0000
committer Ana
committer-mail <ana@example.invalid>
committer-time 1541721600
committer-tz +0000
summary Add \xc3\xa9 file
boundary
filename \"src/\\303\\251.rs\"
\tfn one() {}
1111111111111111111111111111111111111111 2 2
\tfn two() {}
2222222222222222222222222222222222222222 5 3 1
author Tom
author-mail <tom@example.invalid>
author-time 1552089600
author-tz +0000
committer Tom
committer-mail <tom@example.invalid>
committer-time 1552089600
committer-tz +0000
summary Reformat
previous 1111111111111111111111111111111111111111 \"src/\\303\\251.rs\"
filename \"src/\\303\\251.rs\"
unblamable
\t    }
0000000000000000000000000000000000000000 4 4 1
author Not Committed Yet
author-mail <not.committed.yet>
author-time 1790417071
author-tz +0500
committer Not Committed Yet
committer-mail <not.committed.yet>
committer-time 1790417071
committer-tz +0500
summary Version of src/x.rs from src/x.rs
previous 2222222222222222222222222222222222222222 src/x.rs
filename src/x.rs
\tlet fresh = 1;
";

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

#[test]
fn the_2_38_sample_reads_unblamable_quoted_paths_and_uncommitted_lines() {
    let (lines, commits) = parse_porcelain(PORCELAIN_2_38).expect("the 2.38-shaped sample parses");
    assert_eq!(lines.len(), 4);
    assert_eq!(
        lines[1].commit,
        Some(id("1111111111111111111111111111111111111111"))
    );
    assert_eq!((lines[1].orig_line, lines[1].final_line), (2, 2));
    assert_eq!(lines[2].mark, LineMark::Unblamable);
    assert_eq!(
        lines[3].commit, None,
        "the all-zero id is not committed yet"
    );
    assert_eq!(lines[3].text, b"let fresh = 1;");
    assert_eq!(
        commits.len(),
        2,
        "the uncommitted pseudo-commit is not a commit"
    );
    let tom = &commits[&id("2222222222222222222222222222222222222222")];
    let (_, old_path) = tom.previous.clone().expect("previous");
    assert_eq!(
        path_bytes(&old_path),
        "src/\u{e9}.rs".as_bytes(),
        "C-quoting undone"
    );
    assert_eq!(
        commits[&id("1111111111111111111111111111111111111111")].summary,
        "Add \u{e9} file"
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
