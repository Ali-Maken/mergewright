//! The names the app leaves on disk in a repository: its ref namespace, its
//! directory under `.git`, and the trailer line on its backup commits.
//!
//! The app was renamed (ADR-0057 §2), and a repository it has used before
//! still carries the old names until the one-time move runs (ADR-0057,
//! Amendment 2026-10-09). Which set is in force is decided once per
//! repository, at open, from whether [`MIGRATED_MARKER`] exists; every reader
//! and writer of the prefix, the directory or the trailer takes them from
//! the [`OnDiskNames`] it was handed, never from a literal.
//!
//! Plain data: nothing here touches the disk.

/// Which set of on-disk names a repository is using.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnDiskNames {
    /// The new names: `refs/tributary/`, `…/tributary/`. In force once the
    /// marker exists.
    New,
    /// The names written before the rename: `refs/graft/`, `…/graft/`. In
    /// force while no marker exists.
    Old,
}

impl OnDiskNames {
    /// Both sets, new first. For questions that must answer for either —
    /// "is this one of our refs", "is this one of our trailers".
    pub const ALL: [OnDiskNames; 2] = [OnDiskNames::New, OnDiskNames::Old];

    /// The ref namespace the app writes its own refs under, with the
    /// trailing slash: `refs/tributary/` or `refs/graft/`.
    #[must_use]
    pub const fn ref_prefix(self) -> &'static str {
        match self {
            OnDiskNames::New => "refs/tributary/",
            OnDiskNames::Old => "refs/graft/",
        }
    }

    /// The directory name the app keeps its files under, inside `.git`
    /// (and inside the config root): `tributary` or `graft`. One path
    /// segment, no slash.
    #[must_use]
    pub const fn dir(self) -> &'static str {
        match self {
            OnDiskNames::New => "tributary",
            OnDiskNames::Old => "graft",
        }
    }

    /// The key of the line a backup commit's message ends with, colon
    /// included: `Tributary-Link:` or `Graft-Link:`. Backup commits are
    /// immutable, so the old key is read forever; only the new one is
    /// written.
    #[must_use]
    pub const fn link_trailer(self) -> &'static str {
        match self {
            OnDiskNames::New => "Tributary-Link:",
            OnDiskNames::Old => "Graft-Link:",
        }
    }
}

/// The variable that names the running app's askpass socket. git passes it
/// through to the helper; its presence is what selects askpass mode (design
/// note m4, choice 2: an argument cannot ride in `GIT_ASKPASS` reliably).
/// Here with the other on-disk names (ADR-0057 section 2) so the crate that
/// sets it and the binary that reads it share one definition.
pub const ASKPASS_SOCKET_VAR: &str = "TRIB_ASKPASS_SOCKET";

/// The marker file whose existence puts [`OnDiskNames::New`] in force. It
/// lives in `<common_dir>/<OnDiskNames::New.dir()>/`; its one line reads
/// `switched` or `clean`.
pub const MIGRATED_MARKER: &str = "migrated";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_set_spells_its_own_names() {
        assert_eq!(OnDiskNames::New.ref_prefix(), "refs/tributary/");
        assert_eq!(OnDiskNames::Old.ref_prefix(), "refs/graft/");
        assert_eq!(OnDiskNames::New.dir(), "tributary");
        assert_eq!(OnDiskNames::Old.dir(), "graft");
        assert_eq!(OnDiskNames::New.link_trailer(), "Tributary-Link:");
        assert_eq!(OnDiskNames::Old.link_trailer(), "Graft-Link:");
    }

    #[test]
    fn the_prefix_is_the_dir_under_refs() {
        for names in OnDiskNames::ALL {
            assert_eq!(names.ref_prefix(), format!("refs/{}/", names.dir()));
        }
    }
}
