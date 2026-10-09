//! `update` tests: sweep boxes. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// [`screen_for`]'s finish summary: a merge stopped with nothing left
/// unmerged, `c` pressed on its banner and the summary read.
pub(super) fn finish_screen_for(
    walk: JobId,
    focus: graft_ui::finish_summary::SummaryFocus,
    expandable: bool,
) -> AppState {
    use graft_ui::finish_summary::SummaryFocus;

    let mut state = stopped_operation(walk, false, false);
    let job = JobId::new(702);
    assert!(super::super::finish::open_finish(
        &mut state,
        &mut Vec::new(),
        super::super::finish::ReturnTo::Banner,
        &mut || job
    ));
    super::super::finish::summary_landed(
        &mut state,
        job,
        Ok(Arc::new(graft_ops::finish::FinishSummary {
            into: b"main".to_vec(),
            from: b"feature".to_vec(),
            by_you: vec![graft_ops::finish::ByYou {
                path: "notes.txt".into(),
                result: graft_ops::finish::ResultIs::Theirs,
            }],
            by_git: vec![graft_ops::finish::ByGit {
                path: "README.md".into(),
                change: graft_ops::finish::Change::Modified,
                added: 1,
                removed: 1,
            }],
            message: b"Merge branch 'feature'\n".to_vec(),
            recorded: true,
        })),
    );
    if let Some(open) = state.finish.as_mut() {
        open.cursor = usize::from(expandable);
        open.focus = match focus {
            SummaryFocus::Files => super::super::finish::FinishFocus::Files,
            SummaryFocus::Subject => super::super::finish::FinishFocus::Subject,
            SummaryFocus::Body => super::super::finish::FinishFocus::Body,
        };
    }
    state
}

/// [`screen_for`]'s live boxes, each over the graph with a merge under the
/// cursor.
pub(super) fn box_over_the_graph(walk: JobId, context: &HintContext) -> AppState {
    match context {
        // A box whose `yes` is the context's: the revert drift guard for
        // `None` (`y` never takes it), otherwise the undo drift guard with
        // that button's label, so `y` acts where the bar says it does
        // (m6-conflicts/50).
        HintContext::ConfirmBox { yes } => {
            let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
            let view = graft_ui::confirm::ConfirmView::new(
                graft_term::sanitize_line("Discard changes?"),
                vec![graft_term::sanitize_line("3 files lose their edits.")],
                graft_term::sanitize_line("git restore ."),
            );
            let pending = match yes {
                None => super::super::confirm::PendingConfirm {
                    view: view.with_labels("Stop", "Revert"),
                    then: super::super::confirm::ConfirmThen::RevertTo {
                        entry: graft_core::journal::EntryId::new(1, [0; 10]),
                        description: "commit x".into(),
                    },
                },
                Some(yes) => super::super::confirm::PendingConfirm {
                    view: view.with_labels("Cancel", yes.label()),
                    then: super::super::confirm::ConfirmThen::Undo { redo: false },
                },
            };
            assert_eq!(pending.yes(), *yes, "the fixture draws the box it claims");
            state.confirm = Some(pending);
            state
        }
        HintContext::NamePrompt { toggle } => {
            let (mut state, at) = merge_on_screen(walk, MERGE_ROW);
            let kind = if *toggle {
                super::super::name_prompt::NameKind::Branch { at, checkout: true }
            } else {
                super::super::name_prompt::NameKind::Tag { at }
            };
            state.name_prompt = Some(super::super::name_prompt::NamePrompt {
                kind,
                field: graft_ui::text_input::TextField::new("feature"),
                refusal: None,
            });
            state
        }
        HintContext::Credential => {
            let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
            let job = JobId::new(702);
            state.operating = Some(job);
            state.credential = Some(super::super::credential::CredentialPrompt {
                title: graft_term::sanitize_line("Credentials — example.com"),
                prompt: graft_term::sanitize_line("Username for 'https://example.com':"),
                field: super::super::credential::PromptField::Open(
                    graft_ui::text_input::TextField::new("me"),
                ),
                job,
            });
            state
        }
        other => unreachable!("{other:?} is not a box"),
    }
}

/// The three overlays that stand where the graph stands and take its keys
/// (`AppState::graph_overlay`, minus the commit editor — which claims every
/// key before the panes and draws no hint bar, `view::rows::overlay_context`).
#[derive(Clone, Copy, Debug)]
pub(super) enum Shown {
    Map,
    Journal,
    Stash,
}

/// `state` with the branch picker up over it (m7-power-tools/10), its list
/// landed with two rows — so `↑↓` has somewhere to move and `⏎`/`⇥` a row
/// to act on — and the cursor's own branch as the second.
pub(super) fn picker_over(mut state: AppState) -> AppState {
    use graft_ops::switch_list::{Relation, SwitchKind, SwitchList, SwitchRow};
    let row = |full: &str, byte: u8, current: bool| SwitchRow {
        name: graft_core::ref_name::RefName::new(full.as_bytes().to_vec())
            .expect("a valid ref name"),
        kind: SwitchKind::Local,
        tip: Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
        subject: format!("subject of {full}"),
        author: "Sara Lee".to_owned(),
        when: 0,
        relation: Relation::None,
        current,
    };
    state.picker = Some(super::super::picker::PickerState {
        list: Loading::Ready(Arc::new(SwitchList {
            previous: None,
            rows: vec![
                row("refs/heads/login-page", 3, false),
                row("refs/heads/main", 2, true),
            ],
        })),
        ..super::super::picker::PickerState::default()
    });
    state
}

/// Every overlay a bar can be drawn behind.
pub(super) const EVERY_OVERLAY: [Shown; 3] = [Shown::Map, Shown::Journal, Shown::Stash];

/// `state` with `overlay` open over it, its rows loaded — so the keys that
/// move a selection have somewhere to move to and the sweep is not reading
/// an empty list as a dead key.
pub(super) fn overlay_over(mut state: AppState, overlay: Shown) -> AppState {
    match overlay {
        Shown::Map => {
            state.map.shown = true;
            state.map.data = Loading::Ready(Arc::new(two_branch_map()));
        }
        Shown::Journal => {
            state.journal.shown = true;
            state.journal.entries = Loading::Ready(Arc::new(crate::jobs::undo::JournalRead {
                entries: vec![journal_entry()],
                unreadable: Vec::new(),
                live: Vec::new(),
                interrupted_edits: Vec::new(),
            }));
            // As the read's landing does (m7-power-tools/24).
            crate::update::journal::rebuild(&mut state);
        }
        Shown::Stash => {
            state.stash.shown = true;
            state.stash.entries = Loading::Ready(Arc::new(vec![stash_entry()]));
        }
    }
    state
}

/// Two branches in the Map view: one row to sit on and one to move to, so
/// `j/k move` has somewhere to go.
pub(super) fn two_branch_map() -> crate::map::MapData {
    let branches = (0..2)
        .map(|i| crate::map::MapBranch {
            name: graft_core::ref_name::RefName::new(format!("refs/heads/b{i}").into_bytes())
                .expect("a valid ref name"),
            identity: graft_graph::identity::BranchIdentity::of_name(format!("b{i}").as_bytes()),
            tip: Oid::from_bytes(&[u8::try_from(i + 1).expect("a small row"); Oid::SHA1_BYTES])
                .expect("20 bytes is an oid"),
            tip_date: 0,
            counts: vec![0; 10],
            is_current: i == 0,
            is_default: i == 0,
            merged_into: None,
            marks: Vec::new(),
            complete: true,
        })
        .collect();
    crate::map::MapData {
        branches,
        more: 0,
        from: 0,
        to: 1,
        buckets: 10,
        complete: true,
    }
}
