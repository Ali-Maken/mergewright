//! `update` tests: sweep screens. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// The changes pane for [`screen_for`]: `tab` is where the focus sits,
/// `⇥ diff` from the file list, the other two from the diff
/// (m6-conflicts/70). From the diff there is a diff in it: the write keys
/// there act on the hunk under the diff cursor.
pub(super) fn changes_pane_screen(
    walk: JobId,
    unmerged: bool,
    live: Option<graft_ui::graph_screen::LiveC>,
    tab: graft_ui::graph_screen::TabWord,
) -> AppState {
    let mut state = changes_pane_open(walk, unmerged);
    if let Some(c) = live {
        operation_live_as(
            &mut state,
            c,
            tab == graft_ui::graph_screen::TabWord::Banner,
        );
    }
    if tab != graft_ui::graph_screen::TabWord::Diff {
        state.detail.focus = DetailFocus::Diff;
        let (diff, diff_raw) = long_diff("a.rs");
        state.detail.diff = Loading::Ready(diff);
        state.detail.diff_raw = Some(diff_raw);
    }
    state
}

/// The half of [`states_for`] that is one arm per context — kept apart from
/// the width and the pane, which every arm shares.
///
/// **A list, not a state** (m5/42). Most contexts have one screen behind
/// them, but a context can be the bar for several: the detail pane with an
/// overlay behind it draws one bar over three different overlays, and the
/// three do not swallow the same keys — the Map view claims `p`, `d` and
/// `s`, which the stash list answers. A sweep that picked one of them would
/// be as blind to the other two as the whole sweep was to this state.
#[allow(
    clippy::too_many_lines,
    reason = "one arm per `HintContext` variant, exhaustive on purpose: it grows with the enum"
)]
pub(super) fn screen_for(context: &HintContext) -> Vec<AppState> {
    let walk = JobId::new(701);
    match context {
        // The cursor on the uncommitted-changes row. With a pane open it is
        // open on a *commit*: a pane open on the working tree is the
        // changes pane, which is the next context along.
        HintContext::WorkingTreeRow => {
            let mut state = dirty_on_screen(walk);
            state.on_working_tree = true;
            state.history.as_mut().expect("a walk").cursor = MERGE_ROW;
            vec![state]
        }
        // One row of history under the cursor. The badge and the Mistake
        // Radar marker that tell `BranchRow`, `Marked` and `PlainRow` apart
        // are painted on the row, not held in the state, and no key turns
        // on them.
        // A stash's row is the same screen; `states_for` puts the stash
        // under the cursor after `one_stash` (`stash_under_cursor`).
        // A standing search is its row's screen too; `states_for` adds the
        // search (`search_standing`, m8/15b).
        HintContext::Searching(row) => screen_for(row),
        // An ease-pass row bar (03b) is the same screen: the merge row
        // under the cursor answers `m`, `⏎`, `C`, `x`, `P`, `c`, `space`
        // and `u` whatever kind the bar says it is.
        HintContext::BranchRow { .. }
        | HintContext::PlainRow { .. }
        | HintContext::GraphRow { .. }
        | HintContext::StashRow
        | HintContext::Marked(_)
        | HintContext::Rewritable(_)
        | HintContext::Preview => vec![merge_on_screen(walk, MERGE_ROW).0],
        // The rewrite editor over the graph (m7-power-tools/08): its plan
        // landed where `y` starts, else a todo that cannot run.
        HintContext::RewriteEditor { can_start, .. } => {
            let mut state = merge_on_screen(walk, MERGE_ROW).0;
            super::super::rewrite_editor::tests::open_an_editor_planned(&mut state, *can_start);
            vec![state]
        }
        HintContext::RewriteMessage => {
            let mut state = merge_on_screen(walk, MERGE_ROW).0;
            super::super::rewrite_editor::tests::open_an_editor_field(&mut state);
            vec![state]
        }
        HintContext::Resolver { .. } => vec![resolver_for(walk, context)],
        HintContext::Clash(bar) => vec![resolver_for(walk, &clash_as_resolver(bar))],
        // The focused pane, the blame place, a trail under a screen.
        HintContext::PaneFocused { .. }
        | HintContext::Blame { .. }
        | HintContext::BackToBlame(_) => blame_screens(walk, context),
        HintContext::History { .. } | HintContext::BackToHistory(_) => {
            history_screens(walk, context)
        }
        // `merge` is `!skippable` in every form `every_context` makes, the
        // kind `stopped_operation` already picks from `skippable`.
        HintContext::Banner {
            skippable,
            conflicted,
            merge: _,
        } => vec![stopped_operation(walk, *skippable, *conflicted)],
        // A conflicted operation's banner once the resolver has been open at
        // this stop (ease-pass/05c, Q7 a): its `⏎` goes back to the first
        // open clash. Every operation but a merge is skippable.
        HintContext::BannerPaused { merge } => {
            let mut state = stopped_operation(walk, !*merge, true);
            state.picks.resolver_opened();
            vec![state]
        }
        HintContext::ChangesPane {
            unmerged,
            live,
            tab,
            over_overlay: false,
        } => {
            vec![changes_pane_screen(walk, *unmerged, *live, *tab)]
        }
        // m8/09e (11b row 553): the same pane with each overlay behind it;
        // the three do not swallow the same keys, so all three are pressed.
        HintContext::ChangesPane {
            unmerged,
            tab,
            over_overlay: true,
            ..
        } => EVERY_OVERLAY
            .into_iter()
            .map(|overlay| overlay_over(changes_pane_screen(walk, *unmerged, None, *tab), overlay))
            .collect(),
        // The row's own screen with a merge standing and the banner handed
        // back (m6-conflicts/12): `⇥` leads, and every other key on the
        // row's bar must still answer with an operation live.
        // The row's own screen with an operation standing and the banner
        // handed back (m6-conflicts/12): every key on the row's bar must
        // still answer with it live, and `c` does what the bar says
        // (m6-conflicts/52).
        HintContext::OperationLive {
            row,
            c,
            way_to_banner,
            space_stages,
        } => screen_for(row)
            .into_iter()
            .map(|mut state| {
                operation_live_as(&mut state, *c, way_to_banner.is_some());
                state
            })
            // Each state is the `space` form it claims (07d D1).
            .filter(|state| {
                crate::update::keys::edit_stop::space_stages_here(state) == *space_stages
            })
            .collect(),
        HintContext::Picker => vec![picker_over(merge_on_screen(walk, MERGE_ROW).0)],
        // m8 usability Q2: the same picker, its cursor on `main` (current).
        HintContext::PickerOnCurrent => {
            let mut state = picker_over(merge_on_screen(walk, MERGE_ROW).0);
            if let Some(picker) = state.picker.as_mut() {
                picker.selected = 1;
            }
            vec![state]
        }
        // **The state ticket 31's sweep could not reach** (m5/42): the pane
        // has the keyboard and an overlay, not the graph, stands behind it.
        // One per overlay, because the three swallow different keys.
        HintContext::PaneOverOverlay { opens } => EVERY_OVERLAY
            .into_iter()
            .map(|overlay| overlay_over(commit_pane(walk, *opens), overlay))
            .collect(),
        HintContext::Dragging => vec![badge_in_the_air(walk)],
        // A live box over the graph (rulings 07b, Q1): its bar is its own
        // keys, whatever stands behind it.
        HintContext::ConfirmBox { .. }
        | HintContext::NamePrompt { .. }
        | HintContext::Credential => {
            vec![box_over_the_graph(walk, context)]
        }
        // The m6 summary's form: no screen picks it since ease-pass/12
        // (retiring, 03b's); its keys are the review's, so its state is
        // the review's. The cursor stays on the clashed row: on the
        // combined row `⏎` is drawn but waits for 12b's look (ticket 12
        // NOTES, `enter_on_the_combined_row_does_nothing_yet`).
        // The merge review over a resolved merge, read: one clashed row,
        // one combined (ease-pass/12).
        HintContext::Finish { .. } | HintContext::Review(_) => {
            vec![finish_screen_for(walk, false)]
        }
    }
}

/// [`screen_for`]'s arms that `B` changed (m7-power-tools/20): the focused
/// pane, the place, and any screen with a trail waiting under it.
fn blame_screens(walk: JobId, context: &HintContext) -> Vec<AppState> {
    match context {
        // The same row, with the pane open on it and its diff focused —
        // where `⏎` has nothing left to open. Blamable: the files cursor on
        // a file, not on the `src/` row. Folds: the files list on `src/`,
        // where `commit_pane` leaves it. Neither, with the files list
        // focused: on a `✉ message` row, which has no file and no fold
        // (m7-power-tools/20h).
        HintContext::PaneFocused {
            opens,
            blamable,
            folds,
            ..
        } => {
            let mut state = commit_pane(walk, *opens);
            if *blamable {
                super::super::place::tests::on_a_file(&mut state);
            } else if *opens && !*folds {
                on_the_message(&mut state);
            }
            vec![state]
        }
        // A place open from the pane on a commit, its blame read: a line
        // with a parent for `^`, a find for `n N`, the ignore file for `i`.
        // Without `lines`, that place re-reading after `w` and after `i`
        // (20k), and failed (20n).
        HintContext::Blame {
            finding,
            ignore_file,
            lines: true,
        } => vec![super::super::place::tests::place_on_screen(
            commit_pane(walk, false),
            *finding,
            *ignore_file,
        )],
        HintContext::Blame {
            finding,
            ignore_file,
            lines: false,
        } => super::super::place::tests::place_without_lines(
            commit_pane(walk, false),
            *finding,
            *ignore_file,
        ),
        // The inner screen with a trail waiting under it.
        HintContext::BackToBlame(inner) => screen_for(inner)
            .into_iter()
            .map(|mut state| {
                super::super::place::tests::hide_a_trail(&mut state);
                state
            })
            .collect(),
        other => screen_for(other),
    }
}

/// m7-power-tools/11: `o` takes a commit on the graph only. In a place
/// (blame here) it is unbound: no preview, no command, no status.
#[test]
fn o_is_unbound_in_place_mode() {
    let blame = super::super::place::tests::place_on_screen(
        commit_pane(JobId::new(1), false),
        false,
        false,
    );
    let (after, commands) = update(&blame, key(KeyCode::Char('o')));
    // Every update asks the evil-merge probe for the selected commit once
    // (`detail/ops.rs`, runs on every update); that is the fixture settling,
    // not `o`. Anything else would be `o` doing something.
    let from_o: Vec<_> = commands
        .iter()
        .filter(|c| !matches!(c, Command::EvilProbe(..)))
        .collect();
    assert!(from_o.is_empty(), "{from_o:?}");
    assert!(after.preview.is_none());
    assert_eq!(after.status, blame.status);
}

/// [`screen_for`]'s history place (m7-power-tools/21b): `L` pressed on a
/// blame place over the pane, and the trace answered through `update` as
/// the event loop delivers it. With rows: the list focused, or `Tab` to the
/// file pane. Without: still tracing, failed, and zero rows (a depth-1
/// clone). The way back: the inner screen with that place hidden under it.
pub(super) fn history_screens(walk: JobId, context: &HintContext) -> Vec<AppState> {
    use super::super::place::{TraceAnswer, TraceFailure};
    use graft_git::trace::{SHALLOW_END, TraceEnd};

    let blame = super::super::place::tests::place_on_screen(commit_pane(walk, false), false, false);
    let (tracing, commands) = update(&blame, key(KeyCode::Char('L')));
    let job = trace_job_of(&commands);
    let answered = |answer: Result<TraceAnswer, TraceFailure>| {
        update(
            &tracing,
            AppEvent::Produced(job, Product::Trace(answer.map(Arc::new))),
        )
        .0
    };
    let listed = || {
        answered(Ok(TraceAnswer {
            range: None,
            rows: vec![trace_commit(0xa1), trace_commit(0xa2)],
            end: TraceEnd::Start {
                added_by: trace_commit(0xa2).id,
            },
        }))
    };
    match context {
        HintContext::History {
            rows: true,
            on_file,
        } => {
            let state = listed();
            vec![if *on_file {
                update(&state, key(KeyCode::Tab)).0
            } else {
                state
            }]
        }
        HintContext::History { rows: false, .. } => vec![
            tracing.clone(),
            answered(Err(TraceFailure::Failed("fatal: bad revision".to_owned()))),
            answered(Ok(TraceAnswer {
                range: None,
                rows: Vec::new(),
                end: TraceEnd::GitStops(SHALLOW_END.to_owned()),
            })),
        ],
        HintContext::BackToHistory(inner) => {
            let mut trail = listed().trail;
            trail.shown = false;
            screen_for(inner)
                .into_iter()
                .map(|mut state| {
                    state.trail = trail.clone();
                    state
                })
                .collect()
        }
        other => screen_for(other),
    }
}

/// The trace job `L` asked for.
fn trace_job_of(commands: &[Command]) -> JobId {
    commands
        .iter()
        .find_map(|command| match command {
            Command::TraceHistory { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`L` asks for a trace: {commands:?}"))
}

/// One traced version for [`history_screens`], `byte` its whole id.
fn trace_commit(byte: u8) -> graft_git::trace::TraceCommit {
    graft_git::trace::TraceCommit {
        id: Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("an oid"),
        author: "Lina".to_owned(),
        when: 0,
        subject: "a change".to_owned(),
        path: "a.rs".into(),
        renamed_from: None,
        added: 1,
        removed: 1,
        range_at: Some((1, 3)),
    }
}

/// The resolver state a clash bar's keys are pressed into (ease-pass/03b):
/// the clash screen is today's resolver until L moves it, and every key
/// the clash bar draws is one the resolver answers. An open, picked or
/// solved file is a document (`o t b e u n ⏎ esc` act there); every clash
/// solved is the done file after the last one, where `c` acts.
fn clash_as_resolver(bar: &graft_ui::graph_screen::hints::ClashBar) -> HintContext {
    use graft_ui::graph_screen::hints::ClashAt;
    let roles = bar.roles;
    let all_solved = bar.at == ClashAt::AllSolved;
    HintContext::Resolver {
        base: true,
        restart: true,
        whole_file: false,
        done: all_solved,
        roles,
        finishable: all_solved,
    }
}
