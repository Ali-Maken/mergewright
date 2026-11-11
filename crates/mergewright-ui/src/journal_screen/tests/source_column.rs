//! ADR-0057 §10 (m8/52): the source column's words and the legend's
//! product word.

use super::*;

/// ADR-0057 §10: a `●` row says `Mergewright`, a `$` row `outside`, in the
/// 15-cell source column; under 120 there is no column and the legend's
/// sentence names the product, at 80 as at 187.
#[test]
fn source_column_words() {
    assert_eq!(source_word(RowKind::Mergewright), "Mergewright");
    assert_eq!(source_word(RowKind::Outside), "outside");
    for kind in [
        RowKind::Lost,
        RowKind::LostEdit,
        RowKind::Replaced,
        RowKind::Deleted,
    ] {
        assert_eq!(source_word(kind), "", "{kind:?}");
    }
    let view = view(mock_rows(), JournalStatus::Ready);
    let wide = draw(&view, 187, 14);
    assert!(wide[1].contains("●  Mergewright  "), "{wide:?}");
    assert!(wide[2].contains("$  outside        5m ago"), "{wide:?}");
    // Every row's 15-cell source column holds a word of the exact current
    // set: no other product word can be drawn there.
    let column: Vec<String> = wide
        .iter()
        .filter_map(|line| {
            let cells: Vec<char> = line.chars().collect();
            matches!(cells.get(2), Some('●' | '$'))
                .then(|| cells[5..20].iter().collect::<String>().trim().to_owned())
        })
        .collect();
    assert!(!column.is_empty(), "{wide:?}");
    assert!(
        column
            .iter()
            .all(|word| word == "Mergewright" || word == "outside"),
        "{column:?}"
    );
    let narrow = draw(&view, 80, 24);
    assert!(
        narrow
            .iter()
            .any(|line| line.starts_with("● done in Mergewright: u can undo it")),
        "{narrow:?}"
    );
    assert!(
        legend(Keymap::shipped()).contains("$ done outside Mergewright: u never touches it"),
        "{}",
        legend(Keymap::shipped())
    );
}

/// Not an assertion: J1's mock rows at 187×30 and 80×24, for his eye
/// (rule 6; colour unverified, rule 5). Regenerate by hand:
/// `MWR_UPDATE_SHOTS=1 cargo test -p mergewright-ui journal_screen::tests::source_column`.
#[test]
fn capture_source_column_for_docs_shots() {
    let mut shots = mergewright_fixtures::Captures::new();
    for (width, height) in [(187u16, 30u16), (80, 24)] {
        let mut out = String::from(
            "# J with the app's steps, the shell's and two lost commits (ADR-0057 §10, m8/52).\n\
             #\n\
             # What right looks like: at 187 columns each `●` row reads `●  Mergewright`\n\
             # and each `$` row `$  outside`, the ages in one column after them; at 80\n\
             # there is no source column and the rows start at the glyph. At both\n\
             # widths the legend names Mergewright: `● done in Mergewright: u can undo\n\
             # it · $ done outside Mergewright: …`, cut at a word at 80. The list's\n\
             # last line ends `leaves no trace`, whole at 80. Colour is not in this\n\
             # capture.\n\
             \n",
        );
        out.push_str(&format!("=== {width}x{height} ===\n"));
        for line in draw(&view(mock_rows(), JournalStatus::Ready), width, height) {
            out.push_str(&line);
            out.push('\n');
        }
        shots.check(
            format!(
                "{}/../../docs/shots/m8-mergewright-journal-{width}x{height}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            out,
        );
    }
    shots.finish();
}
