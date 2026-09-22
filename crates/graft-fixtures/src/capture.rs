//! Captures under `docs/shots/`: **asserted on every run, rewritten only on
//! request.**
//!
//! A capture is a text (or image) record of what the code draws, and
//! CLAUDE.md rule 6 asks the user to judge it by eye. That only works if the
//! file still matches the code. For four milestones it did not: 35 of the 37
//! generators were `#[ignore]`d — "run by hand to regenerate" — so a full
//! green run said nothing about any capture, and 42 of them drifted without
//! one test going red (m5-history-and-ghost-preview/40).
//!
//! The fix is to swap which direction is the default. A generator calls
//! [`capture`] instead of `std::fs::write`, and that call **compares** the
//! committed file to what the code drew and fails naming the file, the line
//! and the column. Regenerating is an explicit act:
//!
//! ```text
//! GRAFT_UPDATE_SHOTS=1 cargo nextest run
//! git diff --stat docs/shots/
//! ```
//!
//! **A regeneration is always a red run.** Drift is an error whether or not
//! the file was rewritten, so a `GRAFT_UPDATE_SHOTS` somebody left exported
//! cannot turn a real regression green, and a rewritten capture is always
//! something `git status` has to show (M5 groups 12/13 review, m1). The
//! contract, the variable and this rule came from `crates/graft/tests/it`,
//! where they covered two captures; this module is the same thing where all
//! three crates that draw one can reach it.
//!
//! There is deliberately **no tolerance**. A capture allowed to differ from
//! the code is exactly the thing this module exists to delete, so the
//! comparison is byte-for-byte and any difference is a failure. A generator
//! whose output is not reproducible is therefore a defect in the generator —
//! fix the source of the variance (a clock, a temp path, a PID) rather than
//! loosening the check.

use std::ffi::OsStr;
use std::path::Path;

/// The environment variable that lets a run rewrite a capture. Named in
/// `TESTING.md`.
pub const UPDATE_VAR: &str = "GRAFT_UPDATE_SHOTS";

/// May this run rewrite a committed capture? Only the literal `1`.
///
/// `GRAFT_UPDATE_SHOTS=0` or `=false` left exported in a shell used to mean
/// "yes" (`var_os(..).is_some()`), which is the shape of an accident, not of
/// an instruction (M5 groups 12/13 review, m1).
#[must_use]
pub fn updating_shots(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

/// [`updating_shots`] against this process's environment.
#[must_use]
pub fn updating() -> bool {
    updating_shots(std::env::var_os(UPDATE_VAR).as_deref())
}

/// One text capture against what is committed at `path`, rewritten first when
/// this run is a regeneration.
///
/// `Err` carries a message naming the file, the first line that differs and
/// the command that regenerates it — and it is an `Err` **whether or not**
/// the file was just rewritten. A caller drawing several captures collects
/// these and reports them together, so one run names every file that drifted
/// rather than only the first.
///
/// # Errors
///
/// If the committed file differs from `drawn`, is missing, cannot be read, or
/// — when regenerating — cannot be written.
pub fn check(path: &Path, drawn: &str) -> Result<(), String> {
    check_text(path, drawn, updating())
}

/// [`check`] for a capture that is not text — a PNG, an escape stream.
///
/// # Errors
///
/// On the same terms as [`check`]; the message names the first differing byte
/// instead of the first differing line.
pub fn check_bytes(path: &Path, drawn: &[u8]) -> Result<(), String> {
    check_raw(path, drawn, updating())
}

/// Every capture one generator writes, so that one regeneration pass
/// rewrites all of them.
///
/// [`capture`] panics where it stands, which is right for a generator that
/// writes one file and wrong for one that writes several: the first drifted
/// file would end the test and its neighbours would never be reached, so
/// regenerating would take as many passes as there are captures. Collecting
/// keeps the run red — the point of [`check`]'s contract — while still
/// reaching every file, and reports every drifted one at once instead of
/// hiding all but the first.
#[derive(Debug, Default)]
pub struct Captures {
    drifted: Vec<String>,
}

impl Captures {
    /// An empty collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One text capture, its complaint kept for [`Captures::finish`].
    pub fn check(&mut self, path: impl AsRef<Path>, drawn: impl AsRef<str>) {
        if let Err(why) = check(path.as_ref(), drawn.as_ref()) {
            self.drifted.push(why);
        }
    }

    /// Fails if any capture drifted, naming every one of them.
    ///
    /// # Panics
    ///
    /// If any [`Captures::check`] since [`Captures::new`] reported an error.
    pub fn finish(self) {
        assert!(
            self.drifted.is_empty(),
            "{}",
            self.drifted.join("\n────────\n")
        );
    }
}

/// [`check`], panicking on drift: what a generator that writes **one** file
/// calls. A generator that writes several uses [`Captures`] instead, so one
/// regeneration pass rewrites all of them.
///
/// # Panics
///
/// On everything [`check`] reports as an error.
pub fn capture(path: impl AsRef<Path>, drawn: impl AsRef<str>) {
    if let Err(why) = check(path.as_ref(), drawn.as_ref()) {
        panic!("{why}");
    }
}

/// [`check_bytes`], panicking on drift.
///
/// # Panics
///
/// On everything [`check_bytes`] reports as an error.
pub fn capture_bytes(path: impl AsRef<Path>, drawn: impl AsRef<[u8]>) {
    if let Err(why) = check_bytes(path.as_ref(), drawn.as_ref()) {
        panic!("{why}");
    }
}

/// [`check`] with the decision made by the caller, so this module's own tests
/// assert both paths without touching the environment — `cargo test` runs a
/// binary's tests in one process, so a test that set the variable would
/// change what its neighbours saw.
fn check_text(path: &Path, drawn: &str, updating: bool) -> Result<(), String> {
    let committed = std::fs::read(path);
    rewrite(path, drawn.as_bytes(), updating)?;
    let committed = committed.map_err(|why| missing(path, &why.to_string()))?;
    let committed = String::from_utf8(committed)
        .map_err(|_| report(path, "the committed capture is not UTF-8", updating))?;
    match text_difference(&committed, drawn) {
        None => Ok(()),
        Some(complaint) => Err(report(path, &complaint, updating)),
    }
}

/// [`check_bytes`] with the decision made by the caller; see [`check_text`].
fn check_raw(path: &Path, drawn: &[u8], updating: bool) -> Result<(), String> {
    let committed = std::fs::read(path);
    rewrite(path, drawn, updating)?;
    let committed = committed.map_err(|why| missing(path, &why.to_string()))?;
    match byte_difference(&committed, drawn) {
        None => Ok(()),
        Some(complaint) => Err(report(path, &complaint, updating)),
    }
}

fn rewrite(path: &Path, bytes: &[u8], updating: bool) -> Result<(), String> {
    if !updating {
        return Ok(());
    }
    std::fs::write(path, bytes)
        .map_err(|why| format!("capture {} is not writable: {why}", shown(path)))
}

fn missing(path: &Path, why: &str) -> String {
    format!(
        "capture {} could not be read: {why}.\n\n\
         If it is simply missing, create it with:\n\n    {UPDATE_VAR}=1 cargo nextest run\n",
        shown(path),
    )
}

/// The path as a reader should see it: relative to the workspace root, so the
/// message reads `docs/shots/m2-map-view.txt` and not the absolute
/// `…/crates/graft/../../docs/shots/m2-map-view.txt` the call site builds.
fn shown(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    match text.rfind("docs/shots/") {
        Some(at) => text[at..].to_string(),
        None => text,
    }
}

fn report(path: &Path, complaint: &str, updating: bool) -> String {
    let next = if updating {
        format!(
            "It has just been rewritten, and this run is red anyway so that the change cannot \
             pass unnoticed. Read `git diff {}` and commit it if it is right.",
            shown(path),
        )
    } else {
        format!(
            "A capture is the evidence the user judges by eye (CLAUDE.md rule 6), so it must \
             match the code exactly — it is never made tolerant to pass. If the drawn output \
             is the correct one, regenerate and read the diff:\n\n    \
             {UPDATE_VAR}=1 cargo nextest run\n    git diff --stat docs/shots/",
        )
    };
    format!(
        "capture {} does not match what the code draws: {complaint}\n\n{next}\n",
        shown(path),
    )
}

/// Why two capture texts differ, or `None` when they do not.
fn text_difference(committed: &str, drawn: &str) -> Option<String> {
    if committed == drawn {
        return None;
    }
    let committed_lines: Vec<&str> = committed.lines().collect();
    let drawn_lines: Vec<&str> = drawn.lines().collect();
    let mut complaint = String::new();
    for (number, (left, right)) in committed_lines.iter().zip(&drawn_lines).enumerate() {
        if left != right {
            let column = first_differing_column(left, right)
                .map(|column| format!(", column {column}"))
                .unwrap_or_default();
            complaint.push_str(&format!(
                "\n\n  line {}{column}:\n    committed  {left}\n    drawn      {right}",
                number + 1,
            ));
            break;
        }
    }
    if committed_lines.len() == drawn_lines.len() {
        if complaint.is_empty() {
            // Same lines, same count: the difference is in the line endings or
            // the trailing newline, which `lines()` throws away and a grid
            // cares about.
            complaint.push_str(&format!(
                "\n\n  the lines match; the line endings or the trailing newline do not \
                 (committed {} bytes, drawn {})",
                committed.len(),
                drawn.len(),
            ));
        }
    } else {
        complaint.push_str(&format!(
            "\n\n  committed {} lines, drawn {}",
            committed_lines.len(),
            drawn_lines.len(),
        ));
    }
    Some(complaint)
}

/// The 1-based column of the first character that differs, or `None` when one
/// line is a prefix of the other.
fn first_differing_column(left: &str, right: &str) -> Option<usize> {
    left.chars()
        .zip(right.chars())
        .position(|(a, b)| a != b)
        .map(|at| at + 1)
}

/// Why two capture byte strings differ, or `None` when they do not.
fn byte_difference(committed: &[u8], drawn: &[u8]) -> Option<String> {
    if committed == drawn {
        return None;
    }
    let mut complaint = format!(
        "\n\n  committed {} bytes, drawn {}",
        committed.len(),
        drawn.len(),
    );
    if let Some(at) = committed
        .iter()
        .zip(drawn)
        .position(|(left, right)| left != right)
    {
        complaint.push_str(&format!(
            "\n  first differing byte at offset {at}: committed {:#04x}, drawn {:#04x}",
            committed[at], drawn[at],
        ));
    }
    Some(complaint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_literal_one_may_rewrite_a_capture() {
        assert!(updating_shots(Some(OsStr::new("1"))));
        for accident in ["", "0", "false", "no", "2", "yes"] {
            assert!(
                !updating_shots(Some(OsStr::new(accident))),
                "{UPDATE_VAR}={accident:?} is not an instruction to rewrite"
            );
        }
        assert!(!updating_shots(None));
    }

    #[test]
    fn identical_text_is_no_difference() {
        assert_eq!(text_difference("a\nb\n", "a\nb\n"), None);
    }

    #[test]
    fn a_changed_line_is_named_with_its_number_and_column() {
        let complaint =
            text_difference("row one\nrow two\n", "row one\nrow TWO\n").expect("a difference");
        assert!(complaint.contains("line 2"), "{complaint}");
        assert!(complaint.contains("column 5"), "{complaint}");
        assert!(complaint.contains("committed  row two"), "{complaint}");
        assert!(complaint.contains("drawn      row TWO"), "{complaint}");
    }

    #[test]
    fn a_marker_abutting_its_subject_is_one_column_of_difference() {
        // The shape of the m5/40 regression, as the message prints it.
        let complaint = text_difference("\u{258f}\u{26a0} Merge\n", "\u{258f}\u{26a0}Merge\n")
            .expect("a difference");
        assert!(complaint.contains("column 3"), "{complaint}");
    }

    #[test]
    fn a_missing_line_is_reported_as_a_count() {
        let complaint = text_difference("a\nb\n", "a\n").expect("a difference");
        assert!(
            complaint.contains("committed 2 lines, drawn 1"),
            "{complaint}"
        );
    }

    #[test]
    fn a_trailing_newline_alone_is_still_a_difference() {
        let complaint = text_difference("a\nb\n", "a\nb").expect("a difference");
        assert!(complaint.contains("trailing newline"), "{complaint}");
    }

    #[test]
    fn identical_bytes_are_no_difference() {
        assert_eq!(byte_difference(&[1, 2, 3], &[1, 2, 3]), None);
    }

    #[test]
    fn a_changed_byte_is_named_with_its_offset() {
        let complaint = byte_difference(&[1, 2, 3], &[1, 9, 3]).expect("a difference");
        assert!(complaint.contains("offset 1"), "{complaint}");
        assert!(complaint.contains("0x02"), "{complaint}");
        assert!(complaint.contains("0x09"), "{complaint}");
    }

    #[test]
    fn a_truncated_byte_string_reports_both_lengths() {
        let complaint = byte_difference(&[1, 2, 3], &[1, 2]).expect("a difference");
        assert!(
            complaint.contains("committed 3 bytes, drawn 2"),
            "{complaint}"
        );
    }

    #[test]
    fn the_report_names_the_file_relative_to_the_workspace_and_the_command() {
        let message = report(
            Path::new("/a/b/crates/graft/../../docs/shots/m2-map-view.txt"),
            "\n\n  line 1",
            false,
        );
        assert!(
            message.contains("capture docs/shots/m2-map-view.txt"),
            "{message}"
        );
        assert!(
            message.contains("GRAFT_UPDATE_SHOTS=1 cargo nextest run"),
            "{message}"
        );
    }

    #[test]
    fn a_path_outside_docs_shots_is_shown_whole() {
        assert_eq!(shown(Path::new("/tmp/elsewhere.txt")), "/tmp/elsewhere.txt");
    }

    #[test]
    fn a_capture_that_matches_passes() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let path = temp.path().join("shot.txt");
        std::fs::write(&path, "drawn\n").expect("writable");
        assert_eq!(check_text(&path, "drawn\n", false), Ok(()));
    }

    #[test]
    fn a_normal_run_fails_on_drift_and_rewrites_nothing() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let path = temp.path().join("shot.txt");
        std::fs::write(&path, "committed\n").expect("writable");
        let why = check_text(&path, "drawn\n", false).expect_err("drift is an error");
        assert!(why.contains("does not match what the code draws"), "{why}");
        assert_eq!(
            std::fs::read_to_string(&path).expect("readable"),
            "committed\n",
            "a normal run rewrote a committed capture"
        );
    }

    #[test]
    fn a_regeneration_rewrites_the_file_and_is_still_red() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let path = temp.path().join("shot.txt");
        std::fs::write(&path, "committed\n").expect("writable");
        let why = check_text(&path, "drawn\n", true).expect_err("a regeneration is red too");
        assert!(why.contains("just been rewritten"), "{why}");
        assert_eq!(
            std::fs::read_to_string(&path).expect("readable"),
            "drawn\n",
            "a regeneration did not rewrite the capture"
        );
    }

    #[test]
    fn a_regeneration_of_a_capture_that_had_not_drifted_is_green() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let path = temp.path().join("shot.txt");
        std::fs::write(&path, "drawn\n").expect("writable");
        assert_eq!(check_text(&path, "drawn\n", true), Ok(()));
    }

    #[test]
    fn a_missing_capture_names_the_command_that_creates_it() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let why = check_text(&temp.path().join("absent.txt"), "drawn\n", false)
            .expect_err("a missing capture is an error");
        assert!(why.contains("GRAFT_UPDATE_SHOTS=1"), "{why}");
    }

    #[test]
    fn a_regeneration_creates_a_capture_that_did_not_exist_yet() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let path = temp.path().join("new.png");
        let why = check_raw(&path, &[7u8, 8], true).expect_err("a new capture is red once");
        assert!(why.contains("could not be read"), "{why}");
        assert_eq!(std::fs::read(&path).expect("readable"), vec![7u8, 8]);
    }

    #[test]
    fn bytes_that_differ_are_named_by_offset() {
        let temp = tempfile::tempdir().expect("a temp dir");
        let path = temp.path().join("shot.png");
        std::fs::write(&path, [1u8, 2, 3]).expect("writable");
        let why = check_raw(&path, &[1u8, 9, 3], false).expect_err("drift is an error");
        assert!(why.contains("offset 1"), "{why}");
    }
}
