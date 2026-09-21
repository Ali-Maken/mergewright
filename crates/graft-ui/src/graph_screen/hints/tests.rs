use unicode_width::UnicodeWidthStr;

use super::*;

/// Every context the graph footer can be in, `Preview` aside (it is the one
/// context allowed an empty list).
fn all_contexts() -> Vec<HintContext> {
    vec![
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
        HintContext::Marked(Box::new(HintContext::PlainRow)),
        HintContext::Marked(Box::new(HintContext::BranchRow { current: true })),
        HintContext::Banner {
            skippable: true,
            conflicted: true,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: true,
        },
        HintContext::Banner {
            skippable: true,
            conflicted: false,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: false,
        },
        HintContext::ChangesPane { unmerged: false },
        HintContext::ChangesPane { unmerged: true },
        HintContext::Journal,
        HintContext::Stash,
    ]
}

/// The four cursor-row contexts, which are the ones the design note's
/// mock-ups are drawn for.
fn row_contexts() -> Vec<HintContext> {
    vec![
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
    ]
}

/// (a) Every context but `Preview` shows something at every width from 40
/// to 200, with or without a detail pane open — the bar never hides every
/// key (ADR-0037 §4).
#[test]
fn no_context_hides_every_key() {
    for context in all_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in 40..=200u16 {
                let hints = hints_for(&context, divider, columns);
                assert!(
                    !hints.is_empty(),
                    "{context:?} at {columns} columns ({divider:?}) hid every key"
                );
            }
        }
    }
    for columns in 40..=200u16 {
        assert!(
            hints_for(&HintContext::Preview, Divider::Hidden, columns).is_empty(),
            "Preview's main footer must stay empty — the box owns its own"
        );
    }
}

/// (b) At 80 columns, the row contexts show exactly their first three
/// *bound* hints plus `. more`, in the note's table order — the planned
/// entries (`m merge`, `r rebase`, `b branch`, `t tag`) are not among them.
#[test]
fn eighty_columns_shows_three_and_more() {
    let cases: &[(HintContext, [(&str, &str); 3])] = &[
        (
            HintContext::WorkingTreeRow,
            [("⏎", "changes"), ("c", "commit"), ("S", "stash")],
        ),
        (
            HintContext::BranchRow { current: false },
            [("⏎", "open"), ("P", "push"), ("c", "commit")],
        ),
        (
            HintContext::BranchRow { current: true },
            [("⏎", "open"), ("c", "commit"), ("P", "push")],
        ),
        (
            HintContext::PlainRow,
            [("⏎", "open"), ("c", "commit"), ("y", "sha")],
        ),
    ];
    for (context, first_three) in cases {
        let hints = hints_for(context, Divider::Hidden, 80);
        assert_eq!(hints.len(), 4, "{context:?} at 80 columns: {hints:?}");
        for (got, want) in hints.iter().zip(first_three.iter()) {
            assert_eq!((got.key, got.verb), *want, "{context:?}: {hints:?}");
        }
        assert_eq!(hints[3], MORE, "{context:?}: {hints:?}");
    }
}

/// (c) At 120 columns (the 100..160 tier), the row contexts show six hints
/// plus `. more`, and the line still fits.
#[test]
fn hundred_twenty_shows_six() {
    for context in row_contexts() {
        let hints = hints_for(&context, Divider::Hidden, 120);
        assert_eq!(hints.len(), 7, "{context:?}: {hints:?}");
        assert_eq!(hints[6], MORE, "{context:?}: {hints:?}");
        assert!(
            hints_text(&hints).width() <= 120,
            "{context:?}: {:?}",
            hints_text(&hints)
        );
    }
}

/// (d) From 160 columns a row context shows its own list *and* keeps going
/// into the keys the whole screen answers ([`TAIL`]) — `q quit` and
/// `x compare` are hinted at some width rather than none (m5-03 review,
/// Major 3) — dropping from the right only where the row runs out.
#[test]
fn hundred_sixty_shows_the_context_then_the_screen() {
    for context in row_contexts() {
        let hints = hints_for(&context, Divider::Hidden, 160);
        let own: Vec<Hint> = drawable(&context, Divider::Hidden);
        assert!(
            hints.len() > 7,
            "{context:?} shows no more at 160 than at 120: {hints:?}"
        );
        let drawn = &hints[..hints.len() - 1];
        assert_eq!(
            drawn,
            &own[..drawn.len()],
            "{context:?} reordered its list: {hints:?}"
        );
        assert_eq!(hints.last(), Some(&MORE), "{context:?}: {hints:?}");
        let text = hints_text(&hints);
        assert!(text.width() <= 160, "{context:?}: {text:?} does not fit");
        assert!(
            text.contains("q quit"),
            "{context:?} still hides `q quit` at 160: {text:?}"
        );
    }
    // Wide enough for the whole tail as well: `x compare`, the one live key
    // the keymap reconciliation found with no documentation at all.
    let wide = hints_text(&hints_for(&HintContext::PlainRow, Divider::Hidden, 200));
    assert!(wide.contains("x compare"), "{wide:?}");
}

/// (e) A marked row leads with `. explain`, then the row's own list, never
/// shows `. more` (`.` is already on screen) — and, counting `. explain`
/// as one of the width tier's own, never overflows the row.
#[test]
fn marked_leads_with_explain_and_omits_more() {
    let marked = HintContext::Marked(Box::new(HintContext::PlainRow));
    for columns in [40, 80, 120, 160, 200] {
        let hints = hints_for(&marked, Divider::Hidden, columns);
        assert_eq!(hints[0], EXPLAIN, "{hints:?}");
        assert!(
            !hints.contains(&MORE),
            "marked row must not also show `. more`: {hints:?}"
        );
        let plain = drawable(&HintContext::PlainRow, Divider::Hidden);
        assert_eq!(&hints[1..], &plain[..hints.len() - 1], "{hints:?}");
        assert!(
            hints_text(&hints).width() <= usize::from(columns),
            "{columns} columns: {:?}",
            hints_text(&hints)
        );
    }
}

/// (f) `fit_status_line` keeps `· p pull · P force` whole at 80 columns
/// and cuts the reason with `…`; a reason that already fits stays whole
/// (acceptance M2: the old blind cut lost the `P` the line was naming).
/// When not even the keys fit beside a separator, the separator goes rather
/// than three cells with nothing in front of them (m5-03 review, Minor 2).
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

    let cramped = fit_status_line(reason, Some(keys), 12);
    assert!(!cramped.starts_with(" ·"), "{cramped:?}");
    assert!(cramped.width() <= 12, "{cramped:?}");
}

/// Nothing the footer draws is a key no group has built yet: the table
/// keeps the design note's planned entries, the width rule never sees them
/// (m5-03 review, Critical 2 — at 80 columns two of a branch row's three
/// keys did nothing at all).
#[test]
fn no_planned_hint_is_ever_drawn() {
    let planned: Vec<Hint> = every_hint()
        .into_iter()
        .filter(|hint| hint.state == HintState::Planned)
        .collect();
    assert!(
        planned
            .iter()
            .any(|hint| (hint.key, hint.verb) == ("m", "merge")),
        "the design note's planned entries must stay in the table: {planned:?}"
    );
    for context in all_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in [40, 80, 100, 120, 160, 200] {
                for drawn in hints_for(&context, divider, columns) {
                    assert_eq!(
                        drawn.state,
                        HintState::Live,
                        "{context:?} at {columns} draws the unbuilt `{} {}`",
                        drawn.key,
                        drawn.verb
                    );
                }
            }
        }
    }
}

/// `<> resize` is hinted exactly while a detail pane is open, in every
/// context — a commit's pane has the same divider the changes pane has
/// (ADR-0028, m5-03 review, Major 4).
#[test]
fn the_divider_hint_follows_the_pane_not_the_context() {
    for context in [
        HintContext::PlainRow,
        HintContext::BranchRow { current: true },
        HintContext::WorkingTreeRow,
        HintContext::ChangesPane { unmerged: false },
    ] {
        let with = hints_text(&hints_for(&context, Divider::Shown, 200));
        let without = hints_text(&hints_for(&context, Divider::Hidden, 200));
        assert!(with.contains("<> resize"), "{context:?}: {with:?}");
        assert!(!without.contains("<> resize"), "{context:?}: {without:?}");
    }
}

/// And it is drawn where a pane and a graph actually fit side by side: at
/// the six-hint tier, fourth. Appended after the context's own list instead,
/// it sat behind ten keys and no real frame ever reached it — a hint that
/// exists and is never drawn is the bug it was meant to fix (found by
/// `graft::it::hint_bar::the_changes_pane_hints_the_keys_that_write`).
#[test]
fn the_divider_hint_is_fourth_so_the_six_key_tier_reaches_it() {
    for context in [
        HintContext::PlainRow,
        HintContext::BranchRow { current: true },
        HintContext::WorkingTreeRow,
        HintContext::ChangesPane { unmerged: false },
    ] {
        let hints = hints_for(&context, Divider::Shown, 120);
        assert_eq!(
            hints.get(3).map(|hint| hint.key),
            Some("<>"),
            "{context:?}: {:?}",
            hints_text(&hints)
        );
    }
}

/// A pane keeps to its own vocabulary: the changes pane hints the keys it
/// answers and does not spill the graph's ([`TAIL`]) into a pane that does
/// not answer them.
#[test]
fn a_pane_hints_only_its_own_keys() {
    let pane = hints_text(&hints_for(
        &HintContext::ChangesPane { unmerged: false },
        Divider::Hidden,
        200,
    ));
    assert_eq!(
        pane,
        " space stage  d discard  c commit  ⌃A all  e edit  ⇥ graph  . more"
    );
    let unmerged = hints_text(&hints_for(
        &HintContext::ChangesPane { unmerged: true },
        Divider::Hidden,
        200,
    ));
    assert_eq!(unmerged, " e edit  c commit  ⇥ graph  . more");
}

/// The banner draws its **own** keys — the whole list, at 80 columns, with
/// no `. more` and no `<> resize`: it is modal, so those two would name
/// keys it swallows (m5/16). `M`/`e` are there only while something is
/// conflicted; with nothing unmerged both refuse in words, and a hint that
/// can only say no is a hint that lies (m5-09 review, Major 2).
#[test]
fn the_banner_draws_its_own_keys_and_no_doorway() {
    assert_eq!(
        hints_text(&hints_for(
            &HintContext::Banner {
                skippable: true,
                conflicted: true,
            },
            Divider::Shown,
            80,
        )),
        " c continue  s skip  a abort  M mergetool  e edit  ⇥ graph"
    );
    assert_eq!(
        hints_text(&hints_for(
            &HintContext::Banner {
                skippable: false,
                conflicted: true,
            },
            Divider::Shown,
            80,
        )),
        " c continue  a abort  M mergetool  e edit  ⇥ graph"
    );
    assert_eq!(
        hints_text(&hints_for(
            &HintContext::Banner {
                skippable: true,
                conflicted: false,
            },
            Divider::Shown,
            80,
        )),
        " c continue  s skip  a abort  ⇥ graph"
    );
}

/// Every non-`Preview`, non-`Marked` context list is drawn in the order the
/// design note's table gives it — a change there is a spec change, not a
/// refactor.
#[test]
fn context_lists_match_the_design_note_order() {
    let working_tree = hints_for(&HintContext::WorkingTreeRow, Divider::Hidden, 120);
    let text = hints_text(&working_tree);
    assert_eq!(
        text,
        " ⏎ changes  c commit  S stash  p pull  P push  f fetch  . more"
    );
}
