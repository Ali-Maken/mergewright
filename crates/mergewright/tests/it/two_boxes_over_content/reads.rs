//! Reading a box back out of a drawn frame: how much content stands beside
//! it, and the widest strip of graph it left showing.

/// How much of the frame is not the box: the rows that still carry graph
/// text outside the box's own columns. Both boxes are judged by this one
/// number, in opposite directions (ADR-0040) — the confirm box must leave
/// rows standing beside it, the credential prompt must leave none.
pub(super) fn rows_with_content_beside_the_box(drawn: &[String], columns: u16) -> usize {
    drawn
        .iter()
        .map(|line| without_pane_rule(line, columns))
        .filter(|line| {
            let Some(edge) = line.find(['\u{256d}', '\u{2502}', '\u{2570}']) else {
                return false;
            };
            !line[..edge].trim().is_empty() && line.contains('\u{2502}')
        })
        .count()
}

/// The widest strip of graph the box left showing, and the row's tail on
/// the other side of it, **read back out of the frame** rather than written
/// by hand. The tail is trimmed: a run of blank cells is not content that
/// survived.
///
/// This is the whole of what the user is judging, so the caption states it —
/// and three captions in this milestone went stale because a caption stated
/// something the code no longer did. A line derived from the picture cannot.
pub(super) fn what_survives(drawn: &[String], columns: u16) -> (String, String) {
    const LEFT_EDGES: [char; 3] = ['\u{256d}', '\u{2502}', '\u{2570}'];
    const RIGHT_EDGES: [char; 3] = ['\u{256e}', '\u{2502}', '\u{256f}'];
    let mut widest = (String::new(), String::new());
    for line in drawn {
        let cells: Vec<char> = without_pane_rule(line, columns).chars().collect();
        let (Some(first), Some(last)) = (
            cells.iter().position(|cell| LEFT_EDGES.contains(cell)),
            cells.iter().rposition(|cell| RIGHT_EDGES.contains(cell)),
        ) else {
            continue;
        };
        let left: String = cells[..first].iter().collect();
        if left.trim_end().chars().count() > widest.0.chars().count() {
            let right: String = cells[last + 1..].iter().collect();
            widest = (left.trim_end().to_owned(), right.trim().to_owned());
        }
    }
    widest
}

/// `line` (a row of a frame `columns` wide) with the branch pane's rule
/// blanked (ease-pass 09): it stands on every body row, and read as a
/// box's edge it would make every row a box row. The width is the frame's,
/// not the line's: a row is trimmed at its last drawn cell, which beside an
/// empty pane is the rule itself.
pub(super) fn without_pane_rule(line: &str, columns: u16) -> String {
    let mut cells: Vec<char> = line.chars().collect();
    if let Some(width) = mergewright_ui::branch_pane::pane_width(columns) {
        let rule = usize::from(columns - width - 2);
        if cells.get(rule) == Some(&'\u{2502}') {
            cells[rule] = ' ';
        }
    }
    cells.into_iter().collect()
}
