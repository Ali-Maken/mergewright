//! The ease pass's bar words (ticket ease-pass/03b): the row bars, the
//! clash screen's bars and the live merge's banner, words exactly the mocks'
//! (03's Handoff), keys spelled from the shipped keymap.

use super::*;

fn bar(context: &HintContext, columns: u16) -> String {
    hints_text(&hints_for(context, Divider::Hidden, columns))
}

fn row(row: RowBar) -> HintContext {
    HintContext::GraphRow {
        row,
        verbs: RowVerbs::NONE,
    }
}

fn elsewhere(branch: &str) -> RowBar {
    RowBar::Elsewhere {
        branch: graft_term::sanitize(branch),
    }
}

fn clash(at: ClashAt, roles: SideRoles) -> HintContext {
    HintContext::Clash(ClashBar { at, roles })
}

/// Each row kind's whole bar at 120 columns: its mock's words, then the
/// row bar's tail (`u undo`, deduped where the words lead with `u`) and the
/// bar end, whole at this width (`. actions · ? keys`; `. more` is its
/// short rung, Q11 a). `OpenedMerge` is the merge row
/// whose fold is open (mock `mergewright-main-screen.html:206`).
#[test]
fn each_row_bar_draws_the_mock_bar_at_120_columns() {
    let cases = [
        (
            row(RowBar::Own),
            " ⏎ open · P push · c commit · u undo · . actions · ? keys",
        ),
        (
            row(elsewhere("sara/fix-toc")),
            " m merge fix-toc into yours · ⏎ open · C switch to fix-toc · x compare · u undo · . actions · ? keys",
        ),
        (
            row(RowBar::FoldedMerge { inside: 1 }),
            " space show the 1 commit inside · ⏎ open · x compare · u undo · . actions · ? keys",
        ),
        (
            row(RowBar::FoldedMerge { inside: 3 }),
            " space show the 3 commits inside · ⏎ open · x compare · u undo · . actions · ? keys",
        ),
        (
            row(RowBar::OpenedMerge),
            " space hide the commits inside · ⏎ open · x compare · u undo · . actions · ? keys",
        ),
        (
            row(RowBar::JustCommitted),
            " u undo the commit, your edits come back · P push · c commit again · . actions · ? keys",
        ),
        (
            row(RowBar::JustMerged { inside: 3 }),
            " u undo the merge · P push · space show the 3 commits inside · . actions · ? keys",
        ),
    ];
    for (context, want) in cases {
        assert_eq!(bar(&context, 120), want, "{context:?}");
    }
}

/// The opened merge's short rung of its first hint is `space hide`.
#[test]
fn the_opened_merge_bar_shortens_to_space_hide() {
    let hints = hints_for(&row(RowBar::OpenedMerge), Divider::Hidden, 120);
    assert_eq!(hints[0].at(Rung::Short).drawn_verb(), Some("hide"));
}

/// The bar names a branch by its last path segment (Q2 a).
#[test]
fn a_bar_names_a_branch_by_its_last_path_segment() {
    let name = |full: &str| bar_name(&graft_term::sanitize(full)).to_owned();
    assert_eq!(name("sara/fix-toc"), "fix-toc");
    assert_eq!(name("main"), "main");
    assert_eq!(name("team/sara/fix-toc"), "fix-toc");
}

/// A 60-cell branch name at 60 columns: whole hints only. The named verbs
/// fall to their short rung (`m merge`, `C switch`) or are dropped; no hint
/// carries a piece of the name, and the bar fits.
#[test]
fn a_long_name_at_60_columns_draws_whole_hints_only() {
    let long = format!("team/{}", "n".repeat(60));
    let context = row(elsewhere(&long));
    let hints = hints_for(&context, Divider::Hidden, 60);
    let drawn = hints_text(&hints);
    assert!(drawn.width() <= 60, "{drawn:?}");
    assert!(
        !drawn.contains('n'.to_string().repeat(2).as_str()),
        "{drawn:?}"
    );
    for hint in &hints {
        if hint.verb.contains("nnnn") {
            assert_eq!(hint.rung, Rung::Short, "{hint:?}");
        }
    }
    assert!(drawn.starts_with(" m merge · ⏎ open · "), "{drawn:?}");
    assert!(drawn.ends_with(" ? keys"), "{drawn:?}");
}

/// The short rung of every hint is a prefix of its whole verb — the one
/// rule of `short_form`, held by the named verbs too.
#[test]
fn every_short_rung_is_a_prefix_of_its_verb() {
    for hint in every_hint() {
        let short = hint.at(Rung::Short);
        let words = short.drawn_verb().expect("a short rung draws words");
        assert!(hint.verb.starts_with(words), "{hint:?}");
    }
}

/// The banner after `esc pause` leads `⏎ back to the clashes` for a merge
/// and for every other operation (Q7 a, Q17 a), and is otherwise the
/// conflicted banner's list; on first landing the banner still leads
/// `⏎ resolve`: he has not been to the clashes yet.
#[test]
fn the_paused_banner_leads_back_to_the_clashes() {
    for merge in [true, false] {
        let banner = HintContext::Banner {
            skippable: !merge,
            conflicted: true,
            merge,
        };
        let paused = bar(&HintContext::BannerPaused { merge }, 120);
        assert!(paused.starts_with(" ⏎ back to the clashes  c "), "{paused}");
        assert_eq!(
            paused.replacen("⏎ back to the clashes", "⏎ resolve", 1),
            bar(&banner, 120),
        );
    }
    let first = HintContext::Banner {
        skippable: true,
        conflicted: true,
        merge: false,
    };
    assert!(bar(&first, 120).starts_with(" ⏎ resolve  c continue"));
}

/// The paused banner finds `⏎` by its key, not by its place: a list with
/// `⏎` off index 0 gets it replaced where it stands.
#[test]
fn the_paused_banner_replaces_open_by_identity() {
    use super::super::keys::{ABORT, OPEN};
    let list = vec![hint(ABORT, "abort"), hint(OPEN, "resolve")];
    let paused = super::super::lists::paused(list);
    assert!(paused[0].is(ABORT));
    assert!(paused[1].is(OPEN));
    assert_eq!(paused[1].verb, "back to the clashes");
}

/// Below the resolver's floor the clash screen draws one line and answers
/// `esc` alone, so its bar says only that — in the word its `esc` has at
/// every width, `pause` (review ease-pass-03, Minor 1). The resolver keeps
/// `close`.
#[test]
fn a_clash_bar_below_the_resolver_floor_says_only_pause() {
    let narrow = crate::conflict_resolver::MIN_COLUMNS - 1;
    let drawn = bar(&clash(ClashAt::Open, SideRoles::Merge), narrow);
    assert_eq!(drawn.trim(), "esc pause");
}

/// Only a merge's roles make a merge's clash bar.
#[test]
fn a_clash_bar_is_a_merge_only_for_the_merge_roles() {
    for roles in [
        SideRoles::Merge,
        SideRoles::Rebase,
        SideRoles::CherryPick,
        SideRoles::Revert,
        SideRoles::StashPop,
        SideRoles::SetAside,
    ] {
        let bar = ClashBar {
            at: ClashAt::Open,
            roles,
        };
        assert_eq!(bar.is_merge(), roles == SideRoles::Merge, "{roles:?}");
    }
}

/// A file in the clash bar is its last path segment (mock
/// `mergewright-merge-screen.html:150`), and a rebase's finished bar draws
/// no `y`.
#[test]
fn a_clash_bar_names_the_next_file_by_its_last_segment() {
    let at = ClashAt::FileSolved {
        next: graft_term::sanitize("docs/notes.md"),
    };
    let drawn = bar(&clash(at, SideRoles::Merge), 200);
    assert!(drawn.starts_with(" n next file: notes.md · "), "{drawn}");
    let done = bar(&clash(ClashAt::AllSolved, SideRoles::Rebase), 200);
    assert!(!done.contains(" y "), "{done}");
}

/// A solved file's bar ends with the way back to its clashes, `←/→
/// clashes` (Q19 a, user 2026-10-17; ticket ease-pass/05r): every clash is
/// taken, `n` goes on, and the arrows step back through them. A bar over an
/// open or just-picked clash does not draw it.
#[test]
fn a_solved_file_bar_ends_with_the_way_back_to_its_clashes() {
    let next = || ClashAt::FileSolved {
        next: graft_term::sanitize("report.rs"),
    };
    for roles in [SideRoles::Merge, SideRoles::Rebase] {
        for at in [next(), ClashAt::AllSolved] {
            let drawn = bar(&clash(at.clone(), roles), 200);
            assert!(
                drawn.ends_with(" · ←/→ clashes"),
                "{at:?} {roles:?}: {drawn}"
            );
        }
        for at in [ClashAt::Open, ClashAt::Picked] {
            let drawn = bar(&clash(at.clone(), roles), 200);
            assert!(!drawn.contains("clashes"), "{at:?} {roles:?}: {drawn}");
        }
    }
}

/// The clash screen's bars, for a merge (M11) and for a rebase (Q12 a).
#[test]
fn each_clash_bar_draws_its_words() {
    let next = || ClashAt::FileSolved {
        next: graft_term::sanitize("report.rs"),
    };
    let rebase = SideRoles::Rebase;
    let cases = [
        (
            clash(ClashAt::Open, SideRoles::Merge),
            " o take yours (left) · t take theirs (right) · b take both · e edit by hand · \
             u undo pick · esc pause, come back later",
        ),
        (
            clash(ClashAt::Picked, SideRoles::Merge),
            " n next open clash · u undo pick",
        ),
        (
            clash(next(), SideRoles::Merge),
            " n next file: report.rs · u undo pick · esc pause, come back later · ←/→ clashes",
        ),
        (
            clash(ClashAt::AllSolved, SideRoles::Merge),
            " y review the merge · u undo pick · esc pause, come back later · ←/→ clashes",
        ),
        (
            clash(ClashAt::Open, rebase),
            " o take upstream (left) · t take yours (right) · b take both · e edit by hand · \
             u undo pick · esc pause, come back later",
        ),
        (
            clash(ClashAt::Picked, rebase),
            " n next open clash · u undo pick",
        ),
        (
            clash(next(), rebase),
            " ⏎ done · n next file: report.rs · u undo pick · esc pause, come back later · \
             ←/→ clashes",
        ),
        (
            clash(ClashAt::AllSolved, rebase),
            " ⏎ done · c continue · u undo pick · esc pause, come back later · ←/→ clashes",
        ),
    ];
    for (context, want) in cases {
        assert_eq!(bar(&context, 200), want, "{context:?}");
    }
}

/// The review's bar (ticket ease-pass/12, 03 Handoff, mock
/// `mergewright-merge-screen.html:162`): its words whole at 120 columns,
/// keys spelled from the shipped keymap (`j/k`, as every two-key hint is
/// spelled); narrower, each hint steps down to its one-word rung before
/// any drops, and `u` is kept second (`fitting::fit_order`). Never a word
/// cut.
#[test]
fn the_review_bar_draws_the_mock_words() {
    let review = HintContext::Review(ReviewBar::List);
    let hints = [
        "y commit the merge",
        "j/k move",
        "⏎ look at it",
        "u undo last pick",
        "esc back to the clashes (y brings you here again)",
    ];
    assert_eq!(bar(&review, 120), format!(" {}", hints.join(" · ")));
    // The review draws from 62 columns (`FINISH_MIN`), its border 58 of them.
    let cases = [
        (
            58,
            " y commit the merge · j/k move · ⏎ look · u undo last pick",
        ),
        (
            80,
            " y commit the merge · j/k move · ⏎ look at it · u undo last pick · esc back",
        ),
    ];
    for (columns, want) in cases {
        assert_eq!(bar(&review, columns), want, "{columns}");
    }
}
