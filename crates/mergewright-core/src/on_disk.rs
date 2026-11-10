//! The names the app writes to disk and shows: its ref namespace, its
//! directory under `.git` and the config root, the trailer on its backup
//! commits, the tag on the messages it writes into git, its title, command
//! and environment prefix. Earlier names are not read (ADR-0057, Amendment
//! 2026-10-25). A later rename edits these consts. Write them qualified
//! (`on_disk::DIR`), never glob-imported.
//!
//! Plain data: nothing here touches the disk.

/// The variable that names the running app's askpass socket. git passes it
/// through to the helper; its presence is what selects askpass mode (design
/// note m4, choice 2: an argument cannot ride in `GIT_ASKPASS` reliably).
/// Here with the other on-disk names (ADR-0057 section 2) so the crate that
/// sets it and the binary that reads it share one definition.
pub const ASKPASS_SOCKET_VAR: &str = "MWR_ASKPASS_SOCKET";

/// The tag this build puts at the start of every message it writes into
/// git: a set-aside stash (`<tag>: your edits, set aside for …`), a backup
/// commit (`<tag> <id> <label>`), the index, absorb and forecast commits.
/// A const, not a fn: mergewright-core exports no free functions (ADR-0007).
pub const MESSAGE_TAG: &str = DIR;

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
