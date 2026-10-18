//! The rewrite banner's words and its ladder (m7-power-tools/07b).

use mergewright_term::sanitize_line;

use super::{BannerChain, BannerView, RewriteBannerView, banner_line};
use crate::keymap::Keymap;

fn rewrite(chain: bool, edit: bool) -> BannerView {
    BannerView {
        kind: sanitize_line("REBASING"),
        step: Some((3, 7)),
        conflict: None,
        focused: false,
        skippable: true,
        rewrite: Some(RewriteBannerView {
            action: if edit { "edit" } else { "pick" }.to_owned(),
            sha7: "5d4c3b2".to_owned(),
            subject: "wip: tests".to_owned(),
            chain: chain.then(|| BannerChain::Line {
                branch: "feature-b".to_owned(),
                index: 2,
                of: 3,
            }),
            edit_hint: edit,
        }),
        names: None,
    }
}

const WIDE: u16 = 400;

/// A stop inside a fork's section (m8/14c): the fork where the line's
/// branch would be, dropped on the same rung as the chain words.
#[test]
fn a_stop_inside_a_fork_says_the_fork() {
    let mut banner = rewrite(false, false);
    if let Some(rewrite) = banner.rewrite.as_mut() {
        rewrite.chain = Some(BannerChain::Fork {
            branch: "feature-x".to_owned(),
            at: "a1b2c3d".to_owned(),
        });
    }
    assert_eq!(
        banner_line(&banner, WIDE, Keymap::shipped()),
        "REBASING  step 3 of 7 · pick 5d4c3b2 wip: tests · feature-x, forked at a1b2c3d"
    );
    assert_eq!(
        banner_line(&banner, 50, Keymap::shipped()),
        "REBASING  step 3 of 7 · pick 5d4c3b2",
        "the chain words drop with the subject"
    );
}

#[test]
fn a_rewrite_stop_says_the_step_the_row_and_the_branch() {
    assert_eq!(
        banner_line(&rewrite(true, false), WIDE, Keymap::shipped()),
        "REBASING  step 3 of 7 · pick 5d4c3b2 wip: tests · feature-b, branch 2 of 3"
    );
}

#[test]
fn a_single_branch_rewrite_has_no_chain_words() {
    assert_eq!(
        banner_line(&rewrite(false, false), WIDE, Keymap::shipped()),
        "REBASING  step 3 of 7 · pick 5d4c3b2 wip: tests"
    );
}

#[test]
fn an_edit_stop_adds_its_hint() {
    assert_eq!(
        banner_line(&rewrite(false, true), WIDE, Keymap::shipped()),
        "REBASING  step 3 of 7 · edit 5d4c3b2 wip: tests · change files, space stages, \
         a amends it, c continues · \u{21e5} banner"
    );
}

/// As the room shrinks the banner drops, in order: the edit hint, the
/// subject, the chain words, the sha7 with its action. `REBASING  step i of
/// n` is never dropped (the header's own fit may still cut it with `…` below
/// its width, as it cuts any text).
#[test]
fn banner_ladder_drops_hint_subject_chain_sha_in_order() {
    let banner = rewrite(true, true);
    let full = banner_line(&banner, WIDE, Keymap::shipped());
    let rungs = [
        full.clone(),
        "REBASING  step 3 of 7 · edit 5d4c3b2 wip: tests · feature-b, branch 2 of 3".to_owned(),
        "REBASING  step 3 of 7 · edit 5d4c3b2 · feature-b, branch 2 of 3".to_owned(),
        "REBASING  step 3 of 7 · edit 5d4c3b2".to_owned(),
        "REBASING  step 3 of 7".to_owned(),
    ];
    for pair in rungs.windows(2) {
        let (wider, narrower) = (&pair[0], &pair[1]);
        let fits = |text: &str| u16::try_from(text.chars().count()).expect("short");
        // Exactly the room of a rung draws that rung…
        assert_eq!(&banner_line(&banner, fits(wider), Keymap::shipped()), wider);
        // …and one cell less draws the next one down.
        assert_eq!(
            &banner_line(&banner, fits(wider) - 1, Keymap::shipped()),
            narrower
        );
    }
    assert_eq!(
        banner_line(&banner, 3, Keymap::shipped()),
        "REBASING  step 3 of 7"
    );
}

/// Every banner without a rewrite reads as it did before 07b, at any width.
#[test]
fn non_rewrite_banners_are_unchanged() {
    for (kind, step, want) in [
        ("MERGING", None, "MERGING"),
        ("CHERRY-PICKING", Some((2, 5)), "CHERRY-PICKING step 2 of 5"),
        ("REVERTING", Some((1, 1)), "REVERTING step 1 of 1"),
        ("REBASING", Some((3, 7)), "REBASING step 3 of 7"),
    ] {
        let banner = BannerView {
            kind: sanitize_line(kind),
            step,
            conflict: None,
            focused: false,
            skippable: step.is_some(),
            rewrite: None,
            names: None,
        };
        for width in [0u16, 10, 80, WIDE] {
            assert_eq!(
                banner_line(&banner, width, Keymap::shipped()),
                want,
                "{kind} at {width}"
            );
        }
    }
}

/// m8/09d D5: an `edit` stop's words spell each key from his keymap —
/// `space` is `graph.fold`'s key (it stages there), `a` `graph.amend`'s,
/// `c` `graph.commit`'s, `⇥` `app.next_pane`'s — and an unbound key's
/// clause is left out, never `—`.
#[test]
fn the_edit_hint_spells_his_keys() {
    use crate::keymap::sweep::{assert_spelled, swept};
    let (keys, _) = swept();
    let line = banner_line(&rewrite(false, true), WIDE, &keys);
    let hint = &line[line.find("change files").unwrap_or(0)..];
    assert_spelled(
        hint,
        &keys,
        &[
            &["graph.fold"],
            &["graph.amend"],
            &["graph.commit"],
            &["app.next_pane"],
        ],
    );
    let (none, warnings) = Keymap::with_overrides(
        &[
            ("graph.fold".to_owned(), vec!["none".to_owned()]),
            ("app.next_pane".to_owned(), vec!["none".to_owned()]),
        ],
        crate::keymap::KeySwitches::default(),
    );
    assert_eq!(warnings, []);
    let line = banner_line(&rewrite(false, true), WIDE, &none);
    assert!(
        line.ends_with("change files, a amends it, c continues"),
        "{line}"
    );
    assert!(!line.contains('\u{2014}'), "{line}");
}

/// ease-pass/11b, §1.13 edit: a paused merge's banner names both branches,
/// at any width (the header cuts it, not the line).
#[test]
fn a_merge_banner_says_merging_from_into() {
    let banner = BannerView {
        kind: sanitize_line("MERGING"),
        step: None,
        conflict: Some(sanitize_line("report.md")),
        focused: false,
        skippable: false,
        rewrite: None,
        names: Some((
            sanitize_line("sara/fix-toc"),
            sanitize_line("wallet-release"),
        )),
    };
    for width in [0u16, 80, WIDE] {
        assert_eq!(
            banner_line(&banner, width, Keymap::shipped()),
            "merging sara/fix-toc into wallet-release",
            "at {width}"
        );
    }
}
