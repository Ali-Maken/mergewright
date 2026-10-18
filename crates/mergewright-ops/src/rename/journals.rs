//! A copied journal's backups checked against the new refs.

use std::fs;
use std::path::PathBuf;

use mergewright_core::on_disk::Generation;

use super::trees::JOURNAL_FILE;
use super::{MigrateError, map_entries, refs::Listed};
use crate::journal;

/// Every backup a copied journal names, read under the new names, is a
/// ref the create transaction left under the new prefix (`new_refs`, read
/// back in step 3) — review m8-26 Minor 5.
pub(super) fn check_journals(copied: &[PathBuf], new_refs: &Listed) -> Result<(), MigrateError> {
    for path in copied
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == JOURNAL_FILE))
    {
        let bytes = fs::read(path).map_err(|error| MigrateError::files(path, &error))?;
        let mut entries = match journal::parse(&bytes) {
            Ok(parsed) => parsed.entries,
            // The copy is byte-equal to its original (read back above), so
            // a journal that does not parse was already so under the old
            // names; the journal's own reader says so on the next read,
            // exactly as it would have. Refusing here would only strand
            // the move on it for good.
            Err(_) => continue,
        };
        map_entries(&mut entries, Generation::CURRENT);
        for backup in entries.iter().filter_map(|entry| entry.backup_ref.as_ref()) {
            if !new_refs.contains_key(backup) {
                return Err(MigrateError::ReadBack {
                    what: format!("{} (names {backup}, not there)", path.display()),
                });
            }
        }
    }
    Ok(())
}
