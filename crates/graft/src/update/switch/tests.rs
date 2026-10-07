//! The switch rule over scripted state: which switch opens the preview and
//! which launches at once, what an instant switch refused for edits does,
//! and the words a switch lands with. What each does to a real repository is
//! `tests/it/m7_walk/switch.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::journal::{
    EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, PutBack, PutBackWhy, Touches,
};
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_ops::apply::{Applied, Refresh};

use super::*;
use crate::jobs::operate::Operated;
use crate::update::events::AppEvent;
use crate::update::update;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

fn key(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

fn press(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key(code))))
}

fn ids() -> impl FnMut() -> JobId {
    let mut next = 100u64;
    move || {
        next += 1;
        JobId::new(next)
    }
}

/// Three commits: `feature` on row 0 (`0x03`), `main` on row 1 (`0x02`,
/// checked out), nothing on row 2 (`0x01`). Clean, the cursor on row 0.
fn fixture() -> AppState {
    let refs = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/feature"), oid(3)),
        ],
        &HeadState::Branch(name("refs/heads/main")),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here");
    AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(Arc::new(refs)),
        working_tree: Some(Arc::new(WorkingTreeStatus { entries: vec![] })),
        size: crate::update::Size {
            columns: 120,
            rows: 40,
        },
        ..AppState::default()
    }
}

fn entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: PathBuf::from(path),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

/// `fixture()` with one unstaged edit to a tracked file.
fn with_edits() -> AppState {
    let mut state = fixture();
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![StatusEntry {
            unstaged: Some(Change::Modified),
            ..entry("a.rs")
        }],
    }));
    state
}

/// `.` on `row`, then `⏎` on the item labelled `label`.
fn dot_choose(state: &mut AppState, row: usize, label: &str) -> Vec<Command> {
    crate::update::context_menu::open_for_history_row(state, row);
    let open = state.context_menu.as_mut().expect("a menu is open");
    open.menu.cursor = open
        .menu
        .items
        .iter()
        .position(|item| item.label.as_str() == label)
        .unwrap_or_else(|| panic!("no item {label:?}"));
    let mut commands = Vec::new();
    assert!(crate::update::context_menu::context_menu_key(
        state,
        &mut commands,
        key(KeyCode::Enter)
    ));
    commands
}

fn operated_op(commands: &[Command]) -> Option<&Operation> {
    commands.iter().find_map(|command| match command {
        Command::Operate(_, op, ..) => Some(op.as_ref()),
        _ => None,
    })
}

fn planned(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|command| matches!(command, Command::PlanPreview(..)))
        .count()
}

/// The preview's operation and the key it names.
fn previewed(state: &AppState) -> (&Operation, &'static str) {
    match &state.preview.as_ref().expect("a preview opened").subject {
        PreviewSubject::Operation { op, opened_by } => (op, opened_by),
        other @ PreviewSubject::RevertTo(_) => panic!("not an operation preview: {other:?}"),
    }
}

fn checkout_of(op: &Operation) -> &CheckoutParams {
    match op {
        Operation::Checkout(params) => params,
        other => panic!("not a checkout: {other:?}"),
    }
}

// ── `.` Checkout ────────────────────────────────────────────────────────

#[test]
fn dot_checkout_on_a_clean_tree_switches_at_once() {
    let mut state = fixture();
    let commands = dot_choose(&mut state, 0, "Checkout feature");
    assert!(state.preview.is_none(), "no preview");
    assert_eq!(planned(&commands), 0, "{commands:?}");
    let op = checkout_of(operated_op(&commands).expect("the switch launched"));
    assert_eq!(
        op.to,
        CheckoutTarget::Branch("refs/heads/feature".to_owned())
    );
    assert_eq!(
        op.head_before,
        HeadRef::Branch("refs/heads/main".to_owned())
    );
    assert_eq!(op.on_edits, OnEdits::Refuse, "never sets aside unseen");
    assert!(state.operating.is_some());
}

/// Untracked files alone are not unsaved edits.
#[test]
fn dot_checkout_over_untracked_files_only_switches_at_once() {
    let mut state = fixture();
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![StatusEntry {
            untracked: true,
            ..entry("junk")
        }],
    }));
    let commands = dot_choose(&mut state, 0, "Checkout feature");
    assert!(state.preview.is_none());
    assert!(operated_op(&commands).is_some(), "{commands:?}");
}

#[test]
fn dot_checkout_with_edits_opens_the_set_aside_preview() {
    let mut state = with_edits();
    let commands = dot_choose(&mut state, 0, "Checkout feature");
    assert!(
        operated_op(&commands).is_none(),
        "nothing ran: {commands:?}"
    );
    assert_eq!(planned(&commands), 1);
    let (op, opened_by) = previewed(&state);
    assert_eq!(opened_by, ".");
    assert_eq!(checkout_of(op).on_edits, OnEdits::SetAside);
}

#[test]
fn dot_checkout_detached_still_previews() {
    let mut state = fixture();
    let commands = dot_choose(&mut state, 2, "Checkout 0101010, detached");
    assert!(
        operated_op(&commands).is_none(),
        "nothing ran: {commands:?}"
    );
    let (op, _) = previewed(&state);
    assert_eq!(checkout_of(op).to, CheckoutTarget::Detached(oid(1)));
}

// ── `b ⇥` ───────────────────────────────────────────────────────────────

/// `b`, `nb`, `⇥` on row 0, then `⏎`.
fn b_tab_enter(state: &AppState) -> (AppState, Vec<Command>) {
    let (mut state, _) = press(state, KeyCode::Char('b'));
    for character in "nb".chars() {
        state = press(&state, KeyCode::Char(character)).0;
    }
    let (state, _) = press(&state, KeyCode::Tab);
    press(&state, KeyCode::Enter)
}

#[test]
fn b_tab_with_edits_opens_the_preview() {
    let (state, commands) = b_tab_enter(&with_edits());
    assert!(state.name_prompt.is_none(), "the prompt closed");
    assert!(
        operated_op(&commands).is_none(),
        "nothing ran: {commands:?}"
    );
    assert_eq!(planned(&commands), 1);
    let (op, opened_by) = previewed(&state);
    assert_eq!(opened_by, "b");
    assert_eq!(
        *op,
        Operation::BranchCreate(BranchCreateParams {
            name: "nb".to_owned(),
            at: oid(3),
            checkout: true,
            on_edits: OnEdits::SetAside,
        })
    );
    // `Esc` on it: back to the graph, nothing written.
    let (after, commands) = press(&state, KeyCode::Escape);
    assert!(after.preview.is_none());
    assert!(operated_op(&commands).is_none(), "{commands:?}");
}

#[test]
fn b_tab_on_a_clean_tree_is_unchanged() {
    let (state, commands) = b_tab_enter(&fixture());
    assert!(state.preview.is_none(), "no preview");
    assert_eq!(
        operated_op(&commands),
        Some(&Operation::BranchCreate(BranchCreateParams {
            name: "nb".to_owned(),
            at: oid(3),
            checkout: true,
            on_edits: OnEdits::Refuse,
        }))
    );
}

// ── an instant switch refused for edits made since the read ─────────────

#[test]
fn an_instant_switch_refused_for_edits_lands_on_the_preview() {
    for (target, opened_by) in [
        (SwitchTarget::Branch("refs/heads/feature".to_owned()), "."),
        (
            SwitchTarget::NewBranch {
                name: "nb".to_owned(),
                at: oid(3),
            },
            "b",
        ),
    ] {
        let mut state = fixture();
        let mut next = ids();
        let commands = switch_to(&mut state, target.clone(), opened_by, &mut next);
        let launched = operated_op(&commands).expect("launched").clone();
        let id = state.operating.expect("running");
        let commands = crate::update::operate::operated(
            &mut state,
            id,
            Operated::Failed {
                error: "the words nobody should see".into(),
                refresh: None,
                remote: None,
                refused: Some(Refusal::DirtyTree { count: 1 }),
                edits: None,
            },
            &mut next,
        );
        assert_eq!(planned(&commands), 1, "{target:?}: {commands:?}");
        let (op, key) = previewed(&state);
        assert_eq!(key, opened_by, "{target:?}");
        assert_eq!(
            *op,
            with_on_edits(launched, OnEdits::SetAside),
            "{target:?}"
        );
        assert!(
            !state.status.is_failure(),
            "{target:?}: no refusal line: {:?}",
            state.status
        );
    }
}

/// Any other refusal of an instant switch keeps its own words.
#[test]
fn an_instant_switch_refused_otherwise_keeps_the_refusal() {
    let mut state = fixture();
    let mut next = ids();
    let _ = switch_to(
        &mut state,
        SwitchTarget::Branch("refs/heads/feature".to_owned()),
        ".",
        &mut next,
    );
    let id = state.operating.expect("running");
    let commands = crate::update::operate::operated(
        &mut state,
        id,
        Operated::Failed {
            error: "feature is gone".into(),
            refresh: None,
            remote: None,
            refused: Some(Refusal::NoSuchRef {
                name: "refs/heads/feature".to_owned(),
            }),
            edits: None,
        },
        &mut next,
    );
    assert_eq!(planned(&commands), 0);
    assert!(state.preview.is_none());
    assert!(state.status.is_failure(), "{:?}", state.status);
}

// ── the words on landing ────────────────────────────────────────────────

fn applied(kind: OperationKind, set_aside: bool) -> Applied {
    Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry: JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind,
            description: "the entry's own words".to_owned(),
            status: EntryStatus::Committed,
            touches: Touches::default(),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            worktree_raw_before: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: std::collections::BTreeMap::new(),
            config_after: std::collections::BTreeMap::new(),
            put_back: set_aside.then(|| PutBack {
                stash: oid(9),
                why: PutBackWhy::SetAside,
            }),
        },
        refresh: Refresh::Everything,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    }
}

/// Launch `target` at once, answer `Done`, and return the status line.
fn landed_status(target: SwitchTarget, set_aside: bool) -> Status {
    let mut state = fixture();
    let mut next = ids();
    let _ = switch_to(&mut state, target, ".", &mut next);
    let id = state.operating.expect("running");
    let _ = crate::update::operate::operated(
        &mut state,
        id,
        Operated::Done(applied(OperationKind::Checkout, set_aside)),
        &mut next,
    );
    state.status
}

#[test]
fn switch_words_on_landing() {
    let done = |what: &str, also: Option<&str>| Status::Done {
        what: what.to_owned(),
        also: also.map(str::to_owned),
        keys: Some("u goes back".to_owned()),
    };
    assert_eq!(
        landed_status(SwitchTarget::Branch("refs/heads/feature".to_owned()), false),
        done("on feature", None)
    );
    assert_eq!(
        landed_status(
            SwitchTarget::Track {
                remote: Tip {
                    name: "refs/remotes/origin/sara/x".to_owned(),
                    oid: oid(1),
                },
                local: "sara/x".to_owned(),
            },
            false
        ),
        done("on sara/x (new, tracks origin/sara/x)", None)
    );
    assert_eq!(
        landed_status(SwitchTarget::Branch("refs/heads/feature".to_owned()), true),
        done("on feature", Some("your edits are back"))
    );
    // A detached checkout keeps today's words: the entry's description.
    let mut state = fixture();
    let mut next = ids();
    let _ = crate::update::operate::launch(
        &mut state,
        &mut Vec::new(),
        Operation::Checkout(CheckoutParams {
            to: CheckoutTarget::Detached(oid(1)),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            on_edits: OnEdits::SetAside,
        }),
        &mut next,
    );
    let id = state.operating.expect("running");
    let _ = crate::update::operate::operated(
        &mut state,
        id,
        Operated::Done(applied(OperationKind::Checkout, false)),
        &mut next,
    );
    assert_eq!(state.status, Status::done("the entry's own words"));
}

/// Review m7-16b..21b, Minor 1 (gap M7/10b-q, answered yes): `b <name> ⇥`
/// lands on the new branch in the switch's words, not the entry's
/// `branch nb at <sha7>`; a plain `b <name>` (no switch) keeps them.
#[test]
fn a_switching_create_lands_on_the_new_branch() {
    let landed = |checkout: bool| {
        let mut state = fixture();
        let mut next = ids();
        let _ = crate::update::operate::launch(
            &mut state,
            &mut Vec::new(),
            Operation::BranchCreate(BranchCreateParams {
                name: "nb".to_owned(),
                at: oid(3),
                checkout,
                on_edits: OnEdits::Refuse,
            }),
            &mut next,
        );
        let id = state.operating.expect("running");
        let _ = crate::update::operate::operated(
            &mut state,
            id,
            Operated::Done(applied(OperationKind::BranchCreate, false)),
            &mut next,
        );
        state.status
    };
    assert_eq!(
        landed(true),
        Status::Done {
            what: "on nb".to_owned(),
            also: None,
            keys: Some("u goes back".to_owned()),
        }
    );
    assert_eq!(landed(false), Status::done("the entry's own words"));
}
