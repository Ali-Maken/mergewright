//! The app's own refs under one prefix: listed, created under the other in
//! one transaction and read back, deleted at their expected values.

use std::collections::BTreeMap;

use graft_core::oid::Oid;
use graft_core::on_disk::OnDiskNames;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation};

use super::{MigrateError, map_ref};

/// Full ref name to value.
pub(super) type Listed = BTreeMap<String, Oid>;

fn refused(error: &impl std::fmt::Display) -> MigrateError {
    MigrateError::Refs {
        words: error.to_string(),
    }
}

/// Every ref under `names`' prefix — one `for-each-ref`.
pub(super) fn list(repo: &RepoHandle, names: OnDiskNames) -> Result<Listed, MigrateError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Word("--format=%(objectname)%00%(refname)".to_owned()),
            GitArg::Word(names.ref_prefix().to_owned()),
        ])
        .run(&())
        .map_err(|error| refused(&error))?;
    let text = std::str::from_utf8(&output.stdout).map_err(|error| refused(&error))?;
    let mut listed = Listed::new();
    for line in text.lines() {
        let Some((hex, name)) = line.split_once('\0') else {
            return Err(refused(&format!("for-each-ref printed {line:?}")));
        };
        let oid = Oid::from_hex(hex).map_err(|_| refused(&format!("not an object id: {hex:?}")))?;
        listed.insert(name.to_owned(), oid);
    }
    Ok(listed)
}

/// Create each of `old` under the new prefix in one transaction — none if
/// any cannot be — then read every one back. A new ref already at the old
/// value is left as it is (an earlier move stopped after creating it); at
/// another value it is refused, never overwritten. The command line run,
/// for the journal; empty when nothing needed creating.
pub(super) fn create_and_read_back(
    repo: &RepoHandle,
    old: &Listed,
) -> Result<String, MigrateError> {
    if old.is_empty() {
        return Ok(String::new());
    }
    let new = list(repo, OnDiskNames::New)?;
    let mut lines = String::new();
    for (name, oid) in old {
        let wanted = map_ref(name, OnDiskNames::New);
        match new.get(&*wanted) {
            Some(held) if held == oid => {}
            Some(held) => {
                return Err(refused(&format!(
                    "{wanted} already holds {held}, not {name}'s {oid}"
                )));
            }
            None => lines.push_str(&format!("create {wanted} {oid}\n")),
        }
    }
    let command = if lines.is_empty() {
        String::new()
    } else {
        transaction(repo, lines)?
    };
    let back = list(repo, OnDiskNames::New)?;
    for (name, oid) in old {
        let wanted = map_ref(name, OnDiskNames::New);
        if back.get(&*wanted) != Some(oid) {
            return Err(MigrateError::ReadBack {
                what: wanted.into_owned(),
            });
        }
    }
    Ok(command)
}

/// Delete every ref in `listed`, each only if it still holds its listed
/// value — one transaction, all or none.
pub(super) fn delete(repo: &RepoHandle, listed: &Listed) -> Result<(), MigrateError> {
    if listed.is_empty() {
        return Ok(());
    }
    let mut lines = String::new();
    for (name, oid) in listed {
        lines.push_str(&format!("delete {name} {oid}\n"));
    }
    transaction(repo, lines).map(|_| ())
}

/// One `update-ref --stdin`: every line applies, or none does.
fn transaction(repo: &RepoHandle, lines: String) -> Result<String, MigrateError> {
    let invocation = GitInvocation::new(repo)
        .args([GitArg::Flag("update-ref"), GitArg::Flag("--stdin")])
        .stdin(lines.into_bytes());
    invocation.run(&()).map_err(|error| refused(&error))?;
    Ok(invocation.command_line())
}
