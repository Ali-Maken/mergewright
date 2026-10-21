//! The pixel tier sends the selection as its own layer (ADR-0053,
//! m7-power-tools/46): strips are drawn from layout at the frame's one
//! backdrop and stay uploaded while `j` moves; only the emphasis over them
//! changes. Kitty deflates when the probe said it may, and stops when the
//! terminal objects; iTerm2 gets one composited band per strip.
//!
//! Driven through the loop's own `draw` (and `pump_with` where replies
//! matter), on an in-memory terminal; the bytes are read back and each
//! Kitty transmit classified by its header.

use mergewright_core::capabilities::{PixelProtocol, RenderingTier};
use mergewright_graph::raster::Backdrop;
use mergewright_graph::raster::cache::{ByteBudget, STRIP_ROWS, lit_hash};
use mergewright_term::kitty::EMPHASIS_ID_BIT;

use super::*;
use crate::view::strips::pixel_rows;
use crate::view::strips::tests::{fan, fold_plan, woven};
use emphasis_pieces::{emphasis_placed, placed};
use mergewright_graph::fold::Keep;

mod emphasis_pieces;
mod focus;
mod pinned;
mod tall;

/// The spec's terminal for these runs (`120×40`).
const COLUMNS: u16 = 120;
const ROWS: u16 = 40;

/// One Kitty transmit, from its first chunk's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Transmit {
    id: u32,
    deflated: bool,
}

impl Transmit {
    fn is_emphasis(self) -> bool {
        self.id & EMPHASIS_ID_BIT != 0
    }
}

/// Every Kitty transmit in `stream`: `a=t` headers (later chunks carry no `a=`).
fn transmits(stream: &str) -> Vec<Transmit> {
    stream
        .split("\x1b_G")
        .skip(1)
        .filter_map(|command| {
            let header = command.split([';', '\x1b']).next()?;
            let keys: Vec<&str> = header.split(',').collect();
            if !keys.contains(&"a=t") {
                return None;
            }
            let id = keys
                .iter()
                .find_map(|pair| pair.strip_prefix("i="))?
                .parse()
                .ok()?;
            Some(Transmit {
                id,
                deflated: keys.contains(&"o=z"),
            })
        })
        .collect()
}

/// Six chains of 8 under an octopus merge (49 rows, two strips, all on
/// screen at 120x40), at the pixel tier on `protocol`, cursor on chain 1's
/// first row. A thread is a chain whole, so `j` changes the lit columns each
/// time it crosses into the next chain; no lane is folded.
fn branchy(protocol: PixelProtocol, deflate: bool) -> (Session, AppState) {
    let (mut session, mut state) = at_pixel_tier();
    session.capabilities.pixel_protocol = Some(protocol);
    session.capabilities.kitty_deflate = deflate;
    let mut history = fan(6, 8);
    history.strip_rows = Some(STRIP_ROWS);
    history.cursor = 1;
    state.history = Some(history);
    state.size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    (session, state)
}

/// What a run drew: each frame's bytes, the backdrop `pixel_rows` gave it,
/// and the graphics state after.
struct Drawn {
    frames: Vec<String>,
    backdrops: Vec<Backdrop>,
    /// Per frame, each strip's first row and the hash of the selection's
    /// paint over it (`lit_hash`).
    paints: Vec<Vec<(usize, u64)>>,
    graphics: Graphics<NoReplies>,
}

fn backdrop_of(state: &AppState) -> Backdrop {
    pixel_rows(state, crate::view::max_lanes(state), STRIP_ROWS).backdrop
}

fn paints_of(session: &Session, state: &AppState) -> Vec<(usize, u64)> {
    let pixels = pixel_rows(state, crate::view::max_lanes(state), STRIP_ROWS);
    let paint = crate::view::strips::StripPaint::for_frame(session, state);
    pixels
        .strips
        .iter()
        .map(|strip| {
            let hash = strip.with_layers(&pixels, &paint, |_, selection| lit_hash(selection));
            (strip.first_row, hash)
        })
        .collect()
}

/// Strips whose selection paint differs from the frame before's, over every
/// frame after the first.
fn repainted(paints: &[Vec<(usize, u64)>]) -> usize {
    paints
        .windows(2)
        .map(|pair| pair[1].iter().filter(|now| !pair[0].contains(now)).count())
        .sum()
}

/// Draws `state`, then each of `events` in turn, through the loop's `draw`.
fn drive(session: &Session, mut state: AppState, events: Vec<AppEvent>) -> Drawn {
    let (mut terminal, recorder) = terminal(state.size.columns, state.size.rows);
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let mut backdrops = vec![backdrop_of(&state)];
    let mut paints = vec![paints_of(session, &state)];
    draw(&mut terminal, session, &state, &mut graphics).expect("memory");
    for event in events {
        state = update(&state, event).0;
        backdrops.push(backdrop_of(&state));
        paints.push(paints_of(session, &state));
        draw(&mut terminal, session, &state, &mut graphics).expect("memory");
    }
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let mut frames: Vec<String> = recorder.written().split(&end).map(str::to_owned).collect();
    frames.pop();
    assert_eq!(frames.len(), backdrops.len(), "one frame per draw");
    Drawn {
        frames,
        backdrops,
        paints,
        graphics,
    }
}

fn press(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(code)))
}

fn js(count: usize) -> Vec<AppEvent> {
    vec![press(KeyCode::Char('j')); count]
}

/// m8 04: held `j` down a wide graph (12 lanes woven, 241 rows, past seven
/// strips) under a budget of the viewport floor plus two strips — the
/// regime the user's 17×37 cell runs in, where the budget binds and the
/// floor does not (review Major 1: at 1.5 frames the floor won 210 of 230
/// frames, so the budget was never what held the cache). After warm-up the
/// cache weighs no more than its budget on every frame, the floor never
/// wins, and evictions happen; the pool is steady.
#[test]
fn held_j_on_wide_strips_stays_within_the_byte_budget() {
    let (mut session, mut state) = at_pixel_tier();
    session.capabilities.pixel_protocol = Some(PixelProtocol::Kitty);
    let mut history = woven(12, 20);
    history.strip_rows = Some(STRIP_ROWS);
    state.history = Some(history);
    state.size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    let (mut terminal, _recorder) = terminal(COLUMNS, ROWS);
    let (pair, floor) = strip_weight_and_floor(&mut terminal, &session, &state);
    let budget = ByteBudget((floor + 2) * pair);
    let mut graphics = Graphics::new(NoReplies, &session.capabilities).with_strip_budget(budget);
    draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
    let mut budget_evictions = 0;
    let mut warm = None;
    for (index, event) in js(230).into_iter().enumerate() {
        if index == 115 {
            warm = Some(graphics.strip_stats());
        }
        let evicted = graphics.strip_stats().evicted;
        state = update(&state, event).0;
        draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
        let (resident, minimum_wins) = graphics.strip_budget_report();
        assert!(
            !minimum_wins,
            "j {index}: the floor ({floor} a layer) held the cache over a {} byte budget",
            budget.0
        );
        assert!(
            resident <= budget.0,
            "j {index}: {resident} bytes resident over a {} byte budget",
            budget.0
        );
        if graphics.strip_stats().evicted > evicted {
            budget_evictions += 1;
        }
    }
    let top = state.history.as_ref().map_or(0, |history| history.top);
    assert!(
        top > 4 * STRIP_ROWS,
        "premise: j scrolled past strips ({top})"
    );
    assert!(
        budget_evictions > 0,
        "premise: the budget, not the floor, evicted"
    );
    eprintln!(
        "budget {} bytes ({floor} + 2 strips of {pair}); {budget_evictions} frames evicted",
        budget.0
    );
    let (warm, last) = (warm.expect("230 events"), graphics.strip_stats());
    assert!(
        last.rasterised + last.emphasis_rasterised > warm.rasterised + warm.emphasis_rasterised,
        "premise: strips were drawn in the second half"
    );
    assert_eq!(
        last.buffers_allocated, warm.buffers_allocated,
        "steady state draws into evicted buffers"
    );
}

/// What a strip and its emphasis weigh together, from `state`'s first frame
/// drawn with room to spare, and the per-layer floor that frame set.
fn strip_weight_and_floor(
    terminal: &mut Terminal<CrosstermBackend<Recorder>>,
    session: &Session,
    state: &AppState,
) -> (usize, usize) {
    let mut probe = Graphics::new(NoReplies, &session.capabilities);
    draw(terminal, session, state, &mut probe).expect("memory");
    let strips = usize::try_from(probe.strip_stats().rasterised).expect("small");
    assert!(strips > 0, "premise: the first frame drew strips");
    (probe.strip_budget_report().0 / strips, probe.strip_floor())
}

/// Each state a run of `events` passes through from `state`, the first
/// included: what `drive` draws, frame by frame.
fn states(mut state: AppState, events: &[AppEvent]) -> Vec<AppState> {
    let mut all = vec![state.clone()];
    for event in events {
        state = update(&state, event.clone()).0;
        all.push(state.clone());
    }
    all
}

/// One frame's fold as `pixel_rows` plans it, and the strip window.
#[derive(Debug, PartialEq, Eq)]
struct Planned {
    kept: Vec<usize>,
    lane_columns: usize,
    marker: Option<usize>,
    strips: Vec<usize>,
    /// The lanes' roles (ease-pass 13): a focus change re-keys strips.
    roles: Vec<(
        mergewright_graph::identity::BranchIdentity,
        mergewright_graph::LaneRole,
    )>,
}

/// `state`'s fold plan as drawn, and the `Keep` it weighed.
fn planned(state: &AppState) -> (Planned, Keep) {
    let max = crate::view::max_lanes(state);
    let (fold, keep) = fold_plan(state, max).expect("strips draw");
    let pixels = pixel_rows(state, max, STRIP_ROWS);
    let planned = Planned {
        kept: fold.kept().to_vec(),
        lane_columns: pixels.lane_columns,
        marker: pixels.marker.map(|marker| marker.column),
        strips: pixels.strips.iter().map(|strip| strip.first_row).collect(),
        roles: pixels.roles,
    };
    (planned, keep)
}

/// Which `Keep` classes differ between two frames (ADR-0025's names).
fn keep_differs(before: &Keep, now: &Keep) -> Vec<&'static str> {
    let mut classes = Vec::new();
    if now.selected != before.selected {
        classes.push("selected");
    }
    if now.reserved != before.reserved {
        classes.push("reserved");
    }
    if now.preferred != before.preferred {
        classes.push("preferred");
    }
    classes
}

/// `woven(16, 6)` at the pixel tier on Kitty with deflate, 120x40,
/// cursor on the octopus merge at the top.
fn capped_at_pixel_tier() -> (Session, AppState) {
    let (mut session, mut state) = at_pixel_tier();
    session.capabilities.kitty_deflate = true;
    let mut history = woven(16, 6);
    history.strip_rows = Some(STRIP_ROWS);
    history.cursor = 0;
    state.history = Some(history);
    state.size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    (session, state)
}

/// On a graph wider than the lane cap the fold keeps the selected column
/// and the selection's thread lanes first (ADR-0025, `Keep::selected`,
/// `preferred`), so a `j` that moves the selection may change which lanes
/// draw, and every strip goes again. That is layout, not paint. What
/// ADR-0053 promises, and this guards: a backdrop is re-sent only on a frame
/// whose fold plan, strip window or lanes' roles changed (a), the fixture really does
/// change the fold (b), and a changed selection paint always changes the
/// emphasis placed (c). Step 0 (46c) ran the old bound "no backdrop on any
/// j" red first, on a fixture whose side lanes held no node; those drop
/// since ease-pass 06b, so `woven` keeps a node in every column in view.
#[test]
fn a_capped_graph_resends_strips_only_when_the_fold_changes() {
    let (session, state) = capped_at_pixel_tier();
    let events = js(24);
    let cap = crate::view::max_lanes(&state);
    let (fold, _) = fold_plan(&state, cap).expect("strips draw");
    let open = fold.kept().len() + fold.folded().len();
    eprintln!("capped: {open} open lanes, cap {cap}");
    assert!(open > cap, "capped: {open} open lanes, cap {cap}");

    let walk = states(state.clone(), &events);
    let plans: Vec<(Planned, Keep)> = walk.iter().map(planned).collect();
    let run = drive(&session, state, events);

    let mut fold_changes = 0;
    for k in 1..run.frames.len() {
        let sent = transmits(&run.frames[k]);
        let backdrop = sent.iter().filter(|t| !t.is_emphasis()).count();
        let emphasis = sent.len() - backdrop;
        let (now, keep_now) = &plans[k];
        let (before, keep_before) = &plans[k - 1];
        let changed = now != before;
        let classes = keep_differs(keep_before, keep_now);
        let lit_changed = run.paints[k] != run.paints[k - 1];
        eprintln!(
            "frame {k}: plan changed {}, keep differs {classes:?}, kept {:?}, \
             backdrop transmits {backdrop}, emphasis {emphasis}, lit changed {}",
            if changed { "y" } else { "n" },
            now.kept,
            if lit_changed { "y" } else { "n" },
        );
        // (a) Paint alone never re-sends a backdrop (ADR-0053).
        assert!(
            changed || backdrop == 0,
            "frame {k}: {backdrop} backdrops sent with the fold and strip window unchanged"
        );
        // (c) A changed selection paint changes the emphasis placed: pieces
        // sent, or held ones placed anew (ADR-0053 amendment 2026-10-10).
        let lit = |frame: &String| emphasis_placed(frame);
        assert!(
            !lit_changed || emphasis >= 1 || lit(&run.frames[k]) != lit(&run.frames[k - 1]),
            "frame {k}: the selection's paint changed and its emphasis did not"
        );
        if changed {
            fold_changes += 1;
        }
    }
    // (b) The fixture really is capped: the fold follows the selection.
    assert!(fold_changes >= 1, "no j changed the fold");
}

#[test]
fn stash_only_rows_stay_dimmed_without_a_thread() {
    // The cursor's row is not on hand (the wheel left it far above), so
    // there is no thread and nothing waits for it (37).
    let window_from = |stash_only: bool| {
        let mut history = crate::view::tests::history_of(120, 0);
        history.strip_rows = Some(STRIP_ROWS);
        let window = history.window.as_mut().expect("history_of sends a window");
        let marked = window.rows[70].commit.id;
        window.rows = window.rows[63..].into();
        window.first = 63;
        history.top = 64;
        let mut state = AppState {
            history: Some(history),
            settled_tier: Some(RenderingTier::Pixel),
            size: Size {
                columns: COLUMNS,
                rows: 30,
            },
            ..AppState::default()
        };
        if stash_only {
            state.stash_only.commits = Arc::new([(marked, 0)].into());
        }
        state
    };

    let with = pixel_rows(&window_from(true), 12, STRIP_ROWS);
    assert!(!with.strips.is_empty());
    assert_eq!(with.backdrop, Backdrop::Faded);
    let without = pixel_rows(&window_from(false), 12, STRIP_ROWS);
    assert_eq!(without.backdrop, Backdrop::Full);
}

#[test]
fn focus_changes_the_backdrop_only_when_lowest_changes() {
    let (session, state) = branchy(PixelProtocol::Kitty, false);
    let mut events = vec![press(KeyCode::Char('F'))];
    events.extend(js(5));
    events.push(press(KeyCode::Char('F')));
    let run = drive(&session, state, events);

    for (k, frame) in run.frames.iter().enumerate() {
        let sent = transmits(frame);
        let strips = sent.iter().filter(|t| !t.is_emphasis()).count();
        eprintln!(
            "frame {k}: backdrop {:?}, {strips} strips, {} emphasis",
            run.backdrops[k],
            sent.len() - strips
        );
        let changed = k == 0 || run.backdrops[k] != run.backdrops[k - 1];
        assert!(
            changed || strips == 0,
            "frame {k}: a strip sent with the backdrop unchanged"
        );
    }
    // Every row carries the thread, which overrides a row's own dimming:
    // Focus leaves nothing at `Faded`, so the backdrop never moves.
    assert!(run.backdrops.iter().all(|b| *b == Backdrop::OffThread));
}

#[test]
fn cursor_off_screen_draws_full() {
    let (session, mut away) = branchy(PixelProtocol::Kitty, false);
    // Six chains of 20 (121 rows), cursor on row 1, its row gone from the
    // window (the wheel took the view far from it): no thread, so `Full`
    // and no emphasis placed.
    let mut history = fan(6, 20);
    history.strip_rows = Some(STRIP_ROWS);
    history.cursor = 1;
    away.history = Some(history);
    let history = away.history.as_mut().expect("set above");
    let window = history.window.as_mut().expect("fan sends a window");
    window.rows = window.rows[63..].into();
    window.first = 63;
    history.top = 64;
    let run = drive(&session, away.clone(), vec![]);
    assert_eq!(run.backdrops[0], Backdrop::Full);
    assert!(!placed(&run.frames[0]).is_empty(), "strips placed");
    assert!(
        placed(&run.frames[0])
            .iter()
            .all(|id| id & EMPHASIS_ID_BIT == 0),
        "no emphasis placed"
    );

    // Back on screen: the thread, so `OffThread` and its emphasis.
    let history = away.history.as_mut().expect("still there");
    history.cursor = 70;
    let run = drive(&session, away, vec![]);
    assert_eq!(run.backdrops[0], Backdrop::OffThread);
    assert!(
        placed(&run.frames[0])
            .iter()
            .any(|id| id & EMPHASIS_ID_BIT != 0)
    );
}

#[test]
fn deflate_follows_the_probe() {
    for deflate in [false, true] {
        let (session, state) = branchy(PixelProtocol::Kitty, deflate);
        let run = drive(&session, state, js(3));
        let sent: Vec<Transmit> = run.frames.iter().flat_map(|f| transmits(f)).collect();
        assert!(!sent.is_empty());
        assert!(
            sent.iter().all(|t| t.deflated == deflate),
            "kitty_deflate {deflate}: {sent:?}"
        );
    }
}

#[test]
fn a_deflate_error_falls_back_in_the_app() {
    let (mut session, state) = branchy(PixelProtocol::Kitty, true);
    let (mut terminal, recorder) = terminal(COLUMNS, ROWS);
    let mut graphics = Graphics::new(
        ScriptedReplies(vec![b"\x1b_Gi=1000;EINVAL:bad compression\x1b\\".to_vec()].into()),
        &session.capabilities,
    );
    let j = || key(KeyPress::plain(KeyCode::Char('j')));
    pump_with(
        Screen {
            terminal: &mut terminal,
            graphics: &mut graphics,
        },
        &mut ScriptedWakes::spaced(vec![j(), j(), quit_key()]),
        &Recorded::default(),
        &mut session,
        state,
        Outside::default(),
        &mut crate::trace::Tracer::off(),
    )
    .expect("writing to memory cannot fail");
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let written = recorder.written();
    let frames: Vec<&str> = written.split(&end).collect();

    let first = transmits(frames[0]);
    assert_eq!(first.first().map(|t| t.id), Some(1000));
    assert!(first.iter().all(|t| t.deflated), "{first:?}");
    let after: Vec<Transmit> = frames[1..].iter().flat_map(|f| transmits(f)).collect();
    assert!(
        after.iter().any(|t| !t.is_emphasis()),
        "the refused strip goes again: {after:?}"
    );
    assert!(
        after.iter().all(|t| !t.deflated),
        "raw from then on: {after:?}"
    );
}

/// m8 17c, ADR-0060: the pixel-tier state the preview fixture builds —
/// 120 commits, scrolled two screens in, one ghost three rows down — with
/// the preview closed, and with it open and its plan landed.
fn preview_at_pixel() -> (Session, AppState, AppState) {
    let (session, _) = at_pixel_tier();
    let (mut open, _, _) = crate::view::tests::scrolled_preview(120, |height| 2 * height, 3);
    open.settled_tier = Some(RenderingTier::Pixel);
    if let Some(history) = open.history.as_mut() {
        history.strip_rows = Some(STRIP_ROWS);
    }
    let closed = AppState {
        preview: None,
        ..open.clone()
    };
    (session, closed, open)
}

/// Draws each state in turn on one terminal through the loop's `draw`;
/// one string per frame.
fn draw_each(session: &Session, states: &[&AppState]) -> Vec<String> {
    let first = states.first().expect("at least one state");
    let (mut terminal, recorder) = terminal(first.size.columns, first.size.rows);
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    for state in states {
        draw(&mut terminal, session, state, &mut graphics).expect("memory");
    }
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let mut frames: Vec<String> = recorder.written().split(&end).map(str::to_owned).collect();
    frames.pop();
    assert_eq!(frames.len(), states.len(), "one frame per draw");
    frames
}

#[test]
fn a_preview_at_pixel_draws_the_unicode_graph_and_places_no_strip() {
    let (session, closed, open) = preview_at_pixel();
    let frames = draw_each(&session, &[&closed, &open]);

    assert!(
        !placed(&frames[0]).is_empty(),
        "premise: closed, the pixel graph places its strips"
    );
    assert_eq!(
        placed(&frames[1]),
        Vec::<u32>::new(),
        "no strip placed while the preview holds the column"
    );
    assert_eq!(
        transmits(&frames[1]),
        Vec::new(),
        "no strip rastered for the preview"
    );
    // The fixture's one ghost hangs off its anchor with no straight run, so
    // its mark is the ghost node `○` (`mergewright_ui` `ghost_node_glyph`); `┊`
    // only draws on a ghost edge that runs through a row.
    assert!(
        frames[1].contains('○'),
        "the text layer draws the ghost's node"
    );
    assert!(
        frames[1].contains('●'),
        "and the real rows' nodes, in text, as one picture"
    );
}

#[test]
fn closing_the_preview_replaces_the_strips() {
    let (session, closed, open) = preview_at_pixel();
    let frames = draw_each(&session, &[&closed, &open, &closed]);

    let before: std::collections::BTreeSet<u32> = placed(&frames[0]).into_iter().collect();
    let after: std::collections::BTreeSet<u32> = placed(&frames[2]).into_iter().collect();
    assert!(!after.is_empty(), "closed again, the strips are placed");
    assert_eq!(before, after, "the same strips, by id, from the cache");
    let strips: Vec<Transmit> = transmits(&frames[2])
        .into_iter()
        .filter(|t| !t.is_emphasis())
        .collect();
    assert_eq!(strips, Vec::new(), "zero new rasters on close");
}
