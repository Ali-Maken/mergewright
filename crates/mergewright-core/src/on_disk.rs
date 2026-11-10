//! Every name the app has shipped under, one row per [`Generation`]: its ref
//! namespace, its directory under `.git` and the config root, the trailer on
//! its backup commits, the tag on the messages it writes into git, its
//! title, command and environment prefix.
//!
//! The app has been renamed twice (ADR-0057 §2 and its Amendments of
//! 2026-10-09 and 2026-10-11). A repository it has used before still carries
//! an earlier generation's names until the one-time move runs. Which
//! generation is in force is decided once per repository, at open, from the
//! markers ([`MIGRATED_MARKER`]); every reader and writer of a prefix, a
//! directory or a trailer takes it from the [`Generation`] it was handed or
//! from [`Generation::CURRENT`], never from a literal. A later rename is one
//! new row and a flip of `CURRENT`.
//!
//! Plain data: nothing here touches the disk.

/// One set of names the app has shipped under, oldest first: the derived
/// `Ord` is the release order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Generation {
    /// The first names: `refs/graft/`, `…/graft/`. Needs no marker: with
    /// none present, these are in force.
    Graft,
    /// `refs/tributary/`, `…/tributary/`.
    Tributary,
    /// `refs/mergewright/`, `…/mergewright/`, command `mwr`.
    Mergewright,
}

impl Generation {
    /// The names this build writes. Every generation after it is
    /// unreachable: nothing reads or writes its names.
    pub const CURRENT: Self = Generation::Mergewright;

    /// Every generation, oldest first.
    pub const GENERATIONS: [Self; 3] = [
        Generation::Graft,
        Generation::Tributary,
        Generation::Mergewright,
    ];

    /// The directory name the app keeps its files under, inside `.git` and
    /// inside the config root. One path segment, no slash. Also the tag on
    /// the messages the app writes into git, and the backup author's name.
    #[must_use]
    pub const fn dir(self) -> &'static str {
        match self {
            Generation::Graft => "graft",
            Generation::Tributary => "tributary",
            Generation::Mergewright => "mergewright",
        }
    }

    /// The product word, as the user reads it.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Generation::Graft => "Graft",
            Generation::Tributary => "Tributary",
            Generation::Mergewright => "Mergewright",
        }
    }

    /// The command the user types.
    #[must_use]
    pub const fn command(self) -> &'static str {
        match self {
            Generation::Graft => "graft",
            Generation::Tributary => "trib",
            Generation::Mergewright => "mwr",
        }
    }

    /// The prefix of the app's environment variables, underscore included.
    #[must_use]
    pub const fn env_prefix(self) -> &'static str {
        match self {
            Generation::Graft => "GRAFT_",
            Generation::Tributary => "TRIB_",
            Generation::Mergewright => "MWR_",
        }
    }

    /// The ref namespace the app writes its own refs under, with the
    /// trailing slash: `refs/<dir>/`.
    #[must_use]
    pub const fn ref_prefix(self) -> &'static str {
        match self {
            Generation::Graft => "refs/graft/",
            Generation::Tributary => "refs/tributary/",
            Generation::Mergewright => "refs/mergewright/",
        }
    }

    /// The key of the line a backup commit's message ends with, colon
    /// included: `<Title>-Link:`. Backup commits are immutable, so every
    /// generation's key is read forever; only `CURRENT`'s is written.
    #[must_use]
    pub const fn link_trailer(self) -> &'static str {
        match self {
            Generation::Graft => "Graft-Link:",
            Generation::Tributary => "Tributary-Link:",
            Generation::Mergewright => "Mergewright-Link:",
        }
    }

    /// The email of the identity backup commits carry: `<dir>@backup`. Its
    /// name is [`dir`](Self::dir). Spelled per row because a `const` cannot
    /// join strings.
    #[must_use]
    pub const fn backup_email(self) -> &'static str {
        match self {
            Generation::Graft => "graft@backup",
            Generation::Tributary => "tributary@backup",
            Generation::Mergewright => "mergewright@backup",
        }
    }

    /// Whether this generation puts itself in force with a marker at
    /// `<common_dir>/<dir>/`[`MIGRATED_MARKER`]. False for Graft only: with
    /// no marker present, Graft is in force.
    #[must_use]
    pub const fn has_marker(self) -> bool {
        !matches!(self, Generation::Graft)
    }

    /// The generations released before this one, newest first.
    pub fn older(self) -> impl Iterator<Item = Self> {
        Self::GENERATIONS
            .into_iter()
            .rev()
            .filter(move |earlier| *earlier < self)
    }

    // The three readers below are associated fns, not the free
    // `generation_of_*` the ticket sketched: mergewright-core exports no free fn
    // (ADR-0007, `tests::core_has_no_free_functions`).

    /// The generation whose ref namespace `name` lies under, if any.
    #[must_use]
    pub fn of_ref(name: &str) -> Option<Self> {
        Self::GENERATIONS
            .into_iter()
            .find(|generation| name.starts_with(generation.ref_prefix()))
    }

    /// The generation whose backup-commit trailer key starts `line`, if any.
    #[must_use]
    pub fn of_trailer(line: &str) -> Option<Self> {
        Self::GENERATIONS
            .into_iter()
            .find(|generation| line.starts_with(generation.link_trailer()))
    }

    /// The generation that tagged a message the app wrote into git (a
    /// stash, a backup commit, a set-aside index): the message starts
    /// `<dir>:` or `<dir> `.
    #[must_use]
    pub fn of_message(message: &str) -> Option<Self> {
        Self::GENERATIONS.into_iter().find(|generation| {
            message
                .strip_prefix(generation.dir())
                .is_some_and(|rest| rest.starts_with([':', ' ']))
        })
    }
}

/// The variable that names the running app's askpass socket. git passes it
/// through to the helper; its presence is what selects askpass mode (design
/// note m4, choice 2: an argument cannot ride in `GIT_ASKPASS` reliably).
/// Here with the other on-disk names (ADR-0057 section 2) so the crate that
/// sets it and the binary that reads it share one definition.
pub const ASKPASS_SOCKET_VAR: &str = "MWR_ASKPASS_SOCKET";

/// The marker file whose existence puts a generation with
/// [`Generation::has_marker`] in force. It lives in
/// `<common_dir>/<generation.dir()>/`; its one line reads `switched` or
/// `clean`.
pub const MIGRATED_MARKER: &str = "migrated";

/// The tag this build puts at the start of every message it writes into
/// git: a set-aside stash (`<tag>: your edits, set aside for …`), a backup
/// commit (`<tag> <id> <label>`), the index, absorb and forecast commits.
/// [`Generation::CURRENT`]'s dir, so a rename moves it; the messages an
/// earlier generation wrote stay as they are and [`Generation::of_message`]
/// reads them all (ADR-0057 amendment 2026-10-11 §1).
/// A const, not a fn: mergewright-core exports no free functions (ADR-0007).
pub const MESSAGE_TAG: &str = Generation::CURRENT.dir();

/// [`Generation::CURRENT`]'s folder name, for the paths this build writes:
/// short enough to spell inline (`.git/{CURRENT_DIR}/...`).
pub const CURRENT_DIR: &str = Generation::CURRENT.dir();

// The current-only names (ticket pre-publish-cleanup/11b1). Plain consts:
// a later rename edits these lines. They sit beside the `Generation` table
// until 11b4 deletes it; `tests::consts_match_current_generation` pins them
// equal to its `CURRENT` row meanwhile. Write them qualified
// (`on_disk::DIR`), never glob-imported.

/// The folder the app keeps its files under, inside `.git` and the config
/// root; also the tag on its git messages and the backup author's name.
pub const DIR: &str = "mergewright";
/// The product word, as the user reads it.
pub const TITLE: &str = "Mergewright";
/// The command the user types.
pub const COMMAND: &str = "mwr";
/// The prefix of the app's environment variables, underscore included.
pub const ENV_PREFIX: &str = "MWR_";
/// The ref namespace the app writes its own refs under, trailing slash.
pub const REF_PREFIX: &str = "refs/mergewright/";
/// The key a backup commit's last line starts with, colon included.
pub const LINK_TRAILER: &str = "Mergewright-Link:";
/// The email of the identity backup commits carry.
pub const BACKUP_EMAIL: &str = "mergewright@backup";

/// Tests whether a name, message or trailer is this app's own. A unit
/// struct, not free fns: mergewright-core exports no free functions
/// (ADR-0007, `tests::core_has_no_free_functions`).
#[derive(Debug, Clone, Copy)]
pub struct Own;

impl Own {
    /// Whether ref `name` lies under [`REF_PREFIX`]. Bytes, so a name that
    /// is not UTF-8 is judged by its prefix like any other.
    #[must_use]
    pub fn is_ref(name: &[u8]) -> bool {
        name.starts_with(REF_PREFIX.as_bytes())
    }

    /// Whether a message in git was written by this app: it starts
    /// `<DIR>:` or `<DIR> `.
    #[must_use]
    pub fn is_message(message: &str) -> bool {
        message
            .strip_prefix(DIR)
            .is_some_and(|rest| rest.starts_with([':', ' ']))
    }

    /// Whether `line` starts with [`LINK_TRAILER`].
    #[must_use]
    pub fn is_trailer(line: &str) -> bool {
        line.starts_with(LINK_TRAILER)
    }
}

#[cfg(test)]
mod tests;
