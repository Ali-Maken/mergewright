//! The journal, stash and Map footers as the frame draws them (m8/11e F2):
//! the states each is drawn in, and the keys each draws, read off the drawn
//! frame — the bar he sees, not a list beside it (11d's miss was a sweep
//! reading a list no screen drew).

use super::*;
use mergewright_ui::keymap::KeyChord;

/// Wide enough that every footer draws every clause it has.
pub(super) const WIDE: (u16, u16) = (200, 40);

/// The journal open over the merge fixture with a Mergewright row, the shell's
/// reset and the commit it lost; one state per row, the cursor on it. Plus
/// the Mergewright row while its step is live (`⏎` names nothing there).
fn journal_states() -> Vec<(String, AppState)> {
    use mergewright_git::history_reads::{LostRun, Unreached};
    use mergewright_git::reflog::ReflogRecord;
    use mergewright_ops::journal_list::{LostScan, OutsideRecord};

    let oid = |byte: u8| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let record = |refname: &str, old: u8, new: u8, at: i64, subject: &str| OutsideRecord {
        record: ReflogRecord {
            refname: refname.to_owned(),
            old: Some(oid(old)),
            new: Some(oid(new)),
            at,
            subject: subject.to_owned(),
        },
        back: None,
    };
    let walk = JobId::new(701);
    let mut base = overlay_over(merge_on_screen(walk, MERGE_ROW).0, Shown::Journal);
    base.journal.outside = Loading::Ready(Arc::new(vec![
        record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("HEAD", 3, 4, 1004, "commit: D"),
        record("refs/heads/main", 3, 4, 1004, "commit: D"),
        record("HEAD", 5, 3, 1003, "checkout: moving from spike to main"),
    ]));
    base.journal.lost = Loading::Ready(Arc::new(crate::jobs::journal_list::LostFound {
        unreached: Unreached {
            lost: vec![LostRun {
                tip: oid(4),
                under: 0,
            }],
            checked: 1,
            capped: false,
        },
        edits: Vec::new(),
        scan: LostScan::Done,
        subjects: vec![(oid(4), "D".to_owned())],
    }));
    crate::update::journal::rebuild(&mut base);
    let list = base
        .journal
        .list
        .clone()
        .expect("the journal list is built");
    let mut states: Vec<(String, AppState)> = (0..list.rows.len())
        .map(|index| {
            let mut state = base.clone();
            state.journal.cursor = crate::update::journal::row_key(&list, index);
            (format!("journal row {index}"), state)
        })
        .collect();
    let mut live = base;
    live.journal.cursor = None;
    live.live_entries = vec![journal_entry().id];
    states.push(("journal, its Mergewright step live".to_owned(), live));
    states
}

/// Every place state whose footer is drawn from the place's own keys
/// (11e-rulings E3's "yes" rows, Map Loading aside — it has its own test):
/// the journal on every row kind and not ready; the stash list with rows,
/// loading, empty and failed; the Map ready (its `+N more` and `partial`
/// showing), empty and failed.
pub(super) fn place_footer_states() -> Vec<(String, AppState)> {
    let walk = JobId::new(701);
    let behind = || merge_on_screen(walk, MERGE_ROW).0;
    let mut states = journal_states();
    let mut reading = overlay_over(behind(), Shown::Journal);
    reading.journal.entries = Loading::Pending(JobId::new(702));
    states.push(("journal, not read yet".to_owned(), reading));

    states.push((
        "stash list".to_owned(),
        overlay_over(behind(), Shown::Stash),
    ));
    for (what, entries) in [
        ("stash, loading", Loading::Pending(JobId::new(703))),
        ("stash, empty", Loading::Ready(Arc::new(Vec::new()))),
        (
            "stash, failed",
            Loading::Failed("git stash list failed".to_owned()),
        ),
    ] {
        let mut state = overlay_over(behind(), Shown::Stash);
        state.stash.entries = entries;
        states.push((what.to_owned(), state));
    }

    // The cursor on the middle of three rows, so both `j` and `k` have a
    // row to move to (on row 0 `k` cannot move: a test-state artifact, not
    // a dead key).
    for (what, data) in [
        ("map", three_branch_map()),
        ("map, +3 more, partial", map_with_more()),
    ] {
        let mut map = overlay_over(behind(), Shown::Map);
        map.map.data = Loading::Ready(Arc::new(data));
        map.map.selected = 1;
        states.push((what.to_owned(), map));
    }
    let mut empty = overlay_over(behind(), Shown::Map);
    let mut none = two_branch_map();
    none.branches.clear();
    empty.map.data = Loading::Ready(Arc::new(none));
    states.push(("map, empty".to_owned(), empty));
    let mut failed = overlay_over(behind(), Shown::Map);
    failed.map.data = Loading::Failed("git for-each-ref failed".to_owned());
    states.push(("map, failed".to_owned(), failed));
    states
}

/// [`two_branch_map`] with a third row, `b2`.
fn three_branch_map() -> crate::map::MapData {
    let mut data = two_branch_map();
    let mut third = data.branches[1].clone();
    third.name = mergewright_core::ref_name::RefName::new(b"refs/heads/b2".to_vec())
        .expect("a valid ref name");
    third.identity = mergewright_graph::identity::BranchIdentity::of_name(b"b2");
    third.tip = Oid::from_bytes(&[3; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    data.branches.push(third);
    data
}

/// [`three_branch_map`] with three branches more than it shows and one row
/// whose walk was cut short: the Map's ` · +3 more · partial` notes.
pub(super) fn map_with_more() -> crate::map::MapData {
    let mut data = three_branch_map();
    data.more = 3;
    if let Some(row) = data.branches.last_mut() {
        row.complete = false;
    }
    // The footer's ` · partial` reads the build's own flag
    // (`view/map.rs:84`), not a row's.
    data.complete = false;
    data
}

/// `state` drawn at `columns` × `rows`, settled at that size.
pub(super) fn drawn_at(state: &AppState, columns: u16, rows: u16) -> (AppState, String) {
    let state = settled(AppState {
        size: Size { columns, rows },
        ..state.clone()
    });
    let frame = crate::view::tests::draw(&state, columns, rows);
    (state, frame)
}

/// The frame's footer: its last row with anything on it, unquoted and
/// trimmed (`view::tests::draw` quotes each row).
pub(super) fn footer_row(frame: &str) -> String {
    frame
        .lines()
        .map(|row| {
            row.strip_prefix('"')
                .and_then(|row| row.strip_suffix('"'))
                .unwrap_or(row)
                .trim()
        })
        .rfind(|row| !row.is_empty())
        .unwrap_or_default()
        .to_owned()
}

/// Each clause of a footer row and the key labels it draws: the notes
/// (` · +3 more`) are not keys and are cut off; `J/esc close` is `J` and
/// `esc`; `/ find` is `/`.
pub(super) fn footer_keys(row: &str) -> Vec<(String, Vec<String>)> {
    let keys_part = row.split(" · ").next().unwrap_or_default();
    keys_part
        .split("  ")
        .filter(|clause| !clause.is_empty())
        .map(|clause| {
            let key = clause.split(' ').next().unwrap_or_default();
            let labels = if key == "/" {
                vec![key.to_owned()]
            } else {
                key.split('/').map(str::to_owned).collect()
            };
            (clause.to_owned(), labels)
        })
        .collect()
}

/// The press a drawn label stands for: the chord it spells, read back by
/// the keymap's own parser (`KeyChord::parse` reads any `label`).
pub(super) fn press_of(label: &str) -> KeyPress {
    KeyChord::parse(label)
        .unwrap_or_else(|why| panic!("the footer draws `{label}`, which names no key: {why:?}"))
        .to_press()
}

/// The actions the keys a place footer draws act as there, per the
/// keymap's own resolution in the place's context.
pub(super) fn footer_actions(state: &AppState, row: &str) -> Vec<ActionId> {
    let context = crate::update::edge::context_of(state);
    footer_keys(row)
        .into_iter()
        .flat_map(|(_, labels)| labels)
        .filter_map(|label| {
            let chord = KeyChord::parse(&label).ok()?;
            Keymap::shipped().acting(context, chord)
        })
        .collect()
}

/// **Every key a place footer draws is answered** (m8/11e F2): the journal,
/// stash and Map footers as the frame draws them, each key label on them
/// pressed through the whole of `update` and required to **add** something
/// over a key nothing binds (`did_something`). The labels are read off the
/// drawn row, not a list beside it, so a footer word with no key under it
/// fails here (11e-rulings E5: a dead key at Map Empty/Failed or stash
/// Loading/Empty/Failed is a stop, not an exemption).
#[test]
fn every_key_a_place_footer_draws_is_answered() {
    let mut silent = Vec::new();
    let mut pressed = 0;
    let mut pressed_branch = false;
    for (what, state) in place_footer_states() {
        let (state, frame) = drawn_at(&state, WIDE.0, WIDE.1);
        let row = footer_row(&frame);
        let idle = nothing_happened(&state);
        let keys = footer_keys(&row);
        assert!(!keys.is_empty(), "{what}: no footer keys in `{row}`");
        for (clause, labels) in keys {
            for label in labels {
                pressed += 1;
                let answer = outcome(&state, Press::Key(press_of(&label)));
                // 11e-rulings-2 G1: `b` on the stash list names the branch
                // `git stash branch` makes — the prompt opens.
                if what == "stash list" && label == "b" {
                    pressed_branch = true;
                    assert!(
                        answer.0.name_prompt.is_some(),
                        "stash list `b` opens no branch name prompt"
                    );
                }
                if !did_something(&idle, &answer) {
                    silent.push(format!("{what}: `{label}` of `{clause}` in `{row}`"));
                }
            }
        }
    }
    assert!(pressed > 0, "the sweep pressed nothing");
    assert!(pressed_branch, "the stash list's footer draws no `b`");
    assert!(
        silent.is_empty(),
        "a place footer draws a key that does nothing:\n  {}",
        silent.join("\n  ")
    );
}

/// **The Map's Loading footer offers `? keys` only if `?` answers there**
/// (11e-rulings E3, check-first). `?` is put through the whole of `update`
/// at Map Loading; the footer's doorway follows what it did.
#[test]
fn the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers() {
    let mut loading = overlay_over(merge_on_screen(JobId::new(701), MERGE_ROW).0, Shown::Map);
    loading.map.data = Loading::Pending(JobId::new(704));
    let (state, frame) = drawn_at(&loading, 120, 36);
    let row = footer_row(&frame);
    let (asked, _) = outcome(&state, Press::Key(press_of("?")));
    let opened = asked.palette.is_some();
    if opened {
        assert_eq!(
            row, "esc cancels  ? keys",
            "`?` opens the palette at Loading"
        );
        // 11e-rulings-2 G3: closing the palette must not cancel the load
        // (ux H3: closing help is no trap).
        let (closed, commands) = outcome(&asked, Press::Key(press_of("esc")));
        assert!(closed.palette.is_none(), "`esc` closes the palette");
        assert_eq!(
            closed.map.data,
            Loading::Pending(JobId::new(704)),
            "`esc` on the palette left the Map loading"
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::CancelJob(_))),
            "`esc` on the palette cancelled a job: {commands:?}"
        );
    } else {
        assert!(
            !row.contains("keys"),
            "`?` does nothing at Loading: `{row}`"
        );
    }
}

/// **A box over a place draws no keys doorway** (11e-rulings E3): while a
/// journal or stash preview box is up the box owns the keyboard, so its
/// footer is the box's, never `? keys`.
///
/// The stash box is opened by its real key (`⏎`, the apply preview,
/// §2.9). No journal footer key opens a box at once: J's box is the
/// put-back preview `recover_answered` opens with J left shown
/// (`update/journal/answers.rs:141`) when a recover read answers. So the J
/// case drives the state J's view reads (`state.preview` → `box_open`,
/// `view/journal.rs:63`), using the stash's real preview as the box
/// (11e-rulings-2 G4).
#[test]
fn a_box_over_a_place_draws_no_keys_doorway() {
    let mut boxed: Vec<(String, AppState)> = Vec::new();
    let mut stash_box = None;
    for (what, state) in place_footer_states() {
        if what != "stash list" {
            continue;
        }
        let (state, _) = drawn_at(&state, WIDE.0, WIDE.1);
        let (after, _) = outcome(&state, Press::Key(press_of("⏎")));
        assert!(after.preview.is_some(), "stash `⏎` opens the apply preview");
        stash_box.clone_from(&after.preview);
        boxed.push(("stash list `⏎`".to_owned(), after));
    }
    let stash_box = stash_box.expect("the stash list is one of the place states");
    for (what, mut state) in place_footer_states() {
        if !what.starts_with("journal") {
            continue;
        }
        state.preview = Some(stash_box.clone());
        assert!(state.journal.shown, "{what}: J stays shown under its box");
        boxed.push((format!("{what}, a box over it"), state));
    }
    assert!(boxed.len() > 1, "no journal state was boxed");
    let mut drawn: Vec<String> = Vec::new();
    for (what, state) in &boxed {
        for (columns, rows) in [(200, 40), (120, 36), (80, 24), (60, 20)] {
            let (_, frame) = drawn_at(state, columns, rows);
            let row = footer_row(&frame);
            if row.contains("? keys") || row.contains("⌃P keys") {
                drawn.push(format!("{what} at {columns}x{rows}: `{row}`"));
            }
        }
    }
    assert!(
        drawn.is_empty(),
        "a doorway under a box:\n  {}",
        drawn.join("\n  ")
    );
}
