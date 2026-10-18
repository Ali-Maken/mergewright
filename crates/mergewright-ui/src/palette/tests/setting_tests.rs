//! 06i: a setting's choices in the list — `●`/`○` drawn by kind, the muted
//! detail, `(on)` never cut, the config key in the id column.

use super::*;

fn choice(title: &str, detail: &'static str, on: bool) -> PaletteRow {
    PaletteRow {
        label: sanitize_line(title),
        key: None,
        config: Some(ConfigId::Key("graph.merged_branches")),
        detail: Some(detail),
        kind: RowKind::Setting { on },
    }
}

fn graph_mode(wide: bool) -> PaletteView {
    let mut view = help(wide);
    view.groups.insert(
        2,
        group(
            "GRAPH MODE",
            vec![
                choice(
                    "Folded merges",
                    "merged branches fold into their merge row",
                    true,
                ),
                choice(
                    "Every branch",
                    "every branch on its own lane, as before",
                    false,
                ),
            ],
        ),
    );
    view
}

fn cell_style(buf: &Buffer, needle: &str) -> ratatui::style::Style {
    let area = buf.area;
    for y in 0..area.height {
        for x in 0..area.width {
            if buf[(x, y)].symbol() == needle {
                return buf[(x, y)].style();
            }
        }
    }
    panic!("{needle} not drawn");
}

fn accent() -> Option<ratatui::style::Color> {
    foreground(&theme(), Token::Accent).fg
}

#[test]
fn setting_rows_draw_their_dot_by_kind_and_action_keys_stay_accent_bold() {
    let buf = buffer(&graph_mode(true));
    let on = cell_style(&buf, "●");
    assert_eq!(on.fg, accent());
    assert!(on.add_modifier.contains(Modifier::BOLD));
    let off = cell_style(&buf, "○");
    assert_eq!(off.fg, Some(muted()));
    assert!(!off.add_modifier.contains(Modifier::BOLD));
    // The action row's key cell: two cells after the border and the `▸` slot.
    let (_, lines) = drawn(&graph_mode(true), 120, 36);
    let y = lines
        .iter()
        .position(|l| l.contains("merge the selected"))
        .unwrap();
    let x = lines[y].chars().position(|c| c == 'm').unwrap();
    let m = buf[(u16::try_from(x).unwrap(), u16::try_from(y).unwrap())].style();
    assert_eq!(m.fg, accent());
    assert!(m.add_modifier.contains(Modifier::BOLD));
    assert_eq!(style_of(&buf, "merged branches fold"), muted());
    assert_eq!(style_of(&buf, "(on)"), muted());
}

#[test]
fn the_graph_mode_group_sits_after_here_with_its_key_at_120() {
    let (_, lines) = drawn(&graph_mode(true), 120, 36);
    let at = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap();
    assert!(at("HERE") < at("GRAPH MODE"));
    assert!(at("GRAPH MODE") < at("EVERYWHERE"));
    let on = line_with(&lines, "Folded merges");
    assert!(
        on.contains("●     Folded merges   merged branches fold into their merge …   (on)"),
        "{on}"
    );
    assert!(on.contains("graph.merged_branches"), "{on}");
    let off = line_with(&lines, "Every branch");
    assert!(
        off.contains("○     Every branch    every branch on its own lane, as before"),
        "{off}"
    );
    assert!(!off.contains("(on)"), "{off}");
    assert!(off.contains("graph.merged_branches"), "{off}");
    assert!(line_with(&lines, "merge the selected").contains("graph.merge"));
}

#[test]
fn narrow_the_id_drops_then_the_detail_is_cut_and_on_stays_whole() {
    let (_, lines) = drawn(&graph_mode(false), 90, 30);
    let on = line_with(&lines, "Folded merges");
    assert!(!on.contains("graph.merged_branches"), "{on}");
    assert!(on.contains("(on)"), "{on}");
    let (_, lines) = drawn(&graph_mode(false), 50, 30);
    let on = line_with(&lines, "Folded merges");
    assert!(on.contains("(on)"), "{on}");
    assert!(!on.contains("merge row"), "{on}");
}

#[test]
fn a_click_on_a_setting_row_finds_it_and_its_header_finds_nothing() {
    let view = graph_mode(true);
    let (area, lines) = drawn(&view, 120, 36);
    let row = lines
        .iter()
        .position(|l| l.contains("Every branch"))
        .unwrap();
    let column = area.x + 10;
    // IN PROGRESS 2 + HERE 2 + Folded merges: Every branch is the sixth row.
    assert_eq!(
        palette_row_at(area, &view, column, u16::try_from(row).unwrap()),
        Some(5)
    );
    let header = lines.iter().position(|l| l.contains("GRAPH MODE")).unwrap();
    assert_eq!(
        palette_row_at(area, &view, column, u16::try_from(header).unwrap()),
        None
    );
}
