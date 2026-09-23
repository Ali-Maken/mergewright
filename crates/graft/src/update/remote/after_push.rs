//! `u` on work a push already sent (ADR-0038 §4): the box that explains
//! it, the re-push that puts the remote back, and `undo`'s "still has it"
//! line.

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_ops::operation::{Lease, Operation, PushParams};
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use super::push_box::{head_ahead, push_box};
use super::{facts, short_branch, short_tracking};
use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::detail::Loading;
use crate::update::state::AppState;

/// What `u` would take back that a push has already sent, and the re-push
/// that would put the remote back (ADR-0038 §4).
pub(in crate::update) struct UndoAfterPush {
    /// The push that sent it — what `undo_after_push_asked` remembers.
    pub push: graft_core::journal::EntryId,
    /// The entry `u` would undo, in its own words, capped so the box's
    /// first line stays one line.
    pub description: String,
    /// `origin/main`, read from the push entry itself so the box names the
    /// side that push actually moved.
    pub remote_side: String,
    /// The re-push: the old tip, under a lease against the tip the push
    /// itself left on the remote, read from its journal entry (gap row
    /// 557). **Always leased** — `PushParams` has no bare
    /// force to reach for, and `graft_ops` refuses a source commit without
    /// one (`Refusal::RepushNeedsLease`).
    ///
    /// `Err` when the re-push cannot be built, carrying the words the box
    /// shows instead: the remote read failed or has not answered yet, the
    /// branch is not the one `HEAD` is on, there is no upstream, or nothing
    /// was fetched to lease against. **The box still opens** — the journal
    /// alone proves the push happened, and a failed remote read standing in
    /// for "not pushed" is what turned `u` into a silent plain undo
    /// (review M5).
    pub repush: Result<PushParams, String>,
}

/// The longest the box quotes an entry's own description before eliding it.
/// `"commit \"…\" is on origin/main"` has to fit one wrapped line, and the
/// box keeps at most four (`graft_ui::confirm::MAX_MESSAGE_LINES`).
const QUOTED_DESCRIPTION: usize = 34;

/// The branch an entry ran on, full (`refs/heads/main`): the journal's own
/// record of where `HEAD` was when it was written. `None` for a detached
/// head, which `p`/`P` refuse to push from, so a push entry always has one.
fn entry_branch(entry: &JournalEntry) -> Option<&str> {
    match &entry.head_before {
        graft_core::journal::HeadRef::Branch(name) | graft_core::journal::HeadRef::Unborn(name) => {
            Some(name)
        }
        graft_core::journal::HeadRef::Detached(_) => None,
    }
}

/// The newest in-force push **of `branch`** after `after`, or `None` when
/// nothing sent that branch since.
///
/// The branch test is the whole point (review M4): without it a push of
/// `main` counted as "this commit is on the remote" for a commit on
/// `feature`, and the box then named `origin/feature` — a branch nothing
/// had pushed. Read from the journal alone, so the remote being unreadable
/// cannot turn a push that did happen into one that did not (review M5).
fn latest_push_of<'a>(
    entries: &'a [JournalEntry],
    branch: &str,
    after: &graft_core::journal::EntryId,
) -> Option<&'a JournalEntry> {
    entries
        .iter()
        .filter(|entry| {
            entry.kind == OperationKind::Push
                && entry.status == EntryStatus::Committed
                && entry.id > *after
                && entry_branch(entry) == Some(branch)
        })
        .max_by(|a, b| a.id.cmp(&b.id))
}

/// Whether `u` should explain itself first: a push since the entry `u`
/// would take back sent **that entry's own branch** to the remote.
///
/// `None` — and so an ordinary, unasked undo — only when the journal says
/// there is nothing to explain: no journal read yet, an entry with no
/// branch, an entry that never moved that branch (staging a file is not
/// something a push sends), or no in-force push of the branch since.
/// Everything the *re-push* needs is checked separately, in [`repush_for`],
/// and its absence disables that one choice rather than the box: the
/// journal is what knows a push happened, and the remote read is not.
pub(in crate::update) fn undo_after_push(state: &AppState) -> Option<UndoAfterPush> {
    let Loading::Ready(read) = &state.journal.entries else {
        return None;
    };
    let target = graft_ops::journal::next_undo(&read.entries)?;
    let branch = entry_branch(target)?;
    if !target.refs_before.contains_key(branch) {
        return None;
    }
    // The same reading of "already pushed" as `pushed_since`, so the box
    // and the line the undo leaves afterwards cannot disagree.
    let push = latest_push_of(&read.entries, branch, &target.id)?;
    if state.remote.undo_after_push_asked.as_ref() == Some(&push.id) {
        return None;
    }
    Some(UndoAfterPush {
        push: push.id.clone(),
        description: elided(&target.description, QUOTED_DESCRIPTION),
        remote_side: pushed_to(push),
        repush: repush_for(state, target, push, branch),
    })
}

/// What `[ Re-push old tip ]` would send, or the words for why the box
/// cannot offer it.
///
/// Every `Err` here is a fact the user is better off reading than not: a
/// remote that could not be read, a read still in flight (which is not a
/// failure and is not worded as one), a branch other than the one `HEAD`
/// is on — whose upstream is not the upstream we hold — an upstream that
/// does not exist, a push entry that did not record the tip it left, or a
/// branch that did not exist before the entry, where "the old tip" would be
/// a branch deletion.
///
/// The lease is the push's own answer ([`repush_lease`]), never the
/// tracking tip `state.remotes` last read (gap row 557): the two differ
/// whenever the remote re-read has not landed yet, or has landed *after*
/// someone else pushed — and in the second case a lease from the read
/// would match the remote and throw their commit away.
fn repush_for(
    state: &AppState,
    target: &JournalEntry,
    push: &JournalEntry,
    branch: &str,
) -> Result<PushParams, String> {
    let short = short_branch(branch);
    let facts = facts(state).map_err(|why| format!("no re-push \u{2014} {why}"))?;
    if facts.head_branch.as_deref() != Some(branch) {
        return Err(format!("no re-push \u{2014} you are not on {short}"));
    }
    let upstream = facts
        .head_upstream
        .as_ref()
        .ok_or_else(|| format!("no re-push \u{2014} {short} has no upstream"))?;
    let expected = repush_lease(push, &upstream.tracking_ref)
        .ok_or_else(|| "re-push unavailable: entry has no remote tip".to_owned())?;
    // `refs_before` is `Option<Oid>` per ref: a branch that did not exist
    // before the entry has no old tip to send, and pushing "nothing" is a
    // branch deletion, not a re-push.
    let old_tip = target
        .refs_before
        .get(branch)
        .copied()
        .flatten()
        .ok_or_else(|| format!("no re-push \u{2014} {short} had no tip before this"))?;
    Ok(PushParams {
        branch: branch.to_owned(),
        remote: upstream.remote.clone(),
        remote_branch: upstream.remote_branch.clone(),
        set_upstream: false,
        lease: Some(Lease { expected }),
        source_commit: Some(old_tip),
    })
}

/// The tip a push left on the remote, from its journal entry:
/// `refs_after[tracking]`, which `graft_ops` reads right after `git push`
/// has moved the tracking ref. `None` when the entry does not carry it (an
/// entry whose after-state could not be read), and the box then offers no
/// re-push rather than guess.
#[must_use]
pub fn repush_lease(push: &JournalEntry, tracking: &str) -> Option<Oid> {
    push.refs_after.get(tracking).copied().flatten()
}

/// `text`, or its first `limit` characters and an ellipsis.
fn elided(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut short: String = text.chars().take(limit).collect();
    short.push('\u{2026}');
    short
}

/// The box `u` opens when a push has already sent the work (design note,
/// "`u` after a push"). **Nothing runs from here**: `[ Undo locally ]`
/// starts the ordinary undo, and `[ Re-push old tip ]` opens the lease box,
/// which is the explicit confirm ADR-0038 §4 requires before the remote is
/// touched.
///
/// Three choices when the re-push can be built, **two** when it cannot —
/// the third line then says why instead of warning about a rewrite that is
/// not on offer.
///
/// The command row is deliberately blank (the user's decision, 2026-09-21):
/// the default answer here is `Cancel`, and a force-with-lease sitting
/// beside it reads as "Enter runs this". The command belongs to the second
/// box — the lease confirm `repush_chosen` opens, which is where it is
/// what is about to run.
#[must_use]
pub(in crate::update) fn undo_after_push_box(after: &UndoAfterPush) -> PendingConfirm {
    let (third_line, repush) = match &after.repush {
        Ok(params) => (
            format!(
                "Re-push old tip rewrites {}, which others may have already pulled",
                after.remote_side
            ),
            Some(Box::new(params.clone())),
        ),
        Err(why) => (why.clone(), None),
    };
    let mut choices = vec![sanitize_line("Cancel"), sanitize_line("Undo locally")];
    if repush.is_some() {
        choices.push(sanitize_line("Re-push old tip"));
    }
    PendingConfirm {
        view: ConfirmView::new(
            sanitize_line("Undo after a push"),
            vec![
                sanitize_line(&format!(
                    "{} is on {}",
                    after.description, after.remote_side
                )),
                sanitize_line("undo works on your local branch only"),
                sanitize_line(&third_line),
            ],
            sanitize_line(""),
        )
        .with_choices(choices),
        then: ConfirmThen::UndoAfterPush {
            push: after.push.clone(),
            repush,
        },
    }
}

/// `[ Re-push old tip ]`: the second, explicit confirm — the same lease box
/// `P` opens after a rejection, worded for a re-push. Still nothing pushed.
pub(in crate::update) fn repush_chosen(state: &mut AppState, params: PushParams) {
    let local_tip = state.refs.as_deref().and_then(crate::refs::RefBadges::head);
    let view = push_box(&params, head_ahead(state), local_tip);
    state.confirm = Some(PendingConfirm {
        view,
        then: ConfirmThen::Operate(Operation::Push(params)),
    });
}

/// `undo`'s line after it took back an entry that an in-force push newer
/// than it already sent: ` · origin/main still has it`. Empty otherwise.
/// Read from the journal the browser holds — `ops` skipped the push, so the
/// user is owed the reason the remote still differs.
///
/// The same [`latest_push_of`] the box asks, so the line and the box cannot
/// disagree: a push of some **other** branch leaves no line here, because
/// it left this branch where it was.
pub(in crate::update) fn pushed_since(state: &AppState, undone: &JournalEntry) -> String {
    let Some(target) = &undone.reverses else {
        return String::new();
    };
    let Loading::Ready(read) = &state.journal.entries else {
        return String::new();
    };
    // The undo ran on the branch it put back, so `HEAD` before the undo
    // names the branch whose pushes matter. The undone entry is older, so
    // it is in this same read when it names a different one.
    let branch = read
        .entries
        .iter()
        .find(|entry| entry.id == *target)
        .and_then(entry_branch)
        .or_else(|| entry_branch(undone));
    let Some(branch) = branch else {
        return String::new();
    };
    latest_push_of(&read.entries, branch, target).map_or_else(String::new, |push| {
        format!(" · {} still has it", pushed_to(push))
    })
}

/// `origin/main` for a push entry: its recorded tracking ref, else the
/// remote side of its description (`push main → origin/main (set upstream)`).
pub(crate) fn pushed_to(push: &JournalEntry) -> String {
    if let Some(tracking) = push.touches.refs.first() {
        return short_tracking(tracking).to_owned();
    }
    push.description
        .split_once(" → ")
        .map_or("the remote", |(_, rest)| {
            rest.split(" (").next().unwrap_or(rest)
        })
        .to_owned()
}
