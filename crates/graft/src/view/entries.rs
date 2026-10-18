//! Building one frame's `GraphEntry` rows: the selection's thread, the
//! uncommitted-changes row, and each history row's cells and badges.

use graft_git::status::WorkingTreeStatus;
use graft_graph::identity::BranchIdentity;
use graft_graph::row::Row;
use graft_graph::{Ancestry, Kin};
use graft_term::{sanitize_line, sanitize_path_bytes};
use graft_ui::graph_column::{GraphLine, NodeKind, RowPaint};
use graft_ui::graph_screen::{
    AuthorChip, Badge, BadgeKind, GraphEntry, RowTag, SHA_WIDTH, Tracking, badges_that_fit,
    initials, relative_date, short_sha,
};

use crate::history::History;
use crate::refs::{RefBadges, RefLabel, Upstream};
use crate::update::AppState;

/// The selection's thread through the window the walk last sent, keyed by
/// history row number. Worked out from rows already on hand — one pass over
/// the window, never a layout or walk call (DESIGN.md §5.2).
pub(super) struct Thread {
    first: usize,
    pub(super) ancestry: Ancestry,
}

impl Thread {
    pub(super) fn lit(&self, index: usize) -> &std::collections::BTreeSet<usize> {
        self.ancestry.lit(index.saturating_sub(self.first))
    }

    pub(super) fn kin(&self, index: usize) -> Kin {
        match index.checked_sub(self.first) {
            Some(at) => self.ancestry.kin(at),
            None => Kin::Unrelated,
        }
    }
}

pub(super) fn thread(history: &History) -> Option<Thread> {
    let window = history.window.as_ref()?;
    let selected = history.selected()?.commit.id;
    Some(Thread {
        first: window.first,
        ancestry: Ancestry::of(&window.rows, selected)?,
    })
}

/// The rows the viewport shows: the uncommitted-changes row, when the
/// working tree is dirty and `HEAD` is the topmost visible commit, then the
/// history rows the walk last sent.
///
/// Under a selection every lane draws bold on its thread and dim off it; in
/// Focus mode the rows off the thread fade as a whole.
/// What a badge in the air does to the row at absolute history row `index`,
/// or `None` when this row has nothing to do with the drag
/// (m5-history-and-ghost-preview/10, §2.15; reworked by 35 after the user
/// reported that a drag showed nothing until he let go).
///
/// Applied last, over Focus mode's fade: while a badge is in the air the
/// question on screen is where it would land, and that answer wins.
///
/// The row the badge is **over** gets its own [`RowPaint::DropTarget`]
/// rather than the cursor's `Highlighted`. The cursor has not moved and
/// comes back the moment the badge is dropped, so both rows are on screen
/// and both have to be legible. Until 35 they were the same paint and the
/// drop target *replaced* the cursor's highlight — so the whole of a drag,
/// on screen, was one faint background raise moving from one row to
/// another.
///
/// The row the badge came **from** fades, because that is what has
/// happened: the badge is not there any more, it is in the user's hand.
/// This is the mark that lands on the button *down*, before any motion,
/// and it is the answer to "nothing indicates I have picked anything up" —
/// until 35 the press changed nothing on screen but the footer.
fn drag_paint(state: &AppState, index: usize) -> Option<RowPaint> {
    let drag = state.drag.as_ref()?;
    if drag.over == Some(index) {
        Some(RowPaint::DropTarget)
    } else if drag.from_row == index {
        Some(RowPaint::Dimmed)
    } else {
        None
    }
}

pub(super) fn visible_rows<'a>(
    state: &'a AppState,
    history: &'a History,
    thread: Option<&Thread>,
    now: i64,
) -> Vec<GraphEntry<'a>> {
    let mailmap = state.repository.as_ref().map(|facts| &facts.mailmap);
    // The topmost visible row, independent of viewport height: whether the
    // uncommitted row attaches depends only on what is at the very top, not
    // on how many rows are shown below it.
    // `HEAD`'s row, but only while it is the topmost one: off screen, the
    // uncommitted row draws as a lone `◌` with no lane under it.
    let top_row = state
        .uncommitted_row_attached()
        .then(|| history.visible(1).first())
        .flatten();
    let uncommitted = uncommitted_shown(state)
        .then_some(state.working_tree.as_deref())
        .flatten();
    // `graph_rows` already leaves the pinned virtual row out, so this draws
    // exactly the rows every scroller keeps the cursor among (48). The one
    // exception is a one-row pane with the cursor on the virtual row: that
    // row is the cursor's, so it takes the only row (Decided 4).
    let height = if state.uncommitted_row_pinned() || uncommitted.is_none() {
        state.graph_rows()
    } else {
        0
    };
    // While the cursor rests on the virtual row (`AppState::on_working_tree`),
    // the highlight moves onto it and off the history cursor's row —
    // `move_from_graph` never moves `History::cursor` when it sets the flag,
    // so the two would otherwise both read as "the cursor's row" at once.
    uncommitted
        .map(|status| {
            let mut entry = uncommitted_entry(top_row, status);
            if state.on_working_tree {
                entry.line.paint = RowPaint::Highlighted;
            }
            entry
        })
        .into_iter()
        .chain(
            history
                .visible(height)
                .iter()
                .zip(history.top..)
                .map(|(row, index)| {
                    let mut entry = row_entry(
                        state,
                        row,
                        index == history.cursor && !state.on_working_tree,
                        state.compare_base.is_some_and(|base| base.row == index),
                        mailmap,
                        state.refs.as_deref(),
                        now,
                    );
                    entry.fold = super::rows::fold_tail(state, history, row, index);
                    if let Some(thread) = thread {
                        entry.line.thread = Some(thread.lit(index).clone());
                    }
                    // The Explain panel's fade (`m2-details-and-diffs/17`) is
                    // independent of Focus mode and always wins while open:
                    // every row outside `Explanation::highlight` fades, on
                    // its own id, not the selection's ancestry.
                    if state.detail.explain.is_some() {
                        if !state.detail.explain_highlight.contains(&row.commit.id) {
                            entry.line.paint = RowPaint::Dimmed;
                        }
                    } else if let Some(thread) = thread
                        && state.focus
                        && thread.kin(index) == Kin::Unrelated
                    {
                        entry.line.paint = RowPaint::Dimmed;
                    }
                    // The drag's two rows, last: over Focus mode's fade.
                    if let Some(paint) = drag_paint(state, index) {
                        entry.line.paint = paint;
                    }
                    entry
                }),
        )
        .collect()
}

/// Does the uncommitted-changes row take the graph pane's top screen row
/// this frame? [`visible_rows`]'s own check, exposed so a mouse row
/// (`update.rs`) can tell a click or a hover on it apart from a click on the
/// real history row it displaces — the same facts, asked the same way, off
/// the same state.
///
/// Pinned ([`AppState::uncommitted_row_pinned`]), or — in a one-row pane,
/// where it is not pinned — while the cursor rests on it, since that one row
/// draws the cursor's row (m7-power-tools/48, Decided 4).
#[must_use]
pub(crate) fn uncommitted_shown(state: &AppState) -> bool {
    state.uncommitted_row_pinned() || (state.uncommitted_row_shown() && state.on_working_tree)
}

/// The uncommitted-changes row's [`GraphEntry`]: `head_row` — `HEAD`'s row,
/// or `None` on an unborn branch — gives it the same lane and lane
/// occupancy `HEAD`'s row has, so the connector line reads as continuing
/// down into it. Author, date and SHA stay blank (DESIGN.md §1.1's mockup):
/// there is no commit behind this row to show them for.
fn uncommitted_entry<'a>(head_row: Option<&Row>, status: &WorkingTreeStatus) -> GraphEntry<'a> {
    let mut line = GraphLine::owned(uncommitted_row(head_row));
    line.node = NodeKind::Uncommitted;
    GraphEntry {
        badges: Vec::new(),
        line,
        subject: sanitize_line(&uncommitted_subject(status)),
        author: AuthorChip {
            initials: sanitize_line(""),
            identity: BranchIdentity::of_name(b""),
        },
        when: String::new(),
        sha: String::new(),
        // The compare base is always a commit row (`toggle_compare`'s `oid`
        // comes from `History::selected`/`row_at`); the working-tree row can
        // never be marked.
        compare_marked: false,
        // Uncommitted changes have no commit id for `view::radar` to key on,
        // nor for a row tag.
        marker: None,
        tag: None,
        fold: None,
    }
}

/// A `Row` with no commit behind it, drawn only to give the uncommitted
/// row's node the same connector-line geometry `GraphColumn` gives a real
/// one. Never reaches a ref comparison or a selection target: `view` builds
/// it fresh every frame and nothing else holds it.
pub(super) fn uncommitted_row(head_row: Option<&Row>) -> Row {
    let blank = graft_core::commit::Identity {
        name: Vec::new(),
        email: Vec::new(),
        when: graft_core::commit::Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    // Never a real object id — all zero bytes, at whatever length this
    // repository's ids are — so it can never collide with a commit a ref or
    // the selection could name.
    let id_len = head_row.map_or(graft_core::oid::Oid::SHA1_BYTES, |row| {
        row.commit.id.as_bytes().len()
    });
    let id = graft_core::oid::Oid::from_bytes(&vec![0u8; id_len])
        .unwrap_or_else(|_| unreachable!("id_len is always a hash length Oid::from_bytes accepts"));
    Row {
        commit: graft_core::commit::Commit {
            id,
            parents: Vec::new(),
            author: blank.clone(),
            committer: blank,
            message: Vec::new(),
        },
        drawn_parents: Vec::new(),
        lane: head_row.map_or(0, |row| row.lane),
        incoming: Vec::new(),
        outgoing: head_row.map_or_else(Vec::new, |row| vec![(row.lane, row.commit.id)]),
        lanes: head_row.map_or_else(Vec::new, |row| row.lanes.clone()),
        marker: None,
    }
}

/// The subject text the uncommitted row shows: which of unmerged, staged,
/// unstaged and untracked are non-zero, and how many of each (DESIGN.md
/// §1.1's mockup: `"Uncommitted changes (3 staged)"`).
///
/// Built from the status itself rather than [`WorkingTreeCounts`], because
/// that struct has three buckets and the working tree has four states: a
/// porcelain v2 `u` record is in **none** of staged, unstaged and untracked
/// (`WorkingTreeStatus::counts` counts only those three). The row draws
/// whenever the status has any entry at all, so a tree holding nothing but
/// conflicts used to draw `Uncommitted changes ()` — empty parentheses at
/// the one moment the working tree most needs describing (m5/21).
///
/// Unmerged leads: during a conflict it is what the user has to act on, and
/// it never reads as resolved — `(2 unmerged, 1 staged)` says both halves.
fn uncommitted_subject(status: &WorkingTreeStatus) -> String {
    let counts = status.counts();
    let unmerged = status.entries.iter().filter(|entry| entry.unmerged).count();
    let mut parts = Vec::new();
    if unmerged > 0 {
        parts.push(format!("{unmerged} unmerged"));
    }
    if counts.staged > 0 {
        parts.push(format!("{} staged", counts.staged));
    }
    if counts.unstaged > 0 {
        parts.push(format!("{} unstaged", counts.unstaged));
    }
    if counts.untracked > 0 {
        parts.push(format!("{} untracked", counts.untracked));
    }
    format!("Uncommitted changes ({})", parts.join(", "))
}

/// One row's [`GraphEntry`]: the graph cells, the sanitised subject, and the
/// author/date/SHA columns m1-the-graph/10 adds — the author's identity
/// resolved through `mailmap` first, so two identities mapped to one person
/// share a chip and a colour. `compare_marked` is the pinned compare base's
/// row (`x`, m2-details-and-diffs/11).
fn row_entry<'a>(
    state: &AppState,
    row: &'a graft_graph::row::Row,
    highlighted: bool,
    compare_marked: bool,
    mailmap: Option<&graft_git::mailmap::Mailmap>,
    refs: Option<&RefBadges>,
    now: i64,
) -> GraphEntry<'a> {
    let mut line = GraphLine::plain(row);
    // The app's node, the pixel tier's too (`view::node`, ease-pass 06r).
    line.node = super::node::Node::of(row, refs).kind();
    if highlighted {
        line.paint = RowPaint::Highlighted;
    }
    let (name, email) = match mailmap {
        Some(mailmap) => mailmap.resolve(&row.commit.author),
        None => (
            row.commit.author.name.clone(),
            row.commit.author.email.clone(),
        ),
    };
    // The pinned compare base reads `x base` first on its row (m7-14); it
    // is not a ref, so `update::geometry::badge_at` skips it by the same
    // `compare_marked` test.
    let base = compare_marked.then(|| Badge {
        name: sanitize_line(""),
        kind: BadgeKind::CompareBase,
    });
    // Only the badges a row this wide can draw (m8/12c): a commit can carry
    // 16 000 refs, and every one built here was sanitised, measured and
    // packed again each frame. Cut in drawing order, so the row draws and
    // hit-tests exactly as before — `graft_ui::graph_screen::badges_that_fit`
    // says why the cells cannot change.
    let badges = base
        .into_iter()
        .chain(
            refs.into_iter()
                .flat_map(|refs| refs.on(row.commit.id).iter().map(badge_view)),
        )
        .take(badges_that_fit(state.size.columns))
        .collect();
    let subject = String::from_utf8_lossy(row.commit.subject());
    let marker = super::radar::marker_for(
        state,
        row.commit.id,
        crate::history::parents::of(row, state.refs.as_deref()),
        &subject,
    );
    if marker.is_some() {
        // Sticky: once a finding has been seen this session, the marker
        // column keeps drawing even after the row that earned it scrolls
        // off-screen and `History::rows`'s window evicts it
        // (`AppState::radar_column_seen`, `view::rows::graph_view`).
        state.radar_column_seen.set(true);
    }
    GraphEntry {
        badges,
        line,
        subject: sanitize_line(&subject),
        author: AuthorChip {
            initials: sanitize_line(&initials(&String::from_utf8_lossy(&name))),
            identity: BranchIdentity::of_name(&email),
        },
        when: relative_date(row.commit.date().seconds, now),
        sha: short_sha(&row.commit.id.to_string(), usize::from(SHA_WIDTH)),
        compare_marked,
        marker,
        // `HEAD · stopped here` at a rewrite's `edit` stop (07b), else a
        // commit a put-in just changed (m7-power-tools/09).
        tag: stopped_here(state, row.commit.id)
            .or_else(|| state.row_tags.on(row.commit.id).cloned()),
        // The fold's tail needs the row's number: `visible_rows` sets it.
        fold: None,
    }
}

/// `HEAD · stopped here` on `HEAD`'s row while a rewrite is stopped on an
/// `edit` row (m7-power-tools/07b). Drawn from the stop, not held in
/// `row_tags`: every write clears those, and an amend at the stop must
/// leave the new `HEAD` still tagged.
fn stopped_here(state: &AppState, id: graft_core::oid::Oid) -> Option<RowTag> {
    let head = state
        .refs
        .as_deref()
        .and_then(crate::refs::RefBadges::head)?;
    (id == head && crate::update::commit_editor::at_edit_stop(state)).then_some(RowTag::StoppedHere)
}

/// One ref's badge, its names sanitised on the way into `ui`: ref names are
/// bytes a repository chose, and a branch called `ma<ESC>[31min` must reach
/// the terminal as a picture of the escape.
pub(super) fn badge_view(label: &RefLabel) -> Badge {
    match label {
        RefLabel::DetachedHead => Badge {
            name: sanitize_line("HEAD"),
            kind: BadgeKind::Head,
        },
        RefLabel::Branch {
            name,
            current,
            upstream,
        } => Badge {
            name: sanitize_path_bytes(name),
            kind: BadgeKind::Branch {
                identity: BranchIdentity::of_name(name),
                current: *current,
                tracking: match upstream {
                    Upstream::None => Tracking::Untracked,
                    Upstream::InSync { remote } => Tracking::InSync {
                        remote: sanitize_path_bytes(remote),
                    },
                    Upstream::Apart(counts) => Tracking::Apart {
                        ahead: counts.ahead,
                        behind: counts.behind,
                    },
                },
            },
        },
        RefLabel::Remote(name) => Badge {
            name: sanitize_path_bytes(name),
            kind: BadgeKind::Remote {
                identity: BranchIdentity::of_name(name),
            },
        },
        RefLabel::Tag(name) => Badge {
            name: sanitize_path_bytes(name),
            kind: BadgeKind::Tag,
        },
        RefLabel::Stash(index) => Badge {
            // `stash@{n}`, git's own name, so the badge and the `S` overlay
            // call the same entry the same thing
            // (m3-working-tree-undo-and-stash/06).
            name: sanitize_line(&format!("stash@{{{index}}}")),
            kind: BadgeKind::Stash,
        },
    }
}
