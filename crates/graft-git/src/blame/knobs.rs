//! Everything a test may set on [`super::blame_with`], in one place. The
//! knobs replace the blame's **inputs**, never its decisions: [`super::blame`]
//! passes [`Knobs::real`], and no branch anywhere asks "is this a test".
//!
//! `git` and `porcelain_marks` are the Auto path's two git-dependent
//! inputs (18f): with `Given(GitVersion::MINIMUM)` and `Dropped`, the
//! whole git < 2.50 branch (list read, version gate, second spawn, merge)
//! runs on whatever git is on `PATH`.

use crate::run::DEFAULT_CAP;
use crate::version::GitVersion;

/// Where [`super::blame_with`] takes each line's mark from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarksFrom {
    /// The porcelain, plus the human-format pass when
    /// [`super::marks_pass::needs_marks_pass`] says so.
    Auto,
    /// Always the human-format pass: the oracle test on a git whose
    /// porcelain has marks of its own.
    #[cfg_attr(not(test), allow(dead_code))]
    HumanFormat,
}

/// Which git version the second-pass decision (condition 4) sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GitFrom {
    /// The system git's, read once per process.
    System,
    /// This one, whatever git actually runs.
    #[cfg_attr(not(test), allow(dead_code))]
    Given(GitVersion),
}

/// How the porcelain's marks are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PorcelainMarks {
    /// As git wrote them.
    Read,
    /// As git before 2.50 leaves them: every line `Plain`. On such a git
    /// this changes nothing (18e's `the_2_38_porcelain_leaves_the_skipped_lines_plain`).
    #[cfg_attr(not(test), allow(dead_code))]
    Dropped,
}

/// What [`super::blame`] fixes and a test may change.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Knobs {
    /// The runner's output cap for the porcelain pass.
    pub cap: usize,
    /// The runner's output cap for the second, human-format pass.
    pub marks_cap: usize,
    /// Where the marks come from.
    pub marks: MarksFrom,
    /// The version the second-pass decision sees.
    pub git: GitFrom,
    /// How the porcelain's marks are read.
    pub porcelain_marks: PorcelainMarks,
}

impl Knobs {
    /// What [`super::blame`] passes: the runner's default cap on both
    /// passes, marks read the way the system git needs.
    pub(crate) fn real() -> Knobs {
        Knobs {
            cap: DEFAULT_CAP,
            marks_cap: DEFAULT_CAP,
            marks: MarksFrom::Auto,
            git: GitFrom::System,
            porcelain_marks: PorcelainMarks::Read,
        }
    }
}
