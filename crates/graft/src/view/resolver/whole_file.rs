//! The resolver's whole-file and done-file cards, and the words they
//! use (m6-conflicts/05, /25).

use super::finish_row;
use graft_git::status::UnmergedHow;
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::conflict_resolver::{DocView, RegionView, SideNames};

use graft_ui::keymap::{ActionId, Keymap};

use super::{Identity, file_list_view, how_words, resolver_title, start_over_words};
use crate::update::AppState;
use crate::update::resolver::{
    DoneShape, ResolverState, WholeFileShape, side_names, start_over_refusal, whole_file_keys,
};

/// A whole-file shape's own card (m6-conflicts/05): one region, no blocks,
/// no cursor to draw a gutter for.
pub(super) fn whole_file_view(
    state: &AppState,
    open: &ResolverState,
    whole: &WholeFileShape,
    _columns: u16,
) -> DocView {
    let names = side_names(state);
    DocView {
        // A card has no marker blocks to count: `0 conflicts` would read as
        // nothing left to do (m6-conflicts/07 F8). It says what happened to
        // the file on each side instead.
        title: sanitize_line(&resolver_title(
            &open.path,
            Identity::Whole(&how_words(whole.how, whole.binary, &names)),
        )),
        files: file_list_view(state),
        names: side_names(state),
        regions: vec![RegionView::WholeFile {
            how: sanitize_line(whole.how.words()),
            keys: sanitize_line(&whole_file_keys(
                whole.how,
                whole.binary,
                &side_names(state),
                &state.keymap,
            )),
            lines: whole_file_lines(whole),
            note: whole_file_note(whole, &state.keymap),
            ours_absent: absent_on(whole.how, Side::Ours, &names),
            theirs_absent: absent_on(whole.how, Side::Theirs, &names),
        }],
        cursor_block: None,
        base_shown: false,
        notice: open
            .notice
            .as_ref()
            .map(|(text, error)| (sanitize_line(text), *error)),
        finish: finish_row(state),
    }
}

/// A done file with no text to show — binary, removed, or not readable —
/// as a card (m6-conflicts/25): what it is, and `d` to start it over where
/// `d` opens the box (m6-conflicts/28).
pub(super) fn done_card(state: &AppState, open: &ResolverState, done: &DoneShape) -> DocView {
    let unstaged = matches!(done, DoneShape::Binary { unstaged: true });
    let how = match done {
        DoneShape::Binary { .. } => "done ✓ · binary".to_owned(),
        DoneShape::Removed => "done ✓ · removed".to_owned(),
        DoneShape::Unreadable { why } => format!("done ✓ · could not read: {why}"),
        // Text is drawn as its document, never as a card.
        DoneShape::Text { .. } => "done ✓".to_owned(),
    };
    let restart = start_over_refusal(state)
        .is_none()
        .then(|| start_over_words(&state.keymap))
        .flatten();
    DocView {
        title: sanitize_line(&resolver_title(
            &open.path,
            Identity::Done {
                unstaged,
                restart: restart.as_deref(),
            },
        )),
        files: file_list_view(state),
        names: side_names(state),
        regions: vec![RegionView::WholeFile {
            how: sanitize_line(&how),
            // The card offers `d` exactly where it opens the box
            // (m6-conflicts/28); otherwise it offers nothing — `e` and
            // `esc` are on the bar.
            keys: sanitize_line(restart.as_deref().unwrap_or("")),
            lines: vec![],
            note: None,
            ours_absent: None,
            theirs_absent: None,
        }],
        cursor_block: None,
        base_shown: false,
        notice: open
            .notice
            .as_ref()
            .map(|(text, error)| (sanitize_line(text), *error)),
        finish: finish_row(state),
    }
}

/// Which side of a whole-file shape a pane shows.
#[derive(Clone, Copy)]
enum Side {
    Ours,
    Theirs,
}

/// What `side`'s pane says in place of the file when the file is not on
/// that side (three panes only): `(deleted on theirs · feature)`, or `(not
/// on …)` where the other side added it. `None` when the file is there.
fn absent_on(how: UnmergedHow, side: Side, names: &SideNames) -> Option<SanitizedText> {
    let name = match side {
        Side::Ours => names.ours.as_str(),
        Side::Theirs => names.theirs.as_str(),
    };
    let words = match (how, side) {
        (UnmergedHow::BothDeleted, _)
        | (UnmergedHow::DeletedByUs, Side::Ours)
        | (UnmergedHow::DeletedByThem, Side::Theirs) => format!("(deleted on {name})"),
        (UnmergedHow::AddedByUs, Side::Theirs) | (UnmergedHow::AddedByThem, Side::Ours) => {
            format!("(not on {name})")
        }
        _ => return None,
    };
    Some(sanitize_line(&words))
}

/// A whole-file card's plain-text tail: what is on disk, as far as the
/// card read it (`jobs::conflict::glimpse`) — never more than
/// `jobs::conflict::CARD_LINES` lines, so sanitising them per frame is
/// bounded however large the file is. None for a binary file.
fn whole_file_lines(whole: &WholeFileShape) -> Vec<SanitizedText> {
    whole.on_disk.as_ref().map_or_else(Vec::new, |on_disk| {
        on_disk
            .head
            .iter()
            .map(|line| whole_file_line(line))
            .collect()
    })
}

/// The unnumbered line under a card's text: a binary file's size in place
/// of its bytes, or how much of a long file the card left unread.
fn whole_file_note(whole: &WholeFileShape, keys: &Keymap) -> Option<SanitizedText> {
    let on_disk = whole.on_disk.as_ref()?;
    if whole.binary {
        return Some(sanitize_line(&format!(
            "binary · {}",
            size_words(on_disk.size)
        )));
    }
    on_disk.cut.then(|| {
        sanitize_line(&keys.after(
            &format!(
                "… first {} lines of {}",
                on_disk.head.len(),
                size_words(on_disk.size)
            ),
            ActionId("resolver.edit"),
            "opens the whole file",
        ))
    })
}

/// One line of a whole-file card's plain-text tail: raw bytes off disk,
/// through the same sanitiser as every other byte this widget draws.
fn whole_file_line(line: &[u8]) -> SanitizedText {
    sanitize_line(&String::from_utf8_lossy(line))
}

/// A file size the way a person reads one: `812 B`, `64.0 KB`, `4.2 MB`.
#[must_use]
pub fn size_words(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let bytes = u128::from(bytes);
    let mut divisor: u128 = 1024;
    let mut unit = 0;
    while bytes >= divisor * 1024 && unit + 1 < UNITS.len() {
        divisor *= 1024;
        unit += 1;
    }
    // Tenths of a unit, rounded, in integers: no float casts to lint.
    let tenths = (bytes * 10 + divisor / 2) / divisor;
    format!("{}.{} {}", tenths / 10, tenths % 10, UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::size_words;

    #[test]
    fn sizes_read_the_way_a_person_reads_them() {
        assert_eq!(size_words(812), "812 B");
        assert_eq!(size_words(64 * 1024), "64.0 KB");
        assert_eq!(size_words(4_404_019), "4.2 MB");
        assert_eq!(size_words(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
