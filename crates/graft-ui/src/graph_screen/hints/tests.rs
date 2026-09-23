use unicode_width::UnicodeWidthStr;

use super::*;

/// Every context the graph footer can be in, `Preview` and `Resolver` aside.
///
/// `Preview` draws its own footer by design — the one context allowed an
/// empty list on purpose. `Resolver` is empty for a different reason, and
/// only in this build: every entry `resolver_list` draws is
/// `HintState::Planned` (`HintContext::Resolver`'s own doc comment) — this
/// group builds the widget and the table, not the app-side key handling
/// `app-conflict-resolver` (G4) wires it to, so there is nothing live to
/// draw yet. `no_planned_hint_is_ever_drawn` still holds every one of those
/// entries to the same "never drawn" rule this sweep checks for everyone
/// else; this exemption is what keeps that true without also claiming a bar
/// nobody can act on. G4 removes `Resolver` from this filter in the same
/// commit that flips the table to `Live`.
///
/// Taken from `hints::every_context` rather than written out again: this
/// file's own copy of the list missed `Dragging` for two tickets (m5/31).
fn all_contexts() -> Vec<HintContext> {
    every_context()
        .into_iter()
        .filter(|context| {
            *context != HintContext::Preview && !matches!(context, HintContext::Resolver { .. })
        })
        .collect()
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
/// room for, the `. more` doorway leading the bare keys. `m merge` and
/// `r rebase` joined the bound
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
            // `b branch` and `t tag` went live in
            // m5-history-and-ghost-preview/10, in the places the hint-bar
            // design note had already reserved for them; `c commit` moved
            // down a rung to make room.
            [("⏎", "open"), ("b", "branch"), ("r", "rebase")],
        ),
    ];
    for (context, first_three) in cases {
        let hints = hints_for(context, Divider::Hidden, 80);
        assert!(hints.len() >= 4, "{context:?} at 80 columns: {hints:?}");
        for (got, want) in hints.iter().zip(first_three.iter()) {
            assert_eq!((got.key, got.verb), *want, "{context:?}: {hints:?}");
        }
        assert_doorway_leads_the_bare_keys(&hints, &format!("{context:?} at 80 columns"));
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
        let drawn = without_trailer(&hints);
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
        assert_doorway_leads_the_bare_keys(&hints, &format!("{context:?} at 160 columns"));
        let text = hints_text(&hints);
        assert!(text.width() <= 160, "{context:?}: {text:?} does not fit");
        // `q` is reached at 160 in every context. Whether it is reached at
        // its full rung (`q quit`) or as a bare key depends on how much the
        // context has of its own to say: a branch row, which since
        // m5-history-and-ghost-preview/10 offers `b branch` and `t tag` as
        // well, spends its full rungs on those and draws `q` bare.
        assert!(
            text.contains(" q "),
            "{context:?} still hides `q` at 160: {text:?}"
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
/// The canary used to be `m merge`, then `b branch`, then `M mergetool`,
/// then `B base`; all have since been bound (the last two by
/// m6-conflicts/05), and the table holds no `Planned` row today. The sweep
/// below still guards the rule for the next key a design names ahead of its
/// group.
#[test]
fn no_planned_hint_is_ever_drawn() {
    // Positive control: the check the sweep uses rejects a list holding a
    // `Planned` hint, so this test can still fail even though no table row
    // is `Planned` today. (The drop itself is `drawable`'s filter.)
    let canary = [hint("x", "live"), planned("y", "unbuilt")];
    assert!(
        only_live(&canary).is_err(),
        "the check must reject a Planned hint"
    );

    for context in all_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in [40, 80, 100, 120, 160, 200] {
                if let Err(unbuilt) = only_live(&hints_for(&context, divider, columns)) {
                    panic!("{context:?} at {columns} draws the unbuilt `{unbuilt}`");
                }
            }
        }
    }
}

/// `Err` naming the first drawn hint that is not [`HintState::Live`].
fn only_live(drawn: &[Hint]) -> Result<(), String> {
    match drawn.iter().find(|hint| hint.state != HintState::Live) {
        Some(hint) => Err(format!("{} {}", hint.key, hint.verb)),
        None => Ok(()),
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
        // Fifth, not fourth, on a graph row since `b branch` and `t tag`
        // went live (m5-history-and-ghost-preview/10): `drawable` still
        // inserts it fourth, and the doorway `. more` is then promoted in
        // front of it to lead the tail. What the rule is for is that the
        // hint is *reached* at the tier a pane and a graph first fit side
        // by side, and it still is.
        let at = hints
            .iter()
            .position(|hint| hint.key == "<>")
            .unwrap_or_else(|| panic!("{context:?} never draws `<>`: {:?}", hints_text(&hints)));
        assert!(
            at <= 4,
            "{context:?} buried `<>` at {at}: {:?}",
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
    assert_eq!(
        unmerged,
        " ⏎ resolve  M mergetool  e edit  space resolved  c commit  ⇥ graph  . more"
    );
}

/// The banner draws its **own** keys — the whole list, at 80 columns, with
/// no `. more` and no `<> resize`: it is modal, so those two would name
/// keys it swallows (m5/16). `M`/`e` are there only while something is
/// conflicted, and so is `⏎ resolve` (m6-conflicts/04); with nothing
/// unmerged all three refuse in words, and a hint that can only say no is a
/// hint that lies (m5-09 review, Major 2).
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
        " ⏎ resolve  c continue  s skip  a abort  M mergetool  e edit  ⇥ graph"
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
        " ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph"
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

/// **Gap row 553.** The uncommitted-changes row is no merge, so `space`
/// folds nothing there and the bar must not offer it. A commit row keeps
/// it — the key is dropped per row, not from [`TAIL`].
#[test]
fn uncommitted_row_has_no_fold() {
    let working_tree = hints_for(&HintContext::WorkingTreeRow, Divider::Hidden, 1000);
    assert!(
        !working_tree.iter().any(|hint| hint.key == "space"),
        "{:?}",
        hints_text(&working_tree)
    );
    let plain = hints_for(&HintContext::PlainRow, Divider::Hidden, 1000);
    assert!(
        plain
            .iter()
            .any(|hint| hint.key == "space" && hint.verb == "fold"),
        "{:?}",
        hints_text(&plain)
    );
}

// --- The shrink ladder (m5/24, ADR-0039's third amendment) ---

/// Every cell of the short-form table is the user's own word, spelled as he
/// restated it on 2026-09-21 (m5/27), and **nothing else has a short rung**:
/// thirteen rows gone (four squeezes, two suffixes, seven substitutions),
/// nine remain, and every one of the nine is a prefix of its full verb.
///
/// If a later pass reintroduces a squeeze or a substitution — `stsh` for
/// `stash`, or a fresh `find`/`tool`/`stop` for some other verb — this goes
/// red.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the user's table plus the full deleted/never-had list, not logic"
)]
fn every_short_rung_is_the_users_own_word() {
    let table = [
        ("first-parent", "first"),
        ("continue", "cont"),
        ("discard", "disc"),
        ("hide stash", "hide"),
        ("overview", "over"),
        ("renderer", "rend"),
        ("status", "stat"),
        ("revert", "rev"),
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
    // The thirteen deleted rows — four squeezes, two suffixes, seven
    // substitutions — keep their whole word, alongside the ones that were
    // always absent.
    for verb in [
        "stash",
        "merge",
        "rebase",
        "branch",
        "tag",
        "resolved",
        "open",
        "push",
        "pull",
        "fetch",
        "focus",
        "undo",
        "more",
        "quit",
        "move",
        "fold",
        "edit",
        "skip",
        "sha",
        "map",
        "pop",
        "drop",
        "all",
        "compare",
        "commit",
        "stage",
        "graph",
        "mergetool",
        "resize",
        "journal",
        "explain",
        "search",
        "changes",
        "abort",
        "apply",
        "close",
    ] {
        assert_eq!(short_form(verb), None, "`{verb}` must keep its whole word");
    }
}

/// **The short form is a prefix of the full label, or there is no short
/// form.** Stated so a future row that squeezes or substitutes fails here
/// rather than being reviewed: for every verb the table names, `verb`
/// starts with `short_form(verb)`.
#[test]
fn every_short_form_is_a_prefix_of_its_verb() {
    for hint in every_hint() {
        if let Some(short) = short_form(hint.verb) {
            assert!(
                hint.verb.starts_with(short),
                "`{short}` is not a prefix of `{}` — a squeeze or a substitution",
                hint.verb
            );
        }
    }
}

/// The banner defect this table restatement fixes: `a stop` (for `abort`)
/// and `M tool` (for `mergetool`) broke `docs/design/02-keymap.md:45`'s
/// "match the buttons' initials exactly", and `rungs()` chose them with no
/// context test — so they were reachable, not theoretical, wherever the
/// banner degraded to its short rung. Swept over every banner shape and
/// width; must never draw either substitution again.
#[test]
fn the_banner_never_draws_a_stop_or_m_tool() {
    let banners = [
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
    ];
    for context in banners {
        for columns in 0..=240u16 {
            let text = hints_text(&hints_for(&context, Divider::Hidden, columns));
            assert!(
                !text.contains("a stop"),
                "{context:?} at {columns}: {text:?}"
            );
            assert!(
                !text.contains("M tool"),
                "{context:?} at {columns}: {text:?}"
            );
        }
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
/// where it is shorter) are always drawn and never fall below their **floor
/// rung** — their short rung where the user's table names one, their full
/// verb where it does not (`Hint::floor_rung`, m5/27) — and the bar is
/// never empty. Swept 0 to 240 columns.
///
/// The 20..=240 half of this sweep used to be a second test beside it,
/// word for word; deleted in m5/31 (it was a strict subset).
#[test]
fn the_three_most_relevant_never_fall_below_their_floor_rung() {
    for context in all_contexts() {
        if context == HintContext::Preview {
            continue;
        }
        let whole = whole_list(&context, Divider::Hidden);
        let floor = whole.len().min(3);
        for columns in 0..=240u16 {
            let hints = hints_for(&context, Divider::Hidden, columns);
            if is_the_bottom_of_the_ladder(&hints, doorway_of(&context)) {
                continue;
            }
            let body = without_trailer(&hints);
            assert!(
                body.len() >= floor,
                "{context:?} at {columns} columns drew {} of its floor {floor}: {hints:?}",
                body.len()
            );
            for (drawn, want) in body.iter().zip(whole.iter()).take(floor) {
                assert!(
                    drawn.rung >= want.floor_rung(),
                    "{context:?} at {columns} columns put `{}` below its floor rung",
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
                let mut with_one_more = body.clone();
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
                // Without `. more`: it moves along the row as hints are
                // added, and comparing by index would read that move as a
                // reorder.
                let wider = without_trailer(&drawn);
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
/// rule sees it — `whole_bar`'s own answer, so a context the bar filters
/// (a focused pane drops five keys) cannot read as the unfiltered list.
fn whole_list(context: &HintContext, divider: Divider) -> Vec<Hint> {
    super::whole_bar(context, divider).0
}

/// The drawn hints without the `. more` doorway — the one part of the bar
/// the ladder does not touch.
///
/// Taken out wherever it sits rather than off the end: since m5/28 the
/// doorway leads the bare keys, so on a graph row it is in the middle of the
/// row and on a pane row it is still last (ADR-0039).
fn without_trailer(hints: &[Hint]) -> Vec<Hint> {
    hints
        .iter()
        .filter(|hint| (hint.key, hint.verb) != (MORE.key, MORE.verb))
        .copied()
        .collect()
}

/// **The doorway sits at the head of the bare keys — which is the end of
/// the row when there are none** (ADR-0039, m5/28).
///
/// Stated as one invariant over the drawn row, because it is one rule: no
/// bare key ahead of the doorway, nothing but bare keys behind it. A row
/// with a tail of bare keys puts `. more` in front of that tail; a row with
/// none leaves it last, where it always was.
fn assert_doorway_leads_the_bare_keys(hints: &[Hint], context: &str) {
    let at = hints
        .iter()
        .position(|hint| (hint.key, hint.verb) == (MORE.key, MORE.verb))
        .unwrap_or_else(|| panic!("{context}: the doorway is never dropped: {hints:?}"));
    assert!(
        hints[..at].iter().all(|hint| hint.rung != Rung::Key),
        "{context}: a bare key sits ahead of the doorway: {}",
        hints_text(hints)
    );
    assert!(
        hints[at + 1..].iter().all(|hint| hint.rung == Rung::Key),
        "{context}: a labelled hint sits behind the doorway: {}",
        hints_text(hints)
    );
}

/// What the bar is allowed to be when its floor does not fit: exactly the
/// doorway, exactly the doorway's bare key, or nothing at all.
///
/// ADR-0039's bottom, stated completely (m5/28). **Never a clipped prefix
/// of any of them** — the doorway is the one element meant to be reliable,
/// and `. mo` is worse than an empty bar because nobody chose where it cut.
fn is_the_bottom_of_the_ladder(hints: &[Hint], doorway: Option<Hint>) -> bool {
    match hints {
        [] => true,
        [only] => doorway.is_some_and(|door| (only.key, only.verb) == (door.key, door.verb)),
        _ => false,
    }
}

/// The doorway `context` has, if any: `. more` for everything but a marked
/// row (whose `.` is already on screen as `. explain`), the focused banner,
/// a drag in the air and the two overlays — modal in the way that matters,
/// so `.` leads nowhere and is not drawn (ADR-0039's first amendment).
///
/// Not `whole_bar`'s trailer: a marked row's `. explain` leads the *list*
/// rather than trailing it, and this is the hint the tests look for on the
/// drawn row either way. Written out arm by arm rather than with a `_` —
/// the wildcard is how it came to claim a doorway for `Dragging` that the
/// bar never draws (m5/31).
fn doorway_of(context: &HintContext) -> Option<Hint> {
    match context {
        HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Resolver { .. } => None,
        HintContext::Marked(_) => Some(EXPLAIN),
        // A filter keeps the doorway of the context it filters: `. more`
        // still leads to everything, the dropped `⏎` included.
        HintContext::PaneFocused { row, .. } => doorway_of(row),
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow
        | HintContext::ChangesPane { .. } => Some(MORE),
    }
}

/// **One rule, two layouts, no special case** (ADR-0039, m5/28): the
/// doorway sits at the head of the bare keys — which is the end of the row
/// when there are none.
///
/// Asserted in one test because the point is that one rule produces both.
/// A graph row at 80 cells has a tail of bare keys and `. more` leads it; the
/// changes pane and the journal browser draw every hint whole at the same
/// width, and `. more` stays last, where it always was.
#[test]
fn one_rule_puts_the_doorway_before_a_tail_and_last_without_one() {
    let with_a_tail = hints_for(
        &HintContext::BranchRow { current: true },
        Divider::Hidden,
        80,
    );
    let text = hints_text(&with_a_tail);
    assert_eq!(
        text, " ⏎ open  c commit  P push  . more  p  f  b  t  u  S  J  /  q  F2  F3  x  +  j/k",
        "a row context has a tail, so the doorway leads it"
    );
    assert!(
        !text.ends_with(". more"),
        "the doorway is no longer last where a tail exists: {text:?}"
    );

    for context in [
        HintContext::ChangesPane { unmerged: false },
        HintContext::Journal,
    ] {
        let hints = hints_for(&context, Divider::Hidden, 80);
        assert!(
            hints.iter().all(|hint| hint.rung != Rung::Key),
            "{context:?} at 80 cells has no tail to lead: {:?}",
            hints_text(&hints)
        );
        let text = hints_text(&hints);
        // The journal draws no doorway at all — `.` opens nothing over an
        // overlay, which the key sweep found and ADR-0039's first amendment
        // already covers (m5/31). Where there *is* one and no tail to lead,
        // it stays last, where it always was.
        if doorway_of(&context).is_some() {
            assert!(
                text.ends_with(". more"),
                "{context:?} has no bare keys, so the doorway stays last: {text:?}"
            );
        } else {
            assert!(
                !text.contains(". more"),
                "{context:?} is modal: `.` leads nowhere, so it is not drawn: {text:?}"
            );
        }
    }

    // And the invariant behind both, over every context that has a `. more`
    // to place, at every width. A marked row's doorway is `. explain`, which
    // the caller puts at the head of the list itself and the ladder never
    // moves; the focused banner has none at all.
    for context in all_contexts() {
        if doorway_of(&context) != Some(MORE) {
            continue;
        }
        for columns in 1..=240u16 {
            let hints = hints_for(&context, Divider::Hidden, columns);
            if is_the_bottom_of_the_ladder(&hints, doorway_of(&context)) {
                continue;
            }
            assert_doorway_leads_the_bare_keys(&hints, &format!("{context:?} at {columns}"));
        }
    }
}

/// **The bar never draws a fragment of itself**, at any width from 1 to 240.
///
/// Two halves of one property. It never asks for more cells than it has, so
/// the footer never cuts it; and where its floor does not fit it is exactly
/// the doorway, exactly the doorway's bare key, or empty — never a clipped
/// prefix of any of them (ADR-0039's bottom, m5/28).
///
/// **Mutation that produces `. mo`:** drop the rung loop in
/// `below_the_floor` and return `vec![doorway]` unconditionally. At four
/// cells of bar the row is then ` . more`, seven cells asked of four, and
/// the footer's own `fit` cuts it to `. m`. This fails naming the width.
#[test]
fn the_bar_never_draws_a_fragment_of_itself() {
    for context in all_contexts() {
        for divider in [Divider::Shown, Divider::Hidden] {
            for columns in 1..=240u16 {
                let hints = hints_for(&context, divider, columns);
                let text = hints_text(&hints);
                assert!(
                    text.width() <= usize::from(columns),
                    "{context:?} with {divider:?} asked for {} cells of a {columns}-cell bar, \
                     so the footer has to cut it: {text:?}",
                    text.width()
                );
                if hints.is_empty() {
                    continue;
                }
                let whole = whole_list(&context, divider);
                let floor = whole.len().min(3);
                let body = without_trailer(&hints);
                assert!(
                    body.len() >= floor
                        || is_the_bottom_of_the_ladder(&hints, doorway_of(&context)),
                    "{context:?} with {divider:?} at {columns} columns drew {} of its floor \
                     {floor} and is not the bottom of the ladder either: {text:?}",
                    body.len()
                );
            }
        }
    }
}

/// The floor fits at 48 cells of bar — the narrowest bar anything reachable
/// actually has.
///
/// `graft_ui::detail_pane::layout`'s `RIGHT_MIN = 48` clamps the graph side
/// of a side-by-side split, and below 100 columns the pane is a full-width
/// tab instead, so no divider drag leaves the bar narrower than this. The
/// 20-cell overflow the m5/27 sweep found is below anything a user can
/// reach — this is the number that matters, and it is asserted rather than
/// assumed (m5/28).
#[test]
fn the_floor_fits_at_forty_eight_cells_of_bar() {
    for context in all_contexts() {
        if context == HintContext::Preview {
            continue;
        }
        let whole = whole_list(&context, Divider::Hidden);
        let floor = whole.len().min(3);
        let hints = hints_for(&context, Divider::Hidden, 48);
        let body = without_trailer(&hints);
        assert!(
            body.len() >= floor,
            "{context:?} cannot draw its floor of {floor} in 48 cells — that is a live \
             defect, not a completeness gap: {:?}",
            hints_text(&hints)
        );
    }
}

// --- The resolver's table (M6, `docs/design-notes/m6-conflicts.md`) ---

/// `every_context()` names all four `base`/`restart` combinations —
/// [`super::every_context_reaches_every_shape`]-style coverage would be
/// overkill for two booleans, so this states it directly.
#[test]
fn every_context_lists_resolver() {
    let contexts = every_context();
    for base in [false, true] {
        for restart in [false, true] {
            assert!(
                contexts.contains(&HintContext::Resolver {
                    base,
                    restart,
                    whole_file: false
                }),
                "every_context() is missing Resolver {{ base: {base}, restart: {restart} }}"
            );
        }
    }
}

/// A resolver open on a document, with `B`/`d` under the two flags.
fn document(base: bool, restart: bool) -> HintContext {
    HintContext::Resolver {
        base,
        restart,
        whole_file: false,
    }
}

/// The resolver's own table — `context_list`, **before** the `Live` filter
/// [`drawable`] applies — reads exactly as the design note's footer
/// mockups, `B base`/`d start over` only under their flag, and answers
/// [`ladder`]'s width sweep the same way any other ~9-key list would.
///
/// This checks the *table*, not the bar. Since m6-conflicts/04 every entry
/// here is `Live` except `B base` and `d start over`, which are group 05's
/// and stay `Planned` — so `drawable` draws the nine and hides those two.
#[test]
fn resolver_hint_ladder() {
    let list = context_list(&document(false, false));
    let words: Vec<(&str, &str)> = list.iter().map(|hint| (hint.key, hint.verb)).collect();
    assert_eq!(
        words,
        vec![
            ("o", "ours"),
            ("t", "theirs"),
            ("b", "both"),
            ("e", "edit"),
            ("n", "next"),
            ("N", "prev"),
            ("⏎", "resolved"),
            ("M", "mergetool"),
            ("esc", "close"),
        ]
    );
    assert!(
        list.iter().all(|hint| hint.state == HintState::Live),
        "every key the resolver's own list draws is bound (m6-conflicts/04): {list:?}"
    );

    let with_base = context_list(&document(true, false));
    let base_at = with_base
        .iter()
        .position(|hint| (hint.key, hint.verb) == ("B", "base"))
        .expect("`B base` after `b both` while `base`");
    assert_eq!(with_base[base_at - 1].key, "b", "`B base` follows `b both`");

    let with_restart = context_list(&document(false, true));
    let restart_at = with_restart
        .iter()
        .position(|hint| (hint.key, hint.verb) == ("d", "start over"))
        .expect("`d start over` before `esc close` while `restart`");
    assert_eq!(
        with_restart[restart_at + 1].key,
        "esc",
        "`d start over` precedes `esc close`"
    );

    // The width ladder: drawn through `hints_text`/`fit_list` directly (not
    // `hints_for`, which would run the `Live` filter and zero the list) —
    // the ladder's own mechanics do not know or care about `HintState`, so
    // this exercises exactly the code `hints_for` would once the table
    // flips.
    let whole = list.clone();
    let at_160 = fit_list(&whole, None, 160);
    assert_eq!(
        hints_text(&at_160),
        " o ours  t theirs  b both  e edit  n next  N prev  ⏎ resolved  M mergetool  esc close"
    );
    // Observed (cargo test -p graft-ui --lib graph_screen::hints::tests::resolver_hint_ladder
    // -- --nocapture, with a temporary eprintln probe): the 160-column line is
    // 85 cells; at 80 columns the ladder only needs to drop the least relevant
    // word — `close` off the tail `esc close` — to reach 79 cells. Every other
    // hint, `N prev` and `M mergetool` included, keeps its verb at 80 columns;
    // the floor-of-3-relevant-hints mechanics never engage here because the
    // full list already nearly fits.
    let at_80 = fit_list(&whole, None, 80);
    let text_80 = hints_text(&at_80);
    assert_eq!(
        text_80,
        " o ours  t theirs  b both  e edit  n next  N prev  ⏎ resolved  M mergetool  esc"
    );
    assert!(text_80.width() <= 80, "{text_80:?}");
    // Nothing is ever cut to `. more` — the resolver is modal (§2.1), so
    // there is no doorway to lead a dropped tail to.
    assert!(!text_80.contains("more"), "{text_80:?}");
}

/// A whole-file card draws only the keys a card answers (m6-05 review,
/// Major 1): no `b`, `N`, `⏎`, `B` or `d`, whatever `base`/`restart` say,
/// and it is one of the contexts every sweep reads.
#[test]
fn resolver_card_draws_only_what_a_card_answers() {
    for (base, restart) in [(false, false), (true, true)] {
        let card = HintContext::Resolver {
            base,
            restart,
            whole_file: true,
        };
        assert_eq!(
            hints_text(&hints_for(&card, Divider::Hidden, 160)),
            " o ours  t theirs  e edit  n next  M mergetool  esc close"
        );
    }
    assert!(every_context().contains(&HintContext::Resolver {
        base: false,
        restart: false,
        whole_file: true,
    }));
}

/// `⏎ resolve` leads `ChangesPane { unmerged: true }` and `Banner {
/// conflicted: true }` (Proposed change 2a) — pinned by exact string on
/// `context_list`, before the `Live` filter, since both entries stay
/// `Planned` in this build (`ChangesPane`'s `M mergetool`/`space resolved`,
/// `Banner`'s new `⏎ resolve`) and so draw nothing for `hints_for` to check
/// (`every_planned_hint_is_still_unbound`, `graft`'s
/// `every_key_the_bar_draws_is_answered`, cover that half).
///
/// Wording is what the code emits, not a re-derivation from the design
/// note's prose: `⇥` here is `graph` (the existing verb, unchanged by this
/// group), not the note's own paraphrase.
#[test]
fn unmerged_rows_lead_with_resolve() {
    let unmerged = context_list(&HintContext::ChangesPane { unmerged: true });
    assert_eq!(
        hints_text(&unmerged),
        " ⏎ resolve  M mergetool  e edit  space resolved  c commit  ⇥ graph"
    );

    let banner_plain = context_list(&HintContext::Banner {
        skippable: false,
        conflicted: true,
    });
    assert_eq!(
        hints_text(&banner_plain),
        " ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph"
    );

    let banner_skippable = context_list(&HintContext::Banner {
        skippable: true,
        conflicted: true,
    });
    assert_eq!(
        hints_text(&banner_skippable),
        " ⏎ resolve  c continue  s skip  a abort  M mergetool  e edit  ⇥ graph"
    );

    // `conflicted: false` drops `⏎ resolve` along with `M`/`e`: nothing is
    // unmerged, so the whole conflict-resolution group refuses in words.
    let banner_no_conflict = context_list(&HintContext::Banner {
        skippable: true,
        conflicted: false,
    });
    assert!(
        !hints_text(&banner_no_conflict).contains("resolve"),
        "{:?}",
        hints_text(&banner_no_conflict)
    );
}
