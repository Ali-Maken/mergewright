//! `every_context` cannot quietly miss a shape (m5-history-and-ghost-preview/33,
//! review M3). Split out of `graph_screen/tests.rs` (ease-pass/07b).

use super::*;

/// Shapes [`coverage_slot`] can answer with — one per variant and per
/// boolean field combination (54 → 63 in m6-conflicts/50: the confirm box
/// once per `ConfirmYes`; 111 → 112 in m8-polish-and-ship/13: `Quit`, whose
/// place after `Redo` (63) was already the changes pane's; 116 → 113 in
/// m8/11e: `Journal`, `Stash` and `Map` deleted, their slots refilled; 113 → 114 in m8/15 A4: `StashRow`; 114 → 115 in m8/15b: `Searching`; 115 → 119 in m8/20d-b: one `⇧⇥` to the banner; 119 → 120 in m8/47: `PickerOnCurrent`; 120 → 134 in ease-pass/03b: `GraphRow` per
/// `RowBar` (120–125), `Clash` per `ClashAt` × op (126–133); → 136 in 03r).
/// The merge review's list (ease-pass/12): 136; its look (12b): 137.
/// → 136 in ease-pass/07b: `BranchRow` retired; the review took its 1–2.
const COVERAGE_SLOTS: usize = 136;

/// The slot `context` takes in [`every_context_reaches_every_shape`]'s tally.
///
/// **Exhaustive on the enum and on every boolean field**, which is the whole
/// point: a variant added to `HintContext`, or a field combination added to
/// one, stops this crate compiling until it is given a slot here — and then
/// the test below fails until `every_context` actually produces one.
///
/// `Marked` and `PaneFocused` are tallied by the wrapper alone. What they
/// wrap is not a shape of its own: `every_context` builds them by wrapping
/// the plain list, so a plain shape that reaches the list reaches its
/// wrapped forms too.
/// [`coverage_slot`]'s live-operation forms.
fn operation_live_slot(context: &HintContext) -> usize {
    match context {
        // m6-conflicts/52: each `c` × one `⇥` from the banner or not. The
        // form every live bar had before 52 keeps its slot, 29; the other
        // five are 49–53.
        // m7-power-tools/07b: an `edit` stop's `a amend it` (90–91).
        // m8/20d-b: one `⇧⇥` back — each `c` (115–117), `a amend it` (118).
        HintContext::OperationLive {
            c: super::LiveC::ContinueAmends,
            way_to_banner: Some(super::WayToBanner::Back),
            ..
        } => 118,
        HintContext::OperationLive {
            c,
            way_to_banner: Some(super::WayToBanner::Back),
            ..
        } => 115 + live_c_slot(*c),
        HintContext::OperationLive {
            c: super::LiveC::ContinueAmends,
            way_to_banner,
            ..
        } => 90 + usize::from(way_to_banner.is_none()),
        HintContext::OperationLive {
            c, way_to_banner, ..
        } => match 2 * live_c_slot(*c) + usize::from(way_to_banner.is_none()) {
            0 => 29,
            other => 48 + other,
        },
        _ => unreachable!("only live-operation forms come here"),
    }
}

/// A commit row's slot: `base` (1 teammate, 2 own, 3 plain) bare, else
/// 102–110. Bare own/teammate `GraphRow`s are `ease_slot`'s 120–121.
fn row_slot(base: usize, verbs: RowVerbs) -> usize {
    let combo = 2 * usize::from(verbs.amend) + usize::from(verbs.take);
    if combo == 0 {
        base
    } else {
        102 + 3 * (base - 1) + (combo - 1)
    }
}

/// The ease pass's bars (03b): `GraphRow` per row kind (120–125), `Clash`
/// per state and op (126–133), the paused banner per operation (134–135).
fn ease_slot(context: &HintContext) -> usize {
    use super::hints::{ClashAt, RowBar};
    match context {
        HintContext::GraphRow {
            row: row @ (RowBar::Own | RowBar::Elsewhere { .. }),
            verbs,
        } if *verbs != RowVerbs::NONE => row_slot(1 + usize::from(*row == RowBar::Own), *verbs),
        HintContext::GraphRow { row, .. } => {
            120 + match row {
                RowBar::Own => 0,
                RowBar::Elsewhere { .. } => 1,
                RowBar::FoldedMerge { .. } => 2,
                RowBar::OpenedMerge => 3,
                RowBar::JustCommitted => 4,
                RowBar::JustMerged { .. } => 5,
            }
        }
        HintContext::Clash(bar) => {
            let at = match bar.at {
                ClashAt::Open => 0,
                ClashAt::Picked => 1,
                ClashAt::FileSolved { .. } => 2,
                ClashAt::AllSolved => 3,
            };
            126 + 2 * at + usize::from(!bar.is_merge())
        }
        HintContext::BannerPaused { merge } => 134 + usize::from(*merge),
        _ => 134,
    }
}

fn coverage_slot(context: &HintContext) -> usize {
    match context {
        HintContext::WorkingTreeRow => 0,
        // m7-power-tools/40: each row shape once per `RowVerbs`; the bare
        // plain row keeps 3, the other three per shape are 102–110 (07b: the
        // own/teammate `GraphRow`s' 102–107 are in `ease_slot`).
        HintContext::PlainRow { verbs } => row_slot(3, *verbs),
        // m8-polish-and-ship/15 A4.
        HintContext::StashRow => 113,
        // Both fields are plain `bool`s, so the arm stays exhaustive.
        // `merge` is `!skippable` in every form a live operation makes
        // (m6-conflicts/70), so it takes no slot of its own.
        HintContext::Banner {
            skippable,
            conflicted,
            merge: _,
        } => 4 + 2 * usize::from(*skippable) + usize::from(*conflicted),
        HintContext::Preview => 8,
        HintContext::ChangesPane { .. } => changes_pane_slot(context),
        HintContext::Dragging => 13,
        HintContext::Marked(_) => 14,
        // m8-polish-and-ship/15b: tallied by the wrapper, as `Marked` is.
        HintContext::Searching(_) => 114,
        // 15, 16, 73, 79, 80.
        HintContext::PaneFocused { .. } => pane_focused_slot(context),
        // m7-power-tools/20: the blame place's four flag pairs (74–77), the
        // way back to it (78); the same four without lines — a read in
        // flight or a failed one (81–84, m7-power-tools/20k, 20n).
        HintContext::Blame {
            finding,
            ignore_file,
            lines,
        } => {
            let pair = 2 * usize::from(*finding) + usize::from(*ignore_file);
            if *lines { 74 + pair } else { 81 + pair }
        }
        HintContext::BackToBlame(_) => 78,
        // m7-power-tools/21b: the history place's three reachable forms
        // (85–87; `rows: false` draws one bar whatever the focus) and the way
        // back to it (88).
        HintContext::History {
            rows: true,
            on_file,
        } => 85 + usize::from(*on_file),
        HintContext::History { rows: false, .. } => 87,
        HintContext::BackToHistory(_) => 88,
        // m7-power-tools/10: the branch picker; on HEAD's own row 119 (m8/47).
        HintContext::Picker => 89,
        HintContext::PickerOnCurrent => 119,
        // m7-power-tools/08: the rewrite editor's four flag pairs (96–99), its field (17, once
        // 100), `R rewrite` on a commit row (112, once 101; both moved in m8/11e).
        HintContext::RewriteEditor { narrow, can_start } => {
            96 + 2 * usize::from(*narrow) + usize::from(*can_start)
        }
        HintContext::RewriteMessage => 17,
        HintContext::Rewritable(_) => 112,
        HintContext::PaneOverOverlay { opens } => 18 + usize::from(*opens),
        HintContext::Resolver { .. } => resolver_slot(context),
        HintContext::GraphRow { .. } | HintContext::Clash(_) | HintContext::BannerPaused { .. } => {
            ease_slot(context)
        }
        HintContext::ConfirmBox { yes } => confirm_slot(*yes),
        HintContext::NamePrompt { toggle } => 26 + usize::from(*toggle),
        HintContext::Credential => 28,
        HintContext::OperationLive { .. } => operation_live_slot(context),
        HintContext::Finish { .. } | HintContext::Review(_) => finish_slot(context),
    }
}

/// [`coverage_slot`]'s finish forms: m6's (32–35), the review's list and look
/// (1–2, `BranchRow`'s old slots). Out of `coverage_slot` for clippy's line limit, as [`ease_slot`].
fn finish_slot(context: &HintContext) -> usize {
    use crate::finish_summary::SummaryFocus;
    match context {
        // Only the files focus reads `expandable`.
        HintContext::Finish {
            focus: SummaryFocus::Files,
            expandable,
        } => 32 + usize::from(*expandable),
        HintContext::Finish {
            focus: SummaryFocus::Subject,
            ..
        } => 34,
        HintContext::Finish {
            focus: SummaryFocus::Body,
            ..
        } => 35,
        HintContext::Review(ReviewBar::List) => 1,
        HintContext::Review(ReviewBar::Look) => 2,
        // `coverage_slot` sends only the forms above; out of the tally's
        // range, so a wrong call fails loudly.
        _ => usize::MAX,
    }
}

/// [`coverage_slot`] for the focused pane (m7-power-tools/20h split it out).
fn pane_focused_slot(context: &HintContext) -> usize {
    match context {
        HintContext::PaneFocused {
            opens: false,
            blamable: false,
            folds: false,
            ..
        } => 15,
        HintContext::PaneFocused {
            opens: true,
            blamable: false,
            folds: false,
            ..
        } => 16,
        // m7-power-tools/20: `B blame` on the pane, the diff focused (73).
        // m7-power-tools/20h: `B` with the files list on a file (79), and a
        // directory row's `space fold` (80). `folds` with `blamable` or
        // without `opens` is no shape the app makes (`PaneFocused::folds`),
        // so those arms share 80 rather than hold slots nothing can reach.
        HintContext::PaneFocused {
            opens: false,
            blamable: true,
            folds: false,
            ..
        } => 73,
        HintContext::PaneFocused {
            opens: true,
            blamable: true,
            folds: false,
            ..
        } => 79,
        HintContext::PaneFocused { folds: true, .. } => 80,
        _ => unreachable!("only a `PaneFocused` is passed"),
    }
}

/// [`coverage_slot`] for the changes pane (m6-conflicts/70 split it out).
fn changes_pane_slot(context: &HintContext) -> usize {
    match context {
        // m8/09e (11b row 553): over the Map, journal or stash list, `live`
        // is not read (no `c` in any form) — `⇥ graph` 100–101, `⇥ diff`
        // 11–12, each over `unmerged` (112–115 until m8/11e retired slots
        // 11, 12 and 17 with the journal, stash and Map contexts).
        HintContext::ChangesPane {
            unmerged,
            tab: TabWord::Graph | TabWord::Banner,
            over_overlay: true,
            ..
        } => 100 + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            tab: TabWord::Diff,
            over_overlay: true,
            ..
        } => 11 + usize::from(*unmerged),
        // Nothing live: `⇥ graph` from the diff keeps 9–10; `⇥ diff` from
        // the file list is 63–64 (m6-conflicts/70). `⇥ banner` with nothing
        // live is not a screen, and shares 9–10.
        HintContext::ChangesPane {
            unmerged,
            live: None,
            tab: TabWord::Graph | TabWord::Banner,
            over_overlay: false,
        } => 9 + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: None,
            tab: TabWord::Diff,
            over_overlay: false,
        } => 63 + usize::from(*unmerged),
        // m6-conflicts/52: each `c` an operation can make, over each
        // `unmerged`: from the file list, `⇥ diff` (43–48); from the diff
        // with a banner, `⇥ banner` (65–70); a bisect's `⇥ graph` (71–72,
        // which `c finish`/`c continue`, never bannerless, share).
        // m7-power-tools/07b: an `edit` stop's, from the list (92–93) and
        // from the diff (94–95).
        HintContext::ChangesPane {
            unmerged,
            live: Some(super::LiveC::ContinueAmends),
            tab: TabWord::Diff,
            over_overlay: false,
        } => 92 + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: Some(super::LiveC::ContinueAmends),
            tab: TabWord::Banner,
            over_overlay: false,
        } => 94 + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: Some(c),
            tab: TabWord::Diff,
            over_overlay: false,
        } => 43 + 2 * live_c_slot(*c) + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: Some(c),
            tab: TabWord::Banner,
            over_overlay: false,
        } => 65 + 2 * live_c_slot(*c) + usize::from(*unmerged),
        HintContext::ChangesPane {
            unmerged,
            live: Some(_),
            tab: TabWord::Graph,
            over_overlay: false,
        } => 71 + usize::from(*unmerged),
        _ => unreachable!("only called on a `ChangesPane`"),
    }
}

/// [`coverage_slot`] for the confirm box: 25 with no `y`, then one per
/// acting button `y` takes, 54–62 (m6-conflicts/50), and `Quit` at 111
/// (m8-polish-and-ship/13: 63 was taken).
fn confirm_slot(yes: Option<super::ConfirmYes>) -> usize {
    if yes == Some(super::ConfirmYes::Quit) {
        return 111;
    }
    yes.map_or(25, |yes| {
        54 + super::ConfirmYes::ALL
            .iter()
            .position(|each| *each == yes)
            .expect("ALL names every ConfirmYes")
    })
}

/// A [`LiveC`](super::LiveC)'s place in [`coverage_slot`]'s live ranges.
fn live_c_slot(c: super::LiveC) -> usize {
    match c {
        super::LiveC::Finish => 0,
        super::LiveC::Continue => 1,
        super::LiveC::Refuses => 2,
        super::LiveC::ContinueAmends => unreachable!("its slots are its own (90–95)"),
    }
}

/// [`coverage_slot`] for the resolver: done (30–31), document (20–23) or
/// card (24), and the same three shapes with `c` offered
/// (m6-conflicts/31): 36–37, 38–41, 42.
fn resolver_slot(context: &HintContext) -> usize {
    let HintContext::Resolver {
        base,
        restart,
        whole_file,
        done,
        finishable,
        ..
    } = *context
    else {
        panic!("not a resolver context: {context:?}");
    };
    match (done, whole_file, finishable) {
        (true, _, false) => 30 + usize::from(!restart),
        (true, _, true) => 36 + usize::from(!restart),
        (false, false, false) => 20 + usize::from(base) + 2 * usize::from(restart),
        (false, false, true) => 38 + usize::from(base) + 2 * usize::from(restart),
        (false, true, false) => 24,
        (false, true, true) => 42,
    }
}

/// `every_context`'s list is written by hand — Rust cannot enumerate a
/// type's values, and `Marked`/`PaneFocused` are recursive, so there is no
/// finite set to derive without choosing a depth by hand anyway. What the
/// compiler *can* do is refuse: [`coverage_slot`] matches exhaustively, so a
/// new shape cannot compile until it is named, and this test then fails
/// until the list actually produces one.
///
/// So adding a variant is still a human's edit. It is no longer a human's
/// **omission** (m5/33, review M3 — the gap row and the ticket notes used to
/// claim the compiler forced the list itself, which it does not).
#[test]
fn every_context_reaches_every_shape() {
    let mut seen = [false; COVERAGE_SLOTS];
    for context in every_context() {
        seen[coverage_slot(&context)] = true;
    }

    let missed: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter(|(_, hit)| !**hit)
        .map(|(slot, _)| slot)
        .collect();
    assert!(
        missed.is_empty(),
        "`every_context` never produces slot(s) {missed:?} of `coverage_slot` — a shape \
         the sweeps that read it will never see"
    );
}
