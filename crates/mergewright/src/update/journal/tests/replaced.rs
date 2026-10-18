//! m8/46 (usability Q1): a `⇢ replaced` row is still a `ListRow::Lost` —
//! `⏎`, `b` and the bar act on it as on `✗` — but `n`/`N` pass it over and
//! the preselect never lands on it.

use mergewright_ops::journal_list::OutsideStep;

use super::*;

fn frame(state: &AppState) -> Vec<String> {
    crate::view::tests::draw(state, 120, 30)
        .lines()
        .map(|line| line.trim_matches('"').trim_end().to_owned())
        .collect()
}

/// The cursor on row `index` of the hand-built list (`put_cursor` would
/// rebuild it from the journal and drop the hand-built rows).
fn cursor_on(state: &mut AppState, index: usize) {
    let list = state.journal.list.clone().expect("built");
    state.journal.cursor = row_key(&list, index);
}

fn shell_reset_row() -> ListRow {
    ListRow::Outside(OutsideStep {
        record: record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1").record,
        words: "reset main 1 commit back".to_owned(),
        folded: 0,
    })
}

fn lost_on_main(event: usize, tip: u8, fate: Fate) -> ListRow {
    ListRow::Lost {
        event,
        commit: LostCommit {
            run: LostRun {
                tip: oid(tip),
                under: 0,
            },
            subject: Some(format!("subject {tip}")),
            held_by: HeldBy::Branch {
                name: "main".to_owned(),
                gone: false,
            },
            fate,
        },
    }
}

/// J open on: Mergewright's newest entry with a `⇢` row under it (tip 5), the
/// shell's reset with a `✗` row under it (tip 4), and Mergewright's older entry.
fn with_replaced() -> AppState {
    let mut state = opened();
    state.journal.list = Some(Arc::new(JournalList {
        rows: vec![
            ListRow::Mergewright(newest_entry()),
            lost_on_main(0, 5, Fate::Replaced),
            shell_reset_row(),
            lost_on_main(2, 4, Fate::Lost),
            ListRow::Mergewright(older_entry()),
        ],
        scan: LostScan::Done,
    }));
    state
}

#[test]
fn n_passes_over_replaced_rows() {
    let mut state = with_replaced();
    cursor_on(&mut state, 0);
    press(&mut state, KeyCode::Char('n'));
    assert_eq!(state.journal.selected(), 3, "past the `⇢` row to the `✗`");
    press(&mut state, KeyCode::Char('n'));
    assert_eq!(state.journal.selected(), 3, "none older: stays");
    press(&mut state, KeyCode::Char('N'));
    assert_eq!(
        state.journal.selected(),
        3,
        "the `⇢` row is no stop going up"
    );
    cursor_on(&mut state, 4);
    press(&mut state, KeyCode::Char('N'));
    assert_eq!(state.journal.selected(), 3);
}

/// `⏎` asks `recover_for` about the row, and `b` offers a branch at its
/// tip — exactly as on the `✗` row.
#[test]
fn enter_and_b_on_a_replaced_row_act_as_on_lost() {
    for (row, tip) in [(1, 5), (3, 4)] {
        let mut state = with_replaced();
        cursor_on(&mut state, row);
        let (claimed, commands) = press(&mut state, KeyCode::Enter);
        assert!(claimed);
        assert!(
            matches!(
                commands.as_slice(),
                [Command::RecoverFor { row: asked, .. }] if *asked == row
            ),
            "row {row}: {commands:?}"
        );

        let mut state = with_replaced();
        cursor_on(&mut state, row);
        press(&mut state, KeyCode::Char('b'));
        assert_eq!(
            state.name_prompt.as_ref().expect("the name prompt").kind,
            crate::update::name_prompt::NameKind::Branch {
                at: oid(tip),
                checkout: false
            },
            "row {row}"
        );
    }
}

/// The same row, `⇢` or `✗`: the bar is the same.
#[test]
fn replaced_row_bar_matches_lost_row_bar() {
    let bar_for = |fate: Fate| {
        let mut state = opened();
        state.journal.list = Some(Arc::new(JournalList {
            rows: vec![
                ListRow::Mergewright(newest_entry()),
                lost_on_main(0, 5, fate),
            ],
            scan: LostScan::Done,
        }));
        cursor_on(&mut state, 1);
        frame(&state).last().cloned().unwrap_or_default()
    };
    let lost = bar_for(Fate::Lost);
    assert!(lost.contains("put main back to it"), "{lost}");
    assert_eq!(bar_for(Fate::Replaced), lost);
    assert_eq!(bar_for(Fate::Deleted), lost);
}

#[test]
fn preselect_never_lands_on_a_replaced_row() {
    // Newest first: a `●` fetch (transparent to the preselect) with a `⇢`
    // row under it, then the shell's reset with its `✗`.
    let list = JournalList {
        rows: vec![
            ListRow::Mergewright(newest_entry()),
            lost_on_main(0, 5, Fate::Replaced),
            shell_reset_row(),
            lost_on_main(2, 4, Fate::Lost),
        ],
        scan: LostScan::Done,
    };
    assert_eq!(preselect(&list, |_| true), Some(3));
    assert_eq!(
        preselect(&list, |_| false),
        None,
        "Mergewright worked since"
    );
}
