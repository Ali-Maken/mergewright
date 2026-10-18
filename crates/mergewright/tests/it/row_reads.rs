//! What tests read off a graph row since ease-pass 07 drew it as
//! `lanes | branch | message  labels | who | age`: no short id, no
//! `[branch]` badge. Shared so each walk reads a row one way.

use crate::pty::Screen;

/// Whether `line` is a commit row: it ends with an age (`7y`, `now`).
pub fn is_commit_row(line: &str) -> bool {
    line.split_whitespace().last().is_some_and(|age| {
        age == "now"
            || age
                .strip_suffix(['m', 'h', 'd', 'w', 'y'])
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    })
}

/// Whether `HEAD`'s row on `main` reads `subject`: the branch column's
/// `main (you)` and the subject on one line.
pub fn tip_reads(screen: &Screen, subject: &str) -> bool {
    screen
        .lines()
        .iter()
        .any(|line| line.contains("main (you)") && line.contains(subject))
}

/// [`tip_reads`] for `subject`, as a wait's condition.
pub fn on_tip(subject: &str) -> impl Fn(&Screen) -> bool + '_ {
    move |screen| tip_reads(screen, subject)
}

/// The cursor on `main`, level with origin: the remote's label is folded
/// into the local branch's, which says nothing once nothing is to push.
pub fn main_pushed(screen: &Screen) -> bool {
    screen.lines().iter().any(|line| {
        line.contains('\u{25b8}') && line.contains("main (you)") && !line.contains("to push")
    })
}

/// The tour's commit (step 2) on `main`, two to push.
pub fn tour_committed(screen: &Screen) -> bool {
    screen.has("Note the changes  \u{2191}2 to push")
}

/// The tour's `u` (step 3): `main` back on its own commit, one to push.
pub fn tour_undone(screen: &Screen) -> bool {
    screen.has("Add today's specials  \u{2191}1 to push") && !screen.has("\u{2191}2 to push")
}

/// The tour's branch (step 5) on the graph: a label, or the column's name
/// once `HEAD` is on it.
pub fn tour_branched(screen: &Screen) -> bool {
    screen.has("try-it is here") || screen.has("try-it (you)")
}

/// The switch picker's rows, by kind glyph and word: graph rows read
/// `main is here`, so the bare words would match them too.
pub const PICKER_KINDS: [&str; 4] = [
    "\u{21a9} before",
    "\u{25cf} local",
    "\u{25cc} remote",
    "\u{b7} here",
];

/// The rebase walk's target row: `m2` with `main`'s label after it.
pub fn m2_on_main(row: &&str) -> bool {
    row.contains("m2  main is here")
}
