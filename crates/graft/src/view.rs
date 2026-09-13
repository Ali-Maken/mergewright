//! The end of the loop: `view(state)` (ARCHITECTURE.md).
//!
//! One pure function from what Graft knows to a frame. It reads and draws; it
//! never changes anything, so what is on screen is always a direct function of
//! the state.
//!
//! This is also the boundary the sanitiser guards. Everything here that came
//! out of a repository — a path, a branch name, a failure git wrote — crosses
//! into `ui` as [`SanitizedText`], which only `term` can construct, so a branch
//! called `ma<ESC>[31min` reaches the terminal as the picture of an escape and
//! never as the escape (DESIGN.md §8.2).

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_git::status::WorkingTreeCounts;
use graft_git::version::SystemGit;
use graft_graph::identity::BranchIdentity;
use graft_graph::row::Row;
use graft_graph::{Ancestry, Kin};
use graft_term::{SanitizedText, sanitize, sanitize_line, sanitize_path, sanitize_path_bytes};
use graft_ui::graph_column::{GraphLine, NodeKind, RowPaint};
use graft_ui::graph_screen::{
    AuthorChip, Badge, BadgeKind, Compaction, Filters, GraphEntry, GraphScreen, GraphView,
    GraphicsStatus, OrderNotice, SHA_WIDTH, SearchLine, Tracking, WalkView, initials,
    max_lanes_for, relative_date, short_sha, tooltip_date,
};
use graft_ui::status_screen::{
    HeadView, HistoryState, HistoryView, RepositoryView, StatusScreen, StatusView,
};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::history::{History, Phase, Promotion, WalkFacts};
use crate::refs::{RefBadges, RefLabel, Upstream};
use crate::repository::{Head, RepositoryFacts};
use crate::session::Session;
use crate::update::{AppState, GitCheck, Screen, Status};

/// How much of a commit id a detached `HEAD` shows. Long enough to be unique
/// in any repository a person works in, short enough to read back.
const ABBREVIATED_ID: usize = 12;

pub mod strips;

/// The tier's name, as `graph.renderer` and the status screen spell it.
fn tier_name(tier: RenderingTier) -> &'static str {
    match tier {
        RenderingTier::Pixel => "pixel",
        RenderingTier::Unicode => "unicode",
        RenderingTier::Ascii => "ascii",
    }
}

/// Draw the current state.
pub fn view(session: &Session, state: &AppState, frame: &mut Frame) {
    view_with_graphics(session, state, &GraphicsStatus::Ok, frame);
}

/// The tier the graph draws at: `F3`'s override, else the session's.
#[must_use]
pub fn tier_in_force(session: &Session, state: &AppState) -> RenderingTier {
    state.renderer_override.unwrap_or(session.rendering.tier)
}

/// Whether this frame's graph is drawn in pixels: the graph screen, at the
/// pixel tier, over the Kitty protocol or iTerm2 inline images.
///
/// `graph.renderer = pixel` on a terminal that never acked any image
/// protocol attempts Kitty anyway (DESIGN.md §4.2: config wins, and silence
/// is not proof it cannot draw); `F3` drops to Unicode if nothing shows.
#[must_use]
pub fn draws_pixels(session: &Session, state: &AppState) -> bool {
    state.screen == Screen::Graph
        && tier_in_force(session, state) == RenderingTier::Pixel
        && matches!(
            session.capabilities.pixel_protocol,
            Some(PixelProtocol::Kitty | PixelProtocol::Iterm2) | None
        )
}

/// [`view`], saying `graphics` in the header when the graph is drawn in
/// pixels; returns the rectangle those pixels go in, or `None` when this
/// frame draws no pixels.
pub fn view_with_graphics(
    session: &Session,
    state: &AppState,
    graphics: &GraphicsStatus,
    frame: &mut Frame,
) -> Option<Rect> {
    let area = frame.area();
    if area.is_empty() {
        // Nothing to draw into. Not an error — a terminal can legitimately be
        // this size for a moment (ADR-0012) — so there is nothing to report.
        return None;
    }
    let pixels = draws_pixels(session, state);

    match state.screen {
        Screen::Graph => {
            let mut screen = graph_view(state);
            screen.icons = session.icons;
            screen.compaction.max_lanes = max_lanes_for(state.size.columns, session.max_lanes);
            let tier = tier_in_force(session, state);
            screen.ascii = tier == RenderingTier::Ascii;
            // Only below the terminal's own tier: `F3` clears the override
            // when it cycles back to the top (m1-the-graph/33).
            screen.notice = footer_notice(state, tier);
            screen.banner = banner_lines(state, tier);
            screen.pixel = pixels;
            screen.graphics = pixels.then(|| graphics.clone());
            let widget = GraphScreen::new(&screen, &session.theme);
            let graph = pixels.then(|| widget.graph_area(area));
            frame.render_widget(widget, area);
            graph
        }
        Screen::Status => {
            let screen = status_view(session, state);
            frame.render_widget(StatusScreen::new(&screen, &session.theme), area);
            None
        }
    }
}

/// The footer's notice: `F3`'s renderer line while an override is on.
/// `None` with no override, so the key hints show.
fn footer_notice(state: &AppState, tier: RenderingTier) -> Option<SanitizedText> {
    // Only below the terminal's own tier: `F3` clears the override when it
    // cycles back to the top (m1-the-graph/33).
    state
        .renderer_override
        .map(|_| sanitize_line(&format!("renderer: {} (F3 cycles)", tier_name(tier))))
}

/// The harness banner as drawn: its title with ` · <tier>` appended for the
/// tier in force (the harness cannot know it; the probe decides), then its
/// lines. Empty without a harness banner, so the shipped binary draws
/// nothing extra.
fn banner_lines(state: &AppState, tier: RenderingTier) -> Vec<SanitizedText> {
    let Some(banner) = &state.harness.banner else {
        return Vec::new();
    };
    std::iter::once(format!("{} · {}", banner.title, tier_name(tier)))
        .chain(banner.lines.iter().cloned())
        .map(|line| sanitize_line(&line))
        .collect()
}

/// Everything the graph screen draws: the rows in the viewport, the cursor
/// among them, and where the walk is.
///
/// The rows are borrowed from the window the walk last sent; only the
/// subjects, which came out of the repository, are copied — sanitised.
#[must_use]
pub fn graph_view(state: &AppState) -> GraphView<'_> {
    let location = state
        .repository
        .as_ref()
        .map(|facts| sanitize_path(&facts.path));
    let problem = match &state.status {
        Status::Ready => None,
        Status::Failed(message) => Some(sanitize_line(message)),
    };
    let Some(history) = &state.history else {
        return GraphView {
            location,
            problem,
            walk: WalkView::Opening,
            rows: Vec::new(),
            icons: graft_ui::graph_screen::Icons::default(),
            compaction: Compaction::at(crate::config::DEFAULT_MAX_LANES),
            filters: Filters::default(),
            tooltip: None,
            order: None,
            ascii: false,
            pixel: false,
            graphics: None,
            notice: None,
            search: search_line(state),
            banner: Vec::new(),
        };
    };

    let walk = walk_view(history);
    let thread = thread(history);
    let rows = visible_rows(state, history, thread.as_ref());

    GraphView {
        location,
        problem,
        walk,
        rows,
        icons: graft_ui::graph_screen::Icons::default(),
        compaction: compaction(state, history, thread.as_ref()),
        filters: Filters {
            first_parent: history.view.follow == graft_graph::Follow::FirstParent,
            folded: history.view.inclusion.folded.len(),
            stash_hidden: !history.view.inclusion.hidden_stashes.is_empty(),
            focused: state.focus,
        },
        tooltip: tooltip_view(state, history),
        order: order_notice(history),
        ascii: false,
        pixel: false,
        graphics: None,
        notice: None,
        search: search_line(state),
        banner: Vec::new(),
    }
}

/// The header's words for where the walk is.
fn walk_view(history: &History) -> WalkView {
    match &history.phase {
        Phase::Streaming if history.jump.is_some() => WalkView::Jumping {
            laid_out: history.rows,
        },
        Phase::Streaming => WalkView::Reading {
            laid_out: history.rows,
        },
        Phase::Complete => WalkView::Complete { rows: history.rows },
        Phase::Cancelled => WalkView::Cancelled {
            laid_out: history.rows,
        },
        Phase::Failed(reason) => WalkView::Failed(sanitize_line(reason)),
    }
}

/// The footer's search line: the prompt while it is open, else the search's
/// state. The query came from the keyboard and the line is drawn, so it is
/// sanitised like repository text.
fn search_line(state: &AppState) -> Option<SearchLine> {
    if let Some(typed) = &state.search_prompt {
        return Some(SearchLine::Prompt(sanitize_line(typed)));
    }
    let search = state.search.as_ref()?;
    Some(SearchLine::Status {
        text: sanitize_line(&search.line()),
        failed: matches!(search.state, crate::search::SearchState::Failed(_)),
    })
}

/// What the header says about the order the rows are in (ADR-0008): nothing
/// once they are in the order asked for; while commit-date order stands in
/// for a topological one, that it is being sorted — or, once the sort is
/// stopped or has failed, that the order stays as it is.
pub(crate) fn order_notice(history: &History) -> Option<OrderNotice> {
    let provisional = history.walk.as_ref().is_some_and(WalkFacts::is_provisional);
    if !provisional {
        return None;
    }
    Some(match history.promotion {
        Some(Promotion::Kept) => OrderNotice::Provisional,
        Some(Promotion::Sorting | Promotion::Swapping { .. }) | None => OrderNotice::Sorting,
    })
}

/// A clock reading, not repository text — the moment "now" is drawn at, for
/// every relative date on screen. `duration_since` only fails when the
/// system clock reads before 1970, in which case every commit is at least
/// that old and `relative_date` reads it the same as any far-past commit.
fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// An identity's mailmap-resolved name and email, or the identity's own
/// when no mailmap is loaded yet.
fn resolve_identity(
    mailmap: Option<&graft_git::mailmap::Mailmap>,
    identity: &graft_core::commit::Identity,
) -> (Vec<u8>, Vec<u8>) {
    match mailmap {
        Some(mailmap) => mailmap.resolve(identity),
        None => (identity.name.clone(), identity.email.clone()),
    }
}

/// `Name <email>`, sanitised: repository text once mailmap has resolved it.
fn identity_line(name: &[u8], email: &[u8]) -> SanitizedText {
    sanitize(&format!(
        "{} <{}>",
        String::from_utf8_lossy(name),
        String::from_utf8_lossy(email)
    ))
}

/// The commit tooltip, if one is open: a pinned tooltip (`Enter`) wins over
/// a hover, so hovering another row can never overwrite it
/// (m1-the-graph/34). Both read through the same lookup, so their content
/// is identical apart from `pinned`. `None` when the row it names has not
/// (or no longer) arrived.
fn tooltip_view(state: &AppState, history: &History) -> Option<graft_ui::graph_screen::Tooltip> {
    let (row_index, pinned) = match state.pinned_tooltip {
        Some(row) => (row, true),
        None => (state.tooltip?, false),
    };
    let row = history.row_at(row_index)?;
    let commit = &row.commit;
    let mailmap = state.repository.as_ref().map(|facts| &facts.mailmap);
    let (author_name, author_email) = resolve_identity(mailmap, &commit.author);
    let (committer_name, committer_email) = resolve_identity(mailmap, &commit.committer);
    let committer = (committer_name != author_name || committer_email != author_email)
        .then(|| identity_line(&committer_name, &committer_email));
    Some(graft_ui::graph_screen::Tooltip {
        hash: commit.id.to_string(),
        author: identity_line(&author_name, &author_email),
        committer,
        date: tooltip_date(
            now_seconds(),
            commit.author.when.seconds,
            commit.author.when.offset_seconds,
        ),
        parents: commit
            .parents
            .iter()
            .map(|id| short_sha(&id.to_string(), usize::from(SHA_WIDTH)))
            .collect(),
        subject: sanitize_line(&String::from_utf8_lossy(commit.subject())),
        body: sanitize(&String::from_utf8_lossy(commit_body(commit))),
        pinned,
    })
}

/// The message after its subject line, with the separating newline dropped.
/// Empty for a one-line message.
fn commit_body(commit: &graft_core::commit::Commit) -> &[u8] {
    let rest = &commit.message[commit.subject().len()..];
    rest.strip_prefix(b"\n").unwrap_or(rest)
}

/// How the graph folds its lanes: the session's limit is filled in by
/// [`view`], the expansion flag is the user's, and the reserved integration
/// lanes (`1..=n`, after lane 0) are pinned so a fold never hides a lane a
/// branch was promised. So are the columns the selection's thread runs in
/// on screen: a fold never hides where the selected commit came from.
fn compaction(state: &AppState, history: &History, thread: Option<&Thread>) -> Compaction {
    let integration = history
        .walk
        .as_ref()
        .map_or(0, |walk| walk.integration.len());
    Compaction {
        max_lanes: crate::config::DEFAULT_MAX_LANES,
        expanded: state.lanes_expanded,
        pinned: (1..=integration)
            .chain(thread.into_iter().flat_map(|thread| {
                (history.top..history.top + state.graph_rows())
                    .flat_map(|index| thread.lit(index).iter().copied())
            }))
            .collect(),
    }
}

/// The selection's thread through the window the walk last sent, keyed by
/// history row number. Worked out from rows already on hand — one pass over
/// the window, never a layout or walk call (DESIGN.md §5.2).
struct Thread {
    first: usize,
    ancestry: Ancestry,
}

impl Thread {
    fn lit(&self, index: usize) -> &std::collections::BTreeSet<usize> {
        self.ancestry.lit(index.saturating_sub(self.first))
    }

    fn kin(&self, index: usize) -> Kin {
        match index.checked_sub(self.first) {
            Some(at) => self.ancestry.kin(at),
            None => Kin::Unrelated,
        }
    }
}

fn thread(history: &History) -> Option<Thread> {
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
fn visible_rows<'a>(
    state: &'a AppState,
    history: &'a History,
    thread: Option<&Thread>,
) -> Vec<GraphEntry<'a>> {
    let mailmap = state.repository.as_ref().map(|facts| &facts.mailmap);
    let now = now_seconds();
    // The topmost visible row, independent of viewport height: whether the
    // uncommitted row attaches depends only on what is at the very top, not
    // on how many rows are shown below it.
    let top_row = history.visible(1).first();
    let dirty = state.working_tree.filter(|counts| !counts.is_clean());
    let uncommitted = dirty.filter(|_| head_is_topmost(state.refs.as_ref(), top_row));
    // The virtual row takes one of the viewport's rows when it draws:
    // `History::visible` knows nothing about it.
    let height = if uncommitted.is_some() {
        state.graph_rows().saturating_sub(1)
    } else {
        state.graph_rows()
    };
    uncommitted
        .map(|counts| uncommitted_entry(top_row, counts))
        .into_iter()
        .chain(
            history
                .visible(height)
                .iter()
                .zip(history.top..)
                .map(|(row, index)| {
                    let mut entry = row_entry(
                        row,
                        index == history.cursor,
                        mailmap,
                        state.refs.as_ref(),
                        now,
                    );
                    if let Some(thread) = thread {
                        entry.line.thread = Some(thread.lit(index).clone());
                        if state.focus && thread.kin(index) == Kin::Unrelated {
                            entry.line.paint = RowPaint::Dimmed;
                        }
                    }
                    entry
                }),
        )
        .collect()
}

/// Does the uncommitted-changes row draw at the top of the viewport this
/// frame? [`visible_rows`]'s own check, exposed so a mouse row (`update.rs`)
/// can tell a click or a hover on it apart from a click on the real history
/// row it displaces — the same working-tree and `HEAD` facts, asked the same
/// way, off the same state.
#[must_use]
pub(crate) fn uncommitted_shown(state: &AppState) -> bool {
    let Some(history) = &state.history else {
        return false;
    };
    let top_row = history.visible(1).first();
    let dirty = state.working_tree.is_some_and(|counts| !counts.is_clean());
    dirty && head_is_topmost(state.refs.as_ref(), top_row)
}

/// Is `HEAD` the topmost visible row? The uncommitted row attaches only
/// then (PROMPT.md §1.3): otherwise it would draw beside a commit it is not
/// actually on top of, once the viewport has scrolled.
///
/// `refs` unresolved (the job has not answered yet) reads as "no": showing
/// the row and then possibly retracting it once the answer arrives would
/// flicker, and a clean-looking first frame is the safer guess.
fn head_is_topmost(refs: Option<&RefBadges>, top: Option<&Row>) -> bool {
    let Some(refs) = refs else {
        return false;
    };
    match refs.head() {
        // An unborn branch has no commit for the row to sit on top of, so
        // it is trivially the topmost thing there is.
        None => true,
        Some(head_id) => top.is_some_and(|row| row.commit.id == head_id),
    }
}

/// The uncommitted-changes row's [`GraphEntry`]: `head_row` — `HEAD`'s row,
/// or `None` on an unborn branch — gives it the same lane and lane
/// occupancy `HEAD`'s row has, so the connector line reads as continuing
/// down into it. Author, date and SHA stay blank (DESIGN.md §1.1's mockup):
/// there is no commit behind this row to show them for.
fn uncommitted_entry<'a>(head_row: Option<&Row>, counts: WorkingTreeCounts) -> GraphEntry<'a> {
    let mut line = GraphLine::owned(uncommitted_row(head_row));
    line.node = NodeKind::Uncommitted;
    GraphEntry {
        badges: Vec::new(),
        line,
        subject: sanitize_line(&uncommitted_subject(counts)),
        author: AuthorChip {
            initials: sanitize_line(""),
            identity: BranchIdentity::of_name(b""),
        },
        when: String::new(),
        sha: String::new(),
    }
}

/// A `Row` with no commit behind it, drawn only to give the uncommitted
/// row's node the same connector-line geometry `GraphColumn` gives a real
/// one. Never reaches a ref comparison or a selection target: `view` builds
/// it fresh every frame and nothing else holds it.
fn uncommitted_row(head_row: Option<&Row>) -> Row {
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
        lane: head_row.map_or(0, |row| row.lane),
        incoming: Vec::new(),
        outgoing: head_row.map_or_else(Vec::new, |row| vec![(row.lane, row.commit.id)]),
        lanes: head_row.map_or_else(Vec::new, |row| row.lanes.clone()),
        marker: None,
    }
}

/// The subject text the uncommitted row shows: which of staged, unstaged
/// and untracked are non-zero, and how many of each (DESIGN.md §1.1's
/// mockup: `"Uncommitted changes (3 staged)"`).
fn uncommitted_subject(counts: WorkingTreeCounts) -> String {
    let mut parts = Vec::new();
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
/// share a chip and a colour.
fn row_entry<'a>(
    row: &'a graft_graph::row::Row,
    highlighted: bool,
    mailmap: Option<&graft_git::mailmap::Mailmap>,
    refs: Option<&RefBadges>,
    now: i64,
) -> GraphEntry<'a> {
    let mut line = GraphLine::plain(row);
    if let Some(refs) = refs {
        // HEAD wins over the stash and over the parent-derived shapes: it is
        // the one node a person looks for first.
        if refs.head() == Some(row.commit.id) {
            line.node = NodeKind::Head;
        } else if refs.stash() == Some(row.commit.id) {
            line.node = NodeKind::Stash;
        }
    }
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
    let badges = refs.map_or_else(Vec::new, |refs| {
        refs.on(row.commit.id).iter().map(badge_view).collect()
    });
    GraphEntry {
        badges,
        line,
        subject: sanitize_line(&String::from_utf8_lossy(row.commit.subject())),
        author: AuthorChip {
            initials: sanitize_line(&initials(&String::from_utf8_lossy(&name))),
            identity: BranchIdentity::of_name(&email),
        },
        when: relative_date(now, row.commit.date().seconds),
        sha: short_sha(&row.commit.id.to_string(), usize::from(SHA_WIDTH)),
    }
}

/// One ref's badge, its names sanitised on the way into `ui`: ref names are
/// bytes a repository chose, and a branch called `ma<ESC>[31min` must reach
/// the terminal as a picture of the escape.
fn badge_view(label: &RefLabel) -> Badge {
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
        RefLabel::Stash => Badge {
            name: sanitize_line("stash"),
            kind: BadgeKind::Stash,
        },
    }
}

/// Everything the status screen draws, assembled from the two things `app`
/// knows: what was settled at startup, and what has happened since.
///
/// Separate from [`view`] so it can be asserted on directly — a view model is
/// data, and comparing data beats reading cells out of a buffer.
#[must_use]
pub fn status_view(session: &Session, state: &AppState) -> StatusView {
    StatusView {
        repository: state.repository.as_ref().map(repository_view),
        rendering: session.rendering,
        notes: notes(session, state),
        problem: match &state.status {
            Status::Ready => None,
            Status::Failed(message) => Some(sanitize_line(message)),
        },
        history: state.history.as_ref().map(history_view),
    }
}

/// How far the walk has got. A failure's text came out of git, so it is
/// sanitised like any other.
fn history_view(history: &History) -> HistoryView {
    HistoryView {
        rows: history.rows,
        state: match &history.phase {
            Phase::Streaming => HistoryState::Streaming,
            Phase::Complete => HistoryState::Complete,
            Phase::Cancelled => HistoryState::Cancelled,
            Phase::Failed(reason) => HistoryState::Failed(sanitize_line(reason)),
        },
    }
}

/// The repository, with every string it carries sanitised on the way.
fn repository_view(facts: &RepositoryFacts) -> RepositoryView {
    RepositoryView {
        path: sanitize_path(&facts.path),
        bare: facts.bare,
        head: match &facts.head {
            // Ref names are the bytes git stored, which are only mostly UTF-8.
            Head::Branch(name) => HeadView::Branch(sanitize_path_bytes(name)),
            Head::Unborn(name) => HeadView::Unborn(sanitize_path_bytes(name)),
            // Hex out of `Oid`, so there is nothing to escape — sanitised
            // anyway, because "this one is safe" is how the one that isn't
            // gets through.
            Head::Detached(id) => {
                let hex = id.to_string();
                HeadView::Detached(sanitize_line(&hex[..ABBREVIATED_ID.min(hex.len())]))
            }
            Head::Unreadable(why) => HeadView::Unreadable(sanitize_line(why)),
        },
    }
}

/// The advisories, in the order they matter: the machine's git first, then the
/// config file.
///
/// A git that is new enough produces no note at all. There is no permanent
/// "git ok" row, because a field that is always there is a field nobody reads
/// (CLAUDE.md).
fn notes(session: &Session, state: &AppState) -> Vec<SanitizedText> {
    let mut notes = Vec::new();

    match &state.system_git {
        // Still running, or fine. Either way there is nothing to say.
        None | Some(GitCheck::Ran(SystemGit::Supported(_))) => {}
        Some(GitCheck::Ran(SystemGit::TooOld { found, minimum })) => notes.push(sanitize_line(
            &format!("git {found} is older than the {minimum} Graft needs"),
        )),
        Some(GitCheck::Unavailable(why)) => {
            notes.push(sanitize_line(&format!("git could not be run: {why}")));
        }
    }

    notes.extend(session.notes.iter().cloned());
    notes
}

#[cfg(test)]
pub(crate) mod tests {
    use graft_core::oid::Oid;
    use graft_git::version::GitVersion;
    use graft_ui::status_screen::TierSource;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::session::for_tests::session;
    use crate::update::Size;
    use crate::update::{Banner, Harness};

    /// Draw `state` into a terminal of exactly this size and return what the
    /// cells ended up holding.
    fn draw(state: &AppState, columns: u16, rows: u16) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
        terminal
            .draw(|frame| view(&session(), state, frame))
            .expect("TestBackend cannot fail to draw");
        terminal.backend().to_string()
    }

    fn open(head: Head) -> AppState {
        AppState {
            screen: Screen::Status,
            repo_path: std::path::PathBuf::from("/code/api"),
            repository: Some(RepositoryFacts {
                path: std::path::PathBuf::from("/code/api"),
                bare: false,
                head,
                mailmap: graft_git::mailmap::Mailmap::default(),
            }),
            ..AppState::default()
        }
    }

    #[test]
    fn an_open_repository_reaches_the_screen() {
        let screen = draw(&open(Head::Branch(b"main".to_vec())), 60, 12);

        assert!(screen.contains("/code/api"), "{screen}");
        assert!(screen.contains("main"), "{screen}");
        assert!(screen.contains("unicode"), "{screen}");
    }

    /// A history of `count` commits, newest first, already laid out and
    /// sent as one window.
    pub(crate) fn history_of(count: u8, cursor: usize) -> History {
        use graft_core::commit::{Commit, Identity, Timestamp};
        use graft_graph::layout::layout;
        use graft_graph::seeds::Seeds;

        let oid = |byte: u8| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes");
        let commits: Vec<Commit> = (1..=count)
            .rev()
            .map(|id| {
                let who = Identity {
                    name: b"Test".to_vec(),
                    email: b"test@example.invalid".to_vec(),
                    when: Timestamp {
                        seconds: i64::from(id),
                        offset_seconds: 0,
                    },
                };
                Commit {
                    id: oid(id),
                    parents: if id == 1 { vec![] } else { vec![oid(id - 1)] },
                    author: who.clone(),
                    committer: who,
                    message: format!("Commit {id}\x1b[31m").into_bytes(),
                }
            })
            .collect();
        let rows = layout(&commits, Seeds::default());
        History {
            window: Some(crate::history::Window {
                request: 0,
                first: 0,
                laid_out: rows.len(),
                done: true,
                rows: rows.into(),
                promoted: false,
            }),
            rows: usize::from(count),
            phase: Phase::Complete,
            cursor,
            reanchor: None,
            ..History::started()
        }
    }

    #[test]
    fn graft_opens_on_the_graph_with_the_cursor_row_highlighted() {
        let state = AppState {
            history: Some(history_of(5, 1)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let view = graph_view(&state);
        let screen = draw(&state, 60, 5);

        assert_eq!(view.rows.len(), 3, "rows minus the chrome");
        assert_eq!(view.walk, WalkView::Complete { rows: 5 });
        assert_eq!(view.rows[1].line.paint, RowPaint::Highlighted);
        assert_eq!(view.rows[0].line.paint, RowPaint::Normal);
        assert!(screen.contains("Commit 5"), "{screen}");
        assert!(screen.contains("q quit"), "{screen}");
        assert!(
            !screen.contains('\u{1b}'),
            "a subject's escape reached the cells"
        );
    }

    #[test]
    fn a_harness_banner_sits_under_the_header_with_the_tier_and_costs_rows() {
        let mut state = AppState {
            history: Some(history_of(5, 1)),
            size: Size {
                columns: 80,
                rows: 6,
            },
            harness: Harness {
                banner: Some(Banner {
                    title: String::from("gallery 6/23 · foxtrot-merge · ] next"),
                    lines: vec![String::from("shows: a lane jump")],
                }),
                ..Harness::default()
            },
            ..AppState::default()
        };
        assert_eq!(state.graph_rows(), 2, "header, two banner rows, footer");
        let screen = draw(&state, 80, 6);
        let lines: Vec<&str> = screen.lines().collect();
        assert!(lines[0].contains(" graft"), "{screen}");
        assert!(
            lines[1].contains("gallery 6/23 · foxtrot-merge · ] next · unicode"),
            "the title carries the tier in force: {screen}"
        );
        assert!(lines[2].contains("shows: a lane jump"), "{screen}");
        assert!(
            lines[5].contains("q quit"),
            "the hints keep the footer: {screen}"
        );

        state.renderer_override = Some(RenderingTier::Ascii);
        let screen = draw(&state, 80, 6);
        let lines: Vec<&str> = screen.lines().collect();
        assert!(lines[1].contains("] next · ascii "), "{screen}");
        assert!(lines[5].contains("renderer: ascii (F3 cycles)"), "{screen}");
    }

    #[test]
    fn the_tier_cycle_key_forces_ascii_and_says_so_in_the_footer() {
        let state = AppState {
            history: Some(history_of(5, 1)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            renderer_override: Some(RenderingTier::Ascii),
            ..AppState::default()
        };

        let view = graph_view(&state);
        assert!(
            !view.ascii,
            "graph_view alone does not know the session's tier"
        );

        let screen = draw(&state, 60, 5);
        assert!(screen.contains("renderer: ascii (F3 cycles)"), "{screen}");
        assert!(
            !screen.contains('╭'),
            "the graph column drew Unicode: {screen}"
        );
    }

    #[test]
    fn every_graph_renderer_config_value_draws_as_it_says() {
        use crate::config::{Config, Graph, Loaded, RendererChoice};
        use crate::session::for_tests::capabilities;

        let state = AppState {
            history: Some(history_of(5, 1)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        // `Auto` and `Unicode` both draw Unicode here: the detected terminal
        // is a Unicode one, and `Unicode` asks for exactly that. `Pixel` on
        // this silent terminal attempts Kitty (m1-the-graph/24d): the column
        // is left blank for images, so neither glyph set shows.
        for (renderer, ascii, pixel) in [
            (RendererChoice::Auto, false, false),
            (RendererChoice::Unicode, false, false),
            (RendererChoice::Pixel, false, true),
            (RendererChoice::Ascii, true, false),
        ] {
            let loaded = Loaded {
                config: Config {
                    graph: Graph {
                        renderer,
                        ..Graph::default()
                    },
                    ..Config::default()
                },
                ..Loaded::default()
            };
            let session = Session::settle(&loaded, capabilities());
            let mut terminal =
                Terminal::new(TestBackend::new(60, 5)).expect("TestBackend reports its size");
            terminal
                .draw(|frame| view(&session, &state, frame))
                .expect("TestBackend cannot fail to draw");
            let screen = terminal.backend().to_string();

            insta::assert_snapshot!(format!("renderer-{renderer:?}"), screen);
            assert_eq!(
                screen.contains('●'),
                !ascii && !pixel,
                "{renderer:?}: {screen}"
            );
            assert_eq!(screen.contains('*'), ascii, "{renderer:?}: {screen}");
        }
    }

    #[test]
    fn with_no_override_the_footer_keeps_the_ordinary_hints() {
        let state = AppState {
            history: Some(history_of(5, 1)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let screen = draw(&state, 60, 5);
        assert!(screen.contains("q quit"), "{screen}");
        assert!(!screen.contains("renderer forced"), "{screen}");
    }

    /// A message an escape sequence rendered inert in the tooltip
    /// (m1-the-graph/17): `history_of`'s fixture commits carry `\x1b[31m` in
    /// their subject, and neither the tooltip's subject nor its body may let
    /// a raw escape byte through.
    #[test]
    fn a_tooltip_renders_an_escape_sequence_inert() {
        let mut history = history_of(3, 0);
        history.cursor = 0;
        let state = AppState {
            tooltip: Some(0),
            history: Some(history),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let tooltip = graph_view(&state).tooltip.expect("row 0 has arrived");
        assert!(tooltip.subject.as_ref().starts_with("Commit 3"));
        assert!(
            !tooltip.subject.as_ref().contains('\u{1b}'),
            "{}",
            tooltip.subject.as_ref()
        );
        assert!(
            !tooltip.body.as_ref().contains('\u{1b}'),
            "{}",
            tooltip.body.as_ref()
        );

        let screen = draw(&state, 60, 5);
        assert!(!screen.contains('\u{1b}'), "{screen}");
    }

    /// Hover and the keyboard both point `state.tooltip` at the same
    /// absolute row, so both routes build byte-for-byte identical content
    /// (m1-the-graph/17) — the acceptance criterion this proves.
    #[test]
    fn hover_and_keyboard_produce_identical_tooltip_content() {
        let history = history_of(4, 2);
        let base = AppState {
            history: Some(history),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        // The keyboard opens the tooltip for the current selection (cursor).
        let keyboard = AppState {
            tooltip: Some(base.history.as_ref().expect("a walk").cursor),
            ..base.clone()
        };
        // A hover over that same row sets the identical absolute index.
        let hover = AppState {
            tooltip: Some(2),
            ..base.clone()
        };

        let from_keyboard = graph_view(&keyboard).tooltip.expect("row 2 has arrived");
        let from_hover = graph_view(&hover).tooltip.expect("row 2 has arrived");
        assert_eq!(from_keyboard, from_hover);
    }

    /// Refs whose `HEAD` resolves to `head`, or is unborn when `head` is
    /// `None`.
    pub(crate) fn refs_with_head(head: Option<Oid>) -> crate::refs::RefBadges {
        let state = match head {
            Some(id) => graft_git::repo::HeadState::Detached(id),
            None => graft_git::repo::HeadState::Unborn(
                graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec())
                    .expect("a valid ref name"),
            ),
        };
        crate::refs::badges(&[], &state, &std::collections::HashMap::new(), |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        })
        .expect("no branches to count")
    }

    pub(crate) fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes")
    }

    /// `state` with a 3-commit history (newest is `oid(3)`) and `HEAD`
    /// attached to the newest commit.
    fn state_with_head_and_counts(counts: graft_git::status::WorkingTreeCounts) -> AppState {
        AppState {
            history: Some(history_of(3, 0)),
            refs: Some(refs_with_head(Some(oid(3)))),
            working_tree: Some(counts),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        }
    }

    pub(crate) fn counts(
        staged: usize,
        unstaged: usize,
        untracked: usize,
    ) -> graft_git::status::WorkingTreeCounts {
        graft_git::status::WorkingTreeCounts {
            staged,
            unstaged,
            untracked,
        }
    }

    #[test]
    fn a_clean_tree_draws_no_uncommitted_row() {
        let state = state_with_head_and_counts(counts(0, 0, 0));

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 3, "no virtual row taken from the budget");
        assert_eq!(view.rows[0].line.row.commit.id, oid(3));
    }

    #[test]
    fn staged_only_names_only_staged() {
        let state = state_with_head_and_counts(counts(3, 0, 0));

        let view = graph_view(&state);

        assert_eq!(view.rows[0].line.node, NodeKind::Uncommitted);
        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (3 staged)"
        );
        assert_eq!(view.rows[0].author.initials.as_str(), "");
        assert_eq!(view.rows[0].when, "");
        assert_eq!(view.rows[0].sha, "");
        // The virtual row plus history rows, one fewer than a full viewport
        // (graph_rows() is 3 here; the row it draws takes one of them).
        assert_eq!(view.rows.len(), 3, "1 virtual row + 2 history rows");
        assert_eq!(view.rows[1].line.row.commit.id, oid(3));
    }

    #[test]
    fn unstaged_only_names_only_unstaged() {
        let state = state_with_head_and_counts(counts(0, 2, 0));

        let view = graph_view(&state);

        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (2 unstaged)"
        );
    }

    #[test]
    fn untracked_only_names_only_untracked() {
        let state = state_with_head_and_counts(counts(0, 0, 4));

        let view = graph_view(&state);

        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (4 untracked)"
        );
    }

    #[test]
    fn all_three_are_named_together() {
        let state = state_with_head_and_counts(counts(1, 2, 3));

        let view = graph_view(&state);

        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (1 staged, 2 unstaged, 3 untracked)"
        );
    }

    #[test]
    fn the_row_is_hidden_when_head_is_not_the_topmost_visible_commit() {
        // HEAD points at a commit that is not row 0 of what is on screen —
        // scrolled down, say. The row must not draw beside the wrong commit.
        let mut state = state_with_head_and_counts(counts(1, 0, 0));
        state.refs = Some(refs_with_head(Some(oid(1))));

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 3, "no virtual row");
        assert_eq!(view.rows[0].line.row.commit.id, oid(3));
    }

    #[test]
    fn an_unborn_branch_with_staged_files_still_shows_the_row() {
        // No commits at all yet, so nothing else could be "on top" of it.
        let state = AppState {
            history: Some(history_of(0, 0)),
            refs: Some(refs_with_head(None)),
            working_tree: Some(counts(2, 0, 0)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].line.node, NodeKind::Uncommitted);
        assert_eq!(
            view.rows[0].subject.as_str(),
            "Uncommitted changes (2 staged)"
        );
    }

    #[test]
    fn a_dirty_tree_before_refs_have_loaded_draws_no_row() {
        // Attachment cannot be decided without knowing HEAD; showing the row
        // and then possibly retracting it once refs answer would flicker.
        let state = AppState {
            history: Some(history_of(3, 0)),
            refs: None,
            working_tree: Some(counts(1, 0, 0)),
            size: Size {
                columns: 60,
                rows: 5,
            },
            ..AppState::default()
        };

        let view = graph_view(&state);

        assert_eq!(view.rows.len(), 3);
    }

    #[test]
    fn a_pending_jump_reads_as_one() {
        let mut history = history_of(2, 0);
        history.phase = Phase::Streaming;
        history.jump = Some(4);
        history.rows = 1500;
        let state = AppState {
            history: Some(history),
            ..AppState::default()
        };

        assert_eq!(
            graph_view(&state).walk,
            WalkView::Jumping { laid_out: 1500 }
        );
    }

    #[test]
    fn the_first_frame_says_it_is_still_opening() {
        let screen = draw(&AppState::default(), 60, 12);

        assert!(screen.contains("opening"), "{screen}");
    }

    #[test]
    fn a_branch_name_with_an_escape_in_it_cannot_reach_the_terminal() {
        // The whole point of the `SanitizedText` boundary, checked end to end
        // rather than only in `ui`.
        let state = open(Head::Branch(b"ma\x1b[31min".to_vec()));

        let screen = draw(&state, 60, 12);

        assert!(!screen.contains('\u{1b}'), "an escape reached the cells");
        assert!(screen.contains('\u{241b}'), "{screen}");
    }

    #[test]
    fn a_branch_name_with_an_escape_sequence_renders_inert() {
        let label = crate::refs::RefLabel::Branch {
            name: b"ma\x1b[31min".to_vec(),
            current: false,
            upstream: crate::refs::Upstream::InSync {
                remote: b"or\x1b]0;pwned\x07igin".to_vec(),
            },
        };

        let text = badge_view(&label).text(graft_ui::graph_screen::Icons::Plain);

        assert!(!text.contains('\x1b'), "{text:?}");
        assert!(!text.contains('\x07'), "{text:?}");
        assert!(
            text.contains("[31min"),
            "the rest of the name is kept: {text:?}"
        );
    }

    #[test]
    fn a_detached_head_is_abbreviated() {
        let id = Oid::from_hex("4f1a9c02be7711223344556677889900aabbccdd")
            .expect("forty hex characters");

        let view = status_view(&session(), &open(Head::Detached(id)));

        assert_eq!(
            view.repository.expect("the repository is open").head,
            HeadView::Detached(sanitize_line("4f1a9c02be77"))
        );
    }

    #[test]
    fn a_git_that_is_new_enough_says_nothing_at_all() {
        let state = AppState {
            system_git: Some(GitCheck::Ran(SystemGit::Supported(GitVersion::MINIMUM))),
            ..open(Head::Branch(b"main".to_vec()))
        };

        let view = status_view(&session(), &state);

        assert!(view.notes.is_empty(), "{:?}", view.notes);
        assert_eq!(view.problem, None);
    }

    #[test]
    fn an_old_git_and_a_missing_one_both_get_a_note() {
        let old = AppState {
            system_git: Some(GitCheck::Ran(SystemGit::TooOld {
                found: GitVersion {
                    major: 2,
                    minor: 30,
                    patch: 1,
                },
                minimum: GitVersion::MINIMUM,
            })),
            ..AppState::default()
        };
        let missing = AppState {
            system_git: Some(GitCheck::Unavailable("no git on PATH".to_owned())),
            ..AppState::default()
        };

        let old = status_view(&session(), &old);
        let missing = status_view(&session(), &missing);

        assert_eq!(old.notes.len(), 1);
        assert!(old.notes[0].as_str().contains("2.30.1"), "{:?}", old.notes);
        assert_eq!(missing.notes.len(), 1);
        assert!(
            missing.notes[0].as_str().contains("no git on PATH"),
            "{:?}",
            missing.notes
        );
    }

    #[test]
    fn a_failure_becomes_the_problem_line_and_is_sanitised() {
        let state = AppState {
            status: Status::Failed("job 3 failed: no such \u{1b}[31mref".to_owned()),
            ..AppState::default()
        };

        let view = status_view(&session(), &state);
        let problem = view.problem.expect("a failure must be shown");

        assert!(!problem.as_str().contains('\u{1b}'), "{problem:?}");
        assert!(problem.as_str().contains("no such"), "{problem:?}");
    }

    #[test]
    fn what_the_session_settled_is_what_the_screen_shows() {
        let session = session();

        let view = status_view(&session, &AppState::default());

        assert_eq!(view.rendering.tier, RenderingTier::Unicode);
        assert_eq!(view.rendering.tier_source, TierSource::Detected);
    }

    #[test]
    fn a_terminal_with_no_cells_at_all() {
        // ADR-0012's degenerate pty: the size a pty reports before anyone has
        // sized it. The point of the test is that drawing returns at all.
        let mut terminal =
            Terminal::new(TestBackend::new(0, 0)).expect("TestBackend reports its size");

        terminal
            .draw(|frame| view(&session(), &AppState::default(), frame))
            .expect("TestBackend cannot fail to draw");

        assert_eq!(terminal.backend().buffer().area.area(), 0);
    }

    #[test]
    fn a_terminal_two_cells_square() {
        // Far too small for the screen, and still not a panic.
        let screen = draw(&open(Head::Branch(b"main".to_vec())), 2, 2);

        assert_eq!(screen.lines().count(), 2, "{screen}");
    }

    #[test]
    fn drawing_does_not_change_the_state() {
        let state = AppState {
            status: Status::Ready,
            size: Size {
                columns: 40,
                rows: 4,
            },
            ..open(Head::Branch(b"main".to_vec()))
        };
        let before = state.clone();

        let _ = draw(&state, 40, 12);

        assert_eq!(state, before);
    }

    /// The search line draws the query sanitised: an escape sequence typed or
    /// pasted into the prompt, or matched in a subject, is inert on screen.
    #[test]
    fn a_search_line_with_an_escape_sequence_renders_inert() {
        use crate::search::{Query, Search, SearchState};

        let mut search = Search::new(
            Query::parse("Commit 3\x1b[31m").expect("a query"),
            0,
            (0, 0),
        );
        search.state = SearchState::Done { hits: 1 };
        let state = AppState {
            history: Some(history_of(5, 2)),
            search: Some(search),
            size: Size {
                columns: 80,
                rows: 8,
            },
            ..AppState::default()
        };
        let screen = draw(&state, 80, 8);
        assert!(!screen.contains('\x1b'), "{screen}");
        assert!(screen.contains("1 match"), "{screen}");

        let prompt = AppState {
            search_prompt: Some("x\x1b[2J".to_owned()),
            search: None,
            ..state
        };
        let screen = draw(&prompt, 80, 8);
        assert!(!screen.contains('\x1b'), "{screen}");
        assert!(screen.contains(" /x"), "{screen}");
    }
}
