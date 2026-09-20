use unicode_width::UnicodeWidthStr;

use super::*;

fn all_contexts() -> Vec<HintContext> {
    vec![
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
        HintContext::Marked(Box::new(HintContext::PlainRow)),
        HintContext::Marked(Box::new(HintContext::BranchRow { current: true })),
        HintContext::Banner { skippable: true },
        HintContext::Banner { skippable: false },
        HintContext::ChangesPane {
            unmerged: false,
            detail_shown: false,
        },
        HintContext::ChangesPane {
            unmerged: true,
            detail_shown: true,
        },
        HintContext::Journal,
        HintContext::Stash,
    ]
}

/// (a) Every context but `Preview` shows something at every width from 40
/// to 200 — the bar never hides every key (ADR-0037 §4).
#[test]
fn no_context_hides_every_key() {
    for context in all_contexts() {
        for columns in 40..=200u16 {
            let hints = hints_for(&context, columns);
            assert!(
                !hints.is_empty(),
                "{context:?} at {columns} columns hid every key"
            );
        }
    }
    for columns in 40..=200u16 {
        assert!(
            hints_for(&HintContext::Preview, columns).is_empty(),
            "Preview's main footer must stay empty — the box owns its own"
        );
    }
}

/// (b) At 80 columns, the row contexts show exactly their first three hints
/// plus `. more`, in the note's table order.
#[test]
fn eighty_columns_shows_three_and_more() {
    let cases: &[(HintContext, [(&str, &str); 3])] = &[
        (
            HintContext::WorkingTreeRow,
            [("⏎", "changes"), ("c", "commit"), ("S", "stash")],
        ),
        (
            HintContext::BranchRow { current: false },
            [("m", "merge"), ("r", "rebase"), ("⏎", "open")],
        ),
        (
            HintContext::BranchRow { current: true },
            [("⏎", "open"), ("c", "commit"), ("P", "push")],
        ),
        (
            HintContext::PlainRow,
            [("⏎", "open"), ("b", "branch"), ("r", "rebase")],
        ),
    ];
    for (context, first_three) in cases {
        let hints = hints_for(context, 80);
        assert_eq!(hints.len(), 4, "{context:?} at 80 columns: {hints:?}");
        for (got, want) in hints.iter().zip(first_three.iter()) {
            assert_eq!((got.key, got.verb), *want, "{context:?}: {hints:?}");
        }
        assert_eq!(hints[3], MORE, "{context:?}: {hints:?}");
    }
}

/// (c) At 120 columns (the 100..160 tier), the row contexts show six hints
/// plus `. more`.
#[test]
fn hundred_twenty_shows_six() {
    for context in [
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
    ] {
        let hints = hints_for(&context, 120);
        assert_eq!(hints.len(), 7, "{context:?}: {hints:?}");
        assert_eq!(hints[6], MORE, "{context:?}: {hints:?}");
    }
}

/// (d) At 160 columns, the row contexts show every hint they have, plus
/// `. more` (the design note's own 160-column mock-up keeps it even with
/// the full list on screen).
#[test]
fn hundred_sixty_shows_all() {
    let cases = [
        (HintContext::WorkingTreeRow, 10),
        (HintContext::BranchRow { current: false }, 12),
        (HintContext::BranchRow { current: true }, 11),
        (HintContext::PlainRow, 11),
    ];
    for (context, count) in cases {
        let hints = hints_for(&context, 160);
        assert_eq!(hints.len(), count + 1, "{context:?}: {hints:?}");
        assert_eq!(hints[count], MORE, "{context:?}: {hints:?}");
    }
}

/// (e) A marked row leads with `. explain`, then the row's own list, and
/// never shows `. more` — `.` is already on screen.
#[test]
fn marked_leads_with_explain_and_omits_more() {
    let marked = HintContext::Marked(Box::new(HintContext::PlainRow));
    for columns in [80, 120, 160] {
        let hints = hints_for(&marked, columns);
        assert_eq!(hints[0], EXPLAIN, "{hints:?}");
        assert!(
            !hints.contains(&MORE),
            "marked row must not also show `. more`: {hints:?}"
        );
        let plain = hints_for(&HintContext::PlainRow, columns);
        let plain_without_more: Vec<_> = plain.into_iter().filter(|hint| *hint != MORE).collect();
        assert_eq!(&hints[1..], plain_without_more.as_slice(), "{hints:?}");
    }
}

/// (f) `fit_status_line` keeps `· p pull · P force` whole at 80 columns
/// and cuts the reason with `…`; a reason that already fits stays whole
/// (acceptance M2: the old blind cut lost the `P` the line was naming).
#[test]
fn status_line_keeps_its_keys() {
    let reason = "push rejected: origin/main has commits you don't know about yet, seriously";
    let keys = "p pull · P force";
    let line = fit_status_line(reason, Some(keys), 80);
    assert!(line.ends_with(" · p pull · P force"), "{line:?}");
    assert!(line.contains('…'), "{line:?}");
    assert_eq!(line.width(), 80, "{line:?}");

    let short_reason = "push rejected";
    let whole = fit_status_line(short_reason, Some(keys), 80);
    assert_eq!(whole, format!("{short_reason} · {keys}"), "{whole:?}");
}

/// Every non-`Preview`, non-`Marked` context list is drawn in the order the
/// design note's table gives it — a change there is a spec change, not a
/// refactor.
#[test]
fn context_lists_match_the_design_note_order() {
    let working_tree = hints_for(&HintContext::WorkingTreeRow, 200);
    let text = hints_text(&working_tree);
    assert_eq!(
        text,
        " ⏎ changes  c commit  S stash  p pull  P push  f fetch  u undo  J journal  / search  j/k move  . more"
    );
}
