//! `update::undo` tests for edits set aside (m6-conflicts/74, moved out of
//! the inline module by /84, review m6-final-fixes-74 m7): the relaunch line
//! naming where the edits wait, and the landing keeping a done line's
//! `also` when the banner takes the keys.

use super::*;

/// m6-conflicts/74: Graft killed mid-rebase with edits set aside; the
/// relaunch's interrupted line names the stash they wait in, by its
/// position in the list now. Once the stash is gone from the list (he
/// put them back), the clause goes too.
#[test]
fn relaunch_after_a_kill_names_the_stash_the_edits_are_in() {
    let stash = graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes");
    let other = graft_core::oid::Oid::from_bytes(&[0x11; 20]).expect("20 bytes");
    let mut entry = torn_entry(0, "rebase main onto origin/main");
    entry.put_back = Some(graft_core::journal::PutBack {
        stash,
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    let journal = JournalState {
        entries: Loading::Ready(Arc::new(JournalRead {
            entries: vec![entry],
            unreadable: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        })),
        ..JournalState::default()
    };
    let with_stashes = |stashes: &[(graft_core::oid::Oid, usize)]| {
        let main = graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec())
            .expect("a valid ref name");
        AppState {
            journal: journal.clone(),
            refs: Some(Arc::new(
                crate::refs::badges(
                    &[(main.clone(), other)],
                    &graft_git::repo::HeadState::Branch(main),
                    &std::collections::HashMap::new(),
                    stashes,
                    |_, _| Err("no upstreams in this fixture".to_owned()),
                )
                .expect("badges builds from plain data"),
            )),
            ..AppState::default()
        }
    };

    let listed = with_stashes(&[(other, 0), (stash, 1)]);
    assert_eq!(
        interrupted_notice(&listed).as_deref(),
        Some(
            "an operation was interrupted: rebase main onto origin/main \u{b7} edits in stash@{1} \u{b7} J to inspect"
        )
    );
    let gone = with_stashes(&[(other, 0)]);
    assert_eq!(
        interrupted_notice(&gone).as_deref(),
        Some("an operation was interrupted: rebase main onto origin/main \u{b7} J to inspect")
    );
}

/// 30c: the launch that closed the entry read where the edits were
/// (`Recovered::set_aside`); the notice says it in the status line's words,
/// and reads no list to guess. An entry that launch did not close keeps
/// ` · edits in stash@{n}`.
#[test]
fn the_notice_words_where_recovery_found_the_edits() {
    let stash = graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes");
    let mut entry = torn_entry(0, "checkout other");
    entry.put_back = Some(graft_core::journal::PutBack {
        stash,
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    let id = entry.id.clone();
    let main =
        graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec()).expect("a valid ref name");
    let state = |edits: Vec<(graft_core::journal::EntryId, graft_ops::EditsAt)>| AppState {
        journal: JournalState {
            entries: Loading::Ready(Arc::new(JournalRead {
                entries: vec![entry.clone()],
                unreadable: Vec::new(),
                live: Vec::new(),
                interrupted_edits: Vec::new(),
            })),
            ..JournalState::default()
        },
        interrupted_edits: edits,
        refs: Some(Arc::new(
            crate::refs::badges(
                &[(main.clone(), stash)],
                &graft_git::repo::HeadState::Branch(main.clone()),
                &std::collections::HashMap::new(),
                &[(stash, 0)],
                |_, _| Err("no upstreams in this fixture".to_owned()),
            )
            .expect("badges builds from plain data"),
        )),
        ..AppState::default()
    };
    let back = state(vec![(
        id.clone(),
        graft_ops::EditsAt::Back { copy: Some(0) },
    )]);
    assert_eq!(
        interrupted_notice(&back).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} your edits are back \u{b7} a copy stays in stash@{0} \u{b7} J to inspect"
        )
    );
    let in_stash = state(vec![(id, graft_ops::EditsAt::Stash { n: 0 })]);
    assert_eq!(
        interrupted_notice(&in_stash).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} your edits are in stash@{0} \u{b7} S p puts them back \u{b7} J to inspect"
        )
    );
    // Another entry's: this one was closed by an earlier launch.
    let other = state(vec![(
        graft_core::journal::EntryId::new(9, [9; 10]),
        graft_ops::EditsAt::Back { copy: Some(0) },
    )]);
    assert_eq!(
        interrupted_notice(&other).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} edits in stash@{0} \u{b7} J to inspect"
        )
    );
}

/// Review m7-30c-to-10c Major 1: recovery said `Back { copy: Some(0) }`,
/// then he dropped the copy. The notice names no stash that is gone.
#[test]
fn the_notice_drops_a_stash_that_is_gone() {
    let (entry, id) = set_aside_entry();
    let state = with_edits(
        &entry,
        vec![(id.clone(), graft_ops::EditsAt::Back { copy: Some(0) })],
        &[],
    );
    assert_eq!(
        interrupted_notice(&state).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} your edits are back \u{b7} J to inspect"
        )
    );
    let popped = with_edits(&entry, vec![(id, graft_ops::EditsAt::Stash { n: 0 })], &[]);
    let notice = interrupted_notice(&popped).expect("a notice");
    assert!(!notice.contains("stash@"), "{notice}");
}

/// Review m7-30c-to-10c Major 1: `Stash { n: 0 }` at launch, then he stashed
/// something else; the copy is now `stash@{1}` and the notice says so.
#[test]
fn the_stash_clause_follows_the_list() {
    let (entry, id) = set_aside_entry();
    let other = graft_core::oid::Oid::from_bytes(&[0x11; 20]).expect("20 bytes");
    let stash = entry.put_back.as_ref().expect("set aside").stash;
    let state = with_edits(
        &entry,
        vec![(id, graft_ops::EditsAt::Stash { n: 0 })],
        &[(other, 0), (stash, 1)],
    );
    assert_eq!(
        interrupted_notice(&state).as_deref(),
        Some(
            "an operation was interrupted: checkout other \u{b7} your edits are in stash@{1} \u{b7} S p puts them back \u{b7} J to inspect"
        )
    );
}

fn set_aside_entry() -> (JournalEntry, graft_core::journal::EntryId) {
    let stash = graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes");
    let mut entry = torn_entry(0, "checkout other");
    entry.put_back = Some(graft_core::journal::PutBack {
        stash,
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    let id = entry.id.clone();
    (entry, id)
}

fn with_edits(
    entry: &JournalEntry,
    edits: Vec<(graft_core::journal::EntryId, graft_ops::EditsAt)>,
    stashes: &[(graft_core::oid::Oid, usize)],
) -> AppState {
    let main =
        graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec()).expect("a valid ref name");
    let tip = graft_core::oid::Oid::from_bytes(&[0x22; 20]).expect("20 bytes");
    AppState {
        journal: JournalState {
            entries: Loading::Ready(Arc::new(JournalRead {
                entries: vec![entry.clone()],
                unreadable: Vec::new(),
                live: Vec::new(),
                interrupted_edits: Vec::new(),
            })),
            ..JournalState::default()
        },
        interrupted_edits: edits,
        refs: Some(Arc::new(
            crate::refs::badges(
                &[(main.clone(), tip)],
                &graft_git::repo::HeadState::Branch(main),
                &std::collections::HashMap::new(),
                stashes,
                |_, _| Err("no upstreams in this fixture".to_owned()),
            )
            .expect("badges builds from plain data"),
        )),
        ..AppState::default()
    }
}

/// Review m6-final-fixes-74 M4: `land_after_undo` replaces the done line's
/// `keys` with the banner's continue words and leaves `also` alone — a fact
/// is not a key, so the key-driven landing cannot drop it.
#[test]
fn the_landing_keeps_also_when_the_banner_takes_the_keys() {
    let (mut state, read) = finish_undone(AppState::default());
    if let Status::Done { also, .. } = &mut state.status {
        *also = Some("your edits are back".to_owned());
    }
    let state = read_lands(&state, read, Some(graft_git::state::InProgress::Merge));
    assert!(state.banner_focus, "the banner has the keyboard");
    assert_eq!(
        done_line_of(&state.status),
        "undo: finish merge of feature \u{2713} \u{b7} your edits are back \u{b7} c finishes the merge"
    );
}

/// Display cells, the footer cut's own measure (`graft_ui::fit::fit`).
fn cells_of(text: &str) -> usize {
    text.chars()
        .filter_map(unicode_width::UnicodeWidthChar::width)
        .sum()
}

const BACK: &str = "your edits are back \u{b7} a copy stays in stash@{0}";
const IN_STASH: &str = "your edits are in stash@{0} \u{b7} S p puts them back";
const OLDER: &str = "edits in stash@{0}";
const LONG: &str = "rebase feature/payments-retry onto origin/main";

fn rung1(d: &str, c: Option<&str>) -> String {
    match c {
        Some(c) => format!("an operation was interrupted: {d} \u{b7} {c} \u{b7} J to inspect"),
        None => format!("an operation was interrupted: {d} \u{b7} J to inspect"),
    }
}

fn rung2(d: &str, c: &str) -> String {
    format!("interrupted: {d} \u{b7} {c} \u{b7} J to inspect")
}

fn rung4(c: &str) -> String {
    format!("interrupted \u{b7} {c} \u{b7} J to inspect")
}

/// 30d L1: the ladder keeps the clause and `J to inspect` whole, dropping
/// Graft's framing first and his description last.
#[test]
fn the_notice_ladder_keeps_the_clause_and_key() {
    let d = "checkout fix";
    ladder_rows_of_the_table(d);
    whole_from_the_last_rung_up(d);
}

/// L1's sweep: at or above rung 4's width, every result fits and keeps the
/// clause and `J to inspect` whole.
fn whole_from_the_last_rung_up(d: &str) {
    for clause in [BACK, IN_STASH, OLDER] {
        for description in [d, LONG] {
            for cells in 78..=120 {
                let line = notice_line(
                    description,
                    Some(clause),
                    cells,
                    graft_ui::keymap::Keymap::shipped(),
                );
                assert!(cells_of(&line) <= cells, "{line:?} at {cells}");
                assert!(line.contains(clause), "{line:?}");
                assert!(line.ends_with(" \u{b7} J to inspect"), "{line:?}");
            }
        }
    }
}

/// L1's table rows (30d "Widths, worked"), each at its rung widths and 79.
fn ladder_rows_of_the_table(d: &str) {
    for (clause, w1, w2, w4) in [(BACK, 107, 90, 76), (IN_STASH, 108, 91, 77)] {
        assert_eq!(cells_of(&rung1(d, Some(clause))), w1);
        assert_eq!(cells_of(&rung2(d, clause)), w2);
        assert_eq!(cells_of(&rung4(clause)), w4);
        assert_eq!(
            notice_line(d, Some(clause), w1, graft_ui::keymap::Keymap::shipped()),
            rung1(d, Some(clause))
        );
        assert_eq!(
            notice_line(d, Some(clause), w1 - 1, graft_ui::keymap::Keymap::shipped()),
            rung2(d, clause)
        );
        assert_eq!(
            notice_line(d, Some(clause), w2, graft_ui::keymap::Keymap::shipped()),
            rung2(d, clause)
        );
        assert_eq!(
            notice_line(d, Some(clause), 79, graft_ui::keymap::Keymap::shipped()),
            rung4(clause)
        );
    }
    ladder_rows_without_a_new_clause(d);
}

/// L1's last three table rows: the older clause, no clause, and the long
/// description whose rung 3 is the cut.
fn ladder_rows_without_a_new_clause(d: &str) {
    assert_eq!(
        notice_line(d, Some(OLDER), 79, graft_ui::keymap::Keymap::shipped()),
        "an operation was interrupted: checkout fix \u{b7} edits in stash@{0} \u{b7} J to inspect"
    );
    assert_eq!(cells_of(&rung1(d, Some(OLDER))), 78);
    assert_eq!(cells_of(&rung1(d, None)), 57);
    assert_eq!(
        notice_line(d, None, 57, graft_ui::keymap::Keymap::shipped()),
        rung1(d, None)
    );
    assert_eq!(cells_of(&rung1(LONG, Some(OLDER))), 112);
    assert_eq!(cells_of(&rung2(LONG, OLDER)), 95);
    assert_eq!(
        notice_line(LONG, Some(OLDER), 112, graft_ui::keymap::Keymap::shipped()),
        rung1(LONG, Some(OLDER))
    );
    assert_eq!(
        notice_line(LONG, Some(OLDER), 95, graft_ui::keymap::Keymap::shipped()),
        rung2(LONG, OLDER)
    );
    let rung3 = notice_line(LONG, Some(OLDER), 79, graft_ui::keymap::Keymap::shipped());
    assert_eq!(
        rung3,
        "interrupted: rebase feature/payments-retry\u{2026} \u{b7} edits in stash@{0} \u{b7} J to inspect"
    );
    assert_eq!(cells_of(&rung3), 79);
}

/// 30d L2: narrower than the last rung, the last rung (the footer cuts it).
#[test]
fn the_notice_below_its_last_rung_is_the_last_rung() {
    assert_eq!(
        notice_line(
            "checkout fix",
            Some(BACK),
            60,
            graft_ui::keymap::Keymap::shipped()
        ),
        rung4(BACK)
    );
}

/// 30d L3: the rung is picked at draw time, so a resize re-picks.
#[test]
fn the_notice_repicks_on_resize() {
    let mut entry = torn_entry(0, "checkout fix");
    entry.put_back = Some(graft_core::journal::PutBack {
        stash: graft_core::oid::Oid::from_bytes(&[0x5a; 20]).expect("20 bytes"),
        why: graft_core::journal::PutBackWhy::SetAside,
    });
    let id = entry.id.clone();
    let state = AppState {
        journal: JournalState {
            entries: Loading::Ready(Arc::new(JournalRead {
                entries: vec![entry],
                unreadable: Vec::new(),
                live: Vec::new(),
                interrupted_edits: Vec::new(),
            })),
            ..JournalState::default()
        },
        interrupted_edits: vec![(id, graft_ops::EditsAt::Back { copy: Some(0) })],
        ..AppState::default()
    };
    let wide = rung1("checkout fix", Some(BACK));
    let narrow = rung4(BACK);
    assert_eq!(interrupted_notice_in(&state, 120), Some(wide.clone()));
    assert_eq!(interrupted_notice_in(&state, 79), Some(narrow));
    assert_eq!(interrupted_notice_in(&state, 120), Some(wide.clone()));
    assert_eq!(interrupted_notice(&state), Some(wide));
}

/// 30d L4: his description is sanitized before it is measured, so the
/// cells counted are the cells drawn.
#[test]
fn a_hostile_description_is_measured_as_drawn() {
    let hostile =
        "rebase \u{1b}[31m\u{7}fix\u{5b57}\u{5b57}\u{5b57} onto feature/payments-retry-longer";
    for cells in 60..=130 {
        let line = notice_line(
            hostile,
            Some(OLDER),
            cells,
            graft_ui::keymap::Keymap::shipped(),
        );
        let drawn = graft_term::sanitize_line(&line);
        assert_eq!(cells_of(&line), cells_of(drawn.as_str()), "{line:?}");
        assert!(cells_of(&line) <= cells, "{line:?} at {cells}");
    }
}

/// m8-polish-and-ship/20 item 5: with a set-aside clause and the
/// unreadable-journal warning, 80 columns never cuts `J to inspect`: the
/// set-aside clause sheds its detail first.
#[test]
fn at_80_the_notice_keeps_j_to_inspect_whole() {
    let entry = torn_entry(0, "rebase feature/payments-retry onto main");
    let id = entry.id.clone();
    let state = AppState {
        journal: JournalState {
            entries: Loading::Ready(Arc::new(JournalRead {
                entries: vec![entry],
                unreadable: Vec::new(),
                live: Vec::new(),
                interrupted_edits: Vec::new(),
            })),
            unreadable: vec!["torn".to_owned()],
            ..JournalState::default()
        },
        interrupted_edits: vec![(id, graft_ops::EditsAt::Stash { n: 0 })],
        ..AppState::default()
    };
    let wide = interrupted_notice_in(&state, usize::MAX).expect("a notice");
    assert!(wide.contains("puts them back"), "{wide}");
    let line = interrupted_notice_in(&state, 79).expect("a notice");
    assert!(cells_of(&line) <= 79, "{line}");
    assert!(line.ends_with("J to inspect"), "{line}");
    assert!(line.contains("its state could not be read"), "{line}");
}
