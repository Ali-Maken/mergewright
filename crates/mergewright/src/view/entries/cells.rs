//! One history row's `GraphEntry`: the graph cells, subject, who and age,
//! the stop tag and each ref badge (drawn as a label, ease-pass G7).

use mergewright_graph::identity::BranchIdentity;
use mergewright_term::{sanitize_line, sanitize_path_bytes};
use mergewright_ui::graph_column::{GraphLine, RowPaint};
use mergewright_ui::graph_screen::{
    Badge, BadgeKind, GraphEntry, RowTag, Tracking, WhoCell, badges_that_fit, first_name,
    relative_date,
};

use crate::refs::{RefBadges, RefLabel, Upstream};
use crate::update::AppState;

/// One row's [`GraphEntry`]: the graph cells, the sanitised subject, and the
/// who and age columns (ease-pass G8) — the author resolved through
/// `mailmap` first, so two identities mapped to one person read alike. `compare_marked` is the pinned compare base's
/// row (`x`, m2-details-and-diffs/11).
pub(super) fn row_entry<'a>(
    state: &AppState,
    row: &'a mergewright_graph::row::Row,
    highlighted: bool,
    compare_marked: bool,
    mailmap: Option<&mergewright_git::mailmap::Mailmap>,
    refs: Option<&RefBadges>,
    now: i64,
) -> GraphEntry<'a> {
    let mut line = GraphLine::plain(row);
    // The app's node, the pixel tier's too (`view::node`, ease-pass 06r).
    line.node = super::super::node::Node::of(row, refs).kind();
    if highlighted {
        line.paint = RowPaint::Highlighted;
    }
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
    // hit-tests exactly as before — `mergewright_ui::graph_screen::badges_that_fit`
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
    let marker = super::super::radar::marker_for(
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
        who: Some(who(row, mailmap)),
        // `visible_rows` fills both, a run of rows at a time.
        branch: None,
        dim_message: false,
        when: relative_date(row.commit.date().seconds, now),
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

/// The who column (ease-pass G8): `you` on the user's own commits (his
/// `user.email`, through the mailmap), else the author's first name as the
/// mailmap resolves it.
pub(in crate::view) fn who(
    row: &mergewright_graph::row::Row,
    mailmap: Option<&mergewright_git::mailmap::Mailmap>,
) -> WhoCell {
    let author = &row.commit.author;
    if mailmap.is_some_and(|mailmap| mailmap.is_me(author)) {
        return WhoCell {
            name: sanitize_line("you"),
            own: true,
        };
    }
    let name = mailmap.map_or_else(|| author.name.clone(), |mailmap| mailmap.resolve(author).0);
    WhoCell {
        name: sanitize_line(first_name(&String::from_utf8_lossy(&name))),
        own: false,
    }
}

/// `HEAD · stopped here` on `HEAD`'s row while a rewrite is stopped on an
/// `edit` row (m7-power-tools/07b). Drawn from the stop, not held in
/// `row_tags`: every write clears those, and an amend at the stop must
/// leave the new `HEAD` still tagged.
fn stopped_here(state: &AppState, id: mergewright_core::oid::Oid) -> Option<RowTag> {
    let head = state
        .refs
        .as_deref()
        .and_then(crate::refs::RefBadges::head)?;
    (id == head && crate::update::commit_editor::at_edit_stop(state)).then_some(RowTag::StoppedHere)
}

/// One ref's badge, its names sanitised on the way into `ui`: ref names are
/// bytes a repository chose, and a branch called `ma<ESC>[31min` must reach
/// the terminal as a picture of the escape.
pub(in crate::view) fn badge_view(label: &RefLabel) -> Badge {
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
