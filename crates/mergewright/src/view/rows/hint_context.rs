//! Which hint bar the footer and panes draw: the modal box's, the stop
//! line, overlays, the place screens' and the graph cursor's.

use mergewright_graph::Ancestry;
use mergewright_graph::fold::Keep;
use mergewright_graph::row::Row;
use mergewright_term::sanitize_line;
use mergewright_ui::graph_column::RowPaint;
use mergewright_ui::graph_screen::hints::RowVerbs;
use mergewright_ui::graph_screen::{GraphEntry, HintContext, LiveC, TabWord};
use std::collections::BTreeSet;

use crate::update::{AppState, GraphOverlay};

use super::{
    back_to_place, banner_context, cursor_row_context, live_wrapped, resolver_context,
    search_wrapped,
};

/// **The bar under a live modal box**, or `None` with no box up (rulings
/// 07b, Q1).
///
/// A box claims the next key before any screen, overlay or pane does —
/// `credential_key`, then `name_prompt_claimed`, then `confirm_key`, the
/// order `update::core_event` and `update::keys::key_event` ask them in —
/// so a key of the screen behind it is a key that does nothing, and a line
/// never offers one of those (`update::resolver::answers`). This is asked
/// **first**, before any screen's own context: by [`footer_context`] and
/// [`hint_context`] for the graph's footer and the detail pane's bar, and
/// by `view::frame`, once after whichever screen drew, for every other bar
/// on the frame (the resolver's, the overlays' own). The same shape as F10's
/// `draw_modal_boxes`: one decision, not one per screen.
pub(crate) fn box_context(state: &AppState) -> Option<HintContext> {
    if state.credential.is_some() {
        Some(HintContext::Credential)
    } else if let Some(prompt) = &state.name_prompt {
        Some(HintContext::NamePrompt {
            toggle: matches!(
                prompt.kind,
                crate::update::name_prompt::NameKind::Branch { .. }
            ),
        })
    } else {
        // `y` on the bar exactly where `confirm_key` lets it act: both read
        // `PendingConfirm::yes` (m6-conflicts/50).
        state
            .confirm
            .as_ref()
            .map(|pending| HintContext::ConfirmBox { yes: pending.yes() })
    }
}

/// Why the live operation stopped, worded from this frame's working-tree
/// read (`banner::stop_reason`) so the step and the count stay true as
/// files get resolved, with the shorter wordings the footer falls back to
/// (`banner::stop_wordings`). `None` while nothing is stopped.
pub(crate) fn stopped_line(state: &AppState) -> Option<mergewright_ui::graph_screen::StopWording> {
    let ladder = crate::update::banner::stop_line_wordings(state)?;
    let mut wordings = ladder.wordings.iter().map(|line| sanitize_line(line));
    let full = wordings.next()?;
    let wording = mergewright_ui::graph_screen::StopWording::new(full, wordings.collect());
    Some(if ladder.before_keys {
        wording.before_keys()
    } else {
        wording
    })
}

/// What the footer offers under this frame's rows.
///
/// A focused banner owns the keyboard, so the footer must list the banner's
/// keys and nothing else: a hint bar offering `m merge` under a modal
/// banner names a key that does nothing. An *unfocused* banner claims
/// nothing, so the cursor's own context stands.
pub(crate) fn footer_context(
    state: &AppState,
    rows: &[GraphEntry<'_>],
    banner: Option<&mergewright_ui::graph_screen::BannerView>,
) -> HintContext {
    if let Some(boxed) = box_context(state) {
        return boxed;
    }
    match banner {
        // The resolver stands over the banner as well as the graph, and it
        // is modal: `c`, `s`, `a` reach the file, not the operation. Its
        // own bar is what `hint_context` answers (m6-conflicts/04).
        // The finish summary likewise, over the resolver too (m6-conflicts/19).
        _ if state.resolver.is_some() || state.finish.is_some() => hint_context(state, rows),
        Some(banner) if banner.focused => banner_context(state, banner),
        _ => back_to_place(
            state,
            live_wrapped(state, search_wrapped(state, hint_context(state, rows))),
        ),
    }
}

/// What `c` does in the graph or the changes pane right now, or `None`
/// while nothing is live and `c` commits. The one mapping from
/// [`pane_c_acts`](crate::update::commit_editor::pane_c_acts) to the bar's
/// words, so the label and the key cannot come apart.
pub(super) fn live_c(state: &AppState) -> Option<LiveC> {
    use crate::update::commit_editor::{live_operation, pane_c_acts};
    use mergewright_git::state::InProgress;
    live_operation(state).map(|live| match (live.0, pane_c_acts(live)) {
        (_, false) => LiveC::Refuses,
        (InProgress::Merge, true) => LiveC::Finish,
        // A rewrite's `edit` stop: `a amend it` leads (07b).
        (InProgress::Rebase, true) if crate::update::commit_editor::at_edit_stop(state) => {
            LiveC::ContinueAmends
        }
        (_, true) => LiveC::Continue,
    })
}

/// The changes pane's `⇥` word: where one `Tab` lands from here, read off
/// the one reading of the cycle (`update::banner::tab_target`,
/// m6-conflicts/70). A target the pane's bar never shows while it has
/// focus (`Files`, or no move at all under a modal, which draws its own
/// bar) falls to `graph`, the word the pane said before.
fn tab_word(state: &AppState) -> TabWord {
    use crate::update::banner::{TabTarget, tab_target};
    match tab_target(state) {
        Some(TabTarget::Diff) => TabWord::Diff,
        Some(TabTarget::Banner) => TabWord::Banner,
        Some(TabTarget::Graph | TabTarget::Files) | None => TabWord::Graph,
    }
}

/// The hint context the detail pane hints with when it owns the whole
/// screen — the sub-100-column `Tab` layout with the Detail tab up, where
/// the graph screen is not drawn and its footer is not there to borrow
/// (m5-history-and-ghost-preview/29).
///
/// Built from state alone, with no drawn rows: an overlay (the Map view,
/// the commit editor) can own the primary tab while the pane is up, and
/// there is no `GraphView` on that path to read a context off. So the
/// cursor row's own badges — the one thing the rows carry that state does
/// not — are out of reach here and the base is `PlainRow`. The graph's own
/// path passes `GraphView::hint_context` instead, which does have them.
pub(crate) fn pane_hint_context(state: &AppState) -> HintContext {
    back_to_place(state, hint_context(state, &[]))
}

/// **The bar an open overlay puts the footer in**, or `None` when the graph
/// is the thing behind the cursor after all.
///
/// The Map view, the journal browser and the stash list stand *where the
/// graph stands* and answer the keys the graph would: each one's
/// `shown_key` ends in a catch-all that claims the key and does nothing
/// with it (`update::map`, `update::journal`, `update::stash`). So while
/// one is open the graph's own vocabulary — `b`, `t`, `c`, `m`, `r`, `y`,
/// `q`, `x`, `.` — reaches nothing at all.
///
/// Which of the two bars depends on **who has the keyboard**, exactly as
/// `update::keys::pane_and_overlay_key` decides it:
///
/// - the graph has focus, so the overlay is first and answers everything:
///   no context here — the overlay's own footer is the bar (m8/11e F2), and
///   the frame draws no graph bar under it;
/// - the detail pane has focus, so `detail::keys::detail_key` is first and
///   the overlay swallows whatever the pane leaves: the pane's own keys
///   and the two globals that are dispatched above both
///   ([`HintContext::PaneOverOverlay`]).
///
/// Until m5/42 this function did not exist and the footer under an overlay
/// was the graph's row context — a bar of nine verbs, every one of them
/// dead (m5/36's probe, this ticket's
/// `update::tests::the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows`).
///
/// **The commit editor is the overlay that is not here.** It is modal in
/// the stronger sense — `commit_editor::editor_key` claims every key but
/// `Ctrl+C` before the panes are reached — and it never draws a hint bar
/// to be wrong on: `view::frame::draw_commit_editor` refuses the `Tab`
/// layout outright, and the split layout above 100 columns draws no hint
/// row (`view::frame::split_for_detail`). A bar drawn for it would be a
/// bar the app never shows.
fn overlay_context(state: &AppState) -> Option<HintContext> {
    match state.graph_overlay()? {
        // The Map, the journal and the stash list draw their own footers
        // (mergewright-ui `map_screen`, `journal_screen`, `stash_screen`), which
        // carry their keys and the `? keys` doorway (m8/11e F2: one bar per
        // place). With the graph focused there is no hint bar on the frame
        // to be about: `view::frame` draws the overlay where the graph would
        // be, and the Detail tab's row needs the pane focused. So, as for the
        // commit editor, the overlay adds no context of its own: the caller
        // answers for the graph under it, a bar the frame never draws.
        GraphOverlay::Map | GraphOverlay::Journal | GraphOverlay::Stash
            if state.graph_has_focus() =>
        {
            return None;
        }
        GraphOverlay::Map | GraphOverlay::Journal | GraphOverlay::Stash => {}
        GraphOverlay::CommitEditor => return None,
        // Modal over the graph and the pane (keymap §2.7): its own bar.
        GraphOverlay::Rewrite => return crate::view::rewrite_editor::rewrite_context(state),
        // Modal in the stronger sense, like the commit editor — but unlike
        // it, the resolver draws a bar, so it has one. It covers the detail
        // pane too, which is why it answers before the focus test below:
        // there is no pane on screen to have the keyboard.
        //
        GraphOverlay::Resolver => return Some(resolver_context(state)),
        // Modal, covers the detail pane, draws its own bar: the resolver's
        // case (m6-conflicts/19).
        GraphOverlay::Finish => return Some(super::super::finish::finish_context(state)),
        // Modal over the graph and the pane (keymap §2.1): its own bar.
        GraphOverlay::Place => return place_context(state),
    }
    // The changes pane answers its own keys ahead of the overlay (`space`,
    // `d`, `⌃A`, `e`, `⇥`): its own bar, less what the overlay swallows
    // (m8/09e, 11b row 553). A commit's files or diff keep the short bar.
    if state.on_changes_pane() {
        return Some(HintContext::ChangesPane {
            unmerged: cursor_file_unmerged(state),
            live: live_c(state),
            tab: tab_word(state),
            over_overlay: true,
        });
    }
    Some(HintContext::PaneOverOverlay {
        opens: crate::update::detail::enter_opens(&state.detail),
    })
}

/// The blame place's bar (m7-power-tools/20): `n N` only while a find is
/// active, `i` only while the repository lists revisions to ignore — the
/// conditions `update::place::place_key` answers on; without lines (a read
/// in flight, or a failed one) no `⏎`, `^` or `n N`, which need them (20k,
/// 20n).
///
/// The history place's bar (m7-power-tools/21b): its keys only once the
/// trace answered with a row — tracing, failed or empty, `esc back` alone —
/// and `j/k`, `tab` said for whichever pane has the keys.
pub(crate) fn place_context(state: &AppState) -> Option<HintContext> {
    if let Some(history) = state.trail.history_on_screen() {
        return Some(HintContext::History {
            rows: !history.rows().is_empty(),
            on_file: history.focus == mergewright_ui::history::HistoryFocus::File,
        });
    }
    let blame = state.trail.on_screen()?;
    Some(HintContext::Blame {
        finding: blame.find.is_some(),
        ignore_file: blame.ignore_line().is_some(),
        lines: matches!(blame.body, crate::update::place::BlameLoad::Ready { .. }),
    })
}

/// The footer's context for this frame (`hints::hints_for`, ADR-0037 §4):
/// the changes pane and the working-tree row both win over anything the
/// cursor sits on, since they are full-screen states — **the pane first**,
/// because it is the more specific of the two. The pane opens *on* the
/// uncommitted-changes row and the cursor stays there, so `on_working_tree`
/// and `on_changes_pane()` are true together for the whole time the pane is
/// up; testing the row first made the pane's own keys unreachable and hinted
/// `⏎ changes` to someone already in the changes (m5-03 review, Critical 1).
///
/// Otherwise the cursor row's own badges say whether it is a branch, and its
/// Mistake Radar marker (`GraphEntry::marker`) wraps whichever context in
/// `Marked`, exactly as `hints_for` interprets it. `Journal` and `Stash` are
/// never produced here — `view::frame` draws those overlays instead of the
/// graph screen while they are shown, so the graph's own footer never has to
/// hint them. `<> resize` is not a context: `GraphView::divider_shown` carries
/// it, for every context alike.
pub(super) fn hint_context(state: &AppState, rows: &[GraphEntry<'_>]) -> HintContext {
    // A live box before anything: it takes the key before the drag does.
    if let Some(boxed) = box_context(state) {
        return boxed;
    }
    // The branch picker (m7-power-tools/10) is a text field over the graph:
    // no graph key is live under it, so the bar offers its keys only.
    if let Some(picker) = crate::update::picker::hint_context(state) {
        return picker;
    }
    // A badge in the air first: while the gesture is running, the keys the
    // cursor's row would offer are not the ones that end it. First here
    // because it is first in `update::keys::key_event` too — `drag_key`
    // takes the key before any overlay or pane sees it.
    if state.drag.is_some() {
        return HintContext::Dragging;
    }
    // Then an overlay, for the same reason: it stands where the graph
    // stands and answers the keys the graph would (m5/42).
    if let Some(context) = overlay_context(state) {
        return context;
    }
    // The changes pane's own bar, but only while the pane has focus:
    // `space`, `d`, `⌃A` and `e` are read by `changes_pane_write_key`,
    // which refuses every one of them with the graph focused
    // (`update::detail::keys`, `DetailFocus::Graph => return false`). With
    // the pane open and focus back on the graph the row under the cursor is
    // what the bar is about, which the rest of this function works out.
    if state.on_changes_pane() && !state.graph_has_focus() {
        return HintContext::ChangesPane {
            unmerged: cursor_file_unmerged(state),
            live: live_c(state),
            tab: tab_word(state),
            over_overlay: false,
        };
    }
    if state.on_working_tree {
        return HintContext::WorkingTreeRow;
    }
    let cursor = rows
        .iter()
        .find(|entry| entry.line.paint == RowPaint::Highlighted);
    let row = match cursor {
        None => HintContext::PlainRow {
            verbs: RowVerbs::NONE,
        },
        Some(cursor) => cursor_row_context(state, cursor),
    };
    // Last, over whichever row context came out. A focused pane leaves four
    // of the graph screen's keys doing nothing (`AppState::graph_has_focus`)
    // and, with the diff focused, `⏎` as well — it has already opened
    // everything there is to open. Both conditions are the handlers' own
    // functions, read here rather than restated, which is the whole point:
    // the bar and the key cannot disagree if they read one answer
    // (m5 review Q2, m5/31).
    if state.graph_has_focus() {
        row
    } else {
        HintContext::PaneFocused {
            row: Box::new(row),
            opens: crate::update::detail::enter_opens(&state.detail),
            // `B`'s own guard (`update::place::blamable`): not while the
            // commit-graph offer takes the key.
            blamable: crate::update::place::blamable(state),
            // The files list's `space` on a directory row (m7-power-tools/20h).
            folds: crate::update::detail::space_folds(&state.detail),
            // The Detail tab: the graph is off screen (m8/11b decision 3).
            graph_hidden: crate::update::detail::tab_hint_rows(state) > 0,
        }
    }
}

/// Whether the changes pane's cursor is on an unmerged (conflicted) file —
/// the cursor's own file, not "any file in the tree": while one conflict
/// exists, every clean file's row would otherwise lose `space stage` and
/// `d discard` (m5-03 review, Minor 4). A section row, a directory row, or
/// no file list at all: not unmerged.
///
/// Matched by path bytes, the way `update::operate` matches the cursor's
/// file to its status entry — the file list is git's diff output and the
/// status entries are git's porcelain output, and only the path joins them.
fn cursor_file_unmerged(state: &AppState) -> bool {
    use std::ffi::OsStr;

    let Some(list) = state.detail.list.as_ref() else {
        return false;
    };
    let Some(entry) = state.detail.selected_entry() else {
        return false;
    };
    let Some(file) = usize::try_from(entry)
        .ok()
        .and_then(|index| list.files.get(index))
    else {
        return false;
    };
    state.working_tree.as_deref().is_some_and(|status| {
        status.entries.iter().any(|candidate| {
            candidate.unmerged
                && OsStr::as_encoded_bytes(candidate.path.as_os_str()) == file.path.as_slice()
        })
    })
}

/// The columns a fold weighs above the ordinary rank (ADR-0025), built once
/// and shared by the text and pixel tiers so a lane never folds differently
/// between them: the cursor's own column when its row is among `rows`,
/// `reserved` as given, and the selection's thread (`Ancestry::columns`).
pub(in crate::view) fn keep_for(
    rows: &[Row],
    cursor: Option<usize>,
    reserved: &BTreeSet<usize>,
    thread: Option<&Ancestry>,
) -> Keep {
    Keep {
        selected: cursor.and_then(|at| rows.get(at)).map(|row| row.lane),
        reserved: reserved.clone(),
        preferred: thread.map(Ancestry::columns).unwrap_or_default(),
    }
}
