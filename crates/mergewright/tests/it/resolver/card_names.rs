//! One name per side and per-side line numbers, on a real stopped merge
//! (m6-conflicts/10): the capture the user judges by eye.

use super::*;

/// `notes.txt` stopped in a merge of `feature` into `main`: two one-line
/// blocks (lines 2 and 7), four unchanged lines apart so git keeps them
/// apart.
fn notes_merge() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = mergewright_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let write = |text: &str| std::fs::write(repo.join("notes.txt"), text).expect("writable");
    write("one\ntwo\nthree\nfour\nfive\nsix\nseven\n");
    git(repo, &["add", "notes.txt"]);
    git_as_graft(repo, &["commit", "-m", "notes"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    write("one\nTWO from feature\nthree\nfour\nfive\nsix\nSEVEN from feature\n");
    git_as_graft(repo, &["commit", "-am", "shout"]);
    git(repo, &["checkout", "-q", "main"]);
    write("one\ntwo from main\nthree\nfour\nfive\nsix\nseven from main\n");
    git_as_graft(repo, &["commit", "-am", "annotate"]);
    git_stops(repo, &["merge", "feature"]);
    scratch
}

/// ADR-0043 §4/§5 on screen: `yours · main │ theirs · feature`, each side
/// numbered by its own lines, no number on a header, role words on the bar.
#[test]
fn card_names_and_numbers_capture() {
    let scratch = notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "notes.txt");
    for (columns, rows) in [(120u16, 40u16), (80, 24)] {
        let frame = crate::captures::frame(&state, columns, rows);
        let body = frame.join("\n");
        if columns == 120 {
            assert!(
                frame
                    .iter()
                    .any(|row| row.contains("yours · main") && row.contains("│theirs · feature")),
                "{body}"
            );
            // The open clash's lines carry no numbers (ease-pass/11).
            assert!(
                frame.iter().any(|row| row.contains("two from main")
                    && row.contains("TWO from feature")
                    && !row.contains("2 two")),
                "{body}"
            );
        }
        assert!(frame.iter().any(|row| row.contains("3 three")), "{body}");
        assert!(
            frame
                .last()
                .is_some_and(|bar| bar.contains("o take yours") && bar.contains("t take theirs")),
            "{body}"
        );
        crate::captures::check_shot(
            &format!("m6/fix-card-names-numbers-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
    drop(state);
    app.close();
    drop(scratch);
}
