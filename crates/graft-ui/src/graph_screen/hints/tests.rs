use graft_term::SanitizedText;
use unicode_width::UnicodeWidthStr;

use crate::conflict_resolver::SideRoles;
use crate::finish_summary::SummaryFocus;

use super::*;

/// The tests read the bar as the shipped table spells it: these shadow the
/// keymap-taking functions with the default keymap filled in, so each test
/// still says only what it is about.
fn hints_for(context: &HintContext, divider: Divider, columns: u16) -> Vec<Hint> {
    super::hints_for(context, divider, columns, Keymap::shipped())
}

fn hints_text(hints: &[Hint]) -> String {
    super::hints_text(hints, Keymap::shipped())
}

fn stopped_row(reason: &StopWording, hints: &[Hint], columns: u16) -> String {
    super::stopped_row(reason, hints, columns, Keymap::shipped())
}

/// A hint's key as the shipped table spells it. Leaked — a test-only
/// handful of short strings — so `(key, verb)` tuples compare as `&str`.
trait Spelled {
    fn key(&self) -> &'static str;
}

impl Spelled for Hint {
    fn key(&self) -> &'static str {
        let text = self
            .key_text(Keymap::shipped())
            .expect("every table hint names a bound key");
        Box::leak(text.into_boxed_str())
    }
}

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
        HintContext::BranchRow {
            current: false,
            verbs: RowVerbs::NONE,
        },
        HintContext::BranchRow {
            current: true,
            verbs: RowVerbs::NONE,
        },
        HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
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
/// room for, the `? keys` doorway leading the bare keys. `m merge` and
/// `r rebase` joined the bound
/// ones when ticket 25 bound them (see `no_planned_hint_is_ever_drawn`), so
/// a branch row now leads with the two verbs the design note put first.
///
/// It used to be exactly three plus `? keys` whatever else fitted: a tier
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
            HintContext::BranchRow {
                current: false,
                verbs: RowVerbs::NONE,
            },
            [("m", "merge"), ("r", "rebase"), ("⏎", "open")],
        ),
        (
            HintContext::BranchRow {
                current: true,
                verbs: RowVerbs::NONE,
            },
            [("⏎", "open"), ("c", "commit"), ("P", "push")],
        ),
        (
            HintContext::PlainRow {
                verbs: RowVerbs::NONE,
            },
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
            assert_eq!((got.key(), got.verb), *want, "{context:?}: {hints:?}");
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
            drawn.iter().map(|h| (h.key(), h.verb)).collect::<Vec<_>>(),
            own[..drawn.len()]
                .iter()
                .map(|h| (h.key(), h.verb))
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
    let wide = hints_text(&hints_for(
        &HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
        Divider::Hidden,
        200,
    ));
    assert!(wide.contains("x compare"), "{wide:?}");
}

/// (e) A marked row leads with `. explain`, then the row's own list, draws
/// no `. actions` (`.` is already on screen); `? keys` only as a ranked
/// hint (m8/11d R2, `a_marked_row_keeps_…`) — and never overflows the row.
#[test]
fn marked_leads_with_explain_and_draws_no_actions() {
    let marked = HintContext::Marked(Box::new(HintContext::PlainRow {
        verbs: RowVerbs::NONE,
    }));
    for columns in [40, 80, 120, 160, 200] {
        let hints = hints_for(&marked, Divider::Hidden, columns);
        assert_eq!(
            (hints[0].key(), hints[0].verb),
            (EXPLAIN.key(), EXPLAIN.verb)
        );
        assert!(!hints.contains(&ACTIONS), "{hints:?}");
        // `? keys` is a ranked list hint here since m8/11d (R2), drawn last;
        // the rest keep the plain row's order.
        let hints = without_trailer(&hints);
        let plain = drawable(
            &HintContext::PlainRow {
                verbs: RowVerbs::NONE,
            },
            Divider::Hidden,
        );
        // Key and verb only: the ladder varies the rung, not the order —
        // the drawn hints are the plain row's, in its order, though not
        // always a prefix: `u undo` is fitted second (m8/15 A5).
        let drawn: Vec<_> = hints[1..].iter().map(|h| (h.key(), h.verb)).collect();
        let mut plain_keys = plain.iter().map(|h| (h.key(), h.verb));
        assert!(
            drawn
                .iter()
                .all(|want| plain_keys.any(|have| have == *want)),
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
    let canary = [
        hint(hint_keys::COMPARE, "live"),
        planned(hint_keys::COPY_SHA, "unbuilt"),
    ];
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
        Some(hint) => Err(format!("{} {}", hint.key(), hint.verb)),
        None => Ok(()),
    }
}

/// `<> resize` is hinted exactly while a detail pane is open, in every
/// context — a commit's pane has the same divider the changes pane has
/// (ADR-0028, m5-03 review, Major 4).
#[test]
fn the_divider_hint_follows_the_pane_not_the_context() {
    for context in [
        HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
        HintContext::BranchRow {
            current: true,
            verbs: RowVerbs::NONE,
        },
        HintContext::WorkingTreeRow,
        HintContext::ChangesPane {
            unmerged: false,
            live: None,
            tab: TabWord::Graph,
            over_overlay: false,
        },
    ] {
        // Wide enough for the whole list: since R1 resize is the last of a
        // row's keys, so 200 columns no longer reach it on a graph row.
        let with = hints_text(&hints_for(&context, Divider::Shown, 400));
        let without = hints_text(&hints_for(&context, Divider::Hidden, 400));
        assert!(with.contains("</> resize"), "{context:?}: {with:?}");
        assert!(!without.contains("</> resize"), "{context:?}: {without:?}");
    }
}

/// Ruling R1 (ticket m8/10): `</> resize` is the last of the bar's own keys,
/// so it is the first the width drops and nothing else pays for its width.
/// Under 10's first cut it sat fourth, and one cell of its own new spelling
/// (`<>` → `</>`) pushed `u undo` off the bar at 100 columns. The invariant
/// over every context, every width and both dividers: a bar that draws
/// `</> resize` draws every other hint its list holds.
///
/// The bar is monotone in its width by construction — `fit_list` takes the
/// longest prefix of one fixed step order (`fitting::ladder`), so a hint
/// drawn at one width is drawn at every wider one. Checking the narrowest
/// width that draws resize (found by bisection) therefore covers every width
/// above it (walking all 201 widths of every context took a minute).
#[test]
fn resize_is_drawn_only_when_nothing_else_was_dropped() {
    let keys = Keymap::shipped();
    let mut drawn_somewhere = 0;
    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let (list, _door) = spelled_bar(&context, divider, keys);
            if !list.iter().any(|hint| hint.is(hint_keys::RESIZE)) {
                continue;
            }
            let draws_resize = |columns: u16| {
                hints_for(&context, divider, columns)
                    .iter()
                    .any(|hint| hint.is(hint_keys::RESIZE))
            };
            let widths: Vec<u16> = (40..=240).collect();
            let first = widths.partition_point(|columns| !draws_resize(*columns));
            let Some(&columns) = widths.get(first) else {
                continue;
            };
            let hints = hints_for(&context, divider, columns);
            drawn_somewhere += 1;
            // `. actions` ranks after `<> resize` (m8/11b decision 1: last
            // before the doorway), so it alone may be dropped first.
            for wanted in list.iter().filter(|hint| **hint != ACTIONS) {
                assert!(
                    hints
                        .iter()
                        .any(|hint| hint.keys == wanted.keys && hint.verb == wanted.verb),
                    "{context:?} at {columns} draws `</> resize` but drops `{} {}`: {:?}",
                    wanted.key(),
                    wanted.verb,
                    hints_text(&hints)
                );
            }
        }
    }
    assert!(drawn_somewhere > 0, "no bar ever drew `</> resize`");
}

/// Ruling R3 (ticket m8/10): undo visible is a test, not a hope. Each case
/// is the context of a capture whose bar lost a hint to `</> resize`'s
/// wider spelling under 10's first cut; with resize last (R1) each is back.
#[test]
fn the_hints_resize_once_pushed_off_are_drawn_again() {
    let print =
        |context: &HintContext, columns| hints_text(&hints_for(context, Divider::Shown, columns));
    // `m2-duplicate-commit`, `m2-evil-merge`: a marked plain row, 100 columns.
    let marked = HintContext::Marked(Box::new(HintContext::PlainRow {
        verbs: RowVerbs::NONE,
    }));
    let bar = print(&marked, 100);
    assert!(bar.contains("u undo"), "{marked:?} at 100: {bar:?}");
    // `m7/diff-cursor-commit-120x40`, `m7/pane-bar-file-vs-directory-120x40`:
    // the detail pane focused on a file (and on a directory), 120 columns.
    for (opens, blamable, folds) in [
        (false, true, false),
        (true, true, false),
        (true, false, true),
    ] {
        let focused = HintContext::PaneFocused {
            row: Box::new(HintContext::BranchRow {
                current: true,
                verbs: RowVerbs::NONE,
            }),
            opens,
            blamable,
            folds,
            graph_hidden: false,
        };
        let bar = print(&focused, 120);
        assert!(bar.contains("/ search"), "{focused:?} at 120: {bar:?}");
    }
    // `m7/rewrite-editor-100x30`: the narrow editor keeps `d drop` at 100.
    let narrow = HintContext::RewriteEditor {
        narrow: true,
        can_start: true,
    };
    let bar = hints_text(&hints_for(&narrow, Divider::Hidden, 100));
    assert!(bar.contains("d drop"), "{narrow:?} at 100: {bar:?}");
}

/// A pane keeps to its own vocabulary: the changes pane hints the keys it
/// answers and does not spill the graph's ([`TAIL`]) into a pane that does
/// not answer them.
#[test]
fn a_pane_hints_only_its_own_keys() {
    let pane = hints_text(&hints_for(
        &HintContext::ChangesPane {
            unmerged: false,
            live: None,
            tab: TabWord::Graph,
            over_overlay: false,
        },
        Divider::Hidden,
        200,
    ));
    assert_eq!(
        pane,
        " space stage · d discard · c commit · ⌃A all · e edit · ⇥ graph · . actions · ? keys"
    );
    let unmerged = hints_text(&hints_for(
        &HintContext::ChangesPane {
            unmerged: true,
            live: None,
            tab: TabWord::Graph,
            over_overlay: false,
        },
        Divider::Hidden,
        200,
    ));
    assert_eq!(
        unmerged,
        " ⏎ resolve · M mergetool · e edit · space mark resolved · c commit · ⇥ graph · . actions · ? keys"
    );
}

/// The banner draws its **own** keys — the whole list, at 80 columns, with
/// no `? keys` and no `<> resize`: it is modal, so those two would name
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
                merge: false,
            },
            Divider::Shown,
            80,
        )),
        " ⏎ resolve  c continue  s skip  a abort  M mergetool  e edit  ⇥ graph  q quit"
    );
    assert_eq!(
        hints_text(&hints_for(
            &HintContext::Banner {
                skippable: false,
                conflicted: true,
                merge: false,
            },
            Divider::Shown,
            80,
        )),
        " ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph  q quit"
    );
    assert_eq!(
        hints_text(&hints_for(
            &HintContext::Banner {
                skippable: true,
                conflicted: false,
                merge: false,
            },
            Divider::Shown,
            80,
        )),
        " c continue  s skip  a abort  ⇥ graph  q quit"
    );
}

/// m8-13b: `q quit` is the banner bar's last hint at the user's 187
/// columns, and at every width it is the first to go — whenever it is
/// drawn, every other banner key is drawn too, and never at a shorter rung
/// than it.
#[test]
fn q_quit_is_the_banner_bars_last_hint_and_first_dropped() {
    for (skippable, conflicted, merge) in [
        (true, true, false),
        (false, true, true),
        (true, false, false),
        (false, false, true),
    ] {
        let context = HintContext::Banner {
            skippable,
            conflicted,
            merge,
        };
        let full = context_list(&context);
        let wide = hints_for(&context, Divider::Hidden, 187);
        assert_eq!(wide.last().map(|h| (h.key(), h.verb)), Some(("q", "quit")));
        assert_eq!(wide.len(), full.len(), "187 draws them all");
        for columns in 20u16..=187 {
            let drawn = hints_for(&context, Divider::Hidden, columns);
            if drawn.iter().any(|h| h.key() == "q") {
                assert_eq!(
                    drawn.len(),
                    full.len(),
                    "{columns}: `q` drawn while another key was dropped: {}",
                    hints_text(&drawn)
                );
            }
        }
    }
}

/// m8-13b: beside the stop's reason, `q quit` gives way before the reason
/// loses a word — at the 108 cells the changes pane leaves at 187 columns,
/// the conflict count stays and `q` goes.
#[test]
fn q_quit_gives_way_to_the_stop_reason() {
    let context = HintContext::Banner {
        skippable: false,
        conflicted: true,
        merge: false,
    };
    let hints = hints_for(&context, Divider::Hidden, 108);
    assert_eq!(
        hints.last().map(Spelled::key),
        Some("q"),
        "the bar has room"
    );
    let why = StopWording::new(
        reason("merge in progress · 13 files conflicted"),
        vec![reason("merge in progress")],
    );
    let row = stopped_row(&why, &hints, 108);
    assert_eq!(
        row,
        " merge in progress · 13 files conflicted · \u{23ce} resolve  c continue  a abort  \
         M mergetool  e edit  \u{21e5} graph"
    );
    // With room for both, both are drawn.
    let row = stopped_row(&why, &hints_for(&context, Divider::Hidden, 160), 160);
    assert!(row.ends_with("  q quit"), "{row}");
    assert!(row.contains("13 files conflicted"), "{row}");
}

/// Every non-`Preview`, non-`Marked` context list is drawn in the order the
/// design note's table gives it — a change there is a spec change, not a
/// refactor.
#[test]
fn context_lists_match_the_design_note_order() {
    let working_tree = hints_for(&HintContext::WorkingTreeRow, Divider::Hidden, 240);
    let text = hints_text(&working_tree);
    assert!(
        text.starts_with(" ⏎ changes · c commit · S stash · p pull · P push · f fetch"),
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
        !working_tree.iter().any(|hint| hint.key() == "space"),
        "{:?}",
        hints_text(&working_tree)
    );
    let plain = hints_for(
        &HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
        Divider::Hidden,
        1000,
    );
    assert!(
        plain
            .iter()
            .any(|hint| hint.key() == "space" && hint.verb == "fold"),
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
        // m7-power-tools/21b: blame's `L`, third in its bar, fits the
        // 48-cell floor only at this prefix.
        ("history of these lines", "history"),
        // Conductor ruling 2026-09-27 (21b): the history bar at 40 columns.
        ("next change", "next"),
        ("older·newer", "older"),
        // m8 06 (ADR-0058's ` · ` separator): the way back leads its bar and
        // its floor fits the 47-cell narrowest bar only at this prefix.
        // Proposed by the builder, for the conductor's ruling (ticket NOTES).
        ("back to blame", "back"),
        ("back to history", "back"),
        // m8/20c R6' (B4): the conflict row's `space` at 80 columns.
        ("mark resolved", "mark"),
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
                hint.key(),
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
        "keys",
        "actions",
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
            merge: false,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: true,
            merge: false,
        },
        HintContext::Banner {
            skippable: true,
            conflicted: false,
            merge: false,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: false,
            merge: false,
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
                            hint.text(Keymap::shipped()).expect("bound"),
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
///
/// In a words-only bar (ADR-0058 §4) the lowest rung a tail hint is placed
/// at is its floor rung, not its bare key; the clause is the same one rung
/// up.
#[test]
fn a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn() {
    for context in all_contexts() {
        let placed_at = |hint: &Hint| match words_for(&context) {
            Words::Only => hint.floor_rung(),
            Words::Keys => Rung::Key,
        };
        for divider in [Divider::Shown, Divider::Hidden] {
            let whole = whole_list(&context, divider);
            for columns in 0..=240u16 {
                let hints = hints_for(&context, divider, columns);
                // The doorway is outside the body only where it is the
                // trailer; on a marked row it is an ordinary list hint
                // (m8/11d R2), counted in `whole`.
                let body = if super::whole_bar(&context, divider, Keymap::shipped())
                    .1
                    .is_some()
                {
                    without_trailer(&hints)
                } else {
                    hints.clone()
                };
                let raised = body
                    .iter()
                    .skip(3)
                    .find(|hint| hint.rung != placed_at(hint))
                    .map(Spelled::key);
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
                    drawn.key()
                );
            }
        }
    }
}

/// A hint is dropped only when it will not fit **even at its floor rung** —
/// the last clause of ADR-0039's third amendment, as ADR-0058 §4 restates it
/// for a row context's words-only bar (the bare-key rung is gone there). At
/// every width, adding the next hint the context has, with the fewest words
/// it may draw, would overflow the bar.
#[test]
fn a_hint_is_dropped_only_when_its_words_will_not_fit() {
    for context in row_contexts() {
        assert_eq!(words_for(&context), Words::Only, "{context:?}");
        for divider in [Divider::Shown, Divider::Hidden] {
            let whole = whole_list(&context, divider);
            for columns in 40..=240u16 {
                let hints = hints_for(&context, divider, columns);
                let body = without_trailer(&hints);
                if body.len() == whole.len() {
                    continue;
                }
                let words = |hint: Hint| Hint {
                    words_only: true,
                    ..hint
                };
                let mut with_one_more = body.clone();
                // The next hint *in fit order* (m8/15 A5: `u undo` is
                // fitted second, so the drawn body need not be a prefix).
                let next = super::fitting::fit_order(&whole)
                    .into_iter()
                    .map(|at| whole[at])
                    .find(|hint| !body.iter().any(|drawn| drawn.is(hint.keys)))
                    .expect("a hint the bar dropped");
                with_one_more.push(words(next.at(next.floor_rung())));
                with_one_more.push(words(DOORWAY));
                assert!(
                    hints_text(&with_one_more).width() > usize::from(columns),
                    "{context:?} at {columns} columns dropped `{}` with room for its words: {:?}",
                    next.key(),
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
                // Without `? keys`: it moves along the row as hints are
                // added, and comparing by index would read that move as a
                // reorder.
                let wider = without_trailer(&drawn);
                assert!(
                    wider.len() >= narrower.len(),
                    "{context:?} with {divider:?} at {columns} columns lost a hint"
                );
                // Matched by key, not index (m8/15 A5): `u undo` is fitted
                // second but drawn in its list place, so a hint added at a
                // wider bar may land before it. What must hold is that every
                // hint stays, in the same relative order, at no worse a rung.
                let mut last = None;
                for before in &narrower {
                    let at = wider
                        .iter()
                        .position(|hint| hint.is(before.keys) && hint.verb == before.verb);
                    let Some(at) = at else {
                        panic!(
                            "{context:?} with {divider:?} at {columns} columns lost `{}`",
                            before.key()
                        );
                    };
                    assert!(
                        last.is_none_or(|last| at > last),
                        "{context:?} with {divider:?} at {columns} columns reordered"
                    );
                    last = Some(at);
                    assert!(
                        wider[at].rung >= before.rung,
                        "{context:?} with {divider:?}: `{}` fell from {:?} to {:?} \
                         when the bar grew to {columns} columns",
                        before.key(),
                        before.rung,
                        wider[at].rung
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
    super::whole_bar(context, divider, Keymap::shipped()).0
}

/// The drawn hints without the `? keys` doorway — the one part of the bar
/// the ladder does not touch.
///
/// Taken out wherever it sits rather than off the end: since m5/28 the
/// doorway leads the bare keys, so on a graph row it is in the middle of the
/// row and on a pane row it is still last (ADR-0039).
fn without_trailer(hints: &[Hint]) -> Vec<Hint> {
    hints
        .iter()
        .filter(|hint| (hint.key(), hint.verb) != (DOORWAY.key(), DOORWAY.verb))
        .copied()
        .collect()
}

/// **The doorway sits at the head of the bare keys — which is the end of
/// the row when there are none** (ADR-0039, m5/28).
///
/// Stated as one invariant over the drawn row, because it is one rule: no
/// bare key ahead of the doorway, nothing but bare keys behind it. A row
/// with a tail of bare keys puts `? keys` in front of that tail; a row with
/// none leaves it last, where it always was.
fn assert_doorway_leads_the_bare_keys(hints: &[Hint], context: &str) {
    let at = hints
        .iter()
        .position(|hint| (hint.key(), hint.verb) == (DOORWAY.key(), DOORWAY.verb))
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
        [only] => doorway.is_some_and(|door| (only.key(), only.verb) == (door.key(), door.verb)),
        _ => false,
    }
}

/// The doorway `context` has, if any: `? keys` for everything but the
/// focused banner,
/// a drag in the air and the two overlays — modal in the way that matters,
/// so `.` leads nowhere and is not drawn (ADR-0039's first amendment).
///
/// Written out arm by arm rather than with a `_` —
/// the wildcard is how it came to claim a doorway for `Dragging` that the
/// bar never draws (m5/31).
fn doorway_of(context: &HintContext) -> Option<Hint> {
    match context {
        HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::Dragging
        | HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Picker
        | HintContext::Resolver { .. }
        | HintContext::Finish { .. }
        | HintContext::RewriteEditor { .. }
        | HintContext::RewriteMessage => None,
        HintContext::BackToBlame(inner) | HintContext::BackToHistory(inner) => doorway_of(inner),
        HintContext::Marked(_) => Some(EXPLAIN),
        // A filter keeps the doorway of the context it filters: `? keys`
        // still leads to everything, the dropped `⏎` included. So does the
        // way back: it leads the row's list, and changes nothing else.
        // A search's `n N` lead the row's list and change nothing else.
        HintContext::PaneFocused { row, .. }
        | HintContext::OperationLive { row, .. }
        | HintContext::Searching(row) => doorway_of(row),
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::StashRow
        | HintContext::Rewritable(_)
        | HintContext::ChangesPane { .. }
        // Modal for keys, not for the bar (m8/11d R1).
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Blame { .. }
        | HintContext::History { .. } => Some(DOORWAY),
    }
}

/// **One rule, two layouts, no special case** (ADR-0039, m5/28): the
/// doorway sits at the head of the bare keys — which is the end of the row
/// when there are none.
///
/// Since ADR-0058 §4 a row context draws no bare keys, so its doorway is
/// last too: at 80 cells the graph row's bar is words joined by ` · `,
/// ending in `? keys`. The changes pane and the journal browser draw every
/// hint whole at the same width, and `? keys` stays last, where it always
/// was.
#[test]
fn one_rule_puts_the_doorway_before_a_tail_and_last_without_one() {
    let with_a_tail = hints_for(
        &HintContext::BranchRow {
            current: true,
            verbs: RowVerbs::NONE,
        },
        Divider::Hidden,
        80,
    );
    let text = hints_text(&with_a_tail);
    assert_eq!(
        text,
        " ⏎ open \u{b7} c commit \u{b7} P push \u{b7} p pull \u{b7} f fetch \u{b7} b branch \u{b7} u undo \u{b7} ? keys",
        "a row context draws words only, its doorway is last, and `u undo` outranks the tail (m8/15 A5)"
    );

    for context in [
        HintContext::ChangesPane {
            unmerged: false,
            live: None,
            tab: TabWord::Graph,
            over_overlay: false,
        },
        HintContext::PaneOverOverlay { opens: true },
    ] {
        let hints = hints_for(&context, Divider::Hidden, 80);
        assert!(
            hints.iter().all(|hint| hint.rung != Rung::Key),
            "{context:?} at 80 cells has no tail to lead: {:?}",
            hints_text(&hints)
        );
        let text = hints_text(&hints);
        // Where there is a doorway and no tail to lead, it stays last,
        // where it always was (the pane over an overlay, m8/11c; the journal
        // case this held until m8/11e is the journal footer's own test now).
        if doorway_of(&context).is_some() {
            assert!(
                text.ends_with("? keys"),
                "{context:?} has no bare keys, so the doorway stays last: {text:?}"
            );
        } else {
            assert!(
                !text.contains("? keys"),
                "{context:?} is modal: `.` leads nowhere, so it is not drawn: {text:?}"
            );
        }
    }

    // And the invariant behind both, over every context that has a `? keys`
    // to place, at every width. A marked row's doorway is `. explain`, which
    // the caller puts at the head of the list itself and the ladder never
    // moves; the focused banner has none at all.
    for context in all_contexts() {
        if doorway_of(&context) != Some(DOORWAY) {
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

/// The floor fits at 47 cells of bar — the narrowest bar anything reachable
/// actually has.
///
/// `graft_ui::detail_pane::layout`'s `RIGHT_MIN = 48` clamps the graph side
/// of a side-by-side split, and below 100 columns the pane is a full-width
/// tab instead, so no divider drag leaves the graph narrower than this; the
/// bar keeps one cell of right margin of it (ADR-0058 §4). The 20-cell
/// overflow the m5/27 sweep found is below anything a user can reach — this
/// is the number that matters, and it is asserted rather than assumed
/// (m5/28). Since ADR-0058's ` · ` separator, the back-to-a-place leads fit
/// it only at their short rung, `esc back`.
#[test]
fn the_floor_fits_at_the_narrowest_bar() {
    for context in all_contexts() {
        if context == HintContext::Preview {
            continue;
        }
        let whole = whole_list(&context, Divider::Hidden);
        let floor = whole.len().min(3);
        let hints = hints_for(&context, Divider::Hidden, 47);
        let body = without_trailer(&hints);
        assert!(
            body.len() >= floor,
            "{context:?} cannot draw its floor of {floor} in 47 cells — that is a live \
             defect, not a completeness gap: {:?}",
            hints_text(&hints)
        );
    }
}

/// ADR-0058 §4: outside modal contexts no hint is drawn at its bare key, at
/// any width from 40 to 200, and `? keys` is always last where it is drawn.
#[test]
fn a_words_only_bar_draws_no_bare_key_and_ends_with_its_doorway() {
    for context in all_contexts() {
        if words_for(&context) == Words::Keys {
            continue;
        }
        for divider in [Divider::Hidden, Divider::Shown] {
            for columns in 40u16..=200 {
                let hints = hints_for(&context, divider, columns);
                let text = hints_text(&hints);
                assert!(
                    hints.iter().all(|hint| hint.rung != Rung::Key),
                    "{context:?} at {columns}: a bare key: {text:?}"
                );
                if let Some(at) = hints.iter().position(|hint| *hint == DOORWAY) {
                    assert_eq!(at, hints.len() - 1, "{context:?} at {columns}: {text:?}");
                }
                assert!(
                    hints.len() < 2 || text.contains(" \u{b7} "),
                    "{context:?} at {columns}: joined by ` · `: {text:?}"
                );
            }
        }
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
                    whole_file: false,
                    done: false,
                    roles: SideRoles::Merge,
                    finishable: false,
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
        done: false,
        roles: SideRoles::Merge,
        finishable: false,
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
    let words: Vec<(&str, &str)> = list.iter().map(|hint| (hint.key(), hint.verb)).collect();
    assert_eq!(
        words,
        vec![
            ("o", "yours"),
            ("t", "theirs"),
            ("b", "both"),
            ("⌃O/⌃T", "all"),
            ("O/T", "file"),
            ("e", "edit"),
            ("n", "next"),
            ("N", "prev"),
            ("⏎", "done"),
            ("j/k", "file"),
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
        .position(|hint| (hint.key(), hint.verb) == ("B", "base"))
        .expect("`B base` after `b both` while `base`");
    // `⌃O/⌃T` sits between (m6-conflicts/16): `B base` follows the takes
    // and `O/T file` (17).
    assert_eq!(
        with_base[base_at - 3].key(),
        "b",
        "`B base` follows `b both`"
    );
    assert_eq!(with_base[base_at - 1].key(), "O/T");

    let with_restart = context_list(&document(false, true));
    let restart_at = with_restart
        .iter()
        .position(|hint| (hint.key(), hint.verb) == ("d", "start over"))
        .expect("`d start over` before `esc close` while `restart`");
    assert_eq!(
        with_restart[restart_at + 1].key(),
        "esc",
        "`d start over` precedes `esc close`"
    );

    // The width ladder: drawn through `hints_text`/`fit_list` directly (not
    // `hints_for`, which would run the `Live` filter and zero the list) —
    // the ladder's own mechanics do not know or care about `HintState`, so
    // this exercises exactly the code `hints_for` would once the table
    // flips.
    let whole = list.clone();
    let at_160 = fit_list(&whole, None, 160, Words::Keys, Keymap::shipped());
    assert_eq!(
        hints_text(&at_160),
        " o yours  t theirs  b both  ⌃O/⌃T all  O/T file  e edit  n next  N prev  ⏎ done  j/k file  M mergetool  esc close"
    );
    // Since m6-conflicts/17 (`O/T file`, `j/k file`) the full line no
    // longer fits 80: the ladder raises hints in relevance order, so
    // everything after `e edit` is drawn bare (observed by this assertion
    // after review m6-amend-16 M2 made `⌃O/⌃T all` one hint, 2026-09-24).
    // Since m6-conflicts/56 `esc close` keeps its words ahead of the bare
    // keys (ADR-0044 §4), so at 80 `e` goes bare instead.
    let at_80 = fit_list(&whole, None, 80, Words::Keys, Keymap::shipped());
    let text_80 = hints_text(&at_80);
    assert_eq!(
        text_80,
        " o yours  t theirs  b both  ⌃O/⌃T all  O/T file  e  n  N  ⏎  j/k  M  esc close"
    );
    assert!(text_80.width() <= 80, "{text_80:?}");
    // Nothing is ever cut to `? keys` — the resolver is modal (§2.1), so
    // there is no doorway to lead a dropped tail to.
    assert!(!text_80.contains("more"), "{text_80:?}");
}

/// `O/T file` and `j/k file` (m6-conflicts/17 D7): on the document's bar,
/// right after `⌃O/⌃T all` and `⏎ done`; never on a card's or a done file's;
/// and `resolved` is no verb anywhere in the resolver's tables. The bar
/// string at 80, 120 and 200 columns is pinned (ticket 17 NOTES).
#[test]
fn resolver_offers_file_keys() {
    for (base, restart) in [(false, false), (true, true)] {
        let list = context_list(&document(base, restart));
        let at = |key: &str| {
            list.iter()
                .position(|hint| hint.key() == key)
                .unwrap_or_else(|| panic!("`{key}` in {list:?}"))
        };
        assert_eq!(list[at("O/T")].verb, "file");
        assert_eq!(
            at("O/T"),
            at("⌃O/⌃T") + 1,
            "`O/T file` right after `⌃O/⌃T all`"
        );
        assert_eq!(list[at("j/k")].verb, "file");
        assert_eq!(list[at("⏎")].verb, "done");
        assert_eq!(at("j/k"), at("⏎") + 1, "`j/k file` right after `⏎ done`");
    }
    for (whole_file, done) in [(false, false), (true, false), (false, true)] {
        let list = context_list(&HintContext::Resolver {
            base: true,
            restart: true,
            whole_file,
            done,
            roles: SideRoles::Merge,
            finishable: false,
        });
        assert!(list.iter().all(|hint| hint.verb != "resolved"), "{list:?}");
        if whole_file || done {
            assert!(
                list.iter()
                    .all(|hint| hint.key() != "O/T" && hint.key() != "j/k"),
                "neither on the card or done bar: {list:?}"
            );
        }
    }
    let bar = |columns| {
        hints_text(&hints_for(
            &document(false, false),
            Divider::Hidden,
            columns,
        ))
    };
    assert_eq!(
        bar(80),
        " o yours  t theirs  b both  ⌃O/⌃T all  O/T file  e  n  N  ⏎  j/k  M  esc close"
    );
    assert_eq!(
        bar(120),
        " o yours  t theirs  b both  ⌃O/⌃T all  O/T file  e edit  n next  N prev  ⏎ done  j/k file  M mergetool  esc close"
    );
    let at_200 = bar(200);
    assert_eq!(
        at_200,
        " o yours  t theirs  b both  ⌃O/⌃T all  O/T file  e edit  n next  N prev  ⏎ done  j/k file  M mergetool  esc close"
    );
    assert!(at_200.contains("O/T file") && at_200.contains("j/k file"));
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
            done: false,
            roles: SideRoles::Merge,
            finishable: false,
        };
        assert_eq!(
            hints_text(&hints_for(&card, Divider::Hidden, 160)),
            " o yours  t theirs  e edit  n next  M mergetool  esc close"
        );
    }
    assert!(every_context().contains(&HintContext::Resolver {
        base: false,
        restart: false,
        whole_file: true,
        done: false,
        roles: SideRoles::Merge,
        finishable: false,
    }));
}

/// A done file, read-only (m6-conflicts/25): exactly `d`, `e`, `esc` where
/// ops can start it over (`restart`, m6-conflicts/28), whatever the other
/// flags say, and a form every sweep reads.
#[test]
fn a_done_file_offers_start_over_edit_and_close() {
    for (base, restart, whole_file) in [
        (false, true, false),
        (true, true, false),
        (false, true, true),
    ] {
        let done = HintContext::Resolver {
            base,
            restart,
            whole_file,
            done: true,
            roles: SideRoles::Rebase,
            finishable: false,
        };
        assert_eq!(
            hints_text(&hints_for(&done, Divider::Hidden, 160)),
            " d start over  e edit  esc close"
        );
    }
    assert!(
        every_context()
            .iter()
            .any(|context| matches!(context, HintContext::Resolver { done: true, .. }))
    );
}

/// A done file ops cannot start over (m6-conflicts/28): exactly `e edit
/// esc close` — `d` is not offered where it would only answer "can't" — and
/// a form every sweep reads.
#[test]
fn a_done_file_that_cannot_start_over_offers_edit_and_close() {
    for (base, whole_file) in [(false, false), (true, false), (false, true)] {
        let done = HintContext::Resolver {
            base,
            restart: false,
            whole_file,
            done: true,
            roles: SideRoles::Merge,
            finishable: false,
        };
        assert_eq!(
            hints_text(&hints_for(&done, Divider::Hidden, 160)),
            " e edit  esc close"
        );
    }
    assert!(every_context().iter().any(|context| matches!(
        context,
        HintContext::Resolver {
            done: true,
            restart: false,
            ..
        }
    )));
}

/// Every file done, an operation waiting (m6-conflicts/31, P8): the done
/// file's bar offers `c` — `c finish` in a merge, `c continue` otherwise —
/// and only while `finishable`. **First** since m6-conflicts/60: it is the
/// next action (UX 1.4), so at the floor the bar keeps `c, d, e`.
#[test]
fn a_finishable_done_file_offers_c() {
    let done = |roles, finishable| HintContext::Resolver {
        base: true,
        restart: true,
        whole_file: false,
        done: true,
        roles,
        finishable,
    };
    let bar = |context: &HintContext| hints_text(&hints_for(context, Divider::Hidden, 160));
    assert_eq!(
        bar(&done(SideRoles::Merge, true)),
        " c finish  d start over  e edit  esc close"
    );
    assert_eq!(
        bar(&done(SideRoles::Rebase, true)),
        " c continue  d start over  e edit  esc close"
    );
    assert_eq!(
        bar(&done(SideRoles::Merge, false)),
        " d start over  e edit  esc close"
    );
    // Every table, not the done one only: `c` leads.
    for whole_file in [false, true] {
        let list = context_list(&HintContext::Resolver {
            base: true,
            restart: true,
            whole_file,
            done: false,
            roles: SideRoles::Merge,
            finishable: true,
        });
        let keys: Vec<&str> = list.iter().map(Spelled::key).collect();
        assert_eq!(keys.first(), Some(&"c"), "{keys:?}");
        assert_eq!(keys.last(), Some(&"esc"), "{keys:?}");
    }
    assert!(every_context().iter().any(|context| matches!(
        context,
        HintContext::Resolver {
            done: true,
            finishable: true,
            roles: SideRoles::Merge,
            ..
        }
    )));
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
    let unmerged = context_list(&HintContext::ChangesPane {
        unmerged: true,
        live: None,
        tab: TabWord::Graph,
        over_overlay: false,
    });
    assert_eq!(
        hints_text(&unmerged),
        " ⏎ resolve  M mergetool  e edit  space mark resolved  c commit  ⇥ graph"
    );

    let banner_plain = context_list(&HintContext::Banner {
        skippable: false,
        conflicted: true,
        merge: false,
    });
    assert_eq!(
        hints_text(&banner_plain),
        " ⏎ resolve  c continue  a abort  M mergetool  e edit  ⇥ graph  q quit"
    );

    let banner_skippable = context_list(&HintContext::Banner {
        skippable: true,
        conflicted: true,
        merge: false,
    });
    assert_eq!(
        hints_text(&banner_skippable),
        " ⏎ resolve  c continue  s skip  a abort  M mergetool  e edit  ⇥ graph  q quit"
    );

    // `conflicted: false` drops `⏎ resolve` along with `M`/`e`: nothing is
    // unmerged, so the whole conflict-resolution group refuses in words.
    let banner_no_conflict = context_list(&HintContext::Banner {
        skippable: true,
        conflicted: false,
        merge: false,
    });
    assert!(
        !hints_text(&banner_no_conflict).contains("resolve"),
        "{:?}",
        hints_text(&banner_no_conflict)
    );
}

/// ADR-0043 §4: `o`/`t` read as the operation's role words, never a name;
/// a role word has no short form, so the short rung is the word itself.
#[test]
fn resolver_verbs_are_role_words() {
    let bar = |roles| {
        let context = HintContext::Resolver {
            base: false,
            restart: false,
            whole_file: false,
            done: false,
            roles,
            finishable: false,
        };
        hints_text(&hints_for(&context, Divider::Hidden, 200))
    };
    assert!(bar(SideRoles::Merge).starts_with(" o yours  t theirs  b both"));
    assert!(bar(SideRoles::Rebase).starts_with(" o upstream  t yours  b both"));
    // ADR-0043 §4 amended (m6-conflicts/84): the branch, then his edits.
    assert!(bar(SideRoles::SetAside).starts_with(" o branch  t yours  b both"));
    assert!(bar(SideRoles::StashPop).starts_with(" o yours  t stashed  b both"));
    for roles in [
        SideRoles::Merge,
        SideRoles::Rebase,
        SideRoles::CherryPick,
        SideRoles::Revert,
        SideRoles::StashPop,
        SideRoles::SetAside,
    ] {
        for word in [roles.ours(), roles.theirs()] {
            assert_eq!(super::short_form(word), None, "{word} has a short form");
            let short = Hint {
                rung: Rung::Short,
                ..hint(hint_keys::OURS, word)
            };
            assert_eq!(
                short.text(Keymap::shipped()).expect("bound"),
                format!("o {word}"),
                "{word}"
            );
        }
    }
}

/// `⌃O/⌃T all` (m6-conflicts/16, P3; review m6-amend-16 M2): one hint
/// right after `b both` on a document — the same words in every operation,
/// since `o`/`t` just before name the roles — and never on a card or a done
/// file.
#[test]
fn resolver_offers_take_all() {
    for roles in [SideRoles::Merge, SideRoles::Rebase] {
        let list = context_list(&HintContext::Resolver {
            base: false,
            restart: false,
            whole_file: false,
            done: false,
            roles,
            finishable: false,
        });
        let keys: Vec<(&str, &str)> = list.iter().map(|hint| (hint.key(), hint.verb)).collect();
        let both = keys
            .iter()
            .position(|pair| *pair == ("b", "both"))
            .expect("b both");
        assert_eq!(keys[both + 1], ("⌃O/⌃T", "all"), "{roles:?}");
        assert_eq!(
            list[both + 1].text(Keymap::shipped()).expect("bound"),
            "⌃O/⌃T all"
        );
    }

    for (whole_file, done) in [(true, false), (false, true)] {
        let other = context_list(&HintContext::Resolver {
            base: false,
            restart: false,
            whole_file,
            done,
            roles: SideRoles::Merge,
            finishable: false,
        });
        assert!(other.iter().all(|hint| hint.key() != "⌃O/⌃T"), "{other:?}");
    }
}

// ---------------------------------------------------------------------------
// A stop's row and the way back to the banner (m6-conflicts/12)
// ---------------------------------------------------------------------------

/// After `Esc` hands the keyboard back, the bar leads with the one key that
/// returns to the banner — at every width, since it sits in the floor.
#[test]
fn operation_live_leads_with_the_way_back() {
    for row in [
        HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
        HintContext::WorkingTreeRow,
        HintContext::BranchRow {
            current: true,
            verbs: RowVerbs::NONE,
        },
        HintContext::ChangesPane {
            unmerged: true,
            live: None,
            tab: TabWord::Graph,
            over_overlay: false,
        },
    ] {
        let context = HintContext::OperationLive {
            row: Box::new(row.clone()),
            c: LiveC::Finish,
            tab_to_banner: true,
            space_stages: false,
        };
        // 80 is the narrowest the app lays out for; 187 is the user's.
        for columns in [80, 120, 187, 200] {
            let drawn = hints_for(&context, Divider::Hidden, columns);
            assert_eq!(
                drawn.first().map(|hint| (hint.key(), hint.verb)),
                Some(("⇥", "banner")),
                "{row:?} at {columns}: {}",
                hints_text(&drawn)
            );
            assert_eq!(
                drawn.iter().filter(|hint| hint.key() == "⇥").count(),
                1,
                "{row:?} at {columns}: `⇥` means one thing on this bar: {}",
                hints_text(&drawn)
            );
        }
    }
}

/// Review m7-10-to-07c Minor 7 (07d): while an operation is live the graph's
/// write keys refuse, so `c` — the key that moves the operation on — is
/// ranked straight after the way back (and `a amend it` at an edit stop),
/// ahead of `m`/`r`, and keeps its verb at 120 and 187.
#[test]
fn live_c_is_ranked_ahead_of_the_rows_write_keys() {
    for (c, verb) in [
        (LiveC::Continue, "continue"),
        (LiveC::ContinueAmends, "continue"),
        (LiveC::Finish, "finish"),
    ] {
        let context = HintContext::OperationLive {
            row: Box::new(HintContext::PlainRow {
                verbs: RowVerbs::NONE,
            }),
            c,
            tab_to_banner: true,
            space_stages: false,
        };
        for columns in [120, 187] {
            let drawn = hints_for(&context, Divider::Hidden, columns);
            let at = |key: &str| drawn.iter().position(|hint| hint.key() == key);
            let lead = if c == LiveC::ContinueAmends { 2 } else { 1 };
            assert_eq!(
                drawn.get(lead).map(|hint| (hint.key(), hint.verb)),
                Some(("c", verb)),
                "{c:?} at {columns}: {}",
                hints_text(&drawn)
            );
            for write in ["m", "r"] {
                assert!(
                    at(write).is_none_or(|place| place > lead),
                    "{c:?} at {columns}: `{write}` before `c`: {}",
                    hints_text(&drawn)
                );
            }
        }
    }
}

fn reason(text: &str) -> SanitizedText {
    graft_term::sanitize_line(text)
}

/// The keys are fitted first and never give: when what they leave for the
/// reason is under twelve cells, the reason goes entirely rather than
/// drawing a stub.
#[test]
fn stopped_row_drops_the_reason_under_twelve_cells() {
    let hints = hints_for(
        &HintContext::Banner {
            skippable: false,
            conflicted: true,
            merge: false,
        },
        Divider::Hidden,
        80,
    );
    let bar = hints_text(&hints);
    let bar_width = bar.width();
    let why = StopWording::new(reason("merge stopped · 1 file conflicted"), Vec::new());
    // Exactly eleven cells of room: dropped.
    let columns = u16::try_from(bar_width + 3 + 11).expect("fits u16");
    assert_eq!(stopped_row(&why, &hints, columns), bar);
    // Twelve: drawn, cut with `…`.
    let columns = u16::try_from(bar_width + 3 + 12).expect("fits u16");
    let row = stopped_row(&why, &hints, columns);
    assert!(row.starts_with(" merge stop"), "{row}");
    assert!(row.contains('…'), "{row}");
    assert!(row.ends_with(&bar), "{row}");
    assert!(row.width() <= usize::from(columns), "{row}");
}

/// The bar comes back exactly as the ladder fitted it, reason or not, and
/// the whole row never outgrows the columns it was given.
#[test]
fn stopped_row_keeps_the_bar_whole() {
    let context = HintContext::Banner {
        skippable: true,
        conflicted: true,
        merge: false,
    };
    for columns in [40u16, 60, 80, 120, 187, 200] {
        let hints = hints_for(&context, Divider::Hidden, columns);
        let bar = hints_text(&hints);
        let row = stopped_row(
            &StopWording::new(
                reason("rebase stopped at step 3 of 7 · 2 files conflicted"),
                Vec::new(),
            ),
            &hints,
            columns,
        );
        assert!(row.ends_with(&bar), "{columns}: {row}");
        assert!(row.width() <= usize::from(columns), "{columns}: {row}");
    }
    let hints = hints_for(&context, Divider::Hidden, 200);
    let row = stopped_row(
        &StopWording::new(reason("rebase stopped at step 3 of 7"), Vec::new()),
        &hints,
        200,
    );
    assert_eq!(
        row,
        format!(" rebase stopped at step 3 of 7 ·{}", hints_text(&hints))
    );
}

/// Review m6-amend-11-12 M1: at the user's 187 columns with the changes pane
/// open the footer is about 108 cells, and a blind cut from the end drew
/// `1…` for 13 files. The row now takes the fullest wording that fits — the
/// shorter ones drop whole parts, so the count is drawn whole or not at all.
#[test]
fn stopped_row_keeps_the_count_whole_at_108_cells() {
    let hints = hints_for(
        &HintContext::Banner {
            skippable: false,
            conflicted: true,
            merge: false,
        },
        Divider::Hidden,
        108,
    );
    let why = StopWording::new(
        reason("merge in progress · begun outside Graft · 13 files conflicted"),
        vec![
            reason("merge in progress · 13 files conflicted"),
            reason("merge in progress"),
        ],
    );
    let row = stopped_row(&why, &hints, 108);
    assert!(
        row.starts_with(" merge in progress · 13 files conflicted ·"),
        "{row}"
    );
    // `q quit`, last, gave way to the count (m8-13b).
    assert_eq!(hints.last().map(Spelled::key), Some("q"));
    assert!(
        row.ends_with(&hints_text(&hints[..hints.len() - 1])),
        "{row}"
    );
    assert!(row.width() <= 108, "{row}");
    // Every width: the count is either all there or not there at all, and
    // nothing but the last wording is ever cut.
    for columns in 60u16..=200 {
        let hints = hints_for(
            &HintContext::Banner {
                skippable: false,
                conflicted: true,
                merge: false,
            },
            Divider::Hidden,
            columns,
        );
        let row = stopped_row(&why, &hints, columns);
        assert!(
            row.contains("13 files conflicted") || !row.contains(" 1"),
            "{columns}: {row}"
        );
        assert!(!row.contains("files conflicted…"), "{columns}: {row}");
        assert!(row.width() <= usize::from(columns), "{columns}: {row}");
    }
}

/// When no wording fits whole, the last (shortest) one is cut with `…`,
/// under the same twelve-cell floor as before.
#[test]
fn stopped_row_cuts_only_the_last_wording() {
    let hints = hints_for(
        &HintContext::Banner {
            skippable: false,
            conflicted: true,
            merge: false,
        },
        Divider::Hidden,
        80,
    );
    let bar_width = hints_text(&hints).width();
    let why = StopWording::new(
        reason("rebase still in progress from a previous run · 2 files conflicted"),
        vec![reason("rebase still in progress")],
    );
    let columns = u16::try_from(bar_width + 3 + 14).expect("fits u16");
    let row = stopped_row(&why, &hints, columns);
    assert!(row.starts_with(" rebase still"), "{row}");
    assert!(row.contains('…'), "{row}");
    assert!(!row.contains("conflicted"), "{row}");
}

/// The merge resolver with `B base` and `d start over` offered, drawn
/// into `columns` (ADR-0044 §4).
fn resolver_bar(roles: SideRoles, columns: u16) -> String {
    hints_text(&hints_for(
        &HintContext::Resolver {
            base: true,
            restart: true,
            whole_file: false,
            done: false,
            roles,
            finishable: false,
        },
        Divider::Hidden,
        columns,
    ))
}

fn assert_keeps_the_way_out(text: &str) {
    assert!(text.contains("d start over"), "{text}");
    assert!(text.ends_with("esc close"), "{text}");
    assert!(text.contains("j/k  M  d"), "`j/k` and `M` bare: {text}");
}

#[test]
fn resolver_bar_keeps_the_way_out_at_120() {
    assert_keeps_the_way_out(&resolver_bar(SideRoles::Merge, 120));
}

#[test]
fn resolver_bar_keeps_the_way_out_at_120_in_a_rebase() {
    assert_keeps_the_way_out(&resolver_bar(SideRoles::Rebase, 120));
}

#[test]
fn done_bar_keeps_the_way_out_at_100() {
    let text = hints_text(&hints_for(
        &HintContext::Resolver {
            base: false,
            restart: true,
            whole_file: false,
            done: true,
            roles: SideRoles::Merge,
            finishable: false,
        },
        Divider::Hidden,
        100,
    ));
    assert!(text.contains("d start over"), "{text}");
    assert!(text.ends_with("esc close"), "{text}");
}

/// ADR-0044 §4 as it reads (m6-conflicts/64, review M1): every key is
/// placed bare before a marked hint takes its words, so 80 columns loses
/// words, never a key — `j/k` and `M` stay on the bar.
#[test]
fn resolver_bar_shows_every_key_at_80() {
    for roles in [SideRoles::Merge, SideRoles::Rebase] {
        let context = HintContext::Resolver {
            base: true,
            restart: true,
            whole_file: false,
            done: false,
            roles,
            finishable: false,
        };
        let keys = |columns| {
            hints_for(&context, Divider::Hidden, columns)
                .iter()
                .map(Spelled::key)
                .collect::<Vec<_>>()
        };
        let text = resolver_bar(roles, 80);
        assert_eq!(keys(80), keys(240), "{text}");
        assert!(text.contains("j/k"), "{text}");
        assert!(text.contains("  M  "), "{text}");
    }
}

/// m6-conflicts/64 (review m8): the done list marks `d start over` as
/// ADR-0044 §4 decision 2 says, so it keeps its words once the list grows.
#[test]
fn done_list_marks_start_over() {
    let hints = hints_for(
        &HintContext::Resolver {
            base: false,
            restart: true,
            whole_file: false,
            done: true,
            roles: SideRoles::Merge,
            finishable: false,
        },
        Divider::Hidden,
        240,
    );
    let d = hints
        .iter()
        .find(|hint| hint.key() == "d")
        .expect("d drawn");
    assert!(d.keeps_words, "{hints:?}");
}

/// The changes pane's `⇥` names where it lands (m6-conflicts/70): from the
/// file list, the diff, in both arms of the table and with `c` said either
/// way. The other two words follow the same field.
#[test]
fn the_files_list_bar_names_where_tab_goes() {
    for unmerged in [false, true] {
        for live in [None, Some(LiveC::Continue)] {
            for (tab, word) in [
                (TabWord::Diff, "⇥ diff"),
                (TabWord::Banner, "⇥ banner"),
                (TabWord::Graph, "⇥ graph"),
            ] {
                let list = context_list(&HintContext::ChangesPane {
                    unmerged,
                    live,
                    tab,
                    over_overlay: false,
                });
                let text = hints_text(&list);
                assert!(text.ends_with(word), "{unmerged} {live:?}: {text:?}");
            }
        }
    }
}

/// A merge's banner says `c finish`, because `c` opens the finish summary
/// there; a rebase's banner still says `c continue` (m6-conflicts/70, n2).
#[test]
fn a_merge_banner_says_c_finish() {
    for conflicted in [false, true] {
        let merge = hints_text(&context_list(&HintContext::Banner {
            skippable: false,
            conflicted,
            merge: true,
        }));
        assert!(merge.contains("c finish"), "{merge:?}");
        assert!(!merge.contains("continue"), "{merge:?}");
        let rebase = hints_text(&context_list(&HintContext::Banner {
            skippable: true,
            conflicted,
            merge: false,
        }));
        assert!(rebase.contains("c continue"), "{rebase:?}");
    }
}

/// The blame place's bar words (design note pass 2b B1), m7-power-tools/20c.
mod blame_place {
    use super::*;

    fn words(list: &[Hint]) -> Vec<String> {
        list.iter()
            .filter_map(|hint| hint.text(Keymap::shipped()))
            .collect()
    }

    #[test]
    fn blame_bar_words_in_order() {
        assert_eq!(
            words(&blame_place_list(true, true)),
            [
                "⏎ that commit",
                "^ before it",
                "L history of these lines",
                "/ find",
                "n/N match",
                "space ages",
                "w whitespace",
                "i ignore list",
                "esc back",
            ]
        );
    }

    #[test]
    fn n_n_only_while_finding() {
        assert!(!words(&blame_place_list(false, true)).contains(&"n/N match".to_owned()));
        assert!(words(&blame_place_list(true, false)).contains(&"n/N match".to_owned()));
    }

    #[test]
    fn i_only_with_the_file() {
        assert!(!words(&blame_place_list(true, false)).contains(&"i ignore list".to_owned()));
        assert!(words(&blame_place_list(false, true)).contains(&"i ignore list".to_owned()));
    }

    /// m7-power-tools/20k, 20n: without lines (a read in flight, or a
    /// failed one) the line keys go.
    #[test]
    fn a_place_without_lines_draws_no_line_keys() {
        let reading = HintContext::Blame {
            finding: true,
            ignore_file: true,
            lines: false,
        };
        assert_eq!(
            words(&lists::context_list(&reading)),
            [
                "/ find",
                "space ages",
                "w whitespace",
                "i ignore list",
                "esc back"
            ]
        );
    }
}

/// The history place's bar (design note pass 1b "History";
/// m7-power-tools/21b): modal for keys, no `<> resize`; `? keys` since m8/11d.
mod history_place {
    use super::*;

    fn words(context: &HintContext) -> Vec<String> {
        hints_for(context, Divider::Shown, 400)
            .iter()
            .filter_map(|hint| hint.text(Keymap::shipped()))
            .collect()
    }

    #[test]
    fn history_bar_words_in_order() {
        assert_eq!(
            words(&HintContext::History {
                rows: true,
                on_file: false
            }),
            [
                "j/k older·newer",
                "⇥ file",
                "n/N next change",
                "B blame here",
                "⏎ that commit",
                "esc back",
                "? keys",
            ]
        );
        assert_eq!(
            words(&HintContext::History {
                rows: true,
                on_file: true
            }),
            [
                "j/k scroll",
                "⇥ list",
                "n/N next change",
                "B blame here",
                "⏎ that commit",
                "w whitespace",
                "esc back",
                "? keys",
            ]
        );
    }

    /// Tracing, failed, zero rows: nothing but the way back answers.
    #[test]
    fn history_bar_with_no_rows_offers_esc_back_alone() {
        for on_file in [false, true] {
            assert_eq!(
                words(&HintContext::History {
                    rows: false,
                    on_file
                }),
                ["esc back", "? keys"]
            );
        }
    }

    /// `esc back to history` leads the inner bar, which is otherwise whole.
    #[test]
    fn back_to_history_leads_the_inner_bar() {
        let led = words(&HintContext::BackToHistory(Box::new(
            HintContext::PlainRow {
                verbs: RowVerbs::NONE,
            },
        )));
        assert_eq!(led.first().map(String::as_str), Some("esc back to history"));
        assert_eq!(
            led[1..],
            words(&HintContext::PlainRow {
                verbs: RowVerbs::NONE
            })[..]
        );
    }

    /// Conductor ruling 2026-09-27: `next change` gives way first, then
    /// `older·newer`. Under the floor the bar is empty, as every modal's is
    /// (ADR-0039's bottom; `esc back` alone there would break
    /// `no_hint_loses_ground_as_the_room_for_it_grows`). The floor is the
    /// shortest form's width: 28 cells since `tab` became `⇥` (review
    /// m7-16b..21b, Minor 4), 30 before.
    #[test]
    fn history_bar_narrows_by_its_ladder() {
        let list = HintContext::History {
            rows: true,
            on_file: false,
        };
        let at = |columns| hints_text(&hints_for(&list, Divider::Hidden, columns));
        // Since m8/11d the doorway is in the floor (11c decision 4): the
        // floor is 36 cells, and below it the doorway stands alone.
        assert_eq!(at(44), " j/k older·newer  ⇥ file  n/N next  ? keys");
        assert!(!at(44).contains("next change"), "{:?}", at(44));
        assert_eq!(at(40), " j/k older  ⇥ file  n/N next  ? keys");
        assert_eq!(at(36), " j/k older  ⇥ file  n/N next  ? keys");
        assert_eq!(at(35), " ? keys");
        assert_eq!(at(28), " ? keys");
    }

    #[test]
    fn short_form_history_of_these_lines_is_history() {
        assert_eq!(short_form("history of these lines"), Some("history"));
    }
}

/// m7-power-tools/20h: `space` on a focused pane's bar is the pane's own
/// fold, or the graph's where the key still reaches the graph — never a
/// fold the key does not do.
mod pane_space {
    use super::*;

    fn keys(context: &HintContext) -> Vec<(&'static str, &'static str)> {
        hints_for(context, Divider::Hidden, 400)
            .into_iter()
            .map(|hint| (hint.key(), hint.verb))
            .collect()
    }

    fn graph_rows() -> Vec<HintContext> {
        let mut rows = row_contexts();
        rows.push(HintContext::Marked(Box::new(HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        })));
        rows.push(HintContext::Marked(Box::new(HintContext::BranchRow {
            current: true,
            verbs: RowVerbs::NONE,
        })));
        rows
    }

    fn pane(row: HintContext, opens: bool, blamable: bool, folds: bool) -> HintContext {
        HintContext::PaneFocused {
            row: Box::new(row),
            opens,
            blamable,
            folds,
            graph_hidden: false,
        }
    }

    /// With the files list focused, `graft`'s `files_key` takes `space`
    /// whatever the row, so the graph's merge fold is not live there; off a
    /// directory the bar draws no `space` at all.
    #[test]
    fn pane_focused_drops_the_graphs_space_fold() {
        for row in graph_rows() {
            for blamable in [false, true] {
                let context = pane(row.clone(), true, blamable, false);
                let drawn = keys(&context);
                assert!(
                    !drawn.iter().any(|(key, _)| *key == "space"),
                    "{context:?} draws {drawn:?}"
                );
            }
        }
    }

    // Replaced `pane_focused_on_the_diff_keeps_the_graphs_space_fold`: spec
    // change M7/20h-diff-space (ticket 20l).

    /// With the diff focused the pane claims `space` (`graft`'s
    /// `diff_option_key`), so the graph's merge fold is not live and not
    /// offered, on any row.
    #[test]
    fn pane_focused_on_the_diff_drops_the_graphs_space_fold() {
        for row in graph_rows() {
            let context = pane(row.clone(), false, true, false);
            assert!(!keys(&context).contains(&("space", "fold")), "{context:?}");
        }
    }

    /// Another list's `space` is not the graph's fold and is kept.
    #[test]
    fn pane_focused_keeps_the_changes_panes_space() {
        let changes = HintContext::ChangesPane {
            unmerged: false,
            live: None,
            tab: TabWord::Graph,
            over_overlay: false,
        };
        assert!(keys(&changes).contains(&("space", "stage")));
        let context = pane(changes, true, false, false);
        assert!(keys(&context).contains(&("space", "stage")), "{context:?}");
    }

    /// On a directory row the pane's own fold: same key, same verb as the
    /// graph's, placed after `⏎` and `B` when they are there.
    #[test]
    fn pane_focused_folds_offers_space_fold_after_enter_and_b() {
        let at =
            |drawn: &[(&str, &str)], want: &str| drawn.iter().position(|(key, _)| *key == want);
        for row in graph_rows() {
            for blamable in [false, true] {
                let drawn = keys(&pane(row.clone(), true, blamable, true));
                let enter = at(&drawn, "⏎").expect("the files list's ⏎");
                let lead = at(&drawn, "B").map_or(enter, |b| b.max(enter));
                assert_eq!(drawn.get(lead + 1), Some(&("space", "fold")), "{drawn:?}");
                assert_eq!(
                    drawn.iter().filter(|(key, _)| *key == "space").count(),
                    1,
                    "{drawn:?}"
                );
                assert_eq!(at(&drawn, "B").is_some(), blamable, "{drawn:?}");
            }
        }
    }
}

// m7-power-tools/40, planner ruling 1: `a amend` after the daily verbs.
#[test]
fn a_amend_ranks_after_push_pull_fetch() {
    let context = HintContext::BranchRow {
        current: true,
        verbs: RowVerbs {
            take: false,
            amend: true,
        },
    };
    let wide = hints_for(&context, Divider::Hidden, 1000);
    let at = |key: &str| wide.iter().position(|h| h.key() == key).expect(key);
    assert_eq!(at("a"), at("f") + 1, "{wide:?}");
    let narrow = hints_for(&context, Divider::Hidden, 80);
    assert!(narrow.iter().take(3).any(|h| h.key() == "P"), "{narrow:?}");
}

#[test]
fn a_amend_follows_open_where_no_commit_or_remote_verb() {
    let base = vec![
        hint(hint_keys::OPEN, "open"),
        hint(hint_keys::BRANCH, "branch"),
        hint(hint_keys::TAG, "tag"),
    ];
    let list = super::lists::with_verbs(
        base,
        RowVerbs {
            take: false,
            amend: true,
        },
    );
    let keys: Vec<&str> = list.iter().map(Spelled::key).collect();
    assert_eq!(keys, ["⏎", "a", "b", "t"]);
}

/// Ruling R4 (ticket m8/10), as the code stands: a `Fixed` key — spelled
/// from the shipped table whatever his config says — is drawn only where
/// the edge passes every press as typed, so the default *is* the key that
/// acts. Those are the fields `graft::update::state::AppState::text_field_open`
/// names (state.rs: the name prompt, the credential box, the picker, the
/// rewrite editor's `r` field, and the finish summary while its focus is
/// not `Files`); `graft`'s `x_typed_into_every_field_is_an_x_with_merge_on_x`
/// holds each of them to it. Everywhere else a key follows his remap.
///
/// Not R4's letter: R4 has the finish Subject/Body follow a `Finish` row,
/// on the premise that his remap acts there. It does not — `edge::key`
/// returns before `context_of` while `text_field_open` holds, and that
/// includes the summary's Subject and Body — so a remapped spelling there
/// would draw a key that does nothing (ticket 10c NOTES).
#[test]
fn a_fixed_key_is_drawn_only_where_keys_go_on_as_typed() {
    let typed = |context: &HintContext| match context {
        HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Picker
        | HintContext::RewriteMessage => true,
        HintContext::Finish { focus, .. } => *focus != SummaryFocus::Files,
        _ => false,
    };
    let mut fixed_drawn = 0;
    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let (list, door) = spelled_bar(&context, divider, Keymap::shipped());
            for hint in list.iter().chain(door.iter()) {
                if !hint.keys.iter().any(|key| matches!(key, HintKey::Fixed(_))) {
                    continue;
                }
                fixed_drawn += 1;
                assert!(
                    typed(&context),
                    "{context:?} draws `{} {}` from the shipped table, but his remap acts there",
                    hint.key(),
                    hint.verb
                );
            }
        }
    }
    assert!(fixed_drawn > 0, "no list draws a fixed key");
}

// --- m8/11b: the doorway is `? keys`, `. actions` an ordinary candidate ---

/// Decision 1: every bar `every_context()` yields, at 80, 120 and 200
/// columns, draws `? keys` where it has a doorway — last, or leading a
/// place's bare keys (m8/11d R1) — and never draws
/// `. more`.
#[test]
fn every_bar_ends_with_the_keys_doorway_and_never_more() {
    let keys = Keymap::shipped();
    for context in every_context() {
        for divider in [Divider::Hidden, Divider::Shown] {
            for columns in [80, 120, 200] {
                let hints = hints_for(&context, divider, columns);
                let text = hints_text(&hints);
                assert!(
                    !text.contains(". more"),
                    "{context:?} at {columns}: {text:?}"
                );
                if spelled_bar(&context, divider, keys).1.is_some() {
                    // A place's bar keeps bare keys, which the doorway leads
                    // (ADR-0039, m8/11d R1); every other bar ends with it.
                    assert_doorway_placed(&text, "? keys", &format!("{context:?} at {columns}"));
                }
            }
        }
    }
}

/// Decision 1: `. actions` is on the graph's bar at 200 columns, last
/// before the doorway, and is the first hint the width drops.
#[test]
fn actions_is_drawn_wide_and_dropped_first() {
    let row = HintContext::BranchRow {
        current: true,
        verbs: RowVerbs::NONE,
    };
    // The narrowest width that draws `. actions`: it sits last before the
    // doorway, and one cell less drops it and nothing else. (On this row
    // every other candidate fits first, so that width is 267 — m8/11b
    // NOTES.)
    let first = (40..=400u16)
        .find(|columns| hints_for(&row, Divider::Hidden, *columns).contains(&ACTIONS))
        .expect("drawn somewhere");
    let with = hints_for(&row, Divider::Hidden, first);
    let n = with.len();
    assert_eq!(with[n - 2], ACTIONS, "{:?}", hints_text(&with));
    assert_eq!(with[n - 1], DOORWAY, "{:?}", hints_text(&with));
    let without = hints_for(&row, Divider::Hidden, first - 1);
    let named = |hints: &[Hint]| -> Vec<(&str, &str)> {
        hints.iter().map(|hint| (hint.key(), hint.verb)).collect()
    };
    let mut expected = named(&with);
    expected.retain(|pair| *pair != (".", "actions"));
    assert_eq!(named(&without), expected);
}

/// Decision 1: `[keys.app] help = "none"` → the doorway is `⌃P keys`; with
/// `palette` unbound too there is no doorway, and nothing in its place.
#[test]
fn the_doorway_falls_back_to_the_palette_key_then_to_nothing() {
    let row = HintContext::PlainRow {
        verbs: RowVerbs::NONE,
    };
    let unbind = |ids: &[&str]| {
        Keymap::with_overrides(
            &ids.iter()
                .map(|id| ((*id).to_owned(), vec!["none".to_owned()]))
                .collect::<Vec<_>>(),
            crate::keymap::KeySwitches::default(),
        )
        .0
    };
    let no_help = unbind(&["app.help"]);
    let bar = super::hints_for(&row, Divider::Hidden, 120, &no_help);
    let text = super::hints_text(&bar, &no_help);
    assert!(text.ends_with("⌃P keys"), "{text:?}");
    let neither = unbind(&["app.help", "app.palette"]);
    let bar = super::hints_for(&row, Divider::Hidden, 120, &neither);
    let text = super::hints_text(&bar, &neither);
    assert!(!text.contains("keys"), "{text:?}");
    assert!(!text.contains('—'), "{text:?}");
}

/// M8/marked-row-no-ctrlp: a marked row spells the doorway the same way
/// the bar trailer does: `⌃P keys` where `? keys` sat when `help` is
/// unbound; with `palette` unbound too, no doorway and nothing else changes.
#[test]
fn a_marked_row_with_help_unbound_ends_ctrl_p_keys() {
    let marked = HintContext::Marked(Box::new(HintContext::PlainRow {
        verbs: RowVerbs::NONE,
    }));
    let unbind = |ids: &[&str]| {
        Keymap::with_overrides(
            &ids.iter()
                .map(|id| ((*id).to_owned(), vec!["none".to_owned()]))
                .collect::<Vec<_>>(),
            crate::keymap::KeySwitches::default(),
        )
        .0
    };
    let no_help = unbind(&["app.help"]);
    let text = super::hints_text(
        &super::hints_for(&marked, Divider::Hidden, 240, &no_help),
        &no_help,
    );
    assert!(text.ends_with("⌃P keys"), "{text:?}");
    assert!(
        text.contains("u undo") && text.contains("/ search"),
        "{text:?}"
    );
    let neither = unbind(&["app.help", "app.palette"]);
    let text = super::hints_text(
        &super::hints_for(&marked, Divider::Hidden, 240, &neither),
        &neither,
    );
    assert!(!text.contains("keys"), "{text:?}");
    // Nothing else changes: at a width that holds every hint, the same
    // bar as the shipped one, its doorway respelled or gone. (At 240 the
    // cells freed by a missing doorway let one more hint in — the width
    // rule, not a change.)
    let text = super::hints_text(
        &super::hints_for(&marked, Divider::Hidden, 400, &neither),
        &neither,
    );
    let shipped = Keymap::shipped();
    let base = super::hints_text(
        &super::hints_for(&marked, Divider::Hidden, 400, shipped),
        shipped,
    );
    let with_ctrl_p = super::hints_text(
        &super::hints_for(&marked, Divider::Hidden, 400, &no_help),
        &no_help,
    );
    assert_eq!(with_ctrl_p, base.replace("? keys", "⌃P keys"));
    assert_eq!(Some(text.as_str()), base.strip_suffix(" · ? keys"));
}

/// The overlays and places m8/11c decision 1 names: modal for keys, not for
/// the bar (ADR-0058 §4), so `?` answers there and the bar says so.
fn overlays_and_places() -> Vec<HintContext> {
    let mut contexts = vec![
        HintContext::PaneOverOverlay { opens: true },
        HintContext::PaneOverOverlay { opens: false },
        HintContext::ChangesPane {
            unmerged: false,
            live: None,
            tab: TabWord::Graph,
            over_overlay: true,
        },
    ];
    for finding in [false, true] {
        for ignore_file in [false, true] {
            for lines in [false, true] {
                contexts.push(HintContext::Blame {
                    finding,
                    ignore_file,
                    lines,
                });
            }
        }
    }
    for rows in [false, true] {
        for on_file in [false, true] {
            contexts.push(HintContext::History { rows, on_file });
        }
    }
    let row = HintContext::PlainRow {
        verbs: RowVerbs::NONE,
    };
    contexts.push(HintContext::BackToBlame(Box::new(row.clone())));
    contexts.push(HintContext::BackToHistory(Box::new(row)));
    contexts
}

fn unbound(ids: &[&str]) -> Keymap {
    Keymap::with_overrides(
        &ids.iter()
            .map(|id| ((*id).to_owned(), vec!["none".to_owned()]))
            .collect::<Vec<_>>(),
        crate::keymap::KeySwitches::default(),
    )
    .0
}

/// Asserts `doorway` sits where ADR-0039's one rule puts it: after every
/// hint drawn with words, ahead of every bare key (m8/11d R1).
fn assert_doorway_placed(text: &str, doorway: &str, what: &str) {
    // A keys bar joins with two spaces, a words-only one with ` · `.
    let pieces: Vec<&str> = text
        .trim()
        .split(" · ")
        .flat_map(|part| part.split("  "))
        .collect();
    let at = pieces
        .iter()
        .position(|piece| *piece == doorway)
        .unwrap_or_else(|| panic!("{what}: no {doorway:?} in {text:?}"));
    assert!(
        pieces[..at].iter().all(|piece| piece.contains(' ')),
        "{what}: a bare key before the doorway: {text:?}"
    );
    assert!(
        pieces[at + 1..].iter().all(|piece| !piece.contains(' ')),
        "{what}: a worded hint after the doorway: {text:?}"
    );
}

/// m8/11d R1: every overlay and place bar draws `? keys` at 80, 120 and 200
/// cells, after its words and leading its bare keys; `⌃P keys` with `?`
/// unbound; nothing with both unbound.
#[test]
fn overlays_and_places_draw_the_keys_doorway() {
    let no_help = unbound(&["app.help"]);
    let neither = unbound(&["app.help", "app.palette"]);
    for context in overlays_and_places() {
        for columns in [80u16, 120, 200] {
            let what = format!("{context:?} at {columns}");
            let text = hints_text(&hints_for(&context, Divider::Hidden, columns));
            assert_doorway_placed(&text, "? keys", &what);
            let bar = super::hints_for(&context, Divider::Hidden, columns, &no_help);
            let text = super::hints_text(&bar, &no_help);
            assert_doorway_placed(&text, "⌃P keys", &format!("{what}, `?` unbound"));
            let bar = super::hints_for(&context, Divider::Hidden, columns, &neither);
            let text = super::hints_text(&bar, &neither);
            assert!(
                !text.contains("? keys") && !text.contains("⌃P keys") && !text.contains('—'),
                "{context:?} at {columns}, both unbound: {text:?}"
            );
        }
    }
}

/// m8/11c decision 2: the bars where the keys are the point draw no doorway.
#[test]
fn modal_bars_draw_no_doorway() {
    let modal: Vec<HintContext> = every_context()
        .into_iter()
        .filter(|context| {
            matches!(
                context,
                HintContext::Banner { .. }
                    | HintContext::Dragging
                    | HintContext::ConfirmBox { .. }
                    | HintContext::NamePrompt { .. }
                    | HintContext::Credential
                    | HintContext::Picker
                    | HintContext::Resolver { .. }
                    | HintContext::Finish { .. }
                    | HintContext::RewriteEditor { .. }
                    | HintContext::RewriteMessage
                    | HintContext::Preview
            )
        })
        .collect();
    assert!(modal.len() >= 10, "{modal:?}");
    for context in modal {
        for columns in [48u16, 80, 120, 200] {
            let text = hints_text(&hints_for(&context, Divider::Hidden, columns));
            assert!(
                !text.contains("? keys") && !text.contains("⌃P keys"),
                "{context:?} at {columns}: {text:?}"
            );
        }
    }
}

/// m8/11d R2: on a marked row `? keys` ranks right after `u undo` and `/ search`, is drawn
/// last, monotone in width, and never at the cost of either.
#[test]
fn a_marked_row_keeps_undo_and_search_before_the_doorway() {
    let marked = HintContext::Marked(Box::new(HintContext::PlainRow {
        verbs: RowVerbs::NONE,
    }));
    for divider in [Divider::Hidden, Divider::Shown] {
        // Measured first-drawn width, recorded in 11d's NOTES (R2's "drawn
        // at 120" does not hold: the twelve hints ahead of it take 119
        // cells). Pinned so a ranking change shows up here.
        let first = (40..=240u16)
            .find(|columns| hints_text(&hints_for(&marked, divider, *columns)).contains("? keys"));
        assert_eq!(first, Some(129), "{divider:?}");
        let mut drawn_before = false;
        for columns in 40..=240u16 {
            let hints = hints_for(&marked, divider, columns);
            let text = hints_text(&hints);
            assert!(
                text.chars().count() <= usize::from(columns),
                "{divider:?} at {columns}: {text:?} overflows"
            );
            let has = hints.iter().any(|hint| hint.is(DOORWAY.keys));
            assert!(
                has || !drawn_before,
                "{divider:?}: `? keys` drawn below {columns} but not at it"
            );
            if has {
                assert!(
                    text.contains("u undo") && text.contains("/ search"),
                    "{divider:?} at {columns}: the doorway displaced one: {text:?}"
                );
                assert!(
                    hints.last().is_some_and(|hint| hint.is(DOORWAY.keys)),
                    "{divider:?} at {columns}: doorway not last"
                );
            }
            drawn_before |= has;
        }
    }
    // A branch tip's marked row: the doorway never displaces `u undo` or
    // `/ search` — wherever it is drawn, both are too. First drawn at 137,
    // both dividers (m8/11e F1: the measured width is the spec).
    let tip = HintContext::Marked(Box::new(HintContext::BranchRow {
        current: false,
        verbs: RowVerbs::NONE,
    }));
    for divider in [Divider::Hidden, Divider::Shown] {
        let mut first = None;
        for columns in 40..=240u16 {
            let text = hints_text(&hints_for(&tip, divider, columns));
            if text.contains("? keys") {
                first.get_or_insert(columns);
                assert!(
                    text.contains("u undo") && text.contains("/ search"),
                    "branch tip {divider:?} at {columns}: {text:?}"
                );
            }
        }
        assert_eq!(first, Some(137), "branch tip {divider:?}");
    }
}

/// m8/11e F2: the doorway's words come from one function, which follows his
/// keymap: `? keys`; `⌃P keys` with `help` unbound; nothing with both; his
/// own `help` key when he moved it.
#[test]
fn doorway_words_follows_his_keymap() {
    assert_eq!(
        super::doorway_words(Keymap::shipped()).as_deref(),
        Some("? keys")
    );
    assert_eq!(
        super::doorway_words(&unbound(&["app.help"])).as_deref(),
        Some("⌃P keys")
    );
    assert_eq!(
        super::doorway_words(&unbound(&["app.help", "app.palette"])),
        None
    );
    let (moved, warnings) = Keymap::with_overrides(
        &[("app.help".to_owned(), vec!["F1".to_owned()])],
        crate::keymap::KeySwitches::default(),
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    let label = crate::keymap::KeyChord::parse("F1")
        .expect("F1 is a key")
        .label();
    assert_eq!(
        super::doorway_words(&moved),
        Some(format!("{} keys", label.0))
    );
}

/// m8/11e E1: the place footers' one cut.
#[test]
fn shed_footer_drops_the_middle_first() {
    use super::fitting::shed_footer;

    // `⏎ undo this` (11), `/ find` (6), `space command log` (17), `J/esc close` (11).
    let widths = [11, 6, 17, 11];
    let all = 11 + 6 + 17 + 11 + 3 * 2;
    assert_eq!(shed_footer(&widths, 2, all), vec![0, 1, 2, 3]);
    assert_eq!(shed_footer(&widths, 2, 200), vec![0, 1, 2, 3]);
    // The middle from its right: `space command log` first, then `/ find`.
    assert_eq!(shed_footer(&widths, 2, all - 1), vec![0, 1, 3]);
    assert_eq!(shed_footer(&widths, 2, 11 + 6 + 11 + 4 - 1), vec![0, 3]);
    // First and last kept longest; below them, nothing — never a cut clause.
    assert_eq!(shed_footer(&widths, 2, 24), vec![0, 3]);
    assert_eq!(shed_footer(&widths, 2, 23), Vec::<usize>::new());
    assert_eq!(shed_footer(&widths, 2, 0), Vec::<usize>::new());
    assert_eq!(shed_footer(&[11], 2, 11), vec![0]);
    assert_eq!(shed_footer(&[11], 2, 10), Vec::<usize>::new());
    assert_eq!(shed_footer(&[], 2, 10), Vec::<usize>::new());
    // Monotone: more room never keeps less.
    let mut before: Vec<usize> = Vec::new();
    for room in 0..=80 {
        let kept = shed_footer(&widths, 2, room);
        assert!(
            before.iter().all(|at| kept.contains(at)),
            "{room}: {kept:?} lost one of {before:?}"
        );
        before = kept;
    }
}

/// m8/15 A1: while an operation is live, no graph bar offers a key that
/// refuses then — `u`, `m`, `r`, `R`, `o` (`lists::REFUSED_WHILE_LIVE`,
/// each member's refusing line in its doc). Every `OperationLive` form the
/// catalogue yields (each `LiveC` × `tab_to_banner` × `space_stages`, over
/// every row form), at every width that draws a bar.
#[test]
fn no_live_operation_bar_offers_a_key_that_refuses_while_live() {
    let refused = ["u", "m", "r", "R", "o"];
    let mut checked = 0;
    for context in all_contexts() {
        if !matches!(context, HintContext::OperationLive { .. }) {
            continue;
        }
        for columns in [80, 100, 120, 160, 200] {
            for divider in [Divider::Shown, Divider::Hidden] {
                checked += 1;
                let hints = hints_for(&context, divider, columns);
                let offered: Vec<&str> = hints
                    .iter()
                    .map(Spelled::key)
                    .filter(|key| refused.contains(key))
                    .collect();
                assert!(
                    offered.is_empty(),
                    "{context:?} at {columns}: offers {offered:?} in {:?}",
                    hints_text(&hints)
                );
            }
        }
    }
    assert!(checked >= 40, "only {checked} live bars checked");
}

/// m8/15 A5: `u undo` is fitted second, so on every graph-row form ×
/// `RowVerbs::ALL` — bare, `Marked`, `Rewritable` — whose whole bar holds
/// `u`, it is drawn at 100 and at 80 columns. `o`/`a` inserted by
/// `with_verbs` used to push it off at 100.
#[test]
fn undo_is_drawn_at_100_and_80_on_every_row_form_that_holds_it() {
    let mut checked = 0;
    for verbs in RowVerbs::ALL {
        let rows = [
            HintContext::BranchRow {
                current: false,
                verbs,
            },
            HintContext::BranchRow {
                current: true,
                verbs,
            },
            HintContext::PlainRow { verbs },
            HintContext::WorkingTreeRow,
            HintContext::StashRow,
        ];
        for row in rows {
            let forms = [
                row.clone(),
                HintContext::Marked(Box::new(row.clone())),
                HintContext::Rewritable(Box::new(row.clone())),
                HintContext::Marked(Box::new(HintContext::Rewritable(Box::new(row.clone())))),
            ];
            for context in forms {
                for divider in [Divider::Shown, Divider::Hidden] {
                    let whole = hints_text(&hints_for(&context, divider, 1000));
                    if !whole.contains("u undo") {
                        continue;
                    }
                    for columns in [80, 100] {
                        checked += 1;
                        let bar = hints_text(&hints_for(&context, divider, columns));
                        assert!(
                            bar.contains("u undo"),
                            "{context:?} {divider:?} at {columns}: {bar:?}"
                        );
                    }
                }
            }
        }
    }
    assert!(checked >= 100, "only {checked} bars checked");
}

/// m8/15 A4: a stash row's bar offers no `m`, `r`, `R`, `o` — each
/// refuses on a stash — and leads `⏎ open` then `S stash`, bare and marked,
/// at every width.
#[test]
fn a_stash_row_offers_open_then_stash_and_nothing_that_refuses_on_a_stash() {
    let bare = HintContext::StashRow;
    let marked = HintContext::Marked(Box::new(HintContext::StashRow));
    assert!(all_contexts().contains(&bare) && all_contexts().contains(&marked));
    for columns in [80, 100, 120, 160, 200] {
        for divider in [Divider::Shown, Divider::Hidden] {
            for context in [&bare, &marked] {
                let hints = hints_for(context, divider, columns);
                let keys: Vec<&str> = hints.iter().map(Spelled::key).collect();
                for refused in ["m", "r", "R", "o"] {
                    assert!(
                        !keys.contains(&refused),
                        "{context:?} at {columns} offers {refused}: {:?}",
                        hints_text(&hints)
                    );
                }
            }
            let hints = hints_for(&bare, divider, columns);
            assert_eq!(
                hints_text(&hints[..2]).trim(),
                "⏎ open · S stash",
                "{columns}: {:?}",
                hints_text(&hints)
            );
        }
    }
}

/// m8/15b (15 A2): a search left open after `⏎` leads the graph row's bar
/// with `n next  N prev`, then the row's own keys — bare, marked, under
/// `R`, and on a stash's row — at every width.
#[test]
fn a_search_after_enter_leads_the_row_bar_with_next_and_prev() {
    let rows = [
        HintContext::WorkingTreeRow,
        HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
        HintContext::BranchRow {
            current: true,
            verbs: RowVerbs::NONE,
        },
        HintContext::StashRow,
        HintContext::Marked(Box::new(HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        })),
        HintContext::Rewritable(Box::new(HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        })),
    ];
    for row in rows {
        let searching = HintContext::Searching(Box::new(row.clone()));
        assert!(
            all_contexts().contains(&searching),
            "every_context sweeps {searching:?}"
        );
        for columns in [80, 100, 120, 160] {
            for divider in [Divider::Shown, Divider::Hidden] {
                let hints = hints_for(&searching, divider, columns);
                assert_eq!(
                    hints_text(&hints[..2]).trim(),
                    "n next · N prev",
                    "{searching:?} at {columns}: {:?}",
                    hints_text(&hints)
                );
                // The row's own lead follows.
                let own = hints_for(&row, divider, 200);
                assert!(
                    hints.iter().any(|hint| *hint == own[0]),
                    "{searching:?} at {columns} keeps the row's lead: {:?}",
                    hints_text(&hints)
                );
            }
        }
    }
}

/// m8/15b (15 A2): at a stop the stop's own keys still lead; `n N` follow
/// them, ahead of the row's keys.
#[test]
fn at_a_stop_a_search_puts_next_and_prev_after_the_stops_keys() {
    let row = HintContext::PlainRow {
        verbs: RowVerbs::NONE,
    };
    for c in [LiveC::Finish, LiveC::Continue, LiveC::ContinueAmends] {
        let plain = HintContext::OperationLive {
            row: Box::new(row.clone()),
            c,
            tab_to_banner: true,
            space_stages: false,
        };
        let searching = HintContext::OperationLive {
            row: Box::new(HintContext::Searching(Box::new(row.clone()))),
            c,
            tab_to_banner: true,
            space_stages: false,
        };
        assert!(all_contexts().contains(&searching), "{searching:?} swept");
        let stop = hints_for(&plain, Divider::Hidden, 200);
        let bar = hints_for(&searching, Divider::Hidden, 200);
        let next = bar
            .iter()
            .position(|hint| hint.key() == "n")
            .unwrap_or_else(|| panic!("{c:?}: no n: {:?}", hints_text(&bar)));
        assert_eq!(bar[next + 1].key(), "N", "{c:?}: {:?}", hints_text(&bar));
        // Everything before `n` is the stop's own lead, in its order.
        let lead = &bar[..next];
        assert!(!lead.is_empty(), "{c:?}: the stop's keys lead");
        assert_eq!(lead, &stop[..next], "{c:?}: {:?}", hints_text(&bar));
        assert!(
            lead.iter().any(|hint| hint.key() == "c"),
            "{c:?}: {:?}",
            hints_text(&bar)
        );
    }
}

/// m8/20c R6' (B4): `space` on a CONFLICTS row reads `mark resolved` — a
/// verb, not `resolved` one letter off `⏎ resolve` — and at 80 columns its
/// short form `mark`, so `⇥ diff` and `? keys` are still drawn.
#[test]
fn a_conflict_row_says_mark_resolved() {
    let pane = HintContext::ChangesPane {
        unmerged: true,
        live: Some(LiveC::Continue),
        tab: TabWord::Diff,
        over_overlay: false,
    };
    let wide = hints_text(&hints_for(&pane, Divider::Hidden, 120));
    assert!(
        wide.contains("space mark resolved · c continue · ⇥ diff"),
        "{wide}"
    );
    let narrow = hints_text(&hints_for(&pane, Divider::Hidden, 80));
    assert_eq!(
        narrow,
        " ⏎ resolve · M mergetool · e edit · space mark · c cont · ⇥ diff · ? keys"
    );
    assert_eq!(super::MARK_RESOLVED, "mark resolved");
    assert_eq!(super::MARK_RESOLVED_SHORT, "mark");
}
