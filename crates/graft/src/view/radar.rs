//! Turning `AppState`'s plain facts into `graft-ops`'s `Finding`s, and those
//! into what the graph and the detail pane draw: the marker, and the detail
//! pane's radar lines.
//!
//! Pure, like every other `view` module: everything here is read off
//! `AppState`, never off a repository handle. The one fact nothing else on
//! `AppState` already carries — the default branch's first-parent chain and
//! its previous reflog tips — is `state.radar.context`, read by
//! `jobs::radar::run_radar_context` and landed by `update::radar`.

use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;
use graft_ops::mistake_radar::probes::{
    EvilVerdict, ForcePushVerdict, finding_from_evil, findings_from_force_push,
};
use graft_ops::mistake_radar::rules::{classify_commit, classify_refs};
use graft_ops::mistake_radar::{
    BranchFacts, CommitFacts, Evidence, Finding, HeadFacts, RepoFacts, Risk, UpstreamFacts,
};
use graft_term::sanitize::sanitize_line;
use graft_ui::detail::RadarLine;
use graft_ui::graph_screen::MarkerView;

use crate::jobs::radar::RadarContext;
use crate::refs::Upstream;
use crate::repository::Head;
use crate::update::AppState;
use crate::update::detail::Loading;

/// Every local branch `classify_refs` needs to know about, from the refs
/// job's badges — `None` refs (not read yet) is no branches, not an error:
/// there is nothing to find findings about before the first refs answer.
#[must_use]
pub fn branch_facts(state: &AppState) -> Vec<BranchFacts<'_>> {
    let Some(refs) = state.refs.as_deref() else {
        return Vec::new();
    };
    let default = state
        .history
        .as_ref()
        .and_then(|history| history.walk.as_ref())
        .and_then(|walk| walk.default_branch.as_ref());
    refs.branches()
        .filter_map(|(tip, name, _current, upstream)| {
            let name = std::str::from_utf8(name).ok()?;
            Some(BranchFacts {
                name,
                tip,
                is_default: default.is_some_and(|default| default.shorthand() == name.as_bytes()),
                upstream: upstream_facts(upstream),
            })
        })
        .collect()
}

/// `refs::Upstream`, renamed to the shape `graft-ops` classifies on: no new
/// read, the counts are already on hand.
fn upstream_facts(upstream: &Upstream) -> UpstreamFacts {
    match upstream {
        Upstream::None => UpstreamFacts::None,
        Upstream::InSync { .. } => UpstreamFacts::InSync,
        Upstream::Apart(counts) => UpstreamFacts::Apart {
            ahead: u32::try_from(counts.ahead).unwrap_or(u32::MAX),
            behind: u32::try_from(counts.behind).unwrap_or(u32::MAX),
        },
    }
}

/// Where `HEAD` points, from the facts the open-repository job already read
/// (`crate::repository::Head`) — an unreadable `HEAD` reads as unborn: there
/// is nothing for the radar to say about a `HEAD` this state has already put
/// on the status line as a problem.
#[must_use]
pub fn head_facts(state: &AppState) -> HeadFacts<'_> {
    match state.repository.as_ref().map(|facts| &facts.head) {
        Some(Head::Branch(name)) => match std::str::from_utf8(name) {
            Ok(name) => HeadFacts::Branch(name),
            Err(_) => HeadFacts::Unborn,
        },
        Some(Head::Detached(id)) => HeadFacts::Detached(*id),
        Some(Head::Unborn(_) | Head::Unreadable(_)) | None => HeadFacts::Unborn,
    }
}

/// The repository-wide facts `classify_commit` needs, from `ctx` and the
/// default branch's name already on `state`.
///
/// `remotes` is always empty: nothing in `graft-ops::mistake_radar`'s rules
/// reads it today (reserved for a probe that will).
#[must_use]
pub fn repo_facts<'a>(state: &'a AppState, ctx: &'a RadarContext) -> RepoFacts<'a> {
    let default_branch = state
        .history
        .as_ref()
        .and_then(|history| history.walk.as_ref())
        .and_then(|walk| walk.default_branch.as_ref())
        .and_then(|name| std::str::from_utf8(name.shorthand()).ok());
    RepoFacts {
        default_branch,
        remotes: &[],
        previous_default_tips: &ctx.previous_default_tips,
    }
}

/// Every finding at `id`: `classify_commit`'s answer for this commit, plus
/// any `classify_refs` finding that lands on it. Empty until
/// `state.radar.context` is `Ready` — no marker before the context that
/// makes a foxtrot or a backward merge tellable from an ordinary one has
/// landed.
///
/// **Extension point**: `radar-probe-evil-merge`, `-force-push` and
/// `-duplicates` each add one more source here.
#[must_use]
pub fn findings_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Vec<Finding> {
    let Loading::Ready(context) = &state.radar.context else {
        return Vec::new();
    };
    let repo = repo_facts(state, context);
    let parents_on_default_chain: Vec<bool> = parents
        .iter()
        .map(|parent| context.default_chain.contains(parent))
        .collect();
    let commit = CommitFacts {
        id,
        parents,
        subject,
        on_default_chain: context.default_chain.contains(&id),
        parents_on_default_chain: &parents_on_default_chain,
    };
    let mut findings: Vec<Finding> = classify_commit(&commit, &repo, &state.radar.policy)
        .into_iter()
        .collect();
    let branches = branch_facts(state);
    let head = head_facts(state);
    findings.extend(
        classify_refs(&branches, &head, &state.radar.policy)
            .into_iter()
            .filter(|finding| finding.at == id),
    );
    // The evil-merge probe (`m2-details-and-diffs/14`) answers for the open
    // commit only, so its verdict counts only when `id` is that commit.
    if let Loading::Ready(verdict @ EvilVerdict::Evil { .. }) = &state.detail.evil
        && state.detail.evil_at == Some(id)
    {
        findings.extend(finding_from_evil(id, verdict, &state.radar.policy));
    }
    // A force-push verdict belongs to a branch, and shows at the tip that
    // branch stands on. `finding.at` is the verdict's own `current`, so a
    // branch that moved since its probe answered contributes nothing here
    // rather than marking the wrong row.
    for branch in &branches {
        if branch.tip != id {
            continue;
        }
        if let Some(Loading::Ready(verdict)) = state.radar.force_push.get(branch.name) {
            findings.extend(
                findings_from_force_push(verdict, &state.radar.policy)
                    .filter(|finding| finding.at == id),
            );
        }
    }
    findings
}

/// The marker a row draws: the highest-severity finding at `id`, `Warning`
/// winning any tie.
#[must_use]
pub fn marker_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Option<MarkerView> {
    findings_for(state, id, parents, subject)
        .into_iter()
        .map(|finding| finding.severity)
        .max()
        .map(|severity| MarkerView { severity })
}

/// The detail pane's radar lines for this commit: one per finding, plus an
/// `EvilUnchecked` line when this is a merge, the evil-merge kind is on, and
/// the context is ready — the seam `radar-probe-evil-merge` looks at its own
/// `Loading<EvilVerdict>` first and only falls back to this line when that
/// is `Idle`.
// Called by `view::detail::commit_header` (`m2-details-and-diffs/13c`).
#[must_use]
pub fn radar_lines_for(
    state: &AppState,
    id: Oid,
    parents: &[Oid],
    subject: &str,
) -> Vec<RadarLine> {
    let mut lines: Vec<RadarLine> = findings_for(state, id, parents, subject)
        .into_iter()
        .map(|finding| RadarLine::Finding {
            severity: finding.severity,
            title: sanitize_line(&title_for(&finding)),
        })
        .collect();
    let context_ready = matches!(state.radar.context, Loading::Ready(_));
    if parents.len() >= 2
        && state
            .radar
            .policy
            .severity(MistakeRadarKind::EvilMerge)
            .is_some()
        && context_ready
    {
        let verdict = if state.detail.evil_at == Some(id) {
            Some(&state.detail.evil)
        } else {
            None
        };
        match verdict {
            Some(Loading::Pending(_)) => lines.push(RadarLine::EvilChecking),
            Some(Loading::Ready(EvilVerdict::Clean)) => lines.push(RadarLine::EvilClean),
            Some(Loading::Ready(EvilVerdict::NotCheckable(reason))) => {
                lines.push(RadarLine::EvilNotCheckable(sanitize_line(reason)));
            }
            // Already a finding line, from `findings_for`.
            Some(Loading::Ready(EvilVerdict::Evil { .. })) => {}
            Some(Loading::Idle | Loading::Failed(_)) | None => {
                lines.push(RadarLine::EvilUnchecked);
            }
        }
    }
    // A `NotCheckable` force-push verdict never becomes a finding, so the
    // only way the pane can say the check did not run is this line.
    if state
        .radar
        .policy
        .severity(MistakeRadarKind::ForcePushedRemote)
        .is_some()
        && context_ready
    {
        for branch in branch_facts(state) {
            if branch.tip != id {
                continue;
            }
            if let Some(Loading::Ready(ForcePushVerdict::NotCheckable(reason))) =
                state.radar.force_push.get(branch.name)
            {
                lines.push(RadarLine::ForcePushNotCheckable(sanitize_line(reason)));
                break;
            }
        }
    }
    lines
}

/// The short pane-line phrase for a finding's title (not the Explain panel's
/// caps title, which `explain-panel` builds separately).
fn title_for(finding: &Finding) -> String {
    match finding.kind {
        MistakeRadarKind::FoxtrotMerge => "foxtrot merge".to_owned(),
        MistakeRadarKind::BackwardMerge => "backward merge".to_owned(),
        MistakeRadarKind::AccidentalPullMerge => "accidental pull merge".to_owned(),
        MistakeRadarKind::EvilMerge => "evil merge".to_owned(),
        MistakeRadarKind::DuplicateCommit => "duplicate commit".to_owned(),
        MistakeRadarKind::ForcePushedRemote => "force-pushed remote".to_owned(),
        MistakeRadarKind::RiskyState => match &finding.evidence {
            Evidence::Risk(Risk::DetachedHead) => "detached head".to_owned(),
            Evidence::Risk(Risk::UnpushedOnDefault { branch, ahead }) => {
                format!("unpushed on default ({branch}, {ahead} ahead)")
            }
            Evidence::Risk(Risk::NoUpstream { branch }) => format!("no upstream ({branch})"),
            // `RiskyState` only ever carries `Evidence::Risk`
            // (`graft_ops::mistake_radar::rules::classify_refs`).
            _ => "risky state".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use graft_core::mistake_radar::Severity;
    use graft_ops::mistake_radar::Policy;
    use std::collections::HashMap;
    use std::collections::HashSet;

    use super::*;
    use crate::history::{History, WalkFacts};
    use crate::repository::{CommitGraphFact, Head, RepositoryFacts};
    use crate::update::radar::RadarState;
    use graft_git::history::Order;
    use graft_git::repo::HeadState;

    fn oid(n: u8) -> Oid {
        Oid::from_bytes(&[n; 20]).expect("20 bytes is a sha1")
    }

    fn ref_name(name: &str) -> graft_core::ref_name::RefName {
        graft_core::ref_name::RefName::new(format!("refs/heads/{name}").into_bytes())
            .expect("a valid ref name")
    }

    /// A state with a default branch `main`, `HEAD` on it, and a radar
    /// context whose default chain is exactly `chain`.
    fn ready_state(chain: &[Oid], previous_default_tips: Vec<Oid>) -> AppState {
        let mut state = AppState::default();
        let mut history = History::started();
        history.walk = Some(WalkFacts {
            requested: Order::Topological,
            order: Order::Topological,
            default_branch: Some(ref_name("main")),
            integration: std::sync::Arc::from([]),
        });
        state.history = Some(history);
        let head = HeadState::Branch(ref_name("main"));
        let refs = crate::refs::badges(
            &[(ref_name("main"), chain.first().copied().unwrap_or(oid(1)))],
            &head,
            &HashMap::new(),
            |_, _| unreachable!("no upstream configured, nothing to count"),
        )
        .expect("plain data with no upstream to count always builds");
        state.refs = Some(std::sync::Arc::new(refs));
        state.repository = Some(RepositoryFacts {
            path: std::path::PathBuf::from("/repo"),
            bare: false,
            head: Head::Branch(b"main".to_vec()),
            mailmap: graft_git::mailmap::Mailmap::default(),
            commit_graph: CommitGraphFact::Absent { declined: false },
        });
        state.radar = RadarState {
            context: Loading::Ready(std::sync::Arc::new(RadarContext {
                default_chain: chain.iter().copied().collect::<HashSet<_>>(),
                previous_default_tips,
            })),
            policy: Policy::default(),
            force_push: std::collections::HashMap::new(),
        };
        state
    }

    /// The pane's evil line follows the probe: unchecked before it starts,
    /// checking while it runs, and the verdict once it answers.
    #[test]
    fn the_evil_line_follows_the_probe_through_each_loading_state() {
        let merge = oid(7);
        let parents = [oid(1), oid(2)];
        let mut state = ready_state(&[oid(1)], Vec::new());
        state.detail.evil_at = Some(merge);

        let evil_line = |state: &AppState| {
            radar_lines_for(state, merge, &parents, "merge")
                .into_iter()
                .find(|line| !matches!(line, RadarLine::Finding { .. }))
        };

        assert!(matches!(evil_line(&state), Some(RadarLine::EvilUnchecked)));

        state.detail.evil = Loading::Pending(graft_core::job::JobId::new(1));
        assert!(matches!(evil_line(&state), Some(RadarLine::EvilChecking)));

        state.detail.evil = Loading::Ready(EvilVerdict::Clean);
        assert!(matches!(evil_line(&state), Some(RadarLine::EvilClean)));

        state.detail.evil = Loading::Ready(EvilVerdict::NotCheckable("octopus merge".into()));
        assert!(matches!(
            evil_line(&state),
            Some(RadarLine::EvilNotCheckable(reason)) if reason.as_ref() == "octopus merge"
        ));
    }

    /// An `Evil` verdict is one finding and no second line: the finding line
    /// already says it.
    #[test]
    fn an_evil_verdict_is_one_finding_and_no_separate_evil_line() {
        let merge = oid(7);
        let parents = [oid(1), oid(2)];
        let mut state = ready_state(&[oid(1)], Vec::new());
        state.detail.evil_at = Some(merge);
        state.detail.evil = Loading::Ready(EvilVerdict::Evil {
            files: vec![b"evil.txt".to_vec()],
        });

        let findings = findings_for(&state, merge, &parents, "merge");
        assert_eq!(
            findings
                .iter()
                .filter(|finding| finding.kind == MistakeRadarKind::EvilMerge)
                .count(),
            1
        );
        let lines = radar_lines_for(&state, merge, &parents, "merge");
        assert!(
            !lines.iter().any(|line| matches!(
                line,
                RadarLine::EvilUnchecked | RadarLine::EvilClean | RadarLine::EvilChecking
            )),
            "the finding line stands alone, got {lines:?}"
        );
        assert!(lines.iter().any(|line| matches!(
            line,
            RadarLine::Finding { title, .. } if title.as_ref() == "evil merge"
        )));
    }

    /// The verdict belongs to the commit the pane is open on, and to no
    /// other row.
    #[test]
    fn another_rows_evil_line_ignores_the_open_commits_verdict() {
        let mut state = ready_state(&[oid(1)], Vec::new());
        state.detail.evil_at = Some(oid(7));
        state.detail.evil = Loading::Ready(EvilVerdict::Clean);

        let lines = radar_lines_for(&state, oid(8), &[oid(1), oid(2)], "another merge");
        assert!(matches!(lines.last(), Some(RadarLine::EvilUnchecked)));
        assert!(
            findings_for(&state, oid(8), &[oid(1), oid(2)], "another merge")
                .iter()
                .all(|finding| finding.kind != MistakeRadarKind::EvilMerge)
        );
    }

    #[test]
    fn a_foxtrot_merge_on_the_default_chain_finds_one_warning() {
        let merge = oid(9);
        let real_tip = oid(2);
        let state = ready_state(&[merge, real_tip], Vec::new());
        let parents = [oid(1), real_tip];
        let findings = findings_for(&state, merge, &parents, "Sync up");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].kind, MistakeRadarKind::FoxtrotMerge);
        assert_eq!(findings[0].severity, Severity::Warning);

        let marker = marker_for(&state, merge, &parents, "Sync up").expect("a marker");
        assert_eq!(marker.severity, Severity::Warning);

        let lines = radar_lines_for(&state, merge, &parents, "Sync up");
        assert!(matches!(
            &lines[0],
            RadarLine::Finding { severity: Severity::Warning, title } if title.as_str() == "foxtrot merge"
        ));
        assert!(matches!(lines.last(), Some(RadarLine::EvilUnchecked)));
    }

    #[test]
    fn with_evil_merge_off_no_unchecked_line_is_added() {
        let merge = oid(9);
        let real_tip = oid(2);
        let mut state = ready_state(&[merge, real_tip], Vec::new());
        state.radar.policy.set(MistakeRadarKind::EvilMerge, None);
        let parents = [oid(1), real_tip];
        let lines = radar_lines_for(&state, merge, &parents, "Sync up");
        assert!(
            !lines
                .iter()
                .any(|line| matches!(line, RadarLine::EvilUnchecked))
        );
    }

    #[test]
    fn a_non_merge_off_the_chain_finds_nothing() {
        let state = ready_state(&[oid(1)], Vec::new());
        assert!(findings_for(&state, oid(5), &[oid(1)], "Add a feature").is_empty());
        assert!(marker_for(&state, oid(5), &[oid(1)], "Add a feature").is_none());
    }

    #[test]
    fn before_the_context_lands_nothing_is_found() {
        let mut state = ready_state(&[oid(1), oid(2)], Vec::new());
        state.radar.context = Loading::Idle;
        let parents = [oid(3), oid(2)];
        assert!(findings_for(&state, oid(9), &parents, "Sync up").is_empty());
    }
}
