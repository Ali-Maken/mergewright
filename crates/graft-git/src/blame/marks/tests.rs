//! `parse_marks` on git's own human-format bytes (samples in
//! `blame/tests.rs`).

use super::*;
use crate::blame::parse_porcelain;
use crate::blame::tests::{
    EDGES_2_54, EDGES_HUMAN_2_38, EDGES_HUMAN_2_54, HUMAN_2_38, HUMAN_2_54, SKIPPED,
};

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
