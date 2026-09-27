//! m6-conflicts/80 (review m10): can a failed checkout have moved `HEAD`?
//!
//! Yes. `git switch` runs the `post-checkout` hook after it has moved
//! `HEAD` and the files, and the hook's exit status becomes git's own
//! (githooks(5): "the hook's exit status becomes the exit status of git
//! switch"). So an `ApplyError::Git` for a checkout — whose `touches.refs`
//! is empty — can stand on a new `HEAD`, and the app's re-read after a
//! failure must not read "no refs" as "`HEAD` stayed" (the hole ticket 79
//! closed for undo; `graft::jobs::operate::refresh_after_failure` is the
//! other site).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::os::unix::fs::PermissionsExt;

use graft_core::journal::HeadRef;
use graft_ops::apply::ApplyError;
use graft_ops::operation::{CheckoutParams, CheckoutTarget, Operation};

use crate::support::{NOW, TempRepo, try_run};

#[test]
fn a_checkout_whose_post_checkout_hook_fails_has_moved_head() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    let hooks = repo.path().join("failing-hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks dir is made");
    let hook = hooks.join("post-checkout");
    std::fs::write(&hook, "#!/bin/sh\nexit 1\n").expect("the hook is written");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("the hook is executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);

    let checkout = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    });
    let Err(ApplyError::Git { entry, .. }) = try_run(&repo, &checkout, NOW) else {
        panic!("the failing hook fails the checkout");
    };
    assert!(entry.touches.refs.is_empty(), "a checkout names no ref");
    assert_eq!(
        repo.git(&["symbolic-ref", "HEAD"]),
        "refs/heads/feature",
        "git moved HEAD before the hook failed"
    );
    assert_eq!(
        entry.head_after,
        Some(HeadRef::Branch("refs/heads/feature".to_owned())),
        "and the entry's after-state says so"
    );
}
