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

/// (b) At 80 columns the row contexts lead with their first three *bound*
/// hints, in the note's table order — the planned entries (`b branch`,
/// `t tag`) are not among them — and then take as many more as the row has
/// room for, ending in `. more`. `m merge` and `r rebase` joined the bound
/// ones when ticket 25 bound them (see `no_planned_hint_is_ever_drawn`), so
/// a branch row now leads with the two verbs the design note put first.
///
/// It used to be exactly three plus `. more` whatever else fitted: a tier
/// keyed to width, not a fit. ADR-0037 §4's three is a floor (ADR-0039,
/// 2026-09-21 amendment).
#[test]
fn eighty_columns_leads_with_three_and_fills_the_row() {
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
            [("⏎", "open"), ("r", "rebase"), ("c", "commit")],
        ),
    ];
    for (context, first_three) in cases {
        let hints = hints_for(context, Divider::Hidden, 80);
        assert!(hints.len() >= 4, "{context:?} at 80 columns: {hints:?}");
        for (got, want) in hints.iter().zip(first_three.iter()) {
            assert_eq!((got.key, got.verb), *want, "{context:?}: {hints:?}");
        }
        assert_eq!(hints.last().copied(), Some(MORE), "{context:?}: {hints:?}");
        assert!(
            hints_text(&hints).width() <= 80,
            "{context:?}: {:?}",
            hints_text(&hints)
        );
    }
}

/// (c) A wider row takes more hints than a narrower one, and every one of
/// them fits: the bar's answer is a fit, not a step.
#[test]
fn a_wider_row_takes_more_hints_and_still_fits() {
    for context in row_contexts() {
        let narrow = hints_for(&context, Divider::Hidden, 80);
        let wide = hints_for(&context, Divider::Hidden, 120);
        assert!(
            wide.len() >= narrow.len(),
            "{context:?}: 120 drew {wide:?}, 80 drew {narrow:?}"
        );
        assert!(
            hints_text(&wide).width() <= 120,
            "{context:?}: {:?}",
            hints_text(&wide)
        );
        // And the extra room bought something: the 80-cell row could not
        // hold the whole list, so 120 must hold strictly more of it.
        if hints_text(&drawable(&context, Divider::Hidden)).width() > 80 {
            assert!(
                wide.len() > narrow.len(),
                "{context:?}: 120 cells bought no extra hint: {wide:?}"
            );
        }
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
        // By key and verb, not by whole `Hint`: the ladder leaves the tail
        // on lower rungs, and the order is what this test is about.
        assert_eq!(
            drawn.iter().map(|h| (h.key, h.verb)).collect::<Vec<_>>(),
            own[..drawn.len()]
                .iter()
                .map(|h| (h.key, h.verb))
                .collect::<Vec<_>>(),
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
        assert_eq!((hints[0].key, hints[0].verb), (EXPLAIN.key, EXPLAIN.verb));
        assert!(
            !hints
                .iter()
                .any(|h| (h.key, h.verb) == (MORE.key, MORE.verb)),
            "marked row must not also show `. more`: {hints:?}"
        );
        let plain = drawable(&HintContext::PlainRow, Divider::Hidden);
        // Key and verb only: the ladder varies the rung, not the order.
        assert_eq!(
            hints[1..]
                .iter()
                .map(|h| (h.key, h.verb))
                .collect::<Vec<_>>(),
            plain[..hints.len() - 1]
                .iter()
                .map(|h| (h.key, h.verb))
                .collect::<Vec<_>>(),
            "{hints:?}"
        );
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
///
/// The canary used to be `m merge`. M5 ticket 25 bound `m` and `r` to the
/// Ghost Preview (`graft::update::preview::is_history_key`) and the table
/// went on calling them planned, so two working keys were advertised
/// nowhere — the converse of `docs/design/02-keymap.md` §2.1's rule, and
/// the reason they are live here now. `b branch` carries the canary.
#[test]
fn no_planned_hint_is_ever_drawn() {
    let planned: Vec<Hint> = every_hint()
        .into_iter()
        .filter(|hint| hint.state == HintState::Planned)
        .collect();
    assert!(
        planned
            .iter()
            .any(|hint| (hint.key, hint.verb) == ("b", "branch")),
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
    let working_tree = hints_for(&HintContext::WorkingTreeRow, Divider::Hidden, 240);
    let text = hints_text(&working_tree);
    assert!(
        text.starts_with(" ⏎ changes  c commit  S stash  p pull  P push  f fetch"),
        "{text:?}"
    );
}

// --- The shrink ladder (m5/24, ADR-0039's third amendment) ---

/// Every cell of the short-form table is the user's own word, spelled as he
/// approved it on 2026-09-21, and **nothing else has a short rung**.
///
/// The second half is the point: `stash` is the label that bites at 80
/// columns and he ruled against shortening it, because every other hint is a
/// whole word and one consonant-squeeze teaches the reader that labels here
/// can be mangled. If a later pass invents `stsh`, or gives `merge` a short
/// form before the group that binds it asks, this goes red.
#[test]
fn every_short_rung_is_the_users_own_word() {
    let table = [
        ("first-parent", "first"),
        ("continue", "cont"),
        ("mergetool", "tool"),
        ("discard", "disc"),
        ("hide stash", "hide"),
        ("journal", "log"),
        ("overview", "over"),
        ("renderer", "rend"),
        ("explain", "why"),
        ("search", "find"),
        ("changes", "diff"),
        ("status", "stat"),
        ("compare", "cmp"),
        ("resize", "size"),
        ("commit", "cmt"),
        ("revert", "rev"),
        ("stage", "stg"),
        ("abort", "stop"),
        ("apply", "use"),
        ("graph", "grph"),
        ("close", "shut"),
        ("lanes", "lane"),
    ];
    for (verb, want) in table {
        assert_eq!(short_form(verb), Some(want), "`{verb}`'s short rung");
    }
    // No verb the hint tables use has a short form the user did not write.
    for hint in every_hint() {
        if let Some(short) = short_form(hint.verb) {
            assert!(
                table.contains(&(hint.verb, short)),
                "`{} {}` has a short rung `{short}` the user's table does not give it",
                hint.key,
                hint.verb
            );
        }
    }
    // And the ones he deliberately left out keep their whole word.
    for verb in [
        "stash", "merge", "rebase", "branch", "tag", "resolved", "open", "push", "pull", "fetch",
        "focus", "undo", "more", "quit", "move", "fold", "edit", "skip", "sha", "map", "pop",
        "drop", "all",
    ] {
        assert_eq!(short_form(verb), None, "`{verb}` must keep its whole word");
    }
}

/// The bare keys are a **tail**, not a scatter: once the bar has put one
/// hint on its key alone, every hint after it is on its key alone too.
/// Swept over every context, both dividers, every width from 0 to 240.
///
/// This is the shape of ADR-0039's third amendment — the bar fills from the
/// most relevant end, so what runs out runs out at the far end. A bar that
/// read `⏎ open  c  y sha  u  S stash` would be the width rule showing
/// through instead of a priority order.
#[test]
fn the_bare_keys_are_a_tail_not_a_scatter() {
    for context in all_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in 0..=240u16 {
                let hints = hints_for(&context, divider, columns);
                let body = without_trailer(&hints);
                let first_key = body.iter().position(|hint| hint.rung == Rung::Key);
                if let Some(first_key) = first_key {
                    for hint in &body[first_key..] {
                        assert!(
                            hint.rung == Rung::Key,
                            "{context:?} at {columns} columns draws `{}` after a bare key: {:?}",
                            hint.text(),
                            hints_text(&hints)
                        );
                    }
                }
            }
        }
    }
}

/// "Each hint takes the best rung that still leaves room for **the hints
/// ahead of it**" (ADR-0039, third amendment), stated so it can go red: past
/// the three most relevant, a hint rises above its bare key only once every
/// hint in the context is drawn. Room spent on a fourth verb is room the
/// twentieth key has not had yet.
///
/// This is the clause the reverted rule got backwards, and the one a plain
/// left-to-right greedy would break: greedy hands hint four its whole verb
/// while hints five to twenty-two are not on the bar at all.
#[test]
fn a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn() {
    for context in all_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let whole = whole_list(&context, divider);
            for columns in 0..=240u16 {
                let hints = hints_for(&context, divider, columns);
                let body = without_trailer(&hints);
                let raised = body
                    .iter()
                    .skip(3)
                    .find(|hint| hint.rung != Rung::Key)
                    .map(|hint| hint.key);
                if let Some(key) = raised {
                    assert_eq!(
                        body.len(),
                        whole.len(),
                        "{context:?} at {columns} columns gave `{key}` a verb with \
                         {} of its {} hints drawn: {:?}",
                        body.len(),
                        whole.len(),
                        hints_text(&hints)
                    );
                }
            }
        }
    }
}

/// The picture the ladder exists to produce, and the one the list-wide
/// version could not: at 80, 120 **and** 160 columns a row context leads
/// with whole verbs, not with bare keys.
///
/// The reverted rule drew ` ⏎  c  y  u  S  J  …` at all three widths
/// (ADR-0039's third amendment quotes it). If this goes green with bare keys
/// in the lead, the ladder has become list-wide again.
#[test]
fn a_row_context_keeps_whole_verbs_at_the_relevant_end() {
    for context in row_contexts() {
        for columns in [80u16, 120, 160] {
            let hints = hints_for(&context, Divider::Hidden, columns);
            for hint in hints.iter().take(3) {
                assert_eq!(
                    hint.rung,
                    Rung::Full,
                    "{context:?} at {columns} columns: {:?}",
                    hints_text(&hints)
                );
            }
        }
    }
}

/// ADR-0037 §4's floor: the three most relevant hints (or the whole list,
/// where it is shorter) are always drawn and never fall below their short
/// rung — and the bar is never empty. Swept 0 to 240 columns.
#[test]
fn the_three_most_relevant_never_fall_below_their_short_rung() {
    for context in all_contexts() {
        if context == HintContext::Preview {
            continue;
        }
        let whole = whole_list(&context, Divider::Hidden);
        let floor = whole.len().min(3);
        for columns in 0..=240u16 {
            let hints = hints_for(&context, Divider::Hidden, columns);
            let body = without_trailer(&hints);
            assert!(
                body.len() >= floor,
                "{context:?} at {columns} columns drew {} of its floor {floor}: {hints:?}",
                body.len()
            );
            for (drawn, want) in body.iter().zip(whole.iter()).take(floor) {
                assert!(
                    drawn.rung >= want.floor_rung(),
                    "{context:?} at {columns} columns put `{}` below its short rung",
                    drawn.key
                );
            }
        }
    }
}

/// A hint is dropped only when it will not fit **even at its bare key** —
/// the last clause of ADR-0039's third amendment. At every width, adding the
/// next hint the context has, at its key alone, would overflow the bar.
#[test]
fn a_hint_is_dropped_only_when_its_bare_key_will_not_fit() {
    for context in row_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let whole = drawable(&context, divider);
            for columns in 40..=240u16 {
                let hints = hints_for(&context, divider, columns);
                let body = without_trailer(&hints);
                if body.len() == whole.len() {
                    continue;
                }
                let mut with_one_more = body.to_vec();
                with_one_more.push(whole[body.len()].at(Rung::Key));
                with_one_more.push(MORE);
                assert!(
                    hints_text(&with_one_more).width() > usize::from(columns),
                    "{context:?} at {columns} columns dropped `{}` with room for its key: {:?}",
                    whole[body.len()].key,
                    hints_text(&hints)
                );
            }
        }
    }
}

/// Monotone in the bar's own width, rung by rung and hint by hint: a cell
/// more room can add a hint or raise one, never the reverse. Swept over
/// every context, both dividers, 0 to 240 columns.
///
/// The count half of this is
/// `graph_screen::tests::no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`;
/// the rung half is new with the ladder, and is what makes "the bar got
/// wider and said less" impossible in the new dimension the ladder adds.
#[test]
fn no_hint_loses_ground_as_the_room_for_it_grows() {
    for context in all_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let mut narrower: Vec<Hint> = Vec::new();
            for columns in 0..=240u16 {
                let drawn = hints_for(&context, divider, columns);
                // Without `. more`, which is always last: it slides right as
                // hints are added, and comparing by index would read that
                // slide as a reorder.
                let wider = without_trailer(&drawn).to_vec();
                assert!(
                    wider.len() >= narrower.len(),
                    "{context:?} with {divider:?} at {columns} columns lost a hint"
                );
                for (index, before) in narrower.iter().enumerate() {
                    assert_eq!(
                        (wider[index].key, wider[index].verb),
                        (before.key, before.verb),
                        "{context:?} with {divider:?} at {columns} columns reordered"
                    );
                    assert!(
                        wider[index].rung >= before.rung,
                        "{context:?} with {divider:?}: `{}` fell from {:?} to {:?} \
                         when the bar grew to {columns} columns",
                        before.key,
                        before.rung,
                        wider[index].rung
                    );
                }
                narrower = wider;
            }
        }
    }
}

/// Every hint `context` could draw, `. explain` included, before the width
/// rule sees it.
fn whole_list(context: &HintContext, divider: Divider) -> Vec<Hint> {
    if let HintContext::Marked(inner) = context {
        let mut list = vec![EXPLAIN];
        list.extend(drawable(inner, divider));
        list
    } else {
        drawable(context, divider)
    }
}

/// The drawn hints without the `. more` doorway, which is always at its full
/// rung and always last — it is the one part of the bar the ladder does not
/// touch.
fn without_trailer(hints: &[Hint]) -> &[Hint] {
    match hints.split_last() {
        Some((last, rest)) if (last.key, last.verb) == (MORE.key, MORE.verb) => rest,
        _ => hints,
    }
}
