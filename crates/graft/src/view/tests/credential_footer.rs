//! The credential prompt names its push in the box's bottom border
//! (m6-conflicts/78, Decided 2): F9 put the push's description on the row
//! under the prompt; 69 gave that row to the prompt's keys, and the border —
//! which already names what `Esc` cancels — now carries it instead.

use super::*;

const SIZES: [(u16, u16); 2] = [(120, 40), (80, 24)];

/// The box's bottom border as drawn: the row holding `esc cancel`, from
/// `╰` to `╯`.
fn border_row(state: &AppState, columns: u16, rows: u16) -> String {
    let screen = draw(state, columns, rows);
    let line = screen
        .lines()
        .find(|line| line.contains('╰') && line.contains("esc cancel"))
        .unwrap_or_else(|| panic!("no bottom border names esc\n{screen}"));
    let start = line.find('╰').expect("the border's left corner");
    let end = line.rfind('╯').expect("the border's right corner");
    line[start..end + '╯'.len_utf8()].to_owned()
}

/// Everything the border says, its dashes and corners taken off.
fn label(border: &str) -> String {
    border
        .trim_start_matches('╰')
        .trim_end_matches('╯')
        .trim_matches('─')
        .trim()
        .to_owned()
}

#[test]
fn the_credential_prompt_names_its_push() {
    for (columns, rows) in SIZES {
        let state = credential_state(columns, rows, "Password for 'http://user@127.0.0.1:8443': ");
        let drawn = border_row(&state, columns, rows);
        assert_eq!(
            label(&drawn),
            "⏎ send  ⌃U clear  esc cancel push main → origin/main",
            "{columns}x{rows}: {drawn}"
        );

        // A branch whose push would not fit: today's words, nothing cut.
        let mut long = state.clone();
        let branch = "b".repeat(60);
        let running = long.remote.running.as_mut().expect("the push runs");
        running.description = format!("push {branch} → origin/{branch}");
        let drawn = border_row(&long, columns, rows);
        assert_eq!(
            label(&drawn),
            "⏎ send  ⌃U clear  esc cancel the push",
            "{columns}x{rows}: {drawn}"
        );
        assert!(!drawn.contains('…'), "{columns}x{rows}: nothing cut");
    }
}
