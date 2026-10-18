//! Every ref name a plan would write, checked before any step runs.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// Every ref name the plan writes, checked before anything is written (gap
/// :497): git's `check-ref-format` rules, through the one checker Mergewright has
/// ([`RefName::new`]), and a full name under `refs/`. The second rule is the
/// restore path's own: every name here is a full ref a step moves, and one
/// that is not — `-d`, say — would reach git's command line as an option,
/// not a ref.
pub(super) fn check_ref_names(plan: &InversePlan) -> Result<(), RestoreError> {
    for step in &plan.steps {
        let name = match step {
            Step::UpdateRef { name, .. }
            | Step::SetHead(HeadRef::Branch(name) | HeadRef::Unborn(name)) => name,
            Step::SetHead(HeadRef::Detached(_))
            | Step::RestoreIndex { .. }
            | Step::RestoreWorktree { .. }
            | Step::StashStore { .. }
            | Step::StashDrop { .. }
            | Step::SetMergeState(_) => continue,
            // The key's branch, by the same rules: a key reaches a step only
            // through `undo::config_steps`, which refuses one that is not
            // `branch.<name>.<var>`, so it cannot read as an option either.
            Step::SetBranchConfig { key, .. } => match crate::branch_config::branch_of(key) {
                Some(branch) => {
                    check_ref_name(&branch)?;
                    continue;
                }
                None => key,
            },
        };
        check_ref_name(name)?;
    }
    Ok(())
}

pub(super) fn check_ref_name(name: &str) -> Result<(), RestoreError> {
    match RefName::new(name) {
        Err(mergewright_core::error::Error::RefName { name, reason }) => {
            Err(RestoreError::BadRefName {
                name,
                reason: reason.to_string(),
            })
        }
        Err(other) => Err(RestoreError::BadRefName {
            name: name.escape_debug().to_string(),
            reason: other.to_string(),
        }),
        Ok(_) if !name.starts_with("refs/") => Err(RestoreError::BadRefName {
            name: name.escape_debug().to_string(),
            reason: "it is not a full name under refs/".to_owned(),
        }),
        Ok(_) => Ok(()),
    }
}
