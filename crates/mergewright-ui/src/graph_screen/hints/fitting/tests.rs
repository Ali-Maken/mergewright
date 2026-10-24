use super::super::{Divider, HintContext, hints_for};
use super::*;

fn keys() -> &'static Keymap {
    Keymap::shipped()
}

/// M8/11f-b2-ready-join: `place_footer_text` is `fit_place_footer` then
/// `keys_text` — the same string at a width that keeps every clause, one
/// that sheds, and one below the doorway's words.
#[test]
fn keys_text_is_place_footer_text() {
    let clauses: Vec<String> = ["s save", "S/esc close", "d drop"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for width in [120, 24, 4] {
        let fitted = fit_place_footer(&clauses, keys(), &[], width);
        assert_eq!(
            fitted.keys_text(&clauses),
            place_footer_text(&clauses, keys(), width),
            "at {width}"
        );
    }
    let shed = fit_place_footer(&clauses, keys(), &[], 24);
    assert!(shed.clauses.len() < clauses.len(), "24 sheds: {shed:?}");
}

/// M8/11f-b2-ready-join: the one join — kept clauses, then the doorway,
/// two cells apart; dropped clauses and notes are not drawn.
#[test]
fn keys_text_joins_kept_clauses_then_the_doorway() {
    let clauses = vec!["a one".to_owned(), "b two".to_owned(), "c three".to_owned()];
    let fitted = PlaceFooter {
        clauses: vec![0, 2],
        doorway: Some("? keys".to_owned()),
        notes: vec![0],
    };
    assert_eq!(fitted.keys_text(&clauses), "a one  c three  ? keys");
    let bare = PlaceFooter {
        clauses: Vec::new(),
        doorway: None,
        notes: Vec::new(),
    };
    assert_eq!(bare.keys_text(&clauses), "");
}

/// A stash clash's ladder as `mergewright::update::clash::stop_wordings`
/// writes it: this run's, then from the last run.
fn clash_ladder(from_last_run: bool) -> StopWording {
    let head = if from_last_run {
        "stash@{0} clash from last run"
    } else {
        "stash@{0} clashed"
    };
    StopWording::new(
        mergewright_term::sanitize_line(&format!("{head} · 1 file conflicted")),
        [
            "stash@{0} clashed · 1 conflicted",
            "stash@{0} clashed",
            "stash clashed",
        ]
        .iter()
        .map(|words| mergewright_term::sanitize_line(words))
        .collect(),
    )
    .before_keys()
}

/// Review 72+72b M1: at 80 columns on the graph, the clash's reason is
/// drawn before the bar — the fullest wording that leaves the bar its
/// floor, the tail keys giving first — and the relaunch drops its origin,
/// never the name.
#[test]
fn a_clash_reason_outranks_the_tail_keys_at_80() {
    let context = HintContext::WorkingTreeRow;
    for divider in [Divider::Hidden, Divider::Shown] {
        let row = stopped_row_before_keys(&clash_ladder(false), &context, divider, 80, keys());
        assert!(
            row.starts_with(" stash@{0} clashed · 1 file conflicted ·"),
            "{divider:?}: {row:?}"
        );
        assert!(row.width() <= 80, "{row:?}");
        let bar = row
            .strip_prefix(" stash@{0} clashed · 1 file conflicted ·")
            .expect("the reason leads");
        let floor = hints_text(&hints_for(&context, divider, 80, keys())[..1], keys());
        assert!(
            bar.starts_with(floor.as_str()),
            "the floor leads the bar: {bar:?}"
        );

        let relaunch = stopped_row_before_keys(&clash_ladder(true), &context, divider, 80, keys());
        assert!(
            relaunch.starts_with(" stash@{0} clashed · 1 conflicted ·")
                || relaunch.starts_with(" stash@{0} clash from last run · 1 file conflicted ·"),
            "{divider:?}: {relaunch:?}"
        );
        assert!(relaunch.width() <= 80, "{relaunch:?}");
    }
}

/// The pin: a stop without the flag is `stopped_row`'s, the bar whole —
/// here at 80 the bar leaves under twelve cells, so the row is the bar
/// alone, as it was before.
#[test]
fn a_stop_without_before_keys_fits_as_today() {
    let context = HintContext::WorkingTreeRow;
    let plain = StopWording::new(
        mergewright_term::sanitize_line("merge stopped · 1 file conflicted"),
        Vec::new(),
    );
    assert!(!plain.is_before_keys());
    for columns in [80u16, 120, 187] {
        let hints = hints_for(&context, Divider::Hidden, columns, keys());
        let row = stopped_row(&plain, &hints, columns, keys());
        assert!(
            row.ends_with(&hints_text(&hints, keys())),
            "{columns}: {row:?}"
        );
    }
}

/// The banner-focused stop row of a merge begun in the shell with two files
/// conflicted, as `mergewright::update::banner::stop_wordings` writes it.
fn shell_merge(yield_through: bool) -> StopWording {
    let wording = StopWording::new(
        mergewright_term::sanitize_line(
            "merge in progress \u{b7} begun outside Mergewright \u{b7} 2 files conflicted",
        ),
        [
            "merge in progress \u{b7} from the shell \u{b7} 2 conflicted",
            "merge in progress \u{b7} 2 conflicted",
            "merge in progress",
        ]
        .iter()
        .map(|words| mergewright_term::sanitize_line(words))
        .collect(),
    );
    if yield_through {
        wording.keys_yield_through(2)
    } else {
        wording
    }
}

fn banner_row(reason: &StopWording, columns: u16) -> String {
    let context = HintContext::Banner {
        skippable: false,
        conflicted: true,
        merge: true,
    };
    let hints = super::super::hints_for(&context, Divider::Hidden, columns, keys());
    stopped_row(reason, &hints, columns, keys())
}

const FLOOR_KEYS: &str = "\u{23ce} resolve  c finish  a abort";

/// Ticket ease-28 Q3 (his rule: "drop the origin, keep the count"): at 92
/// cells the count is drawn and the bar's tail keys give way for it.
#[test]
fn tail_keys_yield_to_a_counted_wording() {
    assert_eq!(
        banner_row(&shell_merge(true), 92),
        " merge in progress \u{b7} 2 conflicted \u{b7} \u{23ce} resolve  c finish  a abort  \
         M mergetool  e edit"
    );
    for columns in 70u16..=200 {
        let row = banner_row(&shell_merge(true), columns);
        assert!(row.width() <= usize::from(columns), "{columns}: {row:?}");
        assert!(row.contains(" conflicted"), "{columns}: {row:?}");
        assert!(row.contains(FLOOR_KEYS), "{columns}: {row:?}");
    }
}

/// Where the floor plus the shortest counted wording cannot fit, the row is
/// today's: the bar and its first three keys whole.
#[test]
fn the_floor_never_yields_to_the_count() {
    for columns in 30u16..70 {
        let with = banner_row(&shell_merge(true), columns);
        let without = banner_row(&shell_merge(false), columns);
        if !with.contains(" conflicted") {
            assert_eq!(with, without, "{columns}");
        }
        if without.contains(FLOOR_KEYS) {
            assert!(with.contains(FLOOR_KEYS), "{columns}: {with:?}");
        }
    }
}

/// No key but `q quit` gives way to the origin: it drops only when that
/// lets a fuller wording fit whole, as before, and never another key.
#[test]
fn the_origin_takes_no_key_but_quit() {
    for columns in 100u16..=200 {
        let with = banner_row(&shell_merge(true), columns);
        let without = banner_row(&shell_merge(false), columns);
        if without.contains(" conflicted") {
            assert_eq!(with, without, "{columns}");
        }
        if with.contains("from the shell") || with.contains("begun outside") {
            assert!(with.contains("e edit"), "{columns}: {with:?}");
        }
    }
}

/// Unset, the bar is never trimmed beyond `q quit`: the 92-cell row is the
/// bar whole with the reason cut to what is left, as today.
#[test]
fn without_keys_yield_through_nothing_changes() {
    let row = banner_row(&shell_merge(false), 92);
    assert!(!row.contains("2 conflicted"), "{row:?}");
    assert!(row.ends_with("\u{21e5} graph  q quit"), "{row:?}");
}
