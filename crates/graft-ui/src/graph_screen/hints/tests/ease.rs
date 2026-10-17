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

fn clash(at: ClashAt, op: ClashOp) -> HintContext {
    HintContext::Clash(ClashBar { at, op })
}

/// Each row kind leads with its mock's words, at 120 columns; the bar's
/// own frame (today's tail and `? keys`) follows (Q11 a).
#[test]
fn each_row_bar_leads_with_the_mock_words_at_120_columns() {
    let cases = [
        (row(RowBar::Own), " ⏎ open · P push · c commit · "),
        (
            row(elsewhere("sara/fix-toc")),
            " m merge fix-toc into yours · ⏎ open · C switch to fix-toc · x compare · ",
        ),
        (
            row(RowBar::FoldedMerge { inside: 1 }),
            " space show the 1 commit inside · ⏎ open · x compare · ",
        ),
        (
            row(RowBar::FoldedMerge { inside: 3 }),
            " space show the 3 commits inside · ⏎ open · x compare · ",
        ),
        (row(RowBar::Opened), " space hide the commits inside · "),
        (
            row(RowBar::JustCommitted),
            " u undo the commit, your edits come back · P push · c commit again · ",
        ),
        (
            row(RowBar::JustMerged { inside: 3 }),
            " u undo the merge · P push · space show the 3 commits inside · ",
        ),
    ];
    for (context, lead) in cases {
        let drawn = bar(&context, 120);
        assert!(drawn.starts_with(lead), "{context:?}: {drawn:?}");
        assert!(drawn.ends_with("? keys"), "{context:?}: {drawn:?}");
    }
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

/// The live merge's banner after `esc pause` leads `⏎ back to the
/// clashes` (Q7 a); on first landing, and for every other operation, the
/// banner still leads `⏎ resolve`: he has not been to the clashes yet.
#[test]
fn the_paused_merge_banner_leads_back_to_the_clashes() {
    let banner = |merge: bool| HintContext::Banner {
        skippable: !merge,
        conflicted: true,
        merge,
    };
    let paused = bar(&HintContext::BannerPaused, 120);
    assert!(
        paused.starts_with(" ⏎ back to the clashes  c finish"),
        "{paused}"
    );
    assert!(bar(&banner(true), 120).starts_with(" ⏎ resolve  c finish"));
    assert!(bar(&banner(false), 120).starts_with(" ⏎ resolve  c continue"));
}

/// Below the resolver's floor the clash screen draws one line and answers
/// `esc` alone, so its bar says only that (m6-conflicts/04's rule).
#[test]
fn a_clash_bar_below_the_resolver_floor_says_only_close() {
    let narrow = crate::conflict_resolver::MIN_COLUMNS - 1;
    let drawn = bar(&clash(ClashAt::Open, ClashOp::Merge), narrow);
    assert_eq!(drawn.trim(), "esc close");
}

/// The clash screen's bars, for a merge (M11) and for a rebase (Q12 a).
#[test]
fn each_clash_bar_draws_its_words() {
    let next = || ClashAt::FileSolved {
        next: graft_term::sanitize("report.rs"),
    };
    let rebase = ClashOp::Other(SideRoles::Rebase);
    let cases = [
        (
            clash(ClashAt::Open, ClashOp::Merge),
            " o take yours (left) · t take theirs (right) · b take both · e edit by hand · \
             u undo pick · esc pause, come back later",
        ),
        (
            clash(ClashAt::Picked, ClashOp::Merge),
            " n next open clash · u undo pick",
        ),
        (
            clash(next(), ClashOp::Merge),
            " n next file: report.rs · u undo pick · esc pause, come back later",
        ),
        (
            clash(ClashAt::AllSolved, ClashOp::Merge),
            " c finish · u undo pick · esc pause, come back later",
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
            " ⏎ done · n next file: report.rs · u undo pick · esc pause, come back later",
        ),
        (
            clash(ClashAt::AllSolved, rebase),
            " ⏎ done · c continue · u undo pick · esc pause, come back later",
        ),
    ];
    for (context, want) in cases {
        assert_eq!(bar(&context, 200), want, "{context:?}");
    }
}
