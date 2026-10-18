//! Turning `AppState`'s plain facts into `mergewright-ops`'s `Finding`s, and those
//! into what the graph and the detail pane draw: the marker, and the detail
//! pane's radar lines.
//!
//! Pure, like every other `view` module: everything here is read off
//! `AppState`, never off a repository handle. The one fact nothing else on
//! `AppState` already carries — the default branch's first-parent chain and
//! its previous reflog tips — is `state.radar.context`, read by
//! `jobs::radar::run_radar_context` and landed by `update::radar`.

use mergewright_core::mistake_radar::MistakeRadarKind;
use mergewright_core::oid::Oid;
use mergewright_ops::mistake_radar::probes::{EvilVerdict, finding_from_evil};
use mergewright_ops::mistake_radar::rules::classify_commit;
use mergewright_ops::mistake_radar::{CommitFacts, Evidence, Finding, RepoFacts, Risk};
use mergewright_term::sanitize::sanitize_line;
use mergewright_ui::detail::RadarLine;
use mergewright_ui::graph_screen::MarkerView;

use crate::jobs::radar::RadarContext;
use crate::update::AppState;
use crate::update::detail::Loading;

// The two fact adapters the rules read `AppState` through — `branch_facts`
// and `head_facts` — moved to `update::radar` in M2's review fix: they are
// O(refs), and the findings they feed are now built there once per refs
// answer rather than once per painted row (C1).

/// The repository-wide facts `classify_commit` needs, from `ctx` and the
/// default branch's name already on `state`.
///
/// `remotes` are the configured remotes' names, from [`remote_names`]
/// (m4-remotes/05); empty until the remote facts are first read.
#[must_use]
pub fn repo_facts<'a>(
    state: &'a AppState,
    ctx: &'a RadarContext,
    remotes: &'a [&'a str],
) -> RepoFacts<'a> {
    let default_branch = state
        .history
        .as_ref()
        .and_then(|history| history.walk.as_ref())
        .and_then(|walk| walk.default_branch.as_ref())
        .and_then(|name| std::str::from_utf8(name.shorthand()).ok());
    RepoFacts {
        default_branch,
        remotes,
        previous_default_tips: &ctx.previous_default_tips,
    }
}

/// The configured remotes' names, `origin` first, as the last remote facts
/// read said; empty before that read lands.
#[must_use]
pub fn remote_names(state: &AppState) -> Vec<&str> {
    match &state.remotes {
        Loading::Ready(facts) => facts
            .remotes
            .iter()
            .map(|remote| remote.name.as_str())
            .collect(),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => Vec::new(),
    }
}

/// Every finding at `id`: `classify_commit`'s answer for this commit, plus
/// any `classify_refs` finding that lands on it. Only the evil-merge
/// verdict ignores the context — its probe reads none, so a found evil
/// merge shows on `Idle`/`Pending`/`Failed` too (m7-power-tools/33c).
/// Everything else waits for `state.radar.context` to be `Ready` — no
/// marker before the context that makes a foxtrot or a backward merge
/// tellable from an ordinary one has landed.
///
/// **Extension point**: `radar-probe-evil-merge`, `-force-push` and
/// `-duplicates` each add one more source here.
#[must_use]
pub fn findings_for(state: &AppState, id: Oid, parents: &[Oid], subject: &str) -> Vec<Finding> {
    // The evil-merge probe (`m2-details-and-diffs/14`) answers for the open
    // commit only, so its verdict counts only when `id` is that commit. It
    // needs no context, so it is read before the context gate below.
    let mut evil = Vec::new();
    if let Loading::Ready(verdict @ EvilVerdict::Evil { .. }) = &state.detail.evil
        && state.detail.evil_at == Some(id)
    {
        evil.extend(finding_from_evil(id, verdict, &state.radar.policy));
    }
    let Loading::Ready(context) = &state.radar.context else {
        return evil;
    };
    let remotes = remote_names(state);
    let repo = repo_facts(state, context, &remotes);
    let parents_on_default_chain: Vec<bool> = parents
        .iter()
        .map(|parent| context.on_default_chain(parent))
        .collect();
    let commit = CommitFacts {
        id,
        parents,
        subject,
        on_default_chain: context.on_default_chain(&id),
        parents_on_default_chain: &parents_on_default_chain,
    };
    let mut findings: Vec<Finding> = classify_commit(&commit, &repo, &state.radar.policy)
        .into_iter()
        .collect();
    // `classify_refs`'s answer is O(refs) to work out and the same for every
    // row, so `update::radar::refresh_ref_findings` works it out once, when
    // the refs move, and a row looks itself up (M2 review C1).
    findings.extend(
        state
            .radar
            .ref_findings
            .get(&id)
            .into_iter()
            .flatten()
            .cloned(),
    );
    findings.extend(evil);
    // A force-push verdict belongs to a branch and shows at the tip that
    // branch stands on; matching a verdict to a tip is another walk over the
    // refs, so it happens in the same rebuild.
    findings.extend(
        state
            .radar
            .force_push_findings
            .get(&id)
            .into_iter()
            .flatten()
            .cloned(),
    );
    // The duplicates probe answers for the whole window at once; its
    // findings are already indexed by commit (`update::radar`).
    if let Some(finding) = state.radar.duplicate_findings.get(&id) {
        findings.push(finding.clone());
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
/// `EvilUnchecked` line when this is a merge and the evil-merge kind is on,
/// whatever the context (the probe reads none, m7-power-tools/33c) — the
/// seam `radar-probe-evil-merge` looks at its own `Loading<EvilVerdict>`
/// first and only falls back to this line when that is `Idle`.
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
    // The context failed to read: every check that needs it did not run.
    // Said first, so "no findings" never reads as "nothing wrong"
    // (m7-power-tools/33). Only on `Failed` — `Idle`/`Pending` are not errors.
    if let Loading::Failed(why) = &state.radar.context {
        lines.insert(0, RadarLine::NotChecked(sanitize_line(why)));
    }
    let context_ready = matches!(state.radar.context, Loading::Ready(_));
    // A stash commit is a merge by construction, and the probe never runs
    // on one (`update::selection`): no evil-merge line, not even
    // "unchecked" (m7-power-tools/16). `is_stash` knows the entries up to
    // `mergewright_git::stash::MAX_ENTRIES`; one past that still gets the line.
    let is_stash = state.refs.as_deref().is_some_and(|refs| refs.is_stash(id));
    if parents.len() >= 2
        && !is_stash
        && state
            .radar
            .policy
            .severity(MistakeRadarKind::EvilMerge)
            .is_some()
    {
        let verdict = if state.detail.evil_at == Some(id) {
            Some(&state.detail.evil)
        } else {
            None
        };
        match verdict {
            Some(Loading::Pending(_)) => lines.push(RadarLine::EvilChecking),
            Some(Loading::Ready(EvilVerdict::Clean)) => lines.push(RadarLine::EvilClean),
            Some(Loading::Ready(EvilVerdict::Resolved { files })) => {
                lines.push(RadarLine::EvilResolved { files: files.len() });
            }
            // A probe that crashed (`update::job_failed`) could not check
            // either: same line, the failure as its reason.
            Some(Loading::Ready(EvilVerdict::NotCheckable(reason)) | Loading::Failed(reason)) => {
                lines.push(RadarLine::EvilNotCheckable(sanitize_line(reason)));
            }
            // Already a finding line, from `findings_for`.
            Some(Loading::Ready(EvilVerdict::Evil { .. })) => {}
            Some(Loading::Idle) | None => {
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
        && let Some(reason) = state.radar.force_push_not_checkable.get(&id)
    {
        lines.push(RadarLine::ForcePushNotCheckable(sanitize_line(reason)));
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
            // `RiskyState` only ever carries `Evidence::Risk`
            // (`mergewright_ops::mistake_radar::rules::classify_refs`).
            _ => "risky state".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests;
