//! The branch picker's rows (m7-power-tools/10c): every branch the user can
//! switch to, shaped and ordered from `graft_git::switch_refs`, and the pure
//! ranking a typed query applies.
//!
//! Order with nothing typed: `previous` (`@{-1}`, when it still exists and is
//! not HEAD's branch), then local branches newest tip first, then
//! remote-only branches newest first, then HEAD's own branch last.
//!
//! A remote-tracking ref is left out when a local branch tracks it, when a
//! local branch already has its local name (switching would be refused), or
//! when no configured remote prefixes it (no remote to name the row by).

use std::cmp::Reverse;

use crate::matcher::fuzzy_score;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::remote;
use graft_git::repo::{HeadState, RepoHandle};
use graft_git::run::Stop;
use graft_git::switch_refs::{RefListing, switch_refs};

use crate::facts::FactsError;

/// Local branch, or a remote branch with no local copy yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwitchKind {
    /// Under `refs/heads/`.
    Local,
    /// Under `refs/remotes/<remote>/`, nothing local tracks it or has its name.
    RemoteOnly {
        /// The longest configured remote name prefixing the ref.
        remote: String,
        /// The ref less `refs/remotes/<remote>/`: the branch a switch creates.
        local: String,
    },
}

/// A local branch's upstream, as the right column shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Relation {
    /// Upstream under `refs/remotes/`: `tracks origin/main`.
    Tracks(RefName),
    /// Upstream under `refs/heads/`: `built on feature-a`.
    BuiltOn(RefName),
    /// No upstream (and every remote-only row).
    None,
}

/// One row of the picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchRow {
    /// Full ref name.
    pub name: RefName,
    /// Local or remote-only.
    pub kind: SwitchKind,
    /// The branch's tip.
    pub tip: Oid,
    /// The tip's subject (display only).
    pub subject: String,
    /// The tip's author (display only).
    pub author: String,
    /// The tip's committer date, unix seconds.
    pub when: i64,
    /// The upstream, for the right column.
    pub relation: Relation,
    /// HEAD's branch (`· here`).
    pub current: bool,
}

/// Every row, in drawn order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SwitchList {
    /// `↩ before`; not repeated in `rows`.
    pub previous: Option<SwitchRow>,
    /// Local newest first, remote-only newest first, `here` last.
    pub rows: Vec<SwitchRow>,
}

/// Reads and shapes [`SwitchList`].
///
/// # Errors
/// [`FactsError::Stopped`] when `stop` fires; `History` for a failed ref
/// read, `Head` for an unreadable HEAD, `Remote` for unreadable remotes.
pub fn switch_list(repo: &RepoHandle, stop: &dyn Stop) -> Result<SwitchList, FactsError> {
    let refs = switch_refs(repo, stop).map_err(FactsError::of_history)?;
    let current = match repo.head()? {
        HeadState::Branch(name) => Some(name),
        HeadState::Detached(_) | HeadState::Unborn(_) => None,
    };
    let remotes: Vec<String> = remote::list(repo)?.into_iter().map(|r| r.name).collect();
    Ok(shape(
        refs.refs,
        refs.previous.as_ref(),
        current.as_ref(),
        &remotes,
    ))
}

fn shape(
    refs: Vec<RefListing>,
    previous: Option<&RefName>,
    current: Option<&RefName>,
    remotes: &[String],
) -> SwitchList {
    let (local, remote): (Vec<_>, Vec<_>) = refs
        .into_iter()
        .partition(|r| r.name.as_bytes().starts_with(b"refs/heads/"));
    let upstreams: Vec<&[u8]> = local
        .iter()
        .filter_map(|r| r.upstream.as_ref())
        .map(RefName::as_bytes)
        .collect();
    let local_names: Vec<&[u8]> = local.iter().map(|r| r.name.shorthand()).collect();

    let mut remote_rows: Vec<SwitchRow> = remote
        .iter()
        .filter(|r| r.name.as_bytes().starts_with(b"refs/remotes/"))
        .filter(|r| !upstreams.contains(&r.name.as_bytes()))
        .filter_map(|r| {
            let short = String::from_utf8_lossy(r.name.shorthand()).into_owned();
            let remote_name = remotes
                .iter()
                .filter(|name| short.starts_with(&format!("{name}/")))
                .max_by_key(|name| name.len())?;
            let local = short[remote_name.len() + 1..].to_owned();
            if local.is_empty() || local_names.contains(&local.as_bytes()) {
                return None;
            }
            let kind = SwitchKind::RemoteOnly {
                remote: remote_name.clone(),
                local,
            };
            Some(row(r.clone(), kind, false))
        })
        .collect();

    let mut previous_row = None;
    let mut here = None;
    let mut local_rows = Vec::new();
    for listing in local {
        let is_current = current == Some(&listing.name);
        if is_current {
            here = Some(row(listing, SwitchKind::Local, true));
        } else if previous == Some(&listing.name) {
            previous_row = Some(row(listing, SwitchKind::Local, false));
        } else {
            local_rows.push(row(listing, SwitchKind::Local, false));
        }
    }
    local_rows.sort_by_key(|r| Reverse(r.when));
    remote_rows.sort_by_key(|r| Reverse(r.when));
    let mut rows = local_rows;
    rows.append(&mut remote_rows);
    rows.extend(here);
    SwitchList {
        previous: previous_row,
        rows,
    }
}

fn row(listing: RefListing, kind: SwitchKind, current: bool) -> SwitchRow {
    let relation = match listing.upstream {
        Some(up) if up.as_bytes().starts_with(b"refs/remotes/") => Relation::Tracks(up),
        Some(up) if up.as_bytes().starts_with(b"refs/heads/") => Relation::BuiltOn(up),
        _ => Relation::None,
    };
    SwitchRow {
        name: listing.name,
        kind,
        tip: listing.tip,
        subject: listing.subject,
        author: listing.author,
        when: listing.when,
        relation,
        current,
    }
}

/// Pure: rank `list` for `query`; empty query = the drawn order.
///
/// Case-insensitive over the short name, by `crate::matcher`: a match
/// at the start or right after a non-alphanumeric character (`/`, `-`, …)
/// ranks above any other substring match; a query with spaces matches when
/// each word does; ties go to the newest tip. `previous` ranks like any row once something is typed.
#[must_use]
pub fn filter_switch_rows<'a>(list: &'a SwitchList, query: &str) -> Vec<&'a SwitchRow> {
    let all = list.previous.iter().chain(list.rows.iter());
    if query.is_empty() {
        return all.collect();
    }
    let mut ranked: Vec<(u32, &SwitchRow)> = all
        .filter_map(|r| rank(r, query).map(|tier| (tier, r)))
        .collect();
    ranked.sort_by_key(|(tier, r)| (*tier, Reverse(r.when)));
    ranked.into_iter().map(|(_, r)| r).collect()
}

/// 0 when every word of the query matches at a segment start, more for
/// each word that matches only inside; `None` when one does not match
/// (`crate::matcher::fuzzy_score`, whose tiers this keeps — the offset
/// is not a switch rank: ties go to the newest tip).
fn rank(row: &SwitchRow, query: &str) -> Option<u32> {
    let short = String::from_utf8_lossy(row.name.shorthand());
    fuzzy_score(query, &short).map(|score| score / 1024)
}
