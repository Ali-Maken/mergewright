//! `P`: what it would push, and the box that asks first (m4-remotes/05).

use graft_core::oid::Oid;
use graft_ops::operation::{Lease, Operation, PushParams};
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use super::{facts, short_branch, short_oid};
use crate::refs::Upstream;
use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::state::{AppState, Status};

/// `P`: open the push box — the lease variant after a rejection. The box's
/// `Enter` dispatches; nothing is pushed from here.
pub fn push_key(state: &mut AppState) -> bool {
    if crate::update::operate::operation_in_flight(state) {
        return true;
    }
    match push_params(state) {
        Ok(params) => {
            let ahead = head_ahead(state);
            let local_tip = state.refs.as_deref().and_then(crate::refs::RefBadges::head);
            let view = push_box(&params, ahead, local_tip);
            state.confirm = Some(PendingConfirm {
                view,
                then: ConfirmThen::Operate(Operation::Push(params)),
            });
        }
        Err(why) => state.status = Status::Failed(why),
    }
    true
}

/// What `P` would push, or why it cannot. Pure: the table the tests drive.
///
/// The upstream's remote and branch when there is one; otherwise `origin`
/// (or the only remote) under the same name, setting the upstream. After a
/// non-fast-forward rejection of this same push, the lease variant, guarded
/// by the tracking tip the user last fetched.
///
/// # Errors
/// The status line's words: no remote, detached `HEAD`, several remotes and
/// none to choose by, nothing to lease against, or facts not read yet.
pub fn push_params(state: &AppState) -> Result<PushParams, String> {
    let facts = facts(state)?;
    if facts.remotes.is_empty() {
        return Err("no remote — add one in the shell: git remote add origin <url>".to_owned());
    }
    let Some(branch) = &facts.head_branch else {
        return Err("HEAD is detached — check out a branch first".to_owned());
    };
    let mut params = if let Some(upstream) = &facts.head_upstream {
        PushParams {
            branch: branch.clone(),
            remote: upstream.remote.clone(),
            remote_branch: upstream.remote_branch.clone(),
            set_upstream: false,
            lease: None,
            // `P` pushes the branch as it stands. Re-pushing an *old* tip
            // after `u` took a push back is a different offer with its own
            // confirm (ADR-0038 §4, the M5 note's "`u` after a push"), and
            // it is not built yet — so this is `None`, never a quiet reuse
            // of whatever commit happens to be selected.
            source_commit: None,
        }
    } else {
        let remote = match facts.remotes.iter().find(|remote| remote.name == "origin") {
            Some(origin) => origin,
            None if facts.remotes.len() == 1 => &facts.remotes[0],
            None => {
                return Err(format!(
                    "several remotes and no upstream — set one: git push -u <remote> {}",
                    short_branch(branch)
                ));
            }
        };
        PushParams {
            branch: branch.clone(),
            remote: remote.name.clone(),
            remote_branch: branch.clone(),
            set_upstream: true,
            lease: None,
            // A first push of a branch with no upstream: the branch tip is
            // the only thing there is to push. See the arm above.
            source_commit: None,
        }
    };
    let rejected = state
        .remote
        .last_rejected_push
        .as_ref()
        .is_some_and(|last| {
            last.branch == params.branch
                && last.remote == params.remote
                && last.remote_branch == params.remote_branch
        });
    if rejected {
        let Some(expected) = facts
            .head_upstream
            .as_ref()
            .and_then(|upstream| upstream.tracking_tip)
        else {
            return Err("f to fetch first — nothing to lease against".to_owned());
        };
        params.lease = Some(Lease { expected });
    }
    Ok(params)
}

/// How many commits `HEAD`'s branch has that its upstream lacks, when the
/// refs say (`Upstream::Apart`); `None` when they do not.
pub(super) fn head_ahead(state: &AppState) -> Option<usize> {
    let refs = state.refs.as_deref()?;
    refs.branches()
        .find(|&(_, _, current, _)| current)
        .and_then(|(_, _, _, upstream)| match upstream {
            Upstream::Apart(counts) => Some(counts.ahead),
            Upstream::None | Upstream::InSync { .. } => None,
        })
}

/// The push box, in the design note's words (New UI, "Push box"). Pure.
///
/// `ahead` is how many commits go, when known; `local_tip` is what the
/// lease variant says replaces the remote's tip.
#[must_use]
pub fn push_box(params: &PushParams, ahead: Option<usize>, local_tip: Option<Oid>) -> ConfirmView {
    let local = short_branch(&params.branch);
    let remote_side = format!("{}/{}", params.remote, short_branch(&params.remote_branch));
    let command = Operation::Push(params.clone())
        .preview()
        .map_or_else(String::new, |lines| lines.join("\n"));
    match params.lease {
        None => {
            let mut consequence = Vec::new();
            if let Some(count) = ahead {
                consequence.push(if count == 1 {
                    "1 commit".to_owned()
                } else {
                    format!("{count} commits")
                });
            }
            if params.set_upstream {
                consequence.push("sets upstream".to_owned());
            }
            consequence.push(format!(
                "can't be undone from here — {} keeps what you send",
                params.remote
            ));
            ConfirmView::new(
                sanitize_line(&format!("Push {local} → {remote_side}")),
                vec![sanitize_line(&consequence.join(" · "))],
                sanitize_line(&command),
            )
            .with_labels("Cancel", "Push")
        }
        Some(Lease { expected }) => {
            let theirs = short_oid(expected);
            // What replaces the remote's tip. For a re-push that is the old
            // commit being sent, not the branch head — the branch head is
            // exactly what a re-push leaves alone (ADR-0038 §4).
            let ours = params
                .source_commit
                .or(local_tip)
                .map_or_else(|| local.to_owned(), short_oid);
            let mut message = vec![
                sanitize_line(&format!(
                    "{remote_side} {theirs} → {ours} · refused if {} moved since your fetch (lease)",
                    params.remote
                )),
                sanitize_line(&format!("rewrites history for everyone who has {theirs}")),
            ];
            // Titled and worded so it cannot be read as a local undo
            // (ADR-0038 §4): what moves is the remote, and only the remote.
            let (title, act) = if params.source_commit.is_some() {
                message.push(sanitize_line("your local branch keeps every commit it has"));
                (
                    format!("Re-push old tip {ours} → {remote_side}?"),
                    "Re-push",
                )
            } else {
                (format!("Force-push {local} → {remote_side}?"), "Force push")
            };
            ConfirmView::new(sanitize_line(&title), message, sanitize_line(&command))
                .with_labels("Cancel", act)
        }
    }
}
