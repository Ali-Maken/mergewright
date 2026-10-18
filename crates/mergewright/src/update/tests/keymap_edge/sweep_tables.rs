//! The behavioural sweep's lists (m8/09d Major 3), written by hand from the
//! claiming code and checked by the sweep: a listed row that acts fails.
//! Paths are under `crates/mergewright/src/update/` unless named; lines as read
//! on 2026-10-07.

use mergewright_ui::keymap::ContextId;
use mergewright_ui::text_input::TextField;

use super::AppState;
use super::sweep_swallowed::{R_CREDENTIAL, R_PICKER};
use crate::update::credential::PromptField;

/// `(place, id, why)`: answered there, but the fixture gives it nothing to
/// change.
#[rustfmt::skip]
pub(super) const NO_EFFECT_IN_FIXTURE: &[(ContextId, &str, &str)] = &[
    (ContextId::Graph, "graph.lanes", "no folded lanes; + only flips lanes_expanded keys.rs:343"),
    (ContextId::Graph, "graph.hide_stash", "no stash in refs: Stashes -> None view_change.rs:50"),
    (ContextId::Graph, "graph.write_commit_graph", "no commit-graph offer: commit_graph.rs:85"),
    (ContextId::Graph, "graph.decline_commit_graph", "no commit-graph offer: commit_graph.rs:85"),
    (ContextId::Graph, "diff.ignore_whitespace", "pane closed: detail/keys.rs:172 passes, graph has no w"),
    (ContextId::Graph, "diff.word_diff", "pane closed: detail/keys.rs:172 passes, graph has no W"),
    (ContextId::Graph, "diff.side_by_side", "pane closed: detail/keys.rs:172 passes, graph has no v"),
    (ContextId::Graph, "diff.less_context", "pane closed: detail/keys.rs:172 passes, graph has no ["),
    (ContextId::Graph, "diff.more_context", "pane closed: detail/keys.rs:172 passes, graph has no ]"),
    (ContextId::Graph, "diff.shrink", "pane closed: detail/keys.rs:105 needs detail.shown"),
    (ContextId::Graph, "diff.grow", "pane closed: detail/keys.rs:105 needs detail.shown"),
    (ContextId::Graph, "diff.load_all", "pane closed: detail/keys.rs:172 passes, graph has no L"),
    (ContextId::Graph, "app.next_pane", "pane closed: detail/keys.rs:158 needs detail.shown"),
    (ContextId::Graph, "app.prev_pane", "pane closed: detail/keys.rs:162 needs detail.shown"),
    (ContextId::Graph, "app.back", "nothing running/open: keys/escape.rs:9-60"),
    (ContextId::Graph, "app.next_match", "no search: search.rs:22-23"),
    (ContextId::Graph, "app.prev_match", "no search: search.rs:22-23"),
    (ContextId::Stash, "app.next_pane", "list passes Tab stash.rs:199; pane closed detail/keys.rs:158"),
    (ContextId::Stash, "app.prev_pane", "list passes ⇧⇥ stash.rs:199; pane closed detail/keys.rs:162"),
    (ContextId::Journal, "app.next_match", "no ✗ lost row to step to: journal/keys.rs:97,127-139"),
    (ContextId::Journal, "app.prev_match", "no ✗ lost row to step to: journal/keys.rs:98,127-139"),
    (ContextId::Journal, "app.next_pane", "J passes Tab journal/keys.rs:78; pane closed detail/keys.rs:158"),
    (ContextId::Journal, "app.prev_pane", "J passes ⇧⇥ journal/keys.rs:78; pane closed detail/keys.rs:162"),
    (ContextId::Preview, "preview.confirm_merge", "opened by r (preview/tests.rs:175); m confirms m only :87"),
    (ContextId::Preview, "preview.rewrite", "plan keeps no drops: preview/keys.rs:51 None, :57"),
    (ContextId::Preview, "preview.left", "cursor already 0: preview/keys.rs:60 saturating_sub"),
    (ContextId::Confirm, "confirm.left", "cursor rests on 0: confirm.rs:178 left() clamps"),
    (ContextId::Changes, "changes.mergetool", "no conflicted row under cursor: detail/keys.rs:240-241"),
    (ContextId::Changes, "diff.word_diff", "W flips word_tint, colour only (rule 5) detail/keys.rs:393"),
    (ContextId::Changes, "diff.side_by_side", "no diff loaded in fixture; v flips mode detail/keys.rs:377"),
    (ContextId::Changes, "graph.lanes", "no folded lanes; + only flips lanes_expanded keys.rs:343"),
    (ContextId::Changes, "graph.hide_stash", "no stash in refs: view_change.rs:50"),
    (ContextId::Changes, "graph.copy_sha", "on the working-tree row: keys/actions.rs:46"),
    (ContextId::Changes, "graph.compare", "on the working-tree row: keys/actions.rs:64"),
    (ContextId::Changes, "graph.decline_commit_graph", "no commit-graph offer: commit_graph.rs:85"),
    (ContextId::Changes, "app.next_match", "no search: search.rs:22-23"),
    (ContextId::Changes, "app.prev_match", "no search: search.rs:22-23"),
    (ContextId::Diff, "diff.word_diff", "W flips word_tint, colour only (rule 5) detail/keys.rs:393"),
    (ContextId::Diff, "diff.blame", "files cursor on dir row src: no file, detail/cursor.rs:66"),
    (ContextId::Diff, "graph.lanes", "no folded lanes; + only flips lanes_expanded keys.rs:343"),
    (ContextId::Diff, "graph.hide_stash", "no stash in refs: view_change.rs:50"),
    (ContextId::Diff, "graph.decline_commit_graph", "no commit-graph offer: commit_graph.rs:85"),
    (ContextId::Diff, "app.next_match", "no search: search.rs:22-23"),
    (ContextId::Diff, "app.prev_match", "no search: search.rs:22-23"),
    (ContextId::Resolver, "app.up", "on the first unmerged file: resolver.rs:368 step_file"),
    (ContextId::Resolver, "app.top", "document fits the window: resolver/scroll.rs:141-145"),
    (ContextId::Resolver, "app.bottom", "document fits the window: resolver/scroll.rs:146-150"),
    (ContextId::Resolver, "app.page_down", "document fits the window: resolver/scroll.rs:139,157"),
    (ContextId::Resolver, "app.page_up", "top already 0: resolver/scroll.rs:140,157"),
    (ContextId::Resolver, "resolver.review", "no merge in the fixture: resolver/review.rs:64 review_pressed"),
    (ContextId::Finish, "app.down", "cursor on the last row, git combined (sweep_boxes.rs:39); finish/keys.rs:49"),
    (ContextId::Finish, "app.page_down", "cursor on the last row, git combined (sweep_boxes.rs:39); finish/keys.rs:53"),
    (ContextId::Finish, "finish.toggle_diff", "⏎ on the git-combined row waits for ease-pass/12b's look: finish/keys.rs:127 row_path"),
    (ContextId::Rewrite, "rewrite.move_up", "cursor on the top row: rewrite_editor/keys.rs:163"),
    (ContextId::Rewrite, "app.up", "cursor on the top row: rewrite_editor/keys.rs:175"),
    (ContextId::Blame, "blame.heatmap", "flips heatmap, colour only (rule 5): place/keys.rs:59"),
    (ContextId::History, "app.up", "newest row selected: place/history_keys.rs:103"),
    (ContextId::History, "app.page_up", "newest row selected: place/history_keys.rs:103"),
    (ContextId::History, "diff.side_by_side", "flips side_by_side; no file text loaded? history_keys.rs:45"),
    (ContextId::Menu, "changes.stage_all", "menu closes, falls to graph: no changes pane context_menu.rs:157"),
    (ContextId::Menu, "changes.discard", "menu closes, falls to graph: no changes pane context_menu.rs:157"),
    (ContextId::Menu, "changes.edit", "menu closes, falls to graph: no changes pane context_menu.rs:157"),
    (ContextId::Menu, "diff.ignore_whitespace", "menu closes; pane closed detail/keys.rs:172"),
    (ContextId::Menu, "diff.word_diff", "menu closes; pane closed detail/keys.rs:172"),
    (ContextId::Menu, "diff.side_by_side", "menu closes; pane closed detail/keys.rs:172"),
    (ContextId::Menu, "diff.less_context", "menu closes; pane closed detail/keys.rs:172"),
    (ContextId::Menu, "diff.more_context", "menu closes; pane closed detail/keys.rs:172"),
    (ContextId::Menu, "diff.shrink", "menu closes; pane closed detail/keys.rs:105"),
    (ContextId::Menu, "diff.grow", "menu closes; pane closed detail/keys.rs:105"),
    (ContextId::Menu, "diff.load_all", "menu closes; pane closed detail/keys.rs:172"),
    (ContextId::Menu, "diff.blame", "menu closes; pane closed: place.rs:299 claims_b false"),
    (ContextId::Menu, "graph.lanes", "menu closes; no folded lanes keys.rs:343"),
    (ContextId::Menu, "graph.hide_stash", "menu closes; no stash in refs view_change.rs:50"),
    (ContextId::Menu, "graph.decline_commit_graph", "menu closes; no offer commit_graph.rs:85"),
    (ContextId::Menu, "app.next_pane", "menu closes; pane closed detail/keys.rs:158"),
    (ContextId::Menu, "app.prev_pane", "menu closes; pane closed detail/keys.rs:162"),
    (ContextId::Menu, "app.next_match", "menu closes; no search search.rs:22-23"),
    (ContextId::Menu, "app.prev_match", "menu closes; no search search.rs:22-23"),
];

/// `(place, id, default chord as spelled, the row that answers it there,
/// why)`. The sweep checks each holds: that row's swept chord does what
/// this default did.
#[rustfmt::skip]
pub(super) const SHADOWED: &[(ContextId, &str, &str, &str, &str)] = &[
    (ContextId::Graph, "diff.blame", "B", "graph.write_commit_graph", "own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Graph, "diff.file_history", "F", "graph.focus", "own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Graph, "diff.fold_dir", "space", "graph.fold", "own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Map, "app.open", "⏎", "map.seek", "Map own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Stash, "app.open", "⏎", "stash.apply", "Stash own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Journal, "app.open", "⏎", "journal.revert", "J own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Preview, "app.next_pane", "⇥", "preview.cycle", "Preview own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Preview, "app.prev_pane", "⇧⇥", "preview.cycle", "Preview own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Preview, "app.open", "⏎", "preview.choose", "Preview own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Confirm, "app.next_pane", "⇥", "confirm.cycle", "Confirm own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Confirm, "app.prev_pane", "⇧⇥", "confirm.cycle", "Confirm own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Confirm, "app.open", "⏎", "confirm.choose", "Confirm own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Changes, "diff.fold_dir", "space", "changes.stage", "Changes own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Changes, "graph.map", "M", "changes.mergetool", "Changes own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Changes, "graph.focus", "F", "diff.file_history", "scope Diff before Graph: mergewright-ui keymap.rs:149"),
    (ContextId::Changes, "graph.fold", "space", "changes.stage", "Changes own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Changes, "graph.write_commit_graph", "B", "diff.blame", "scope Diff before Graph: mergewright-ui keymap.rs:149"),
    (ContextId::Diff, "graph.focus", "F", "diff.file_history", "Diff own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Diff, "graph.fold", "space", "diff.fold_dir", "Diff own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Diff, "graph.write_commit_graph", "B", "diff.blame", "Diff own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Resolver, "app.next_match", "n", "resolver.next_block", "Resolver own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Resolver, "app.prev_match", "N", "resolver.prev_block", "Resolver own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Resolver, "app.open", "⏎", "resolver.mark_done", "Resolver own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Finish, "app.open", "⏎", "finish.toggle_diff", "Finish own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Rewrite, "app.open", "⏎", "rewrite.toggle_changes", "Rewrite own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Blame, "diff.load_all", "L", "blame.line_history", "Blame own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Blame, "diff.fold_dir", "space", "blame.heatmap", "Blame own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::History, "app.next_match", "n", "history.next_change", "History own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::History, "app.prev_match", "N", "history.prev_change", "History own row first: mergewright-ui keymap/resolve.rs:357"),
    (ContextId::Menu, "diff.fold_dir", "space", "changes.stage", "scope Changes before Diff: mergewright-ui keymap.rs:154"),
    (ContextId::Menu, "graph.map", "M", "changes.mergetool", "scope Changes before Graph: mergewright-ui keymap.rs:154"),
    (ContextId::Menu, "graph.focus", "F", "diff.file_history", "scope Diff before Graph: mergewright-ui keymap.rs:154"),
    (ContextId::Menu, "graph.fold", "space", "changes.stage", "scope Changes before Graph: mergewright-ui keymap.rs:154"),
    (ContextId::Menu, "graph.write_commit_graph", "B", "diff.blame", "scope Diff before Graph: mergewright-ui keymap.rs:154"),
];

/// Why one chord of a row is idle in the sweep while its other chord acts
/// (m8/09h R1′).
pub(super) enum IdleWhy {
    /// Acts in its handler; the sweep's fixture leaves it nothing to
    /// change. `at`: the handler arm's `file:line`. `given` puts the place
    /// where the action has something to change; `then` reads the effect
    /// off state.
    ActsGivenState {
        at: &'static str,
        given: fn(&mut AppState),
        then: fn(before: &AppState, after: &AppState) -> bool,
    },
    /// Neither acts nor types. `at`: where the chord falls idle. Gap row
    /// `M8/one-chord-idle` (broken).
    Defect { at: &'static str },
}

impl IdleWhy {
    /// The `file:line` the row cites.
    pub(super) fn at(&self) -> &'static str {
        match self {
            Self::ActsGivenState { at, .. } | Self::Defect { at } => at,
        }
    }
}

use IdleWhy::{ActsGivenState, Defect};

/// The open picker, its selection on the second of at least two rows.
fn picker_on_row_1(state: &mut AppState) {
    let picker = state.picker.as_mut().expect("the picker is open");
    assert!(picker.rows().len() >= 2, "the picker has two rows");
    picker.selected = 1;
}

fn picker_row_1_to_0(before: &AppState, after: &AppState) -> bool {
    let selected = |s: &AppState| s.picker.as_ref().map(|p| p.selected);
    selected(before) == Some(1) && selected(after) == Some(0)
}

/// `abc`, caret at `at`.
fn abc_at(at: usize) -> TextField {
    let mut field = TextField::new("abc");
    for _ in at..3 {
        field.left();
    }
    field
}

fn subject_abc(state: &mut AppState, at: usize) {
    state
        .commit_editor
        .as_mut()
        .expect("the editor is open")
        .subject = abc_at(at);
}

fn file_cursor_on_1(state: &mut AppState) {
    state
        .commit_editor
        .as_mut()
        .expect("the editor is open")
        .file_cursor = 1;
}

fn file_cursor(state: &AppState) -> Option<usize> {
    Some(state.commit_editor.as_ref()?.file_cursor)
}

fn subject(state: &AppState) -> Option<(String, usize)> {
    let subject = &state.commit_editor.as_ref()?.subject;
    Some((subject.text().to_owned(), subject.cursor()))
}

fn credential_abc(state: &mut AppState, at: usize) {
    state.credential.as_mut().expect("the prompt is open").field = PromptField::Open(abc_at(at));
}

fn credential(state: &AppState) -> Option<(String, usize)> {
    match &state.credential.as_ref()?.field {
        PromptField::Open(field) => Some((field.text().to_owned(), field.cursor())),
        PromptField::Secret(_) => None,
    }
}

type Read = fn(&AppState) -> Option<(String, usize)>;

/// `read` gives `from` before the press and `to` after it.
fn went(
    read: Read,
    before: &AppState,
    after: &AppState,
    from: (&str, usize),
    to: (&str, usize),
) -> bool {
    let owned = |(text, caret): (&str, usize)| Some((text.to_owned(), caret));
    read(before) == owned(from) && read(after) == owned(to)
}

/// `(place, id, chord, why)`: one chord of a row does nothing there while
/// its other chord acts — the per-row lists cannot say it.
#[rustfmt::skip]
pub(super) const ONE_CHORD_IDLE: &[(ContextId, &str, &str, IdleWhy)] = &[
    (ContextId::Picker, "app.undo", "⌃Z", Defect { at: "picker.rs:158 swallows ⌃Z; u is typed (:150)" }),
    (ContextId::Picker, "app.up", "↑", ActsGivenState {
        at: "picker.rs:141", given: picker_on_row_1, then: picker_row_1_to_0 }),
    (ContextId::Picker, "app.top", "home", Defect { at: "picker.rs:158 swallows home; g is typed (:150)" }),
    (ContextId::Picker, "app.bottom", "end", Defect { at: "picker.rs:158 swallows end; G is typed (:150)" }),
    (ContextId::CommitEditor, "app.undo", "⌃Z", Defect { at: "commit_editor/keys.rs:154 swallows ⌃Z; u is typed" }),
    (ContextId::CommitEditor, "app.up", "↑", ActsGivenState {
        at: "commit_editor/keys.rs subject_key: ↑ moves the file cursor up", given: file_cursor_on_1,
        then: |b, a| file_cursor(b) == Some(1) && file_cursor(a) == Some(0) }),
    (ContextId::CommitEditor, "app.top", "home", ActsGivenState {
        at: "commit_editor/keys.rs:273", given: |s| subject_abc(s, 2),
        then: |b, a| went(subject, b, a, ("abc", 2), ("abc", 0)) }),
    (ContextId::CommitEditor, "app.bottom", "end", ActsGivenState {
        at: "commit_editor/keys.rs:274", given: |s| subject_abc(s, 0),
        then: |b, a| went(subject, b, a, ("abc", 0), ("abc", 3)) }),
    (ContextId::Credential, "credential.edit_text", "delete", ActsGivenState {
        at: "credential.rs:378", given: |s| credential_abc(s, 1),
        then: |b, a| went(credential, b, a, ("abc", 1), ("ac", 1)) }),
    (ContextId::Credential, "credential.edit_text", "←", ActsGivenState {
        at: "credential.rs:379", given: |s| credential_abc(s, 3),
        then: |b, a| went(credential, b, a, ("abc", 3), ("abc", 2)) }),
    (ContextId::Credential, "credential.edit_text", "→", ActsGivenState {
        at: "credential.rs:380", given: |s| credential_abc(s, 1),
        then: |b, a| went(credential, b, a, ("abc", 1), ("abc", 2)) }),
    (ContextId::Credential, "credential.edit_text", "home", ActsGivenState {
        at: "credential.rs:381", given: |s| credential_abc(s, 3),
        then: |b, a| went(credential, b, a, ("abc", 3), ("abc", 0)) }),
    (ContextId::Credential, "credential.edit_text", "end", ActsGivenState {
        at: "credential.rs:382", given: |s| credential_abc(s, 1),
        then: |b, a| went(credential, b, a, ("abc", 1), ("abc", 3)) }),
    (ContextId::Credential, "app.undo", "⌃Z", Defect { at: "credential.rs:326 swallows ⌃Z; u is typed" }),
    (ContextId::Credential, "app.down", "↓", Defect { at: "credential.rs:385 swallows ↓; j is typed" }),
    (ContextId::Credential, "app.up", "↑", Defect { at: "credential.rs:385 swallows ↑; k is typed" }),
    (ContextId::Credential, "app.top", "home", ActsGivenState {
        at: "credential.rs:381", given: |s| credential_abc(s, 3),
        then: |b, a| went(credential, b, a, ("abc", 3), ("abc", 0)) }),
    (ContextId::Credential, "app.bottom", "end", ActsGivenState {
        at: "credential.rs:382", given: |s| credential_abc(s, 1),
        then: |b, a| went(credential, b, a, ("abc", 1), ("abc", 3)) }),
    (ContextId::Picker, "app.palette", "⌃P", Defect { at: R_PICKER }),
    (ContextId::Credential, "app.palette", "⌃P", Defect { at: R_CREDENTIAL }),
    (ContextId::Menu, "changes.stage", "s", Defect { at: "context_menu.rs:162 closes, falls through; the graph has no s" }),
];
