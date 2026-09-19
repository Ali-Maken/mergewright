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
use graft_ops::mistake_radar::probes::{
    Duplicates, ForcePushVerdict, findings_from_duplicates, findings_from_force_push,
};
use graft_ops::mistake_radar::rules::classify_refs;
use graft_ops::mistake_radar::{BranchFacts, Finding, HeadFacts, Policy, UpstreamFacts};

use crate::jobs::radar::RadarContext;
use crate::refs::{RefBadges, RefLabel, Upstream};
use crate::repository::Head;

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
    /// The duplicate-commit probe's groups, over the newest 500 commits.
    /// Asked for once the context lands, thrown away by a refresh, like
    /// [`RadarState::force_push`] — but one repo-wide job, not one per
    /// branch.
    pub duplicates: Loading<Arc<Duplicates>>,
    /// One `DuplicateCommit` finding per copy, by that commit's id, rebuilt
    /// whole each time [`RadarState::duplicates`] lands
    /// (`probes::findings_from_duplicates`). The probe itself is
    /// policy-blind; this is where `[radar] duplicate = off` takes effect.
    pub duplicate_findings: HashMap<Oid, Finding>,
    /// Every finding the refs themselves carry — `classify_refs`'s answer —
    /// indexed by the commit it lands on. Rebuilt by
    /// [`refresh_ref_findings`] only when the refs, `HEAD`, the default
    /// branch, the context or the policy move, so drawing a row is a hash
    /// lookup instead of a walk over every ref (M2 review C1).
    pub ref_findings: HashMap<Oid, Vec<Finding>>,
    /// The same, for the force-push verdicts: each `Rewritten` verdict's
    /// finding at the tip its branch stands on.
    pub force_push_findings: HashMap<Oid, Vec<Finding>>,
    /// Why a branch tip's force-push check could not run, by tip — the
    /// detail pane's `ForcePushNotCheckable` line, from the same rebuild.
    pub force_push_not_checkable: HashMap<Oid, String>,
    /// Bumped whenever a force-push verdict lands or they are all thrown
    /// away: what tells [`refresh_ref_findings`] its inputs moved, without
    /// comparing the verdict map itself.
    pub force_push_revision: u64,
    /// What the three maps above were built from; `None` until the first
    /// rebuild. [`refresh_ref_findings`] is a no-op while it still holds.
    pub built_from: Option<RefFindings>,
}

/// What the ref-derived findings depend on: everything `classify_refs` and
/// the force-push verdicts read off `AppState`.
///
/// Compared by [`RefFindings::same_as`], which takes the two `Arc`s by
/// pointer: a fresh refs answer is a fresh allocation, and the one it
/// replaces is kept alive here for as long as the comparison can see it, so
/// pointer equality cannot confuse two different answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefFindings {
    refs: Option<Arc<RefBadges>>,
    context: Option<Arc<RadarContext>>,
    head: Option<Head>,
    default_branch: Option<RefName>,
    force_push_revision: u64,
    policy: Policy,
}

impl RefFindings {
    /// Would a rebuild produce the same maps? Cheap by construction: no
    /// field here costs more than a pointer or a short name to compare.
    fn same_as(&self, other: &Self) -> bool {
        fn same_arc<T>(one: Option<&Arc<T>>, two: Option<&Arc<T>>) -> bool {
            match (one, two) {
                (None, None) => true,
                (Some(one), Some(two)) => Arc::ptr_eq(one, two),
                _ => false,
            }
        }
        same_arc(self.refs.as_ref(), other.refs.as_ref())
            && same_arc(self.context.as_ref(), other.context.as_ref())
            && self.head == other.head
            && self.default_branch == other.default_branch
            && self.force_push_revision == other.force_push_revision
            && self.policy == other.policy
    }
}

/// Every local branch `classify_refs` needs to know about, from the refs
/// job's badges — `None` refs (not read yet) is no branches, not an error:
/// there is nothing to find findings about before the first refs answer.
///
/// O(refs), which is why nothing on a frame calls it any more: only
/// [`refresh_ref_findings`] does, and only when the refs themselves moved.
#[must_use]
pub fn branch_facts(state: &AppState) -> Vec<BranchFacts<'_>> {
    let Some(refs) = state.refs.as_deref() else {
        return Vec::new();
    };
    let default = default_branch(state);
    refs.branches()
        .filter_map(|(tip, name, _current, upstream)| {
            let name = std::str::from_utf8(name).ok()?;
            Some(BranchFacts {
                name,
                tip,
                is_default: default.is_some_and(|default| default.shorthand() == name.as_bytes()),
                upstream: upstream_facts(upstream),
            })
        })
        .collect()
}

/// The default branch the walk resolved, once it has.
fn default_branch(state: &AppState) -> Option<&RefName> {
    state
        .history
        .as_ref()
        .and_then(|history| history.walk.as_ref())
        .and_then(|walk| walk.default_branch.as_ref())
}

/// `refs::Upstream`, renamed to the shape `graft-ops` classifies on: no new
/// read, the counts are already on hand.
fn upstream_facts(upstream: &Upstream) -> UpstreamFacts {
    match upstream {
        Upstream::None => UpstreamFacts::None,
        Upstream::InSync { .. } => UpstreamFacts::InSync,
        Upstream::Apart(counts) => UpstreamFacts::Apart {
            ahead: u32::try_from(counts.ahead).unwrap_or(u32::MAX),
            behind: u32::try_from(counts.behind).unwrap_or(u32::MAX),
        },
    }
}

/// Where `HEAD` points, from the facts the open-repository job already read
/// (`crate::repository::Head`) — an unreadable `HEAD` reads as unborn: there
/// is nothing for the radar to say about a `HEAD` this state has already put
/// on the status line as a problem.
#[must_use]
pub fn head_facts(state: &AppState) -> HeadFacts<'_> {
    match state.repository.as_ref().map(|facts| &facts.head) {
        Some(Head::Branch(name)) => match std::str::from_utf8(name) {
            Ok(name) => HeadFacts::Branch(name),
            Err(_) => HeadFacts::Unborn,
        },
        Some(Head::Detached(id)) => HeadFacts::Detached(*id),
        Some(Head::Unborn(_) | Head::Unreadable(_)) | None => HeadFacts::Unborn,
    }
}

/// Rebuilds the ref-derived findings, but only when something they are made
/// of moved.
///
/// Called at the tail of every `update`, beside the `request_*` functions.
/// Before M2's review this work happened inside `view::radar::findings_for`,
/// which every painted row and every strip mark calls: O(refs) per row per
/// frame (C1). The refs change once per refs answer, so the findings are
/// built once per refs answer too, and a frame is a hash lookup.
pub fn refresh_ref_findings(state: &mut AppState) {
    // Nothing is findable before the context lands, and a context thrown
    // away takes the maps with it (`reset_for_refresh`), so there is never
    // anything stale to clear here: leaving early keeps an event that
    // changed nothing from changing anything.
    if !matches!(state.radar.context, Loading::Ready(_)) {
        return;
    }
    let now = RefFindings {
        refs: state.refs.clone(),
        context: match &state.radar.context {
            Loading::Ready(context) => Some(Arc::clone(context)),
            _ => None,
        },
        head: state.repository.as_ref().map(|facts| facts.head.clone()),
        default_branch: default_branch(state).cloned(),
        force_push_revision: state.radar.force_push_revision,
        policy: state.radar.policy.clone(),
    };
    if state
        .radar
        .built_from
        .as_ref()
        .is_some_and(|before| before.same_as(&now))
    {
        return;
    }
    let (ref_findings, force_push_findings, not_checkable) = ref_findings_now(state);
    state.radar.ref_findings = ref_findings;
    state.radar.force_push_findings = force_push_findings;
    state.radar.force_push_not_checkable = not_checkable;
    state.radar.built_from = Some(now);
}

/// Findings by the commit they land on.
type ByCommit = HashMap<Oid, Vec<Finding>>;

/// The three maps [`refresh_ref_findings`] stores, from the state as it is
/// right now. Empty until the context is `Ready`, the same gate
/// `view::radar::findings_for` has always applied: nothing shows before the
/// context that makes a foxtrot tellable from an ordinary merge lands.
fn ref_findings_now(state: &AppState) -> (ByCommit, ByCommit, HashMap<Oid, String>) {
    let mut by_commit: ByCommit = HashMap::new();
    let mut force_push: ByCommit = HashMap::new();
    let mut not_checkable: HashMap<Oid, String> = HashMap::new();
    if !matches!(state.radar.context, Loading::Ready(_)) {
        return (by_commit, force_push, not_checkable);
    }
    let branches = branch_facts(state);
    let head = head_facts(state);
    for finding in classify_refs(&branches, &head, &state.radar.policy) {
        by_commit.entry(finding.at).or_default().push(finding);
    }
    // A force-push verdict belongs to a branch, and shows at the tip that
    // branch stands on. `finding.at` is the verdict's own `current`, so a
    // branch that moved since its probe answered contributes nothing rather
    // than marking the wrong row.
    for branch in &branches {
        let Some(Loading::Ready(verdict)) = state.radar.force_push.get(branch.name) else {
            continue;
        };
        if let Some(finding) = findings_from_force_push(verdict, &state.radar.policy)
            .filter(|finding| finding.at == branch.tip)
        {
            force_push.entry(finding.at).or_default().push(finding);
        }
        if let ForcePushVerdict::NotCheckable(reason) = verdict {
            not_checkable
                .entry(branch.tip)
                .or_insert_with(|| reason.clone());
        }
    }
    (by_commit, force_push, not_checkable)
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
        AppEvent::Produced(id, Product::DuplicatesProbe(result)) => {
            duplicates_probed(state, id, result);
        }
        // `answer`'s pattern routes only the three above here.
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

/// The most branches one session ever probes for a force-pushed remote.
///
/// Each probe is a reflog read plus a `git merge-base --is-ancestor` spawn,
/// so the count has to be bounded by what the user can see, not by the
/// repository's branch count: M2's review found 50,000 tracked branches
/// meant 50,000 jobs and up to 100,000 spawns, issued at the tail of an
/// `update` (C1). Nothing above a couple of dozen can be on screen at once.
pub const MAX_FORCE_PUSH_PROBES: usize = 32;

/// Asks for a force-push probe for the tracked branches whose verdict could
/// actually be seen, up to [`MAX_FORCE_PUSH_PROBES`] in a session.
///
/// Called at the tail of every `update`, beside [`request_context`], and
/// idempotent: a branch already in `force_push` — pending or answered — is
/// not asked about again. Gated on the context being `Ready` because nothing
/// shows a radar finding before that, and on the kind being on, so a
/// repository with `force_pushed = "off"` spawns no `git` at all.
///
/// "Could be seen" is [`force_push_candidates`]: the branch `HEAD` is on and
/// the branches standing on a resident row. A verdict only ever draws as a
/// marker on a row, a strip mark for a row, or a line about the selected
/// commit — all of which are rows this window holds. A branch that is not
/// one of them is asked about when scrolling brings it in, until the cap.
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
    if state.radar.force_push.len() >= MAX_FORCE_PUSH_PROBES {
        return;
    }
    // Collected first: the rest of this borrows `state` mutably.
    let wanted = force_push_candidates(state);
    for (branch, tip) in wanted {
        if state.radar.force_push.len() >= MAX_FORCE_PUSH_PROBES {
            break;
        }
        let id = next_id();
        state
            .radar
            .force_push
            .insert(branch.clone(), Loading::Pending(id));
        commands.push(Command::ForcePushProbe(id, branch, tip));
    }
}

/// The tracked branches worth probing right now, `HEAD`'s first: looked up
/// by commit (`RefBadges::on`, one hash lookup) over the rows this window
/// holds, never by walking every ref. The cost is the resident window's, so
/// a repository with 50,000 refs pays the same as one with five.
fn force_push_candidates(state: &AppState) -> Vec<(String, Oid)> {
    let Some(refs) = state.refs.as_deref() else {
        return Vec::new();
    };
    let mut wanted: Vec<(String, Oid)> = Vec::new();
    let want_at = |id: Oid, wanted: &mut Vec<(String, Oid)>| {
        for label in refs.on(id) {
            let RefLabel::Branch { name, upstream, .. } = label else {
                continue;
            };
            if matches!(upstream, Upstream::None) {
                continue;
            }
            let Ok(name) = std::str::from_utf8(name) else {
                continue;
            };
            if state.radar.force_push.contains_key(name) {
                continue;
            }
            if wanted.iter().any(|(already, _)| already == name) {
                continue;
            }
            wanted.push((name.to_owned(), id));
        }
    };
    if let Some(head) = refs.head() {
        want_at(head, &mut wanted);
    }
    if let Some(window) = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
    {
        for row in window.rows.iter() {
            if wanted.len() >= MAX_FORCE_PUSH_PROBES {
                break;
            }
            want_at(row.commit.id, &mut wanted);
        }
    }
    wanted
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
    state.radar.force_push_revision = state.radar.force_push_revision.wrapping_add(1);
}

/// How many of the newest commits the duplicates probe looks at
/// (design note m2-details-and-diffs, hard-to-reverse choice 2). No config
/// key: PROMPT.md does not ask for one.
const DUPLICATES_WINDOW: u32 = 500;

/// Asks, once, for the duplicates probe over the newest
/// [`DUPLICATES_WINDOW`] commits.
///
/// Called at the tail of every `update`, beside [`request_context`] and
/// [`request_force_push_probes`], gated on the context being `Ready` (the
/// same reason force-push waits) and idempotent: once `duplicates` leaves
/// `Idle` it is not asked for again until [`reset_for_refresh`] clears it.
/// Unlike the force-push probe this is not gated on the `duplicate` kind
/// being on — the probe itself is policy-blind (cheap enough not to skip
/// the `git` calls), and only [`duplicates_probed`] applies the policy.
pub fn request_duplicates_probe(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut dyn FnMut() -> JobId,
) {
    if !matches!(state.radar.context, Loading::Ready(_)) {
        return;
    }
    if !matches!(state.radar.duplicates, Loading::Idle) {
        return;
    }
    let id = next_id();
    state.radar.duplicates = Loading::Pending(id);
    commands.push(Command::DuplicatesProbe(id, DUPLICATES_WINDOW));
}

/// The duplicates probe answered. `duplicate_findings` is rebuilt whole from
/// the fresh groups, indexed by each finding's own commit, so
/// `view::radar::findings_for` can look a row up by id.
pub fn duplicates_probed(state: &mut AppState, id: JobId, result: Result<Arc<Duplicates>, String>) {
    if state.radar.duplicates != Loading::Pending(id) {
        return;
    }
    match result {
        Ok(duplicates) => {
            state.radar.duplicate_findings =
                findings_from_duplicates(&duplicates, &state.radar.policy)
                    .into_iter()
                    .map(|finding| (finding.at, finding))
                    .collect();
            state.radar.duplicates = Loading::Ready(duplicates);
        }
        Err(failure) => state.radar.duplicates = Loading::Failed(failure),
    }
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
    state.radar.duplicates = Loading::Idle;
    state.radar.duplicate_findings.clear();
    // The ref-derived maps go with the context they were built against;
    // `refresh_ref_findings` fills them again once a fresh context lands.
    state.radar.ref_findings.clear();
    state.radar.force_push_findings.clear();
    state.radar.force_push_not_checkable.clear();
    state.radar.built_from = None;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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
                &[],
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
            Ok(Arc::new(RadarContext::of_ids([oid(1)], Vec::new()))),
        );
        assert_eq!(state.radar.context, Loading::Pending(JobId::new(2)));

        context_landed(
            &mut state,
            JobId::new(2),
            Ok(Arc::new(RadarContext::of_ids([oid(1)], Vec::new()))),
        );
        assert!(matches!(state.radar.context, Loading::Ready(_)));
    }

    #[test]
    fn refresh_throws_the_context_away() {
        let mut state = state_with_default_branch(Some(oid(1)));
        state.radar.context = Loading::Ready(Arc::new(RadarContext::of_ids([], Vec::new())));
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
            &[],
            |_, _| {
                Ok(graft_git::history::AheadBehind {
                    ahead: 1,
                    behind: 1,
                })
            },
        )
        .expect("badges builds from plain data");
        state.refs = Some(Arc::new(refs));
        state.radar.context = Loading::Ready(Arc::new(RadarContext::of_ids([], Vec::new())));
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

    /// M2 review C1 and M9: the probes are bounded by what could be seen,
    /// not by the repository's refs — 200 tracked branches off-screen cost
    /// one probe, for the branch `HEAD` is on.
    #[test]
    fn probes_are_bounded_by_what_is_on_screen_not_by_the_ref_count() {
        let mut state = state_with_many_tracked_branches(100);
        let mut commands = Vec::new();
        let mut ids = 0;
        let mut next_id = || {
            ids += 1;
            JobId::new(ids)
        };
        request_force_push_probes(&mut state, &mut commands, &mut next_id);
        assert_eq!(
            commands.len(),
            1,
            "only `HEAD`'s branch could be seen: {commands:?}"
        );
        assert!(state.radar.force_push.len() <= MAX_FORCE_PUSH_PROBES);
    }

    /// `main` at `tip` tracking `origin/main`, plus `count` other tracked
    /// branches pointing nowhere the window reaches.
    fn state_with_many_tracked_branches(count: usize) -> AppState {
        let mut state = state_with_default_branch(None);
        let head = HeadState::Branch(ref_name("main"));
        let mut refs = Vec::new();
        let mut upstreams = HashMap::new();
        for index in 0..=count {
            let (local, remote) = if index == 0 {
                ("main".to_owned(), "origin/main".to_owned())
            } else {
                (format!("b{index}"), format!("origin/b{index}"))
            };
            let remote_name = RefName::new(format!("refs/remotes/{remote}").into_bytes())
                .expect("a valid ref name");
            let index = u8::try_from(index).expect("a small test fixture");
            refs.push((ref_name(&local), oid(index + 1)));
            refs.push((remote_name.clone(), oid(index + 128)));
            upstreams.insert(ref_name(&local), remote_name);
        }
        let badges = crate::refs::badges(&refs, &head, &upstreams, &[], |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 1,
                behind: 1,
            })
        })
        .expect("badges builds from plain data");
        state.refs = Some(Arc::new(badges));
        state.radar.context = Loading::Ready(Arc::new(RadarContext::of_ids([], Vec::new())));
        state
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

    #[test]
    fn duplicates_are_asked_for_once_the_context_lands_and_only_once() {
        let mut state = state_with_a_tracked_branch(oid(1));
        let mut commands = Vec::new();
        let mut ids = 0;
        let mut next_id = || {
            ids += 1;
            JobId::new(ids)
        };
        request_duplicates_probe(&mut state, &mut commands, &mut next_id);
        assert_eq!(commands, vec![Command::DuplicatesProbe(JobId::new(1), 500)]);
        assert_eq!(state.radar.duplicates, Loading::Pending(JobId::new(1)));

        commands.clear();
        request_duplicates_probe(&mut state, &mut commands, &mut next_id);
        assert!(commands.is_empty(), "already asked");
    }

    #[test]
    fn nothing_is_asked_for_duplicates_before_the_context_lands() {
        let mut state = state_with_a_tracked_branch(oid(1));
        state.radar.context = Loading::Idle;
        let mut commands = Vec::new();
        request_duplicates_probe(&mut state, &mut commands, &mut || JobId::new(1));
        assert!(commands.is_empty(), "no context yet");
    }

    #[test]
    fn a_duplicates_answer_populates_one_finding_per_copy() {
        let mut state = state_with_a_tracked_branch(oid(1));
        let mut commands = Vec::new();
        request_duplicates_probe(&mut state, &mut commands, &mut || JobId::new(9));

        let duplicates = Duplicates {
            groups: vec![vec![oid(5), oid(6)]],
            window: 500,
            complete: true,
        };
        duplicates_probed(&mut state, JobId::new(9), Ok(Arc::new(duplicates)));

        assert!(matches!(state.radar.duplicates, Loading::Ready(_)));
        assert_eq!(state.radar.duplicate_findings.len(), 2);
        assert!(state.radar.duplicate_findings.contains_key(&oid(5)));
        assert!(state.radar.duplicate_findings.contains_key(&oid(6)));
    }

    #[test]
    fn the_duplicate_kind_off_leaves_findings_empty_though_the_probe_ran() {
        let mut state = state_with_a_tracked_branch(oid(1));
        state
            .radar
            .policy
            .set(MistakeRadarKind::DuplicateCommit, None);
        let mut commands = Vec::new();
        request_duplicates_probe(&mut state, &mut commands, &mut || JobId::new(9));
        assert_eq!(
            commands,
            vec![Command::DuplicatesProbe(JobId::new(9), 500)],
            "the probe is policy-blind: it runs even with the kind off"
        );

        let duplicates = Duplicates {
            groups: vec![vec![oid(5), oid(6)]],
            window: 500,
            complete: true,
        };
        duplicates_probed(&mut state, JobId::new(9), Ok(Arc::new(duplicates)));
        assert!(matches!(state.radar.duplicates, Loading::Ready(_)));
        assert!(state.radar.duplicate_findings.is_empty());
    }

    #[test]
    fn a_stale_duplicates_answer_is_dropped_and_a_refresh_clears_the_rest() {
        let mut state = state_with_a_tracked_branch(oid(1));
        state.radar.duplicates = Loading::Pending(JobId::new(2));
        duplicates_probed(
            &mut state,
            JobId::new(1),
            Ok(Arc::new(Duplicates {
                groups: Vec::new(),
                window: 500,
                complete: true,
            })),
        );
        assert_eq!(state.radar.duplicates, Loading::Pending(JobId::new(2)));

        duplicates_probed(
            &mut state,
            JobId::new(2),
            Ok(Arc::new(Duplicates {
                groups: vec![vec![oid(1), oid(2)]],
                window: 500,
                complete: true,
            })),
        );
        assert_eq!(state.radar.duplicate_findings.len(), 2);

        reset_for_refresh(&mut state);
        assert_eq!(state.radar.duplicates, Loading::Idle);
        assert!(state.radar.duplicate_findings.is_empty());
    }
}
