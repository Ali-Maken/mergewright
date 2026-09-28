//! The operation banner's text (moved out of `mod.rs` by m7-power-tools/07b):
//! [`BannerView`], the plain data `graft` fills each frame, and
//! [`banner_line`], the words the header draws for it.

use graft_term::SanitizedText;

/// The operation banner: `REBASING step 3 of 7` — the state and the step,
/// said once ([`banner_line`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerView {
    /// The operation's name, upper-case: `REBASING`, `MERGING`,
    /// `CHERRY-PICKING`, `REVERTING`.
    pub kind: SanitizedText,
    /// The step git is on and how many there are, when the operation has
    /// steps (a rebase/cherry-pick/revert sequence). `None` for a plain
    /// merge, which has none to count.
    pub step: Option<(u32, u32)>,
    /// The first conflicted path, once a conflict has landed. `None` before
    /// that — an in-progress operation the user has not hit trouble in yet.
    ///
    /// **Never drawn.** It is read by `hint_context` alone, to decide
    /// whether `M mergetool` and `e edit` have anything to act on. The
    /// banner used to name it, four rows above a changes pane listing every
    /// conflicted file under `CONFLICTS (n)` — the same fact twice on a
    /// 24-row screen (m5/20 item 1, the user's own fix).
    pub conflict: Option<SanitizedText>,
    /// The banner has taken focus (a conflict just landed, ADR-0037 §3):
    /// its slot draws reversed so a key-driven user notices without a
    /// click.
    pub focused: bool,
    /// `s` (skip) is offered — a rebase/cherry-pick/revert sequence can
    /// skip a step; a plain merge cannot. Affects only the hint bar
    /// (`hint-bar-redesign`'s group, not this one's `graph_screen`
    /// footer) — never the banner's own text or buttons.
    pub skippable: bool,
}

/// `banner`'s text: `MERGING`, or `REBASING step 3 of 7` when the operation
/// counts steps.
///
/// **One wording at every width**, and no `width` to word it to. The two
/// earlier defects here were one defect: a `width >= 100` tier chose between
/// `— conflict in notes.md` and `— conflict notes.md`, and the width it was
/// given is the banner's own region, which is 120 cells on a 120-column
/// screen (detail pane below) and about 92 on a 160-column one (detail pane
/// beside). So the same repository in the same state spelled itself two
/// ways, and the *wider* terminal got the terser spelling. Counting from
/// the banner's own room was right (ADR-0039's amendment); wording a
/// fourteen-cell fact in two ways was not. The text is now short enough to
/// fit everywhere, so there is nothing left to tier.
///
/// No `⚠` either: a stopped merge is a state to finish, not a failure, and
/// CLAUDE.md's rule is that a status field must never read as an error when
/// nothing is wrong. `⚠` stays for the Mistake Radar's real findings
/// (`GraphScreen::marker_glyph`, `OverviewStrip`, the Map screen), which is
/// what keeps it meaning something.
#[must_use]
pub fn banner_line(banner: &BannerView) -> String {
    let mut line = String::from(banner.kind.as_str());
    if let Some((step, total)) = banner.step {
        line.push_str(&format!(" step {step} of {total}"));
    }
    line
}
