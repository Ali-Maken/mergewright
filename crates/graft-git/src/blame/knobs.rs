//! Everything a test may set on [`super::blame_with`], in one place. The
//! knobs replace the blame's **inputs**, never its decisions: [`super::blame`]
//! passes [`Knobs::real`], and no branch anywhere asks "is this a test".

use crate::run::DEFAULT_CAP;

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

/// What [`super::blame`] fixes and a test may change.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Knobs {
    /// The runner's output cap for each blame spawn.
    pub cap: usize,
    /// Where the marks come from.
    pub marks: MarksFrom,
}

impl Knobs {
    /// What [`super::blame`] passes: the runner's default cap, marks read
    /// the way the system git needs.
    pub(crate) fn real() -> Knobs {
        Knobs {
            cap: DEFAULT_CAP,
            marks: MarksFrom::Auto,
        }
    }
}
