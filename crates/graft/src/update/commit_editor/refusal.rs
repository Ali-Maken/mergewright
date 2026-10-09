//! Why `c` in the editor is refused while an operation is live, and the words that say so.

use graft_ui::keymap::{ActionId, Keymap};

use super::{AppState, InProgress, Status};

/// Who began the operation git is part-way through — which decides whether
/// the operation banner's `c` and `a` can end it (m6-07 F12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Begun {
    /// Graft did: its journal entry is live, and the banner's keys end it.
    InGraft,
    /// The shell did: no entry of Graft's is live. The banner's `c` finishes
    /// a merge (as its own `FinishMerge` entry, m6-conflicts/20); every other
    /// key, and `c` on any other kind, refuses it (`Refusal::NotInProgress`)
    /// — that is finished in the shell.
    InShell,
}

/// The operation git is part-way through and who began it, as
/// [`CommitEditor::refusal`] needs it; `None` when nothing is in progress.
///
/// "Graft began it" is `AppState::live_entries` being non-empty — the last
/// journal read's `Pending` entries git is still inside, the same fact the
/// banner's `c`/`a` look up in `graft_ops` before they run. Until the first
/// journal read lands it is empty, so an operation a previous run of Graft
/// left is worded as the shell's for that moment: the refusal then names
/// git's own command, which still works, rather than a key that might not.
#[must_use]
pub fn live_operation(state: &AppState) -> Option<(InProgress, Begun)> {
    let kind = state.in_progress.as_ref()?.kind;
    let begun = if state.live_entries.is_empty() {
        Begun::InShell
    } else {
        Begun::InGraft
    };
    Some((kind, begun))
}

/// Whether `c` in the graph or the changes pane is the banner's `c` while
/// `live` is (m6-conflicts/52): true where the banner's `c` acts — any
/// merge, any rebase or cherry-pick (one the shell began is handed to
/// `git <kind> --continue`, m7-power-tools/07c), and a revert Graft began.
///
/// False for a bisect (the banner's `c` does nothing there, silently) and
/// for a revert the shell began (ops refuses it only after the command is
/// sent): there `c` keeps
/// [`operation_refusal`]'s words, which say what to do instead.
///
/// The one place this is decided: the key route (`update::keys`) and the
/// hint bar (`view::rows`) both read it, so the bar offers `c` exactly
/// where `c` acts.
#[must_use]
pub(crate) fn pane_c_acts(live: (InProgress, Begun)) -> bool {
    match live {
        // A rebase or cherry-pick the shell began: `c` hands the terminal
        // to `git <kind> --continue` (m7-power-tools/07c).
        (InProgress::Merge | InProgress::Rebase | InProgress::CherryPick, _) => true,
        (InProgress::Revert, begun) => begun == Begun::InGraft,
        (InProgress::Bisect, _) => false,
    }
}

/// Why a commit cannot start while an operation is live: a reason, and the
/// keys that lead on from it (m6-conflicts/48).
///
/// One value makes both the editor's row and the pane's `Sticky` status, so
/// the two cannot drift.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationRefusal {
    /// What is live: `merge in progress`, `merge begun in the shell`, or a
    /// whole sentence where there are no keys.
    pub reason: String,
    /// `None` where there is no banner to point at (bisect; a shell-begun
    /// rebase, cherry-pick or revert).
    pub keys: Option<String>,
}

impl OperationRefusal {
    /// The editor's row: `{reason} — esc, {keys}`, or `reason` alone. `Esc`
    /// first, because the editor is modal and holds every key while open.
    #[must_use]
    pub fn editor_words(&self) -> String {
        match &self.keys {
            Some(keys) => format!("{} — esc, {keys}", self.reason),
            None => self.reason.clone(),
        }
    }

    /// The pane's answer: `Status::Sticky { reason, keys }`, the `u`
    /// refusal's variant.
    #[must_use]
    pub fn status(&self) -> Status {
        Status::Sticky {
            reason: self.reason.clone(),
            keys: self.keys.clone(),
        }
    }
}

/// The refusal for a live operation. Finishing a merge, rebase, cherry-pick
/// or revert Graft began is the operation banner's (keymap §2.6a); `⇥` leads
/// there from the panes, and the keys name the shorter way from where the
/// keyboard is (`way`, [`refusal_way`]; m6-conflicts/70, m8/20d).
///
/// The banner's keys are spelled from `keys` (m8/10f): his keymap for the
/// pane's status, `Keymap::shipped()` for the editor's row, which is drawn
/// while its text field is open (10b W4). An unbound key drops its part;
/// none bound, no keys at all (W3).
#[must_use]
pub fn operation_refusal(
    (kind, begun): (InProgress, Begun),
    way: crate::update::banner::BannerWay,
    keys: &Keymap,
) -> OperationRefusal {
    // No key reaches the banner (`app.next_pane` and `app.prev_pane` unbound): its keys name
    // nothing he can press from here, so the reason stands alone (W3).
    let to_banner = crate::update::banner::way_keys(way, keys);
    let then = |parts: &[(&'static str, &str)]| {
        let words = keys.key_verbs(parts);
        let to_banner = to_banner.as_deref()?;
        (!words.is_empty()).then(|| format!("{to_banner}, then {words}"))
    };
    let (reason, keys) = match (kind, begun) {
        // A bisect draws no banner — that is M7's — so there are no keys to
        // name and git's own command is still the way out.
        (InProgress::Bisect, _) => (
            format!(
                "{} in progress — {} first",
                kind.name(),
                kind.continue_command()
            ),
            None,
        ),
        // A merge begun in the shell: the banner's `c` finishes it since
        // m6-conflicts/20. Its `a` still refuses one Graft did not start
        // (`resume_target`), so no `a abort`.
        (InProgress::Merge, Begun::InShell) => (
            "merge begun in the shell".to_owned(),
            then(&[("operation.continue", "finishes it")]),
        ),
        // A rebase or cherry-pick begun in the shell: `c` hands it to git
        // (m7-power-tools/07c).
        (InProgress::Rebase | InProgress::CherryPick, Begun::InShell) => (
            match keys.key_verb(&[ActionId("operation.continue")], "hands it to git") {
                Some(hint) => format!("{} begun in the shell — {hint}", kind.name()),
                None => format!("{} begun in the shell", kind.name()),
            },
            None,
        ),
        // Any other begun in the shell has no entry of Graft's; the banner's
        // keys refuse it (m6-07 F12), so the way out is git's own command.
        (_, Begun::InShell) => (
            format!(
                "{} begun in the shell — finish it there: {}",
                kind.name(),
                kind.continue_command()
            ),
            None,
        ),
        // On a merge the banner's `c` opens the finish summary, so it is
        // named `c finish`, as the banner's own bar names it (n2).
        (InProgress::Merge, Begun::InGraft) => (
            format!("{} in progress", kind.name()),
            then(&[
                ("operation.continue", "finish"),
                ("operation.abort", "abort"),
            ]),
        ),
        (_, Begun::InGraft) => (
            format!("{} in progress", kind.name()),
            then(&[
                ("operation.continue", "continue"),
                ("operation.abort", "abort"),
            ]),
        ),
    };
    OperationRefusal { reason, keys }
}

/// The way a refusal names from where the keyboard is: the one reading of
/// the cycle ([`crate::update::banner::banner_way`]). With the editor open
/// it is the way from the pane under it, which `esc` hands the keyboard
/// back to.
///
/// `None` there means no banner is drawn — a bisect, or a non-merge begun
/// in the shell — and those arms of [`operation_refusal`] name no keys, so
/// the one `⇥` it falls back to is never drawn.
#[must_use]
pub fn refusal_way(state: &AppState) -> crate::update::banner::BannerWay {
    crate::update::banner::banner_way(state).unwrap_or(crate::update::banner::BannerWay {
        forward: 1,
        back: None,
    })
}
