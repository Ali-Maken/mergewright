//! The pixel tier sends the selection as its own layer (ADR-0053,
//! m7-power-tools/46): strips are drawn from layout at the frame's one
//! backdrop and stay uploaded while `j` moves; only the emphasis over them
//! changes. Kitty deflates when the probe said it may, and stops when the
//! terminal objects; iTerm2 gets one composited band per strip.
//!
//! Driven through the loop's own `draw` (and `pump_with` where replies
//! matter), on an in-memory terminal; the bytes are read back and each
//! Kitty transmit classified by its header.

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_graph::raster::Backdrop;
use graft_graph::raster::cache::{STRIP_ROWS, lit_hash};
use graft_term::kitty::EMPHASIS_ID_BIT;

use super::*;
use crate::view::strips::pixel_rows;
use crate::view::strips::tests::{capped, fan, fold_plan};
use graft_graph::fold::Keep;

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

/// Every Kitty transmit in `stream`: `a=t` headers (continuation chunks
/// carry no `a=`).
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

/// Image ids placed in `stream`.
fn placed(stream: &str) -> Vec<u32> {
    stream
        .split("a=p,i=")
        .skip(1)
        .filter_map(|rest| rest.split(',').next()?.parse().ok())
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
    let paint = crate::view::strips::StripPaint::of(session);
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

#[test]
fn j_inside_resident_strips_sends_no_strip() {
    for deflate in [false, true] {
        let (session, state) = branchy(PixelProtocol::Kitty, deflate);
        let run = drive(&session, state, js(20));

        let first = transmits(&run.frames[0]);
        assert!(
            first.iter().any(|t| !t.is_emphasis()),
            "deflate {deflate}: the first frame uploads its strips"
        );
        let later: Vec<Transmit> = run.frames[1..].iter().flat_map(|f| transmits(f)).collect();
        let strips = later.iter().filter(|t| !t.is_emphasis()).count();
        let emphasis = later.iter().filter(|t| t.is_emphasis()).count();
        eprintln!("deflate {deflate}: 20 j sent {strips} strips, {emphasis} emphasis images");
        assert_eq!(strips, 0, "deflate {deflate}: a j re-sends no strip");
        assert!(
            repainted(&run.paints) >= 1,
            "premise: the thread moved over 20 j"
        );
        assert!(emphasis >= 1, "deflate {deflate}: its emphasis went");
        assert!(
            run.backdrops.iter().all(|b| *b == Backdrop::OffThread),
            "{:?}",
            run.backdrops
        );
    }
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

/// `capped(16, 50, 70)` at the pixel tier on Kitty with deflate, 120x40,
/// cursor on the first merge.
fn capped_at_pixel_tier() -> (Session, AppState) {
    let (mut session, mut state) = at_pixel_tier();
    session.capabilities.kitty_deflate = true;
    let mut history = capped(16, 50, 70);
    history.strip_rows = Some(STRIP_ROWS);
    history.cursor = 0;
    state.history = Some(history);
    state.size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    (session, state)
}

/// On a graph wider than the lane cap the fold keeps the selection's thread
/// lanes (ADR-0025 Decision 2, `Keep::preferred`), so a `j` that moves the
/// thread may change which lanes draw, and every strip goes again. That is
/// layout, not paint. What ADR-0053 promises, and this guards: a backdrop is
/// re-sent only on a frame whose fold plan or strip window changed (a), the
/// fixture really does change the fold (b), and a changed selection paint
/// always goes as an emphasis layer (c).
///
/// Step 0 (46c), the old bound "no backdrop on any j" run red first: 16 red
/// frames (1–16), every one a plan change, `preferred` the only class that
/// moved; frames 17–24 changed neither plan nor paint and sent nothing.
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
        // (c) A changed selection paint goes as an emphasis layer.
        assert!(
            !lit_changed || emphasis >= 1,
            "frame {k}: the selection's paint changed and no emphasis went"
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
fn iterm2_bands_are_composites() {
    let (session, state) = branchy(PixelProtocol::Iterm2, false);
    let run = drive(&session, state, js(20));

    let bands = run.frames[0].matches("\x1b]1337;File=").count();
    assert!(bands > 0);
    for frame in &run.frames {
        assert_eq!(
            frame.matches("\x1b]1337;File=").count(),
            bands,
            "one band per strip"
        );
        assert!(transmits(frame).is_empty(), "no Kitty bytes on iTerm2");
    }
    // A band is encoded again only where the selection's paint over its
    // strip changed; a j inside a chain changes none.
    let repainted = repainted(&run.paints);
    assert!(repainted >= 1, "premise: the thread moved over 20 j");
    let encoded = run.graphics.bands_encoded();
    eprintln!("iterm2: {bands} bands, {encoded} encodes, {repainted} strips repainted over 20 j");
    assert_eq!(encoded, u64::try_from(bands + repainted).expect("few"));
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
        &mut terminal,
        &mut ScriptedWakes::spaced(vec![j(), j(), quit_key()]),
        &Recorded::default(),
        &mut session,
        state,
        &mut graphics,
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
