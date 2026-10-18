//! The operation banner's text (moved out of `mod.rs` by m7-power-tools/07b):
//! [`BannerView`], the plain data `graft` fills each frame, and
//! [`banner_line`], the words the header draws for it.

use mergewright_term::SanitizedText;

use crate::keymap::{ActionId, Keymap};

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
    /// A rewrite's stop, read from git's todo and Mergewright's saved plan
    /// (m7-power-tools/07b): what the step is, and which branch of the line
    /// it belongs to. `None` for every other operation, and for a rebase
    /// whose progress could not be read — those draw today's words.
    pub rewrite: Option<RewriteBannerView>,
    /// A merge's `(from, into)` branch names (ease-pass/11b, §1.13 edit):
    /// the banner reads `merging <from> into <into>` in place of
    /// `MERGING`. `None` for every other operation, and for a merge whose
    /// names are not read yet — those keep `kind`.
    pub names: Option<(SanitizedText, SanitizedText)>,
}

/// What a rewrite's stop adds to the banner after `REBASING  step i of n`:
/// `· pick 7f6e5d4 Handle expired tokens · feature-b, branch 2 of 3`, and at
/// an `edit` stop the hint that the graph has the keyboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteBannerView {
    /// git's action word for the row it stopped on: `pick`, `edit`, …
    pub action: String,
    /// The stopped row's commit, short.
    pub sha7: String,
    /// The stopped row's subject, sanitised by `graft`.
    pub subject: String,
    /// Whose step this is: a branch of the line, or a fork replayed with it.
    /// `None` for a single branch outside a fork, and for a rebase begun in
    /// the shell.
    pub chain: Option<BannerChain>,
    /// An `edit` stop: adds `change files, space stages, a amends it, c
    /// continues · ⇥ banner`.
    pub edit_hint: bool,
}

/// The chain words of [`RewriteBannerView`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BannerChain {
    /// `feature-b, branch 2 of 3`: the branch of the line, its place
    /// (1-based) and how many branches the line has.
    Line {
        /// The branch, short.
        branch: String,
        /// Its place, 1-based.
        index: usize,
        /// How many branches the line has.
        of: usize,
    },
    /// `feature-x, forked at a1b2c3d` (ADR-0059 §3, m8/14c): a fork that
    /// follows, in its section of the rebase.
    Fork {
        /// The fork, short.
        branch: String,
        /// The old commit it leaves the line at, short.
        at: String,
    },
}

impl BannerChain {
    fn words(&self) -> String {
        match self {
            Self::Line { branch, index, of } => format!("{branch}, branch {index} of {of}"),
            Self::Fork { branch, at } => format!("{branch}, forked at {at}"),
        }
    }
}

/// `banner`'s text in `width` cells (the banner's own room, after the
/// header's prefix): `MERGING`, `REBASING step 3 of 7`, or at a rewrite's
/// stop `REBASING  step 3 of 7 · pick 7f6e5d4 Handle expired tokens ·
/// feature-b, branch 2 of 3`.
///
/// **Without a rewrite, one wording at every width**: `width` is not read.
/// Two earlier defects were one defect — a `width >= 100` tier chose between
/// `— conflict in notes.md` and `— conflict notes.md`, so the same state
/// spelled itself two ways and the *wider* terminal got the terser one
/// (m5/20 item 3). Those words are short enough to fit everywhere.
///
/// **A rewrite's words are a ladder** (m7-power-tools/07b, note R4): they are
/// long, and a cut `…` would take the step count's neighbours at random. As
/// the room shrinks they drop whole, in this order: the edit hint (the hint
/// bar says the same keys), the subject, the chain words, the action with
/// its sha7. `REBASING  step i of n` is never dropped; the header cuts it
/// with `…` only below its own width, as it cuts any text. Each rung is the
/// same facts, fewer of them — never the same fact spelled two ways.
///
/// No `⚠` either: a stopped merge is a state to finish, not a failure, and
/// CLAUDE.md's rule is that a status field must never read as an error when
/// nothing is wrong. `⚠` stays for the Mistake Radar's real findings
/// (`GraphScreen::marker_glyph`, `OverviewStrip`, the Map screen), which is
/// what keeps it meaning something.
///
/// The edit hint's keys are spelled from `keys` ([`edit_hint`], m8/09d).
#[must_use]
pub fn banner_line(banner: &BannerView, width: u16, keys: &Keymap) -> String {
    let Some(rewrite) = banner.rewrite.as_ref() else {
        let mut line = match &banner.names {
            Some((from, into)) => format!("merging {} into {}", from.as_str(), into.as_str()),
            None => String::from(banner.kind.as_str()),
        };
        if let Some((step, total)) = banner.step {
            line.push_str(&format!(" step {step} of {total}"));
        }
        return line;
    };
    let mut base = String::from(banner.kind.as_str());
    if let Some((step, total)) = banner.step {
        base.push_str(&format!("  step {step} of {total}"));
    }
    let row = format!("{} {}", rewrite.action, rewrite.sha7);
    let chain = rewrite.chain.as_ref().map(BannerChain::words);
    let hint = rewrite.edit_hint.then(|| edit_hint(keys));
    // Widest first; each rung drops the next piece: the hint, the subject,
    // the chain words. Below the last, the row itself goes (`base`).
    let rungs = [
        (true, true, true),
        (true, true, false),
        (false, true, false),
        (false, false, false),
    ];
    for (with_subject, with_chain, with_hint) in rungs {
        let mut line = format!("{base} · {row}");
        if with_subject && !rewrite.subject.is_empty() {
            line.push(' ');
            line.push_str(&rewrite.subject);
        }
        if let Some(chain) = chain.as_ref().filter(|_| with_chain) {
            line.push_str(" · ");
            line.push_str(chain);
        }
        if let Some(hint) = hint.as_ref().filter(|_| with_hint) {
            line.push_str(" · ");
            line.push_str(hint);
        }
        if super::walk_text::text_width(&line) <= usize::from(width) {
            return line;
        }
    }
    base
}

/// What an `edit` stop's banner adds: the graph has the keyboard, and these
/// are the keys that finish the step — `change files, space stages, a
/// amends it, c continues · ⇥ banner`. Each key is the graph row it is at
/// an `edit` stop (`space` is `graph.fold`'s key, which stages there,
/// `mergewright::update::keys::edit_stop`; `c` is `graph.commit`'s, the banner's
/// `c`); an unbound one's clause is left out (m8/09d).
fn edit_hint(keys: &Keymap) -> String {
    let steps: Vec<String> = std::iter::once(Some("change files".to_owned()))
        .chain([
            keys.key_verb(&[ActionId("graph.fold")], "stages"),
            keys.key_verb(&[ActionId("graph.amend")], "amends it"),
            keys.key_verb(&[ActionId("graph.commit")], "continues"),
        ])
        .flatten()
        .collect();
    keys.after(&steps.join(", "), ActionId("app.next_pane"), "banner")
}

#[cfg(test)]
mod tests;
