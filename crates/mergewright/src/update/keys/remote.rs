//! The three remote keys, sent on to `update::remote`.

use super::predicates::RemoteKey;
use crate::update::events::Command;
use crate::update::state::AppState;

/// One remote key, with `local_jobs` borrowed out for the closure.
pub(super) fn remote_key(next: &mut AppState, commands: &mut Vec<Command>, remote: RemoteKey) {
    let mut local_jobs = next.local_jobs;
    let mut next_id = || local_jobs.next_id();
    let _claimed: bool = match remote {
        RemoteKey::Fetch => crate::update::remote::fetch_key(next, commands, &mut next_id),
        RemoteKey::Pull => crate::update::remote::pull_key(next, commands, &mut next_id),
        RemoteKey::Push => crate::update::remote::push_key(next),
    };
    next.local_jobs = local_jobs;
}
