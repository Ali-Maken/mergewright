//! [`ConfirmYes`]: the confirm box's acting button, by its label (moved
//! out of `context.rs`, ease-pass/12).

/// The acting button `y` confirms in a confirm box (ADR-0044 §3.1,
/// m6-conflicts/50): one per label a `y`-live box draws at index 1.
///
/// Built from the button's own label ([`ConfirmYes::from_label`]), never
/// chosen beside it, so the bar's `y <word>` and the button cannot name two
/// different acts: an untracked file's box reads `[ Delete ]`, its bar
/// `y delete` (conductor's decision (a), 2026-09-25).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmYes {
    /// `[ Push ]`, the plain push box.
    Push,
    /// `[ Drop ]`, the stash-drop box.
    Drop,
    /// `[ Discard ]`, a tracked file's or a hunk's discard box.
    Discard,
    /// `[ Delete ]`, an untracked file's discard box.
    Delete,
    /// `[ Skip ]`, the banner's skip box.
    Skip,
    /// `[ Abort ]`, the banner's abort box.
    Abort,
    /// `[ Start over ]`, the resolver's restart box.
    StartOver,
    /// `[ Undo ]`, the undo drift guard.
    Undo,
    /// `[ Redo ]`, the redo drift guard.
    Redo,
    /// `[ Quit ]`, the box `q` opens while a write runs or git is stopped
    /// (m8-polish-and-ship/13).
    Quit,
}

impl ConfirmYes {
    /// Every value, in declaration order: what the sweeps iterate.
    pub const ALL: [Self; 10] = [
        Self::Push,
        Self::Drop,
        Self::Discard,
        Self::Delete,
        Self::Skip,
        Self::Abort,
        Self::StartOver,
        Self::Undo,
        Self::Redo,
        Self::Quit,
    ];

    /// The button's label exactly as the box draws it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Push => "Push",
            Self::Drop => "Drop",
            Self::Discard => "Discard",
            Self::Delete => "Delete",
            Self::Skip => "Skip",
            Self::Abort => "Abort",
            Self::StartOver => "Start over",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Quit => "Quit",
        }
    }

    /// The word after `y` on the bar: the button's label, lowercased.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Drop => "drop",
            Self::Discard => "discard",
            Self::Delete => "delete",
            Self::Skip => "skip",
            Self::Abort => "abort",
            Self::StartOver => "start over",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::Quit => "quit",
        }
    }

    /// The value whose [`label`](Self::label) is `label`, or `None` for a
    /// label no `y`-live box draws — and then `y` does not act either, so
    /// the bar never names a key that would not.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|yes| yes.label() == label)
    }
}
