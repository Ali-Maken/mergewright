//! A cursor on a placeholder stays while every group is one, and moves
//! to a row that lands in another group. Moved out of `tests.rs` to
//! keep that file under its size row.

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

#[test]
fn cursor_on_placeholder_moves_to_a_landed_row_in_another_group() {
    let (open, commands) = press(&with_a_tag(), KeyPress::control(KeyCode::Char('p')));
    let read = commands
        .iter()
        .find_map(|c| match c {
            Command::ReadPaletteBranches(id) => Some(*id),
            _ => None,
        })
        .expect("a branch read");
    let (typing, asked) = typed(&open, "v2");
    let asked = names_asked(&asked).expect("tags asked");
    let typing = cursor_on(typing, "BRANCHES", "reading…");
    // TAGS answers while BRANCHES still reads.
    let tags = answer(&typing, &asked);
    assert_eq!(
        under_cursor(&tags),
        ("TAGS".to_owned(), "v2.1.0".to_owned()),
        "{:?}",
        shown(&tags)
    );
    // BRANCHES answers above it: the cursor keeps the tag.
    let branches = SwitchRefs {
        refs: vec![RefListing {
            name: name("refs/heads/v2-fix"),
            tip: oid(3),
            subject: String::new(),
            author: String::new(),
            when: 10,
            upstream: None,
        }],
        previous: None,
    };
    let (landed, commands) = update(
        &tags,
        AppEvent::Produced(read, Product::PaletteBranches(Ok(Arc::new(branches)))),
    );
    let landed = answer(&landed, &names_asked(&commands).expect("re-asked"));
    assert!(
        shown(&landed)
            .iter()
            .any(|(h, l, _)| h == "BRANCHES" && l == "v2-fix"),
        "{:?}",
        shown(&landed)
    );
    assert_eq!(
        under_cursor(&landed),
        ("TAGS".to_owned(), "v2.1.0".to_owned())
    );
}
