//! The keys mergewright-ops' words tell him to press, named by what they do — never
//! by chord (ticket m8/10e, ADR-0055: the app spells them from his keymap).
//!
//! mergewright-ops sits below the UI and cannot name a keymap, so every wording
//! that names a key takes a [`KeyNames`] and asks it for the label. A key he
//! unbound has no label, and the clause naming it is dropped whole (10b
//! Ruling W3) — never a wrong letter, never a `—`. [`NoKeys`] answers `None`
//! for every key: the keyless words a `Display` or a log line gets.

/// A key a mergewright-ops wording names, by what it does where he presses it.
/// Exhaustive: the app's map to its keymap rows has no catch-all, so adding
/// one fails to compile until it is mapped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpKey {
    /// Commit (`c` in the graph).
    Commit,
    /// Amend, or put staged changes into an older commit (`a` in the graph).
    Amend,
    /// Continue the operation in progress (`c` on the banner).
    Continue,
    /// Abort the operation in progress (`a` on the banner).
    Abort,
    /// Stage the selected change (`space` in the changes pane).
    Stage,
    /// Stash (`S` in the graph).
    Stash,
    /// Switch branch (`C` in the graph).
    Switch,
    /// Open the rewrite editor (`R` in the graph).
    Rewrite,
    /// Fetch (`f`).
    Fetch,
    /// Pull (`p`).
    Pull,
    /// Push, a force with lease included (`P`).
    Push,
    /// Merge (`m`).
    Merge,
    /// Rebase (`r`).
    Rebase,
    /// Undo (`u`).
    Undo,
    /// Redo, and putting back a failed undo's backup (`U`).
    Redo,
}

impl OpKey {
    /// Every key, in declaration order.
    pub const ALL: &'static [OpKey] = &[
        OpKey::Commit,
        OpKey::Amend,
        OpKey::Continue,
        OpKey::Abort,
        OpKey::Stage,
        OpKey::Stash,
        OpKey::Switch,
        OpKey::Rewrite,
        OpKey::Fetch,
        OpKey::Pull,
        OpKey::Push,
        OpKey::Merge,
        OpKey::Rebase,
        OpKey::Undo,
        OpKey::Redo,
    ];
}

/// Spells an [`OpKey`] as he has it bound.
pub trait KeyNames {
    /// His label for `key`; `None` when unbound (W3).
    fn label(&self, key: OpKey) -> Option<String>;
}

/// Every key unbound: the keyless words, for a `Display` and logs.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoKeys;

impl KeyNames for NoKeys {
    fn label(&self, _key: OpKey) -> Option<String> {
        None
    }
}

/// `<label> <rest>`, or `None` when `key` is unbound.
pub(crate) fn clause(keys: &dyn KeyNames, key: OpKey, rest: &str) -> Option<String> {
    keys.label(key).map(|label| format!("{label} {rest}"))
}

/// The bound alternatives joined by `sep` (`c continues, a aborts`), or
/// `None` when none is bound.
pub(crate) fn either(parts: &[Option<String>], sep: &str) -> Option<String> {
    let bound: Vec<&str> = parts.iter().flatten().map(String::as_str).collect();
    (!bound.is_empty()).then(|| bound.join(sep))
}

/// `c continues, a aborts`, keeping the bound ones (W3): the banner's two
/// ways out of an operation in progress.
pub(crate) fn continue_or_abort(keys: &dyn KeyNames) -> Option<String> {
    either(
        &[
            clause(keys, OpKey::Continue, "continues"),
            clause(keys, OpKey::Abort, "aborts"),
        ],
        ", ",
    )
}

/// `words`, then each clause that is there after ` · `.
pub(crate) fn then(words: String, clauses: impl IntoIterator<Item = Option<String>>) -> String {
    clauses.into_iter().flatten().fold(words, |mut line, part| {
        line.push_str(" · ");
        line.push_str(&part);
        line
    })
}

/// Today's letters, for mergewright-ops' own tests (the one test impl; never
/// production — the app spells from his keymap).
#[cfg(test)]
pub(crate) struct Letters;

#[cfg(test)]
impl KeyNames for Letters {
    fn label(&self, key: OpKey) -> Option<String> {
        Some(
            match key {
                OpKey::Commit | OpKey::Continue => "c",
                OpKey::Amend | OpKey::Abort => "a",
                OpKey::Stage => "space",
                OpKey::Stash => "S",
                OpKey::Switch => "C",
                OpKey::Rewrite => "R",
                OpKey::Fetch => "f",
                OpKey::Pull => "p",
                OpKey::Push => "P",
                OpKey::Merge => "m",
                OpKey::Rebase => "r",
                OpKey::Undo => "u",
                OpKey::Redo => "U",
            }
            .to_owned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_keys_spells_nothing() {
        assert!(OpKey::ALL.iter().all(|key| NoKeys.label(*key).is_none()));
    }

    #[test]
    fn an_unbound_clause_is_dropped_and_alternatives_keep_the_bound() {
        let line = then(
            "a merge has nothing to skip".to_owned(),
            [either(
                &[
                    clause(&NoKeys, OpKey::Continue, "continues"),
                    clause(&Letters, OpKey::Abort, "aborts"),
                ],
                ", ",
            )],
        );
        assert_eq!(line, "a merge has nothing to skip · a aborts");
        let none = then(
            "x".to_owned(),
            [either(
                &[clause(&NoKeys, OpKey::Continue, "continues")],
                ", ",
            )],
        );
        assert_eq!(none, "x");
    }
}
