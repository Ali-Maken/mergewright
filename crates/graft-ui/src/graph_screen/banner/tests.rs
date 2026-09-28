//! The rewrite banner's words and its ladder (m7-power-tools/07b).

use graft_term::sanitize_line;

use super::{BannerView, RewriteBannerView, banner_line};

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
            chain: chain.then(|| ("feature-b".to_owned(), 2, 3)),
            edit_hint: edit,
        }),
    }
}

const WIDE: u16 = 400;

#[test]
fn a_rewrite_stop_says_the_step_the_row_and_the_branch() {
    assert_eq!(
        banner_line(&rewrite(true, false), WIDE),
        "REBASING  step 3 of 7 · pick 5d4c3b2 wip: tests · feature-b, branch 2 of 3"
    );
}

#[test]
fn a_single_branch_rewrite_has_no_chain_words() {
    assert_eq!(
        banner_line(&rewrite(false, false), WIDE),
        "REBASING  step 3 of 7 · pick 5d4c3b2 wip: tests"
    );
}

#[test]
fn an_edit_stop_adds_its_hint() {
    assert_eq!(
        banner_line(&rewrite(false, true), WIDE),
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
    let full = banner_line(&banner, WIDE);
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
        assert_eq!(&banner_line(&banner, fits(wider)), wider);
        // …and one cell less draws the next one down.
        assert_eq!(&banner_line(&banner, fits(wider) - 1), narrower);
    }
    assert_eq!(banner_line(&banner, 3), "REBASING  step 3 of 7");
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
        };
        for width in [0u16, 10, 80, WIDE] {
            assert_eq!(banner_line(&banner, width), want, "{kind} at {width}");
        }
    }
}
