//! The 50 000-ref fixture (split out of `run`).

use super::*;

/// [`REFS`] refs, as `refs::read` would hand them over: half local branches
/// each tracking a remote-tracking ref, half those remote-tracking refs.
/// Nearly all point outside the first screen, as in a real repository with
/// that many; the rows on hand carry a few each, so badges are drawn and
/// looked up on every frame.
pub(super) fn fifty_thousand(state: &AppState) -> Result<RefBadges, String> {
    let on_screen: Vec<Oid> = state
        .history
        .as_ref()
        .and_then(|h| h.window.as_ref())
        .map(|w| w.rows.iter().map(|row| row.commit.id).collect())
        .unwrap_or_default();
    if on_screen.is_empty() {
        return Err("no rows on hand to put badges on".to_owned());
    }
    let elsewhere = |index: usize| {
        let mut bytes = [0xEEu8; Oid::SHA1_BYTES];
        bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
        Oid::from_bytes(&bytes).map_err(|why| why.to_string())
    };
    let name = |text: String| RefName::new(text).map_err(|why| why.to_string());

    let branches = REFS / 2;
    let mut refs = Vec::with_capacity(REFS);
    let mut upstreams = HashMap::with_capacity(branches);
    for index in 0..branches {
        // One branch in 100 sits on a row on hand.
        let id = if index % 100 == 0 {
            on_screen[index / 100 % on_screen.len()]
        } else {
            elsewhere(index)?
        };
        let local = name(format!("refs/heads/topic/{index:05}"))?;
        let remote = name(format!("refs/remotes/origin/topic/{index:05}"))?;
        refs.push((local.clone(), id));
        refs.push((remote.clone(), id));
        upstreams.insert(local, remote);
    }
    let head = HeadState::Branch(refs[0].0.clone());

    let started = Instant::now();
    let built = badges(&refs, &head, &upstreams, &[], |_, _| {
        Ok(AheadBehind {
            ahead: 0,
            behind: 0,
        })
    })?;
    let building = started.elapsed();

    let started = Instant::now();
    let mut found = 0;
    for index in 0..branches {
        found += built.on(refs[index * 2].1).len();
    }
    let lookups = started.elapsed();
    println!(
        "refs: {REFS} refs, badges() {building:?}, {branches} lookups {lookups:?} ({} ns each), {found} labels found",
        lookups.as_nanos() / branches as u128
    );
    Ok(built)
}
