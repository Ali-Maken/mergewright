//! What tests read off a graph row since ease-pass 07 drew it as
//! `lanes | branch | message  labels | who | age`: no short id, no
//! `[branch]` badge. Shared so each walk reads a row one way.

use crate::pty::Screen;

/// `line` (a full-width frame row, `columns` wide) left of the branch
/// pane's slot (ease-pass 09): the graph's rows only, whose last word is
/// the age. The whole line when the width has no pane.
pub fn graph_side(line: &str, columns: u16) -> String {
    let slot = mergewright_ui::branch_pane::pane_width(columns).map_or(0, |width| width + 2);
    line.chars().take(usize::from(columns - slot)).collect()
}

/// Whether `line` is a commit row: it ends with an age (`7y`, `now`).
pub fn is_commit_row(line: &str) -> bool {
    line.split_whitespace().last().is_some_and(|age| {
        age == "now"
            || age
                .strip_suffix(['m', 'h', 'd', 'w', 'y'])
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    })
}

/// Whether `line` draws `subject` whole or cut short with `…`. Since the
/// branch pane (ease-pass 09) takes 38 columns at 120, the message column
/// gives way first to the fold tail and the push label, so a merge row's
/// subject may read `M…` there.
pub fn reads_subject(line: &str, subject: &str) -> bool {
    line.contains(subject)
        || (1..subject.len()).any(|cut| {
            subject.is_char_boundary(cut) && line.contains(&format!("{}\u{2026}", &subject[..cut]))
        })
}

/// The line of `frame` that draws `subject` and the byte where it starts:
/// the first drawing it whole, else the one drawing the longest cut of it
/// (`Merged PR 16…`). `None` when no line does, or two draw the same cut.
pub fn row_reading(frame: &[String], subject: &str) -> Option<(usize, usize)> {
    if let Some(at) = frame.iter().position(|line| line.contains(subject)) {
        return Some((at, frame[at].find(subject)?));
    }
    let cut_on = |line: &str| {
        (1..subject.len()).rev().find_map(|cut| {
            let shown = format!("{}\u{2026}", subject.get(..cut)?);
            line.find(&shown).map(|byte| (cut, byte))
        })
    };
    let best = frame
        .iter()
        .filter_map(|line| cut_on(line))
        .map(|(cut, _)| cut)
        .max()?;
    let mut on = frame.iter().enumerate().filter_map(|(at, line)| {
        cut_on(line)
            .filter(|(cut, _)| *cut == best)
            .map(|(_, byte)| (at, byte))
    });
    let first = on.next()?;
    on.next().is_none().then_some(first)
}

/// Whether `line` is the cursor row (`▸` first) and draws `subject`, whole
/// or cut ([`reads_subject`]).
pub fn cursor_on(line: &str, subject: &str) -> bool {
    line.starts_with('\u{25b8}') && reads_subject(line, subject)
}

/// Whether `line` is the cursor row on `HEAD`'s commit: [`cursor_on`] and
/// the branch column's `(you)`. A cut subject alone (`M…`) cannot tell two
/// merges apart; `(you)` can.
pub fn cursor_on_head(line: &str, subject: &str) -> bool {
    cursor_on(line, subject) && line.contains("(you)")
}

/// The one subject of `subjects` that `line` draws: the longest drawn
/// whole, else the one whose cut is longest. Panics when two read the same
/// (a cut too short to tell them apart), naming them.
pub fn subject_on(line: &str, subjects: &[String]) -> String {
    if let Some(whole) = subjects
        .iter()
        .filter(|subject| line.contains(subject.as_str()))
        .max_by_key(|subject| subject.len())
    {
        return whole.clone();
    }
    let cut = |subject: &str| {
        (1..subject.len())
            .rev()
            .find(|at| {
                subject.is_char_boundary(*at)
                    && line.contains(&format!("{}\u{2026}", &subject[..*at]))
            })
            .unwrap_or(0)
    };
    let best = subjects.iter().map(|s| cut(s)).max().unwrap_or(0);
    let read: Vec<&String> = subjects
        .iter()
        .filter(|s| best > 0 && cut(s) == best)
        .collect();
    match read.as_slice() {
        [one] => (*one).clone(),
        [] => panic!("no subject on the row {line:?}"),
        many => panic!("the row {line:?} reads as any of {many:?}"),
    }
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
/// The label may be cut (`try-it is h…`) since the branch pane narrowed the
/// rows (ease-pass 09); the branch's whole name still has to show.
pub fn tour_branched(screen: &Screen) -> bool {
    let label = "try-it is here";
    screen.has("try-it (you)")
        || screen.lines().iter().any(|line| {
            line.contains(label)
                || ("try-it".len()..label.len())
                    .any(|cut| line.contains(&format!("{}\u{2026}", &label[..cut])))
        })
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
