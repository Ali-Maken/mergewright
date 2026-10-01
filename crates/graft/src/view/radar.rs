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
use graft_ops::mistake_radar::probes::{EvilVerdict, finding_from_evil};
use graft_ops::mistake_radar::rules::classify_commit;
use graft_ops::mistake_radar::{CommitFacts, Evidence, Finding, RepoFacts, Risk};
use graft_term::sanitize::sanitize_line;
use graft_ui::detail::RadarLine;
use graft_ui::graph_screen::MarkerView;

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
    // `graft_git::stash::MAX_ENTRIES`; one past that still gets the line.
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
            // (`graft_ops::mistake_radar::rules::classify_refs`).
            _ => "risky state".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use graft_core::mistake_radar::Severity;
    use graft_ops::mistake_radar::Policy;
    use graft_ops::mistake_radar::probes::ForcePushVerdict;
    use std::collections::HashMap;

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
            &[],
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
            context: Loading::Ready(std::sync::Arc::new(RadarContext::of_ids(
                chain.iter().copied(),
                previous_default_tips,
            ))),
            policy: Policy::default(),
            force_push: std::collections::HashMap::new(),
            duplicates: Loading::Idle,
            duplicate_findings: std::collections::HashMap::new(),
            ..Default::default()
        };
        // The ref-derived findings are built by `update`, once per refs
        // answer (M2 review C1); a state assembled by hand builds them here.
        crate::update::radar::refresh_ref_findings(&mut state);
        state
    }

    /// `RepoFacts.remotes` is the list the remote facts read, `origin`
    /// first — not the `&[]` it was before M4 (m4-remotes/05).
    #[test]
    fn remotes_come_from_facts() {
        let mut state = ready_state(&[oid(1)], Vec::new());
        let remote = |name: &str| graft_git::remote::Remote {
            name: name.to_owned(),
            fetch_url: format!("file:///{name}.git"),
            push_url: None,
        };
        assert!(remote_names(&state).is_empty(), "nothing read yet");
        state.remotes = Loading::Ready(std::sync::Arc::new(crate::jobs::remote::RemoteFacts {
            remotes: vec![remote("origin"), remote("upstream")],
            head_branch: Some("refs/heads/main".to_owned()),
            head_upstream: None,
        }));
        let Loading::Ready(context) = &state.radar.context else {
            unreachable!("ready_state builds a ready context");
        };
        let names = remote_names(&state);
        let facts = repo_facts(&state, context, &names);
        assert_eq!(facts.remotes, ["origin", "upstream"]);
    }

    /// A failed context says so, first, with its reason; no other context
    /// state draws the line (a status must not read as an error when
    /// nothing is wrong). The merge's evil line needs the context, so it
    /// is absent while it has failed.
    #[test]
    fn a_failed_context_says_radar_not_checked() {
        let merge = oid(7);
        let parents = [oid(1), oid(2)];
        let mut state = ready_state(&[oid(1)], Vec::new());
        let not_checked = |state: &AppState| {
            radar_lines_for(state, merge, &parents, "merge")
                .into_iter()
                .filter(|line| matches!(line, RadarLine::NotChecked(_)))
                .count()
        };
        assert_eq!(not_checked(&state), 0, "Ready");
        state.radar.context = Loading::Idle;
        assert_eq!(not_checked(&state), 0, "Idle");
        state.radar.context = Loading::Pending(graft_core::job::JobId::new(1));
        assert_eq!(not_checked(&state), 0, "Pending");

        state.radar.context = Loading::Failed("commit b32274b is not in the repository".to_owned());
        // The evil probe needs no context, so its line still shows after the
        // NotChecked line (m7-power-tools/33c m1); unasked, it says unchecked.
        assert_eq!(
            radar_lines_for(&state, merge, &parents, "merge"),
            vec![
                RadarLine::NotChecked(sanitize_line("commit b32274b is not in the repository")),
                RadarLine::EvilUnchecked,
            ]
        );
        assert_eq!(marker_for(&state, merge, &parents, "merge"), None);
    }

    /// The evil probe needs no context: a found evil merge shows while the
    /// context has failed, after the `NotChecked` line (m7-power-tools/33c m1).
    #[test]
    fn an_evil_merge_shows_while_the_context_failed() {
        let merge = oid(7);
        let parents = [oid(1), oid(2)];
        let mut state = ready_state(&[oid(1)], Vec::new());
        state.detail.evil_at = Some(merge);
        state.detail.evil = Loading::Ready(EvilVerdict::Evil {
            files: vec![b"evil.txt".to_vec()],
        });
        state.radar.context = Loading::Failed("commit b32274b is not in the repository".to_owned());

        let findings = findings_for(&state, merge, &parents, "merge");
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.kind)
                .collect::<Vec<_>>(),
            vec![MistakeRadarKind::EvilMerge]
        );
        let lines = radar_lines_for(&state, merge, &parents, "merge");
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(matches!(lines[0], RadarLine::NotChecked(_)), "{lines:?}");
        assert!(
            matches!(&lines[1], RadarLine::Finding { title, .. }
                if title.as_ref() == title_for(&findings[0])),
            "{lines:?}"
        );
    }

    /// On `Idle` and `Pending` contexts the evil line shows too, and no
    /// `NotChecked` line (m7-power-tools/33c m1).
    #[test]
    fn the_evil_line_shows_on_an_idle_or_pending_context() {
        let merge = oid(7);
        let parents = [oid(1), oid(2)];
        let mut state = ready_state(&[oid(1)], Vec::new());
        state.detail.evil_at = Some(merge);
        for context in [
            Loading::Idle,
            Loading::Pending(graft_core::job::JobId::new(1)),
        ] {
            state.radar.context = context;
            state.detail.evil = Loading::Ready(EvilVerdict::Clean);
            assert_eq!(
                radar_lines_for(&state, merge, &parents, "merge"),
                vec![RadarLine::EvilClean]
            );
            state.detail.evil = Loading::Ready(EvilVerdict::Evil {
                files: vec![b"evil.txt".to_vec()],
            });
            let lines = radar_lines_for(&state, merge, &parents, "merge");
            assert!(
                matches!(lines.as_slice(), [RadarLine::Finding { .. }]),
                "{lines:?}"
            );
        }
    }

    /// On a failed context the marker column is empty for a plain commit and
    /// shows the evil marker on an evil merge (m7-power-tools/33c m1).
    #[test]
    fn a_failed_context_marks_only_the_evil_merge() {
        let merge = oid(7);
        let parents = [oid(1), oid(2)];
        let mut state = ready_state(&[oid(1)], Vec::new());
        state.radar.context = Loading::Failed("gone".to_owned());
        assert_eq!(
            marker_for(&state, oid(5), &[oid(1)], "plain"),
            None,
            "plain commit"
        );
        state.detail.evil_at = Some(merge);
        state.detail.evil = Loading::Ready(EvilVerdict::Evil {
            files: vec![b"evil.txt".to_vec()],
        });
        let expected = Policy::default()
            .severity(MistakeRadarKind::EvilMerge)
            .map(|severity| MarkerView { severity });
        assert!(expected.is_some());
        assert_eq!(
            marker_for(&state, merge, &parents, "merge"),
            expected,
            "evil merge"
        );
    }

    /// A crashed probe (`Failed`) reads as not checkable, with its reason
    /// (m7-power-tools/33c m2).
    #[test]
    fn a_failed_evil_probe_reads_as_not_checkable() {
        let merge = oid(7);
        let parents = [oid(1), oid(2)];
        let mut state = ready_state(&[oid(1)], Vec::new());
        state.detail.evil_at = Some(merge);
        state.detail.evil = Loading::Failed("job panicked".to_owned());
        assert_eq!(
            radar_lines_for(&state, merge, &parents, "merge"),
            vec![RadarLine::EvilNotCheckable(sanitize_line("job panicked"))]
        );
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

    /// m7-power-tools/16: a stash commit is a merge whose tree differs from
    /// its parents by construction; the probe never runs on one
    /// (`update::selection`), so "evil merge: unchecked" would be a line
    /// that can never be answered. Every entry, not just the newest.
    #[test]
    fn a_stash_commit_has_no_evil_merge_line() {
        let mut state = ready_state(&[oid(1)], Vec::new());
        let (newest, older) = (oid(8), oid(9));
        let stash = graft_core::ref_name::RefName::new("refs/stash").expect("valid");
        let refs = crate::refs::badges(
            &[(ref_name("main"), oid(1)), (stash, newest)],
            &HeadState::Branch(ref_name("main")),
            &HashMap::new(),
            &[(newest, 0), (older, 1)],
            |_, _| unreachable!("no upstream configured, nothing to count"),
        )
        .expect("plain data with no upstream to count always builds");
        state.refs = Some(std::sync::Arc::new(refs));
        for entry in [newest, older] {
            let lines = radar_lines_for(&state, entry, &[oid(1), oid(2)], "WIP on main");
            assert!(
                lines.iter().all(|line| !matches!(
                    line,
                    RadarLine::EvilUnchecked
                        | RadarLine::EvilChecking
                        | RadarLine::EvilClean
                        | RadarLine::EvilNotCheckable(_)
                        | RadarLine::EvilResolved { .. }
                )),
                "{lines:?}"
            );
        }
        // A merge that is no stash still says it was not checked.
        let lines = radar_lines_for(&state, oid(7), &[oid(1), oid(2)], "merge");
        assert!(matches!(lines.last(), Some(RadarLine::EvilUnchecked)));
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

    /// A `Rewritten` verdict on a branch's tip is one finding at that tip,
    /// and no separate line: the finding line already says it.
    #[test]
    fn a_rewritten_remote_is_a_finding_at_the_branchs_tip() {
        let tip = oid(1);
        let mut state = ready_state(&[tip], Vec::new());
        crate::update::radar::force_push_probed(
            &mut state,
            "main".to_owned(),
            tip,
            ForcePushVerdict::Rewritten {
                remote_ref: "origin/main".to_owned(),
                previous: oid(9),
                current: tip,
            },
        );
        crate::update::radar::refresh_ref_findings(&mut state);

        let findings = findings_for(&state, tip, &[], "tip");
        assert_eq!(findings.len(), 1, "one force-push finding: {findings:?}");
        assert_eq!(findings[0].kind, MistakeRadarKind::ForcePushedRemote);
        assert_eq!(findings[0].at, tip);

        // Another row on the same branch's history gets nothing.
        assert!(findings_for(&state, oid(5), &[], "older").is_empty());

        let lines = radar_lines_for(&state, tip, &[], "tip");
        assert!(
            lines.iter().all(|line| matches!(
                line,
                RadarLine::Finding { title, .. } if title.as_ref() == "force-pushed remote"
            )),
            "no extra line beside the finding: {lines:?}"
        );
    }

    /// A `NotCheckable` verdict is never a finding — absence of evidence is
    /// not a clean bill — but the pane still says the check did not run.
    #[test]
    fn a_not_checkable_verdict_is_a_line_and_never_a_finding() {
        let tip = oid(1);
        let mut state = ready_state(&[tip], Vec::new());
        crate::update::radar::force_push_probed(
            &mut state,
            "main".to_owned(),
            tip,
            ForcePushVerdict::NotCheckable("no remote-tracking reflog".to_owned()),
        );
        crate::update::radar::refresh_ref_findings(&mut state);

        assert!(findings_for(&state, tip, &[], "tip").is_empty());
        let lines = radar_lines_for(&state, tip, &[], "tip");
        assert!(
            matches!(
                lines.as_slice(),
                [RadarLine::ForcePushNotCheckable(reason)]
                    if reason.as_ref() == "no remote-tracking reflog"
            ),
            "one not-checkable line: {lines:?}"
        );

        // With the kind off the pane says nothing at all.
        state
            .radar
            .policy
            .set(MistakeRadarKind::ForcePushedRemote, None);
        assert!(radar_lines_for(&state, tip, &[], "tip").is_empty());

        // A pending probe has nothing to say yet either.
        state.radar.policy = Policy::default();
        state.radar.force_push.insert(
            "main".to_owned(),
            Loading::Pending(graft_core::job::JobId::new(1)),
        );
        // Overwriting an answer by hand is not something the update path
        // does, so the stamp is bumped here as `force_push_probed` would.
        state.radar.force_push_revision += 1;
        crate::update::radar::refresh_ref_findings(&mut state);
        assert!(radar_lines_for(&state, tip, &[], "tip").is_empty());
    }
}
