//! The cursor stays on a placeholder while every group is one.
//! Moved out of `tests.rs` to keep that file under its size row.

use super::*;

#[test]
fn cursor_on_placeholder_stays_while_every_group_is_a_placeholder() {
    let mut nothing_read = with_a_tag();
    nothing_read.refs = None;
    let (open, _) = press(&nothing_read, KeyPress::control(KeyCode::Char('p')));
    let (typing, asked) = typed(&open, "v2");
    assert!(names_asked(&asked).is_none(), "nothing read to match yet");
    let typing = cursor_on(typing, "FILES", "reading…");
    let commits = typing
        .palette
        .as_ref()
        .and_then(|p| p.find.commits.pending.as_ref())
        .map(|(id, _)| *id)
        .expect("a commits job in flight");
    let (landed, _) = update(
        &typing,
        AppEvent::Produced(commits, Product::PaletteCommits(Ok(Arc::from([])))),
    );
    assert_eq!(
        under_cursor(&landed),
        ("FILES".to_owned(), "reading…".to_owned()),
        "{:?}",
        shown(&landed)
    );
}
