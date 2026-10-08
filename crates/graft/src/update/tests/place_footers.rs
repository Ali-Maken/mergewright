//! The journal, stash and Map footers as the frame draws them (m8/11e F2):
//! the states each is drawn in, and the keys each draws, read off the drawn
//! frame — the bar he sees, not a list beside it (11d's miss was a sweep
//! reading a list no screen drew).

use super::*;
use graft_ui::keymap::KeyChord;

/// Wide enough that every footer draws every clause it has.
pub(super) const WIDE: (u16, u16) = (200, 40);

/// The journal open over the merge fixture with a Graft row, the shell's
/// reset and the commit it lost; one state per row, the cursor on it. Plus
/// the Graft row while its step is live (`⏎` names nothing there).
fn journal_states() -> Vec<(String, AppState)> {
    use graft_git::history_reads::{LostRun, Unreached};
    use graft_git::reflog::ReflogRecord;
    use graft_ops::journal_list::{LostScan, OutsideRecord};

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
    let list = base.journal.list.clone().expect("the journal list is built");
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
    states.push(("journal, its Graft step live".to_owned(), live));
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

    states.push(("stash list".to_owned(), overlay_over(behind(), Shown::Stash)));
    for (what, entries) in [
        ("stash, loading", Loading::Pending(JobId::new(703))),
        ("stash, empty", Loading::Ready(Arc::new(Vec::new()))),
        ("stash, failed", Loading::Failed("git stash list failed".to_owned())),
    ] {
        let mut state = overlay_over(behind(), Shown::Stash);
        state.stash.entries = entries;
        states.push((what.to_owned(), state));
    }

    states.push(("map".to_owned(), overlay_over(behind(), Shown::Map)));
    let mut noted = overlay_over(behind(), Shown::Map);
    noted.map.data = Loading::Ready(Arc::new(map_with_more()));
    states.push(("map, +3 more, partial".to_owned(), noted));
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

/// [`two_branch_map`] with three branches more than it shows and one row
/// whose walk was cut short: the Map's ` · +3 more · partial` notes.
pub(super) fn map_with_more() -> crate::map::MapData {
    let mut data = two_branch_map();
    data.more = 3;
    if let Some(row) = data.branches.last_mut() {
        row.complete = false;
    }
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
                if !did_something(&idle, &answer) {
                    silent.push(format!("{what}: `{label}` of `{clause}` in `{row}`"));
                }
            }
        }
    }
    assert!(pressed > 0, "the sweep pressed nothing");
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
    let opened = outcome(&state, Press::Key(press_of("?")))
        .0
        .palette
        .is_some();
    if opened {
        assert_eq!(row, "esc cancels  ? keys", "`?` opens the palette at Loading");
    } else {
        assert!(!row.contains("keys"), "`?` does nothing at Loading: `{row}`");
    }
}

/// **A box over a place draws no keys doorway** (11e-rulings E3): while a
/// journal or stash preview box is up the box owns the keyboard, so its
/// footer is the box's, never `? keys`. The boxes are the ones the footers'
/// own keys open.
#[test]
fn a_box_over_a_place_draws_no_keys_doorway() {
    let mut boxed: Vec<String> = Vec::new();
    let mut drawn: Vec<String> = Vec::new();
    for (what, state) in place_footer_states() {
        if !(what.starts_with("journal") || what == "stash list") {
            continue;
        }
        let (state, frame) = drawn_at(&state, WIDE.0, WIDE.1);
        for (_, labels) in footer_keys(&footer_row(&frame)) {
            for label in labels {
                let (after, _) = outcome(&state, Press::Key(press_of(&label)));
                if after.preview.is_none() {
                    continue;
                }
                boxed.push(format!("{what} `{label}`"));
                for (columns, rows) in [(120, 36), (80, 24), (60, 20)] {
                    let (_, frame) = drawn_at(&after, columns, rows);
                    let row = footer_row(&frame);
                    if row.contains("? keys") || row.contains("⌃P keys") {
                        drawn.push(format!("{what} `{label}` at {columns}x{rows}: `{row}`"));
                    }
                }
            }
        }
    }
    for wanted in ["journal", "stash"] {
        assert!(
            boxed.iter().any(|opened| opened.starts_with(wanted)),
            "no {wanted} key opened a box: {boxed:?}"
        );
    }
    assert!(
        drawn.is_empty(),
        "a doorway under a box:\n  {}",
        drawn.join("\n  ")
    );
}
