//! The body's line arithmetic around the cursor row's second line
//! (ease-pass/07b, mock G9): the cursor row grows one line, so the body
//! shows one commit row fewer while the cursor is in view, and every line
//! below the cursor belongs to the row one up. Plain numbers, so the app's
//! scroll and hit-tests call the same code the drawing does
//! (ARCHITECTURE.md: shared layout arithmetic lives here).

/// Which drawn row a body line belongs to; the line under the cursor row is
/// its second line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyLine {
    /// The row drawn at this index (into the rows drawn, from the body's top).
    Row(u16),
    /// The second line of the cursor row at this index.
    SecondOf(u16),
}

impl BodyLine {
    /// The drawn row this line belongs to, second line or not.
    #[must_use]
    pub fn row(self) -> u16 {
        match self {
            BodyLine::Row(row) | BodyLine::SecondOf(row) => row,
        }
    }
}

/// The row body line `line` belongs to. `cursor`: the cursor row's index
/// among the rows drawn, if it is in view.
#[must_use]
pub fn body_line(line: u16, cursor: Option<u16>) -> BodyLine {
    match cursor {
        Some(cursor) if line == cursor.saturating_add(1) => BodyLine::SecondOf(cursor),
        Some(cursor) if line > cursor => BodyLine::Row(line - 1),
        _ => BodyLine::Row(line),
    }
}

/// Rows the body shows commits on: one fewer while the cursor is in view
/// (never below 1 while there is a line at all).
#[must_use]
pub fn commit_rows(body_rows: u16, cursor_in_view: bool) -> u16 {
    if cursor_in_view && body_rows > 1 {
        body_rows - 1
    } else {
        body_rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_no_cursor_in_view_every_line_is_its_own_row() {
        for line in 0..5 {
            assert_eq!(body_line(line, None), BodyLine::Row(line));
        }
        assert_eq!(commit_rows(10, false), 10);
    }

    #[test]
    fn the_cursor_at_the_top_pushes_every_row_below_it_down_one() {
        assert_eq!(body_line(0, Some(0)), BodyLine::Row(0));
        assert_eq!(body_line(1, Some(0)), BodyLine::SecondOf(0));
        assert_eq!(body_line(2, Some(0)), BodyLine::Row(1));
        assert_eq!(body_line(9, Some(0)), BodyLine::Row(8));
        assert_eq!(body_line(1, Some(0)).row(), 0);
    }

    #[test]
    fn the_cursor_on_the_last_row_takes_the_last_line_for_its_second() {
        // Ten body lines, nine commit rows: the cursor on row 8 draws its
        // second line on line 9, the body's last.
        assert_eq!(commit_rows(10, true), 9);
        assert_eq!(body_line(7, Some(8)), BodyLine::Row(7));
        assert_eq!(body_line(8, Some(8)), BodyLine::Row(8));
        assert_eq!(body_line(9, Some(8)), BodyLine::SecondOf(8));
    }

    #[test]
    fn commit_rows_never_drops_below_one() {
        assert_eq!(commit_rows(1, true), 1);
        assert_eq!(commit_rows(2, true), 1);
        assert_eq!(commit_rows(0, true), 0);
    }
}
