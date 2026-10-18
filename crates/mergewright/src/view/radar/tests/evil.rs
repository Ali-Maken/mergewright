//! The evil-merge probe's lines and marker, through each context and
//! loading state.

use super::*;

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
        Loading::Pending(mergewright_core::job::JobId::new(1)),
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

    state.detail.evil = Loading::Pending(mergewright_core::job::JobId::new(1));
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
    let stash = mergewright_core::ref_name::RefName::new("refs/stash").expect("valid");
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
