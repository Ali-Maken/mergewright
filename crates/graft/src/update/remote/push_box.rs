//! `P`: what it would push, and the box that asks first (m4-remotes/05).

use graft_core::oid::Oid;
use graft_ops::operation::{Lease, Operation, PushParams};
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::confirm::ConfirmView;
use graft_ui::fit::fit;
use unicode_width::UnicodeWidthStr;

use super::{
    LEAVES_SHOWN, LeavesRead, REMOTES_PENDING, RejectedPush, facts, refusal, short_branch,
    short_oid,
};
use crate::jobs::remote::RemoteFacts;
use crate::refs::Upstream;
use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::state::{AppState, RefsRead};

/// `P` while the refs read a write asked for has not answered, when the
/// answer would otherwise be the lease: the refs held are from before the
/// write, so whether `HEAD` is still the refused tip is not known yet.
pub(crate) const REFS_PENDING: &str = "still reading the refs \u{2014} try again in a moment";

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
            // `push_params` offered the lease only with this answer read
            // for the lease's own key.
            let leaves = params.lease.and_then(|_| {
                state
                    .remote
                    .last_rejected_push
                    .as_ref()
                    .and_then(|rejected| rejected.leaves.as_ref())
                    .map(|(_, _, answer)| answer)
            });
            let view = push_box(&params, ahead, local_tip, leaves);
            state.confirm = Some(PendingConfirm {
                view,
                then: ConfirmThen::Operate(Operation::Push(params)),
            });
        }
        Err(why) => state.status = refusal(why),
    }
    true
}

/// What `P` would push, or why it cannot. Pure: the table the tests drive.
///
/// The upstream's remote and branch when there is one; otherwise `origin`
/// (or the only remote) under the same name, setting the upstream. After a
/// non-fast-forward rejection of this same push, the lease variant, guarded
/// by the tracking tip the user last fetched — only while the refs are
/// current and `HEAD` is still the tip that was refused (m6-conflicts/49).
///
/// # Errors
/// The status line's words: no remote, detached `HEAD`, several remotes and
/// none to choose by, nothing to lease against, facts not read yet, or —
/// when the answer would be the lease — the refs or remote facts a write
/// asked for still on their way.
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
    params.lease = lease(state, facts, &params)?;
    Ok(params)
}

/// The lease `params` gets, if any: only after git refused this same push,
/// and only while `HEAD` is still the tip it refused, on refs that answer
/// the last read asked (m6-conflicts/49). Anything that moved `HEAD` since
/// — a rebase onto the teammate's work, a commit, a reset — gets the plain
/// box: git decides, and a push still not a fast-forward is refused again
/// and asks once more.
fn lease(
    state: &AppState,
    facts: &RemoteFacts,
    params: &PushParams,
) -> Result<Option<Lease>, String> {
    let rejected = state.remote.last_rejected_push.as_ref().filter(|last| {
        last.params.branch == params.branch
            && last.params.remote == params.remote
            && last.params.remote_branch == params.remote_branch
    });
    let Some(rejected) = rejected else {
        return Ok(None);
    };
    let Some(key) = lease_key(state, facts, rejected)? else {
        return Ok(None);
    };
    // The box lists what the lease takes off the remote, read for this very
    // key (m7-power-tools/16b). Not read yet, or read for a key a fetch has
    // since moved: say so — never a stale list, never a box without it.
    match &rejected.leaves {
        Some((sent, expected, answer))
            if (*sent, *expected) == key && !matches!(answer, LeavesRead::Reading(_)) => {}
        _ => {
            return Err(format!(
                "still reading what leaves {}/{} \u{2014} try again in a moment",
                params.remote,
                short_branch(&params.remote_branch)
            ));
        }
    }
    Ok(Some(Lease { expected: key.1 }))
}

/// The lease `rejected` would get now, as its key `(sent, expected)`: the
/// tip git refused and the tracking tip the lease names. `Ok(None)` when no
/// lease is offered (refs not vouched for, `HEAD` moved off the refused
/// tip). The one place these rules live, so [`lease`] and the settle tail's
/// read of what leaves ([`super::request_leaves`]) cannot disagree.
///
/// # Errors
/// The words `P` shows while the answer could be the lease and the refs or
/// remote facts to decide it are on their way, or there is nothing to lease
/// against.
pub(super) fn lease_key(
    state: &AppState,
    facts: &RemoteFacts,
    rejected: &RejectedPush,
) -> Result<Option<(Oid, Oid)>, String> {
    match state.refs_read {
        // The answer could be the lease, and the refs to decide it are on
        // their way: say so rather than decide from the refs before a write.
        RefsRead::Pending(_) => return Err(REFS_PENDING.to_owned()),
        // Refs nobody can vouch for: nothing to compare with.
        RefsRead::Unknown => return Ok(None),
        RefsRead::Current(_) => {}
    }
    let head = state.refs.as_deref().and_then(crate::refs::RefBadges::head);
    let Some(sent) = rejected.sent.filter(|sent| head == Some(*sent)) else {
        return Ok(None);
    };
    // The remote facts the lease is built from are being re-read.
    if state.remote.reading.is_some() {
        return Err(REMOTES_PENDING.to_owned());
    }
    let Some(expected) = facts
        .head_upstream
        .as_ref()
        .and_then(|upstream| upstream.tracking_tip)
    else {
        return Err("f to fetch first \u{2014} nothing to lease against".to_owned());
    };
    Ok(Some((sent, expected)))
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
/// lease variant says replaces the remote's tip; `leaves` is what the lease
/// takes off the remote, listed under the history warning
/// (m7-power-tools/16b) — `None` for a box that has no such read (the
/// re-push after `u`).
#[must_use]
pub fn push_box(
    params: &PushParams,
    ahead: Option<usize>,
    local_tip: Option<Oid>,
    leaves: Option<&LeavesRead>,
) -> ConfirmView {
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
            if let Some(leaves) = leaves {
                message.extend(leaves_lines(&remote_side, leaves));
            }
            ConfirmView::new(sanitize_line(&title), message, sanitize_line(&command))
                .with_labels("Cancel", act)
                .with_max_lines(LEASE_BOX_ROWS)
        }
    }
}

/// The lease box's message rows at most: line 1 (up to 3 rows with a long
/// remote name), the history warning, the re-push line, `leaves …:`, three
/// commits and `+N more` (m7-power-tools/16b ruling 1).
const LEASE_BOX_ROWS: usize = 10;

/// Content cells of the narrowest confirm box: `confirm_area` caps it at 64
/// and snaps outward only, less two borders and two padding cells. A commit
/// row fits this, so it never wraps.
const BOX_CONTENT: usize = 60;

/// Widest author column; a longer name is cut with `…`.
const AUTHOR_CELLS: usize = 16;

/// What leaves `remote_side`, in the box's words: `leaves origin/main:` and
/// one aligned row per commit (`sha7  subject  author`, newest first), `+N
/// more` past [`LEAVES_SHOWN`]; or the one line for nothing lost or a read
/// that failed.
fn leaves_lines(remote_side: &str, leaves: &LeavesRead) -> Vec<SanitizedText> {
    let leaves = match leaves {
        // `lease` never offers the box on this answer; drawn honestly if
        // some other route does.
        LeavesRead::Reading(_) => {
            return vec![sanitize_line(&format!(
                "still reading what leaves {remote_side}"
            ))];
        }
        LeavesRead::Failed(why) => {
            return vec![sanitize_line(&format!(
                "couldn't read what leaves {remote_side}: {why}"
            ))];
        }
        LeavesRead::Read(leaves) => leaves,
    };
    if leaves.count == 0 {
        return vec![sanitize_line(&format!("nothing on {remote_side} is lost"))];
    }
    let authors: Vec<String> = leaves
        .newest
        .iter()
        .map(|commit| {
            sanitize_line(&String::from_utf8_lossy(&commit.author.name))
                .as_str()
                .to_owned()
        })
        .collect();
    let author_cells = authors
        .iter()
        .map(|name| UnicodeWidthStr::width(name.as_str()))
        .max()
        .unwrap_or(0)
        .min(AUTHOR_CELLS);
    // The sha, and two gaps of two cells. No indent: the box wraps on
    // spaces and drops leading ones.
    let subject_cells = BOX_CONTENT.saturating_sub(7 + 2 + 2 + author_cells);
    let mut lines = vec![sanitize_line(&format!("leaves {remote_side}:"))];
    for (commit, author) in leaves.newest.iter().take(LEAVES_SHOWN).zip(&authors) {
        let subject = sanitize_line(&commit.subject);
        let row = format!(
            "{}  {}  {}",
            short_oid(commit.id),
            fit(subject.as_str(), subject_cells),
            fit(author, author_cells)
        );
        lines.push(sanitize_line(row.trim_end()));
    }
    if leaves.count > LEAVES_SHOWN {
        lines.push(sanitize_line(&format!(
            "+{} more",
            leaves.count - LEAVES_SHOWN
        )));
    }
    lines
}
