//! The three panes on a real stop (m6-conflicts/14) — the captures the
//! user judges by eye, at 180, 187 and 200 columns.

use super::*;

/// A merge of `feature` into `main` stopped on two files: `gone.txt`
/// (changed on `main`, deleted on `feature`) and `notes.txt`, two blocks
/// — line 2 one line a side, line 7 one line on `main` and two on
/// `feature` — with `eight` after the second, so the result pane has a
/// plain line to number after an open block.
fn notes_merge() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let write = |name: &str, text: &str| std::fs::write(repo.join(name), text).expect("writable");
    write(
        "notes.txt",
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n",
    );
    write("gone.txt", "the plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "notes and a plan"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    write(
        "notes.txt",
        "one\nTWO from feature\nthree\nfour\nfive\nsix\nSEVEN from feature\nMORE from feature\neight\n",
    );
    git(repo, &["rm", "--quiet", "gone.txt"]);
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "shout, drop the plan"]);
    git(repo, &["checkout", "-q", "main"]);
    write(
        "notes.txt",
        "one\ntwo from main\nthree\nfour\nfive\nsix\nseven from main\neight\n",
    );
    write("gone.txt", "our plan\n");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "annotate, keep the plan"]);
    git_stops(repo, &["merge", "feature"]);
    scratch
}

/// The row holding `needle`, or a panic showing the frame.
fn row_with<'a>(frame: &'a [String], needle: &str) -> &'a str {
    frame
        .iter()
        .find(|row| row.contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?}:\n{}", frame.join("\n")))
}

/// The resolver on `notes.txt` after `t` on its first block: block 1 took
/// theirs, the cursor on block 2, still open.
#[test]
fn three_panes_capture() {
    let scratch = notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "notes.txt");
    let state = app.key(&state, KeyCode::Char('t'));
    let state = settled(&app, state);
    // The take moved the cursor on to the next open block at the keypress
    // (m6-conflicts/15), as the mockup draws it. An `n` here would now
    // cross to `gone.txt`: block 1 is the last.
    assert_eq!(open(&state).cursor_block, Some(1), "{:?}", state.status);
    for columns in [200u16, 187, 180] {
        let frame = crate::captures::frame(&state, columns, 50);
        let body = frame.join("\n");
        let titles = row_with(&frame, "│ result ");
        assert!(
            titles.contains("│ yours · main ") && titles.contains("│ theirs · feature"),
            "{body}"
        );
        let block1 = row_with(&frame, "2 two from main");
        assert_eq!(block1.matches("2 TWO from feature").count(), 2, "{body}");
        assert!(
            row_with(&frame, "── 2 of 2 · open").contains("── o take"),
            "{body}"
        );
        assert!(body.contains("? open · o t b"), "{body}");
        // After the open block: the result's 6 before it + its one yours
        // line + 1 = 8, as yours; theirs, a line longer there, says 9.
        let after = row_with(&frame, "8 eight");
        assert_eq!(after.matches("│ 8 eight").count(), 2, "{body}");
        assert!(after.contains("│ 9 eight"), "{body}");
        assert!(body.contains("▸ notes.txt"), "{body}");
        if columns == 180 {
            // list column + `│` + three panes + two `│` = 180, and the
            // narrowest pane at least 48 wide (gutter included).
            let bars: Vec<usize> = titles
                .chars()
                .enumerate()
                .filter_map(|(x, c)| (c == '│').then_some(x))
                .collect();
            assert_eq!(bars.len(), 3, "{titles}");
            let panes = [
                bars[1] - bars[0] - 1,
                bars[2] - bars[1] - 1,
                180 - bars[2] - 1,
            ];
            assert_eq!(
                bars[0] + 1 + panes.iter().sum::<usize>() + 2,
                180,
                "{panes:?}"
            );
            assert!(panes.iter().all(|w| *w >= 48), "{panes:?}");
        }
        crate::captures::check_shot(
            &format!("m6/fix-three-panes-{columns}x50.txt"),
            &format!("{body}\n"),
        );
    }
    drop(state);
    app.close();
    drop(scratch);
}
