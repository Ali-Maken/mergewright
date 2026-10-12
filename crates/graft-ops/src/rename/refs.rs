//! The app's own refs under one prefix: listed; before the switch mirrored
//! under the other in one transaction and read back; after it, carried or
//! set aside by the stray pass.

use std::collections::BTreeMap;

use graft_core::oid::Oid;
use graft_core::on_disk::Generation;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation};

use super::{KEPT_ASIDE_DIR, MigrateError, map_ref};

/// Full ref name to value.
pub(super) type Listed = BTreeMap<String, Oid>;

fn refused(error: &impl std::fmt::Display) -> MigrateError {
    MigrateError::Refs {
        words: error.to_string(),
    }
}

/// Every ref under `generation`'s prefix — one `for-each-ref`.
pub(super) fn list(repo: &RepoHandle, generation: Generation) -> Result<Listed, MigrateError> {
    list_under(repo, &[generation.ref_prefix()])
}

/// Every ref under each of `froms`' prefixes, then every ref under
/// `CURRENT`'s — one `for-each-ref` naming them all, the stray pass's first
/// spawn.
pub(super) fn list_both(
    repo: &RepoHandle,
    froms: &[Generation],
) -> Result<(Listed, Listed), MigrateError> {
    let mut prefixes: Vec<&str> = froms.iter().map(|from| from.ref_prefix()).collect();
    prefixes.push(Generation::CURRENT.ref_prefix());
    let all = list_under(repo, &prefixes)?;
    let (new, old) = all
        .into_iter()
        .partition(|(name, _)| name.starts_with(Generation::CURRENT.ref_prefix()));
    Ok((old, new))
}

/// Every ref under `scratch`'s prefixes deleted at the value listed, in
/// one transaction, then listed again: none may be left.
pub(super) fn delete_all(repo: &RepoHandle, scratch: &[Generation]) -> Result<(), MigrateError> {
    let prefixes: Vec<&str> = scratch.iter().map(|from| from.ref_prefix()).collect();
    let listed = list_under(repo, &prefixes)?;
    if listed.is_empty() {
        return Ok(());
    }
    let mut lines = String::new();
    for (name, oid) in &listed {
        lines.push_str(&format!("delete {name} {oid}\n"));
    }
    transaction(repo, lines)?;
    match list_under(repo, &prefixes)?.into_keys().next() {
        Some(left) => Err(MigrateError::ReadBack { what: left }),
        None => Ok(()),
    }
}

fn list_under(repo: &RepoHandle, prefixes: &[&str]) -> Result<Listed, MigrateError> {
    let mut args = vec![
        GitArg::Flag("for-each-ref"),
        GitArg::Word("--format=%(objectname)%00%(refname)".to_owned()),
    ];
    args.extend(
        prefixes
            .iter()
            .map(|prefix| GitArg::Word((*prefix).to_owned())),
    );
    let output = GitInvocation::new(repo)
        .args(args)
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

/// Before the switch: make the new prefix an exact copy of `old`, in one
/// transaction — none of it if any line cannot apply — then read it back.
/// A new ref already at the old value is left; at another value it is
/// updated (expected at the value read); one with no old counterpart is
/// deleted (review m8-26 M1: until the switch the new refs are the move's
/// own scratch). The command line run, for the journal (empty when nothing
/// changed), and the new refs read back.
pub(super) fn mirror_and_read_back(
    repo: &RepoHandle,
    old: &Listed,
) -> Result<(String, Listed), MigrateError> {
    let new = list(repo, Generation::CURRENT)?;
    let wanted: Listed = old
        .iter()
        .map(|(name, oid)| (map_ref(name, Generation::CURRENT).into_owned(), *oid))
        .collect();
    let mut lines = String::new();
    for (name, oid) in &wanted {
        match new.get(name) {
            Some(held) if held == oid => {}
            Some(held) => lines.push_str(&format!("update {name} {oid} {held}\n")),
            None => lines.push_str(&format!("create {name} {oid}\n")),
        }
    }
    for (name, held) in &new {
        if !wanted.contains_key(name) {
            lines.push_str(&format!("delete {name} {held}\n"));
        }
    }
    if lines.is_empty() {
        return Ok((String::new(), new));
    }
    let command = transaction(repo, lines)?;
    let back = list(repo, Generation::CURRENT)?;
    if let Some(name) = wanted
        .iter()
        .find(|(name, oid)| back.get(*name) != Some(oid))
        .map(|(name, _)| name)
        .or_else(|| back.keys().find(|name| !wanted.contains_key(*name)))
    {
        return Err(MigrateError::ReadBack { what: name.clone() });
    }
    Ok((command, back))
}

/// The stray pass's refs, after the switch, in one transaction read back
/// after: each old ref with no new one is created under the new name; one
/// whose new ref holds the same value is only deleted; one whose new ref
/// holds another is moved to `<CURRENT prefix>older-version/<pass>/<its
/// generation's dir>/<rest>`, so two generations' refs with the same rest
/// never collide. The old ref is deleted at its listed value in every
/// case. `old` is under `froms`' prefixes, `new` under `CURRENT`'s. How
/// many were set aside.
pub(super) fn carry_or_set_aside(
    repo: &RepoHandle,
    old: &Listed,
    new: &Listed,
    pass: usize,
    froms: &[Generation],
) -> Result<usize, MigrateError> {
    if old.is_empty() {
        return Ok(0);
    }
    let mut lines = String::new();
    let mut targets = Listed::new();
    let mut aside = 0;
    for (name, oid) in old {
        let wanted = map_ref(name, Generation::CURRENT).into_owned();
        match new.get(&wanted) {
            Some(held) if held == oid => {}
            Some(_) => {
                let kept = match Generation::of_ref(name) {
                    Some(from) => {
                        let rest = name.strip_prefix(from.ref_prefix()).unwrap_or(name);
                        format!("{}{}/{rest}", kept_aside_prefix(pass), from.dir())
                    }
                    // Listed under one of `froms`' prefixes, so never here.
                    None => format!("{}{name}", kept_aside_prefix(pass)),
                };
                lines.push_str(&format!("create {kept} {oid}\n"));
                targets.insert(kept, *oid);
                aside += 1;
            }
            None => {
                lines.push_str(&format!("create {wanted} {oid}\n"));
                targets.insert(wanted, *oid);
            }
        }
        lines.push_str(&format!("delete {name} {oid}\n"));
    }
    transaction(repo, lines)?;
    let (old_back, new_back) = list_both(repo, froms)?;
    let gone = old
        .keys()
        .find(|name| old_back.get(*name) == old.get(*name));
    let missing = targets
        .iter()
        .find(|(name, oid)| new_back.get(*name) != Some(oid))
        .map(|(name, _)| name);
    if let Some(name) = gone.or(missing) {
        return Err(MigrateError::ReadBack { what: name.clone() });
    }
    Ok(aside)
}

/// `<CURRENT prefix>older-version/<pass>/`.
pub(super) fn kept_aside_prefix(pass: usize) -> String {
    format!(
        "{}{KEPT_ASIDE_DIR}/{pass}/",
        Generation::CURRENT.ref_prefix()
    )
}

/// One `update-ref --stdin`: every line applies, or none does.
fn transaction(repo: &RepoHandle, lines: String) -> Result<String, MigrateError> {
    let invocation = GitInvocation::new(repo)
        .args([GitArg::Flag("update-ref"), GitArg::Flag("--stdin")])
        .stdin(lines.into_bytes());
    invocation.run(&()).map_err(|error| refused(&error))?;
    Ok(invocation.command_line())
}
