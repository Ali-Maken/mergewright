//! Every preview refuses exactly what check refuses.
//! Moved out of `history_ops.rs` to keep that file under its size row.

use super::*;

/// One repository per operation `plan` draws, each built against its own
/// ids, and built once: the caller judges each case on its clean tree first,
/// then dirties the same tree and judges it again. `check` and `plan` only
/// read.
fn refusal_cases() -> Vec<(TempRepo, Operation)> {
    let mut cases: Vec<(TempRepo, Operation)> = Vec::new();

    let first = diverged_tagged();
    let kinds = local_kinds(&first).len() + 1;
    let mut first = Some(first);
    for at in 0..kinds {
        let fresh = first.take().unwrap_or_else(diverged_tagged);
        let mut ops = local_kinds(&fresh);
        ops.push(Operation::BranchCreate(BranchCreateParams {
            name: "switched".to_owned(),
            at: id(&fresh, "HEAD"),
            checkout: true,
            on_edits: graft_ops::operation::OnEdits::Refuse,
        }));
        let op = ops.swap_remove(at);
        cases.push((fresh, op));
    }

    let rebasing = diverged();
    rebasing.git(&["switch", "-q", "feature"]);
    let rebase = rebase_op(&rebasing, "refs/heads/main");
    cases.push((rebasing, rebase));

    let stashed = diverged();
    stashed.write("base.txt", b"wip\n");
    run(
        &stashed,
        &Operation::StashPush(StashParams {
            message: None,
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    let from_stash = Operation::BranchFromStash(BranchFromStashParams {
        name: "from-stash".to_owned(),
        index: 0,
        commit: id(&stashed, "refs/stash"),
    });
    cases.push((stashed, from_stash));
    cases
}

/// `plan` is `Err(Refused(r))` exactly when `check` is `Err(r)`, for every
/// kind `plan` draws, on a clean tree and on a dirty one. Push is left out:
/// it needs a remote, and its `check` does not read the tree.
#[test]
fn every_preview_refuses_exactly_what_check_refuses() {
    let cases = refusal_cases();
    for dirty_tree in [false, true] {
        let mut refused = 0;
        for (repo, op) in &cases {
            if dirty_tree {
                dirty(repo);
            }
            let handle = repo.open();
            let facts = facts_for(&handle, op);
            let checked = op.check(&facts);
            let planned = preview::plan(&handle, &git_version(), op, &facts, &Never);
            match (&checked, &planned) {
                (Ok(()), Ok(_)) => {}
                (Err(want), Err(preview::PreviewError::Refused(got))) => {
                    assert_eq!(got, want, "{} (dirty: {dirty_tree})", op.describe());
                    refused += 1;
                }
                _ => panic!(
                    "{} (dirty: {dirty_tree}): check {checked:?}, plan {:?}",
                    op.describe(),
                    planned.as_ref().err()
                ),
            }
        }
        // The table is not vacuous: a dirty tree refuses something.
        assert_eq!(refused > 0, dirty_tree, "refusals: {refused}");
    }
}
