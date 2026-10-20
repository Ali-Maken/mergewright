//! Which tips lie on a line's first-parent chain (ease-pass fix round 15).

use std::collections::HashSet;

use mergewright_core::oid::Oid;

use super::compare::parse_parents;
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// Which of `tips` lie on `line`'s first-parent chain (or are `line`).
///
/// One `rev-list --first-parent --parents <line> ^<tip>…`: the chain from
/// `line` down to the first commit any tip reaches. For **independent**
/// tips (none reaches another; `merge-base --independent`) a tip on the
/// chain is always the first one reached, so it shows up as the first
/// parent of the last commit printed; a tip merged in through a second
/// parent never does. Uncapped: a walk past the runner's output cap is
/// `Unparsable`, never a partial answer. Empty `tips` spawn nothing.
///
/// # Errors
/// [`RunError`] from the runner, `Stopped` included.
pub fn on_first_parent_line(
    repo: &RepoHandle,
    line: Oid,
    tips: &[Oid],
    stop: &dyn Stop,
) -> Result<Vec<Oid>, RunError> {
    if tips.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec![
        GitArg::Flag("rev-list"),
        GitArg::Flag("--first-parent"),
        GitArg::Flag("--parents"),
        GitArg::Oid(line),
    ];
    args.extend(tips.iter().map(|tip| GitArg::Word(format!("^{tip}"))));
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    if output.truncated {
        return Err(RunError::Unparsable(
            "rev-list's first-parent chain outgrew the output cap".to_owned(),
        ));
    }
    let (_, printed) = parse_parents(&output.stdout)?;
    let under: HashSet<Oid> = printed
        .values()
        .filter_map(|up| up.first().copied())
        .collect();
    Ok(tips
        .iter()
        .copied()
        .filter(|tip| *tip == line || under.contains(tip))
        .collect())
}
