//! `parse_marks` and `merge_marks` on git's own bytes (samples in
//! `blame/tests.rs`).

use super::*;
use crate::blame::marks_pass::merge_marks;
use crate::blame::tests::{
    EDGES_2_38, EDGES_2_54, EDGES_HUMAN_2_38, EDGES_HUMAN_2_54, HUMAN_2_38, HUMAN_2_54,
    PORCELAIN_2_38, SKIPPED,
};
use crate::blame::{BlameError, parse_porcelain};

/// The marks alone, in order.
fn marks_only(lines: &[MarkedLine]) -> Vec<LineMark> {
    lines.iter().map(|line| line.mark).collect()
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
    assert_eq!(marks_only(&human), marks, "both readers agree");
    let human = parse_marks(EDGES_HUMAN_2_38).expect("the human format parses");
    assert_eq!(marks_only(&human), marks, "2.38.5's human format too");
}

#[test]
fn the_human_samples_mark_the_skipped_lines_on_both_gits() {
    for (git, sample) in [("2.38.5", HUMAN_2_38), ("2.54", HUMAN_2_54)] {
        let marks = marks_only(&parse_marks(sample).expect("the human format parses"));
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
    let parsed = parse_marks(bytes.as_bytes()).expect("parses");
    assert_eq!(
        marks_only(&parsed),
        [Skipped, Unblamable, Unblamable, Plain, Plain],
        "a last line with no newline still counts"
    );
    let prefixes: Vec<&str> = parsed.iter().map(|line| line.id_prefix.as_str()).collect();
    let (id00, id0) = (format!("{id}00"), format!("{id}0"));
    assert_eq!(
        prefixes,
        [id, id, id, id00.as_str(), id0.as_str()],
        "the hex after the marks, and only it"
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
    longer.push(MarkedLine {
        mark: LineMark::Plain,
        id_prefix: "0".repeat(40),
    });
    match merge_marks(&mut lines, longer) {
        Err(BlameError::Changed { first, second }) => assert_eq!((first, second), (60, 61)),
        other => panic!("a count mismatch is Changed: {other:?}"),
    }
    assert!(
        lines.iter().all(|line| line.mark == LineMark::Plain),
        "nothing merged on a mismatch"
    );
}

/// Review 18e-20f Minor 1: the same line count is not the same lines. One
/// line's id prefix swapped for another commit's: `Changed`, counts equal,
/// nothing merged. Red with the prefix check in `merge_marks` removed.
#[test]
fn merge_marks_refuses_a_line_whose_commit_differs() {
    let (mut lines, _) = parse_porcelain(PORCELAIN_2_38).expect("parses");
    let mut marks = parse_marks(HUMAN_2_38).expect("parses");
    let other = (1..lines.len())
        .find(|&index| lines[index].commit != lines[0].commit)
        .expect("the file has more than one commit");
    let first = marks[0].id_prefix.clone();
    marks[other].id_prefix = first;
    match merge_marks(&mut lines, marks) {
        Err(BlameError::Changed { first, second }) => assert_eq!((first, second), (60, 60)),
        other => panic!("a line from another commit is Changed: {other:?}"),
    }
    assert!(
        lines.iter().all(|line| line.mark == LineMark::Plain),
        "nothing merged on a mismatch"
    );
}

/// git prints the all-zero id, unshortened, for a line not committed yet
/// (both edges samples, line 6); the porcelain reads it as `commit: None`.
#[test]
fn merge_marks_matches_zero_ids_to_uncommitted_lines() {
    for (git, porcelain, human) in [
        ("2.38.5", EDGES_2_38, EDGES_HUMAN_2_38),
        ("2.54", EDGES_2_54, EDGES_HUMAN_2_54),
    ] {
        let (mut lines, _) = parse_porcelain(porcelain).expect("parses");
        let marks = parse_marks(human).expect("parses");
        assert_eq!(lines[5].commit, None, "{git}");
        assert_eq!(marks[5].id_prefix, "0".repeat(40), "{git}");
        merge_marks(&mut lines, marks.clone()).unwrap_or_else(|why| panic!("{git}: {why}"));

        let (mut lines, _) = parse_porcelain(porcelain).expect("parses");
        let mut zero_on_committed = marks;
        zero_on_committed[0].id_prefix = "0".repeat(39);
        assert!(
            matches!(
                merge_marks(&mut lines, zero_on_committed),
                Err(BlameError::Changed { .. })
            ),
            "{git}: a zero id on a committed line"
        );
    }
}

/// A `^` shortens the id by one (line 1: 39 hex digits), `^?` by two: still
/// a prefix of the porcelain's full id.
#[test]
fn merge_marks_accepts_a_boundary_prefix() {
    let (mut lines, _) = parse_porcelain(EDGES_2_38).expect("parses");
    let marks = parse_marks(EDGES_HUMAN_2_38).expect("parses");
    assert_eq!(marks[0].id_prefix.len(), 39, "`^` takes one digit");
    assert_eq!(marks[2].id_prefix.len(), 38, "`^?` takes two");
    merge_marks(&mut lines, marks).expect("boundary prefixes match");
    assert_eq!(lines[2].mark, LineMark::Skipped);
}
