//! A branch's own config — `branch.<name>.*` — as journal state.
//!
//! A tracking checkout (`switch --create <b> --track <remote>`) writes
//! `branch.<b>.remote` and `.merge`. Undo used to delete the branch and leave
//! those lines, so a later branch of the same name silently tracked the
//! teammate's branch (review m7-16b..21b, Major 1). Every entry now records
//! the `branch.<b>.*` keys of each branch it touches, before and after, and
//! the inverse puts back exactly the keys that differ — config that was
//! there before stays, config the operation wrote goes.
//!
//! Only the repository's own file (`--local`): that is where git writes a
//! branch's upstream, and a restore must never copy a user- or system-level
//! value into it.

use std::collections::BTreeMap;

use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};

/// Key → values, in git's canonical key spelling (`branch.sara/x.remote`).
/// An absent key is not in the map.
pub(crate) type BranchConfig = BTreeMap<String, Vec<String>>;

/// `git config --unset-all`'s exit code for a key that is not set.
const NOT_SET: i32 = 5;

/// Every `branch.<name>.*` key of the local config for each `refs/heads/<name>`
/// in `refs`. No branch among them is no spawn.
pub(crate) fn read(
    repo: &RepoHandle,
    refs: &[String],
    stop: &dyn Stop,
) -> Result<BranchConfig, RunError> {
    let names: Vec<&str> = refs
        .iter()
        .filter_map(|name| name.strip_prefix("refs/heads/"))
        .collect();
    if names.is_empty() {
        return Ok(BranchConfig::new());
    }
    let invocation = GitInvocation::new(repo).args([
        GitArg::Flag("config"),
        GitArg::Flag("--local"),
        GitArg::Flag("-z"),
        GitArg::Flag("--get-regexp"),
        GitArg::Word(r"^branch\.".to_owned()),
    ]);
    match invocation.run(stop) {
        Ok(output) => parse(&output.stdout, &names),
        // `--get-regexp` exits 1 when no key matches: no branch has config.
        Err(RunError::Exited { status: 1, .. }) => Ok(BranchConfig::new()),
        Err(other) => Err(other),
    }
}

/// The branch a key belongs to, as a full ref name: `branch.<name>.<var>` →
/// `refs/heads/<name>`. The variable never has a dot; the name may.
pub(crate) fn branch_of(key: &str) -> Option<String> {
    let (name, _variable) = key.strip_prefix("branch.")?.rsplit_once('.')?;
    (!name.is_empty()).then(|| format!("refs/heads/{name}"))
}

/// The arguments that make `key` hold exactly `to`, in order: one
/// `--replace-all` then an `--add` per further value; `--unset-all` for none.
pub(crate) fn writes(key: &str, to: &[String]) -> Vec<Vec<GitArg>> {
    let config = |flag: &'static str, value: Option<&String>| {
        let mut args = vec![
            GitArg::Flag("config"),
            GitArg::Flag("--local"),
            GitArg::Flag(flag),
            GitArg::Word(key.to_owned()),
        ];
        args.extend(value.map(|value| GitArg::Word(value.clone())));
        args
    };
    let Some((first, rest)) = to.split_first() else {
        return vec![config("--unset-all", None)];
    };
    let mut all = vec![config("--replace-all", Some(first))];
    all.extend(rest.iter().map(|value| config("--add", Some(value))));
    all
}

/// Whether a write's failure is the state it asked for: `--unset-all` of a
/// key that is already unset leaves it unset, which is the whole step (a
/// roll-back repeating an unset lands here).
pub(crate) fn already_so(args: &[GitArg], why: &RunError) -> bool {
    matches!(args.get(2), Some(GitArg::Flag("--unset-all")))
        && matches!(why, RunError::Exited { status, .. } if *status == NOT_SET)
}

/// `-z` records: `<key>\n<value>\0`, or `<key>\0` for a key with no value.
/// A value of a named branch that is not UTF-8 is an error: the journal
/// records text, and a mangled value written back would be a different one.
fn parse(stdout: &[u8], names: &[&str]) -> Result<BranchConfig, RunError> {
    let mut found = BranchConfig::new();
    for record in stdout.split(|byte| *byte == 0).filter(|r| !r.is_empty()) {
        let (key, value) = match record.iter().position(|byte| *byte == b'\n') {
            Some(at) => (&record[..at], Some(&record[at + 1..])),
            None => (record, None),
        };
        let key = String::from_utf8_lossy(key);
        let owned = branch_of(&key).is_some_and(|branch| {
            names
                .iter()
                .any(|name| branch.strip_prefix("refs/heads/") == Some(name))
        });
        if !owned {
            continue;
        }
        let value = match value {
            Some(bytes) => String::from_utf8(bytes.to_vec()).map_err(|_why| {
                RunError::Unparsable(format!("{key} has a value that is not UTF-8"))
            })?,
            // git reads a key with no `=` as boolean true, and a branch's
            // valueless keys are booleans (`rebase`): `true` written back
            // is the same setting.
            None => "true".to_owned(),
        };
        found.entry(key.into_owned()).or_default().push(value);
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keeps_only_the_named_branches_and_every_value_in_order() {
        let out = b"branch.main.remote\norigin\0branch.main.merge\nrefs/heads/a\0\
branch.main.merge\nrefs/heads/b\0branch.sara/x.remote\norigin\0\
branch.a.b.remote\nup\0branch.main.rebase\0";
        let found = parse(out, &["main", "a.b"]).expect("parses");
        assert_eq!(found["branch.main.remote"], ["origin"]);
        assert_eq!(found["branch.main.merge"], ["refs/heads/a", "refs/heads/b"]);
        assert_eq!(found["branch.main.rebase"], ["true"]);
        assert_eq!(found["branch.a.b.remote"], ["up"]);
        assert!(!found.contains_key("branch.sara/x.remote"));
    }

    #[test]
    fn a_key_names_its_branch_even_with_dots_in_the_name() {
        assert_eq!(
            branch_of("branch.a.b.remote").as_deref(),
            Some("refs/heads/a.b")
        );
        assert_eq!(branch_of("remote.origin.url"), None);
        assert_eq!(branch_of("branch..remote"), None);
    }

    #[test]
    fn writes_replace_then_add_or_unset() {
        let lines = |to: &[&str]| -> Vec<String> {
            let to: Vec<String> = to.iter().map(|v| (*v).to_owned()).collect();
            writes("branch.x.merge", &to)
                .iter()
                .map(|args| mergewright_git::run::command_line(args))
                .collect()
        };
        assert_eq!(
            lines(&[]),
            ["git config --local --unset-all branch.x.merge"]
        );
        assert_eq!(
            lines(&["a", "b"]),
            [
                "git config --local --replace-all branch.x.merge a",
                "git config --local --add branch.x.merge b"
            ]
        );
    }
}
