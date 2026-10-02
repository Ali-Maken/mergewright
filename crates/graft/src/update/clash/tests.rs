//! `update::clash` tests (m6-conflicts/72b): the refresh rule over the
//! app's reads, the words, and the answer's landing.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use graft_core::journal::{
    EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, OperationKind, PutBack, Touches,
};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;
use graft_git::status::{StatusEntry, WorkingTreeStatus};

use super::*;
use crate::jobs::operate::Operated;
use crate::jobs::undo::JournalRead;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn ids() -> impl FnMut() -> JobId {
    let mut next = 100u64;
    move || {
        next += 1;
        JobId::new(next)
    }
}

const STASH: u8 = 0x5a;

/// A `Committed` pop from a merged index into a clash, carrying its
/// put-back.
pub(crate) fn pop_entry() -> JournalEntry {
    JournalEntry {
        id: EntryId::new(7, [7; 10]),
        at_unix_ms: 7,
        kind: OperationKind::StashPop,
        description: "pop stash@{0}".to_owned(),
        status: EntryStatus::Committed,
        touches: Touches {
            refs: vec!["refs/stash".to_owned()],
            index: true,
            worktree: true,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: BTreeMap::new(),
        refs_after: BTreeMap::new(),
        index_before: Some(IndexState::Merged(oid(1))),
        index_after: Some(IndexState::Unmerged { unmerged: oid(2) }),
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: None,
        pins: vec![oid(STASH)],
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: Some(PutBack {
            stash: oid(STASH),
            why: PutBackWhy::Pop,
        }),
    }
}

fn entry(path: &str, unmerged: bool) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged,
        unmerged_how: None,
        submodule: None,
    }
}

fn refs_with_stash() -> crate::refs::RefBadges {
    let main = RefName::new(b"refs/heads/main".to_vec()).expect("a valid ref name");
    crate::refs::badges(
        &[(main.clone(), oid(1))],
        &HeadState::Branch(main),
        &HashMap::new(),
        &[(oid(STASH), 0)],
        |_, _| unreachable!("no upstream is configured"),
    )
    .expect("badges builds from plain data")
}

/// Every fact a clash is read from, nothing in flight: the journal holding
/// the pop, `notes.md` unmerged (or not), the stash listed.
fn facts(unmerged: bool) -> AppState {
    let mut state = AppState::default();
    state.journal.entries = Loading::Ready(Arc::new(JournalRead {
        entries: vec![pop_entry()],
        unreadable: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![entry("notes.md", unmerged)],
    }));
    state.refs = Some(Arc::new(refs_with_stash()));
    state
}

/// The stage tree `pop_entry` recorded as its index after.
fn popped_index() -> IndexState {
    IndexState::Unmerged { unmerged: oid(2) }
}

/// The index reads `commands` asks for.
fn index_reads(commands: &[Command]) -> Vec<JobId> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::ReadIndexState { id } => Some(*id),
            _ => None,
        })
        .collect()
}

/// A working-tree read landed (`refresh_and_land`'s trigger), holding
/// `status`.
fn tree_read_lands(
    state: &mut AppState,
    status: Vec<StatusEntry>,
    next: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    state.working_tree = Some(Arc::new(WorkingTreeStatus { entries: status }));
    let mut commands = Vec::new();
    refresh_and_land(state, &mut commands, next);
    commands
}

/// The one index read out answers `read`.
fn index_lands(
    state: &mut AppState,
    read: Result<IndexState, String>,
    next: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let IndexRead::Pending { id, .. } = state.clash_index else {
        panic!("no index read out: {:?}", state.clash_index)
    };
    let mut commands = Vec::new();
    index_answered(state, &mut commands, id, read, next);
    commands
}

/// A clash landed this run and tied: the pop in the journal, `notes.md`
/// unmerged, the index read answered with the pop's own stage tree.
fn tied(next: &mut impl FnMut() -> JobId) -> AppState {
    let mut state = facts(true);
    state.clash_facts_seen = true;
    tree_read_lands(&mut state, vec![entry("notes.md", true)], next);
    index_lands(&mut state, Ok(popped_index()), next);
    assert!(state.clash.is_some(), "the fixture's clash is tied");
    state
}

#[test]
fn refresh_clash_follows_the_reads() {
    // A journal read: the pop is in the journal, the tree unmerged — a
    // candidate, so the index is asked for; the tie holds on its answer.
    let mut state = facts(true);
    state.clash_facts_seen = true;
    let mut commands = Vec::new();
    let reading = JobId::new(9);
    state.journal.reading = Some(reading);
    let read = state.journal.entries.clone();
    let Loading::Ready(read) = read else {
        unreachable!("facts() holds a ready journal")
    };
    let product = crate::update::Product::Journal(Ok(read));
    let mut next = ids();
    crate::update::journal::journal_event(&mut state, &mut commands, reading, &product, &mut next);
    assert_eq!(index_reads(&commands).len(), 1, "{commands:?}");
    index_lands(&mut state, Ok(popped_index()), &mut next);
    let clash = state
        .clash
        .clone()
        .expect("the journal read found the clash");
    assert_eq!(clash.clash.index, Some(0));
    assert_eq!(clash.clash.why, PutBackWhy::Pop);
    assert_eq!(clash.origin, LiveOrigin::ThisRun, "found after the launch");

    // Cleared by a working-tree read with nothing unmerged.
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![entry("notes.md", false)],
    }));
    refresh_clash(&mut state);
    assert_eq!(state.clash, None, "nothing unmerged, no clash");
}

/// ADR-0045 §2 (amended): the index is read only while a candidate exists,
/// one read out at a time; the clash held keeps its value while it is out;
/// an answer under any other id is dropped.
#[test]
fn the_index_is_read_only_while_a_candidate_exists() {
    let mut next = ids();

    // Nothing unmerged: no candidate, no read.
    let mut clean = facts(false);
    let commands = tree_read_lands(&mut clean, vec![entry("notes.md", false)], &mut next);
    assert!(index_reads(&commands).is_empty(), "{commands:?}");

    // Unmerged, but no entry carries a put-back (a shell merge's conflict).
    let mut no_put_back = facts(true);
    if let Loading::Ready(read) = &mut no_put_back.journal.entries {
        let mut entries = read.entries.clone();
        entries[0].put_back = None;
        *read = Arc::new(JournalRead {
            entries,
            unreadable: Vec::new(),
            live: Vec::new(),
            interrupted_edits: Vec::new(),
        });
    }
    let commands = tree_read_lands(&mut no_put_back, vec![entry("notes.md", true)], &mut next);
    assert!(index_reads(&commands).is_empty(), "{commands:?}");

    // A candidate, and a clash held from the answer: one read, and the clash
    // stays while it is out.
    let mut state = tied(&mut next);
    let held = state.clash.clone();
    let first = tree_read_lands(&mut state, vec![entry("notes.md", true)], &mut next);
    let [asked] = index_reads(&first)[..] else {
        panic!("one index read: {first:?}")
    };
    assert_eq!(state.clash, held, "kept while the read is out");

    // Another read lands while it is out: none more asked, the one out is
    // marked stale.
    let second = tree_read_lands(&mut state, vec![entry("notes.md", true)], &mut next);
    assert!(
        index_reads(&second).is_empty(),
        "one out at a time: {second:?}"
    );
    assert_eq!(
        state.clash_index,
        IndexRead::Pending {
            id: asked,
            stale: true
        }
    );
}

/// An index answer under another id is dropped; a stale one is not judged
/// and asks again; the fresh one decides.
#[test]
fn a_stale_or_foreign_index_answer_is_not_judged() {
    let mut next = ids();
    let mut state = tied(&mut next);
    let held = state.clash.clone();
    tree_read_lands(&mut state, vec![entry("notes.md", true)], &mut next);
    tree_read_lands(&mut state, vec![entry("notes.md", true)], &mut next);
    assert!(matches!(
        state.clash_index,
        IndexRead::Pending { stale: true, .. }
    ));

    // A stale id is dropped: nothing changes.
    let before = state.clone();
    let mut commands = Vec::new();
    index_answered(
        &mut state,
        &mut commands,
        JobId::new(1),
        Ok(IndexState::Unmerged { unmerged: oid(9) }),
        &mut next,
    );
    assert!(commands.is_empty());
    assert_eq!(state, before, "an answer under another id is dropped");

    // The stale read answers: not judged, one more asked in its place.
    let again = index_lands(
        &mut state,
        Ok(IndexState::Unmerged { unmerged: oid(9) }),
        &mut next,
    );
    assert_eq!(index_reads(&again).len(), 1, "{again:?}");
    assert_eq!(state.clash, held, "a stale answer is not judged");

    // The fresh one answers with an index Graft never recorded: no clash.
    index_lands(
        &mut state,
        Ok(IndexState::Unmerged { unmerged: oid(9) }),
        &mut next,
    );
    assert_eq!(state.clash, None, "the tie broke");
    assert_eq!(
        state.clash_index,
        IndexRead::Read(Ok(IndexState::Unmerged { unmerged: oid(9) }))
    );
}

/// Rule 3: the index could not be read — no clash (nothing named, nothing
/// dropped), and the status says it could not tell, sanitised.
#[test]
fn an_index_read_failure_is_no_clash_and_says_so() {
    let mut next = ids();
    let mut state = tied(&mut next);
    tree_read_lands(&mut state, vec![entry("notes.md", true)], &mut next);
    index_lands(
        &mut state,
        Err("index.lock\u{1b}[31m held".to_owned()),
        &mut next,
    );
    assert_eq!(state.clash, None);
    let Status::Failed(words) = &state.status else {
        panic!("a failure line: {:?}", state.status)
    };
    assert!(
        words.starts_with("can't tell which stash clashed \u{b7} index.lock"),
        "{words}"
    );
    assert!(!words.contains('\u{1b}'), "sanitised: {words:?}");
}

/// Review 72+72b m1: the landing's pane owe goes when the clash does. A
/// clean read first (`esc u`), then an unrelated edit: the pane stays shut.
#[test]
fn the_pane_owe_is_dropped_when_the_clash_ends() {
    let mut next = ids();
    let mut state = tied(&mut next);
    state.clash_pane_owed = true;
    tree_read_lands(&mut state, Vec::new(), &mut next);
    assert_eq!(state.clash, None, "a clean tree has no clash");
    assert!(!state.clash_pane_owed, "the owe went with the clash");

    let mut dirty = entry("later.md", false);
    dirty.untracked = true;
    tree_read_lands(&mut state, vec![dirty], &mut next);
    assert!(
        !state.on_working_tree,
        "no pane opened for an unrelated edit"
    );
}

/// Review 72+72b m1: under a box the owe is not paid — it waits, and is
/// paid once the box goes.
#[test]
fn the_pane_owe_waits_under_a_modal() {
    let mut next = ids();
    let mut state = tied(&mut next);
    state.clash_pane_owed = true;
    state.search_prompt = Some(String::new());
    tree_read_lands(&mut state, vec![entry("notes.md", true)], &mut next);
    index_lands(&mut state, Ok(popped_index()), &mut next);
    assert!(!state.on_working_tree, "not under the box");
    assert!(state.clash_pane_owed, "still owed");

    state.search_prompt = None;
    tree_read_lands(&mut state, vec![entry("notes.md", true)], &mut next);
    index_lands(&mut state, Ok(popped_index()), &mut next);
    assert!(state.on_working_tree, "paid once the box is gone");
    assert!(!state.clash_pane_owed);
}

#[test]
fn refresh_clash_waits_while_a_read_is_out() {
    let mut state = facts(false);
    state.clash = Some(ClashState {
        clash: StashClash {
            entry: EntryId::new(7, [7; 10]),
            stash: oid(STASH),
            index: Some(0),
            why: PutBackWhy::Pop,
        },
        origin: LiveOrigin::ThisRun,
    });
    state.journal.reading = Some(JobId::new(3));
    refresh_clash(&mut state);
    assert!(
        state.clash.is_some(),
        "a fresh status against a journal from before the write is not judged"
    );
}

#[test]
fn a_clash_found_by_the_launch_is_from_the_last_run() {
    let mut state = facts(true);
    state.clash_index = IndexRead::Read(Ok(popped_index()));
    refresh_clash(&mut state);
    assert_eq!(
        state.clash.as_ref().map(|clash| clash.origin),
        Some(LiveOrigin::PreviousRun)
    );
    assert_eq!(
        crate::update::banner::stop_line(&state).as_deref(),
        Some("stash@{0} clash from last run \u{b7} 1 file conflicted")
    );
    assert!(state.stopped.is_none(), "a clash is not a stop");
}

#[test]
fn the_journal_is_read_after_a_write() {
    for scope in [
        graft_ops::apply::Refresh::WorkingTree,
        graft_ops::apply::Refresh::Everything,
    ] {
        let mut state = AppState::default();
        let commands = crate::update::operate::refresh_scope(&mut state, scope, &mut ids());
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadJournal(_))),
            "{scope:?}: {commands:?}"
        );
    }
}

/// `Running::op` pin (67): the `Clashed` answer reads the stash's position
/// from the op that was launched, so the notice names the popped index —
/// `stash@{2}` here — not `stash@{0}`.
#[test]
fn the_notice_names_the_popped_index_after_a_clashed_answer() {
    let mut state = AppState::default();
    let mut commands = Vec::new();
    let mut next = ids();
    crate::update::operate::launch(
        &mut state,
        &mut commands,
        Operation::StashPop {
            index: 2,
            commit: oid(STASH),
        },
        &mut next,
    )
    .expect("nothing else is running");
    let id = state.operating.expect("the pop is running");
    let answer = Operated::Clashed {
        entry: EntryId::new(7, [7; 10]),
        clash: Clash {
            stash: oid(STASH),
            why: PutBackWhy::Pop,
            paths: vec![PathBuf::from("notes.md"), PathBuf::from("todo.md")],
        },
    };
    crate::update::operate::operated(&mut state, id, answer, &mut next);
    assert_eq!(
        state.clash.as_ref().and_then(|clash| clash.clash.index),
        Some(2)
    );
    assert_eq!(state.status, Status::Ready, "no failure line");
    let open = state.resolver.as_ref().expect("landed in the resolver");
    assert_eq!(open.path, PathBuf::from("notes.md"));
    assert_eq!(
        open.notice
            .as_ref()
            .map(|(words, problem)| (words.as_str(), *problem)),
        Some((
            "stash@{2} clashed in 2 files \u{b7} esc, u puts it all back",
            false
        ))
    );
    assert!(state.stopped.is_none() && state.live_entries.is_empty());
}

/// Review 72+72b m2: a `Clashed` answer whose landing is refused — a
/// resolver already open — still says it, on the status line; the open
/// resolver's own notice row is left as it was.
#[test]
fn a_refused_landing_puts_the_notice_on_the_status_line() {
    let mut state = AppState::default();
    let mut next = ids();
    let clash = Clash {
        stash: oid(STASH),
        why: PutBackWhy::Pop,
        paths: vec![PathBuf::from("notes.md"), PathBuf::from("todo.md")],
    };
    let pop_at = |index| Operation::StashPop {
        index,
        commit: oid(STASH),
    };
    clashed(
        &mut state,
        Some(&pop_at(2)),
        EntryId::new(7, [7; 10]),
        &clash,
        &mut next,
    );
    let notice_before = state
        .resolver
        .as_ref()
        .expect("the first landing")
        .notice
        .clone();

    clashed(
        &mut state,
        Some(&pop_at(0)),
        EntryId::new(8, [8; 10]),
        &clash,
        &mut next,
    );

    assert_eq!(
        state.status,
        Status::Waiting("stash@{0} clashed in 2 files \u{b7} esc, u puts it all back".to_owned())
    );
    assert_eq!(
        state.resolver.as_ref().expect("still open").notice,
        notice_before,
        "the resolver's notice row is not doubled"
    );
}

#[test]
fn the_stop_ladder_drops_the_origin_then_the_count_then_the_name() {
    let clash = ClashState {
        clash: StashClash {
            entry: EntryId::new(7, [7; 10]),
            stash: oid(STASH),
            index: Some(1),
            why: PutBackWhy::Apply,
        },
        origin: LiveOrigin::PreviousRun,
    };
    assert_eq!(
        stop_wordings(&clash, 2).wordings,
        [
            "stash@{1} clash from last run \u{b7} 2 files conflicted",
            "stash@{1} clashed \u{b7} 2 conflicted",
            "stash@{1} clashed",
            "stash clashed",
        ]
    );
}

#[test]
fn the_close_words() {
    assert_eq!(
        kept_status(Some(0)),
        Status::Done {
            what: "every file done \u{b7} stash@{0} kept".to_owned(),
            also: None,
            keys: None,
        }
    );
    assert_eq!(
        dropped_status(Some(0), PutBackWhy::Pop),
        Status::Done {
            what: "every file done \u{b7} stash@{0} dropped".to_owned(),
            also: None,
            keys: Some("u brings it back".to_owned()),
        }
    );
    assert!(drops(PutBackWhy::Pop) && drops(PutBackWhy::SetAside));
    assert!(!drops(PutBackWhy::Apply));
}

/// A set-aside's clash (m6-conflicts/74), as `clashed` records it after
/// `launched`.
fn set_aside_clash(state: &mut AppState, launched: Option<&Operation>, files: usize) {
    let mut next = ids();
    let clash = Clash {
        stash: oid(STASH),
        why: PutBackWhy::SetAside,
        paths: (0..files)
            .map(|at| PathBuf::from(format!("file{at}.md")))
            .collect(),
    };
    clashed(state, launched, EntryId::new(7, [7; 10]), &clash, &mut next);
}

fn a_checkout() -> Operation {
    Operation::Checkout(graft_ops::operation::CheckoutParams {
        to: graft_ops::operation::CheckoutTarget::Branch("refs/heads/fix".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    })
}

/// m6-conflicts/74, /84: a put-back that clashes lands in the resolver with
/// the set-aside's names — `branch · main` | `yours · edits set aside`, bar
/// roles `branch`/`yours` (ADR-0043 §4 amended) — and the notice names the
/// edits and what `u` takes back with them: the switch after a checkout, the
/// rebase otherwise.
#[test]
fn a_put_back_clash_lands_on_the_resolver_with_set_aside_names() {
    let mut state = AppState {
        refs: Some(Arc::new(refs_with_stash())),
        ..AppState::default()
    };
    set_aside_clash(&mut state, Some(&a_checkout()), 1);

    let open = state.resolver.as_ref().expect("landed in the resolver");
    assert_eq!(open.path, PathBuf::from("file0.md"));
    assert_eq!(
        open.notice.as_ref().map(|(words, _)| words.as_str()),
        Some("your edits clashed in 1 file \u{b7} esc, u undoes the switch too")
    );
    let names = crate::update::resolver::side_names(&state);
    assert_eq!(
        names.roles,
        graft_ui::conflict_resolver::SideRoles::SetAside
    );
    assert_eq!(names.ours.as_str(), "branch \u{b7} main");
    assert_eq!(names.theirs.as_str(), "yours \u{b7} edits set aside");
    assert_eq!(
        format!("o {}  t {}", names.roles.ours(), names.roles.theirs()),
        "o branch  t yours"
    );

    // After a `c` that finished a stopped rebase there is no launched op:
    // the rebase is what `u` takes back.
    let mut resumed = AppState::default();
    set_aside_clash(&mut resumed, None, 2);
    assert_eq!(
        resumed
            .resolver
            .as_ref()
            .and_then(|open| open.notice.as_ref())
            .map(|(words, _)| words.as_str()),
        Some("your edits clashed in 2 files \u{b7} esc, u undoes the rebase too")
    );
    // A pop's words are unchanged.
    assert_eq!(
        notice_words(Some(0), 2, None),
        "stash@{0} clashed in 2 files \u{b7} esc, u puts it all back"
    );
}

/// m7-power-tools/10b: a new branch switched to over edits sets them aside
/// like a checkout does, and its clash says `u` undoes the switch, not a
/// rebase.
#[test]
fn a_switching_create_clash_is_labelled_switch() {
    let mut state = AppState {
        refs: Some(Arc::new(refs_with_stash())),
        ..AppState::default()
    };
    let create = Operation::BranchCreate(graft_ops::operation::BranchCreateParams {
        name: "nb".to_owned(),
        at: oid(3),
        checkout: true,
        on_edits: graft_ops::operation::OnEdits::SetAside,
    });
    set_aside_clash(&mut state, Some(&create), 1);
    assert_eq!(
        state
            .resolver
            .as_ref()
            .and_then(|open| open.notice.as_ref())
            .map(|(words, _)| words.as_str()),
        Some("your edits clashed in 1 file \u{b7} esc, u undoes the switch too")
    );
}

/// The stop line while a set-aside's clash is open and the resolver shut:
/// `your edits clashed · 1 file conflicted`; the count goes, then all but
/// `your edits clashed`. The stash is never named.
#[test]
fn a_put_back_clash_stop_line_says_your_edits() {
    let mut clash = ClashState {
        clash: StashClash {
            entry: EntryId::new(7, [7; 10]),
            stash: oid(STASH),
            index: Some(0),
            why: PutBackWhy::SetAside,
        },
        origin: LiveOrigin::ThisRun,
    };
    assert_eq!(
        stop_wordings(&clash, 1).wordings,
        [
            "your edits clashed \u{b7} 1 file conflicted",
            "your edits clashed \u{b7} 1 conflicted",
            "your edits clashed",
        ]
    );
    clash.origin = LiveOrigin::PreviousRun;
    assert_eq!(
        stop_wordings(&clash, 2).wordings,
        [
            "your edits clash from last run \u{b7} 2 files conflicted",
            "your edits clashed \u{b7} 2 conflicted",
            "your edits clashed",
        ]
    );
    assert!(stop_wordings(&clash, 2).before_keys);
}

/// The drop after a set-aside's last file answered: `every file done ·
/// set-aside copy dropped ✓ · u brings it back` (ADR-0045 §3 amended) —
/// what was dropped, never the stash's position (he never named it), and
/// not `your edits are back`, which `o branch` everywhere makes false.
#[test]
fn a_put_back_clash_done_names_the_dropped_copy() {
    let closed = Status::Done {
        what: "every file done \u{b7} set-aside copy dropped".to_owned(),
        also: None,
        keys: Some("u brings it back".to_owned()),
    };
    assert_eq!(dropped_status(Some(0), PutBackWhy::SetAside), closed);
    assert_eq!(
        crate::view::status::done_line(
            "every file done \u{b7} set-aside copy dropped",
            None,
            Some("u brings it back")
        ),
        "every file done \u{b7} set-aside copy dropped \u{2713} \u{b7} u brings it back"
    );
    // Through the close: the last file done launches the drop, and its
    // answer is worded as the set-aside's end.
    let mut state = AppState {
        refs: Some(Arc::new(refs_with_stash())),
        ..AppState::default()
    };
    set_aside_clash(&mut state, Some(&a_checkout()), 1);
    let mut commands = Vec::new();
    let mut next = ids();
    crate::update::resolver::close_after_last(&mut state, &mut commands, &mut next);
    let id = state.operating.expect("the drop is running");
    let applied = graft_ops::apply::Applied {
        came_back: Vec::new(),
        entry: pop_entry(),
        refresh: graft_ops::apply::Refresh::Everything,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    };
    crate::update::operate::operated(&mut state, id, Operated::Done(applied), &mut next);
    assert_eq!(state.status, closed);
}
