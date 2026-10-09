//! The keys `c`'s refusal names (m8/10l, m8/11f): what acts where he is
//! left when `c` is refused while files are still conflicted.

use graft_ui::detail::WorkingTreeSection;
use graft_ui::keymap::{ContextId, KeyChord, KeyCode, Keymap};

use super::AppState;
use super::resume::{MERGETOOL_ROWS, edit_row, joined, row_at};

/// Where `c` was refused, as the resolve words need it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RefusedAt {
    pub ctx: ContextId,
    /// The changes pane's cursor row is in `WorkingTreeSection::Unmerged`.
    /// Read only at the changes pane; [`refused_at`] sets it `false`
    /// everywhere else.
    pub on_conflict_row: bool,
    /// The way to the banner from here (`refusal_way`, m8/20d).
    pub way: super::BannerWay,
}

/// `edge::context_of(state)` plus the changes cursor's section. The one
/// place the cursor is read for these words.
pub(crate) fn refused_at(state: &AppState) -> RefusedAt {
    let ctx = crate::update::edge::context_of(state);
    let on_conflict_row = ctx == ContextId::Changes
        && state.detail.selected_section() == Some(WorkingTreeSection::Unmerged);
    RefusedAt {
        ctx,
        on_conflict_row,
        way: crate::update::commit_editor::refusal_way(state),
    }
}

/// The keys `c`'s refusal offers, as said at `at` (m8/11f ruling M1, on
/// 10j ruling 1's place rule). Each clause is spelled from the row that
/// acts there; an unbound key or a key with no row drops its clause;
/// nothing left → `None`.
///
/// - Banner: `⏎ resolve  M mergetool  e edit` (`⏎` is `app.open`, which
///   opens the resolver on the first conflicted file).
/// - Changes pane on a CONFLICTS row: the same on that file, then `space
///   mark resolved` — last, since it stages the file as it stands.
/// - Changes pane on any other row: `space`, `M` and `e` would act on a
///   different file there, so the words name the way on instead: the `⇥`s
///   that reach the banner, `⇥ ⇥ banner` from the file list, `⇥ banner`
///   from the diff, the shorter way ([`super::way_keys`], m8/20b R4, 20d).
/// - Elsewhere: `M`, `e` where their rows act (the resolver); the graph
///   gets the reason alone.
pub(crate) fn resolve_keys(at: RefusedAt, keys: &Keymap) -> Option<String> {
    let place = at.ctx;
    if place == ContextId::Changes && !at.on_conflict_row {
        return super::way_keys(at.way, keys);
    }
    let resolve = matches!(place, ContextId::Banner | ContextId::Changes)
        .then(|| Keymap::shipped().acting(place, KeyChord::plain(KeyCode::Enter)))
        .flatten()
        .filter(|id| id.0 == "app.open")
        .and_then(|row| keys.key_verb(&[row], "resolve"));
    let tool =
        row_at(place, 'M', &MERGETOOL_ROWS).and_then(|row| keys.key_verb(&[row], "mergetool"));
    let edit = edit_row(place).and_then(|row| keys.key_verb(&[row], "edit"));
    let mark = row_at(place, ' ', &["changes.stage"])
        .and_then(|row| keys.key_verb(&[row], graft_ui::graph_screen::hints::MARK_RESOLVED));
    joined(joined(joined(resolve, tool), edit), mark)
}
