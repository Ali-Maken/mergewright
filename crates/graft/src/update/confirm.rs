//! The confirmation box: opened by `d` (discard) in the changes pane
//! (`update::operate::request_discard`), answered here — its own keys first,
//! nothing else reaches the pane while it is open (`context_menu.rs`'s own
//! pattern, m3-working-tree-undo-and-stash/04).

use std::path::PathBuf;

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;

use graft_ops::operation::Operation;
use graft_ui::confirm::ConfirmView;

use super::events::Command;
use super::state::{AppState, Status};

/// A confirm box waiting for an answer: what it shows, and what `Confirm`
/// does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingConfirm {
    /// What the box draws.
    pub view: ConfirmView,
    /// What `Confirm` dispatches.
    pub then: ConfirmThen,
}

impl PendingConfirm {
    /// **The one place `y` is decided** (ADR-0044 §3, m6-conflicts/50):
    /// the acting button `y` takes, or `None` where the box swallows it.
    /// `confirm_key` acts on it and `view::rows::box_context` draws it, so
    /// the bar offers `y` exactly where `y` acts.
    ///
    /// The word is read from the button's own label (conductor's decision
    /// (a), 2026-09-25): the discard box says `Delete` for an untracked file
    /// and `Discard` otherwise, and nothing in `then` says which.
    #[must_use]
    pub(crate) fn yes(&self) -> Option<graft_ui::graph_screen::ConfirmYes> {
        if !self.then.takes_y() {
            return None;
        }
        let acting = self.view.choices.get(1)?;
        graft_ui::graph_screen::ConfirmYes::from_label(acting.as_str())
    }
}

impl ConfirmThen {
    /// Whether `y` may confirm this box: ADR-0044 §3's rule and its three
    /// exceptions. The exceptions are the boxes whose acting choice writes
    /// to the remote or rewinds past a recorded state — the lease push (`P`
    /// after a rejection, and `[ Re-push old tip ]`), the undo-after-push
    /// box (three answers, none of them "yes"), and the revert drift guard —
    /// where a `y` typed from habit must never be the answer.
    #[must_use]
    pub(crate) fn takes_y(&self) -> bool {
        match self {
            // The force (lease) push is never `y` (P15 option (b)).
            Self::Operate(Operation::Push(params)) => params.lease.is_none(),
            Self::Operate(Operation::StashDrop { .. } | Operation::Discard(_))
            | Self::Undo { .. }
            | Self::Resume(_)
            | Self::RestartConflict { .. } => true,
            // `Operate(_)`: no box opens over any other operation today, so
            // `y` stays swallowed until one does and is added above on
            // purpose.
            Self::Operate(_) | Self::UndoAfterPush { .. } | Self::RevertTo { .. } => false,
        }
    }
}

/// What a confirmed box goes on to do.
///
/// Two kinds, because the undo drift guard is not an [`Operation`]: `ops`
/// plans an undo from the journal, not from a key, so there is nothing for
/// `app` to hand over but "the same key again, forcing"
/// (m3-working-tree-undo-and-stash/06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfirmThen {
    /// Run this write (`d` discard, stash drop).
    Operate(Operation),
    /// Re-run `u` (or `U`) past the drift rail the user has just read.
    Undo {
        /// `true` for `U`.
        redo: bool,
    },
    /// Re-run the journal's time machine past the same rail
    /// (m5-history-and-ghost-preview/08). Its own variant rather than a
    /// flag on [`ConfirmThen::Undo`] because it is a different call with a
    /// different answer: `revert_to` takes an entry id and comes back as
    /// [`crate::jobs::undo::RevertedTo`].
    RevertTo {
        /// The entry to jump back past.
        entry: graft_core::journal::EntryId,
        /// The entry's own words, carried through so the busy line reads
        /// `revert to before “commit …”` rather than the raw ULID
        /// (review 08 Minor 2).
        description: String,
    },
    /// Carry the live operation forward, skip its step, or abort it — `s`
    /// and `a` on the operation banner, which both box before they run
    /// (m5-history-and-ghost-preview/09). `c` does not: continuing is the
    /// operation doing what the user already asked for.
    Resume(graft_ops::apply::Resume),
    /// `u` on work a push has already sent (ADR-0038 §4): the one box with
    /// three real answers, so the index the cursor is on decides which.
    /// `[ Undo locally ]` runs the ordinary undo; `[ Re-push old tip ]`
    /// opens the lease box, which is the confirm that actually touches the
    /// remote — this one never does.
    UndoAfterPush {
        /// The push entry that sent it. Remembered in
        /// [`crate::update::remote::RemoteState::undo_after_push_asked`] once
        /// answered, so the same push explains itself once.
        push: graft_core::journal::EntryId,
        /// What `[ Re-push old tip ]` would push: the old tip, under a
        /// lease. Boxed because it is three `String`s wide and every other
        /// variant here is a word or two. `None` when the box could not
        /// build one — the remote could not be read, or has not answered
        /// yet — and then the box has no third choice to reach it by.
        repush: Option<Box<graft_ops::operation::PushParams>>,
    },
    /// `d` in the resolver: start this file over, behind a confirm box
    /// (m6-conflicts/05, design note Proposed change 2(c)). Its own variant
    /// rather than a plain `Operate` because the resolver has to reopen the
    /// path once the write lands, which `ConfirmThen::Operate` has no way
    /// to ask for.
    RestartConflict {
        /// The file to put back the way git left it.
        path: PathBuf,
        /// What ops' gate said the restart puts back when the box opened;
        /// ops refuses `RestartChanged` if the stages say otherwise now.
        kind: graft_ops::operation::RestartKind,
    },
}

/// The box's own keys, while it is open: `Left`/`Right` move between
/// `Cancel` and the acting choices and **clamp** at either end, `Tab`
/// cycles, `Enter` acts on whichever is under the cursor, `Esc` closes it
/// without acting. `Ctrl+C` passes through (returns `false`) so the app
/// quits. Every other key is swallowed: the box stays open.
/// Modal: this returns `true` for every other key while a box is open, so
/// the caller sends it nothing else.
pub fn confirm_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(pending) = next.confirm.as_mut() else {
        return false;
    };
    // `Ctrl+C` quits from anywhere, a box included (keymap §2.1), exactly
    // as `preview_key` lets it through.
    if key.code == KeyCode::Char('c') && key.modifiers == Modifiers::CONTROL {
        return false;
    }
    match key.code {
        // The arrows clamp; only `Tab` wraps. The safe choice is index 0
        // and the cursor rests there (ADR-0038 §1), so an `←` that wrapped
        // would put the acting choice one key-repeat away from where the
        // cursor sits — the exact thing the uniform default is for.
        KeyCode::Left => pending.view.left(),
        KeyCode::Right => pending.view.right(),
        KeyCode::Tab | KeyCode::BackTab => pending.view.toggle(),
        KeyCode::Enter => {
            let chosen = pending.view.selected();
            act(next, commands, chosen, next_id);
        }
        // `y` takes the acting button from wherever the cursor is, as `y`
        // confirms every preview (ADR-0044 §3) — only where
        // `PendingConfirm::yes` says so, which is also what the bar reads.
        KeyCode::Char('y') if key.modifiers == Modifiers::NONE && pending.yes().is_some() => {
            act(next, commands, 1, next_id);
        }
        KeyCode::Escape => next.confirm = None,
        // Swallowed: the box stays open, cursor and status untouched
        // (usability walk M1). A stray key — `y` in a box it never
        // confirms, a `j` meant for the graph — must never cancel the box
        // without a word, nor reach what is behind it.
        _ => {}
    }
    true
}

/// Close the box and answer it with choice `chosen`: acts unless `chosen`
/// is `Cancel` (index 0). `Enter` passes the cursor's choice, `y` passes 1,
/// so the two cannot answer the same choice differently.
fn act(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    chosen: usize,
    next_id: &mut impl FnMut() -> JobId,
) {
    // `confirm_key` calls this only with a box open; nothing to answer
    // otherwise.
    let Some(pending) = next.confirm.take() else {
        return;
    };
    // Index 0 is always the safe choice and always where the
    // cursor starts (ADR-0038 §1), so anything else is the box's
    // acting answer. Most boxes have exactly two choices; the
    // undo-after-push box has three, and is handed the index so it
    // can tell `[ Undo locally ]` from `[ Re-push old tip ]`.
    let then = pending.then;
    if chosen != 0 {
        match then {
            ConfirmThen::Operate(op) => {
                super::operate::launch(next, commands, op, next_id);
            }
            ConfirmThen::Undo { redo } => {
                commands.extend(super::undo::forced(next, redo, next_id));
            }
            ConfirmThen::RevertTo { entry, description } => {
                super::undo::revert_to_forced(next, commands, entry, &description, next_id);
            }
            ConfirmThen::Resume(what) => {
                super::banner::resume(next, commands, what, next_id);
            }
            ConfirmThen::UndoAfterPush { push, repush } => {
                super::undo::after_push_chosen(
                    next,
                    commands,
                    chosen,
                    push,
                    repush.map(|params| *params),
                    next_id,
                );
            }
            // A write that began while the box was up owns the
            // index: a second one would run beside it, and the
            // answer of whichever lost the race would be dropped
            // (m6-05 review, Major 3). `restart_box` refuses the
            // same before the box opens.
            ConfirmThen::RestartConflict { .. } if next.operating.is_some() => {
                next.status = Status::sticky("a write is already running");
            }
            ConfirmThen::RestartConflict { path, kind } => {
                super::operate::launch(
                    next,
                    commands,
                    Operation::RestartConflict(graft_ops::operation::RestartParams { path, kind }),
                    next_id,
                );
                // The same "which write is this?" tag `⏎`'s `Stage`
                // carries as `staging`, but on its own field: a
                // restart reopens the file fresh rather than
                // advancing to the next one, and the two must never
                // be confused for the same write landing twice.
                if let Some(open) = next.resolver.as_mut() {
                    open.restarting = next.operating;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use graft_ops::operation::Target;
    use graft_term::sanitize_line;

    use super::*;
    use crate::update::state::AppState;

    fn pending() -> PendingConfirm {
        PendingConfirm {
            view: ConfirmView::new(
                sanitize_line("Discard changes to a.rs?"),
                vec![sanitize_line("consequence")],
                sanitize_line("git restore --worktree -- a.rs"),
            ),
            then: ConfirmThen::Operate(Operation::Discard(Target::Paths(vec!["a.rs".into()]))),
        }
    }

    fn key(code: KeyCode) -> KeyPress {
        KeyPress {
            code,
            modifiers: Modifiers::NONE,
        }
    }

    /// ADR-0038 §1: `Cancel` is where the cursor rests, and walking left
    /// off it must not arrive at the acting choice. `←` on index 0 stays on
    /// index 0 — on the three-choice box too, where `toggle`'s wrap used to
    /// move the cursor *right*.
    #[test]
    fn left_on_cancel_stays_on_cancel() {
        for choices in [
            vec![sanitize_line("Cancel"), sanitize_line("Discard")],
            vec![
                sanitize_line("Cancel"),
                sanitize_line("Undo locally"),
                sanitize_line("Re-push old tip"),
            ],
        ] {
            let mut view = pending();
            view.view = view.view.clone().with_choices(choices);
            let mut state = AppState {
                confirm: Some(view),
                ..AppState::default()
            };
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Left),
                &mut || { graft_core::job::JobId::new(1) }
            ));
            let after = state.confirm.expect("the box stays open").view;
            assert_eq!(after.selected(), 0, "← walked off Cancel");
            assert!(commands.is_empty(), "moving the cursor ran something");
        }
    }

    /// `→` moves onto the acting choice and clamps at the last one: the
    /// user reaches it on purpose, and never past it back onto `Cancel`.
    #[test]
    fn right_moves_onto_the_acting_choice_and_clamps() {
        let mut view = pending();
        view.view = view.view.clone().with_choices(vec![
            sanitize_line("Cancel"),
            sanitize_line("Undo locally"),
            sanitize_line("Re-push old tip"),
        ]);
        let mut state = AppState {
            confirm: Some(view),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        for _ in 0..4 {
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Right),
                &mut || { graft_core::job::JobId::new(1) }
            ));
        }
        assert_eq!(state.confirm.expect("still open").view.selected(), 2);
    }

    #[test]
    fn absent_box_answers_nothing() {
        let mut state = AppState::default();
        let mut commands = Vec::new();
        assert!(!confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Enter),
            &mut || graft_core::job::JobId::new(1)
        ));
    }

    #[test]
    fn tab_toggles_the_cursor() {
        let mut state = AppState {
            confirm: Some(pending()),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Tab),
            &mut || { graft_core::job::JobId::new(1) }
        ));
        assert_eq!(state.confirm.unwrap().view.selected(), 1);
    }

    #[test]
    fn enter_on_cancel_closes_without_dispatching() {
        let mut state = AppState {
            confirm: Some(pending()),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Enter),
            &mut || graft_core::job::JobId::new(1)
        ));
        assert!(state.confirm.is_none());
        assert!(commands.is_empty());
        assert!(state.operating.is_none());
    }

    #[test]
    fn enter_on_confirm_dispatches_the_operation() {
        let mut view_pending = pending();
        view_pending.view.toggle();
        let mut state = AppState {
            confirm: Some(view_pending),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Enter),
            &mut || graft_core::job::JobId::new(7)
        ));
        assert!(state.confirm.is_none());
        assert_eq!(state.operating, Some(graft_core::job::JobId::new(7)));
        assert_eq!(commands.len(), 1);
        let Command::Operate(id, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(*id, graft_core::job::JobId::new(7));
        assert_eq!(**op, Operation::Discard(Target::Paths(vec!["a.rs".into()])));
    }

    #[test]
    fn escape_closes_without_dispatching() {
        let mut state = AppState {
            confirm: Some(pending()),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Escape),
            &mut || graft_core::job::JobId::new(1)
        ));
        assert!(state.confirm.is_none());
        assert!(commands.is_empty());
    }

    fn oid() -> graft_core::oid::Oid {
        graft_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes is a SHA-1")
    }

    fn push_params() -> graft_ops::operation::PushParams {
        graft_ops::operation::PushParams {
            branch: "refs/heads/main".into(),
            remote: "origin".into(),
            remote_branch: "refs/heads/main".into(),
            set_upstream: false,
            lease: None,
            source_commit: None,
        }
    }

    fn entry() -> graft_core::journal::EntryId {
        graft_core::journal::EntryId::new(1, [0; 10])
    }

    fn with_then(then: ConfirmThen) -> PendingConfirm {
        PendingConfirm { then, ..pending() }
    }

    /// A box whose buttons read `labels`, left to right, as its site draws
    /// them.
    fn labelled(then: ConfirmThen, labels: &[&str]) -> PendingConfirm {
        let mut open = with_then(then);
        open.view = open
            .view
            .clone()
            .with_choices(labels.iter().map(|label| sanitize_line(label)).collect());
        open
    }

    fn lease_params() -> graft_ops::operation::PushParams {
        graft_ops::operation::PushParams {
            lease: Some(graft_ops::operation::Lease { expected: oid() }),
            source_commit: Some(oid()),
            ..push_params()
        }
    }

    /// The lease box exactly as `P` after a rejection and `[ Re-push old
    /// tip ]` both build it: `push_box`'s own view over the lease `then`.
    fn lease_box() -> PendingConfirm {
        PendingConfirm {
            view: crate::update::remote::push_box(&lease_params(), Some(1), Some(oid())),
            then: ConfirmThen::Operate(Operation::Push(lease_params())),
        }
    }

    /// One row per `PendingConfirm {` site outside tests (scout 50's
    /// table, re-read 2026-09-25), with the labels each site draws — the
    /// discard-paths site twice, once per label it can draw. The push boxes
    /// use `push_box`'s own view; the rest copy their site's
    /// `with_labels` (the site line is named so a drift is one grep away).
    fn every_box() -> Vec<(&'static str, PendingConfirm)> {
        vec![
            (
                "operate.rs:514 discard a patch",
                labelled(
                    ConfirmThen::Operate(Operation::Discard(Target::Patch {
                        path: "a.rs".into(),
                        patch: b"@@".to_vec(),
                        side: graft_ops::operation::PatchSide::Worktree,
                    })),
                    &["Cancel", "Discard"],
                ),
            ),
            (
                "operate.rs:656 discard a tracked file",
                labelled(pending().then, &["Cancel", "Discard"]),
            ),
            (
                "operate.rs:656 delete an untracked file",
                labelled(pending().then, &["Cancel", "Delete"]),
            ),
            (
                "stash.rs:481 stash drop",
                labelled(
                    ConfirmThen::Operate(Operation::StashDrop {
                        index: 0,
                        commit: oid(),
                    }),
                    &["Keep", "Drop"],
                ),
            ),
        ]
        .into_iter()
        .chain(later_boxes())
        .collect()
    }

    /// The undo, resume and restart rows of [`every_box`], then the push
    /// rows.
    fn later_boxes() -> Vec<(&'static str, PendingConfirm)> {
        vec![
            (
                "banner.rs:501 resume skip",
                labelled(
                    ConfirmThen::Resume(graft_ops::apply::Resume::Skip),
                    &["Cancel", "Skip"],
                ),
            ),
            (
                "banner.rs:534 resume abort",
                labelled(
                    ConfirmThen::Resume(graft_ops::apply::Resume::Abort),
                    &["Cancel", "Abort"],
                ),
            ),
            (
                "after_push.rs:217 undo after push",
                labelled(
                    ConfirmThen::UndoAfterPush {
                        push: entry(),
                        repush: Some(Box::new(lease_params())),
                    },
                    &["Cancel", "Undo locally", "Re-push old tip"],
                ),
            ),
            (
                "undo.rs:525 undo drift guard",
                labelled(ConfirmThen::Undo { redo: false }, &["Stop", "Undo"]),
            ),
            (
                "undo.rs:525 redo drift guard",
                labelled(ConfirmThen::Undo { redo: true }, &["Stop", "Redo"]),
            ),
            (
                "undo.rs:493 revert-to drift guard",
                labelled(
                    ConfirmThen::RevertTo {
                        entry: entry(),
                        description: "commit x".into(),
                    },
                    &["Stop", "Revert"],
                ),
            ),
            (
                "resolver/restart.rs:85 restart conflict",
                labelled(
                    ConfirmThen::RestartConflict {
                        path: "a.rs".into(),
                        kind: graft_ops::operation::RestartKind::Markers,
                    },
                    &["Cancel", "Start over"],
                ),
            ),
        ]
        .into_iter()
        .chain(push_boxes())
        .collect()
    }

    /// The push rows of [`every_box`]: `push_box`'s own views.
    fn push_boxes() -> Vec<(&'static str, PendingConfirm)> {
        vec![
            (
                "remote/push_box.rs:30 plain push",
                PendingConfirm {
                    view: crate::update::remote::push_box(&push_params(), Some(1), Some(oid())),
                    then: ConfirmThen::Operate(Operation::Push(push_params())),
                },
            ),
            ("remote/push_box.rs:30 lease push", lease_box()),
            ("remote/after_push.rs:247 re-push lease", lease_box()),
        ]
    }

    /// ADR-0044 §3's three exceptions: `y` never takes these.
    fn never_y(site: &str) -> bool {
        site.contains("lease") || site.contains("undo after push") || site.contains("revert-to")
    }

    fn press(state: &mut AppState, code: KeyCode, id: u64) -> Vec<Command> {
        let mut commands = Vec::new();
        assert!(confirm_key(state, &mut commands, key(code), &mut || {
            graft_core::job::JobId::new(id)
        }));
        commands
    }

    /// Find-and-pin 1: the word after `y` is the acting button's own label,
    /// lowercased, for every box `y` takes; `None` for the three it never
    /// takes.
    #[test]
    fn the_y_word_is_the_acting_label() {
        for (site, open) in every_box() {
            let expected = (!never_y(site)).then(|| open.view.choices[1].as_str().to_lowercase());
            assert_eq!(
                open.yes().map(|yes| yes.word().to_owned()),
                expected,
                "{site}"
            );
        }
    }

    /// The force (lease) push box, from both sites that open it, and the
    /// other two exceptions: `y` leaves the box up, cursor on `Cancel`,
    /// nothing sent. Ticket 49's box writes to the user's remote.
    #[test]
    fn y_is_swallowed_where_the_box_does_not_take_it() {
        let mut state = AppState::default();
        crate::update::remote::repush_chosen(&mut state, lease_params());
        let rows: Vec<(&str, PendingConfirm)> = every_box()
            .into_iter()
            .filter(|(site, _)| never_y(site))
            .chain([(
                "repush_chosen itself",
                state.confirm.clone().expect("repush_chosen opens a box"),
            )])
            .collect();
        assert_eq!(
            rows.len(),
            5,
            "lease ×2, undo-after-push, revert, repush_chosen"
        );
        for (site, open) in rows {
            assert_eq!(open.yes(), None, "{site}");
            let mut state = AppState {
                confirm: Some(open.clone()),
                status: Status::sticky("before"),
                ..AppState::default()
            };
            let commands = press(&mut state, KeyCode::Char('y'), 1);
            assert_eq!(state.confirm, Some(open), "{site}: `y` changed the box");
            assert!(commands.is_empty(), "{site}: `y` ran {commands:?}");
            assert!(state.operating.is_none(), "{site}: `y` started a write");
            assert_eq!(state.status, Status::sticky("before"), "{site}");
        }
    }

    /// `y` from `Cancel` and `⏎` with the cursor on the acting button are
    /// one answer: same commands, same state after, for every box `y`
    /// takes.
    #[test]
    fn y_and_enter_emit_the_same_commands() {
        let mut checked = 0;
        for (site, open) in every_box().into_iter().filter(|(site, _)| !never_y(site)) {
            let mut by_y = AppState {
                confirm: Some(open.clone()),
                ..AppState::default()
            };
            let mut by_enter = by_y.clone();
            by_enter.confirm.as_mut().expect("open").view.right();
            let y_commands = press(&mut by_y, KeyCode::Char('y'), 9);
            let enter_commands = press(&mut by_enter, KeyCode::Enter, 9);
            assert!(by_y.confirm.is_none(), "{site}: `y` left the box open");
            assert_eq!(y_commands, enter_commands, "{site}");
            assert_eq!(by_y, by_enter, "{site}");
            checked += 1;
        }
        assert_eq!(checked, 10, "every `y`-live row");
    }

    /// The bar and the key agree box by box: the bar draws `y` exactly
    /// where pressing `y` closes the box.
    #[test]
    fn the_bar_offers_y_exactly_where_y_acts() {
        use graft_ui::graph_screen::{Divider, HintContext, hints_for};
        for (site, open) in every_box() {
            let mut state = AppState {
                confirm: Some(open),
                ..AppState::default()
            };
            let context = crate::view::rows::box_context(&state).expect("a box is up");
            assert!(matches!(context, HintContext::ConfirmBox { .. }), "{site}");
            let drawn = hints_for(&context, Divider::Hidden, 187);
            let offers_y = drawn.iter().any(|hint| hint.key == "y");
            press(&mut state, KeyCode::Char('y'), 1);
            let acted = state.confirm.is_none();
            assert_eq!(
                offers_y, acted,
                "{site}: bar offers `y` {offers_y}, `y` acted {acted}"
            );
            assert_eq!(offers_y, !never_y(site), "{site}");
        }
    }

    /// ADR-0044 §3: `y` confirms a plain push box from wherever the cursor
    /// is, as `y` confirms in every preview.
    #[test]
    fn y_confirms_the_push_box_from_cancel() {
        let mut open = with_then(ConfirmThen::Operate(Operation::Push(push_params())));
        open.view = open.view.clone().with_labels("Cancel", "Push");
        assert_eq!(open.view.selected(), 0, "the box opens on Cancel");
        let mut state = AppState {
            confirm: Some(open),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Char('y')),
            &mut || graft_core::job::JobId::new(3)
        ));
        assert!(state.confirm.is_none(), "`y` left the box open");
        let pushes: Vec<_> = commands
            .iter()
            .filter(|c| matches!(c, Command::Operate(_, op) if matches!(**op, Operation::Push(_))))
            .collect();
        assert_eq!(pushes.len(), 1, "one push, got {commands:?}");
    }

    /// Usability walk M1: a key the box does not bind is swallowed. The box
    /// stays, the cursor stays on `Cancel`, the status line is untouched.
    #[test]
    fn a_key_the_box_does_not_bind_leaves_it_open() {
        let mut state = AppState {
            confirm: Some(with_then(ConfirmThen::Operate(Operation::Push(
                push_params(),
            )))),
            status: Status::sticky("before"),
            ..AppState::default()
        };
        // Not `y`: the push box takes it now (m6-conflicts/50); where a box
        // does not, `y_is_swallowed_where_the_box_does_not_take_it` holds it.
        for code in [
            KeyCode::Char('Y'),
            KeyCode::Char('q'),
            KeyCode::Char('j'),
            KeyCode::Char('P'),
            KeyCode::Char(' '),
        ] {
            let before = state.status.clone();
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(code),
                &mut || { graft_core::job::JobId::new(1) }
            ));
            let open = state
                .confirm
                .as_ref()
                .unwrap_or_else(|| panic!("{code:?} closed the box"));
            assert_eq!(open.view.selected(), 0, "{code:?} moved the cursor");
            assert_eq!(state.status, before, "{code:?} changed the status");
            assert!(commands.is_empty(), "{code:?} ran something");
        }
    }

    #[test]
    fn a_key_the_box_does_not_bind_leaves_it_open_in_every_box() {
        for (site, open) in every_box() {
            let mut state = AppState {
                confirm: Some(open.clone()),
                ..AppState::default()
            };
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Char('q')),
                &mut || graft_core::job::JobId::new(1)
            ));
            assert_eq!(state.confirm, Some(open), "{site}: `q` changed the box");
            assert!(commands.is_empty(), "{site}: `q` ran something");
        }
    }

    #[test]
    fn esc_still_closes_every_box_without_acting() {
        for (site, open) in every_box() {
            let mut state = AppState {
                confirm: Some(open),
                ..AppState::default()
            };
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Escape),
                &mut || graft_core::job::JobId::new(1)
            ));
            assert!(state.confirm.is_none(), "{site}: Esc left the box open");
            assert!(commands.is_empty(), "{site}: Esc ran something");
            assert!(state.operating.is_none(), "{site}: Esc started a write");
        }
    }

    /// Keymap §2.1: every modal excepts `Ctrl+C`, which quits (:164). Driven
    /// through `update`, so any handler between the box and `graph_key`
    /// that claimed it would show here.
    #[test]
    fn ctrl_c_with_a_box_open_quits() {
        for (site, open) in every_box() {
            let state = AppState {
                confirm: Some(open),
                ..AppState::default()
            };
            let event = crate::update::events::AppEvent::Core(graft_core::event::Event::Key(
                KeyPress::control(KeyCode::Char('c')),
            ));
            let (next, commands) = crate::update::update(&state, event);
            assert_eq!(
                next.quit,
                Some(crate::update::state::Quit::Requested),
                "{site}: Ctrl+C did not quit"
            );
            assert!(
                !commands.iter().any(|c| matches!(c, Command::Operate(..))),
                "{site}: Ctrl+C ran a write"
            );
        }
    }
}
