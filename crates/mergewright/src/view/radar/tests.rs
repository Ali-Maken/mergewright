use mergewright_core::mistake_radar::Severity;
use mergewright_ops::mistake_radar::Policy;
use mergewright_ops::mistake_radar::probes::ForcePushVerdict;
use std::collections::HashMap;

use super::*;
use crate::history::{History, WalkFacts};
use crate::repository::{CommitGraphFact, Head, RepositoryFacts};
use crate::update::radar::RadarState;
use mergewright_git::history::Order;
use mergewright_git::repo::HeadState;

mod evil;

fn oid(n: u8) -> Oid {
    Oid::from_bytes(&[n; 20]).expect("20 bytes is a sha1")
}

fn ref_name(name: &str) -> mergewright_core::ref_name::RefName {
    mergewright_core::ref_name::RefName::new(format!("refs/heads/{name}").into_bytes())
        .expect("a valid ref name")
}

/// A state with a default branch `main`, `HEAD` on it, and a radar
/// context whose default chain is exactly `chain`.
fn ready_state(chain: &[Oid], previous_default_tips: Vec<Oid>) -> AppState {
    let mut state = AppState::default();
    let mut history = History::started(crate::config::MergedBranches::Folded);
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
    state.repository = Some(Box::new(RepositoryFacts {
        path: std::path::PathBuf::from("/repo"),
        bare: false,
        head: Head::Branch(b"main".to_vec()),
        head_commit: None,
        mailmap: mergewright_git::mailmap::Mailmap::default(),
        commit_graph: CommitGraphFact::Absent { declined: false },
    }));
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
    let remote = |name: &str| mergewright_git::remote::Remote {
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
    state.radar.context = Loading::Pending(mergewright_core::job::JobId::new(1));
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
        Loading::Pending(mergewright_core::job::JobId::new(1)),
    );
    // Overwriting an answer by hand is not something the update path
    // does, so the stamp is bumped here as `force_push_probed` would.
    state.radar.force_push_revision += 1;
    crate::update::radar::refresh_ref_findings(&mut state);
    assert!(radar_lines_for(&state, tip, &[], "tip").is_empty());
}
