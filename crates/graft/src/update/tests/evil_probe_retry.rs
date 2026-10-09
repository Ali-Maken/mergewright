//! `update` tests: evil probe retry. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// The merge selected and probed, then the probe's job crashed through
/// `update` (m7-power-tools/33c m2): the state after, and the merge.
fn evil_probe_crashed() -> (AppState, Oid) {
    let (state, merge) = merge_on_screen(JobId::new(41), 1);
    let (state, commands) = update(&state, key(KeyCode::Char(' ')));
    let Some(Command::EvilProbe(id, _)) = commands
        .iter()
        .find(|c| matches!(c, Command::EvilProbe(_, oid) if *oid == merge))
    else {
        panic!("the merge is probed: {commands:?}");
    };
    let failure = JobFailure::Panicked {
        message: "tree walk broke".to_owned(),
        report: None,
    };
    let (state, _) = update(&state, core(Event::JobFailed(*id, failure)));
    (state, merge)
}

fn evil_probes(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|c| matches!(c, Command::EvilProbe(..)))
        .count()
}

/// m7-power-tools/33c m2: a crashed evil probe is `Failed`, and the pane
/// says "not checkable" with the failure, not "checking".
#[test]
fn a_crashed_evil_probe_reads_as_not_checkable() {
    let (state, merge) = evil_probe_crashed();
    assert_eq!(state.detail.evil_at, Some(merge));
    assert!(
        matches!(&state.detail.evil, EvilLoading::Failed(why) if why.contains("tree walk broke")),
        "{:?}",
        state.detail.evil
    );
    let parents = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .and_then(|window| window.rows.iter().find(|row| row.commit.id == merge))
        .map(|row| row.commit.parents.clone())
        .expect("the merge is on screen");
    let lines = crate::view::radar::radar_lines_for(&state, merge, &parents, "a commit");
    assert!(
        lines.iter().any(
            |line| matches!(line, graft_ui::detail::RadarLine::EvilNotCheckable(why)
                if why.as_ref().contains("tree walk broke"))
        ),
        "{lines:?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| matches!(line, graft_ui::detail::RadarLine::EvilChecking)),
        "{lines:?}"
    );
}

/// m7-power-tools/33c m2: a failed probe is an answer. Five events that
/// leave the selection on the merge (an unrelated job finishing) ask no
/// probe; moving away and back asks one each way (the plain row is not a
/// merge, so "away" is the evil state moving to it, and "back" re-probes).
#[test]
fn a_failed_evil_probe_is_not_asked_again_for_the_same_commit() {
    let (mut state, merge) = evil_probe_crashed();
    let mut commands = Vec::new();
    for n in 0..5 {
        let (next, more) = update(&state, core(Event::JobFinished(JobId::new(900 + n))));
        commands.extend(more);
        state = next;
    }
    assert_eq!(evil_probes(&commands), 0, "{commands:?}");
    assert!(matches!(state.detail.evil, EvilLoading::Failed(_)));

    let (away, commands) = update(&state, key(KeyCode::Char('k')));
    assert_eq!(evil_probes(&commands), 0, "the plain row is no merge");
    assert_ne!(away.detail.evil_at, Some(merge));
    let (back, commands) = update(&away, key(KeyCode::Char('j')));
    assert_eq!(evil_probes(&commands), 1, "{commands:?}");
    assert_eq!(back.detail.evil_at, Some(merge));
}

/// m7-power-tools/20o (review m1): `Ctrl+R` is the user's "try again". After
/// a crashed evil probe it asks the probe once more; further events that
/// leave the selection on the merge still ask nothing (no loop).
#[test]
fn ctrl_r_retries_a_crashed_evil_probe_once() {
    let (state, merge) = evil_probe_crashed();
    let (mut state, commands) = update(
        &state,
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
    );
    let asked: Vec<_> = commands
        .iter()
        .filter(|c| matches!(c, Command::EvilProbe(_, oid) if *oid == merge))
        .collect();
    assert_eq!(asked.len(), 1, "{commands:?}");
    assert_eq!(state.detail.evil_at, Some(merge));

    let mut more = Vec::new();
    for n in 0..5 {
        let (next, cmds) = update(&state, core(Event::JobFinished(JobId::new(950 + n))));
        more.extend(cmds);
        state = next;
    }
    assert_eq!(evil_probes(&more), 0, "{more:?}");
}

// m7-power-tools/48: the cursor stays on a row the graph draws.
