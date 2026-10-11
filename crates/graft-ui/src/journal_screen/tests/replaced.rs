//! Usability Q1 (m8/46): `⇢ replaced` / `⇢ deleted` rows and the legend's
//! `⇢` clause.

use super::*;

fn moved_on(kind: RowKind, words: &str, under: usize) -> JournalRowView {
    JournalRowView {
        kind,
        ..lost(words, under)
    }
}

/// A `●` rebase with the old tip it replaced, and the shell's reset with
/// the commit it really lost.
fn replaced_rows() -> Vec<JournalRowView> {
    vec![
        row("1m ago", "rebase feature onto main", RowMark::NextUndo),
        moved_on(
            RowKind::Replaced,
            "9a8b7c6 \"Add rate limiter\" · was feature's tip · undo brings it back",
            0,
        ),
        outside(
            "5m ago",
            "reset main 1 commit back",
            "reset: moving to HEAD~1",
        ),
        lost(
            "4d5e6f7 \"Try redis cache\" · was main's tip · no branch holds it",
            2,
        ),
    ]
}

#[test]
fn replaced_row_label_and_ink() {
    assert_eq!("⇢".width(), 1, "`⇢` is one cell by `cut_at_word`'s width");
    for (kind, label) in [
        (RowKind::Replaced, "└ ⇢ replaced"),
        (RowKind::Deleted, "└ ⇢ deleted"),
    ] {
        let row = moved_on(kind, "9a8b7c6 \"Add rate limiter\" · was HEAD", 0);
        for width in [80, 187] {
            let pieces = pieces::row_pieces(
                &row,
                false,
                Columns::for_width(width),
                usize::from(width),
                Keymap::shipped(),
            );
            let lead = pieces
                .iter()
                .find(|piece| piece.text == label)
                .unwrap_or_else(|| panic!("{label} at {width}: {pieces:?}"));
            assert_eq!(lead.ink, Ink::Muted, "{label}");
            assert!(
                pieces.iter().all(|piece| piece.ink != Ink::Warning),
                "{pieces:?}"
            );
            // The words start after the label, never over it.
            let words = pieces
                .iter()
                .find(|piece| piece.text.starts_with("9a8b7c6"))
                .expect("the words");
            assert!(words.x > lead.x + label.width(), "{pieces:?}");
        }
    }
    // `✗` rows unchanged: a warning.
    let lost = pieces::row_pieces(
        &lost("x", 0),
        false,
        Columns::for_width(80),
        80,
        Keymap::shipped(),
    );
    assert!(
        lost.iter()
            .any(|piece| piece.text == "└ ✗ lost" && piece.ink == Ink::Warning)
    );

    let grid = draw(&view(replaced_rows(), JournalStatus::Ready), 187, 12);
    assert!(grid[2].starts_with("     └ ⇢ replaced"), "{grid:?}");
    assert!(grid[4].starts_with("     └ ✗ lost"), "{grid:?}");
}

/// Review m8 g1, minor 5: below 120 columns the `⇢` and `✗` rows' shas
/// start in one column, the widest label's, not each its own label's.
#[test]
fn lost_rows_shas_line_up_at_every_width() {
    for width in [80u16, 119, 120, 187] {
        let grid = draw(&view(replaced_rows(), JournalStatus::Ready), width, 12);
        let sha_at = |sha: &str| {
            let line = grid
                .iter()
                .find(|line| line.contains(sha))
                .unwrap_or_else(|| panic!("{sha} at {width}: {grid:?}"));
            let byte = line.find(sha).expect("found above");
            line[..byte].width()
        };
        assert_eq!(sha_at("9a8b7c6"), sha_at("4d5e6f7"), "{width}: {grid:?}");
    }
}

#[test]
fn legend_shows_the_replaced_clause_only_when_whole() {
    let keys = Keymap::shipped();
    let today = legend(keys);
    let whole = format!("{today} · ⇢ undo brings it back");
    let fits = whole.width();
    assert_eq!(legend_at(keys, fits), whole, "fits: the clause is last");
    assert_eq!(legend_at(keys, fits + 40), whole);
    assert_eq!(
        legend_at(keys, fits - 1),
        today,
        "one cell short: today's legend exactly"
    );
    assert_eq!(legend_at(keys, 60), cut_at_word(&today, 60), "cut as today");

    let at_187 = draw(&view(replaced_rows(), JournalStatus::Ready), 187, 12);
    assert!(at_187.contains(&whole), "{at_187:?}");
    let at_80 = draw(&view(replaced_rows(), JournalStatus::Ready), 80, 12);
    assert!(
        at_80
            .iter()
            .all(|line| !line.contains("undo brings it back") || line.contains("replaced")),
        "{at_80:?}"
    );
}

/// Not an assertion — J with a `⇢ replaced` row under Graft's rebase and a
/// `✗ lost` row under the shell's reset, at 187×30 and 80×24, for his eye
/// (rule 6; colour unverified, rule 5). Regenerate by hand:
/// `TRIB_UPDATE_SHOTS=1 cargo test -p graft-ui journal_screen::tests::capture_replaced_for_docs_shots`.
#[test]
fn capture_replaced_for_docs_shots() {
    let mut shots = graft_fixtures::Captures::new();
    for (width, height) in [(187u16, 30u16), (80, 24)] {
        let mut out = String::from(
            "# J after a rebase in the app and a reset in the shell (usability Q1, m8/46).\n\
             #\n\
             # What right looks like: under the `●` rebase, `└ ⇢ replaced` and the\n\
             # feature branch's old tip, `· undo brings it back`, muted; under the `$`\n\
             # reset, `└ ✗ lost` as before, in warning ink. The legend ends\n\
             # `· ⇢ undo brings it back` at 187 columns; at 80 it is today's legend,\n\
             # cut at a word, with no half clause. The two rows' shas start in\n\
             # one column at both widths. Colour is not in this capture.\n\
             \n",
        );
        out.push_str(&format!("=== {width}x{height} ===\n"));
        for line in draw(&view(replaced_rows(), JournalStatus::Ready), width, height) {
            out.push_str(&line);
            out.push('\n');
        }
        shots.check(
            format!(
                "{}/../../docs/shots/m8-q1-journal-replaced-ui-{width}x{height}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            out,
        );
    }
    shots.finish();
}
