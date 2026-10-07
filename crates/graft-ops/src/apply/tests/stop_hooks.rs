//! Which git an operation stops hook-first (m8-polish-and-ship/13c). What the
//! stop leaves is walked end to end in `tests/it/stop_hooks.rs`.

use std::path::Path;

use graft_git::repo::RepoHandle;
use graft_git::run::GitArg;
use graft_git::state::InProgress;

use crate::apply::engine::rewrite_git;
use crate::apply::resume::resume_invocation;
use crate::apply::run::plan_git;
use crate::operation::Invocation;

fn line(remote: bool) -> Invocation {
    Invocation {
        args: vec![GitArg::Flag("merge")],
        stdin: None,
        remote,
    }
}

#[test]
fn local_write_lines_stop_hooks_first_and_remote_lines_do_not() {
    let repo = RepoHandle::open(&graft_fixtures::borrow(graft_fixtures::Shape::Trivial))
        .expect("the fixture opens");
    let plan = Path::new("/nonexistent/plan");
    let resumed = |kind: InProgress, dir: Option<&Path>| {
        let args = vec![GitArg::Flag(kind.name()), GitArg::Flag("--continue")];
        resume_invocation(&repo, kind, args, dir, None).stops_hooks_first()
    };

    assert!(
        plan_git(&repo, &line(false)).stops_hooks_first(),
        "a local plan line"
    );
    assert!(
        !plan_git(&repo, &line(true)).stops_hooks_first(),
        "a remote line (fetch, push) keeps the group stop"
    );
    for kind in [
        InProgress::Merge,
        InProgress::CherryPick,
        InProgress::Revert,
    ] {
        assert!(resumed(kind, None), "{kind:?}'s resume");
    }
    // A rebase's hooks failing do not stop it (`engine::rewrite_git`): the
    // group stop is what leaves it paused.
    assert!(
        !rewrite_git(&repo, &line(false), plan).stops_hooks_first(),
        "a rewrite's rebase"
    );
    assert!(
        !resumed(InProgress::Rebase, Some(plan)),
        "a rewrite's resume"
    );
    assert!(!resumed(InProgress::Rebase, None), "a rebase's resume");
}
