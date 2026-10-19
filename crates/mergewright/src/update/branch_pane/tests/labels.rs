//! `not merged` from the cache (ease-pass 09b), and when a read counts as
//! settled.

use super::*;

/// `not merged` (09b): only a landed `Merged::NotYet`; every other `Merged`,
/// an unread, reading or failed key, and a key after a refs read say none.
#[test]
fn not_merged_only_from_a_landed_not_yet_read() {
    let key = |branch: &str| StatusKey {
        branch: format!("refs/heads/{branch}"),
        tips: 7,
    };
    let with = |merged: Merged| {
        Arc::new(BranchStatus {
            route: Vec::new(),
            pushed: Pushed::NoUpstream,
            merged,
            behind: Behind::NoBase,
            who: Who::NoOwnCommits,
        })
    };
    let mut state = AppState::default();
    let sara = "refs/heads/sara/fix-toc";
    assert!(!state.look.reads.not_merged(sara), "unread");
    state
        .look
        .reads
        .by_key
        .insert(key("sara/fix-toc"), StatusRead::Reading);
    assert!(!state.look.reads.not_merged(sara), "reading");
    read(&mut state, key("sara/fix-toc"), Err("gone".to_owned()));
    assert!(!state.look.reads.not_merged(sara), "failed");
    for merged in [
        Merged::MainLine,
        Merged::Into("refs/heads/main".to_owned()),
        Merged::NothingToMerge,
    ] {
        read(&mut state, key("sara/fix-toc"), Ok(with(merged.clone())));
        assert!(!state.look.reads.not_merged(sara), "{merged:?}");
    }
    read(
        &mut state,
        key("sara/fix-toc"),
        Ok(with(Merged::NotYet { into: None })),
    );
    assert!(state.look.reads.not_merged(sara), "landed NotYet");
    assert!(
        !state.look.reads.not_merged("refs/heads/ali/x"),
        "only the branch read"
    );
    refs_landed(&mut state);
    assert!(!state.look.reads.not_merged(sara), "cleared by a refs read");
}

/// 09b Q2: a read landed under the old refs is not settled while a refs
/// read is in flight — that read clears the cache when it lands, so a
/// capture taken "settled" in between drew `not merged` or not depending on
/// which job answered first (seen under load in three captures).
#[test]
fn a_read_is_not_settled_while_a_refs_read_is_in_flight() {
    let (state, commands) = rest(&on_screen());
    let Some(Command::AtRest(_, RestRead::BranchStatus(ask))) = commands
        .iter()
        .find(|c| matches!(c, Command::AtRest(_, RestRead::BranchStatus(_))))
    else {
        panic!("asked: {commands:?}");
    };
    let (mut state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(1),
            Product::BranchStatusRead {
                key: ask.key.clone(),
                result: Ok(status(Vec::new())),
            },
        ),
    );
    assert!(state.look.reads.not_merged("refs/heads/sara/fix-toc"));
    assert!(settled(&state), "landed, no refs read asked");
    state.refs_read.asked(JobId::new(77));
    assert!(!settled(&state), "a refs read in flight will clear it");
    let (state, _) = update(
        &state,
        AppEvent::Produced(JobId::new(77), Product::Refs(Ok(badges()))),
    );
    assert!(
        !state.look.reads.not_merged("refs/heads/sara/fix-toc"),
        "the landing cleared it: what the capture lost"
    );
    assert!(
        !settled(&state),
        "cleared, re-asked: not settled until it lands"
    );
}
