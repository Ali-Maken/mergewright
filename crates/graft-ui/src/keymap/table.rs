//! The table: every binding of `docs/design/02-keymap.md` §2.2–§2.18 the
//! code answers, every key the code answers that the doc has no row for,
//! and the rows an M8 ticket is building (ids decided once, here).
//!
//! Rules (ticket m8/09): one action per verb (opposite verbs are two
//! actions, synonyms one); a key the doc repeats with the same verb in a
//! nested place is one row, in the outermost context holding it; a row's
//! first default is the one every surface spells.
//!
//! **Adding a row edits `crates/graft-ui/tests/keymap_ids.txt` in the same
//! commit. Renaming one breaks his config: keep the old id as an alias.**

use std::sync::LazyLock;

use super::{ActionId, Binding, ContextId, KeyChord, KeyCode};

mod commit_editor;
mod resolver;

/// Every action, grouped by context in `ContextId::ALL` order.
#[must_use]
pub fn defaults() -> &'static [Binding] {
    &DEFAULTS
}

const fn b(
    action: &'static str,
    context: ContextId,
    default: &'static [KeyChord],
    label: &'static str,
) -> Binding {
    Binding {
        action: ActionId(action),
        context,
        default,
        label,
        search: &[],
    }
}

const fn k(c: char) -> KeyChord {
    KeyChord::key(c)
}

const fn ctrl(c: char) -> KeyChord {
    KeyChord::ctrl(c)
}

const ENTER: KeyChord = KeyChord::plain(KeyCode::Enter);
const ESC: KeyChord = KeyChord::plain(KeyCode::Escape);
const TAB: KeyChord = KeyChord::plain(KeyCode::Tab);
const BACKTAB: KeyChord = KeyChord::plain(KeyCode::BackTab);
const SPACE: KeyChord = KeyChord::key(' ');
const UP: KeyChord = KeyChord::plain(KeyCode::Up);
const DOWN: KeyChord = KeyChord::plain(KeyCode::Down);
const LEFT: KeyChord = KeyChord::plain(KeyCode::Left);
const RIGHT: KeyChord = KeyChord::plain(KeyCode::Right);
const HOME: KeyChord = KeyChord::plain(KeyCode::Home);
const END: KeyChord = KeyChord::plain(KeyCode::End);
const PGUP: KeyChord = KeyChord::plain(KeyCode::PageUp);
const PGDN: KeyChord = KeyChord::plain(KeyCode::PageDown);
const BACKSPACE: KeyChord = KeyChord::plain(KeyCode::Backspace);
const DELETE: KeyChord = KeyChord::plain(KeyCode::Delete);
const PICKER_SWITCH: &str = "Switch to it; on your own branch, back to the graph";

use ContextId::{
    Banner, Blame, Changes, CommitEditor, Confirm, Credential, Diff, EmptyState, Finish, Global,
    Graph, History, Journal, Map, Palette, Picker, Preview, Resolver, Rewrite, Stash, Tour,
};

/// The rows in the man page's and `?`'s order: the blocks kept in their own
/// files sit where they would here.
static DEFAULTS: LazyLock<Vec<Binding>> = LazyLock::new(|| {
    [
        BEFORE_RESOLVER,
        resolver::ROWS,
        BANNER_TO_STASH,
        commit_editor::ROWS,
        FROM_JOURNAL,
    ]
    .iter()
    .flat_map(|rows| rows.iter())
    .map(|row| Binding { ..*row })
    .collect()
});

const BEFORE_RESOLVER: &[Binding] = &[
    // §2.2 Global.
    b(
        "app.palette",
        Global,
        &[ctrl('p'), k(':')],
        "Command palette",
    ),
    b("app.help", Global, &[k('?')], "Help for this place"),
    b("app.next_pane", Global, &[TAB], "Next pane"),
    b("app.prev_pane", Global, &[BACKTAB], "Previous pane"),
    b("app.back", Global, &[ESC], "Back out / close / cancel"),
    b("app.undo", Global, &[k('u'), ctrl('z')], "Undo"),
    // No key out of the box: with `suspend_on_ctrl_z` it is `⌃Z`, taken off
    // `app.undo` (`Keymap::with_overrides`, ticket m8/09c D2).
    b("app.suspend", Global, &[], "Suspend"),
    b("app.redo", Global, &[k('U')], "Redo"),
    b("app.refresh", Global, &[ctrl('r')], "Refresh"),
    b("app.quit", Global, &[k('q')], "Quit"),
    b(
        "app.quit_now",
        Global,
        &[KeyChord::QUIT_NOW],
        "Quit at once",
    ),
    b("app.down", Global, &[k('j'), DOWN], "Move down"),
    b("app.up", Global, &[k('k'), UP], "Move up"),
    b("app.top", Global, &[k('g'), HOME], "Jump to the top"),
    b("app.bottom", Global, &[k('G'), END], "Jump to the bottom"),
    b("app.page_down", Global, &[PGDN], "Page down"),
    b("app.page_up", Global, &[PGUP], "Page up"),
    b("app.find", Global, &[k('/')], "Find in the focused list"),
    b("app.next_match", Global, &[k('n')], "Next match"),
    b("app.prev_match", Global, &[k('N')], "Previous match"),
    b("app.open", Global, &[ENTER], "Open / drill in"),
    b("app.menu", Global, &[k('.')], "Context actions"),
    // §2.3 Graph (§2.4's `c`/`a` are these, reached through `scope`).
    b("graph.commit", Graph, &[k('c')], "Commit"),
    b("graph.amend", Graph, &[k('a')], "Amend"),
    b("graph.rewrite", Graph, &[k('R')], "Open the rewrite editor"),
    b(
        "graph.switch",
        Graph,
        &[k('C')],
        "Switch branch (branch picker)",
    ),
    b(
        "graph.take",
        Graph,
        &[k('o')],
        "Take this commit onto your branch (cherry-pick)",
    ),
    b(
        "graph.merge",
        Graph,
        &[k('m')],
        "Merge selected branch into current",
    ),
    b(
        "graph.rebase",
        Graph,
        &[k('r')],
        "Rebase current onto selection",
    ),
    b(
        "graph.branch",
        Graph,
        &[k('b')],
        "New branch from selected commit",
    ),
    b("graph.tag", Graph, &[k('t')], "New tag at selected commit"),
    b(
        "graph.compare",
        Graph,
        &[k('x')],
        "Compare against a marked base",
    ),
    b(
        "graph.lanes",
        Graph,
        &[k('+')],
        "Expand / collapse folded lanes",
    ),
    Binding {
        search: &["aside"],
        ..b("graph.stash", Graph, &[k('S')], "Stash…")
    },
    b("graph.fetch", Graph, &[k('f')], "Fetch"),
    b("graph.pull", Graph, &[k('p')], "Pull"),
    b("graph.push", Graph, &[k('P')], "Push"),
    b("graph.map", Graph, &[k('M')], "Map view"),
    b("graph.focus", Graph, &[k('F')], "Focus mode"),
    b("graph.overview", Graph, &[k('O')], "Overview strip"),
    b(
        "graph.fold",
        Graph,
        &[SPACE],
        "Open or close a merged branch",
    ),
    b("graph.first_parent", Graph, &[k('1')], "First-parent view"),
    b(
        "graph.hide_stash",
        Graph,
        &[k('H')],
        "Hide / show the stash",
    ),
    b("graph.copy_sha", Graph, &[k('y')], "Copy the commit's SHA"),
    b("graph.journal", Graph, &[k('J')], "Undo journal"),
    b(
        "graph.status",
        Graph,
        &[KeyChord::plain(KeyCode::Function(2))],
        "Status screen",
    ),
    b(
        "graph.renderer",
        Graph,
        &[KeyChord::plain(KeyCode::Function(3))],
        "Cycle the graph renderer",
    ),
    b(
        "graph.write_commit_graph",
        Graph,
        &[k('B')],
        "Write the commit-graph file (when offered)",
    ),
    b(
        "graph.decline_commit_graph",
        Graph,
        &[k('X')],
        "Decline the commit-graph offer",
    ),
    // §2.3a Map view.
    b("map.close", Map, &[k('M')], "Close the Map view"),
    b("map.seek", Map, &[ENTER], "Seek the graph to this branch"),
    // §2.3b Ghost Preview.
    // `y` is `confirm.yes`, borrowed (`ContextId::borrows`). These two are
    // the opening verb again; their chords follow `graph.merge`'s and
    // `graph.rebase`'s (ticket m8/09c, D3).
    b(
        "preview.confirm_merge",
        Preview,
        &[k('m')],
        "Confirm the merge (its opening key again)",
    ),
    b(
        "preview.confirm_rebase",
        Preview,
        &[k('r')],
        "Confirm the rebase (its opening key again)",
    ),
    b(
        "preview.rewrite",
        Preview,
        &[k('R')],
        "Open the rewrite editor on this rebase",
    ),
    b(
        "preview.choose",
        Preview,
        &[ENTER],
        "The choice under the cursor",
    ),
    b("preview.left", Preview, &[LEFT], "Choice to the left"),
    b("preview.right", Preview, &[RIGHT], "Choice to the right"),
    b("preview.cycle", Preview, &[TAB, BACKTAB], "Next choice"),
    // §2.3c Confirm box.
    // The one confirm key (§2.3b): previews, the finish list, the rewrite
    // editor and the init box borrow it (`ContextId::borrows`).
    b("confirm.yes", Confirm, &[k('y')], "Confirm"),
    b(
        "confirm.choose",
        Confirm,
        &[ENTER],
        "The choice under the cursor",
    ),
    b("confirm.left", Confirm, &[LEFT], "Choice to the left"),
    b("confirm.right", Confirm, &[RIGHT], "Choice to the right"),
    b("confirm.cycle", Confirm, &[TAB, BACKTAB], "Next choice"),
    // §2.4 Working tree / staging.
    b(
        "changes.stage",
        Changes,
        &[SPACE, k('s')],
        "Stage / unstage",
    ),
    b("changes.stage_all", Changes, &[ctrl('a')], "Stage all"),
    b("changes.discard", Changes, &[k('d')], "Discard"),
    b("changes.edit", Changes, &[k('e')], "Open in $EDITOR"),
    b(
        "changes.mergetool",
        Changes,
        &[k('M')],
        "Launch the mergetool on this file",
    ),
    // §2.5 Diff pane.
    b(
        "diff.ignore_whitespace",
        Diff,
        &[k('w')],
        "Ignore whitespace",
    ),
    b("diff.word_diff", Diff, &[k('W')], "Word-diff granularity"),
    b(
        "diff.side_by_side",
        Diff,
        &[k('v')],
        "Unified / side-by-side",
    ),
    b("diff.less_context", Diff, &[k('[')], "Fewer context lines"),
    b("diff.more_context", Diff, &[k(']')], "More context lines"),
    b("diff.shrink", Diff, &[k('<')], "Shrink the detail pane"),
    b("diff.grow", Diff, &[k('>')], "Grow the detail pane"),
    b("diff.load_all", Diff, &[k('L')], "Load the full file"),
    b(
        "diff.blame",
        Diff,
        &[k('B')],
        "Blame this file at this line",
    ),
    b("diff.file_history", Diff, &[k('F')], "This file's history"),
    b("diff.fold_dir", Diff, &[SPACE], "Fold / unfold a directory"),
];

const BANNER_TO_STASH: &[Binding] = &[
    // §2.6a Operation banner (`⏎` is `app.open`, `q` is `app.quit`).
    b("operation.continue", Banner, &[k('c')], "Continue"),
    b("operation.skip", Banner, &[k('s')], "Skip this commit"),
    b("operation.abort", Banner, &[k('a')], "Abort"),
    b(
        "operation.mergetool",
        Banner,
        &[k('M')],
        "Launch the mergetool on the first conflicted file",
    ),
    b(
        "operation.edit",
        Banner,
        &[k('e')],
        "Open the first conflicted file in $EDITOR",
    ),
    // §2.6b Finish summary.
    b(
        "finish.toggle_diff",
        Finish,
        &[ENTER, SPACE],
        "Show / hide the file's diff",
    ),
    b(
        "finish.edit_outside",
        Finish,
        &[ctrl('e')],
        "Open the message in $EDITOR",
    ),
    // §2.7 Rewrite editor.
    b("rewrite.pick", Rewrite, &[k('p')], "Pick"),
    b("rewrite.edit", Rewrite, &[k('e')], "Edit"),
    b("rewrite.squash", Rewrite, &[k('s')], "Squash"),
    b("rewrite.fixup", Rewrite, &[k('f')], "Fixup"),
    b("rewrite.drop", Rewrite, &[k('d')], "Drop"),
    b("rewrite.reword", Rewrite, &[k('r')], "Reword"),
    b(
        "rewrite.move_up",
        Rewrite,
        &[KeyChord::alt(KeyCode::Up)],
        "Move the row up",
    ),
    b(
        "rewrite.move_down",
        Rewrite,
        &[KeyChord::alt(KeyCode::Down)],
        "Move the row down",
    ),
    b(
        "rewrite.toggle_changes",
        Rewrite,
        &[ENTER],
        "Show / hide the row's changes",
    ),
    // §2.16 Branch picker (a text field: fixed).
    b("picker.switch", Picker, &[ENTER], PICKER_SWITCH),
    b("picker.show", Picker, &[TAB], "Show it on the graph"),
    // §2.8 Blame (`w` is `diff.ignore_whitespace`).
    b(
        "blame.parent",
        Blame,
        &[k('^')],
        "Blame the parent revision",
    ),
    b(
        "blame.line_history",
        Blame,
        &[k('L')],
        "History of these lines",
    ),
    b("blame.heatmap", Blame, &[SPACE], "Age heatmap"),
    b(
        "blame.ignore_revs",
        Blame,
        &[k('i')],
        "The repo's ignore-revisions file",
    ),
    // §2.9 Stash list.
    b("stash.save", Stash, &[k('s')], "Save a new stash"),
    b("stash.apply", Stash, &[ENTER], "Apply (preview)"),
    b("stash.pop", Stash, &[k('p')], "Pop (preview)"),
    b("stash.drop", Stash, &[k('d')], "Drop"),
    b(
        "stash.branch",
        Stash,
        &[k('b')],
        "New branch from this stash",
    ),
    b("stash.preview", Stash, &[SPACE], "Preview the diff"),
    b("stash.close", Stash, &[k('S')], "Close the stash list"),
];

const FROM_JOURNAL: &[Binding] = &[
    // §2.11 Undo journal browser.
    b(
        "journal.revert",
        Journal,
        &[ENTER],
        "Undo / redo this row, else put back to before it (preview)",
    ),
    b(
        "journal.branch",
        Journal,
        &[k('b')],
        "New branch at this row's commit",
    ),
    b(
        "journal.expand",
        Journal,
        &[SPACE],
        "Expand / collapse the row",
    ),
    b("journal.close", Journal, &[k('J')], "Close the journal"),
    // §2.17 History.
    b("history.next_change", History, &[k('n')], "Next change"),
    b("history.prev_change", History, &[k('N')], "Previous change"),
    // §2.12 Command palette (a text field: fixed).
    b("palette.run", Palette, &[ENTER], "Run the row's action"),
    // §2.13 First-run / empty state.
    b("empty_state.open", EmptyState, &[k('o')], "Open…"),
    b("empty_state.init", EmptyState, &[k('i')], "Init"),
    b("empty_state.clone", EmptyState, &[k('c')], "Clone…"),
    b("empty_state.tour", EmptyState, &[k('t')], "Practice tour"),
    // §2.18 Practice tour: `Esc` leaves, only when nothing else on screen
    // takes it (`graft::update::keys::escape::escape_key`, its last link).
    b("tour.leave", Tour, &[ESC], "Leave the practice tour"),
    // §2.14 Credential prompt (a text field: fixed).
    b("credential.send", Credential, &[ENTER], "Submit the answer"),
    b(
        "credential.clear",
        Credential,
        &[ctrl('u')],
        "Clear the field",
    ),
    b(
        "credential.edit_text",
        Credential,
        &[BACKSPACE, DELETE, LEFT, RIGHT, HOME, END],
        "Edit the typed text",
    ),
];
