//! Merge, rebase, cherry-pick, revert, reset, checkout, branch and tag: the
//! operations that move history (M5 design note, G5).
//!
//! Every one is pinned to the tips the Ghost Preview was drawn against
//! ([`Tip`], hard choice 3): `check` refuses `Moved` when the facts disagree,
//! so what runs is always the picture the user confirmed, never a silent
//! re-plan. The arguments here were each verified against git 2.54 in a
//! throwaway repository before being written down (ticket m5/07 NOTES):
//!
//! - `merge --no-edit -m <msg> <source>` never opens an editor, and the
//!   message is used byte for byte, so the ghost's subject equals git's.
//!   `-F -` is **not** accepted by `merge` ("could not read file '-'").
//! - `rebase <onto>` on a conflict exits 1 leaving `rebase-merge/`; on a
//!   dirty tracked file it refuses before starting ("You have unstaged
//!   changes"), which is why `apply` sets changed tracked files aside
//!   around it (ADR-0045 §4; before m6-conflicts/73, `check` refused them).
//! - `revert --no-edit -m 1 <non-merge>` is **accepted** by git 2.54 (older
//!   gits refused), so `check` refuses `MainlineOnNonMerge` itself; a merge
//!   without `-m` is refused by git ("is a merge but no -m option was given").
//! - `update-ref <name> <to> <expected>` refuses when moved (exit 128, "but
//!   expected"); `update-ref -d <name> <expected>` likewise (exit 1).
//! - `switch <branch>` on a dirty tree whose files differ exits 1 ("would be
//!   overwritten by checkout"); `switch` wants a short name, not
//!   `refs/heads/x` ("a branch is expected").
//! - `stash branch <name> stash@{n}` creates the branch at the stash's own
//!   base commit and applies there, so it cannot conflict; exit 0 and the
//!   stash is dropped.
//! - `tag -a -F - <name> <oid>` reads the message from stdin.
//! - `merge --ff-only` on diverged branches exits 128 with `HEAD` unchanged.

use std::path::PathBuf;

use graft_core::journal::{HeadRef, Touches};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::run::GitArg;
use graft_git::version::GitVersion;

use crate::rewrite::BranchLine;

use super::{Invocation, Refusal, RepoFacts, stash};

/// A ref as the preview saw it: the name and the commit it pointed at.
/// `check` refuses [`Refusal::Moved`] when the repository disagrees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tip {
    /// Full ref name (`refs/heads/main`), or `HEAD` when detached.
    pub name: String,
    /// The commit it pointed at when the preview was drawn.
    pub oid: Oid,
}

/// What a merge, rebase or checkout points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A branch by full name.
    Branch(String),
    /// A bare commit.
    Commit(Oid),
}

/// Whether a merge may or must fast-forward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FastForward {
    /// git's default: fast-forward when possible, else a merge commit.
    Default,
    /// `--ff-only`: refuse rather than make a merge commit.
    Only,
    /// `--no-ff`: always a merge commit.
    Never,
}

/// `git merge`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeParams {
    /// What is merged in.
    pub source: Source,
    /// The branch merged into: `HEAD`'s branch as the preview saw it.
    pub into: Tip,
    /// The source's commit as the preview saw it.
    pub source_tip: Oid,
    /// Fast-forward policy.
    pub ff: FastForward,
}

/// `git rebase` (`r`), run by the rewrite engine (ADR-0047 §6): his branch
/// and every branch built on it, replayed onto `onto`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebaseParams {
    /// The branch replayed: `HEAD`'s branch as the preview saw it.
    pub branch: Tip,
    /// What it is replayed onto.
    pub onto: Source,
    /// `onto`'s commit as the preview saw it.
    pub onto_tip: Oid,
    /// His branch first, then each branch built on it, bottom up: every
    /// branch `y` moves. Filled once, by [`crate::preview::plan_as_run`];
    /// [`RebaseParams::new`] gives his branch alone.
    pub line: BranchLine,
    /// His commits the todo marks `drop`: a squash-merged bottom branch
    /// whose change `onto` already has ([`crate::preview::suggest_drops`]).
    /// Filled with `line`; empty from [`RebaseParams::new`].
    pub drops: Vec<Oid>,
}

impl RebaseParams {
    /// `branch` replayed onto `onto`, its line his branch alone and nothing
    /// dropped: what every key builds. The preview job completes it
    /// ([`crate::preview::plan_as_run`]) before `y` can run it.
    #[must_use]
    pub fn new(branch: Tip, onto: Source, onto_tip: Oid) -> Self {
        Self {
            line: BranchLine {
                branches: vec![branch.clone()],
            },
            branch,
            onto,
            onto_tip,
            drops: Vec::new(),
        }
    }
}

/// `git cherry-pick`. One commit in M5; the `Vec` is M7's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickParams {
    /// The commits to replay, in order.
    pub commits: Vec<Oid>,
    /// The first commit's subject, for the description.
    pub subject: String,
    /// `HEAD` as the preview saw it.
    pub head: Tip,
}

impl PickParams {
    /// Whether `commit` can be taken onto `head`, given two facts only a
    /// read of history holds (m7-power-tools/11a, `KNOWN_GAPS` :806):
    /// `on_head` — `commit` is `HEAD` or one of its ancestors
    /// (`merge-base --is-ancestor`) — and its parent count.
    ///
    /// `check` reads no object, so it cannot ask these; whoever holds the
    /// answers asks this — the preview and `apply` from git, the app from
    /// its graph — and every one refuses in the same words. Already on the
    /// branch wins over a merge: that is the truer reason, and nothing is
    /// taken either way.
    ///
    /// # Errors
    /// [`Refusal::AlreadyOnBranch`], [`Refusal::PickMerge`].
    pub fn check_commit(&self, commit: Oid, on_head: bool, parents: usize) -> Result<(), Refusal> {
        if on_head {
            return Err(Refusal::AlreadyOnBranch {
                commit,
                branch: short_ref(&self.head.name).to_owned(),
            });
        }
        if parents >= 2 {
            return Err(Refusal::PickMerge { commit });
        }
        Ok(())
    }
}

/// `git revert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevertParams {
    /// The commit whose change is reversed.
    pub commit: Oid,
    /// Its subject, for the description.
    pub subject: String,
    /// `-m <n>`: which parent of a merge is the mainline.
    pub mainline: Option<u8>,
    /// How many parents the commit has: `check` needs it to know whether
    /// `mainline` applies, and `check` reads no object.
    pub parents: usize,
    /// The parent whose change is being undone — `c^` for an ordinary
    /// commit, `c^<mainline>` for a merge — as the caller read it from the
    /// commit it is reverting. The Ghost Preview forecasts the reverse
    /// patch against it (`merge-tree` with base `c` and theirs this); `None`
    /// costs the forecast, nothing else. It is a parameter rather than a
    /// read because the caller already has the row's parents, and `plan`
    /// must not depend on a commit read this crate has no reader for
    /// (m5/07).
    pub parent: Option<Oid>,
    /// `HEAD` as the preview saw it.
    pub head: Tip,
}

/// `git reset`, or a branch moved without touching `HEAD`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResetParams {
    /// What is moved.
    pub target: ResetTarget,
    /// Where to.
    pub to: Oid,
}

/// Which ref a reset moves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResetTarget {
    /// `HEAD`'s branch (or `HEAD` itself when detached): `git reset --<mode>`.
    Head {
        /// `HEAD`'s branch as the preview saw it.
        branch: Tip,
        /// Soft, mixed or hard.
        mode: ResetMode,
    },
    /// Another branch: `update-ref` with compare-and-swap. The index and
    /// working tree are untouched.
    Branch(Tip),
}

/// `git reset`'s mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetMode {
    /// Move the ref; index and working tree stay.
    Soft,
    /// Move the ref and the index; the working tree stays.
    Mixed,
    /// Move the ref, the index and the working tree. Uncommitted changes
    /// are lost — the preview says `LOSES`, the backup keeps them (§6.3).
    Hard,
}

/// What a switch does with changed tracked files (the switch rule,
/// m7-power-tools/10b; ADR-0045 §4).
///
/// The app launches a switch it did not preview with `Refuse`, so a tree
/// that turned dirty since its last status read is refused
/// ([`Refusal::DirtyTree`]) rather than set aside unseen; it answers that
/// refusal with the preview, whose `y` runs `SetAside`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnEdits {
    /// Changed tracked files refuse the switch.
    Refuse,
    /// Changed tracked files are set aside and put back (ADR-0045 §4).
    SetAside,
}

/// `git switch`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckoutParams {
    /// Where `HEAD` goes.
    pub to: CheckoutTarget,
    /// `HEAD` as the preview saw it; `check` refuses when it moved.
    pub head_before: HeadRef,
    /// Refuse changed tracked files, or set them aside.
    pub on_edits: OnEdits,
}

/// Where a checkout puts `HEAD`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckoutTarget {
    /// A branch by full name.
    Branch(String),
    /// A commit, detached.
    Detached(Oid),
    /// A remote-only branch: a new local branch `local` (short name) made at
    /// `remote` (full `refs/remotes/…` name and the tip the list saw), set to
    /// track it, and switched to.
    Track {
        /// The remote-tracking ref and the commit the list saw it at.
        remote: Tip,
        /// The new local branch's short name.
        local: String,
    },
}

/// `git branch <name> <at>`, optionally switched to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchCreateParams {
    /// Short name (`feature/x`).
    pub name: String,
    /// The commit it starts at.
    pub at: Oid,
    /// `switch` to it afterwards.
    pub checkout: bool,
    /// With `checkout`: refuse changed tracked files, or set them aside.
    /// At `HEAD`'s own commit nothing is set aside either way: `switch -c`
    /// leaves the index and files as they are.
    pub on_edits: OnEdits,
}

/// A branch or tag deleted by compare-and-swap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefDeleteParams {
    /// Full name.
    pub name: String,
    /// What it must still point at.
    pub expected: Oid,
}

/// `git tag`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagCreateParams {
    /// Short name (`v1.2`).
    pub name: String,
    /// The commit tagged.
    pub at: Oid,
    /// An annotation message; `None` is a lightweight tag.
    pub message: Option<String>,
}

/// `git stash branch`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchFromStashParams {
    /// Short branch name.
    pub name: String,
    /// Position in the stash list.
    pub index: usize,
    /// The stash commit the list showed there.
    pub commit: Oid,
}

// ── describe ────────────────────────────────────────────────────────────

/// The first seven hex digits.
pub(crate) fn short(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

/// `main` for `refs/heads/main`, `v1` for `refs/tags/v1`; `HEAD` stays.
pub(crate) fn short_ref(full: &str) -> &str {
    full.strip_prefix("refs/heads/")
        .or_else(|| full.strip_prefix("refs/tags/"))
        .unwrap_or(full)
}

fn source_text(source: &Source) -> String {
    match source {
        Source::Branch(name) => short_ref(name).to_owned(),
        Source::Commit(oid) => short(*oid),
    }
}

pub(super) fn describe_merge(params: &MergeParams) -> String {
    format!(
        "merge {} into {}",
        source_text(&params.source),
        short_ref(&params.into.name)
    )
}

pub(super) fn describe_rebase(params: &RebaseParams) -> String {
    format!(
        "rebase {} onto {}",
        short_ref(&params.branch.name),
        source_text(&params.onto)
    )
}

pub(super) fn describe_pick(params: &PickParams) -> String {
    match params.commits.as_slice() {
        [one] => format!("cherry-pick {} \"{}\"", short(*one), params.subject),
        many => format!("cherry-pick {} commits", many.len()),
    }
}

pub(super) fn describe_revert(params: &RevertParams) -> String {
    format!("revert {} \"{}\"", short(params.commit), params.subject)
}

pub(super) fn describe_reset(params: &ResetParams) -> String {
    match &params.target {
        ResetTarget::Head { branch, mode } => format!(
            "reset {} to {} ({})",
            short_ref(&branch.name),
            short(params.to),
            mode_name(*mode)
        ),
        ResetTarget::Branch(tip) => {
            format!("move {} to {}", short_ref(&tip.name), short(params.to))
        }
    }
}

fn mode_name(mode: ResetMode) -> &'static str {
    match mode {
        ResetMode::Soft => "soft",
        ResetMode::Mixed => "mixed",
        ResetMode::Hard => "hard",
    }
}

pub(super) fn describe_checkout(params: &CheckoutParams) -> String {
    match &params.to {
        CheckoutTarget::Branch(name) => format!("checkout {}", short_ref(name)),
        CheckoutTarget::Detached(oid) => format!("checkout {} (detached)", short(*oid)),
        CheckoutTarget::Track { remote, local } => {
            format!(
                "checkout {local} (new, tracks {})",
                remote_short(&remote.name)
            )
        }
    }
}

/// `refs/remotes/origin/x` → `origin/x`; anything else as it stands.
pub(crate) fn remote_short(full: &str) -> &str {
    full.strip_prefix("refs/remotes/").unwrap_or(full)
}

pub(super) fn describe_branch_create(params: &BranchCreateParams) -> String {
    format!("branch {} at {}", params.name, short(params.at))
}

pub(super) fn describe_delete(what: &str, params: &RefDeleteParams) -> String {
    format!("delete {what} {}", short_ref(&params.name))
}

pub(super) fn describe_tag_create(params: &TagCreateParams) -> String {
    format!("tag {} at {}", params.name, short(params.at))
}

pub(super) fn describe_branch_from_stash(params: &BranchFromStashParams) -> String {
    format!("branch {} from stash@{{{}}}", params.name, params.index)
}

// ── which refs the facts must carry ─────────────────────────────────────

/// The full name a short branch name lives under.
pub(crate) fn branch_ref(short: &str) -> String {
    format!("refs/heads/{short}")
}

/// The full name a short tag name lives under.
pub(crate) fn tag_ref(short: &str) -> String {
    format!("refs/tags/{short}")
}

fn source_ref(source: &Source) -> Option<String> {
    match source {
        Source::Branch(name) => Some(name.clone()),
        Source::Commit(_) => None,
    }
}

pub(super) fn refs_of_merge(params: &MergeParams) -> Vec<String> {
    let mut refs = vec![params.into.name.clone()];
    refs.extend(source_ref(&params.source));
    refs
}

/// His branch, `onto`'s, then each branch that follows his: the tips
/// `check` holds the rebase to.
pub(super) fn refs_of_rebase(params: &RebaseParams) -> Vec<String> {
    let mut refs = vec![params.branch.name.clone()];
    refs.extend(source_ref(&params.onto));
    refs.extend(followers(params).map(|tip| tip.name.clone()));
    refs
}

/// Every tip a rebase is pinned to: his branch, then each branch of the
/// line that follows it (05b).
pub(super) fn tips_of_rebase(params: &RebaseParams) -> Vec<Tip> {
    let mut tips = vec![params.branch.clone()];
    tips.extend(followers(params).cloned());
    tips
}

/// The branches of the line built on his: every one but his own.
fn followers(params: &RebaseParams) -> impl Iterator<Item = &Tip> {
    params
        .line
        .branches
        .iter()
        .filter(move |tip| tip.name != params.branch.name)
}

pub(super) fn refs_of_reset(params: &ResetParams) -> Vec<String> {
    match &params.target {
        ResetTarget::Head { branch, .. } | ResetTarget::Branch(branch) => vec![branch.name.clone()],
    }
}

pub(super) fn refs_of_checkout(params: &CheckoutParams) -> Vec<String> {
    let mut refs = vec!["HEAD".to_owned()];
    match &params.to {
        CheckoutTarget::Branch(name) => refs.push(name.clone()),
        CheckoutTarget::Detached(_) => {}
        CheckoutTarget::Track { remote, local } => {
            refs.push(remote.name.clone());
            refs.push(branch_ref(local));
        }
    }
    refs
}

pub(super) fn refs_of_branch_create(params: &BranchCreateParams) -> Vec<String> {
    let mut refs = vec![branch_ref(&params.name)];
    if params.checkout {
        refs.push("HEAD".to_owned());
    }
    refs
}

// ── check ───────────────────────────────────────────────────────────────

/// The tip must still be where the preview saw it.
pub(super) fn tip_holds(tip: &Tip, facts: &RepoFacts) -> Result<(), Refusal> {
    let now = if tip.name == "HEAD" {
        match &facts.head {
            HeadRef::Detached(oid) => Some(*oid),
            HeadRef::Branch(_) | HeadRef::Unborn(_) => None,
        }
    } else {
        facts.refs.get(&tip.name).copied().flatten()
    };
    if now == Some(tip.oid) {
        Ok(())
    } else {
        Err(Refusal::Moved {
            name: tip.name.clone(),
        })
    }
}

/// `HEAD` must still be on the tip's branch (or detached, for `HEAD`), and
/// the tip must hold.
pub(super) fn head_is(tip: &Tip, facts: &RepoFacts) -> Result<(), Refusal> {
    let on_it = match &facts.head {
        HeadRef::Branch(name) => *name == tip.name,
        HeadRef::Detached(_) => tip.name == "HEAD",
        HeadRef::Unborn(_) => false,
    };
    if !on_it {
        return Err(Refusal::BranchChanged);
    }
    tip_holds(tip, facts)
}

/// A branch source must still be at the tip the preview saw.
fn source_holds(source: &Source, tip: Oid, facts: &RepoFacts) -> Result<(), Refusal> {
    match source {
        Source::Branch(name) => tip_holds(
            &Tip {
                name: name.clone(),
                oid: tip,
            },
            facts,
        ),
        Source::Commit(_) => Ok(()),
    }
}

/// Staged or unstaged changes to tracked files: what a switch to a new
/// branch, or a branch from a stash, refuses over. Untracked files never
/// count (hard choice 4). A rebase or checkout sets them aside instead
/// ([`Operation::sets_aside`](super::Operation::sets_aside)).
fn dirty_tracked(facts: &RepoFacts) -> usize {
    facts
        .status
        .entries
        .iter()
        .filter(|entry| entry.is_staged() || entry.is_unstaged())
        .count()
}

fn clean_tree(facts: &RepoFacts) -> Result<(), Refusal> {
    match dirty_tracked(facts) {
        0 => Ok(()),
        count => Err(Refusal::DirtyTree { count }),
    }
}

/// A name git would accept under `prefix`, that no ref holds yet.
pub(super) fn new_ref(full: &str, short: &str, facts: &RepoFacts) -> Result<(), Refusal> {
    if let Err(graft_core::error::Error::RefName { reason, .. }) = RefName::new(full.as_bytes()) {
        return Err(Refusal::BadRefName {
            name: short.to_owned(),
            rule: reason,
        });
    }
    if facts.refs.get(full).copied().flatten().is_some() {
        return Err(Refusal::RefExists {
            name: short.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn check_merge(params: &MergeParams, facts: &RepoFacts) -> Result<(), Refusal> {
    head_is(&params.into, facts)?;
    source_holds(&params.source, params.source_tip, facts)
}

/// No clean-tree rule: changed tracked files are set aside around the
/// rebase and put back after it (ADR-0045 §4, `apply::set_aside`).
pub(super) fn check_rebase(params: &RebaseParams, facts: &RepoFacts) -> Result<(), Refusal> {
    head_is(&params.branch, facts)?;
    source_holds(&params.onto, params.onto_tip, facts)?;
    // Every branch that follows his is still where the preview drew it.
    followers(params).try_for_each(|tip| tip_holds(tip, facts))
}

pub(super) fn check_pick(params: &PickParams, facts: &RepoFacts) -> Result<(), Refusal> {
    if params.commits.is_empty() {
        return Err(Refusal::NoCommits);
    }
    head_is(&params.head, facts)
}

pub(super) fn check_revert(params: &RevertParams, facts: &RepoFacts) -> Result<(), Refusal> {
    match params.mainline {
        Some(n) if params.parents < 2 || usize::from(n) == 0 || usize::from(n) > params.parents => {
            return Err(Refusal::MainlineOnNonMerge);
        }
        None if params.parents >= 2 => return Err(Refusal::MergeNeedsMainline),
        _ => {}
    }
    head_is(&params.head, facts)
}

pub(super) fn check_reset(params: &ResetParams, facts: &RepoFacts) -> Result<(), Refusal> {
    match &params.target {
        ResetTarget::Head { branch, .. } => head_is(branch, facts),
        ResetTarget::Branch(tip) => tip_holds(tip, facts),
    }
}

/// `SetAside`: no clean-tree rule, as for a rebase (ADR-0045 §4). `Refuse`
/// (a switch launched without a preview): changed tracked files refuse it,
/// so nothing is ever set aside unseen (m7-power-tools/10b).
pub(super) fn check_checkout(params: &CheckoutParams, facts: &RepoFacts) -> Result<(), Refusal> {
    if facts.head != params.head_before {
        return Err(Refusal::BranchChanged);
    }
    match &params.to {
        CheckoutTarget::Branch(name) if facts.refs.get(name).copied().flatten().is_none() => {
            return Err(Refusal::NoSuchRef { name: name.clone() });
        }
        CheckoutTarget::Branch(_) | CheckoutTarget::Detached(_) => {}
        CheckoutTarget::Track { remote, local } => {
            new_ref(&branch_ref(local), local, facts)?;
            if facts.refs.get(&remote.name).copied().flatten().is_none() {
                return Err(Refusal::NoSuchRef {
                    name: remote.name.clone(),
                });
            }
            tip_holds(remote, facts)?;
        }
    }
    match params.on_edits {
        OnEdits::Refuse => clean_tree(facts),
        OnEdits::SetAside => Ok(()),
    }
}

/// A switching create at `HEAD`'s own commit never refuses changed files:
/// `switch -c` leaves the index and files untouched. Elsewhere, `Refuse`
/// refuses them and `SetAside` sets them aside (m7-power-tools/10b).
pub(super) fn check_branch_create(
    params: &BranchCreateParams,
    facts: &RepoFacts,
) -> Result<(), Refusal> {
    new_ref(&branch_ref(&params.name), &params.name, facts)?;
    if params.checkout && params.on_edits == OnEdits::Refuse && !creates_at_head(params, facts) {
        clean_tree(facts)?;
    }
    Ok(())
}

/// Whether a new branch starts at the commit `HEAD` is on, as the facts
/// read it (`refs_of_branch_create` asks for `HEAD` when it switches).
pub(crate) fn creates_at_head(params: &BranchCreateParams, facts: &RepoFacts) -> bool {
    facts.refs.get("HEAD").copied().flatten() == Some(params.at)
}

pub(super) fn check_branch_delete(
    params: &RefDeleteParams,
    facts: &RepoFacts,
) -> Result<(), Refusal> {
    if matches!(&facts.head, HeadRef::Branch(name) | HeadRef::Unborn(name) if *name == params.name)
    {
        return Err(Refusal::CheckedOut {
            branch: short_ref(&params.name).to_owned(),
        });
    }
    check_delete(params, facts)
}

pub(super) fn check_delete(params: &RefDeleteParams, facts: &RepoFacts) -> Result<(), Refusal> {
    match facts.refs.get(&params.name).copied().flatten() {
        None => Err(Refusal::NoSuchRef {
            name: params.name.clone(),
        }),
        Some(now) if now == params.expected => Ok(()),
        Some(_) => Err(Refusal::Moved {
            name: params.name.clone(),
        }),
    }
}

pub(super) fn check_tag_create(params: &TagCreateParams, facts: &RepoFacts) -> Result<(), Refusal> {
    new_ref(&tag_ref(&params.name), &params.name, facts)?;
    if params
        .message
        .as_deref()
        .is_some_and(|message| super::commit::subject(message).is_empty())
    {
        return Err(Refusal::EmptySubject);
    }
    Ok(())
}

pub(super) fn check_branch_from_stash(
    params: &BranchFromStashParams,
    facts: &RepoFacts,
) -> Result<(), Refusal> {
    new_ref(&branch_ref(&params.name), &params.name, facts)?;
    stash::find(params.index, params.commit, facts)?;
    clean_tree(facts)
}

// ── plan ────────────────────────────────────────────────────────────────

pub(crate) fn source_arg(source: &Source) -> GitArg {
    match source {
        Source::Branch(name) => GitArg::Ref(name.clone().into_bytes()),
        Source::Commit(oid) => GitArg::Oid(*oid),
    }
}

/// The merge commit's subject, in git's own shape so the Mistake Radar
/// reads a Graft merge like any other: `Merge branch 'feat' into main`,
/// `Merge commit '<full id>' into main`.
pub(crate) fn merge_subject(params: &MergeParams) -> String {
    let what = match &params.source {
        Source::Branch(name) => format!("branch '{}'", short_ref(name)),
        Source::Commit(oid) => format!("commit '{oid}'"),
    };
    format!("Merge {what} into {}", short_ref(&params.into.name))
}

pub(super) fn plan_merge(params: &MergeParams) -> Vec<Invocation> {
    let mut args = vec![GitArg::Flag("merge"), GitArg::Flag("--no-edit")];
    match params.ff {
        // `--ff` is spelled out rather than left to git's default, because
        // git's default is `merge.ff` — a config the preview does not read.
        // With `merge.ff=false` set, a merge the box drew as a fast-forward
        // wrote a two-parent merge commit instead, and with `merge.ff=only`
        // one it drew as a ghost merge failed outright (m5/07 review, M1;
        // both verified against git 2.54). An explicit flag on every arm
        // makes the preview's rule git's rule, and keeps `check` pure.
        FastForward::Default => args.push(GitArg::Flag("--ff")),
        FastForward::Only => args.push(GitArg::Flag("--ff-only")),
        FastForward::Never => args.push(GitArg::Flag("--no-ff")),
    }
    args.push(GitArg::Flag("-m"));
    args.push(GitArg::Word(merge_subject(params)));
    args.push(source_arg(&params.source));
    vec![Invocation::new(args)]
}

/// `r` runs through the rewrite engine (ADR-0047 §6): git's `rebase -i` on a
/// todo of `pick`s Graft wrote, which `apply::engine` hands git. With
/// branches following his, git is started on the top of the line, by short
/// name (02b's `<top>` rule); his branch alone is `… <onto>`, as before.
pub(super) fn plan_rebase(params: &RebaseParams, git: GitVersion) -> Vec<Invocation> {
    let top = match params.line.branches.as_slice() {
        [_, .., top] => Some(short_ref(&top.name)),
        _ => None,
    };
    vec![rebase_invocation(source_arg(&params.onto), top, git)]
}

/// Every rebase and rewrite Graft runs: `git rebase -i [--no-update-refs
/// --no-autostash --empty=drop] <onto> [<top>]` (ADR-0047 §1).
///
/// - `-i`: git runs the todo `GIT_SEQUENCE_EDITOR` copies over its own.
/// - `--no-update-refs`: `rebase.updateRefs` rewrites every branch pointing
///   inside the range (verified — `f1` moved under a rebase of `f2`), which
///   no entry records, so `u` would leave them rewritten (m5/07 review, M2).
///   The branches that follow are Graft's own `update-ref` lines instead,
///   each one recorded. The flag only shapes the todo git generates, which
///   Graft's replaces; it is passed so no git version's rule can reach in.
/// - `--no-autostash`: an autostash pops outside any entry; the set-aside
///   is Graft's own, inside it (ADR-0045 §4).
/// - `--empty=drop`: a commit that becomes empty is dropped, as the plain
///   `rebase` before ADR-0047 did. `-i` alone stops on it instead (verified,
///   git 2.54: "The previous cherry-pick is now empty"), a stop with nothing
///   to resolve.
/// - `<top>`: the **short** name of the branch at the top of the line, when
///   branches follow (ADR-0047 §3); a full `refs/heads/…` would detach.
///
/// The three flags are git 2.38's at the newest (`--no-update-refs`), which
/// is [`GitVersion::MINIMUM`]. Below it the plan stays as it was: an older
/// git rejects the flag, and has no `rebase.updateRefs` to honour.
pub(crate) fn rebase_invocation(onto: GitArg, top: Option<&str>, git: GitVersion) -> Invocation {
    let mut args = vec![GitArg::Flag("rebase"), GitArg::Flag("-i")];
    if git >= GitVersion::MINIMUM {
        args.push(GitArg::Flag("--no-update-refs"));
        args.push(GitArg::Flag("--no-autostash"));
        args.push(GitArg::Flag("--empty=drop"));
    }
    args.push(onto);
    if let Some(top) = top {
        args.push(GitArg::Ref(top.as_bytes().to_vec()));
    }
    Invocation::new(args)
}

pub(super) fn plan_pick(params: &PickParams) -> Vec<Invocation> {
    let mut args = vec![GitArg::Flag("cherry-pick"), GitArg::Flag("--no-edit")];
    args.extend(params.commits.iter().map(|oid| GitArg::Oid(*oid)));
    vec![Invocation::new(args)]
}

pub(super) fn plan_revert(params: &RevertParams) -> Vec<Invocation> {
    let mut args = vec![GitArg::Flag("revert"), GitArg::Flag("--no-edit")];
    if let Some(n) = params.mainline {
        args.push(GitArg::Flag("-m"));
        args.push(GitArg::Word(n.to_string()));
    }
    args.push(GitArg::Oid(params.commit));
    vec![Invocation::new(args)]
}

pub(super) fn plan_reset(params: &ResetParams) -> Vec<Invocation> {
    match &params.target {
        ResetTarget::Head { mode, .. } => {
            let flag = match mode {
                ResetMode::Soft => "--soft",
                ResetMode::Mixed => "--mixed",
                ResetMode::Hard => "--hard",
            };
            vec![Invocation::new(vec![
                GitArg::Flag("reset"),
                GitArg::Flag(flag),
                GitArg::Oid(params.to),
            ])]
        }
        ResetTarget::Branch(tip) => vec![update_ref(&tip.name, params.to, tip.oid)],
    }
}

/// `update-ref <name> <to> <expected>`: compare-and-swap.
pub(super) fn update_ref(name: &str, to: Oid, expected: Oid) -> Invocation {
    Invocation::new(vec![
        GitArg::Flag("update-ref"),
        GitArg::Ref(name.as_bytes().to_vec()),
        GitArg::Oid(to),
        GitArg::Oid(expected),
    ])
}

fn switch(short: &str) -> Invocation {
    Invocation::new(vec![
        GitArg::Flag("switch"),
        GitArg::Ref(short.as_bytes().to_vec()),
    ])
}

pub(super) fn plan_checkout(params: &CheckoutParams) -> Vec<Invocation> {
    match &params.to {
        CheckoutTarget::Branch(name) => vec![switch(short_ref(name))],
        CheckoutTarget::Detached(oid) => vec![Invocation::new(vec![
            GitArg::Flag("switch"),
            GitArg::Flag("--detach"),
            GitArg::Oid(*oid),
        ])],
        // `--track` spelled out, so `branch.autoSetupMerge` cannot change
        // whether it tracks; the start point is the remote-tracking ref by
        // full name, which `--track` needs to know what to track. Measured,
        // git 2.54 with `-c branch.autoSetupMerge=false`: `branch.<local>.
        // remote origin`, `.merge refs/heads/<x>` (m7-power-tools/10b). The
        // entry records `branch.<local>.*` before and after (its ref is in
        // `touches_checkout`), so `u` removes those lines again
        // (`crate::branch_config`, review m7-16b..21b Major 1).
        CheckoutTarget::Track { remote, local } => vec![Invocation::new(vec![
            GitArg::Flag("switch"),
            GitArg::Flag("--create"),
            GitArg::Ref(local.clone().into_bytes()),
            GitArg::Flag("--track"),
            GitArg::Ref(remote.name.clone().into_bytes()),
        ])],
    }
}

pub(super) fn plan_branch_create(params: &BranchCreateParams) -> Vec<Invocation> {
    let mut plan = vec![Invocation::new(vec![
        GitArg::Flag("branch"),
        GitArg::Ref(params.name.clone().into_bytes()),
        GitArg::Oid(params.at),
    ])];
    if params.checkout {
        plan.push(switch(&params.name));
    }
    plan
}

pub(super) fn plan_delete(params: &RefDeleteParams) -> Vec<Invocation> {
    vec![Invocation::new(vec![
        GitArg::Flag("update-ref"),
        GitArg::Flag("-d"),
        GitArg::Ref(params.name.clone().into_bytes()),
        GitArg::Oid(params.expected),
    ])]
}

pub(super) fn plan_tag_create(params: &TagCreateParams) -> Vec<Invocation> {
    let mut args = vec![GitArg::Flag("tag")];
    if params.message.is_some() {
        args.push(GitArg::Flag("-a"));
        args.push(GitArg::Flag("-F"));
        args.push(GitArg::Word("-".to_owned()));
    }
    args.push(GitArg::Ref(params.name.clone().into_bytes()));
    args.push(GitArg::Oid(params.at));
    match &params.message {
        Some(message) => vec![Invocation::with_stdin(args, message_bytes(message))],
        None => vec![Invocation::new(args)],
    }
}

/// `\n` line ends and exactly one trailing newline, as `commit` does.
fn message_bytes(message: &str) -> Vec<u8> {
    let mut text = message.replace("\r\n", "\n");
    let end = text.trim_end_matches('\n').len();
    text.truncate(end);
    text.push('\n');
    text.into_bytes()
}

pub(super) fn plan_branch_from_stash(params: &BranchFromStashParams) -> Vec<Invocation> {
    vec![Invocation::new(vec![
        GitArg::Flag("stash"),
        GitArg::Flag("branch"),
        GitArg::Ref(params.name.clone().into_bytes()),
        GitArg::Ref(format!("stash@{{{}}}", params.index).into_bytes()),
    ])]
}

// ── touches ─────────────────────────────────────────────────────────────

pub(super) fn touches_reset(params: &ResetParams) -> Touches {
    match &params.target {
        ResetTarget::Head { branch, mode } => Touches {
            refs: vec![branch.name.clone()],
            index: *mode != ResetMode::Soft,
            worktree: *mode == ResetMode::Hard,
        },
        ResetTarget::Branch(tip) => Touches {
            refs: vec![tip.name.clone()],
            index: false,
            worktree: false,
        },
    }
}

/// `HEAD` is deliberately **not** a touched ref: `HEAD` itself is recorded
/// by the entry's `head_before`/`head_after` and put back by `SetHead`. An
/// `UpdateRef` on `HEAD` while it is symbolic would move the branch it
/// points at instead (verified: `update-ref HEAD <old> <new>` on `main`
/// moves `main`).
///
/// A tracking checkout names the local branch it creates, so `u` deletes it.
pub(super) fn touches_checkout(params: &CheckoutParams) -> Touches {
    let refs = match &params.to {
        CheckoutTarget::Track { local, .. } => vec![branch_ref(local)],
        CheckoutTarget::Branch(_) | CheckoutTarget::Detached(_) => vec![],
    };
    Touches {
        refs,
        index: true,
        worktree: true,
    }
}

pub(super) fn touches_branch_create(params: &BranchCreateParams) -> Touches {
    Touches {
        refs: vec![branch_ref(&params.name)],
        index: params.checkout,
        worktree: params.checkout,
    }
}

pub(super) fn touches_branch_from_stash(params: &BranchFromStashParams) -> Touches {
    Touches {
        refs: vec![branch_ref(&params.name), stash::STASH_REF.to_owned()],
        index: true,
        worktree: true,
    }
}

/// Unmerged paths that still carry a `<<<<<<<` marker: a `Stage` of one is
/// refused as [`Refusal::MarkersRemain`], since staging it would commit the
/// markers as content.
pub(super) fn markers_in(paths: &[PathBuf], facts: &RepoFacts) -> Result<(), Refusal> {
    match paths.iter().find(|path| facts.markers.contains(path)) {
        Some(path) => Err(Refusal::MarkersRemain { path: path.clone() }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
