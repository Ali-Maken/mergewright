//! The Mistake Radar's own state: the default-branch context every rule
//! needs before it can find anything, and the policy `[radar]` config
//! resolved to.
//!
//! Pure, like every other `update` module: no I/O happens here. The context
//! itself is read by `jobs::radar::run_radar_context`, off the loop's thread.

use std::collections::HashMap;
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_ops::mistake_radar::Policy;
use graft_ops::mistake_radar::probes::ForcePushVerdict;

use crate::jobs::radar::RadarContext;
use crate::refs::Upstream;

use super::detail::Loading;
use super::events::{AppEvent, Command, Product};
use super::state::AppState;

/// The Mistake Radar's state: the default-branch context, once the job that
/// reads it has answered, and the policy `[radar]` config resolved to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RadarState {
    /// The default branch's first-parent chain and its previous reflog
    /// tips, once `jobs::radar::run_radar_context` has answered. `Idle`
    /// until the default branch's tip is known, then `Pending` while the
    /// job runs.
    pub context: Loading<Arc<RadarContext>>,
    /// Which kinds report, and how loudly: `[radar]` config's policy,
    /// starting from `Policy::default()` (PROMPT.md §1.4, every kind on).
    pub policy: Policy,
    /// One force-push verdict per upstream-tracked local branch, by short
    /// name: `Pending` while its probe runs, `Ready` once it answered.
    /// Asked for once the context lands, thrown away by a refresh — a fetch
    /// is exactly when a remote's history can have been rewritten.
    pub force_push: HashMap<String, Loading<ForcePushVerdict>>,
}

/// The default branch's shorthand name and its tip, purely from facts
/// already on `state` — the walk's own resolution (`crate::history`) and the
/// refs job's badges (`crate::refs`). `None` until both have answered, or in
/// a repository with no default branch.
fn default_branch_tip(state: &AppState) -> Option<(RefName, Oid)> {
    let default_ref = state
        .history
        .as_ref()?
        .walk
        .as_ref()?
        .default_branch
        .clone()?;
    let refs = state.refs.as_deref()?;
    let tip = refs
        .branches()
        .find(|(_, name, _, _)| *name == default_ref.shorthand())
        .map(|(id, ..)| id)?;
    Some((default_ref, tip))
}

/// Asks for the radar context once the default branch's tip is known and
/// nothing is already waiting on it (or the last attempt failed). Called at
/// the end of every `update`, which is what satisfies "at startup and at
/// refresh" without a literal second call site: neither startup nor a
/// refresh knows the default branch's tip until the refs job that
/// `Command::ReadRefs` started has answered, so asking for the context is
/// gated on that fact landing, not on `ReadRefs` being dispatched.
pub fn request_context(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut dyn FnMut() -> JobId,
) {
    if !matches!(state.radar.context, Loading::Idle | Loading::Failed(_)) {
        return;
    }
    let Some((default_ref, default_tip)) = default_branch_tip(state) else {
        return;
    };
    let id = next_id();
    state.radar.context = Loading::Pending(id);
    commands.push(Command::RadarContext(id, default_tip, default_ref));
}

/// Routes the two products the radar owns to the function that lands each.
///
/// One arm in `update::answer`, so the radar's own event handling stays in
/// this module rather than spreading across the dispatcher.
pub fn radar_event(state: &mut AppState, event: AppEvent) {
    match event {
        AppEvent::Produced(id, Product::RadarContext(result)) => {
            context_landed(state, id, result);
        }
        AppEvent::Produced(_, Product::ForcePushProbe(branch, verdict)) => {
            force_push_probed(state, branch, verdict);
        }
        // `answer`'s pattern routes only the two above here.
        other => debug_assert!(false, "not a radar event: {other:?}"),
    }
}

/// The radar context job answered. A result for any id but the one being
/// waited on is stale — the default branch's tip moved since it was asked
/// for — and is dropped, the same rule `update::detail`'s jobs use.
pub fn context_landed(state: &mut AppState, id: JobId, result: Result<Arc<RadarContext>, String>) {
    if state.radar.context != Loading::Pending(id) {
        return;
    }
    state.radar.context = match result {
        Ok(context) => Loading::Ready(context),
        Err(failure) => Loading::Failed(failure),
    };
}

/// Asks, once, for one force-push probe per upstream-tracked branch.
///
/// Called at the tail of every `update`, beside [`request_context`], and
/// idempotent: a branch already in `force_push` — pending or answered — is
/// not asked about again, so the first `update` after the context lands is
/// the only one that pushes commands. Gated on the context being `Ready`
/// because nothing shows a radar finding before that, and on the kind being
/// on, so a repository with `force_pushed = "off"` spawns no `git` at all.
///
/// The count is the repository's own upstream-tracked branch count: a few
/// dozen at worst, each one small spawn, so there is no further cap.
pub fn request_force_push_probes(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut dyn FnMut() -> JobId,
) {
    if !matches!(state.radar.context, Loading::Ready(_)) {
        return;
    }
    if state
        .radar
        .policy
        .severity(MistakeRadarKind::ForcePushedRemote)
        .is_none()
    {
        return;
    }
    let Some(refs) = state.refs.as_deref() else {
        return;
    };
    // Collected first: the rest of this borrows `state` mutably.
    let tracked: Vec<(String, Oid)> = refs
        .branches()
        .filter(|(_, _, _, upstream)| !matches!(upstream, Upstream::None))
        .filter_map(|(tip, name, _, _)| Some((std::str::from_utf8(name).ok()?.to_owned(), tip)))
        .collect();
    for (branch, tip) in tracked {
        if state.radar.force_push.contains_key(&branch) {
            continue;
        }
        let id = next_id();
        state
            .radar
            .force_push
            .insert(branch.clone(), Loading::Pending(id));
        commands.push(Command::ForcePushProbe(id, branch, tip));
    }
}

/// One branch's force-push probe answered.
///
/// Kept whatever its id: unlike the detail pane's jobs these are not tied to
/// the selection, and a verdict for a branch is a verdict for that branch.
pub fn force_push_probed(state: &mut AppState, branch: String, verdict: ForcePushVerdict) {
    state
        .radar
        .force_push
        .insert(branch, Loading::Ready(verdict));
}

/// A manual refresh (`Ctrl+R`) throws the context away: the default branch's
/// tip may have moved, and a stale chain would misclassify a foxtrot merge
/// against a tip that is no longer real. [`request_context`] asks for a
/// fresh one once the refreshed refs land.
pub fn reset_for_refresh(state: &mut AppState) {
    state.radar.context = Loading::Idle;
    // A refresh usually follows a fetch, which is when a remote's history
    // can have been rewritten under us: every verdict has to be asked again.
    state.radar.force_push.clear();
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use graft_git::history::Order;
    use graft_git::repo::HeadState;

    use super::*;
    use crate::history::{History, WalkFacts};

    fn oid(n: u8) -> Oid {
        Oid::from_bytes(&[n; 20]).expect("20 bytes is a sha1")
    }

    fn ref_name(name: &str) -> RefName {
        RefName::new(format!("refs/heads/{name}").into_bytes()).expect("a valid ref name")
    }

    fn state_with_default_branch(tip: Option<Oid>) -> AppState {
        let mut state = AppState::default();
        let mut history = History::started();
        history.walk = Some(WalkFacts {
            requested: Order::Topological,
            order: Order::Topological,
            default_branch: Some(ref_name("main")),
            integration: std::sync::Arc::from([]),
        });
        state.history = Some(history);
        if let Some(tip) = tip {
            let head = HeadState::Branch(ref_name("main"));
            let refs = crate::refs::badges(
                &[(ref_name("main"), tip)],
                &head,
                &HashMap::new(),
                |_, _| unreachable!("no upstream is configured, so no count is ever asked for"),
            )
            .expect("badges builds from plain data with no upstream to count");
            state.refs = Some(Arc::new(refs));
        }
        state
    }

    #[test]
    fn asks_once_the_default_branch_and_its_tip_are_both_known() {
        let mut state = state_with_default_branch(None);
        let mut commands = Vec::new();
        request_context(&mut state, &mut commands, &mut || JobId::new(1));
        assert!(commands.is_empty(), "no refs yet: nothing to ask for");
        assert_eq!(state.radar.context, Loading::Idle);

        let mut state = state_with_default_branch(Some(oid(1)));
        request_context(&mut state, &mut commands, &mut || JobId::new(7));
        assert_eq!(
            commands,
            vec![Command::RadarContext(
                JobId::new(7),
                oid(1),
                ref_name("main")
            )]
        );
        assert_eq!(state.radar.context, Loading::Pending(JobId::new(7)));
    }

    #[test]
    fn a_pending_request_is_not_asked_again() {
        let mut state = state_with_default_branch(Some(oid(1)));
        let mut commands = Vec::new();
        request_context(&mut state, &mut commands, &mut || JobId::new(1));
        commands.clear();
        request_context(&mut state, &mut commands, &mut || JobId::new(2));
        assert!(commands.is_empty());
    }

    #[test]
    fn a_stale_answer_is_dropped_and_the_current_one_lands() {
        let mut state = state_with_default_branch(Some(oid(1)));
        state.radar.context = Loading::Pending(JobId::new(2));
        context_landed(
            &mut state,
            JobId::new(1),
            Ok(Arc::new(RadarContext {
                default_chain: HashSet::from([oid(1)]),
                previous_default_tips: Vec::new(),
            })),
        );
        assert_eq!(state.radar.context, Loading::Pending(JobId::new(2)));

        context_landed(
            &mut state,
            JobId::new(2),
            Ok(Arc::new(RadarContext {
                default_chain: HashSet::from([oid(1)]),
                previous_default_tips: Vec::new(),
            })),
        );
        assert!(matches!(state.radar.context, Loading::Ready(_)));
    }

    #[test]
    fn refresh_throws_the_context_away() {
        let mut state = state_with_default_branch(Some(oid(1)));
        state.radar.context = Loading::Ready(Arc::new(RadarContext {
            default_chain: HashSet::new(),
            previous_default_tips: Vec::new(),
        }));
        reset_for_refresh(&mut state);
        assert_eq!(state.radar.context, Loading::Idle);
    }

    /// `main` tracks `origin/main`; `solo` has no upstream.
    fn state_with_a_tracked_branch(tip: Oid) -> AppState {
        let mut state = state_with_default_branch(None);
        let head = HeadState::Branch(ref_name("main"));
        let remote = RefName::new(b"refs/remotes/origin/main".to_vec()).expect("a valid ref name");
        let mut upstreams = HashMap::new();
        upstreams.insert(ref_name("main"), remote.clone());
        let refs = crate::refs::badges(
            &[
                (ref_name("main"), tip),
                (ref_name("solo"), oid(4)),
                (remote, oid(3)),
            ],
            &head,
            &upstreams,
            |_, _| {
                Ok(graft_git::history::AheadBehind {
                    ahead: 1,
                    behind: 1,
                })
            },
        )
        .expect("badges builds from plain data");
        state.refs = Some(Arc::new(refs));
        state.radar.context = Loading::Ready(Arc::new(RadarContext {
            default_chain: HashSet::new(),
            previous_default_tips: Vec::new(),
        }));
        state
    }

    #[test]
    fn one_probe_per_tracked_branch_and_only_once() {
        let mut state = state_with_a_tracked_branch(oid(1));
        let mut commands = Vec::new();
        let mut ids = 0;
        let mut next_id = || {
            ids += 1;
            JobId::new(ids)
        };
        request_force_push_probes(&mut state, &mut commands, &mut next_id);
        assert_eq!(
            commands,
            vec![Command::ForcePushProbe(
                JobId::new(1),
                "main".to_owned(),
                oid(1)
            )],
            "`solo` has no upstream, so nothing is asked about it"
        );
        assert_eq!(
            state.radar.force_push.get("main"),
            Some(&Loading::Pending(JobId::new(1)))
        );

        commands.clear();
        request_force_push_probes(&mut state, &mut commands, &mut next_id);
        assert!(commands.is_empty(), "already asked");
    }

    #[test]
    fn nothing_is_asked_before_the_context_lands_or_with_the_kind_off() {
        let mut state = state_with_a_tracked_branch(oid(1));
        state.radar.context = Loading::Idle;
        let mut commands = Vec::new();
        request_force_push_probes(&mut state, &mut commands, &mut || JobId::new(1));
        assert!(commands.is_empty(), "no context yet");

        let mut state = state_with_a_tracked_branch(oid(1));
        state
            .radar
            .policy
            .set(MistakeRadarKind::ForcePushedRemote, None);
        request_force_push_probes(&mut state, &mut commands, &mut || JobId::new(1));
        assert!(commands.is_empty(), "the kind is off: no `git` at all");
        assert!(state.radar.force_push.is_empty());
    }

    #[test]
    fn a_verdict_lands_under_its_branch_and_a_refresh_asks_again() {
        let mut state = state_with_a_tracked_branch(oid(1));
        let mut commands = Vec::new();
        request_force_push_probes(&mut state, &mut commands, &mut || JobId::new(1));

        let verdict = ForcePushVerdict::Rewritten {
            remote_ref: "origin/main".to_owned(),
            previous: oid(3),
            current: oid(1),
        };
        force_push_probed(&mut state, "main".to_owned(), verdict.clone());
        assert_eq!(
            state.radar.force_push.get("main"),
            Some(&Loading::Ready(verdict))
        );

        reset_for_refresh(&mut state);
        assert!(
            state.radar.force_push.is_empty(),
            "a fetch is when a remote can have been rewritten"
        );
    }
}
