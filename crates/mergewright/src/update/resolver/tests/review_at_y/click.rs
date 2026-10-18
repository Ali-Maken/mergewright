//! A click on the list's `✓ Review the merge  y` (ease-pass/11) does what
//! `y` does: before, the click reached `leave::finish_pressed` through the
//! notice row's finish words, and skipped the review's door.

use mergewright_core::event::KeyCode;
use mergewright_ui::conflict_resolver::review_row_at;

use super::super::file_list::click;
use super::super::press;
use super::{on, read_lands, reads_the_review, reviewed, solved};
use crate::update::AppState;
use crate::update::finish::ReturnTo;

/// The screen cell of the list's review row, or a panic when it is not
/// drawn.
fn review_cell(state: &AppState) -> (u16, u16) {
    let (files, placed, split) =
        crate::view::resolver::resolver_split(state, super::super::super::banner_rows(state));
    for dy in 0..split.list.height {
        for dx in 0..split.list.width {
            if review_row_at(&files, placed, split.list.width, split.list.height, dx, dy) {
                return (split.list.x + dx, split.list.y + dy);
            }
        }
    }
    panic!("no review row in the list: {:?}", files.tail);
}

#[test]
fn a_click_on_the_review_row_is_y() {
    let start = on("a", solved(), &["a"], &["a"]);
    let (by_key, key_commands) = press(&start, KeyCode::Char('y'));
    let (column, row) = review_cell(&start);
    let (by_click, commands) = click(&start, column, row);
    // Through the door: the read is asked, nothing drawn until it lands.
    assert!(reads_the_review(&key_commands).is_some());
    assert!(reads_the_review(&commands).is_some(), "{commands:?}");
    assert!(by_click.finish.is_none(), "drawn before the read landed");
    let by_key = read_lands(&by_key, &key_commands, Ok(reviewed()));
    let by_click = read_lands(&by_click, &commands, Ok(reviewed()));
    let finish = by_click.finish.as_ref().expect("the review is open");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    let key_finish = by_key.finish.as_ref().expect("`y` opened it too");
    assert_eq!(
        (finish.return_to, finish.cursor),
        (key_finish.return_to, key_finish.cursor)
    );
    assert!(by_click.resolver.is_some(), "the resolver stays under it");
}
