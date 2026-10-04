//! An open modal owns the bottom row (m6-conflicts/69, Decided 1): the row
//! is the modal's keys, or blank under the Ghost Preview — never a status
//! from before the modal. The status is hidden, not cleared: `esc` draws it
//! again (Decided 2).

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::state::{InProgress, InProgressDetail};

use super::*;
use crate::update::AppEvent;

const SIZES: [(u16, u16); 2] = [(120, 40), (80, 24)];

const REJECTION: &str = "push rejected: origin/main has commits you don't";
const REJECTION_KEYS: &str = "p pull, then m or r · P force with lease";

fn key(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

fn press(state: &AppState, code: KeyCode) -> AppState {
    crate::update::update(state, AppEvent::Core(Event::Key(key(code)))).0
}

/// The frame's last row, as drawn: the graph footer, or the bar a box
/// writes over it.
fn bottom_row(state: &AppState, columns: u16, rows: u16) -> String {
    let screen = draw(state, columns, rows);
    let last = screen
        .lines()
        .rev()
        .find(|line| !line.is_empty())
        .expect("a frame has rows");
    last.trim_matches('"').trim().to_owned()
}

/// The keys the open box's bar should read at this width.
fn box_keys(state: &AppState, columns: u16) -> String {
    let context = crate::view::rows::box_context(state).expect("a box is open");
    graft_ui::graph_screen::hints_text(
        &graft_ui::graph_screen::hints_for(
            &context,
            graft_ui::graph_screen::Divider::Hidden,
            columns,
            graft_ui::keymap::Keymap::shipped(),
        ),
        graft_ui::keymap::Keymap::shipped(),
    )
    .trim()
    .to_owned()
}

/// The graph with no pane open, `main` pushed to `origin` and refused as
/// not a fast-forward: the rejection line stands, `P` is one key away.
fn rejected_push(columns: u16, rows: u16) -> AppState {
    use crate::jobs::remote::RemoteFacts;
    use crate::update::detail::Loading;
    use graft_git::remote::{Remote, TrackedUpstream};

    let mut state = changes_pane_state(columns, rows);
    state.detail.shown = false;
    // The pane closed hands the keyboard back to the graph (as `esc` does);
    // the offer's keys read it (m7-power-tools/36).
    state.detail.focus = graft_ui::detail::DetailFocus::Graph;
    state.remotes = Loading::Ready(std::sync::Arc::new(RemoteFacts {
        remotes: vec![Remote {
            name: "origin".to_owned(),
            fetch_url: "git@example.invalid:team/api.git".to_owned(),
            push_url: None,
        }],
        head_branch: Some("refs/heads/main".to_owned()),
        head_upstream: Some(TrackedUpstream {
            remote: "origin".to_owned(),
            remote_branch: "refs/heads/main".to_owned(),
            tracking_ref: "refs/remotes/origin/main".to_owned(),
            tracking_tip: Some(oid(9)),
        }),
    }));
    state.refs_read = crate::update::RefsRead::Current(graft_core::job::JobId::new(1));
    let sent = state.refs.as_deref().and_then(crate::refs::RefBadges::head);
    state.remote.last_rejected_push = Some(crate::update::remote::RejectedPush {
        params: crate::update::remote::push_params(&state).expect("an upstream to push to"),
        sent,
        // What leaves was read for this lease's key (m7-power-tools/16b).
        leaves: sent.map(|sent| {
            (
                sent,
                oid(9),
                crate::update::remote::LeavesRead::Read(crate::update::remote::Leaves::default()),
            )
        }),
    });
    state.status = Status::Sticky {
        reason: REJECTION.to_owned(),
        keys: Some(REJECTION_KEYS.to_owned()),
    };
    state
}

/// The failing test first (Decided 1): the rejection's sticky line stood
/// under the lease box `P` opened, so the bar named `p pull first` — a key
/// the box swallows — beside a box asking something else.
#[test]
fn the_lease_box_owns_the_bottom_row_over_a_rejection() {
    for (columns, rows) in SIZES {
        let state = press(&rejected_push(columns, rows), KeyCode::Char('P'));
        assert!(state.confirm.is_some(), "P opened the lease box");
        let row = bottom_row(&state, columns, rows);
        assert_eq!(row, "←/→ move  ⏎ choose  esc cancel", "{columns}x{rows}");
    }
}

/// Decided 2: the status is hidden under the box, not cleared. `esc` puts
/// the rejection and its two ways forward back on the row.
#[test]
fn esc_from_the_box_brings_the_rejection_back() {
    for (columns, rows) in SIZES {
        let state = press(&rejected_push(columns, rows), KeyCode::Char('P'));
        let state = press(&state, KeyCode::Escape);
        assert!(state.confirm.is_none(), "esc closed the box");
        let row = bottom_row(&state, columns, rows);
        assert!(row.contains("push rejected"), "{columns}x{rows}: {row}");
        assert!(
            row.contains("p pull, then m or r"),
            "{columns}x{rows}: {row}"
        );
        assert!(
            row.contains("P force with lease"),
            "{columns}x{rows}: {row}"
        );
    }
}

/// Three statuses a modal can open over: a sticky failure, a `Done`, and a
/// `Busy` (another job's). Each carries words no modal's bar says.
fn statuses() -> [(Status, &'static str); 3] {
    [
        (
            Status::Sticky {
                reason: REJECTION.to_owned(),
                keys: Some(REJECTION_KEYS.to_owned()),
            },
            "push rejected",
        ),
        (
            Status::done("committed 1a2b3c4 \"Add OAuth\""),
            "committed 1a2b3c4",
        ),
        (
            Status::Busy("fetch all remotes · esc cancels".to_owned()),
            "fetch all remotes",
        ),
    ]
}

fn stash_entry() -> graft_git::stash::StashEntry {
    graft_git::stash::StashEntry {
        index: 0,
        commit: oid(7),
        message: "WIP on main: 0202020 Commit 2".to_owned(),
        at: graft_core::commit::Timestamp {
            seconds: 1,
            offset_seconds: 0,
        },
        paths: vec![std::path::PathBuf::from("src/main.rs")],
    }
}

/// A rebase stopped on a conflict, the banner focused, a stop live — the
/// footer would word `stopped` here — and the abort box `a` opened over
/// it. `detail` puts the working tree pane up: at 80 columns that is the
/// tab layout, whose banner row is its own bar (`draw_tab_banner`).
fn abort_box_over_a_stop(columns: u16, rows: u16, detail: bool) -> AppState {
    let mut state = changes_pane_state(columns, rows);
    state.detail.shown = detail;
    state.in_progress = Some(InProgressDetail {
        kind: InProgress::Rebase,
        step: Some((3, 7)),
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    state.stopped = Some(crate::update::Stop {
        rewrite: None,
        origin: crate::update::LiveOrigin::ThisRun,
        again: false,
        kind: InProgress::Rebase,
        paths_at_stop: 1,
    });
    state.banner_focus = true;
    let state = press(&state, KeyCode::Char('a'));
    assert!(state.confirm.is_some(), "a opened the abort box");
    state
}

/// Every modal the ticket names, opened over a status that is standing.
/// `None` for the preview, whose row is blank (its context draws no hints).
fn modals(columns: u16, rows: u16) -> Vec<(&'static str, AppState)> {
    let mut modals = Vec::new();

    modals.push((
        "lease box",
        press(&rejected_push(columns, rows), KeyCode::Char('P')),
    ));

    let mut credential =
        credential_state(columns, rows, "Password for 'http://user@127.0.0.1:8443': ");
    credential.detail.shown = false;
    modals.push(("credential prompt", credential));

    let mut name = changes_pane_state(columns, rows);
    name.detail.shown = false;
    assert!(crate::update::name_prompt::open_name_prompt(
        &mut name,
        crate::update::name_prompt::NameKind::Branch {
            at: oid(3),
            checkout: false,
        },
    ));
    modals.push(("name prompt", name));

    let mut drop = changes_pane_state(columns, rows);
    drop.detail.shown = false;
    drop.stash.entries =
        crate::update::detail::Loading::Ready(std::sync::Arc::new(vec![stash_entry()]));
    drop.stash.shown = true;
    let drop = press(&drop, KeyCode::Char('d'));
    assert!(drop.confirm.is_some(), "d opened the stash drop box");
    modals.push(("stash drop box", drop));

    modals.push(("abort box", abort_box_over_a_stop(columns, rows, false)));
    // Below 100 columns the pane is a tab and the banner draws its own key
    // row at the frame's foot (`draw_tab_banner`). Above, the pane stacks
    // under the graph and the graph's footer is the row "abort box" covers.
    if columns < 100 {
        modals.push((
            "abort box, tab layout",
            abort_box_over_a_stop(columns, rows, true),
        ));
    }

    let mut preview = changes_pane_state(columns, rows);
    preview.detail.shown = false;
    let mut next = 50;
    assert!(crate::update::preview::open_preview(
        &mut preview,
        &mut Vec::new(),
        crate::update::preview::PreviewSubject::Operation {
            op: Box::new(graft_ops::operation::Operation::BranchCreate(
                graft_ops::operation::BranchCreateParams {
                    name: "topic".to_owned(),
                    at: oid(2),
                    checkout: true,
                    on_edits: graft_ops::operation::OnEdits::Refuse,
                },
            )),
            opened_by: "b",
        },
        false,
        &mut || {
            next += 1;
            graft_core::job::JobId::new(next)
        },
    ));
    modals.push(("preview", preview));

    modals
}

/// Decided 1, over every modal × every status × both sizes: the row is the
/// modal's keys (or blank under the preview) and never the status's words.
#[test]
fn a_modal_owns_the_bottom_row() {
    for (columns, rows) in SIZES {
        for (status, words) in statuses() {
            for (modal, mut state) in modals(columns, rows) {
                state.status = status.clone();
                let row = bottom_row(&state, columns, rows);
                assert!(
                    !row.contains(words),
                    "{modal} at {columns}x{rows} over {status:?}: {row}"
                );
                if modal == "preview" {
                    assert_eq!(row, "", "{modal} at {columns}x{rows} over {status:?}");
                } else {
                    assert_eq!(
                        row,
                        box_keys(&state, columns),
                        "{modal} at {columns}x{rows} over {status:?}"
                    );
                }
            }
        }
    }
}

/// Decided 3: the name prompt's own refusal is the row's problem line while
/// it is open — the one status-like line a modal draws.
#[test]
fn the_name_prompts_refusal_is_its_bottom_row() {
    for (columns, rows) in SIZES {
        let mut state = rejected_push(columns, rows);
        assert!(crate::update::name_prompt::open_name_prompt(
            &mut state,
            crate::update::name_prompt::NameKind::Branch {
                at: oid(3),
                checkout: false,
            },
        ));
        let state = press(&state, KeyCode::Enter);
        let row = bottom_row(&state, columns, rows);
        assert_eq!(row, "a branch needs a name", "{columns}x{rows}");
    }
}

/// Decided 4's other half: a box opened while **another** job runs leaves
/// that job's `Busy` alone — hidden under the box, drawn again after `esc`.
/// A real fetch (`f`) is running; `b` opens the name prompt over it
/// (`open_name_prompt` does not refuse while a write runs; `P` does —
/// `remote/push_box.rs` `push_key`'s `operation_in_flight` check — so the
/// spec's "fetch, then `P`" pair cannot happen).
#[test]
fn a_busy_of_another_job_survives_a_box() {
    for (columns, rows) in SIZES {
        let mut state = changes_pane_state(columns, rows);
        state.detail.shown = false;
        state.on_working_tree = false;
        let (state, commands) =
            crate::update::update(&state, AppEvent::Core(Event::Key(key(KeyCode::Char('f')))));
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, crate::update::Command::Operate(..))),
            "the fetch launched: {commands:?}"
        );
        let fetching = state.status.clone();
        assert!(
            matches!(&fetching, Status::Busy(words) if words.contains("fetch")),
            "{fetching:?}"
        );
        let before = bottom_row(&state, columns, rows);
        assert!(before.contains("fetch"), "{columns}x{rows}: {before}");

        let boxed = press(&state, KeyCode::Char('b'));
        assert!(boxed.name_prompt.is_some(), "b opened the name prompt");
        assert_eq!(boxed.status, fetching, "the fetch's Busy is not touched");
        assert_eq!(
            bottom_row(&boxed, columns, rows),
            box_keys(&boxed, columns),
            "{columns}x{rows}: hidden under the box"
        );

        let after = press(&boxed, KeyCode::Escape);
        assert!(after.name_prompt.is_none(), "esc closed the prompt");
        assert!(after.operating.is_some(), "and did not stop the fetch");
        assert_eq!(after.status, fetching);
        assert_eq!(
            bottom_row(&after, columns, rows),
            before,
            "{columns}x{rows}"
        );
    }
}

/// Which modal is up, by kind: a key that closes one box and opens another
/// is not "a key that keeps the modal open".
fn modal_kind(state: &AppState) -> Option<&'static str> {
    if state.credential.is_some() {
        Some("credential")
    } else if state.name_prompt.is_some() {
        Some("name")
    } else if state.confirm.is_some() {
        Some("confirm")
    } else if state.preview.is_some() {
        Some("preview")
    } else {
        None
    }
}

/// Decided 3's guard (scout Q5): a key that leaves its modal open never
/// writes `AppState::status` — the modal owns the row, so a status written
/// under it would be unseen until the modal closed, then stale. Per modal,
/// every key its bar names, `⏎`/`y` on empty or unconfirmable input, and a
/// letter. Both sizes; the preview twice — planning, and 67's failed box
/// (a refusal closes it first, so the box a plan can still leave open is
/// the failed one, whose `⏎`/`y` confirm nothing). No `Done`→`Ready`
/// allowance any more: nothing is acknowledged under a modal
/// (m6-conflicts/78, Decided 1). The count is exact, so a modal or key that
/// stops being checked fails here rather than passing on fewer.
#[test]
fn a_key_that_keeps_a_modal_open_leaves_the_status_alone() {
    let keys = [
        KeyCode::Char('y'),
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Enter,
        KeyCode::Backspace,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::Char('n'),
        KeyCode::Char('x'),
    ];
    let mut checked = 0;
    for (columns, rows) in SIZES {
        for (status, _) in statuses() {
            let mut all = modals(columns, rows);
            all.push(("failed preview", failed_preview(columns, rows)));
            for (modal, mut state) in all {
                state.status = status.clone();
                let kind = modal_kind(&state);
                for code in keys {
                    let after = press(&state, code);
                    if modal_kind(&after) != kind {
                        continue;
                    }
                    checked += 1;
                    assert_eq!(
                        after.status, status,
                        "{modal} at {columns}x{rows}, {code:?} left it open and wrote the status"
                    );
                }
            }
        }
    }
    assert_eq!(checked, KEPT_OPEN, "keys that kept a modal open");
}

/// How many (size, status, modal, key) presses keep their modal open in
/// [`a_key_that_keeps_a_modal_open_leaves_the_status_alone`]: of 663
/// (2 sizes × 3 statuses × 13 keys × 8 modals at 120x40, 9 at 80x24), the
/// rest close their modal (`⏎` acting, `y` confirming) and are skipped.
const KEPT_OPEN: usize = 531;

/// 67's failed preview: the plan's read failed, and the box that says so
/// stays open with nothing to confirm.
fn failed_preview(columns: u16, rows: u16) -> AppState {
    let (_, preview) = modals(columns, rows)
        .into_iter()
        .find(|(modal, _)| *modal == "preview")
        .expect("the preview is one of the modals");
    let job = preview.preview.as_ref().expect("open").job;
    let state = crate::update::update(
        &preview,
        AppEvent::Produced(
            job,
            crate::update::Product::Preview(Err(crate::update::preview::PreviewFailure::Failed(
                "git exited 128".to_owned(),
            ))),
        ),
    )
    .0;
    assert!(state.preview.is_some(), "the failed box stays open");
    state
}

mod rungs;
mod unseen;
