//! The read behind `R` (m7-power-tools/08, note R1): what the rewrite editor
//! opens on.
//!
//! `graft_ops::rewrite::plan` with `Open { from }` gives the rows (autosquash
//! rows already marked and moved) and the line of branches that follow. The
//! editor also needs every row's **whole** message — the squash group's
//! message block is built from them with `default_message`, and a reword's
//! field is prefilled from them — and the base row it draws under his rows.
//! Both are read here, off the update thread, once.

use std::collections::HashMap;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::{RefTarget, RepoHandle};
use graft_git::run::Stop;
use graft_git::version::system_git;
use graft_ops::operation::{FactsScope, Refusal};
use graft_ops::rewrite::{RewriteError, RewriteParams, RewriteRequest};

use crate::repos::RepoStore;

/// What `R` opened on: the editor's facts, or `plan`'s refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RewriteOpened {
    /// The editor's facts.
    Editor(Box<OpenedRewrite>),
    /// `plan` refused, in its own words (`Refusal::explain`).
    Refused(Refusal),
}

/// Everything the editor opens with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedRewrite {
    /// `plan(Open { from })` as returned: autosquash rows marked and moved,
    /// a squash group's message already `default_message`'s.
    pub params: RewriteParams,
    /// Every row's whole message, keyed by its commit: read with
    /// `graft_git::history::read_commit`, the read the detail pane's commit
    /// rows come from (`Commit::message`).
    pub whole: HashMap<Oid, String>,
    /// The row drawn under his, not editable.
    pub base: BaseRow,
}

/// The `─ main` row under his rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseRow {
    /// `params.onto`.
    pub oid: Oid,
    /// Its subject line.
    pub subject: String,
    /// The upstream's short name when `onto` is where his branch left it,
    /// else the default branch's when it is where his branch left that,
    /// else `onto`'s short sha.
    pub label: String,
}

/// The job: resolves the default branch the way the put-in's plan does
/// (`jobs::rewrite::plan`), then [`open_rewrite`].
///
/// # Errors
/// A read failed (the words the status line shows).
pub(super) fn open(repos: &RepoStore, from: Oid, stop: &dyn Stop) -> Result<RewriteOpened, String> {
    let default = repos
        .branches()
        .map_err(|error| error.to_string())?
        .default_branch()
        .cloned();
    open_rewrite(repos, from, default.as_ref(), stop)
}

/// Plans `Open { from }` and reads the messages and the base row.
///
/// `RewriteError::Refused` → [`RewriteOpened::Refused`]; `Facts` → `Err` with
/// its `explain()`; `Stopped` → `Err("stopped")`, as `jobs/rewrite.rs`'s
/// `planned` maps it.
///
/// # Errors
/// A read failed.
pub(crate) fn open_rewrite(
    repos: &RepoStore,
    from: Oid,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteOpened, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let git = system_git().map_err(|error| error.to_string())?.version();
    let facts = graft_ops::facts::read_facts(&handle, git, stop, &FactsScope::default())
        .map_err(|error| error.explain())?;
    let request = RewriteRequest::Open { from };
    let params =
        match graft_ops::rewrite::plan(&handle, &git, &request, &facts, default_branch, stop) {
            Ok(params) => params,
            Err(RewriteError::Refused(refusal)) => return Ok(RewriteOpened::Refused(refusal)),
            Err(RewriteError::Facts(error)) => return Err(error.explain()),
            Err(RewriteError::Stopped) => return Err("stopped".to_owned()),
        };
    let mut whole = HashMap::with_capacity(params.todo.len());
    for row in &params.todo {
        whole.insert(row.commit, message_of(&handle, row.commit)?);
    }
    let base = base_row(&handle, &params, default_branch)?;
    Ok(RewriteOpened::Editor(Box::new(OpenedRewrite {
        params,
        whole,
        base,
    })))
}

/// One commit's whole message, as stored.
fn message_of(handle: &RepoHandle, commit: Oid) -> Result<String, String> {
    let read = graft_git::history::read_commit(handle, commit)
        .map_err(|error| format!("couldn't read {}: {error}", short(commit)))?;
    Ok(String::from_utf8_lossy(&read.message).into_owned())
}

/// The base row: `onto`, its subject, and the name it is known by.
fn base_row(
    handle: &RepoHandle,
    params: &RewriteParams,
    default_branch: Option<&RefName>,
) -> Result<BaseRow, String> {
    let onto = params.onto;
    let read = graft_git::history::read_commit(handle, onto)
        .map_err(|error| format!("couldn't read {}: {error}", short(onto)))?;
    let subject = String::from_utf8_lossy(read.subject()).into_owned();
    let label = base_label(handle, params, default_branch)?.unwrap_or_else(|| short(onto));
    Ok(BaseRow {
        oid: onto,
        subject,
        label,
    })
}

/// The upstream's short name when `onto` is the merge-base with it, else the
/// default branch's when `onto` is the merge-base with that; `None` when
/// neither (`R` pressed deeper than the base).
fn base_label(
    handle: &RepoHandle,
    params: &RewriteParams,
    default_branch: Option<&RefName>,
) -> Result<Option<String>, String> {
    let head = params.head.oid;
    let upstream = graft_git::remote::upstream_of(handle, &params.head.name)
        .map_err(|error| error.to_string())?;
    if let Some(upstream) = upstream
        && let Some(tip) = upstream.tracking_tip
        && merge_base(handle, head, tip)? == Some(params.onto)
    {
        return Ok(Some(short_name(&upstream.tracking_ref)));
    }
    let Some(name) = default_branch else {
        return Ok(None);
    };
    let tip = match handle.reference(name).map_err(|error| error.to_string())? {
        Some(RefTarget::Object(tip)) => tip,
        // A symbolic default branch or none at all: no base to name by it.
        Some(RefTarget::Symbolic(_)) | None => return Ok(None),
    };
    if merge_base(handle, head, tip)? == Some(params.onto) {
        return Ok(Some(short_name(&String::from_utf8_lossy(name.as_bytes()))));
    }
    Ok(None)
}

fn merge_base(handle: &RepoHandle, a: Oid, b: Oid) -> Result<Option<Oid>, String> {
    graft_git::history_reads::merge_base(handle, a, b).map_err(|error| error.to_string())
}

/// `refs/heads/main` → `main`; `refs/remotes/origin/main` → `origin/main`.
fn short_name(full: &str) -> String {
    full.strip_prefix("refs/heads/")
        .or_else(|| full.strip_prefix("refs/remotes/"))
        .unwrap_or(full)
        .to_owned()
}

fn short(oid: Oid) -> String {
    crate::update::place::short(oid)
}

#[cfg(test)]
mod tests;
