//! `update` tests: job ids. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// Major 5 of the m3/06 review: every key handler copies `local_jobs` out of
/// the state, hands a closure over the copy to a callee, and writes the copy
/// back — so a callee that mints from `state.local_jobs` has its ids thrown
/// away, and the next mint reissues an id still in flight. Two callees were
/// fixed by threading `next_id` into them; the pattern that made it possible
/// is still at fourteen call sites, and the existing regression test only
/// names those two callees.
///
/// This is the guard for the class: over a scripted run, no local job id is
/// ever handed out twice. It reads the ids out of each `Command`'s `Debug`
/// rather than matching thirty variants by hand — deliberately, because a
/// match would have to be extended for every new command and would go quietly
/// stale, which is the same failure this test exists to prevent.
#[test]
fn no_local_job_id_is_ever_minted_twice() {
    use std::collections::HashMap;

    let events = [
        key(KeyCode::Char('J')),
        key(KeyCode::Char('j')),
        key(KeyCode::Char(' ')),
        key(KeyCode::Char('J')),
        key(KeyCode::Char('S')),
        key(KeyCode::Char(' ')),
        key(KeyCode::Char('s')),
        key(KeyCode::Escape),
        key(KeyCode::Char('S')),
        core(Event::Key(KeyPress::control(KeyCode::Char('r')))),
        key(KeyCode::Char('u')),
        key(KeyCode::Char('U')),
        core(Event::FocusGained),
    ];

    let mut state = AppState {
        repository: Some(facts("/code/api", "main")),
        ..AppState::default()
    };
    // Which press first handed out each id, so a clash names both.
    let mut seen: HashMap<u64, String> = HashMap::new();
    for (press, event) in events.into_iter().enumerate() {
        let (next, commands) = update(&state, event);
        for command in &commands {
            // `CancelJob` names a job that already exists; it is the one
            // command that legitimately repeats an id.
            if matches!(command, Command::CancelJob(_)) {
                continue;
            }
            let rendered = format!("{command:?}");
            for id in local_job_ids(&rendered) {
                let what = format!("press {press}: {rendered}");
                if let Some(first) = seen.insert(id, what.clone()) {
                    panic!("job id {id} handed out twice — first {first}, then {what}");
                }
            }
        }
        state = next;
    }
}

/// Every `JobId(n)` in a `Debug` rendering that carries the local-job bit.
fn local_job_ids(rendered: &str) -> Vec<u64> {
    let mut found = Vec::new();
    let mut rest = rendered;
    while let Some(at) = rest.find("JobId(") {
        rest = &rest[at + "JobId(".len()..];
        let end = rest.find(')').unwrap_or(rest.len());
        if let Ok(id) = rest[..end].parse::<u64>()
            && id & crate::update::detail::LOCAL_JOB_BIT != 0
        {
            found.push(id);
        }
        rest = &rest[end..];
    }
    found
}

/// `Product::Remotes` under any id but the read in flight is dropped: a
/// slow answer from before a refresh must not replace a newer one
/// (m4-remotes/05).
#[test]
fn stale_remotes_dropped() {
    let facts = |branch: &str| {
        Arc::new(crate::jobs::remote::RemoteFacts {
            remotes: Vec::new(),
            head_branch: Some(branch.to_owned()),
            head_upstream: None,
        })
    };
    let mut state = AppState::default();
    let (old_id, fresh) = (JobId::new(7), JobId::new(8));
    state.remote.reading = Some(fresh);
    state.remotes = detail::Loading::Pending(fresh);
    let (next, _) = update(
        &state,
        AppEvent::Produced(old_id, Product::Remotes(Ok(facts("refs/heads/old")))),
    );
    assert_eq!(
        next.remotes,
        detail::Loading::Pending(fresh),
        "the stale answer is dropped"
    );
    let (next, _) = update(
        &next,
        AppEvent::Produced(fresh, Product::Remotes(Ok(facts("refs/heads/main")))),
    );
    assert_eq!(
        next.remotes,
        detail::Loading::Ready(facts("refs/heads/main"))
    );
    assert_eq!(next.remote.reading, None);
}
